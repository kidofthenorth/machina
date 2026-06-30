//! Performance metrics from an equity curve.
//!
//! **Accounting metrics** — total return and maximum drawdown — are computed in [`Decimal`] so they
//! are exact and so max drawdown is provably in `[0,1]`. **Statistical reporting metrics** — CAGR,
//! volatility, Sharpe, Sortino, Calmar — require roots/powers and are computed in `f64`. They are
//! *display-only* and never feed back into accounting (see `docs/invariants.md`, invariant 7).

use research_core::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Computed metrics for a single run.
#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    /// Exact total return over the whole curve, e.g. `0.25` = +25%.
    pub total_return: Decimal,
    /// Exact maximum drawdown, in `[0,1]`.
    pub max_drawdown: Decimal,
    /// Number of return periods used (curve length − 1).
    pub n_periods: usize,
    /// Compound annual growth rate (display-only). `None` if undefined.
    pub cagr: Option<f64>,
    /// Annualized volatility of period returns (display-only).
    pub volatility: Option<f64>,
    /// Sharpe ratio, risk-free = 0 (display-only).
    pub sharpe: Option<f64>,
    /// Sortino ratio, risk-free = 0 (display-only).
    pub sortino: Option<f64>,
    /// Calmar ratio = CAGR / max drawdown (display-only).
    pub calmar: Option<f64>,
}

impl Metrics {
    /// Compute metrics from an equity curve sampled once per bar.
    ///
    /// `periods_per_year` annualizes the statistical metrics (e.g. `365.0` for daily bars).
    #[must_use]
    pub fn from_equity(equity: &[Decimal], periods_per_year: f64) -> Self {
        let n = equity.len();
        if n == 0 {
            return Self::empty();
        }
        let total_return = total_return(equity);
        let max_drawdown = max_drawdown(equity);

        if n < 2 {
            return Self {
                total_return,
                max_drawdown,
                n_periods: 0,
                ..Self::empty()
            };
        }

        // Period simple returns in f64 (statistical layer only).
        let returns: Vec<f64> = equity
            .windows(2)
            .map(|w| {
                let prev = w[0].to_f64().unwrap_or(0.0);
                let cur = w[1].to_f64().unwrap_or(0.0);
                if prev == 0.0 {
                    0.0
                } else {
                    cur / prev - 1.0
                }
            })
            .collect();

        let n_periods = returns.len();
        let mean = returns.iter().sum::<f64>() / n_periods as f64;
        let volatility = sample_stdev(&returns, mean).map(|sd| sd * periods_per_year.sqrt());
        let sharpe = sample_stdev(&returns, mean)
            .filter(|sd| *sd > 0.0)
            .map(|sd| (mean / sd) * periods_per_year.sqrt());
        let sortino = downside_stdev(&returns)
            .filter(|dd| *dd > 0.0)
            .map(|dd| (mean / dd) * periods_per_year.sqrt());

        let years = n_periods as f64 / periods_per_year;
        let growth = total_return.to_f64().unwrap_or(0.0) + 1.0;
        let cagr = if years > 0.0 && growth > 0.0 {
            Some(growth.powf(1.0 / years) - 1.0)
        } else {
            None
        };
        let max_dd_f = max_drawdown.to_f64().unwrap_or(0.0);
        let calmar = match cagr {
            Some(c) if max_dd_f > 0.0 => Some(c / max_dd_f),
            _ => None,
        };

        Self {
            total_return,
            max_drawdown,
            n_periods,
            cagr,
            volatility,
            sharpe,
            sortino,
            calmar,
        }
    }

    fn empty() -> Self {
        Self {
            total_return: Decimal::ZERO,
            max_drawdown: Decimal::ZERO,
            n_periods: 0,
            cagr: None,
            volatility: None,
            sharpe: None,
            sortino: None,
            calmar: None,
        }
    }
}

/// Exact total return: `last / first − 1`. Zero if the curve is empty or starts at zero.
#[must_use]
pub fn total_return(equity: &[Decimal]) -> Decimal {
    match (equity.first(), equity.last()) {
        (Some(&first), Some(&last)) if first != Decimal::ZERO => last / first - Decimal::ONE,
        _ => Decimal::ZERO,
    }
}

/// Exact maximum drawdown in `[0,1]`: the largest peak-to-trough decline as a fraction of the peak.
#[must_use]
pub fn max_drawdown(equity: &[Decimal]) -> Decimal {
    let mut peak = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;
    let mut seen = false;
    for &e in equity {
        if !seen || e > peak {
            peak = e;
            seen = true;
        }
        if peak > Decimal::ZERO {
            let dd = (peak - e) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    // Defensive clamp; for non-negative equity this is already in [0,1].
    max_dd.clamp(Decimal::ZERO, Decimal::ONE)
}

/// Sample standard deviation (n−1). `None` if fewer than two samples.
fn sample_stdev(xs: &[f64], mean: f64) -> Option<f64> {
    if xs.len() < 2 {
        return None;
    }
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (xs.len() as f64 - 1.0);
    Some(var.sqrt())
}

/// Downside deviation: RMS of negative returns (target = 0). `None` if fewer than two samples.
fn downside_stdev(xs: &[f64]) -> Option<f64> {
    if xs.len() < 2 {
        return None;
    }
    let sumsq = xs
        .iter()
        .map(|x| if *x < 0.0 { x * x } else { 0.0 })
        .sum::<f64>();
    Some((sumsq / (xs.len() as f64 - 1.0)).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn total_return_basic() {
        assert_eq!(total_return(&[dec!(100), dec!(125)]), dec!(0.25));
        assert_eq!(total_return(&[dec!(100), dec!(50)]), dec!(-0.5));
        assert_eq!(total_return(&[]), dec!(0));
    }

    #[test]
    fn drawdown_is_in_unit_interval() {
        // Peak 120, trough 60 → dd 0.5.
        let eq = [dec!(100), dec!(120), dec!(60), dec!(90)];
        let dd = max_drawdown(&eq);
        assert_eq!(dd, dec!(0.5));
        assert!(dd >= dec!(0) && dd <= dec!(1));
    }

    #[test]
    fn monotonic_curve_has_zero_drawdown() {
        let eq = [dec!(100), dec!(110), dec!(120), dec!(130)];
        assert_eq!(max_drawdown(&eq), dec!(0));
    }

    #[test]
    fn drawdown_bounded_on_crash_to_near_zero() {
        let eq = [dec!(100), dec!(1)];
        let dd = max_drawdown(&eq);
        assert!(dd > dec!(0.98) && dd <= dec!(1));
    }

    #[test]
    fn statistical_metrics_present_for_volatile_curve() {
        let eq = [dec!(100), dec!(110), dec!(95), dec!(120), dec!(115)];
        let m = Metrics::from_equity(&eq, 365.0);
        assert!(m.volatility.is_some());
        assert!(m.sharpe.is_some());
        assert_eq!(m.n_periods, 4);
        assert!(m.max_drawdown >= dec!(0) && m.max_drawdown <= dec!(1));
    }

    #[test]
    fn flat_curve_has_no_volatility_or_sharpe() {
        let eq = [dec!(100), dec!(100), dec!(100)];
        let m = Metrics::from_equity(&eq, 365.0);
        assert_eq!(m.total_return, dec!(0));
        assert_eq!(m.volatility, Some(0.0));
        assert_eq!(m.sharpe, None); // zero stdev → undefined
    }

    #[test]
    fn single_point_curve_is_trivial() {
        let m = Metrics::from_equity(&[dec!(100)], 365.0);
        assert_eq!(m.total_return, dec!(0));
        assert_eq!(m.n_periods, 0);
        assert!(m.sharpe.is_none());
    }
}
