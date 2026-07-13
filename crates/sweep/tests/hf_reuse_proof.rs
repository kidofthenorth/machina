//! Reuse proof (card M-HF-C2.5, m-hf-track.md §1/§5 row C2.5): 1s bars need no new bar type — a
//! 1s signal executing at the next 1s bar's open IS next-bar execution at finer grain. This test
//! proves the existing engine runs on 1s bars with **ZERO source changes**: `run_sweep`,
//! `SweepSpec`, `WalkForward`, and `ParamGrid` are exercised exactly as M4 shipped them, over
//! `market_data::synthetic::generate`'s (card M-HF-C2) 1-second series. If any assertion here
//! ever needs a `src/` change to pass, the reuse-first bet is wrong.

use market_data::synthetic::{generate, SyntheticSpec};
use portfolio::CostModel;
use research_core::Bar;
use rust_decimal_macros::dec;
use sweep::{
    run_sweep, AdvancementThresholds, Parallelism, ParamGrid, PartitionSpec, SweepSpec,
    WalkForward, WindowKind,
};

fn hf_bars() -> Vec<Bar> {
    let spec = SyntheticSpec {
        seed: 7,
        steps: 4_000,
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

fn hf_spec() -> SweepSpec {
    SweepSpec {
        allowlist_version: "2026-06-29".to_string(),
        partition: PartitionSpec::by_index(3_000, 3_600),
        walk_forward: WalkForward::new(WindowKind::Rolling, 900, 300, 300, 5).unwrap(),
        thresholds: AdvancementThresholds {
            drawdown_budget: dec!(0.35),
            turnover_budget: dec!(12),
            baseline_margin: dec!(0.02),
            dispersion_budget: dec!(0.40),
            neighbor_tolerance: dec!(0.15),
            min_windows: 6,
        },
        grids: vec![
            ParamGrid::TrendAlloc {
                sma_periods: vec![20, 50],
                weights_above: vec![dec!(0.75), dec!(0.5)],
                weights_below: vec![dec!(0)],
            },
            ParamGrid::ThresholdRebalance {
                target_sol_weights: vec![dec!(0.25), dec!(0.5)],
                bands: vec![dec!(0.05), dec!(0.1)],
            },
        ],
    }
}

fn hf_cost_model() -> CostModel {
    CostModel {
        dex_fee_bps: 5,
        slippage_bps: 20,
        base_fee_lamports: 5_000,
        priority_fee_lamports: 50_000,
    }
}

/// Run the unmodified engine on the 1s series; return the report's JSON and the holdout read
/// count, so byte-identity and seal-integrity assertions never need to touch report internals.
fn run(parallelism: Parallelism) -> (String, u32) {
    let outcome = run_sweep(
        &hf_spec(),
        hf_bars(),
        &hf_cost_model(),
        dec!(10_000),
        31_536_000.0,
        parallelism,
    )
    .unwrap();
    (
        outcome.report.to_json(),
        outcome.sealed.holdout_read_count(),
    )
}

#[test]
fn existing_engine_runs_unmodified_on_1s_bars() {
    let outcome = run_sweep(
        &hf_spec(),
        hf_bars(),
        &hf_cost_model(),
        dec!(10_000),
        31_536_000.0,
        Parallelism::Sequential,
    )
    .unwrap();
    assert!(outcome.report.trial_count > 0, "expected nonzero trials");
    assert_eq!(
        outcome.report.verdicts.len(),
        8,
        "expected one verdict per candidate point (4 TrendAlloc + 4 ThresholdRebalance)"
    );
    // Anti-vacuous floor: >= 8 windows x 8 points; the BeforeCosts/Base/Doubled scenario ladder
    // multiplies it further, so this is a floor, not the exact product.
    assert!(
        outcome.report.trial_count >= 64,
        "expected trial_count >= 64, got {}",
        outcome.report.trial_count
    );
}

#[test]
fn hf_sweep_deterministic_across_thread_counts() {
    let (reference_json, reference_reads) = run(Parallelism::Sequential);

    for n in [1usize, 2, 3, 7, 8] {
        let threads = Parallelism::Threads(std::num::NonZeroUsize::new(n).unwrap());
        let (json, reads) = run(threads);
        assert_eq!(json, reference_json, "byte mismatch at thread count {n}");
        assert_eq!(
            reads, reference_reads,
            "holdout reads mismatch at thread count {n}"
        );
    }

    let (repeat_json, repeat_reads) = run(Parallelism::Sequential);
    assert_eq!(
        repeat_json, reference_json,
        "repeated sequential run must be byte-identical"
    );
    assert_eq!(repeat_reads, reference_reads);
}

#[test]
fn hf_report_validates_against_schema() {
    let outcome = run_sweep(
        &hf_spec(),
        hf_bars(),
        &hf_cost_model(),
        dec!(10_000),
        31_536_000.0,
        Parallelism::Sequential,
    )
    .unwrap();

    let schema_path = format!(
        "{}/../../schemas/sweep-report.schema.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let schema_text =
        std::fs::read_to_string(&schema_path).unwrap_or_else(|e| panic!("read {schema_path}: {e}"));
    let schema: serde_json::Value =
        serde_json::from_str(&schema_text).unwrap_or_else(|e| panic!("parse {schema_path}: {e}"));
    let validator =
        jsonschema::validator_for(&schema).expect("sweep-report schema is valid JSON Schema");

    let value = outcome.report.to_value();
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "schema validation failed: {errors:?}");
}

#[test]
fn holdout_stays_sealed_on_1s_bars() {
    let (_json, holdout_read_count) = run(Parallelism::Sequential);
    assert_eq!(
        holdout_read_count, 0,
        "holdout must stay sealed by run_sweep alone"
    );
}
