//! The pure unit of sweep work.
//!
//! [`eval_cell`] evaluates one `(ParamPoint, bars, CostModel)` triple into a [`CellResult`] by
//! wrapping the already-deterministic `portfolio::run` and `metrics::Metrics::from_equity`. It owns
//! no clock, no RNG, and no shared state, so its output value is a pure function of its inputs —
//! the property the parallel scheduler (S7) relies on for `parallel == sequential`.
//!
//! Money fields are exact `Decimal`. The display-only statistical metrics (`cagr`, `volatility`,
//! `sharpe`, `sortino`, `calmar`) are `f64`, but any non-finite value is normalized to `None` **at
//! construction** so structural equality between cell results can never flake on `NaN != NaN`.

use crate::param::{build_strategy, ParamPoint};
use crate::turnover::turnover_ratio;
use metrics::Metrics;
use portfolio::{run, CostModel, SimError};
use research_core::{Bar, Decimal};
use strategies::Strategy;

/// The deterministic result of evaluating one parameter point over one bar series and cost model.
///
/// Does **not** derive `Eq` (it holds `f64`); equality is `PartialEq`. Because every `f64` field is
/// finite-or-`None` and the underlying computation is deterministic, two results from identical
/// inputs are bit-identical and compare equal.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CellResult {
    // Decimal accounting fields (exact; used for all ordering/selection).
    pub total_return: Decimal,
    pub max_drawdown: Decimal,
    pub turnover: Decimal,
    pub n_trades: u32,
    pub final_equity: Decimal,
    pub traded_notional_quote: Decimal,
    pub fees_paid_quote: Decimal,
    pub slippage_paid_quote: Decimal,
    pub priority_fees_paid_sol: Decimal,
    pub time_in_market: Decimal,
    // Display-only statistics (non-finite normalized to None; never drive ordering).
    pub cagr: Option<f64>,
    pub volatility: Option<f64>,
    pub sharpe: Option<f64>,
    pub sortino: Option<f64>,
    pub calmar: Option<f64>,
}

/// Evaluate one parameter point over `bars` under `cost`. Pure and deterministic.
///
/// # Errors
/// Propagates `portfolio::SimError` (e.g. `NoBars` for an empty series).
pub fn eval_cell(
    point: &ParamPoint,
    bars: &[Bar],
    cost: &CostModel,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
) -> Result<CellResult, SimError> {
    eval_strategy(
        &*build_strategy(point),
        bars,
        cost,
        initial_cash_usdc,
        periods_per_year,
    )
}

/// Evaluate any intent-only [`Strategy`] over `bars` under `cost` into a [`CellResult`]. The shared
/// deterministic core of [`eval_cell`]; also the path baselines (`baseline.rs`) run through, so a
/// candidate and a cost-matched baseline are scored by byte-identical logic — no drift between them.
///
/// # Errors
/// Propagates `portfolio::SimError` (e.g. `NoBars` for an empty series).
pub(crate) fn eval_strategy(
    strat: &dyn Strategy,
    bars: &[Bar],
    cost: &CostModel,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
) -> Result<CellResult, SimError> {
    let out = run(bars, initial_cash_usdc, cost, |h, w| {
        strat.target_weight(h, w)
    })?;
    let equity: Vec<Decimal> = out.equity_curve.iter().map(|p| p.equity_quote).collect();
    let m = Metrics::from_equity(&equity, periods_per_year);
    let turnover = turnover_ratio(out.traded_notional_quote, &equity);
    // The simulator marks equity at each bar close, so the last point is the final marked equity.
    let final_equity = equity.last().copied().unwrap_or(Decimal::ZERO);

    Ok(CellResult {
        total_return: m.total_return,
        max_drawdown: m.max_drawdown,
        turnover,
        n_trades: out.n_trades,
        final_equity,
        traded_notional_quote: out.traded_notional_quote,
        fees_paid_quote: out.fees_paid_quote,
        slippage_paid_quote: out.slippage_paid_quote,
        priority_fees_paid_sol: out.priority_fees_paid_sol,
        time_in_market: out.time_in_market(),
        cagr: finite(m.cagr),
        volatility: finite(m.volatility),
        sharpe: finite(m.sharpe),
        sortino: finite(m.sortino),
        calmar: finite(m.calmar),
    })
}

/// Normalize a statistical metric: keep finite values, map `NaN`/`±∞`/absent to `None`.
fn finite(x: Option<f64>) -> Option<f64> {
    x.filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use research_core::Timestamp;
    use rust_decimal_macros::dec;

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

    #[test]
    fn eval_cell_matches_a_directly_wired_run() {
        let bars = series(&[100, 110, 105, 120, 130]);
        let cost = CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        };
        let point = ParamPoint::TrendAlloc {
            sma_period: 3,
            weight_above: dec!(0.75),
            weight_below: dec!(0),
        };
        let cell = eval_cell(&point, &bars, &cost, dec!(10000), 365.0).unwrap();

        // Wire the same thing directly through portfolio + metrics.
        let strat = build_strategy(&point);
        let out = run(&bars, dec!(10000), &cost, |h, w| strat.target_weight(h, w)).unwrap();
        let equity: Vec<Decimal> = out.equity_curve.iter().map(|p| p.equity_quote).collect();
        let m = Metrics::from_equity(&equity, 365.0);
        assert_eq!(cell.total_return, m.total_return);
        assert_eq!(cell.max_drawdown, m.max_drawdown);
        assert_eq!(cell.n_trades, out.n_trades);
        assert_eq!(cell.traded_notional_quote, out.traded_notional_quote);
        assert_eq!(cell.final_equity, *equity.last().unwrap());
    }

    #[test]
    fn turnover_is_exact_against_a_hand_computed_value() {
        // Threshold 50%/band-0 over a flat-100 series, zero cost: the only trade is the bar-1 deploy
        // of 500 USDC (5 SOL at 100); price never drifts so no rebalance fires. mean equity over the
        // four closes is 1000, so turnover = 500 / 1000 = 0.5 exactly, on a single trade.
        let bars = series(&[100, 100, 100, 100]);
        let point = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };
        let cell = eval_cell(&point, &bars, &CostModel::zero(), dec!(1000), 365.0).unwrap();
        assert_eq!(cell.n_trades, 1);
        assert_eq!(cell.traded_notional_quote, dec!(500));
        assert_eq!(cell.turnover, dec!(0.5));
    }

    #[test]
    fn rebalancer_has_nonzero_turnover_holdusdc_has_zero() {
        // Constant 50% target over a moving price churns without closing a round trip → turnover > 0.
        let bars = series(&[100, 100, 200, 100]);
        let reb = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };
        let cell = eval_cell(&reb, &bars, &CostModel::zero(), dec!(1000), 365.0).unwrap();
        assert!(
            cell.turnover > dec!(0),
            "rebalancer turnover: {}",
            cell.turnover
        );
        assert_eq!(cell.max_drawdown.min(dec!(0)), dec!(0)); // drawdown is in [0,1]

        // trend_alloc with below-weight 0 and an SMA that never triggers long → no trades, turnover 0.
        let flat = ParamPoint::TrendAlloc {
            sma_period: 100,
            weight_above: dec!(1),
            weight_below: dec!(0),
        };
        let none = eval_cell(&flat, &bars, &CostModel::zero(), dec!(1000), 365.0).unwrap();
        assert_eq!(none.n_trades, 0);
        assert_eq!(none.turnover, dec!(0));
    }

    #[test]
    fn nonfinite_statistics_are_normalized_to_none() {
        // A single-bar series has no returns → no statistical metrics; nothing non-finite survives.
        let bars = series(&[100]);
        let point = ParamPoint::TrendAlloc {
            sma_period: 1,
            weight_above: dec!(1),
            weight_below: dec!(0),
        };
        let cell = eval_cell(&point, &bars, &CostModel::zero(), dec!(1000), 365.0).unwrap();
        for v in [
            cell.cagr,
            cell.volatility,
            cell.sharpe,
            cell.sortino,
            cell.calmar,
        ]
        .into_iter()
        .flatten()
        {
            assert!(v.is_finite(), "no non-finite f64 may reach a CellResult");
        }
    }

    #[test]
    fn empty_bars_propagate_error() {
        let point = ParamPoint::TrendAlloc {
            sma_period: 1,
            weight_above: dec!(1),
            weight_below: dec!(0),
        };
        assert_eq!(
            eval_cell(&point, &[], &CostModel::zero(), dec!(1000), 365.0).unwrap_err(),
            SimError::NoBars
        );
    }
}
