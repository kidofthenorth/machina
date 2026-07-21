//! The HF sibling of [`crate::cell`]: evaluates one `(ParamPoint | baseline, bars-slice, rung
//! bundle)` into an [`HfCellResult`] through `portfolio::run_hf_priced`. Mirrors `cell.rs` line
//! for line so candidates AND baselines score through the same priced core (the S10 no-drift
//! principle, applied to HF, decision D-f).

use crate::param::{build_strategy, ParamPoint};
use crate::turnover::turnover_ratio;
use metrics::Metrics;
use portfolio::{AdversarialModel, CongestionRegime, HfCostModel, HfError, LatencyPipeline};
use research_core::{Bar, Decimal};
use strategies::Strategy;

/// The deterministic result of evaluating one parameter point over one bar series and HF rung
/// bundle. Does **not** derive `Eq` (`cell` holds `f64`s via [`crate::cell::CellResult`]).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct HfCellResult {
    pub cell: crate::cell::CellResult,
    /// In-range attempts that did not land (`landed == false`).
    pub unlanded_orders: u32,
    /// Number of landed fills that drew an adverse-selection hit.
    pub adverse_selection_hits: u32,
    /// Total adverse-selection cost paid, in USDC.
    pub adverse_selection_paid_quote: Decimal,
}

/// Evaluate any intent-only [`Strategy`] over `bars` under `hf`/`adversarial`/`pipeline`/`regimes`
/// into an [`HfCellResult`]. The shared priced core baselines (`baseline.rs`) also run through.
///
/// # Errors
/// Propagates `portfolio::HfError` (e.g. `Sim(NoBars)` for an empty series, `RegimesLength` on a
/// length mismatch).
#[allow(clippy::too_many_arguments)]
pub(crate) fn eval_hf_strategy(
    strat: &dyn Strategy,
    bars: &[Bar],
    hf: &HfCostModel,
    adversarial: &AdversarialModel,
    pipeline: &LatencyPipeline,
    regimes: &[CongestionRegime],
    cell_id: u64,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
) -> Result<HfCellResult, HfError> {
    let out = portfolio::run_hf_priced(
        bars,
        initial_cash_usdc,
        hf,
        adversarial,
        pipeline,
        regimes,
        cell_id,
        |h, w| strat.target_weight(h, w),
    )?;
    let equity: Vec<Decimal> = out
        .base
        .base
        .equity_curve
        .iter()
        .map(|p| p.equity_quote)
        .collect();
    let m = Metrics::from_equity(&equity, periods_per_year);
    let turnover = turnover_ratio(out.base.base.traded_notional_quote, &equity);
    let final_equity = equity.last().copied().unwrap_or(Decimal::ZERO);

    Ok(HfCellResult {
        cell: crate::cell::CellResult {
            total_return: m.total_return,
            max_drawdown: m.max_drawdown,
            turnover,
            n_trades: out.base.base.n_trades,
            final_equity,
            traded_notional_quote: out.base.base.traded_notional_quote,
            fees_paid_quote: out.base.base.fees_paid_quote,
            slippage_paid_quote: out.base.base.slippage_paid_quote,
            priority_fees_paid_sol: out.base.base.priority_fees_paid_sol,
            time_in_market: out.base.base.time_in_market(),
            cagr: finite(m.cagr),
            volatility: finite(m.volatility),
            sharpe: finite(m.sharpe),
            sortino: finite(m.sortino),
            calmar: finite(m.calmar),
        },
        unlanded_orders: out.base.unlanded_orders,
        adverse_selection_hits: out.adverse_selection_hits,
        adverse_selection_paid_quote: out.adverse_selection_paid_quote,
    })
}

/// Evaluate one parameter point over `bars` under `hf`/`adversarial`/`pipeline`/`regimes`. Pure
/// and deterministic.
///
/// # Errors
/// Propagates `portfolio::HfError` (e.g. `Sim(NoBars)` for an empty series, `RegimesLength` on a
/// length mismatch).
#[allow(clippy::too_many_arguments)]
pub fn eval_hf_cell(
    point: &ParamPoint,
    bars: &[Bar],
    hf: &HfCostModel,
    adversarial: &AdversarialModel,
    pipeline: &LatencyPipeline,
    regimes: &[CongestionRegime],
    cell_id: u64,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
) -> Result<HfCellResult, HfError> {
    eval_hf_strategy(
        &*build_strategy(point),
        bars,
        hf,
        adversarial,
        pipeline,
        regimes,
        cell_id,
        initial_cash_usdc,
        periods_per_year,
    )
}

/// Normalize a statistical metric: keep finite values, map `NaN`/`±∞`/absent to `None`. Duplicated
/// from [`crate::cell`] rather than exported from there (this card is sweep-side only; `cell.rs`
/// stays byte-untouched).
fn finite(x: Option<f64>) -> Option<f64> {
    x.filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use portfolio::{
        AdversarialModel, CongestionPriorityTable, CongestionRegime, CostModel, DepthBand,
        DepthCurve, HfError, LandingOutcome, LatencyPipeline,
    };
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

    fn hf_model() -> HfCostModel {
        HfCostModel {
            base: CostModel {
                dex_fee_bps: 30,
                slippage_bps: 0,
                base_fee_lamports: 5_000,
                priority_fee_lamports: 0,
            },
            depth_curve: DepthCurve::new(vec![DepthBand {
                notional_upto: dec!(1_000_000),
                impact_bps: 15,
            }])
            .unwrap(),
            congestion_priority_table: CongestionPriorityTable::new(1_000, 2_000, 3_000).unwrap(),
            tip_bps: 5,
        }
    }

    fn zero_adversarial() -> AdversarialModel {
        AdversarialModel {
            sandwich_bps: 0,
            pickoff_bps: 0,
            p_adverse_num: 0,
            p_adverse_den: 1,
        }
    }

    fn pipeline_for(n: usize) -> LatencyPipeline {
        LatencyPipeline::new(
            1,
            (0..n)
                .map(|_| LandingOutcome {
                    offset_bars: 1,
                    landed: true,
                })
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn eval_hf_cell_matches_a_directly_wired_run_hf_priced() {
        let bars = series(&[100, 110, 105, 120, 130]);
        let hf = hf_model();
        let adversarial = zero_adversarial();
        let pipeline = pipeline_for(bars.len());
        let regimes = vec![CongestionRegime::Busy; bars.len()];
        let point = ParamPoint::TrendAlloc {
            sma_period: 3,
            weight_above: dec!(0.75),
            weight_below: dec!(0),
        };

        let cell = eval_hf_cell(
            &point,
            &bars,
            &hf,
            &adversarial,
            &pipeline,
            &regimes,
            7,
            dec!(10000),
            365.0,
        )
        .unwrap();

        let strat = build_strategy(&point);
        let out = portfolio::run_hf_priced(
            &bars,
            dec!(10000),
            &hf,
            &adversarial,
            &pipeline,
            &regimes,
            7,
            |h, w| strat.target_weight(h, w),
        )
        .unwrap();
        let equity: Vec<Decimal> = out
            .base
            .base
            .equity_curve
            .iter()
            .map(|p| p.equity_quote)
            .collect();
        let m = Metrics::from_equity(&equity, 365.0);
        let turnover = turnover_ratio(out.base.base.traded_notional_quote, &equity);

        assert_eq!(cell.cell.total_return, m.total_return);
        assert_eq!(cell.cell.max_drawdown, m.max_drawdown);
        assert_eq!(cell.cell.turnover, turnover);
        assert_eq!(cell.cell.n_trades, out.base.base.n_trades);
        assert_eq!(
            cell.cell.traded_notional_quote,
            out.base.base.traded_notional_quote
        );
        assert_eq!(cell.cell.final_equity, *equity.last().unwrap());
        assert_eq!(cell.unlanded_orders, out.base.unlanded_orders);
        assert_eq!(cell.adverse_selection_hits, out.adverse_selection_hits);
        assert_eq!(
            cell.adverse_selection_paid_quote,
            out.adverse_selection_paid_quote
        );
    }

    #[test]
    fn eval_hf_cell_is_deterministic_across_repeat_calls() {
        let bars = series(&[100, 105, 95, 110]);
        let hf = hf_model();
        let adversarial = zero_adversarial();
        let pipeline = pipeline_for(bars.len());
        let regimes = vec![CongestionRegime::Calm; bars.len()];
        let point = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };

        let a = eval_hf_cell(
            &point,
            &bars,
            &hf,
            &adversarial,
            &pipeline,
            &regimes,
            3,
            dec!(1000),
            365.0,
        )
        .unwrap();
        let b = eval_hf_cell(
            &point,
            &bars,
            &hf,
            &adversarial,
            &pipeline,
            &regimes,
            3,
            dec!(1000),
            365.0,
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn slippage_paid_quote_is_always_zero_on_the_priced_path() {
        let bars = series(&[100, 100, 100]);
        let hf = hf_model();
        let adversarial = zero_adversarial();
        let pipeline = pipeline_for(bars.len());
        let regimes = vec![CongestionRegime::Calm; bars.len()];
        let point = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };
        let cell = eval_hf_cell(
            &point,
            &bars,
            &hf,
            &adversarial,
            &pipeline,
            &regimes,
            1,
            dec!(1000),
            365.0,
        )
        .unwrap();
        assert_eq!(cell.cell.slippage_paid_quote, Decimal::ZERO);
    }

    #[test]
    fn empty_bars_propagate_hf_error() {
        let hf = hf_model();
        let adversarial = zero_adversarial();
        let pipeline = pipeline_for(0);
        let point = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };
        let err = eval_hf_cell(
            &point,
            &[],
            &hf,
            &adversarial,
            &pipeline,
            &[],
            1,
            dec!(1000),
            365.0,
        )
        .unwrap_err();
        assert_eq!(err, HfError::Sim(portfolio::SimError::NoBars));
    }

    #[test]
    fn regimes_length_mismatch_surfaces_hf_error() {
        let bars = series(&[100, 100]);
        let hf = hf_model();
        let adversarial = zero_adversarial();
        let pipeline = pipeline_for(bars.len());
        let point = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };
        let err = eval_hf_cell(
            &point,
            &bars,
            &hf,
            &adversarial,
            &pipeline,
            &[CongestionRegime::Calm],
            1,
            dec!(1000),
            365.0,
        )
        .unwrap_err();
        assert_eq!(
            err,
            HfError::RegimesLength {
                regimes: 1,
                bars: 2
            }
        );
    }
}
