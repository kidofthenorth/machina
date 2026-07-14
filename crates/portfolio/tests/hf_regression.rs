//! M-HF-C3 gate: `run_hf(fixed_latency(1)) == run` bar-for-bar (orders, fills, balances,
//! equity — exact Decimal equality), and the landing-table primitive is identical across
//! explicit thread counts {1, 2, 3, 7, 8} (never `available_parallelism`; m4-sweep §5.6).

use portfolio::{build_landing_table, fixed_latency, run, run_hf, CostModel, LandingOutcome};
use research_core::{Bar, Decimal, Timestamp};
use rust_decimal_macros::dec;

/// 1s-spaced bars with open ≠ close so "executes at OPEN" is distinguishable
/// (OHLC-valid: open = p, close = p + 2, high = p + 3, low = p − 1).
fn series(prices: &[i64]) -> Vec<Bar> {
    prices
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let open = Decimal::from(*p);
            Bar {
                ts: Timestamp::from_unix(1_700_000_000 + i as i64),
                open,
                high: open + dec!(3),
                low: open - dec!(1),
                close: open + dec!(2),
                volume: dec!(1),
            }
        })
        .collect()
}

type Strat = Box<dyn FnMut(&[Bar], Decimal) -> Decimal>;

fn constant_half() -> Strat {
    Box::new(|_h, _w| dec!(0.5))
}
fn stepper() -> Strat {
    let mut step = 0u32;
    Box::new(move |_h, _w| {
        step += 1;
        if step % 3 == 0 {
            dec!(0)
        } else {
            dec!(1)
        }
    })
}
fn banded() -> Strat {
    Box::new(|_h, w| {
        if (w - dec!(0.5)).abs() > dec!(0.1) {
            dec!(0.5)
        } else {
            w
        }
    })
}

fn shapes() -> Vec<Vec<Bar>> {
    vec![
        series(&[100, 110, 121, 133, 146, 160]), // rising
        series(&[160, 146, 133, 121, 110, 100]), // falling
        series(&[100, 140, 90, 150, 80, 160]),   // choppy
        series(&[100, 100, 100, 100, 100, 100]), // flat
    ]
}
fn costs() -> Vec<CostModel> {
    vec![
        CostModel::zero(),
        CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        },
    ]
}

fn assert_fixed1_equals_run(mut mk: impl FnMut() -> Strat) {
    for bars in shapes() {
        for cost in costs() {
            let baseline = run(&bars, dec!(10_000), &cost, mk()).unwrap();
            let hf = run_hf(
                &bars,
                dec!(10_000),
                &cost,
                &fixed_latency(1, bars.len()),
                mk(),
            )
            .unwrap();
            assert_eq!(hf.base, baseline, "run_hf(fixed_latency(1)) must equal run");
            assert_eq!(hf.unlanded_orders, 0);
        }
    }
}

#[test]
fn fixed_latency_one_equals_run_constant_half() {
    assert_fixed1_equals_run(constant_half);
}
#[test]
fn fixed_latency_one_equals_run_stateful_stepper() {
    assert_fixed1_equals_run(stepper);
}
#[test]
fn fixed_latency_one_equals_run_weight_banded() {
    assert_fixed1_equals_run(banded);
}

#[test]
fn landing_table_identical_across_thread_counts() {
    const N: u64 = 100_000;
    let cell_id = 0x00C0_FFEE_u64;
    let sequential = build_landing_table(cell_id, 0..N, 3, 7, 10).unwrap();
    // Non-degenerate: both outcomes occur, so the equality below is discriminating.
    assert!(sequential.iter().any(|l| l.landed));
    assert!(sequential.iter().any(|l| !l.landed));
    for &k in &[1usize, 2, 3, 7, 8] {
        let chunk = N.div_ceil(k as u64);
        let mut parts: Vec<Vec<LandingOutcome>> = Vec::new();
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..k as u64)
                .map(|i| {
                    let start = i * chunk;
                    let end = ((i + 1) * chunk).min(N);
                    s.spawn(move || {
                        if start >= end {
                            Vec::new()
                        } else {
                            build_landing_table(cell_id, start..end, 3, 7, 10).unwrap()
                        }
                    })
                })
                .collect();
            for h in handles {
                parts.push(h.join().unwrap());
            }
        });
        assert_eq!(parts.concat(), sequential, "thread count {k} diverged");
    }
}
