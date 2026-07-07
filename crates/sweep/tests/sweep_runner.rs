//! M4-C5 gate: `run_sweep` end-to-end — the single orchestration entry point that serves all three
//! M4 gate criteria at once: parallel==sequential, repeated runs identical, and the holdout stays
//! sealed (never read) throughout.

use portfolio::CostModel;
use research_core::{Bar, Decimal, Timestamp};
use rust_decimal_macros::dec;
use sweep::{
    run_sweep, AdvancementThresholds, Parallelism, ParamGrid, PartitionSpec, SweepSpec,
    WalkForward, WindowKind,
};

const DAY: i64 = 86_400;

/// A deterministic, sorted, OHLC-valid synthetic SOL/USDC daily path (NOT real market data) —
/// the same sawtooth family used by `tests/holdout_sealing.rs`.
fn synthetic_series(n: usize) -> Vec<Bar> {
    (0..n)
        .map(|i| {
            let base = 100 + (i as i64 % 7) * 3 + (i as i64 / 7) * 2;
            let p = Decimal::from(base);
            Bar {
                ts: Timestamp::from_unix(i as i64 * DAY),
                open: p,
                high: p,
                low: p,
                close: p,
                volume: dec!(1000),
            }
        })
        .collect()
}

fn cost() -> CostModel {
    CostModel {
        dex_fee_bps: 5,
        slippage_bps: 20,
        base_fee_lamports: 5_000,
        priority_fee_lamports: 50_000,
    }
}

fn spec() -> SweepSpec {
    SweepSpec {
        allowlist_version: "test".to_string(),
        partition: PartitionSpec::by_index(100, 130),
        walk_forward: WalkForward::new(WindowKind::Rolling, 20, 10, 10, 0).unwrap(),
        thresholds: AdvancementThresholds {
            drawdown_budget: dec!(0.5),
            turnover_budget: dec!(50),
            baseline_margin: dec!(-1),
            dispersion_budget: dec!(10),
            neighbor_tolerance: dec!(10),
            min_windows: 1,
        },
        grids: vec![
            ParamGrid::TrendAlloc {
                sma_periods: vec![3, 5],
                weights_above: vec![dec!(0.75)],
                weights_below: vec![dec!(0)],
            },
            ParamGrid::ThresholdRebalance {
                target_sol_weights: vec![dec!(0.25), dec!(0.5)],
                bands: vec![dec!(0.05)],
            },
        ],
    }
}

#[test]
fn report_is_byte_identical_across_parallelism_and_repeats() {
    let spec = spec();
    let cost = cost();

    let seq1 = run_sweep(
        &spec,
        synthetic_series(160),
        &cost,
        dec!(10000),
        365.0,
        Parallelism::Sequential,
    )
    .unwrap();
    let seq2 = run_sweep(
        &spec,
        synthetic_series(160),
        &cost,
        dec!(10000),
        365.0,
        Parallelism::Sequential,
    )
    .unwrap();
    let threads2 = run_sweep(
        &spec,
        synthetic_series(160),
        &cost,
        dec!(10000),
        365.0,
        Parallelism::Threads(std::num::NonZeroUsize::new(2).unwrap()),
    )
    .unwrap();
    let threads8 = run_sweep(
        &spec,
        synthetic_series(160),
        &cost,
        dec!(10000),
        365.0,
        Parallelism::Threads(std::num::NonZeroUsize::new(8).unwrap()),
    )
    .unwrap();

    let baseline = seq1.report.to_json();
    assert_eq!(seq2.report.to_json(), baseline);
    assert_eq!(threads2.report.to_json(), baseline);
    assert_eq!(threads8.report.to_json(), baseline);
}

#[test]
fn holdout_is_never_read_and_survives_intact() {
    let spec = spec();
    let cost = cost();

    let run = |parallelism| {
        run_sweep(
            &spec,
            synthetic_series(160),
            &cost,
            dec!(10000),
            365.0,
            parallelism,
        )
        .unwrap()
    };

    let seq1 = run(Parallelism::Sequential);
    let seq2 = run(Parallelism::Sequential);
    let threads2 = run(Parallelism::Threads(
        std::num::NonZeroUsize::new(2).unwrap(),
    ));
    let threads8 = run(Parallelism::Threads(
        std::num::NonZeroUsize::new(8).unwrap(),
    ));

    for outcome in [&seq1, &seq2, &threads2, &threads8] {
        assert_eq!(outcome.sealed.holdout_read_count(), 0);
        assert_eq!(outcome.sealed.holdout_len(), 30);
    }
    assert_eq!(seq1.sealed.holdout_digest(), seq2.sealed.holdout_digest());
}

#[test]
fn every_candidate_gets_exactly_one_verdict() {
    let outcome = run_sweep(
        &spec(),
        synthetic_series(160),
        &cost(),
        dec!(10000),
        365.0,
        Parallelism::Sequential,
    )
    .unwrap();

    assert_eq!(outcome.report.verdicts.len(), 4);
    let labels: Vec<&str> = outcome
        .report
        .verdicts
        .iter()
        .map(|v| v.candidate_label.as_str())
        .collect();
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(labels, sorted, "verdicts must be sorted and unique");
}

#[test]
fn trial_count_is_windows_times_three_scenarios_times_points() {
    // Closes the C5 review gap: without this, narrowing the scenario set (ladder[..2]), counting
    // points instead of cells, or a window-count regression would survive the other four tests.
    let spec = spec();
    let outcome = run_sweep(
        &spec,
        synthetic_series(160),
        &cost(),
        dec!(10000),
        365.0,
        Parallelism::Sequential,
    )
    .unwrap();

    // Recompute the canonical cell count from the fixture itself: dev+val = 130 bars (the holdout
    // starts at index 130), exactly three cost scenarios (before_costs/base/doubled), every point.
    let n_windows = spec.walk_forward.windows(130).len();
    let n_points: usize = spec.grids.iter().map(|g| g.points().len()).sum();
    assert!(
        n_windows > 0 && n_points == 4,
        "fixture must be non-vacuous"
    );
    let expected = u32::try_from(n_windows * 3 * n_points).unwrap();
    assert_eq!(outcome.report.trial_count, expected);
}

#[test]
fn report_validates_against_schema() {
    let path = format!(
        "{}/../../schemas/sweep-report.schema.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let schema: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"));
    let validator =
        jsonschema::validator_for(&schema).expect("sweep-report schema is valid JSON Schema");

    let outcome = run_sweep(
        &spec(),
        synthetic_series(160),
        &cost(),
        dec!(10000),
        365.0,
        Parallelism::Sequential,
    )
    .unwrap();

    let value = outcome.report.to_value();
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "schema validation failed: {errors:?}");
}
