//! The deterministic spot simulator.
//!
//! Event ordering (plan §6): the strategy sees only *completed* bars `[0..t]` and the resulting
//! target weight is executed at bar `t`'s **open** price. This is next-bar execution: the final
//! bar's signal is never executed, so it cannot open a position. Equity is marked at each bar close.
//!
//! The strategy is supplied as a pure `FnMut(&[Bar], Decimal) -> Decimal`: given completed history
//! and the *current* SOL weight, it returns a target SOL weight in `[0,1]`. (Threshold rebalancing
//! needs the current weight to decide whether drift exceeds its band.) The simulator owns no
//! strategy logic and the strategy owns no execution state.

use crate::cost::{CostModel, Side};
use crate::equity::{EquityPoint, RoundTrip};
use crate::state::{PortfolioState, SimError, TradeOutcome};
use research_core::{Bar, Decimal, Timestamp};

/// Below this SOL amount the position is treated as flat.
fn dust() -> Decimal {
    Decimal::new(1, 9) // 1e-9 SOL
}

/// Skip rebalances whose notional is below this (USDC) to avoid sub-cent churn.
fn min_trade_notional() -> Decimal {
    Decimal::new(1, 2) // 0.01 USDC
}

/// Everything a single run produces. Deterministic for a given `(bars, cash, cost, strategy)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutput {
    pub equity_curve: Vec<EquityPoint>,
    pub round_trips: Vec<RoundTrip>,
    pub final_state: PortfolioState,
    pub n_trades: u32,
    /// Sum of USDC notional moved per executed trade (buy: USDC spent; sell: mid value of SOL sold,
    /// gas leg excluded). Counts every rebalance, including churn that never closes a round trip —
    /// so it is the correct base for turnover, which `round_trips` would undercount (see M4 plan §4).
    pub traded_notional_quote: Decimal,
    pub fees_paid_quote: Decimal,
    pub slippage_paid_quote: Decimal,
    pub gas_paid_sol: Decimal,
    pub priority_fees_paid_sol: Decimal,
    /// Bars whose close was held with a non-dust SOL position (for time-in-market).
    pub bars_in_market: u32,
}

impl RunOutput {
    /// Fraction of bars spent holding SOL, in `[0,1]`.
    #[must_use]
    pub fn time_in_market(&self) -> Decimal {
        if self.equity_curve.is_empty() {
            return Decimal::ZERO;
        }
        Decimal::from(self.bars_in_market) / Decimal::from(self.equity_curve.len())
    }
}

struct OpenPosition {
    entry_ts: Timestamp,
    entry_price: Decimal,
    qty_base: Decimal,
    quote_at_open: Decimal,
}

/// Run the deterministic simulation.
pub fn run<F>(
    bars: &[Bar],
    initial_cash_usdc: Decimal,
    cost: &CostModel,
    mut target_fn: F,
) -> Result<RunOutput, SimError>
where
    F: FnMut(&[Bar], Decimal) -> Decimal,
{
    if bars.is_empty() {
        return Err(SimError::NoBars);
    }

    let mut state = PortfolioState::new(initial_cash_usdc);
    let mut equity_curve = Vec::with_capacity(bars.len());
    let mut round_trips = Vec::new();
    let mut open: Option<OpenPosition> = None;
    let mut n_trades = 0u32;
    let mut traded_notional_quote = Decimal::ZERO;
    let mut fees_paid_quote = Decimal::ZERO;
    let mut slippage_paid_quote = Decimal::ZERO;
    let mut gas_paid_sol = Decimal::ZERO;
    let mut bars_in_market = 0u32;

    for (t, bar) in bars.iter().enumerate() {
        // 1. Execute the target decided from completed bars [0..t] at this bar's OPEN price.
        if t >= 1 {
            let exec_price = bar.open;
            let current_weight = current_weight(&state, exec_price);
            let target = clamp01(target_fn(&bars[..t], current_weight));
            let was_flat = state.base_balance <= dust();
            let quote_before = state.quote_balance;
            if let Some(outcome) = rebalance(&mut state, target, exec_price, cost)? {
                n_trades += 1;
                traded_notional_quote += traded_notional(&outcome);
                fees_paid_quote += outcome.dex_fee_quote;
                slippage_paid_quote += outcome.slippage_quote;
                gas_paid_sol += outcome.gas_sol;
                record_round_trip(
                    &mut open,
                    &mut round_trips,
                    &state,
                    &outcome,
                    bar.ts,
                    was_flat,
                    quote_before,
                );
            }
        }
        // 2. Mark equity at this bar's CLOSE.
        equity_curve.push(EquityPoint {
            ts: bar.ts,
            equity_quote: state.equity(bar.close),
        });
        if state.base_balance > dust() {
            bars_in_market += 1;
        }
    }

    let priority_fees_paid_sol = cost.priority_sol() * Decimal::from(n_trades);
    Ok(RunOutput {
        equity_curve,
        round_trips,
        final_state: state,
        n_trades,
        traded_notional_quote,
        fees_paid_quote,
        slippage_paid_quote,
        gas_paid_sol,
        priority_fees_paid_sol,
        bars_in_market,
    })
}

/// Move the portfolio toward `target` SOL weight at `price`. Returns the executed trade, if any.
fn rebalance(
    state: &mut PortfolioState,
    target: Decimal,
    price: Decimal,
    cost: &CostModel,
) -> Result<Option<TradeOutcome>, SimError> {
    let equity = state.equity(price);
    if equity <= Decimal::ZERO || price <= Decimal::ZERO {
        return Ok(None);
    }
    let desired_base = (target * equity) / price;
    let delta = desired_base - state.base_balance;
    if (delta * price).abs() < min_trade_notional() {
        return Ok(None);
    }

    if delta > Decimal::ZERO {
        // Buy: spend the smaller of the gap notional and available cash.
        let quote_in = (delta * price).min(state.quote_balance);
        if quote_in <= Decimal::ZERO {
            return Ok(None);
        }
        match state.apply_buy(quote_in, price, cost) {
            Ok(o) => Ok(Some(o)),
            // Trade too small to cover gas — skip rather than fail the run.
            Err(SimError::InsufficientSolForGas { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    } else {
        // Sell: dispose of the gap, but keep enough SOL to pay gas.
        let sellable = state.base_balance - cost.gas_sol();
        if sellable <= Decimal::ZERO {
            return Ok(None);
        }
        let base_in = (-delta).min(sellable);
        if base_in <= Decimal::ZERO {
            return Ok(None);
        }
        match state.apply_sell(base_in, price, cost) {
            Ok(o) => Ok(Some(o)),
            Err(SimError::InsufficientSolForGas { .. } | SimError::Oversell { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn record_round_trip(
    open: &mut Option<OpenPosition>,
    round_trips: &mut Vec<RoundTrip>,
    state: &PortfolioState,
    outcome: &TradeOutcome,
    ts: Timestamp,
    was_flat: bool,
    quote_before: Decimal,
) {
    let now_long = state.base_balance > dust();
    match outcome.side {
        Side::Buy if was_flat && now_long => {
            *open = Some(OpenPosition {
                entry_ts: ts,
                entry_price: outcome.exec_price,
                qty_base: state.base_balance,
                quote_at_open: quote_before,
            });
        }
        Side::Sell if !now_long => {
            if let Some(op) = open.take() {
                round_trips.push(RoundTrip {
                    entry_ts: op.entry_ts,
                    exit_ts: ts,
                    entry_price: op.entry_price,
                    exit_price: outcome.exec_price,
                    qty_base: op.qty_base,
                    // Exact realized P&L: cash at close − cash at open (includes all costs).
                    pnl_quote: state.quote_balance - op.quote_at_open,
                });
            }
        }
        _ => {}
    }
}

fn clamp01(w: Decimal) -> Decimal {
    w.max(Decimal::ZERO).min(Decimal::ONE)
}

/// USDC notional moved by one trade, at mid price, excluding the gas leg.
///
/// Buy: the exact USDC spent (`-quote_delta == quote_in`). Sell: the mid value of the SOL disposed;
/// `base_delta` on a sell is `-(base_in + gas_sol)`, so `base_in = |base_delta| - gas_sol` and the
/// mid notional is `(|base_delta| - gas_sol) * exec_price`. Both are exact `Decimal`.
fn traded_notional(outcome: &TradeOutcome) -> Decimal {
    match outcome.side {
        Side::Buy => -outcome.quote_delta,
        Side::Sell => (outcome.base_delta.abs() - outcome.gas_sol) * outcome.exec_price,
    }
}

/// Current SOL weight = base value / equity at `price`. Zero when flat or equity is non-positive.
fn current_weight(state: &PortfolioState, price: Decimal) -> Decimal {
    let equity = state.equity(price);
    if equity <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    (state.base_balance * price) / equity
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn bar(unix: i64, price: i64) -> Bar {
        let p = Decimal::from(price);
        Bar {
            ts: Timestamp::from_unix(unix),
            open: p,
            high: p,
            low: p,
            close: p,
            volume: dec!(1),
        }
    }

    /// A series of flat-price bars at `prices[i]` (open=high=low=close).
    fn series(prices: &[i64]) -> Vec<Bar> {
        prices
            .iter()
            .enumerate()
            .map(|(i, p)| bar(100 + i as i64 * 86_400, *p))
            .collect()
    }

    #[test]
    fn repeated_runs_are_deterministic() {
        let bars = series(&[100, 110, 90, 120, 130]);
        let cost = CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5000,
            priority_fee_lamports: 50000,
        };
        let strat = |_h: &[Bar], _w: Decimal| dec!(0.5);
        let a = run(&bars, dec!(10000), &cost, strat).unwrap();
        let b = run(&bars, dec!(10000), &cost, strat).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn buy_and_hold_deploys_then_holds() {
        let bars = series(&[100, 100, 100]);
        let out = run(&bars, dec!(1000), &CostModel::zero(), |_h, _w| dec!(1)).unwrap();
        // One trade: full deploy at bar 1 open; held after.
        assert_eq!(out.n_trades, 1);
        assert!(out.final_state.base_balance > dec!(9));
        assert_eq!(out.final_state.quote_balance, dec!(0));
    }

    #[test]
    fn final_bar_signal_cannot_open_a_position() {
        // Strategy says "go long only once we can see >= 3 bars of history". With 3 bars total, that
        // condition is first true when the strategy is called with all 3 bars — i.e. for execution
        // at a 4th bar that does not exist. So no position is ever opened.
        let bars = series(&[100, 100, 100]);
        let out = run(
            &bars,
            dec!(1000),
            &CostModel::zero(),
            |h: &[Bar], _w: Decimal| {
                if h.len() >= 3 {
                    dec!(1)
                } else {
                    dec!(0)
                }
            },
        )
        .unwrap();
        assert_eq!(out.n_trades, 0);
        assert_eq!(out.final_state.base_balance, dec!(0));
        assert_eq!(out.final_state.quote_balance, dec!(1000));
    }

    #[test]
    fn round_trip_records_realized_pnl_and_costs_are_a_drag() {
        // Go long at bar 1 (price 100), flat at bar 3 (price 120). Zero cost → positive realized P&L.
        let bars = series(&[100, 100, 110, 120]);
        let mut step = 0;
        let out = run(&bars, dec!(1000), &CostModel::zero(), |_h, _w| {
            step += 1;
            // targets executed at bars 1,2,3: long, long, flat
            if step >= 3 {
                dec!(0)
            } else {
                dec!(1)
            }
        })
        .unwrap();
        assert_eq!(out.round_trips.len(), 1);
        let rt = &out.round_trips[0];
        assert!(
            rt.pnl_quote > dec!(0),
            "expected profit on a rising round trip: {}",
            rt.pnl_quote
        );
        assert_eq!(rt.entry_price, dec!(100));
        assert_eq!(rt.exit_price, dec!(120));
    }

    #[test]
    fn costs_make_a_flat_price_round_trip_lose_money() {
        let bars = series(&[100, 100, 100, 100]);
        let cost = CostModel {
            dex_fee_bps: 30,
            slippage_bps: 50,
            base_fee_lamports: 5000,
            priority_fee_lamports: 50000,
        };
        let mut step = 0;
        let out = run(&bars, dec!(1000), &cost, |_h, _w| {
            step += 1;
            if step >= 3 {
                dec!(0)
            } else {
                dec!(1)
            }
        })
        .unwrap();
        assert_eq!(out.round_trips.len(), 1);
        assert!(
            out.round_trips[0].pnl_quote < dec!(0),
            "round trip at flat price must lose to costs: {}",
            out.round_trips[0].pnl_quote
        );
        assert!(out.fees_paid_quote > dec!(0));
        assert!(out.slippage_paid_quote > dec!(0));
    }

    #[test]
    fn traded_notional_matches_hand_computed_for_a_round_trip() {
        // Flat price 100, zero cost. Full deploy at bar 1 (1000 USDC in), full exit at bar 3
        // (10 SOL out at mid 100 = 1000). Hand total = 2000. A round trip is also closed.
        let bars = series(&[100, 100, 100, 100]);
        let mut step = 0;
        let out = run(&bars, dec!(1000), &CostModel::zero(), |_h, _w| {
            step += 1;
            if step >= 3 {
                dec!(0)
            } else {
                dec!(1)
            }
        })
        .unwrap();
        assert_eq!(out.n_trades, 2);
        assert_eq!(out.round_trips.len(), 1);
        assert_eq!(out.traded_notional_quote, dec!(2000));
    }

    #[test]
    fn traded_notional_counts_rebalancer_churn_with_zero_round_trips() {
        // Constant 50% target (Static5050-style) over a moving price never goes flat, so it closes
        // ZERO round trips while still trading — the exact case a round-trip-based turnover misses.
        // Zero cost. Hand: buy 500 @b1, sell 1.25 SOL @200=250 @b3, buy 187.5 @b4 → 937.5.
        let bars = series(&[100, 100, 200, 100]);
        let out = run(&bars, dec!(1000), &CostModel::zero(), |_h, _w| dec!(0.5)).unwrap();
        assert_eq!(out.round_trips.len(), 0, "rebalancer never goes flat");
        assert_eq!(out.n_trades, 3);
        assert!(out.traded_notional_quote > dec!(0));
        assert_eq!(out.traded_notional_quote, dec!(937.5));
    }

    #[test]
    fn traded_notional_is_zero_for_a_no_trade_run() {
        let bars = series(&[100, 110, 90, 120]);
        let out = run(&bars, dec!(1000), &CostModel::zero(), |_h, _w| dec!(0)).unwrap();
        assert_eq!(out.n_trades, 0);
        assert_eq!(out.traded_notional_quote, dec!(0));
    }

    #[test]
    fn empty_bars_rejected() {
        assert_eq!(
            run(&[], dec!(1000), &CostModel::zero(), |_h, _w| dec!(0)).unwrap_err(),
            SimError::NoBars
        );
    }
}
