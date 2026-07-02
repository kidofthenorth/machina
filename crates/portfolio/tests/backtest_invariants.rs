//! Offline end-to-end integration tests for the portfolio simulator.
//!
//! Each test runs a small synthetic backtest through the full public stack
//! ([`CostModel`] → [`portfolio::PortfolioState`] → [`portfolio::run`]) and asserts the
//! load-bearing invariants from `docs/invariants.md`:
//!
//! - **Determinism** — identical `(bars, cash, cost, strategy)` inputs produce byte-identical
//!   [`portfolio::RunOutput`]s.
//! - **Equity consistency** — the equity curve marks every bar at its close and ends exactly at
//!   `final_state.equity(last_close)`.
//! - **Accounting across fills** — value is conserved exactly under zero costs, cost drag equals
//!   the reported fee/slippage/gas totals (up to bounded token-decimal flooring), and realized
//!   round-trip P&L reconciles with the cash delta.
//!
//! Everything here is synthetic and offline: no network, no keys, no real market data.

use portfolio::{run, CostModel};
use research_core::{Bar, Decimal, Timestamp};
use rust_decimal_macros::dec;

/// Daily bars where each bar opens at the previous close and closes at `closes[i]`.
/// High/low bracket open and close, so OHLC invariants hold; volume is a constant 1.
fn chained_series(closes: &[i64]) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(closes.len());
    let mut prev_close = Decimal::from(closes[0]);
    for (i, c) in closes.iter().enumerate() {
        let close = Decimal::from(*c);
        let open = prev_close;
        bars.push(Bar {
            ts: Timestamp::from_unix(1_609_459_200 + i as i64 * 86_400),
            open,
            high: open.max(close),
            low: open.min(close),
            close,
            volume: dec!(1),
        });
        prev_close = close;
    }
    bars
}

/// Realistic nonzero execution costs (fees, slippage, and gas all engaged).
fn realistic_cost() -> CostModel {
    CostModel {
        dex_fee_bps: 5,
        slippage_bps: 20,
        base_fee_lamports: 5_000,
        priority_fee_lamports: 50_000,
    }
}

/// A stateless momentum rule: long when the latest completed close is above the close
/// two bars earlier, otherwise flat.
fn momentum(h: &[Bar], _w: Decimal) -> Decimal {
    if h.len() >= 3 && h[h.len() - 1].close > h[h.len() - 3].close {
        dec!(1)
    } else {
        dec!(0)
    }
}

/// A strategy that replays a fixed target-weight table, one entry per strategy call
/// (i.e. per executed bar `t = 1..bars.len()`), then stays flat.
fn scripted(targets: &'static [Decimal]) -> impl FnMut(&[Bar], Decimal) -> Decimal {
    let mut step = 0usize;
    move |_h: &[Bar], _w: Decimal| {
        let t = targets.get(step).copied().unwrap_or(Decimal::ZERO);
        step += 1;
        t
    }
}

#[test]
fn identical_inputs_produce_identical_outputs() {
    let bars = chained_series(&[100, 105, 98, 103, 110, 95, 100, 108, 112, 90]);
    let cost = realistic_cost();
    let a = run(&bars, dec!(10000), &cost, momentum).unwrap();
    let b = run(&bars, dec!(10000), &cost, momentum).unwrap();
    // Non-vacuous: the momentum rule actually trades on this path.
    assert!(a.n_trades >= 3, "expected trades, got {}", a.n_trades);
    assert!(!a.round_trips.is_empty());
    // Full structural equality: equity curve, round trips, balances, and cost aggregates.
    assert_eq!(a, b);
}

#[test]
fn equity_curve_marks_every_bar_at_its_close() {
    // Zero cost, full deploy at bar 1's open (price 100, 1000 USDC → exactly 10 SOL).
    // From then on the curve must be exactly 10 × close; bar 0 is the pre-trade cash mark.
    let bars = chained_series(&[100, 110, 95, 120, 90]);
    let out = run(&bars, dec!(1000), &CostModel::zero(), |_h, _w| dec!(1)).unwrap();

    assert_eq!(out.n_trades, 1);
    assert_eq!(out.equity_curve.len(), bars.len());
    for (point, bar) in out.equity_curve.iter().zip(&bars) {
        assert_eq!(point.ts, bar.ts, "curve must mark bars in order");
    }
    let expected: Vec<Decimal> = vec![dec!(1000), dec!(1100), dec!(950), dec!(1200), dec!(900)];
    let actual: Vec<Decimal> = out.equity_curve.iter().map(|p| p.equity_quote).collect();
    assert_eq!(actual, expected);
    // The curve's final mark and the final state must agree exactly.
    let last = out.equity_curve.last().unwrap();
    assert_eq!(last.equity_quote, out.final_state.equity(bars[4].close));
}

#[test]
fn zero_cost_flat_price_backtest_conserves_value_exactly() {
    // Alternating full deploy / full exit at a flat price with zero costs: every fill is exact
    // (1000 USDC ↔ 10 SOL at 100), so equity must equal initial cash at every single close.
    static TARGETS: &[Decimal] = &[
        Decimal::ONE,
        Decimal::ZERO,
        Decimal::ONE,
        Decimal::ZERO,
        Decimal::ONE,
        Decimal::ZERO,
        Decimal::ONE,
    ];
    let bars = chained_series(&[100; 8]);
    let out = run(&bars, dec!(1000), &CostModel::zero(), scripted(TARGETS)).unwrap();

    assert_eq!(out.n_trades, 7);
    for point in &out.equity_curve {
        assert_eq!(
            point.equity_quote,
            dec!(1000),
            "zero-cost flat-price trading must conserve value at {}",
            point.ts.to_rfc3339()
        );
    }
    // Three trips completed (the fourth entry is still open when the run ends); each one is
    // costless at a flat price, so realized P&L is exactly zero.
    assert_eq!(out.round_trips.len(), 3);
    for trip in &out.round_trips {
        assert_eq!(trip.pnl_quote, Decimal::ZERO);
    }
}

#[test]
fn cost_drag_reconciles_with_reported_costs() {
    // Flat price with real costs: prices never move, so the only equity changes are costs.
    // Exact identity: initial − final = fees + slippage + gas×price + flooring loss, where the
    // flooring loss (token outputs floored to SOL/USDC decimals) is bounded per trade by
    // 1e-9 SOL × price (buys) or 1e-6 USDC (sells).
    static TARGETS: &[Decimal] = &[
        Decimal::ONE,
        Decimal::ZERO,
        Decimal::ONE,
        Decimal::ZERO,
        Decimal::ZERO,
    ];
    let price = dec!(100);
    let initial = dec!(1000);
    let bars = chained_series(&[100; 6]);
    let out = run(&bars, initial, &realistic_cost(), scripted(TARGETS)).unwrap();

    assert_eq!(out.n_trades, 4);
    let final_equity = out.final_state.equity(price);
    let drag = initial - final_equity;
    let reported = out.fees_paid_quote + out.slippage_paid_quote + out.gas_paid_sol * price;
    assert!(drag > Decimal::ZERO, "costed churn must lose money");
    assert!(
        drag >= reported,
        "drag {drag} cannot be below reported costs {reported}"
    );
    let flooring_bound = Decimal::from(out.n_trades) * (dec!(0.0000001) + dec!(0.000001));
    assert!(
        drag - reported < flooring_bound,
        "drag {drag} exceeds reported costs {reported} by more than flooring dust"
    );
    // The curve's final mark agrees with the reconciled final state.
    assert_eq!(out.equity_curve.last().unwrap().equity_quote, final_equity);
}

#[test]
fn round_trip_pnl_reconciles_with_cash_delta_when_run_ends_flat() {
    // Two full in/out trips over a moving price with real costs. The run starts and ends flat,
    // so the sum of realized round-trip P&L must equal the total cash delta exactly.
    static TARGETS: &[Decimal] = &[
        Decimal::ONE,
        Decimal::ONE,
        Decimal::ZERO,
        Decimal::ONE,
        Decimal::ZERO,
        Decimal::ZERO,
    ];
    let initial = dec!(1000);
    let bars = chained_series(&[100, 105, 98, 103, 110, 95, 100]);
    let out = run(&bars, initial, &realistic_cost(), scripted(TARGETS)).unwrap();

    assert_eq!(out.n_trades, 4);
    assert_eq!(out.round_trips.len(), 2);
    assert_eq!(
        out.final_state.base_balance,
        Decimal::ZERO,
        "a full exit must leave exactly zero SOL"
    );
    let pnl_sum: Decimal = out.round_trips.iter().map(|t| t.pnl_quote).sum();
    assert_eq!(pnl_sum, out.final_state.quote_balance - initial);
    // Trips are ordered and non-overlapping: each closes after it opens, and the next opens
    // no earlier than the previous close.
    for trip in &out.round_trips {
        assert!(trip.exit_ts > trip.entry_ts);
    }
    for pair in out.round_trips.windows(2) {
        assert!(pair[1].entry_ts >= pair[0].exit_ts);
    }
    // All equity marks stay positive: long-only spot with non-negative balances cannot go under.
    assert!(out
        .equity_curve
        .iter()
        .all(|p| p.equity_quote > Decimal::ZERO));
}
