//! Exact turnover.
//!
//! `turnover = traded_notional_quote / mean_equity`, all in `Decimal`. It is sourced from
//! `portfolio::RunOutput::traded_notional_quote` (the sum of USDC notional moved per trade), **not**
//! from round trips — a rebalancer can churn heavily while closing zero round trips, so a
//! round-trip-based turnover would undercount exactly the strategies §25 wants measured. The ratio
//! is un-annualized so it can drive a `Decimal` rejection threshold without an `f64` path.

use research_core::Decimal;

/// Turnover ratio: traded notional divided by mean equity over the curve.
///
/// Returns `Decimal::ZERO` when the curve is empty or its mean equity is non-positive (mirrors the
/// `first != 0` guard in `metrics::total_return`), so a degenerate run cannot panic or divide by
/// zero. The summation is exact and order-independent in `Decimal`.
#[must_use]
pub fn turnover_ratio(traded_notional_quote: Decimal, equity: &[Decimal]) -> Decimal {
    if equity.is_empty() {
        return Decimal::ZERO;
    }
    let sum: Decimal = equity.iter().copied().sum();
    let mean = sum / Decimal::from(equity.len());
    if mean <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    traded_notional_quote / mean
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn ratio_is_traded_notional_over_mean_equity() {
        // mean equity = (1000 + 1000 + 1000) / 3 = 1000; notional 2000 → turnover 2.0.
        let equity = [dec!(1000), dec!(1000), dec!(1000)];
        assert_eq!(turnover_ratio(dec!(2000), &equity), dec!(2));
        // mean = (1000 + 2000) / 2 = 1500; notional 750 → 0.5.
        assert_eq!(
            turnover_ratio(dec!(750), &[dec!(1000), dec!(2000)]),
            dec!(0.5)
        );
    }

    #[test]
    fn zero_notional_and_degenerate_curves_are_zero() {
        assert_eq!(turnover_ratio(dec!(0), &[dec!(1000)]), dec!(0));
        assert_eq!(turnover_ratio(dec!(100), &[]), dec!(0));
        assert_eq!(turnover_ratio(dec!(100), &[dec!(0), dec!(0)]), dec!(0));
    }

    #[test]
    fn summation_is_order_independent() {
        let equity = [
            dec!(137.5),
            dec!(982.01),
            dec!(4.4),
            dec!(1000),
            dec!(0.333),
        ];
        let mut reversed = equity;
        reversed.reverse();
        assert_eq!(
            turnover_ratio(dec!(321.5), &equity),
            turnover_ratio(dec!(321.5), &reversed),
            "Decimal sum must be exact and order-independent"
        );
    }
}
