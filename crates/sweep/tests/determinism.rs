//! The M4 determinism gate (plan §19, invariant 3): a real parameter sweep must produce results that
//! are identical across thread counts, identical between parallel and sequential paths, and
//! byte-identical on repeated runs. Uses EXPLICIT thread counts (never `available_parallelism`), so
//! the gate is a property of the engine, not of the host.

use portfolio::CostModel;
use research_core::{Bar, Decimal, Timestamp};
use rust_decimal_macros::dec;
use std::num::NonZeroUsize;
use sweep::{run_cells, CellResult, Parallelism, ParamGrid, SweepCell};

/// A deterministic synthetic SOL/USDC daily path with trend and pullbacks (not real market data).
fn synthetic_series() -> Vec<Bar> {
    const CLOSES: [i64; 30] = [
        100, 102, 105, 103, 108, 112, 115, 110, 107, 111, 118, 125, 130, 128, 122, 119, 124, 131,
        138, 135, 129, 133, 140, 145, 139, 132, 141, 150, 147, 155,
    ];
    const DAY: i64 = 86_400;
    CLOSES
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let p = Decimal::from(c);
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

/// Build a realistic batch of cells: a trend grid × two cost models over the whole series.
fn build_cells(bars: &[Bar]) -> Vec<SweepCell> {
    let points = ParamGrid::TrendAlloc {
        sma_periods: vec![2, 3, 5, 8],
        weights_above: vec![dec!(0.5), dec!(0.75), dec!(1)],
        weights_below: vec![dec!(0), dec!(0.25)],
    }
    .points(); // 4 * 3 * 2 = 24 points
    let costs = [
        CostModel::zero(),
        CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        },
    ];
    let mut cells = Vec::new();
    let mut index = 0;
    for cost in &costs {
        for point in &points {
            cells.push(SweepCell {
                index,
                point: point.clone(),
                cost: cost.clone(),
                test: 0..bars.len(),
            });
            index += 1;
        }
    }
    assert_eq!(cells.len(), 48, "expected 24 points x 2 cost models");
    cells
}

fn run(cells: &[SweepCell], bars: &[Bar], p: Parallelism) -> Vec<CellResult> {
    run_cells(cells, bars, dec!(10000), 365.0, p).expect("all cells valid")
}

fn threads(n: usize) -> Parallelism {
    Parallelism::Threads(NonZeroUsize::new(n).unwrap())
}

#[test]
fn parallel_equals_sequential_across_thread_counts() {
    let bars = synthetic_series();
    let cells = build_cells(&bars);
    let reference = run(&cells, &bars, Parallelism::Sequential);
    let reference_json = serde_json::to_string(&reference).unwrap();

    for k in [1usize, 2, 3, 7, 8] {
        let parallel = run(&cells, &bars, threads(k));
        // Full structural equality (catches divergence that {:.10} f64 formatting could mask)...
        assert_eq!(
            parallel, reference,
            "structural mismatch at thread count {k}"
        );
        // ...AND byte-identical serialized output.
        assert_eq!(
            serde_json::to_string(&parallel).unwrap(),
            reference_json,
            "byte mismatch at thread count {k}"
        );
    }
}

#[test]
fn repeated_runs_are_byte_identical() {
    let bars = synthetic_series();
    let cells = build_cells(&bars);
    let a = serde_json::to_string(&run(&cells, &bars, threads(4))).unwrap();
    let b = serde_json::to_string(&run(&cells, &bars, threads(4))).unwrap();
    assert_eq!(a, b, "repeated runs must be byte-identical");
    // And identical to the sequential serialization.
    let s = serde_json::to_string(&run(&cells, &bars, Parallelism::Sequential)).unwrap();
    assert_eq!(a, s);
}

#[test]
fn a_nonzero_fraction_of_cells_actually_trade() {
    // Guard against a vacuous gate: if every cell were a no-op, "determinism" would be trivial.
    let bars = synthetic_series();
    let cells = build_cells(&bars);
    let results = run(&cells, &bars, Parallelism::Sequential);
    let trading = results.iter().filter(|c| c.n_trades > 0).count();
    assert!(
        trading >= results.len() / 2,
        "expected most cells to trade, got {trading}/{}",
        results.len()
    );
}
