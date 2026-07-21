//! The HF sibling of `runner.rs`'s aggregation (decision D-b): one pass from
//! `(keys, HfCellResults, floors)` to [`CandidateEvidence`] (with `cost_drag_share` /
//! `per_trade_edge` now `Some` — redeeming C6's promise), [`FeeSensitivity`], and the
//! per-candidate HF-rung rollups (decision D-h). Pure; no execution wiring.
//!
//! The LF `aggregate_evidence` / `aggregate_fee_sensitivity` stay byte-untouched — their
//! wildcard arms remain the protection that HF rungs can never perturb M4/M5 byte-identity.
//! The shared field definitions here are COPIES of theirs, exactly: returns mean over windows;
//! drawdown/turnover worst = max; dispersion = max − min; `baseline_margin = mean(Base) −
//! mean(base_floors)`; neighbor degradation via `neighbor_indices`, floored at 0; fee slots:
//! return mean / turnover max / trades+fees+priority sums; `survives_floor =
//! mean(doubled_floors)`.
//!
//! All decision math is exact `Decimal`; `f64` only ever sits inside the copied `CellResult`
//! display fields, never read here.

use crate::advance::CandidateEvidence;
use crate::hf_cell::HfCellResult;
use crate::param::{ParamGrid, ParamPoint};
use crate::runner::{neighbor_indices, CellKey};
use crate::sensitivity::{FeeSensitivity, ScenarioId, ScenarioMetrics};
use research_core::Decimal;

/// Per-candidate rollup of the three HF-only rungs plus the BASE-rung event counters
/// (decision D-h): everyday economics are priced at the Base rung; the worst-case rung's
/// totals are visible in its own metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct HfRungRollup {
    pub hot_congestion: ScenarioMetrics,
    pub adversarial_worst: ScenarioMetrics,
    pub latency_2x: ScenarioMetrics,
    /// BASE-rung sum of adverse-selection hits across windows.
    pub adverse_selection_hits: u32,
    /// BASE-rung sum of adverse-selection cost paid across windows, in USDC.
    pub adverse_selection_paid_quote: Decimal,
    /// BASE-rung sum of in-range attempts that did not land, across windows.
    pub unlanded_orders: u32,
}

/// Everything one HF aggregation pass produces, all vectors in candidate (point) order.
#[derive(Debug, Clone, PartialEq)]
pub struct HfAggregate {
    pub evidence: Vec<CandidateEvidence>,
    pub fee: Vec<FeeSensitivity>,
    pub rungs: Vec<HfRungRollup>,
}

/// One accumulator per (point, rung) slot — the LF `Acc` shape plus the HF counters and the
/// per-trade-edge numerator.
#[derive(Clone, Default)]
struct Acc {
    returns: Vec<Decimal>,
    max_drawdown: Decimal,
    turnover: Decimal,
    n_trades: u32,
    fees: Decimal,
    slippage: Decimal,
    priority: Decimal,
    /// Σ over windows of `(final_equity − initial_cash_usdc)` — the D-g per-trade-edge numerator.
    equity_edge: Decimal,
    adverse_selection_hits: u32,
    adverse_selection_paid_quote: Decimal,
    unlanded_orders: u32,
}

const SLOT_BEFORE_COSTS: usize = 0;
const SLOT_BASE: usize = 1;
const SLOT_DOUBLED: usize = 2;
const SLOT_HOT_CONGESTION: usize = 3;
const SLOT_ADVERSARIAL_WORST: usize = 4;
const SLOT_LATENCY_2X: usize = 5;

/// Aggregate raw HF cell results into per-candidate evidence, fee sensitivity, and HF-rung
/// rollups — the HF sibling of `aggregate_evidence` + `aggregate_fee_sensitivity` in one pass.
///
/// New over LF (decision D-g, exact `Decimal`):
/// - `cost_drag_share = fee.return_drag_costs / fee.before_costs.total_return`, `None` when
///   `before_costs.total_return <= 0` (guarded for non-positive gross);
/// - `per_trade_edge = Σ_w (Base final_equity − initial_cash_usdc) / Σ_w Base n_trades`, `None`
///   when that trade sum is 0 (the adverse-selection sink is already inside `final_equity` via
///   `run_hf_priced`).
///
/// # Panics
/// On an LF-only rung (`DoubledSlippage` / `DoubledPriority`) in `keys` — an internal-contract
/// assertion: the HF caller enumerates only the 6-rung ladder.
#[must_use]
pub fn aggregate_hf(
    grids: &[ParamGrid],
    keys: &[CellKey],
    results: &[HfCellResult],
    n_windows: usize,
    base_floors: &[Decimal],    // per-window best baseline return, Base rung
    doubled_floors: &[Decimal], // per-window best baseline return, Doubled rung
    initial_cash_usdc: Decimal,
) -> HfAggregate {
    let points: Vec<ParamPoint> = grids.iter().flat_map(|g| g.points()).collect();

    let mut acc: Vec<[Acc; 6]> = (0..points.len()).map(|_| Default::default()).collect();
    for (k, r) in keys.iter().zip(results) {
        let slot = match k.scenario {
            ScenarioId::BeforeCosts => SLOT_BEFORE_COSTS,
            ScenarioId::Base => SLOT_BASE,
            ScenarioId::Doubled => SLOT_DOUBLED,
            ScenarioId::HotCongestion => SLOT_HOT_CONGESTION,
            ScenarioId::AdversarialWorst => SLOT_ADVERSARIAL_WORST,
            ScenarioId::Latency2x => SLOT_LATENCY_2X,
            ScenarioId::DoubledSlippage | ScenarioId::DoubledPriority => {
                unreachable!("LF-only rung in HF aggregation")
            }
        };
        let a = &mut acc[k.point_index][slot];
        a.returns.push(r.cell.total_return);
        a.max_drawdown = a.max_drawdown.max(r.cell.max_drawdown);
        a.turnover = a.turnover.max(r.cell.turnover);
        a.n_trades += r.cell.n_trades;
        a.fees += r.cell.fees_paid_quote;
        a.slippage += r.cell.slippage_paid_quote;
        a.priority += r.cell.priority_fees_paid_sol;
        a.equity_edge += r.cell.final_equity - initial_cash_usdc;
        a.adverse_selection_hits += r.adverse_selection_hits;
        a.adverse_selection_paid_quote += r.adverse_selection_paid_quote;
        a.unlanded_orders += r.unlanded_orders;
    }

    let mean = |xs: &[Decimal]| {
        if xs.is_empty() {
            Decimal::ZERO
        } else {
            xs.iter().copied().sum::<Decimal>() / Decimal::from(xs.len() as u64)
        }
    };
    let spread = |xs: &[Decimal]| {
        if xs.is_empty() {
            Decimal::ZERO
        } else {
            let max = xs.iter().copied().max().expect("non-empty");
            let min = xs.iter().copied().min().expect("non-empty");
            max - min
        }
    };
    let metrics = |scenario: ScenarioId, a: &Acc| ScenarioMetrics {
        scenario,
        total_return: mean(&a.returns),
        turnover: a.turnover,
        n_trades: a.n_trades,
        fees_paid_quote: a.fees,
        slippage_paid_quote: a.slippage,
        priority_fees_paid_sol: a.priority,
    };

    let base_floor_mean = mean(base_floors);
    let doubled_floor_mean = mean(doubled_floors);
    let means: Vec<Decimal> = acc.iter().map(|s| mean(&s[SLOT_BASE].returns)).collect();
    let valid_windows = u32::try_from(n_windows).unwrap_or(u32::MAX);

    let fee: Vec<FeeSensitivity> = acc
        .iter()
        .map(|slots| {
            FeeSensitivity::from_scenarios(
                metrics(ScenarioId::BeforeCosts, &slots[SLOT_BEFORE_COSTS]),
                metrics(ScenarioId::Base, &slots[SLOT_BASE]),
                metrics(ScenarioId::Doubled, &slots[SLOT_DOUBLED]),
                doubled_floor_mean,
            )
        })
        .collect();

    let evidence: Vec<CandidateEvidence> = points
        .iter()
        .enumerate()
        .map(|(pi, point)| {
            let slots = &acc[pi];
            let fs = &fee[pi];
            let cost_drag_share = if fs.before_costs.total_return > Decimal::ZERO {
                Some(fs.return_drag_costs / fs.before_costs.total_return)
            } else {
                None
            };
            let base = &slots[SLOT_BASE];
            let per_trade_edge = if base.n_trades == 0 {
                None
            } else {
                Some(base.equity_edge / Decimal::from(base.n_trades))
            };
            CandidateEvidence {
                candidate_label: format!("{}/{}", point.family(), point.param_id()),
                max_drawdown: base.max_drawdown,
                turnover: base.turnover,
                baseline_margin: means[pi] - base_floor_mean,
                doubled_return: mean(&slots[SLOT_DOUBLED].returns),
                doubled_baseline_floor: doubled_floor_mean,
                fold_dispersion: spread(&base.returns),
                neighbor_degradation: neighbor_indices(grids, pi)
                    .into_iter()
                    .map(|ni| means[pi] - means[ni])
                    .max()
                    .unwrap_or(Decimal::ZERO)
                    .max(Decimal::ZERO),
                valid_windows,
                cost_drag_share,
                per_trade_edge,
            }
        })
        .collect();

    let rungs: Vec<HfRungRollup> = acc
        .iter()
        .map(|slots| {
            let base = &slots[SLOT_BASE];
            HfRungRollup {
                hot_congestion: metrics(ScenarioId::HotCongestion, &slots[SLOT_HOT_CONGESTION]),
                adversarial_worst: metrics(
                    ScenarioId::AdversarialWorst,
                    &slots[SLOT_ADVERSARIAL_WORST],
                ),
                latency_2x: metrics(ScenarioId::Latency2x, &slots[SLOT_LATENCY_2X]),
                adverse_selection_hits: base.adverse_selection_hits,
                adverse_selection_paid_quote: base.adverse_selection_paid_quote,
                unlanded_orders: base.unlanded_orders,
            }
        })
        .collect();

    HfAggregate {
        evidence,
        fee,
        rungs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellResult;
    use rust_decimal_macros::dec;

    const LADDER: [ScenarioId; 6] = [
        ScenarioId::BeforeCosts,
        ScenarioId::Base,
        ScenarioId::Doubled,
        ScenarioId::HotCongestion,
        ScenarioId::AdversarialWorst,
        ScenarioId::Latency2x,
    ];

    fn grids() -> Vec<ParamGrid> {
        vec![ParamGrid::ThresholdRebalance {
            target_sol_weights: vec![dec!(0.25), dec!(0.5)],
            bands: vec![dec!(0.1)],
        }]
    }

    #[allow(clippy::too_many_arguments)]
    fn hf_result(
        total_return: Decimal,
        max_drawdown: Decimal,
        turnover: Decimal,
        n_trades: u32,
        final_equity: Decimal,
        fees: Decimal,
        adverse_hits: u32,
        adverse_paid: Decimal,
        unlanded: u32,
    ) -> HfCellResult {
        HfCellResult {
            cell: CellResult {
                total_return,
                max_drawdown,
                turnover,
                n_trades,
                final_equity,
                traded_notional_quote: Decimal::ZERO,
                fees_paid_quote: fees,
                slippage_paid_quote: Decimal::ZERO,
                priority_fees_paid_sol: Decimal::ZERO,
                time_in_market: Decimal::ZERO,
                cagr: None,
                volatility: None,
                sharpe: None,
                sortino: None,
                calmar: None,
            },
            unlanded_orders: unlanded,
            adverse_selection_hits: adverse_hits,
            adverse_selection_paid_quote: adverse_paid,
        }
    }

    /// 2 windows × 6 rungs × 2 points in canonical order (window → scenario → point). Values
    /// are chosen so every aggregate below is an exact hand-computable Decimal. Point 1's cells
    /// are point 0's with return shifted by −0.05 and no trades (exercising the zero-trade
    /// guard alongside real values).
    fn keys_and_results() -> (Vec<CellKey>, Vec<HfCellResult>) {
        let mut keys = Vec::new();
        let mut results = Vec::new();
        for wi in 0..2usize {
            for &scenario in &LADDER {
                for pi in 0..2usize {
                    keys.push(CellKey {
                        window_index: wi,
                        scenario,
                        point_index: pi,
                    });
                    let w = Decimal::from(wi as u64); // window 0 → 0, window 1 → 1
                    let (ret, dd, to, trades, equity, fees, hits, paid, unlanded) = match scenario {
                        ScenarioId::BeforeCosts => (
                            dec!(0.20) + w * dec!(0.20), // 0.20, 0.40 → mean 0.30
                            dec!(0.05),
                            dec!(1),
                            2,
                            dec!(1200),
                            dec!(0),
                            0,
                            dec!(0),
                            0,
                        ),
                        ScenarioId::Base => (
                            dec!(0.10) + w * dec!(0.20), // 0.10, 0.30 → mean 0.20
                            dec!(0.10) + w * dec!(0.30), // max 0.40
                            dec!(1) + w * dec!(2),       // max 3
                            2 + wi as u32,               // 2, 3 → sum 5
                            dec!(1100) + w * dec!(200),  // 1100, 1300
                            dec!(1) + w * dec!(2),       // 1, 3 → sum 4
                            1 + wi as u32,               // 1, 2 → sum 3
                            dec!(0.5) + w * dec!(1),     // 0.5, 1.5 → sum 2
                            2 + wi as u32,               // 2, 3 → sum 5
                        ),
                        ScenarioId::Doubled => (
                            dec!(0.05) + w * dec!(0.10), // 0.05, 0.15 → mean 0.10
                            dec!(0.15),
                            dec!(2),
                            2,
                            dec!(1050),
                            dec!(2) + w * dec!(2), // 2, 4 → sum 6
                            0,
                            dec!(0),
                            0,
                        ),
                        ScenarioId::HotCongestion => (
                            dec!(0.08) + w * dec!(0.10), // mean 0.13
                            dec!(0.2),
                            dec!(1) + w * dec!(3), // max 4
                            1 + wi as u32,         // sum 3
                            dec!(1080),
                            dec!(3) + w * dec!(1), // sum 7
                            0,
                            dec!(0),
                            1,
                        ),
                        ScenarioId::AdversarialWorst => (
                            dec!(0.02) + w * dec!(0.04), // mean 0.04
                            dec!(0.3),
                            dec!(2),
                            2,
                            dec!(1020),
                            dec!(4), // sum 8
                            2,
                            dec!(2),
                            0,
                        ),
                        ScenarioId::Latency2x => (
                            dec!(0.06) + w * dec!(0.06), // mean 0.09
                            dec!(0.25),
                            dec!(1),
                            1,
                            dec!(1060),
                            dec!(2), // sum 4
                            0,
                            dec!(0),
                            3 + wi as u32, // sum 7
                        ),
                        ScenarioId::DoubledSlippage | ScenarioId::DoubledPriority => {
                            unreachable!("not in the HF ladder")
                        }
                    };
                    let shift = Decimal::from(pi as u64) * dec!(0.05);
                    results.push(hf_result(
                        ret - shift,
                        dd,
                        to,
                        if pi == 0 { trades } else { 0 },
                        equity,
                        fees,
                        hits,
                        paid,
                        unlanded,
                    ));
                }
            }
        }
        (keys, results)
    }

    #[test]
    fn evidence_fields_are_exact_including_the_new_some_values() {
        let (keys, results) = keys_and_results();
        let base_floors = vec![dec!(0.05), dec!(0.05)];
        let doubled_floors = vec![dec!(0.02), dec!(0.04)];

        let agg = aggregate_hf(
            &grids(),
            &keys,
            &results,
            2,
            &base_floors,
            &doubled_floors,
            dec!(1000),
        );
        assert_eq!(agg.evidence.len(), 2);
        let ev = &agg.evidence[0];

        assert_eq!(ev.max_drawdown, dec!(0.40));
        assert_eq!(ev.turnover, dec!(3));
        assert_eq!(ev.baseline_margin, dec!(0.15)); // mean(0.10, 0.30) − mean(floors)
        assert_eq!(ev.doubled_return, dec!(0.10));
        assert_eq!(ev.doubled_baseline_floor, dec!(0.03));
        assert_eq!(ev.fold_dispersion, dec!(0.20));
        // Point 0's Base mean 0.20 vs point 1's 0.15 → degradation 0.05 for point 0's neighbor?
        // Point 0's only axis neighbor is point 1 (weights axis): 0.20 − 0.15 = 0.05.
        assert_eq!(ev.neighbor_degradation, dec!(0.05));
        assert_eq!(ev.valid_windows, 2);

        // D-g: cost_drag_share = return_drag_costs / before_costs return = (0.30 − 0.20) / 0.30.
        assert_eq!(ev.cost_drag_share, Some(dec!(0.1) / dec!(0.3)));
        // D-g: per_trade_edge = ((1100 − 1000) + (1300 − 1000)) / (2 + 3) = 400 / 5 = 80.
        assert_eq!(ev.per_trade_edge, Some(dec!(80)));

        // Point 1 (worse neighbor): degradation floored at 0; zero Base trades → None edge.
        let ev1 = &agg.evidence[1];
        assert_eq!(ev1.neighbor_degradation, Decimal::ZERO);
        assert_eq!(ev1.per_trade_edge, None);
        assert_eq!(ev1.baseline_margin, dec!(0.10));
    }

    #[test]
    fn fee_sensitivity_matches_the_lf_recipe_exactly() {
        let (keys, results) = keys_and_results();
        let doubled_floors = vec![dec!(0.02), dec!(0.04)];
        let agg = aggregate_hf(
            &grids(),
            &keys,
            &results,
            2,
            &[],
            &doubled_floors,
            dec!(1000),
        );
        assert_eq!(agg.fee.len(), 2);
        let fs = &agg.fee[0];
        assert_eq!(fs.before_costs.total_return, dec!(0.30));
        assert_eq!(fs.base.total_return, dec!(0.20));
        assert_eq!(fs.base.turnover, dec!(3));
        assert_eq!(fs.base.n_trades, 5);
        assert_eq!(fs.base.fees_paid_quote, dec!(4));
        assert_eq!(fs.doubled.total_return, dec!(0.10));
        assert_eq!(fs.doubled.fees_paid_quote, dec!(6));
        assert_eq!(fs.return_drag_costs, dec!(0.10));
        assert_eq!(fs.return_drag_doubled, dec!(0.10));
        assert!(fs.survives_doubled); // 0.10 > mean(0.02, 0.04) = 0.03
    }

    #[test]
    fn rung_rollups_report_hf_metrics_and_base_rung_counter_sums() {
        let (keys, results) = keys_and_results();
        let agg = aggregate_hf(&grids(), &keys, &results, 2, &[], &[], dec!(1000));
        assert_eq!(agg.rungs.len(), 2);
        let r = &agg.rungs[0];

        assert_eq!(r.hot_congestion.scenario, ScenarioId::HotCongestion);
        assert_eq!(r.hot_congestion.total_return, dec!(0.13));
        assert_eq!(r.hot_congestion.turnover, dec!(4));
        assert_eq!(r.hot_congestion.n_trades, 3);
        assert_eq!(r.hot_congestion.fees_paid_quote, dec!(7));

        assert_eq!(r.adversarial_worst.scenario, ScenarioId::AdversarialWorst);
        assert_eq!(r.adversarial_worst.total_return, dec!(0.04));
        assert_eq!(r.adversarial_worst.fees_paid_quote, dec!(8));

        assert_eq!(r.latency_2x.scenario, ScenarioId::Latency2x);
        assert_eq!(r.latency_2x.total_return, dec!(0.09));
        assert_eq!(r.latency_2x.fees_paid_quote, dec!(4));

        // Counters = BASE-rung sums only (the other rungs' events must not leak in).
        assert_eq!(r.adverse_selection_hits, 3);
        assert_eq!(r.adverse_selection_paid_quote, dec!(2));
        assert_eq!(r.unlanded_orders, 5);
    }

    #[test]
    fn cost_drag_share_is_none_on_non_positive_gross() {
        let grids = vec![ParamGrid::ThresholdRebalance {
            target_sol_weights: vec![dec!(0.5)],
            bands: vec![dec!(0.1)],
        }];
        // Zero gross: BeforeCosts return exactly 0 → the <= 0 guard must yield None.
        let keys = vec![
            CellKey {
                window_index: 0,
                scenario: ScenarioId::BeforeCosts,
                point_index: 0,
            },
            CellKey {
                window_index: 0,
                scenario: ScenarioId::Base,
                point_index: 0,
            },
        ];
        let results = vec![
            hf_result(
                Decimal::ZERO,
                Decimal::ZERO,
                Decimal::ZERO,
                1,
                dec!(1000),
                dec!(0),
                0,
                dec!(0),
                0,
            ),
            hf_result(
                dec!(-0.10),
                Decimal::ZERO,
                Decimal::ZERO,
                1,
                dec!(900),
                dec!(1),
                0,
                dec!(0),
                0,
            ),
        ];
        let agg = aggregate_hf(&grids, &keys, &results, 1, &[], &[], dec!(1000));
        assert_eq!(agg.evidence[0].cost_drag_share, None);
        // Negative gross must also guard to None.
        let results_neg = vec![
            hf_result(
                dec!(-0.05),
                Decimal::ZERO,
                Decimal::ZERO,
                1,
                dec!(950),
                dec!(0),
                0,
                dec!(0),
                0,
            ),
            results[1].clone(),
        ];
        let agg = aggregate_hf(&grids, &keys, &results_neg, 1, &[], &[], dec!(1000));
        assert_eq!(agg.evidence[0].cost_drag_share, None);
    }

    #[test]
    fn per_trade_edge_is_none_on_zero_trades() {
        let grids = vec![ParamGrid::ThresholdRebalance {
            target_sol_weights: vec![dec!(0.5)],
            bands: vec![dec!(0.1)],
        }];
        let keys = vec![CellKey {
            window_index: 0,
            scenario: ScenarioId::Base,
            point_index: 0,
        }];
        let results = vec![hf_result(
            dec!(0.10),
            Decimal::ZERO,
            Decimal::ZERO,
            0,
            dec!(1100),
            dec!(0),
            0,
            dec!(0),
            0,
        )];
        let agg = aggregate_hf(&grids, &keys, &results, 1, &[], &[], dec!(1000));
        assert_eq!(agg.evidence[0].per_trade_edge, None);
    }

    #[test]
    fn per_trade_edge_can_be_negative_in_exact_decimal() {
        let grids = vec![ParamGrid::ThresholdRebalance {
            target_sol_weights: vec![dec!(0.5)],
            bands: vec![dec!(0.1)],
        }];
        let keys = vec![CellKey {
            window_index: 0,
            scenario: ScenarioId::Base,
            point_index: 0,
        }];
        let results = vec![hf_result(
            dec!(-0.025),
            Decimal::ZERO,
            Decimal::ZERO,
            4,
            dec!(975),
            dec!(0),
            0,
            dec!(0),
            0,
        )];
        let agg = aggregate_hf(&grids, &keys, &results, 1, &[], &[], dec!(1000));
        assert_eq!(agg.evidence[0].per_trade_edge, Some(dec!(-6.25)));
    }

    #[test]
    fn aggregation_is_deterministic_across_repeat_calls() {
        let (keys, results) = keys_and_results();
        let floors = vec![dec!(0.01), dec!(0.02)];
        let a = aggregate_hf(&grids(), &keys, &results, 2, &floors, &floors, dec!(1000));
        let b = aggregate_hf(&grids(), &keys, &results, 2, &floors, &floors, dec!(1000));
        assert_eq!(a, b);
    }
}
