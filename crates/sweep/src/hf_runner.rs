//! `run_hf_sweep` — the windowed HF sweep capstone (m-hf-track row C8). Wires C8.1–C8.7e into
//! one deterministic entry: spec + source + provenance in, sealed holdout + schema-valid
//! `SweepReport` out. Mirrors `run_sweep`'s shape (`runner.rs:329-398`); a parallel path that
//! never touches the LF sweep.

use std::ops::Range;

use market_data::{ColumnarError, IntradaySource};
use portfolio::{HfError, LatencyPipeline};
use research_core::{intraday::Provenance, Bar, Decimal};

use crate::advance::evaluate_candidate;
use crate::baseline::BaselineId;
use crate::congestion::classify_congestion_regimes;
use crate::hf_aggregate::aggregate_hf;
use crate::hf_cell::{eval_hf_cell, eval_hf_strategy, HfCellResult};
use crate::hf_scenarios::hf_cost_scenarios;
use crate::hf_spec::HfSweepSpec;
use crate::intraday_partition::{IntradayPartitionError, IntradayPartitionedBars, IntradaySealed};
use crate::parallel::{run_in_parallel, Parallelism};
use crate::param::ParamPoint;
use crate::report::{CandidateMetricsDto, HfRungsDto, SweepReport};
use crate::runner::CellKey;

/// Everything an HF sweep produces. The seal is returned UNCONSUMED so callers can PROVE the
/// holdout was never read (`sealed.holdout_read_count() == 0`) — mirror of `SweepOutcome`'s doc,
/// `runner.rs:286-292`.
#[derive(Debug)]
pub struct HfSweepOutcome {
    pub report: SweepReport,
    pub sealed: IntradaySealed,
}

/// Why an HF sweep failed.
#[derive(Debug)]
pub enum HfSweepError {
    Columnar(ColumnarError),
    Partition(IntradayPartitionError),
    Hf(HfError),
}

impl std::fmt::Display for HfSweepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Columnar(e) => write!(f, "hf sweep source read failed: {e}"),
            Self::Partition(e) => write!(f, "hf sweep partitioning failed: {e}"),
            Self::Hf(e) => write!(f, "hf sweep simulation failed: {e}"),
        }
    }
}

impl std::error::Error for HfSweepError {}

impl From<ColumnarError> for HfSweepError {
    fn from(e: ColumnarError) -> Self {
        Self::Columnar(e)
    }
}

impl From<IntradayPartitionError> for HfSweepError {
    fn from(e: IntradayPartitionError) -> Self {
        Self::Partition(e)
    }
}

impl From<HfError> for HfSweepError {
    fn from(e: HfError) -> Self {
        Self::Hf(e)
    }
}

/// Candidate cell identity (decision D-d): global dev-val-relative, window index EXCLUDED so the
/// same physical slot gets the same landing outcome under any windowing.
fn hf_cell_id(scenario_index: usize, point_index: usize) -> u64 {
    ((scenario_index as u64) << 32) | point_index as u64
}

/// Baseline cell identity (decision D-d): disjoint tag bit from candidate cell ids.
fn hf_baseline_cell_id(scenario_index: usize, baseline_index: usize) -> u64 {
    0x8000_0000_0000_0000 | ((scenario_index as u64) << 32) | baseline_index as u64
}

/// One unit of HF sweep work: which window, which ladder rung, which candidate point.
///
/// `index` is the cell's canonical position, assigned at enumeration (mirror `SweepCell::index`,
/// `parallel.rs:33`) — read only by the enumeration-order test, hence `#[cfg(test)]`.
struct HfSweepCell {
    #[cfg(test)]
    index: usize,
    scenario_index: usize,
    point_index: usize,
    test: Range<usize>,
}

/// Materialize the full HF work list in canonical order: window (outer) → scenario → point
/// (inner). `cells[i].index == i` and `keys[i]` describes `cells[i]` (mirror `enumerate_cells`,
/// `runner.rs:32-58`).
fn enumerate_hf_cells(
    n_points: usize,
    ladder_len: usize,
    windows: &[crate::window::Window],
) -> (Vec<HfSweepCell>, Vec<CellKey>) {
    let n = windows.len() * ladder_len * n_points;
    let mut cells = Vec::with_capacity(n);
    let mut keys = Vec::with_capacity(n);
    for (wi, w) in windows.iter().enumerate() {
        for si in 0..ladder_len {
            for pi in 0..n_points {
                cells.push(HfSweepCell {
                    #[cfg(test)]
                    index: cells.len(),
                    scenario_index: si,
                    point_index: pi,
                    test: w.test.clone(),
                });
                keys.push(CellKey {
                    window_index: wi,
                    scenario: crate::sensitivity::ScenarioId::Base,
                    point_index: pi,
                });
            }
        }
    }
    (cells, keys)
}

/// Run the full deterministic windowed HF sweep: seal the holdout BEFORE any cell runs, evaluate
/// every `(window, ladder rung, candidate point)` cell through the shared priced core
/// (`eval_hf_cell`), re-score the four baselines under the Base/Doubled rungs, aggregate into
/// `CandidateEvidence`/`FeeSensitivity`/HF rung rollups, and assemble a schema-valid
/// `SweepReport`. `evaluate_intraday_on_holdout` is never called — the seal is returned
/// unconsumed.
///
/// # Errors
/// Returns [`HfSweepError`] if the source read, holdout partitioning, or any cell/baseline
/// simulation fails.
pub fn run_hf_sweep(
    spec: &HfSweepSpec,
    source: &dyn IntradaySource,
    provenance: &[Provenance],
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
    parallelism: Parallelism,
) -> Result<HfSweepOutcome, HfSweepError> {
    let bars: Vec<Bar> = source.slice(0..source.len())?;
    let (dev, sealed) = IntradayPartitionedBars::from_spec(bars, &spec.partition)?.seal_holdout();
    let windows = dev.walk_forward_windows(&spec.walk_forward);
    let ladder = hf_cost_scenarios(&spec.hf_cost, &spec.adversarial, &spec.latency);
    let regimes_all = classify_congestion_regimes(dev.dev_validation(), spec.congestion_lookback);
    let points: Vec<ParamPoint> = spec.grids.iter().flat_map(|g| g.points()).collect();
    let (cells, keys) = enumerate_hf_cells(points.len(), ladder.len(), &windows);

    let results: Vec<HfCellResult> = run_in_parallel(&cells, parallelism, |c| {
        let s = &ladder[c.scenario_index];
        let slice = &dev.dev_validation()[c.test.clone()];
        let cell_id = hf_cell_id(c.scenario_index, c.point_index);
        let landing = portfolio::build_landing_table(
            cell_id,
            c.test.start as u64..c.test.end as u64,
            s.latency.offset_bars,
            s.latency.p_land_num,
            s.latency.p_land_den,
        )?;
        let pipeline = LatencyPipeline::new(s.latency.offset_bars, landing)?;
        let regimes = match s.force_regime {
            Some(r) => vec![r; slice.len()],
            None => regimes_all[c.test.clone()].to_vec(),
        };
        eval_hf_cell(
            &points[c.point_index],
            slice,
            &s.hf,
            &s.adversarial,
            &pipeline,
            &regimes,
            cell_id,
            initial_cash_usdc,
            periods_per_year,
        )
        .map_err(HfSweepError::from)
    })
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;

    let mut base_floors = Vec::with_capacity(windows.len());
    let mut doubled_floors = Vec::with_capacity(windows.len());
    for w in &windows {
        let slice = &dev.dev_validation()[w.test.clone()];
        for (rung_index, floors) in [(1usize, &mut base_floors), (2usize, &mut doubled_floors)] {
            let s = &ladder[rung_index];
            let landing = portfolio::build_landing_table(
                hf_baseline_cell_id(rung_index, 0),
                w.test.start as u64..w.test.end as u64,
                s.latency.offset_bars,
                s.latency.p_land_num,
                s.latency.p_land_den,
            )?;
            let pipeline = LatencyPipeline::new(s.latency.offset_bars, landing)?;
            let regimes = match s.force_regime {
                Some(r) => vec![r; slice.len()],
                None => regimes_all[w.test.clone()].to_vec(),
            };
            let mut best: Option<Decimal> = None;
            for (baseline_index, id) in BaselineId::all().into_iter().enumerate() {
                let cell_id = hf_baseline_cell_id(rung_index, baseline_index);
                let result = eval_hf_strategy(
                    &*id.build(),
                    slice,
                    &s.hf,
                    &s.adversarial,
                    &pipeline,
                    &regimes,
                    cell_id,
                    initial_cash_usdc,
                    periods_per_year,
                )?;
                let ret = result.cell.total_return;
                best = Some(best.map_or(ret, |b| b.max(ret)));
            }
            floors.push(best.unwrap_or(Decimal::ZERO));
        }
    }

    let agg = aggregate_hf(
        &spec.grids,
        &keys,
        &results,
        windows.len(),
        &base_floors,
        &doubled_floors,
        initial_cash_usdc,
    );

    let candidates: Vec<CandidateMetricsDto> = agg
        .evidence
        .iter()
        .zip(&agg.fee)
        .zip(&agg.rungs)
        .map(|((ev, fs), r)| {
            CandidateMetricsDto::new(ev.candidate_label.clone(), ev.max_drawdown, ev.turnover, fs)
                .with_hf(
                    HfRungsDto::new(
                        &r.hot_congestion,
                        &r.adversarial_worst,
                        &r.latency_2x,
                        r.adverse_selection_hits,
                        r.adverse_selection_paid_quote,
                        r.unlanded_orders,
                    ),
                    ev.cost_drag_share,
                    ev.per_trade_edge,
                )
        })
        .collect();
    let verdicts = agg
        .evidence
        .iter()
        .map(|ev| evaluate_candidate(ev, &spec.thresholds))
        .collect();
    let trial_count = u32::try_from(cells.len()).unwrap_or(u32::MAX);
    let report = SweepReport::new(&spec.thresholds, trial_count, verdicts, candidates)
        .with_data_provenance(provenance);
    Ok(HfSweepOutcome { report, sealed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PartitionSpec;
    use crate::hf_scenarios::HfLatencyParams;
    use crate::param::ParamGrid;
    use crate::window::WindowKind;
    use market_data::synthetic::{generate, SyntheticSpec};
    use portfolio::{
        AdversarialModel, CongestionPriorityTable, DepthBand, DepthCurve, HfCostModel,
    };
    use rust_decimal_macros::dec;

    fn hf_model() -> HfCostModel {
        HfCostModel {
            base: portfolio::CostModel {
                dex_fee_bps: 5,
                slippage_bps: 20,
                base_fee_lamports: 5_000,
                priority_fee_lamports: 50_000,
            },
            depth_curve: DepthCurve::new(vec![DepthBand {
                notional_upto: dec!(1_000_000),
                impact_bps: 15,
            }])
            .unwrap(),
            congestion_priority_table: CongestionPriorityTable::new(1_000, 2_000, 3_000).unwrap(),
            tip_bps: 5,
        }
    }

    fn zero_adversarial() -> AdversarialModel {
        AdversarialModel {
            sandwich_bps: 0,
            pickoff_bps: 0,
            p_adverse_num: 0,
            p_adverse_den: 1,
        }
    }

    fn thresholds() -> crate::advance::AdvancementThresholds {
        crate::advance::AdvancementThresholds {
            drawdown_budget: dec!(0.90),
            turnover_budget: None,
            baseline_margin: dec!(-1.0),
            dispersion_budget: dec!(1.0),
            neighbor_tolerance: dec!(1.0),
            min_windows: 1,
            cost_drag_share_ceiling: None,
            per_trade_edge_floor: None,
        }
    }

    fn tiny_spec() -> HfSweepSpec {
        HfSweepSpec {
            allowlist_version: "test".to_string(),
            partition: PartitionSpec::by_index(40, 180),
            walk_forward: crate::window::WalkForward::new(WindowKind::Rolling, 20, 20, 20, 0)
                .unwrap(),
            thresholds: thresholds(),
            grids: vec![ParamGrid::ThresholdRebalance {
                target_sol_weights: vec![dec!(0.5)],
                bands: vec![dec!(0)],
            }],
            resolution_secs: 1,
            max_lookback_bars: 5,
            hf_cost: hf_model(),
            latency: HfLatencyParams {
                offset_bars: 1,
                p_land_num: 9,
                p_land_den: 10,
            },
            adversarial: zero_adversarial(),
            congestion_lookback: 10,
        }
    }

    fn synthetic_bars(n: usize) -> Vec<Bar> {
        let spec = SyntheticSpec {
            seed: 7,
            steps: n,
            start_unix: 1_609_459_200,
            start_slot: 100_000,
            venue: "venue_a".to_string(),
            mid0: dec!(100),
            anchor: dec!(100),
            reversion: dec!(0.05),
            vol_step: dec!(0.2),
            impact_every: 50,
            impact_size: dec!(1.5),
            impact_decay: dec!(0.5),
            regime_period: 100,
        };
        generate(&spec).unwrap().bars_1s
    }

    /// A trivial in-memory [`IntradaySource`] for tests: `ColumnarFile` is the only production
    /// impl, but the entry point only needs the trait.
    struct VecSource(Vec<Bar>);

    impl IntradaySource for VecSource {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn slice(&self, range: Range<usize>) -> Result<Vec<Bar>, ColumnarError> {
            if range.end > self.0.len() {
                return Err(ColumnarError::SliceOutOfBounds {
                    requested_end: range.end,
                    len: self.0.len(),
                });
            }
            Ok(self.0[range].to_vec())
        }
    }

    #[test]
    fn enumerate_hf_cells_is_canonical_order_with_stable_index() {
        let windows = vec![
            crate::window::Window {
                train: 0..10,
                test: 10..20,
            },
            crate::window::Window {
                train: 0..20,
                test: 20..30,
            },
        ];
        let (cells, keys) = enumerate_hf_cells(3, 6, &windows);
        assert_eq!(cells.len(), 2 * 6 * 3);
        assert_eq!(keys.len(), cells.len());
        for (i, c) in cells.iter().enumerate() {
            assert_eq!(c.index, i);
        }
        // window (outer) -> scenario -> point (inner): first cell is window 0, scenario 0, point 0.
        assert_eq!(cells[0].scenario_index, 0);
        assert_eq!(cells[0].point_index, 0);
        // last cell of window 0 is scenario 5, point 2.
        assert_eq!(cells[6 * 3 - 1].scenario_index, 5);
        assert_eq!(cells[6 * 3 - 1].point_index, 2);
        // first cell of window 1 starts scenario/point back at 0.
        assert_eq!(cells[6 * 3].scenario_index, 0);
        assert_eq!(cells[6 * 3].point_index, 0);
        assert_eq!(keys[6 * 3].window_index, 1);
    }

    #[test]
    fn candidate_and_baseline_cell_ids_are_disjoint_by_tag_bit() {
        let candidate = hf_cell_id(3, 7);
        let baseline = hf_baseline_cell_id(3, 7);
        assert_eq!(candidate & 0x8000_0000_0000_0000, 0);
        assert_eq!(baseline & 0x8000_0000_0000_0000, 0x8000_0000_0000_0000);
        assert_ne!(candidate, baseline);
    }

    #[test]
    fn two_window_smoke_sequential_equals_threads_and_holdout_untouched() {
        let bars = synthetic_bars(200);
        let source = VecSource(bars);
        let spec = tiny_spec();
        let provenance = vec![];

        let seq = run_hf_sweep(
            &spec,
            &source,
            &provenance,
            dec!(10_000),
            365.0,
            Parallelism::Sequential,
        )
        .unwrap();
        let par = run_hf_sweep(
            &spec,
            &source,
            &provenance,
            dec!(10_000),
            365.0,
            Parallelism::Threads(std::num::NonZeroUsize::new(2).unwrap()),
        )
        .unwrap();

        assert_eq!(seq.report.to_json(), par.report.to_json());
        assert_eq!(seq.sealed.holdout_read_count(), 0);
        assert_eq!(par.sealed.holdout_read_count(), 0);
    }
}
