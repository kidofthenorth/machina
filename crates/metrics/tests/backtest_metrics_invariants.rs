//! Offline end-to-end integration tests: simulator equity curve → metrics.
//!
//! Each test runs a small synthetic backtest through the portfolio simulator and feeds the
//! resulting equity curve into this crate, asserting the load-bearing invariants:
//!
//! - **Determinism** — identical `(bars, cash, cost, strategy)` inputs produce identical
//!   [`Metrics`], including the display-only `f64` fields.
//! - **Equity consistency** — the exact accounting metrics (`total_return`, `max_drawdown`)
//!   reconcile with the run's final portfolio state and with a hand-computed equity curve.
//! - **Layer separation** — the exact `Decimal` metrics never depend on the annualization
//!   factor, which only feeds the display-only statistical layer (`docs/invariants.md`,
//!   invariant 7).
//!
//! Everything here is synthetic and offline: no network, no keys, no real market data.

use metrics::{max_drawdown, total_return, Metrics};
use portfolio::{run, CostModel, RunOutput};
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

/// The run's equity curve as a plain slice of marks, ready for [`Metrics::from_equity`].
fn curve(out: &RunOutput) -> Vec<Decimal> {
    out.equity_curve.iter().map(|p| p.equity_quote).collect()
}

#[test]
fn identical_backtests_produce_identical_metrics() {
    let bars = chained_series(&[100, 105, 98, 103, 110, 95, 100, 108, 112, 90]);
    let cost = realistic_cost();
    let a = run(&bars, dec!(10000), &cost, momentum).unwrap();
    let b = run(&bars, dec!(10000), &cost, momentum).unwrap();
    // Non-vacuous: the momentum rule actually trades on this path.
    assert!(a.n_trades >= 3, "expected trades, got {}", a.n_trades);

    let m_a = Metrics::from_equity(&curve(&a), 365.0);
    let m_b = Metrics::from_equity(&curve(&b), 365.0);
    // Full structural equality, including the display-only f64 fields: identical inputs
    // must produce bit-identical statistical metrics, not merely approximately equal ones.
    assert_eq!(m_a, m_b);
    // Non-vacuous: the curve moved, so the statistical layer is actually populated.
    assert!(m_a.volatility.is_some());
    assert!(m_a.sharpe.is_some());
    assert_eq!(m_a.n_periods, bars.len() - 1);
}

#[test]
fn total_return_reconciles_exactly_with_the_final_portfolio_state() {
    // A costed run that starts from an all-cash mark: equity_curve[0] is the initial cash,
    // and the last mark equals final_state.equity(last close). total_return is computed in
    // exact Decimal, so the reconciliation must hold with equality, not tolerance.
    let bars = chained_series(&[100, 105, 98, 103, 110, 95, 100, 108, 112, 90]);
    let initial = dec!(10000);
    let out = run(&bars, initial, &realistic_cost(), momentum).unwrap();
    let eq = curve(&out);

    assert_eq!(eq[0], initial, "bar 0 is the pre-trade cash mark");
    let final_equity = out.final_state.equity(bars.last().unwrap().close);
    assert_eq!(*eq.last().unwrap(), final_equity);

    let m = Metrics::from_equity(&eq, 365.0);
    assert_eq!(m.total_return, final_equity / initial - Decimal::ONE);
    // The struct field and the free function must agree on the same curve.
    assert_eq!(m.total_return, total_return(&eq));
    assert_eq!(m.max_drawdown, max_drawdown(&eq));
}

#[test]
fn max_drawdown_matches_an_independent_reference_on_a_real_run() {
    let bars = chained_series(&[100, 105, 98, 103, 110, 95, 100, 108, 112, 90]);
    let out = run(&bars, dec!(10000), &realistic_cost(), momentum).unwrap();
    let eq = curve(&out);

    // Straightforward reference: for every (peak-so-far, mark) pair, the worst fractional
    // decline. Written independently of the crate's single-pass implementation.
    let mut reference = Decimal::ZERO;
    for (i, &mark) in eq.iter().enumerate() {
        let peak = eq[..=i].iter().copied().max().unwrap();
        if peak > Decimal::ZERO {
            reference = reference.max((peak - mark) / peak);
        }
    }

    let m = Metrics::from_equity(&eq, 365.0);
    assert_eq!(m.max_drawdown, reference);
    assert!(m.max_drawdown >= Decimal::ZERO && m.max_drawdown <= Decimal::ONE);
    // Non-vacuous: this path actually draws down.
    assert!(m.max_drawdown > Decimal::ZERO);
}

#[test]
fn buy_and_hold_round_trip_has_exact_hand_computed_metrics() {
    // Zero cost, full deploy at bar 1's open (price 100, 1000 USDC → exactly 10 SOL), price
    // returning to its start: the curve is exactly [1000, 1100, 1200, 900, 1050, 1000], so
    // every accounting metric is hand-checkable with equality.
    let bars = chained_series(&[100, 110, 120, 90, 105, 100]);
    let out = run(&bars, dec!(1000), &CostModel::zero(), |_h, _w| dec!(1)).unwrap();
    let eq = curve(&out);

    assert_eq!(out.n_trades, 1);
    let expected = vec![
        dec!(1000),
        dec!(1100),
        dec!(1200),
        dec!(900),
        dec!(1050),
        dec!(1000),
    ];
    assert_eq!(eq, expected);

    let m = Metrics::from_equity(&eq, 365.0);
    // Round trip to the starting price at zero cost: exactly zero total return.
    assert_eq!(m.total_return, Decimal::ZERO);
    // Peak 1200 → trough 900 is exactly a 25% drawdown.
    assert_eq!(m.max_drawdown, dec!(0.25));
    assert_eq!(m.n_periods, bars.len() - 1);
}

#[test]
fn pure_cost_drag_makes_total_return_the_exact_negative_of_drawdown() {
    // Flat price with real costs: prices never move, so every equity change is a cost and the
    // curve is non-increasing from its initial peak. For such a curve the identities
    // max_drawdown = (initial − final) / initial and total_return = −max_drawdown are exact.
    static TARGETS: &[Decimal] = &[
        Decimal::ONE,
        Decimal::ZERO,
        Decimal::ONE,
        Decimal::ZERO,
        Decimal::ZERO,
    ];
    let initial = dec!(1000);
    let bars = chained_series(&[100; 6]);
    let out = run(&bars, initial, &realistic_cost(), scripted(TARGETS)).unwrap();
    let eq = curve(&out);

    assert_eq!(out.n_trades, 4);
    for pair in eq.windows(2) {
        assert!(
            pair[1] <= pair[0],
            "flat-price costed churn must never mark equity up: {} > {}",
            pair[1],
            pair[0]
        );
    }
    assert!(
        *eq.last().unwrap() < initial,
        "costed churn must lose money"
    );

    let m = Metrics::from_equity(&eq, 365.0);
    let final_equity = *eq.last().unwrap();
    assert_eq!(m.max_drawdown, (initial - final_equity) / initial);
    assert_eq!(m.total_return, -m.max_drawdown);
}

#[test]
fn exact_accounting_metrics_are_independent_of_annualization() {
    // periods_per_year feeds only the display-only statistical layer. The exact Decimal
    // metrics and the period count must be identical under wildly different annualization.
    let bars = chained_series(&[100, 105, 98, 103, 110, 95, 100, 108, 112, 90]);
    let out = run(&bars, dec!(10000), &realistic_cost(), momentum).unwrap();
    let eq = curve(&out);

    let daily = Metrics::from_equity(&eq, 365.0);
    let hourly = Metrics::from_equity(&eq, 8760.0);
    assert_eq!(daily.total_return, hourly.total_return);
    assert_eq!(daily.max_drawdown, hourly.max_drawdown);
    assert_eq!(daily.n_periods, hourly.n_periods);
    // And the statistical layer really does depend on it (guards against the test being
    // vacuous if annualization were ever dropped from the computation).
    assert_ne!(daily.volatility, hourly.volatility);
}
