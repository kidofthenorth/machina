//! Integration: every strategy produces explainable orders/fills/balances when wired through the
//! deterministic simulator (M3 gate, behavioral — NOT a performance claim).

use portfolio::{run, CostModel, RunOutput};
use research_core::{Bar, Decimal, Timestamp};
use rust_decimal_macros::dec;
use strategies::{
    BuyAndHoldSol, DcaIntoSol, HoldUsdc, Static5050, Strategy, ThresholdRebalanceV1, TrendAllocV1,
};

fn series(closes: &[i64]) -> Vec<Bar> {
    closes
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let p = Decimal::from(c);
            Bar {
                ts: Timestamp::from_unix(i as i64 * 86_400),
                open: p,
                high: p,
                low: p,
                close: p,
                volume: dec!(1),
            }
        })
        .collect()
}

fn run_strategy<S: Strategy>(s: &S, bars: &[Bar], cost: &CostModel) -> RunOutput {
    run(bars, dec!(10000), cost, |h, w| s.target_weight(h, w)).unwrap()
}

#[test]
fn hold_usdc_never_trades() {
    let bars = series(&[100, 120, 90, 130]);
    let out = run_strategy(&HoldUsdc, &bars, &CostModel::zero());
    assert_eq!(out.n_trades, 0);
    assert_eq!(out.final_state.base_balance, dec!(0));
    assert_eq!(out.final_state.quote_balance, dec!(10000));
    // Equity is flat at the starting cash regardless of SOL price.
    assert!(out
        .equity_curve
        .iter()
        .all(|p| p.equity_quote == dec!(10000)));
}

#[test]
fn buy_and_hold_deploys_fully_and_is_deterministic() {
    let bars = series(&[100, 110, 120]);
    let a = run_strategy(&BuyAndHoldSol, &bars, &CostModel::zero());
    let b = run_strategy(&BuyAndHoldSol, &bars, &CostModel::zero());
    assert_eq!(a, b, "repeated runs must be identical");
    assert_eq!(a.n_trades, 1);
    assert_eq!(a.final_state.quote_balance, dec!(0));
    assert!(a.final_state.base_balance > dec!(0));
}

#[test]
fn static_50_50_holds_about_half_in_sol() {
    let bars = series(&[100, 100, 100, 100]);
    let out = run_strategy(&Static5050, &bars, &CostModel::zero());
    // At a flat price, after the initial rebalance ~half of equity is SOL.
    let last = out.equity_curve.last().unwrap().equity_quote;
    let sol_value = out.final_state.base_balance * dec!(100);
    let half = last / dec!(2);
    assert!(
        (sol_value - half).abs() < dec!(1),
        "sol_value={sol_value} half={half}"
    );
}

#[test]
fn trend_alloc_goes_long_in_an_uptrend() {
    let s = TrendAllocV1 {
        sma_period: 3,
        weight_above: dec!(1),
        weight_below: dec!(0),
    };
    let bars = series(&[100, 101, 102, 103, 104, 105, 106]);
    let out = run_strategy(&s, &bars, &CostModel::zero());
    assert!(
        out.final_state.base_balance > dec!(0),
        "trend alloc should be long in an uptrend"
    );
    assert!(out.n_trades >= 1);
}

#[test]
fn trend_alloc_stays_flat_in_a_downtrend() {
    let s = TrendAllocV1 {
        sma_period: 3,
        weight_above: dec!(1),
        weight_below: dec!(0),
    };
    let bars = series(&[106, 105, 104, 103, 102, 101, 100]);
    let out = run_strategy(&s, &bars, &CostModel::zero());
    assert_eq!(
        out.final_state.base_balance,
        dec!(0),
        "trend alloc should avoid a downtrend"
    );
    assert_eq!(out.n_trades, 0);
}

#[test]
fn threshold_rebalance_rebalances_from_flat_then_stays_bounded() {
    let s = ThresholdRebalanceV1 {
        target_sol_weight: dec!(0.5),
        band: dec!(0.1),
    };
    let bars = series(&[100, 100, 100, 100, 100]);
    let out = run_strategy(&s, &bars, &CostModel::zero());
    // Starts flat (weight 0); drift 0.5 > band → first rebalance to ~50%.
    assert!(out.n_trades >= 1);
    assert!(out.final_state.base_balance > dec!(0));
    // At a flat price it should not churn after reaching the band.
    assert!(
        out.n_trades <= 2,
        "should not churn at a flat price: {} trades",
        out.n_trades
    );
}

#[test]
fn dca_ramps_into_sol_over_time() {
    // Ramp over 3 bars; with 6 daily bars the position should be (near) fully invested by the end.
    let s = DcaIntoSol { ramp_bars: 3 };
    let bars = series(&[100, 100, 100, 100, 100, 100]);
    let out = run_strategy(&s, &bars, &CostModel::zero());
    assert!(out.n_trades >= 1, "DCA should make progressive buys");
    let last_equity = out.equity_curve.last().unwrap().equity_quote;
    let sol_value = out.final_state.base_balance * dec!(100);
    // After the ramp completes, nearly all equity is in SOL.
    assert!(
        sol_value > last_equity * dec!(0.95),
        "expected ~fully invested: {sol_value}/{last_equity}"
    );
}
