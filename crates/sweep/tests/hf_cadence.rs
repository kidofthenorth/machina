//! Cadence + determinism proof for `intraday_meanrev_v1` (card M-HF-C7): the new family, run
//! directly through `portfolio::run_hf` (NOT `run_sweep`/`ParamGrid` — that wiring is C8's job) over
//! a ≥100k-bar C2 synthetic 1s series, is byte-identical on repeat.

use market_data::synthetic::{generate, SyntheticSpec};
use portfolio::{fixed_latency, run_hf, CostModel};
use research_core::Bar;
use rust_decimal_macros::dec;
use strategies::{IntradayMeanRevV1, Strategy};

fn hf_bars() -> Vec<Bar> {
    let spec = SyntheticSpec {
        seed: 7,
        steps: 100_000,
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

fn hf_cost_model() -> CostModel {
    CostModel {
        dex_fee_bps: 5,
        slippage_bps: 20,
        base_fee_lamports: 5_000,
        priority_fee_lamports: 50_000,
    }
}

#[test]
fn intraday_meanrev_v1_runs_deterministically_at_100k_bar_cadence() {
    let bars = hf_bars();
    assert!(bars.len() >= 100_000, "expected >=100k synthetic bars");

    let s = IntradayMeanRevV1::illustrative();
    let cost = hf_cost_model();
    let pipeline = fixed_latency(1, bars.len());

    let a = run_hf(&bars, dec!(10_000), &cost, &pipeline, |h, w| {
        s.target_weight(h, w)
    })
    .unwrap();
    let b = run_hf(&bars, dec!(10_000), &cost, &pipeline, |h, w| {
        s.target_weight(h, w)
    })
    .unwrap();

    assert_eq!(
        a.base.equity_curve.len(),
        bars.len(),
        "expected the engine to process every bar end-to-end"
    );
    assert_eq!(a, b, "repeated run_hf must be byte-identical");
}

#[test]
fn intraday_meanrev_v1_trades_on_a_cheap_dip() {
    // Hand-built series: flat at 100 for the anchor window, then a dip well past the 1% band.
    use research_core::Timestamp;

    let s = IntradayMeanRevV1 {
        anchor_period: 3,
        band: dec!(0.01),
        weight_in: dec!(0.5),
        weight_out: dec!(0),
    };
    let bar = |i: usize, close: rust_decimal::Decimal| Bar {
        ts: Timestamp::from_unix(i as i64),
        open: close,
        high: close,
        low: close,
        close,
        volume: dec!(1),
    };
    let history = vec![bar(0, dec!(100)), bar(1, dec!(100)), bar(2, dec!(97))];
    assert_eq!(s.target_weight(&history, dec!(0)), dec!(0.5));
}
