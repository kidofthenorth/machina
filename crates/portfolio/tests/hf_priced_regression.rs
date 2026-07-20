//! M-HF-C8.6b gate: `run_hf_priced` determinism, "never cheaper than plain `CostModel`" at a
//! matched reference config, landing-draw/adverse-draw independence, and `regimes.len() !=
//! bars.len()` rejection.

use portfolio::{
    fixed_latency, run_hf, run_hf_priced, AdversarialModel, CongestionPriorityTable,
    CongestionRegime, CostModel, DepthBand, DepthCurve, HfCostModel, HfError,
};
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

fn regimes_of(n: usize, regime: CongestionRegime) -> Vec<CongestionRegime> {
    vec![regime; n]
}

#[test]
fn repeated_runs_are_deterministic() {
    let bars = series(&[100, 200, 250, 400]);
    let hf = HfCostModel {
        base: CostModel {
            dex_fee_bps: 5,
            slippage_bps: 0,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 0,
        },
        depth_curve: DepthCurve::new(vec![DepthBand {
            notional_upto: dec!(1_000_000),
            impact_bps: 10,
        }])
        .unwrap(),
        congestion_priority_table: CongestionPriorityTable::new(1_000, 2_000, 3_000).unwrap(),
        tip_bps: 2,
    };
    let adversarial = AdversarialModel {
        sandwich_bps: 5,
        pickoff_bps: 5,
        p_adverse_num: 1,
        p_adverse_den: 4,
    };
    let regimes = regimes_of(bars.len(), CongestionRegime::Busy);
    let strat = |_h: &[Bar], _w: Decimal| dec!(0.5);
    let a = run_hf_priced(
        &bars,
        dec!(10_000),
        &hf,
        &adversarial,
        &fixed_latency(2, 4),
        &regimes,
        7,
        strat,
    )
    .unwrap();
    let b = run_hf_priced(
        &bars,
        dec!(10_000),
        &hf,
        &adversarial,
        &fixed_latency(2, 4),
        &regimes,
        7,
        strat,
    )
    .unwrap();
    assert_eq!(a, b);
}

/// Reference-config recipe (card §"never cheaper" proof sketch): a matched plain `CostModel`
/// vs. an HF config that exactly reproduces it (zero slack), then a second case where
/// tip/adverse-selection strictly bite.
#[test]
fn priced_run_is_never_cheaper_than_plain_cost_model() {
    let bars = series(&[100, 200, 250, 400]);
    let dex_fee_bps = 10;
    let slippage_bps = 25;
    let base_fee_lamports = 5_000;
    let priority_fee_lamports = 20_000;

    let plain_cost = CostModel {
        dex_fee_bps,
        slippage_bps,
        base_fee_lamports,
        priority_fee_lamports,
    };

    // Case 1: exact-match boundary config, zero slack — depth curve reproduces slippage_bps
    // flatly, priority table reproduces priority_fee_lamports for every regime, adverse
    // selection off.
    let hf_matched = HfCostModel {
        base: CostModel {
            dex_fee_bps,
            slippage_bps: 0,
            base_fee_lamports,
            priority_fee_lamports: 0,
        },
        depth_curve: DepthCurve::new(vec![DepthBand {
            notional_upto: dec!(1_000_000),
            impact_bps: slippage_bps,
        }])
        .unwrap(),
        congestion_priority_table: CongestionPriorityTable::new(
            priority_fee_lamports,
            priority_fee_lamports,
            priority_fee_lamports,
        )
        .unwrap(),
        tip_bps: 0,
    };
    let adversarial_off = AdversarialModel {
        sandwich_bps: 0,
        pickoff_bps: 0,
        p_adverse_num: 0,
        p_adverse_den: 1,
    };
    let regimes = regimes_of(bars.len(), CongestionRegime::Calm);
    let strat = |_h: &[Bar], _w: Decimal| dec!(1);

    let plain = run_hf(
        &bars,
        dec!(10_000),
        &plain_cost,
        &fixed_latency(1, 4),
        strat,
    )
    .unwrap();
    let priced_matched = run_hf_priced(
        &bars,
        dec!(10_000),
        &hf_matched,
        &adversarial_off,
        &fixed_latency(1, 4),
        &regimes,
        1,
        strat,
    )
    .unwrap();

    let plain_total_cost =
        plain.base.fees_paid_quote + plain.base.slippage_paid_quote + plain.base.gas_paid_sol;
    let priced_total_cost = priced_matched.base.base.fees_paid_quote
        + priced_matched.base.base.slippage_paid_quote
        + priced_matched.base.base.gas_paid_sol;
    assert!(
        priced_total_cost >= plain_total_cost,
        "priced={priced_total_cost} plain={plain_total_cost}"
    );

    // Case 2: same matched config, but with a strictly positive tip AND a forced (p=1)
    // adverse-selection hit — must show a strictly larger cost.
    let hf_biting = HfCostModel {
        tip_bps: 3,
        ..hf_matched
    };
    let adversarial_forced = AdversarialModel {
        sandwich_bps: 4,
        pickoff_bps: 4,
        p_adverse_num: 1,
        p_adverse_den: 1,
    };
    let priced_biting = run_hf_priced(
        &bars,
        dec!(10_000),
        &hf_biting,
        &adversarial_forced,
        &fixed_latency(1, 4),
        &regimes,
        1,
        strat,
    )
    .unwrap();
    let priced_biting_total_cost = priced_biting.base.base.fees_paid_quote
        + priced_biting.base.base.slippage_paid_quote
        + priced_biting.base.base.gas_paid_sol;
    assert!(
        priced_biting_total_cost > plain_total_cost,
        "biting={priced_biting_total_cost} plain={plain_total_cost}"
    );
}

// The landing-vs-adverse-draw independence proof over `0..100_000` lives as an in-module test
// in `crates/portfolio/src/hf_priced.rs` (`landing_and_adverse_draws_are_independent_over_100k`)
// rather than here: `adverse_selection_hit`/`adverse_draw` are module-private (matching the
// card's file-1 scoping — only `run_hf_priced`/`HfPricedRunOutput` are re-exported via `lib.rs`),
// so an integration test in this file has no way to call them directly.

#[test]
fn regimes_length_mismatch_is_rejected() {
    let bars = series(&[100, 200, 250]);
    let hf = HfCostModel {
        base: CostModel {
            dex_fee_bps: 5,
            slippage_bps: 0,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 0,
        },
        depth_curve: DepthCurve::new(vec![DepthBand {
            notional_upto: dec!(1_000_000),
            impact_bps: 10,
        }])
        .unwrap(),
        congestion_priority_table: CongestionPriorityTable::new(1_000, 2_000, 3_000).unwrap(),
        tip_bps: 0,
    };
    let adversarial = AdversarialModel {
        sandwich_bps: 0,
        pickoff_bps: 0,
        p_adverse_num: 0,
        p_adverse_den: 1,
    };
    let regimes = regimes_of(bars.len() - 1, CongestionRegime::Calm); // deliberately short
    let strat = |_h: &[Bar], _w: Decimal| dec!(0.5);
    let err = run_hf_priced(
        &bars,
        dec!(10_000),
        &hf,
        &adversarial,
        &fixed_latency(1, bars.len()),
        &regimes,
        1,
        strat,
    )
    .unwrap_err();
    assert_eq!(
        err,
        HfError::RegimesLength {
            regimes: bars.len() - 1,
            bars: bars.len()
        }
    );
}
