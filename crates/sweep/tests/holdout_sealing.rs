//! S9 gate (plan §6, §14, invariant 11): the holdout must be *sealed*. Parameter selection runs the
//! full sweep + walk-forward + selection pipeline holding only a `&DevValidation`; it must be
//! structurally unable to read or mutate the holdout, which lives in a separate allocation reachable
//! only through the call-once `evaluate_on_holdout` (the M5 entry point — never the M4 CLI).
//!
//! This integration test exercises the public API only. The *type-level* halves of the gate — that
//! `DevValidation` has no holdout accessor, that a `Sealed` cannot be fabricated, and that
//! `evaluate_on_holdout` consumes its seal by value (call-once) — are pinned by `compile_fail`
//! doctests on those items (run via `cargo test --doc`).

use portfolio::CostModel;
use research_core::{Bar, Decimal, Timestamp};
use rust_decimal_macros::dec;
use sweep::{
    eval_cell, evaluate_on_holdout, run_cells, CellResult, DevValidation, Parallelism, ParamGrid,
    ParamPoint, PartitionError, PartitionSpec, PartitionedBars, SweepCell, WalkForward, WindowKind,
};

const DAY: i64 = 86_400;

/// A deterministic, sorted, OHLC-valid synthetic SOL/USDC daily path (NOT real market data): a
/// gentle saw-tooth around a rising trend, so the rebalancers actually trade and parameter points
/// score differently.
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

/// A realistic execution cost model (matches the determinism-gate fixture).
fn cost() -> CostModel {
    CostModel {
        dex_fee_bps: 5,
        slippage_bps: 20,
        base_fee_lamports: 5_000,
        priority_fee_lamports: 50_000,
    }
}

/// The full sweep + selection pipeline: build the cells of a parameter grid crossed with the
/// walk-forward folds of the dev/val partition, run them deterministically, and pick the best
/// parameter point by an exact `Decimal` key (summed out-of-sample `total_return`, ties broken by
/// the grid's fixed enumeration order). It holds **only** a `&DevValidation` — it is handed no
/// access to the holdout whatsoever.
fn select_best(dev: &DevValidation, cash: Decimal) -> ParamPoint {
    let grid = ParamGrid::ThresholdRebalance {
        target_sol_weights: vec![dec!(0.25), dec!(0.5), dec!(0.75)],
        bands: vec![dec!(0), dec!(0.05)],
    };
    let points = grid.points();
    let p_count = points.len();

    // Walk-forward folds over the dev/val length only — structurally inside `0..dev.len()`.
    let wf = WalkForward::new(WindowKind::Rolling, 10, 5, 5, 0).unwrap();
    let windows = dev.walk_forward_windows(&wf);
    assert!(
        !windows.is_empty(),
        "test needs at least one walk-forward fold"
    );

    let cost = cost();
    // Canonical cell order: window index (outer) → parameter order (inner).
    let mut cells = Vec::new();
    for w in &windows {
        for p in &points {
            cells.push(SweepCell {
                index: cells.len(),
                point: p.clone(),
                cost: cost.clone(),
                test: w.test.clone(),
            });
        }
    }
    let results = run_cells(
        &cells,
        dev.dev_validation(),
        cash,
        365.0,
        Parallelism::Sequential,
    )
    .expect("dev/val cells evaluate");

    // Aggregate each parameter point's out-of-sample return across folds (exact Decimal) and take
    // the max; first point wins ties, for a deterministic choice.
    let mut best_idx = 0usize;
    let mut best_sum: Option<Decimal> = None;
    for i in 0..p_count {
        let sum: Decimal = (0..windows.len())
            .map(|w| results[w * p_count + i].total_return)
            .sum();
        let take = match best_sum {
            None => true,
            Some(b) => sum > b,
        };
        if take {
            best_idx = i;
            best_sum = Some(sum);
        }
    }
    points[best_idx].clone()
}

#[test]
fn full_pipeline_never_reads_or_mutates_the_holdout() {
    // dev = [0,30), val = [30,45), holdout = [45,60).
    let bars = synthetic_series(60);
    let pb = PartitionedBars::from_spec(bars, &PartitionSpec::by_index(30, 45)).unwrap();
    let (dev, sealed) = pb.seal_holdout();

    // The holdout is untouched before the pipeline runs.
    assert_eq!(sealed.holdout_read_count(), 0);
    let digest_before = sealed.holdout_digest();

    // Run the FULL sweep + selection pipeline. It receives only `&dev` — never `sealed`.
    let chosen = select_best(&dev, dec!(10_000));

    // Selection could not — and did not — read or change the holdout.
    assert_eq!(
        sealed.holdout_read_count(),
        0,
        "selection must never read the holdout"
    );
    assert_eq!(
        sealed.holdout_digest(),
        digest_before,
        "holdout content must be unchanged after selection"
    );

    // Re-running selection is deterministic and still never touches the holdout.
    let chosen_again = select_best(&dev, dec!(10_000));
    assert_eq!(chosen, chosen_again, "selection must be deterministic");
    assert_eq!(sealed.holdout_read_count(), 0);
    assert_eq!(sealed.holdout_digest(), digest_before);
}

#[test]
fn evaluate_on_holdout_scores_exactly_the_sealed_holdout_bars() {
    let bars = synthetic_series(60);
    let expected_holdout: Vec<Bar> = bars[45..].to_vec();
    let pb = PartitionedBars::from_spec(bars, &PartitionSpec::by_index(30, 45)).unwrap();
    let (dev, sealed) = pb.seal_holdout();

    // Choose parameters using only dev/val…
    let chosen = select_best(&dev, dec!(10_000));

    // …then take the single, call-once holdout read (the M5 entry point). `sealed` is consumed here.
    let on_holdout = evaluate_on_holdout(sealed, &chosen, &cost(), dec!(10_000), 365.0).unwrap();

    // The result equals a direct, deterministic evaluation over exactly the held-out slice — proving
    // the right bars, and only those, were scored.
    let direct: CellResult =
        eval_cell(&chosen, &expected_holdout, &cost(), dec!(10_000), 365.0).unwrap();
    assert_eq!(on_holdout, direct);
}

#[test]
fn overlapping_partition_config_is_rejected() {
    let bars = synthetic_series(20);
    // holdout_start (5) lies before val_start (12): overlapping / mis-ordered → rejected, not bled.
    let err = PartitionedBars::from_spec(bars, &PartitionSpec::by_index(12, 5)).unwrap_err();
    assert_eq!(
        err,
        PartitionError::ValidationEmptyOrOverlapping {
            val_start: 12,
            holdout_start: 5
        }
    );
}

#[test]
fn date_bounded_overlapping_config_is_rejected() {
    // Daily bars from 1970-01-01 (day 0) to day 19. Holdout date (day 4) lies before the validation
    // date (day 11) → mis-ordered, rejected.
    let bars = synthetic_series(20);
    let spec = PartitionSpec::by_date("1970-01-12", "1970-01-05").unwrap();
    assert!(matches!(
        PartitionedBars::from_spec(bars, &spec).unwrap_err(),
        PartitionError::ValidationEmptyOrOverlapping { .. }
    ));
}

#[test]
fn date_bounded_partition_seals_the_correct_tail() {
    // A date-driven three-way split, end to end, via the S2 date parser.
    let bars = synthetic_series(60); // days 0..60 from the epoch
    let expected_holdout: Vec<Bar> = bars[45..].to_vec(); // day 45 = 1970-02-15
    let spec = PartitionSpec::by_date("1970-01-31", "1970-02-15").unwrap();
    let pb = PartitionedBars::from_spec(bars, &spec).unwrap();
    assert_eq!(pb.development_len(), 30); // days 0..30
    assert_eq!(pb.validation_len(), 15); // days 30..45
    assert_eq!(pb.holdout_len(), 15); // days 45..60

    let (dev, sealed) = pb.seal_holdout();
    let chosen = select_best(&dev, dec!(10_000));
    assert_eq!(sealed.holdout_read_count(), 0);
    let on_holdout = evaluate_on_holdout(sealed, &chosen, &cost(), dec!(10_000), 365.0).unwrap();
    let direct = eval_cell(&chosen, &expected_holdout, &cost(), dec!(10_000), 365.0).unwrap();
    assert_eq!(on_holdout, direct);
}
