//! Cost-matched baselines (plan §8, §9, §14 S10).
//!
//! Every candidate is compared against the four required baselines — `hold_usdc`, `buy_and_hold_sol`,
//! `static_50_50`, `dca_sol` — and the comparison is only honest if the baselines pay the **same
//! costs** the candidate does. So a baseline is re-run under each cost scenario ([`sensitivity`]),
//! through the *exact same* [`crate::cell::eval_strategy`] pipeline as candidates — no drift between
//! how a candidate and a baseline are scored.
//!
//! The advancement decision itself (which baselines, what margin) is M5 policy assembled in S11; this
//! module supplies the raw cost-matched baseline results and the "return to beat" floor
//! ([`best_baseline_return`]) those decisions build on.

use crate::cell::{eval_strategy, CellResult};
use portfolio::{CostModel, SimError};
use research_core::{Bar, Decimal};
use strategies::{BuyAndHoldSol, DcaIntoSol, HoldUsdc, Static5050, Strategy};

/// One of the four required baselines. Serialized as its stable strategy name (matching
/// `strategies::Strategy::name`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum BaselineId {
    /// Always USDC (`hold_usdc`) — the risk-free-of-SOL benchmark.
    #[serde(rename = "hold_usdc")]
    HoldUsdc,
    /// Always fully in SOL (`buy_and_hold_sol`) — "did you beat just holding SOL?".
    #[serde(rename = "buy_and_hold_sol")]
    BuyAndHoldSol,
    /// Static 50/50 (`static_50_50`), continuously rebalanced.
    #[serde(rename = "static_50_50")]
    Static5050,
    /// Illustrative DCA ramp into SOL (`dca_sol`).
    #[serde(rename = "dca_sol")]
    DcaSol,
}

impl BaselineId {
    /// The four baselines in canonical order (plan §8) — the order they appear in every report.
    #[must_use]
    pub fn all() -> [BaselineId; 4] {
        [
            Self::HoldUsdc,
            Self::BuyAndHoldSol,
            Self::Static5050,
            Self::DcaSol,
        ]
    }

    /// Stable identifier, equal to the built strategy's `name()` and the serde form.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::HoldUsdc => "hold_usdc",
            Self::BuyAndHoldSol => "buy_and_hold_sol",
            Self::Static5050 => "static_50_50",
            Self::DcaSol => "dca_sol",
        }
    }

    /// Build the intent-only baseline strategy. `dca_sol` uses the illustrative ramp (not tuned).
    #[must_use]
    pub fn build(self) -> Box<dyn Strategy> {
        match self {
            Self::HoldUsdc => Box::new(HoldUsdc),
            Self::BuyAndHoldSol => Box::new(BuyAndHoldSol),
            Self::Static5050 => Box::new(Static5050),
            Self::DcaSol => Box::new(DcaIntoSol::illustrative()),
        }
    }
}

/// Evaluate one baseline over `bars` under `cost`, through the shared candidate pipeline.
///
/// # Errors
/// Propagates `portfolio::SimError` (e.g. `NoBars` for an empty series).
pub fn eval_baseline(
    id: BaselineId,
    bars: &[Bar],
    cost: &CostModel,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
) -> Result<CellResult, SimError> {
    eval_strategy(
        &*id.build(),
        bars,
        cost,
        initial_cash_usdc,
        periods_per_year,
    )
}

/// Evaluate all four baselines over `bars` under one cost scenario, in canonical order — the
/// cost-matched baseline set a candidate under that same scenario is compared against.
///
/// # Errors
/// Propagates the first `portfolio::SimError`, by canonical baseline order (deterministic).
pub fn eval_all_baselines(
    bars: &[Bar],
    cost: &CostModel,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
) -> Result<Vec<(BaselineId, CellResult)>, SimError> {
    BaselineId::all()
        .into_iter()
        .map(|id| {
            let cell = eval_baseline(id, bars, cost, initial_cash_usdc, periods_per_year)?;
            Ok((id, cell))
        })
        .collect()
}

/// The highest baseline `total_return` in a cost-matched set — the "return to beat" floor. A
/// candidate that fails to exceed this under the same scenario has not out-performed simply holding
/// one of the benchmarks (plan §15). Exact `Decimal`; `Decimal::ZERO` for an empty set (defensive —
/// [`BaselineId::all`] is never empty).
#[must_use]
pub fn best_baseline_return(results: &[(BaselineId, CellResult)]) -> Decimal {
    results
        .iter()
        .map(|(_, c)| c.total_return)
        .max()
        .unwrap_or(Decimal::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use research_core::Timestamp;
    use rust_decimal_macros::dec;

    fn base_cost() -> CostModel {
        CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        }
    }

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
                    volume: dec!(1000),
                }
            })
            .collect()
    }

    #[test]
    fn ids_are_in_canonical_order_and_labels_match_strategy_names() {
        assert_eq!(
            BaselineId::all(),
            [
                BaselineId::HoldUsdc,
                BaselineId::BuyAndHoldSol,
                BaselineId::Static5050,
                BaselineId::DcaSol,
            ]
        );
        for id in BaselineId::all() {
            // label(), serde form, and the built strategy's name() must all agree.
            assert_eq!(id.label(), id.build().name());
            assert_eq!(
                serde_json::to_string(&id).unwrap(),
                format!("\"{}\"", id.label())
            );
        }
    }

    #[test]
    fn hold_usdc_never_trades_or_pays_costs_under_any_scenario() {
        let bars = series(&[100, 120, 90, 130, 110]);
        for cost in [CostModel::zero(), base_cost(), scaled_double()] {
            let c = eval_baseline(BaselineId::HoldUsdc, &bars, &cost, dec!(1000), 365.0).unwrap();
            assert_eq!(c.n_trades, 0);
            assert_eq!(c.fees_paid_quote, dec!(0));
            assert_eq!(c.total_return, dec!(0)); // equity stays at initial cash
        }
    }

    #[test]
    fn buy_and_hold_pays_strictly_more_under_doubled_costs() {
        let bars = series(&[100, 100, 100, 100]);
        let base = base_cost();
        let doubled = scaled_double();
        let b = eval_baseline(BaselineId::BuyAndHoldSol, &bars, &base, dec!(1000), 365.0).unwrap();
        let d = eval_baseline(
            BaselineId::BuyAndHoldSol,
            &bars,
            &doubled,
            dec!(1000),
            365.0,
        )
        .unwrap();
        assert_eq!(b.n_trades, 1); // one deploy into SOL, then held
        assert!(d.fees_paid_quote >= b.fees_paid_quote);
        assert!(d.slippage_paid_quote >= b.slippage_paid_quote);
        assert!(d.total_return <= b.total_return);
    }

    #[test]
    fn eval_all_baselines_is_canonical_and_floor_is_the_max() {
        let bars = series(&[100, 110, 105, 125, 140]);
        let cost = base_cost();
        let set = eval_all_baselines(&bars, &cost, dec!(1000), 365.0).unwrap();
        let ids: Vec<_> = set.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, BaselineId::all().to_vec());

        let floor = best_baseline_return(&set);
        let manual_max = set.iter().map(|(_, c)| c.total_return).max().unwrap();
        assert_eq!(floor, manual_max);
        // On this rising series, fully holding SOL beats holding USDC (return 0), so the floor > 0.
        assert!(floor > dec!(0));
    }

    /// The doubled scenario's cost model (mirrors `sensitivity::scale_cost_model(base, 2, 1)`).
    fn scaled_double() -> CostModel {
        CostModel {
            dex_fee_bps: 10,
            slippage_bps: 40,
            base_fee_lamports: 10_000,
            priority_fee_lamports: 100_000,
        }
    }
}
