//! Sweep orchestration (M4 cards C3–C5): canonical enumeration → parallel evaluation →
//! evidence aggregation → SweepReport. Deterministic by construction: the cell order is fixed
//! before any thread spawns (m4-sweep.md §5: window → cost-scenario → param order).

use crate::advance::{evaluate_candidate, CandidateEvidence};
use crate::baseline::{best_baseline_return, eval_all_baselines};
use crate::cell::CellResult;
use crate::parallel::{run_cells, Parallelism, SweepCell};
use crate::param::{ParamGrid, ParamPoint};
use crate::partition::{PartitionError, PartitionedBars, Sealed};
use crate::report::SweepReport;
use crate::sensitivity::{
    cost_scenarios, scale_cost_model, CostScenario, FeeSensitivity, ScenarioId, ScenarioMetrics,
};
use crate::spec::SweepSpec;
use crate::window::Window;
use portfolio::{CostModel, SimError};
use research_core::{Bar, Decimal};

/// Where a cell sits in the canonical enumeration. `point_index` indexes the caller's
/// concatenated `points` list (grids in fixed family order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellKey {
    pub window_index: usize,
    pub scenario: ScenarioId,
    pub point_index: usize,
}

/// Materialize the full work list in canonical order: window (outer) → scenario → point (inner).
/// `cells[i].index == i` and `keys[i]` describes `cells[i]`.
#[must_use]
pub fn enumerate_cells(
    points: &[ParamPoint],
    scenarios: &[CostScenario],
    windows: &[Window],
) -> (Vec<SweepCell>, Vec<CellKey>) {
    let n = windows.len() * scenarios.len() * points.len();
    let mut cells = Vec::with_capacity(n);
    let mut keys = Vec::with_capacity(n);
    for (wi, w) in windows.iter().enumerate() {
        for s in scenarios {
            for (pi, p) in points.iter().enumerate() {
                cells.push(SweepCell {
                    index: cells.len(),
                    point: p.clone(),
                    cost: s.cost.clone(),
                    test: w.test.clone(),
                });
                keys.push(CellKey {
                    window_index: wi,
                    scenario: s.id,
                    point_index: pi,
                });
            }
        }
    }
    (cells, keys)
}

/// Aggregate raw cell results into one CandidateEvidence per param point.
///
/// Definitions (all exact Decimal; f64 never enters):
/// - candidate returns come from Base-scenario cells; means divide by the window count;
/// - fold_dispersion = max − min of the Base-scenario window returns (0 with < 2 windows);
/// - max_drawdown / turnover = the WORST (max) across windows (conservative);
/// - baseline_margin = mean(Base returns) − mean(base_floors);
/// - doubled_return = mean(Doubled-scenario returns); doubled_baseline_floor = mean(doubled_floors);
/// - neighbor_degradation = max over axis-neighbors of (own mean − neighbor mean), floored at 0;
///   axis-neighbors differ by exactly one position on exactly one grid axis;
/// - valid_windows = the window count (run_cells is total — any SimError aborts the sweep).
#[must_use]
pub fn aggregate_evidence(
    grids: &[ParamGrid],
    keys: &[CellKey],
    results: &[CellResult],
    n_windows: usize,
    base_floors: &[Decimal], // per-window best cost-matched baseline return, Base costs
    doubled_floors: &[Decimal], // per-window best cost-matched baseline return, Doubled costs
) -> Vec<CandidateEvidence> {
    let points: Vec<ParamPoint> = grids.iter().flat_map(|g| g.points()).collect();

    let mut base_returns: Vec<Vec<Decimal>> = vec![Vec::new(); points.len()];
    let mut doubled_returns: Vec<Vec<Decimal>> = vec![Vec::new(); points.len()];
    let mut max_drawdown: Vec<Decimal> = vec![Decimal::ZERO; points.len()];
    let mut turnover: Vec<Decimal> = vec![Decimal::ZERO; points.len()];
    for (k, r) in keys.iter().zip(results) {
        match k.scenario {
            ScenarioId::Base => {
                base_returns[k.point_index].push(r.total_return);
                max_drawdown[k.point_index] = max_drawdown[k.point_index].max(r.max_drawdown);
                turnover[k.point_index] = turnover[k.point_index].max(r.turnover);
            }
            ScenarioId::Doubled => {
                doubled_returns[k.point_index].push(r.total_return);
            }
            _ => {}
        }
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
    let base_floor_mean = mean(base_floors);
    let doubled_floor_mean = mean(doubled_floors);

    let means: Vec<Decimal> = base_returns.iter().map(|xs| mean(xs)).collect();

    let valid_windows = u32::try_from(n_windows).unwrap_or(u32::MAX);

    points
        .iter()
        .enumerate()
        .map(|(pi, point)| CandidateEvidence {
            candidate_label: format!("{}/{}", point.family(), point.param_id()),
            max_drawdown: max_drawdown[pi],
            turnover: turnover[pi],
            baseline_margin: means[pi] - base_floor_mean,
            doubled_return: mean(&doubled_returns[pi]),
            doubled_baseline_floor: doubled_floor_mean,
            fold_dispersion: spread(&base_returns[pi]),
            neighbor_degradation: neighbor_indices(grids, pi)
                .into_iter()
                .map(|ni| means[pi] - means[ni])
                .max()
                .unwrap_or(Decimal::ZERO)
                .max(Decimal::ZERO),
            valid_windows,
        })
        .collect()
}

/// Per-candidate aggregated fee-sensitivity blocks, in candidate (point) order — the reportable
/// projection of the sweep (m4-sweep.md §9). Aggregation across windows, all exact Decimal:
/// total_return = mean (matching `aggregate_evidence`); turnover = max (worst window);
/// n_trades / fees / slippage / priority fees = sums. `survives_floor` = mean of the per-window
/// doubled-cost baseline floors — the same value `aggregate_evidence` records as
/// `doubled_baseline_floor`, so `survives_doubled` cannot drift from the edge-vanishes verdict.
#[must_use]
pub fn aggregate_fee_sensitivity(
    grids: &[ParamGrid],
    keys: &[CellKey],
    results: &[CellResult],
    doubled_floors: &[Decimal],
) -> Vec<FeeSensitivity> {
    let n_points: usize = grids.iter().map(|g| g.points().len()).sum();

    #[derive(Clone, Default)]
    struct Acc {
        returns: Vec<Decimal>,
        turnover: Decimal,
        n_trades: u32,
        fees: Decimal,
        slippage: Decimal,
        priority: Decimal,
    }
    let mut acc: Vec<[Acc; 3]> = (0..n_points).map(|_| Default::default()).collect();
    for (k, r) in keys.iter().zip(results) {
        let slot = match k.scenario {
            ScenarioId::BeforeCosts => 0,
            ScenarioId::Base => 1,
            ScenarioId::Doubled => 2,
            _ => continue,
        };
        let a = &mut acc[k.point_index][slot];
        a.returns.push(r.total_return);
        a.turnover = a.turnover.max(r.turnover);
        a.n_trades += r.n_trades;
        a.fees += r.fees_paid_quote;
        a.slippage += r.slippage_paid_quote;
        a.priority += r.priority_fees_paid_sol;
    }

    let mean = |xs: &[Decimal]| {
        if xs.is_empty() {
            Decimal::ZERO
        } else {
            xs.iter().copied().sum::<Decimal>() / Decimal::from(xs.len() as u64)
        }
    };
    let survives_floor = mean(doubled_floors);
    let metrics = |scenario: ScenarioId, a: &Acc| ScenarioMetrics {
        scenario,
        total_return: mean(&a.returns),
        turnover: a.turnover,
        n_trades: a.n_trades,
        fees_paid_quote: a.fees,
        slippage_paid_quote: a.slippage,
        priority_fees_paid_sol: a.priority,
    };
    acc.iter()
        .map(|slots| {
            FeeSensitivity::from_scenarios(
                metrics(ScenarioId::BeforeCosts, &slots[0]),
                metrics(ScenarioId::Base, &slots[1]),
                metrics(ScenarioId::Doubled, &slots[2]),
                survives_floor,
            )
        })
        .collect()
}

/// Concatenated-list indices of `pi`'s axis neighbors (±1 on exactly one grid axis).
fn neighbor_indices(grids: &[ParamGrid], pi: usize) -> Vec<usize> {
    let mut offset = 0;
    for g in grids {
        let len = g.points().len();
        if pi < offset + len {
            return in_grid_neighbors(g, pi - offset)
                .into_iter()
                .map(|n| n + offset)
                .collect();
        }
        offset += len;
    }
    Vec::new()
}

/// Axis neighbors of the in-grid row-major index `local` (last axis fastest).
fn in_grid_neighbors(grid: &ParamGrid, local: usize) -> Vec<usize> {
    let dims: Vec<usize> = match grid {
        ParamGrid::TrendAlloc {
            sma_periods,
            weights_above,
            weights_below,
        } => {
            vec![sma_periods.len(), weights_above.len(), weights_below.len()]
        }
        ParamGrid::ThresholdRebalance {
            target_sol_weights,
            bands,
        } => {
            vec![target_sol_weights.len(), bands.len()]
        }
    };
    let mut idx = vec![0usize; dims.len()];
    let mut rem = local;
    for a in (0..dims.len()).rev() {
        idx[a] = rem % dims[a];
        rem /= dims[a];
    }
    let compose = |ix: &[usize]| ix.iter().zip(&dims).fold(0, |acc, (i, d)| acc * d + i);
    let mut out = Vec::new();
    for a in 0..dims.len() {
        if idx[a] > 0 {
            let mut nix = idx.clone();
            nix[a] -= 1;
            out.push(compose(&nix));
        }
        if idx[a] + 1 < dims[a] {
            let mut nix = idx.clone();
            nix[a] += 1;
            out.push(compose(&nix));
        }
    }
    out
}

/// Everything a sweep produces. The seal is returned UNCONSUMED so callers can PROVE the holdout
/// was never read (`sealed.holdout_read_count() == 0`). M4 code must never consume it.
#[derive(Debug)]
pub struct SweepOutcome {
    pub report: SweepReport,
    pub sealed: Sealed,
}

/// Why a sweep failed.
#[derive(Debug)]
pub enum SweepError {
    Partition(PartitionError),
    Sim(SimError),
}

impl std::fmt::Display for SweepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Partition(e) => write!(f, "sweep partitioning failed: {e}"),
            Self::Sim(e) => write!(f, "sweep simulation failed: {e}"),
        }
    }
}

impl std::error::Error for SweepError {}

impl From<PartitionError> for SweepError {
    fn from(e: PartitionError) -> Self {
        Self::Partition(e)
    }
}

impl From<SimError> for SweepError {
    fn from(e: SimError) -> Self {
        Self::Sim(e)
    }
}

/// Run the full deterministic sweep. Scenario set = the first three rungs of the cost ladder:
/// BeforeCosts, Base, Doubled (m4-sweep.md §13). `trial_count` = total cells evaluated.
///
/// # Errors
/// Returns [`SweepError`] if partitioning or any cell/baseline simulation fails.
pub fn run_sweep(
    spec: &SweepSpec,
    bars: Vec<Bar>,
    base_cost: &CostModel,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
    parallelism: Parallelism,
) -> Result<SweepOutcome, SweepError> {
    let (dev, sealed) = PartitionedBars::from_spec(bars, &spec.partition)?.seal_holdout();
    let windows = dev.walk_forward_windows(&spec.walk_forward);
    let ladder = cost_scenarios(base_cost);
    // BeforeCosts, Base, Doubled — ladder order is test-pinned. BeforeCosts cells are evaluated
    // and counted in trial_count but not read by aggregate_evidence: they are the "same strategy
    // before costs" sensitivity rung m4-sweep.md §13 mandates, kept for M5's FeeSensitivity use
    // and honest multiple-testing accounting. Deliberate — do not narrow to [1..3].
    let scenarios = &ladder[..3];
    let points: Vec<ParamPoint> = spec.grids.iter().flat_map(|g| g.points()).collect();
    let (cells, keys) = enumerate_cells(&points, scenarios, &windows);
    let results = run_cells(
        &cells,
        dev.dev_validation(),
        initial_cash_usdc,
        periods_per_year,
        parallelism,
    )?;
    let doubled = scale_cost_model(base_cost, 2, 1);
    let mut base_floors = Vec::with_capacity(windows.len());
    let mut doubled_floors = Vec::with_capacity(windows.len());
    for w in &windows {
        let slice = &dev.dev_validation()[w.test.clone()];
        base_floors.push(best_baseline_return(&eval_all_baselines(
            slice,
            base_cost,
            initial_cash_usdc,
            periods_per_year,
        )?));
        doubled_floors.push(best_baseline_return(&eval_all_baselines(
            slice,
            &doubled,
            initial_cash_usdc,
            periods_per_year,
        )?));
    }
    let evidence = aggregate_evidence(
        &spec.grids,
        &keys,
        &results,
        windows.len(),
        &base_floors,
        &doubled_floors,
    );
    let verdicts = evidence
        .iter()
        .map(|ev| evaluate_candidate(ev, &spec.thresholds))
        .collect();
    let trial_count = u32::try_from(cells.len()).unwrap_or(u32::MAX);
    Ok(SweepOutcome {
        report: SweepReport::new(&spec.thresholds, trial_count, verdicts),
        sealed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param::ParamGrid;
    use crate::sensitivity::cost_scenarios;
    use portfolio::CostModel;
    use rust_decimal_macros::dec;

    fn windows() -> Vec<Window> {
        vec![
            Window {
                train: 0..4,
                test: 5..8,
            },
            Window {
                train: 3..7,
                test: 8..11,
            },
        ]
    }

    fn scenarios() -> Vec<CostScenario> {
        let base = CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        };
        cost_scenarios(&base)[..2].to_vec()
    }

    fn points() -> Vec<ParamPoint> {
        ParamGrid::ThresholdRebalance {
            target_sol_weights: vec![dec!(0.25), dec!(0.5)],
            bands: vec![dec!(0.1)],
        }
        .points()
    }

    #[test]
    fn enumeration_is_canonical_window_scenario_point_order() {
        let windows = windows();
        let scenarios = scenarios();
        let points = points();
        let (cells, keys) = enumerate_cells(&points, &scenarios, &windows);

        assert_eq!(cells.len(), 8);
        assert_eq!(keys.len(), 8);
        for (i, cell) in cells.iter().enumerate() {
            assert_eq!(cell.index, i);
        }

        let expected = [
            (0, ScenarioId::BeforeCosts, 0),
            (0, ScenarioId::BeforeCosts, 1),
            (0, ScenarioId::Base, 0),
            (0, ScenarioId::Base, 1),
            (1, ScenarioId::BeforeCosts, 0),
            (1, ScenarioId::BeforeCosts, 1),
            (1, ScenarioId::Base, 0),
            (1, ScenarioId::Base, 1),
        ];
        for (key, (window_index, scenario, point_index)) in keys.iter().zip(expected) {
            assert_eq!(key.window_index, window_index);
            assert_eq!(key.scenario, scenario);
            assert_eq!(key.point_index, point_index);
        }

        for (cell, key) in cells.iter().zip(&keys) {
            let window = &windows[key.window_index];
            let scenario = &scenarios[match key.scenario {
                ScenarioId::BeforeCosts => 0,
                ScenarioId::Base => 1,
                _ => unreachable!(),
            }];
            assert_eq!(cell.test, window.test);
            assert_eq!(cell.cost, scenario.cost);
        }
    }

    fn cell_result(total_return: Decimal, max_drawdown: Decimal, turnover: Decimal) -> CellResult {
        CellResult {
            total_return,
            max_drawdown,
            turnover,
            n_trades: 0,
            final_equity: Decimal::ZERO,
            traded_notional_quote: Decimal::ZERO,
            fees_paid_quote: Decimal::ZERO,
            slippage_paid_quote: Decimal::ZERO,
            priority_fees_paid_sol: Decimal::ZERO,
            time_in_market: Decimal::ZERO,
            cagr: None,
            volatility: None,
            sharpe: None,
            sortino: None,
            calmar: None,
        }
    }

    #[test]
    fn evidence_means_and_dispersion_are_exact() {
        let grids = vec![ParamGrid::ThresholdRebalance {
            target_sol_weights: vec![dec!(0.5)],
            bands: vec![dec!(0.1)],
        }];
        let keys = vec![
            CellKey {
                window_index: 0,
                scenario: ScenarioId::Base,
                point_index: 0,
            },
            CellKey {
                window_index: 0,
                scenario: ScenarioId::Doubled,
                point_index: 0,
            },
            CellKey {
                window_index: 1,
                scenario: ScenarioId::Base,
                point_index: 0,
            },
            CellKey {
                window_index: 1,
                scenario: ScenarioId::Doubled,
                point_index: 0,
            },
        ];
        let results = vec![
            cell_result(dec!(0.10), dec!(0.1), dec!(1)),
            cell_result(dec!(0.05), Decimal::ZERO, Decimal::ZERO),
            cell_result(dec!(0.30), dec!(0.4), dec!(3)),
            cell_result(dec!(0.15), Decimal::ZERO, Decimal::ZERO),
        ];
        let base_floors = vec![dec!(0.05), dec!(0.05)];
        let doubled_floors = vec![Decimal::ZERO, Decimal::ZERO];

        let evidence =
            aggregate_evidence(&grids, &keys, &results, 2, &base_floors, &doubled_floors);
        assert_eq!(evidence.len(), 1);
        let ev = &evidence[0];
        let expected_label = format!(
            "{}/{}",
            grids[0].points()[0].family(),
            grids[0].points()[0].param_id()
        );
        assert_eq!(ev.candidate_label, expected_label);
        assert_eq!(ev.max_drawdown, dec!(0.4));
        assert_eq!(ev.turnover, dec!(3));
        assert_eq!(ev.baseline_margin, dec!(0.15));
        assert_eq!(ev.doubled_return, dec!(0.1));
        assert_eq!(ev.doubled_baseline_floor, Decimal::ZERO);
        assert_eq!(ev.fold_dispersion, dec!(0.2));
        assert_eq!(ev.neighbor_degradation, Decimal::ZERO);
        assert_eq!(ev.valid_windows, 2);
    }

    #[test]
    fn neighbor_degradation_picks_the_worst_axis_neighbor() {
        let grids = vec![ParamGrid::ThresholdRebalance {
            target_sol_weights: vec![dec!(0.25), dec!(0.5), dec!(0.75)],
            bands: vec![dec!(0.1)],
        }];
        let returns = [dec!(0.1), dec!(0.5), dec!(0.2)];
        let keys: Vec<CellKey> = (0..3)
            .map(|pi| CellKey {
                window_index: 0,
                scenario: ScenarioId::Base,
                point_index: pi,
            })
            .collect();
        let results: Vec<CellResult> = returns
            .iter()
            .map(|&r| cell_result(r, Decimal::ZERO, Decimal::ZERO))
            .collect();

        let evidence = aggregate_evidence(&grids, &keys, &results, 1, &[], &[]);
        assert_eq!(evidence.len(), 3);
        assert_eq!(evidence[0].neighbor_degradation, Decimal::ZERO);
        assert_eq!(evidence[1].neighbor_degradation, dec!(0.4));
        assert_eq!(evidence[2].neighbor_degradation, Decimal::ZERO);
    }

    #[test]
    fn empty_windows_yield_zeroed_insufficient_evidence() {
        let grids = vec![ParamGrid::ThresholdRebalance {
            target_sol_weights: vec![dec!(0.5)],
            bands: vec![dec!(0.1)],
        }];
        let evidence = aggregate_evidence(&grids, &[], &[], 0, &[], &[]);
        assert_eq!(evidence.len(), 1);
        let ev = &evidence[0];
        assert_eq!(ev.max_drawdown, Decimal::ZERO);
        assert_eq!(ev.turnover, Decimal::ZERO);
        assert_eq!(ev.baseline_margin, Decimal::ZERO);
        assert_eq!(ev.doubled_return, Decimal::ZERO);
        assert_eq!(ev.doubled_baseline_floor, Decimal::ZERO);
        assert_eq!(ev.fold_dispersion, Decimal::ZERO);
        assert_eq!(ev.neighbor_degradation, Decimal::ZERO);
        assert_eq!(ev.valid_windows, 0);
    }

    fn cell_result_full(
        total_return: Decimal,
        turnover: Decimal,
        n_trades: u32,
        fees: Decimal,
    ) -> CellResult {
        CellResult {
            total_return,
            max_drawdown: Decimal::ZERO,
            turnover,
            n_trades,
            final_equity: Decimal::ZERO,
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
        }
    }

    #[test]
    fn fee_sensitivity_aggregation_is_exact() {
        let grids = vec![ParamGrid::ThresholdRebalance {
            target_sol_weights: vec![dec!(0.5)],
            bands: vec![dec!(0.1)],
        }];
        let mut keys = Vec::new();
        let mut results = Vec::new();
        // Window 0: before_costs, base, doubled.
        keys.push(CellKey {
            window_index: 0,
            scenario: ScenarioId::BeforeCosts,
            point_index: 0,
        });
        results.push(cell_result_full(dec!(0.20), dec!(1), 2, dec!(0)));
        keys.push(CellKey {
            window_index: 0,
            scenario: ScenarioId::Base,
            point_index: 0,
        });
        results.push(cell_result_full(dec!(0.10), dec!(2), 2, dec!(1)));
        keys.push(CellKey {
            window_index: 0,
            scenario: ScenarioId::Doubled,
            point_index: 0,
        });
        results.push(cell_result_full(dec!(0.05), dec!(3), 2, dec!(2)));
        // Window 1: before_costs, base, doubled.
        keys.push(CellKey {
            window_index: 1,
            scenario: ScenarioId::BeforeCosts,
            point_index: 0,
        });
        results.push(cell_result_full(dec!(0.40), dec!(4), 3, dec!(0)));
        keys.push(CellKey {
            window_index: 1,
            scenario: ScenarioId::Base,
            point_index: 0,
        });
        results.push(cell_result_full(dec!(0.30), dec!(1), 3, dec!(3)));
        keys.push(CellKey {
            window_index: 1,
            scenario: ScenarioId::Doubled,
            point_index: 0,
        });
        results.push(cell_result_full(dec!(0.15), dec!(2), 3, dec!(4)));

        let doubled_floors = vec![dec!(0.05), dec!(0.05)];
        let fee = aggregate_fee_sensitivity(&grids, &keys, &results, &doubled_floors);
        assert_eq!(fee.len(), 1);
        let fs = &fee[0];

        assert_eq!(fs.before_costs.total_return, dec!(0.3));
        assert_eq!(fs.before_costs.turnover, dec!(4));
        assert_eq!(fs.before_costs.n_trades, 5);
        assert_eq!(fs.before_costs.fees_paid_quote, dec!(0));

        assert_eq!(fs.base.total_return, dec!(0.2));
        assert_eq!(fs.base.turnover, dec!(2));
        assert_eq!(fs.base.n_trades, 5);
        assert_eq!(fs.base.fees_paid_quote, dec!(4));

        assert_eq!(fs.doubled.total_return, dec!(0.1));
        assert_eq!(fs.doubled.turnover, dec!(3));
        assert_eq!(fs.doubled.n_trades, 5);
        assert_eq!(fs.doubled.fees_paid_quote, dec!(6));

        assert_eq!(fs.return_drag_doubled, dec!(0.1));
        assert_eq!(fs.return_drag_costs, dec!(0.1));
        assert_eq!(fs.survives_doubled, dec!(0.1) > dec!(0.05));
    }
}
