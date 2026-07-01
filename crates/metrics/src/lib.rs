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

    /// `(a - b).abs()` under a small tolerance, for the display-only f64 metrics.
    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn empty_curve_is_all_zero_and_none() {
        let m = Metrics::from_equity(&[], 365.0);
        assert_eq!(m.total_return, dec!(0));
        assert_eq!(m.max_drawdown, dec!(0));
        assert_eq!(m.n_periods, 0);
        assert!(m.cagr.is_none());
        assert!(m.volatility.is_none());
        assert!(m.sharpe.is_none());
        assert!(m.sortino.is_none());
        assert!(m.calmar.is_none());
    }

    #[test]
    fn total_return_guards_zero_and_negative_start() {
        // A zero starting value is undefined → guarded to 0 rather than dividing by zero.
        assert_eq!(total_return(&[dec!(0), dec!(100)]), dec!(0));
        // Single element: first == last → 0.
        assert_eq!(total_return(&[dec!(100)]), dec!(0));
    }

    #[test]
    fn max_drawdown_edge_inputs() {
        // Empty and single-point curves have no decline.
        assert_eq!(max_drawdown(&[]), dec!(0));
        assert_eq!(max_drawdown(&[dec!(100)]), dec!(0));
        // An all-zero curve never establishes a positive peak, so drawdown stays 0.
        assert_eq!(max_drawdown(&[dec!(0), dec!(0), dec!(0)]), dec!(0));
    }

    #[test]
    fn max_drawdown_clamps_when_equity_goes_negative() {
        // Peak 100 then -50 would be a 1.5 raw decline; the defensive clamp caps it at 1.
        assert_eq!(max_drawdown(&[dec!(100), dec!(-50)]), dec!(1));
    }

    #[test]
    fn max_drawdown_takes_the_largest_of_several_declines() {
        // Two separate declines: 100→80 (0.2) recovers to 150, then 150→90 (0.4). Max is 0.4.
        let eq = [dec!(100), dec!(80), dec!(150), dec!(90), dec!(150)];
        assert_eq!(max_drawdown(&eq), dec!(0.4));
    }

    #[test]
    fn monotonic_up_curve_has_no_downside_so_sortino_is_none() {
        // Every period return is positive → downside deviation is 0 → Sortino undefined.
        let m = Metrics::from_equity(&[dec!(100), dec!(110), dec!(120), dec!(130)], 365.0);
        assert!(m.sortino.is_none(), "no downside → Sortino undefined");
        // Volatility and Sharpe are still defined (there is dispersion around the mean).
        assert!(m.volatility.is_some());
        assert!(m.sharpe.is_some());
        // A rising curve has a strictly positive Sharpe.
        assert!(m.sharpe.unwrap() > 0.0);
    }

    #[test]
    fn total_loss_curve_has_no_cagr_or_calmar() {
        // Ending at zero equity → growth factor 0, so CAGR (a root of growth) is undefined, and
        // Calmar (CAGR / max_dd) is undefined in turn.
        let m = Metrics::from_equity(&[dec!(100), dec!(50), dec!(0)], 365.0);
        assert_eq!(m.total_return, dec!(-1));
        assert_eq!(m.max_drawdown, dec!(1));
        assert!(m.cagr.is_none(), "growth factor 0 → CAGR undefined");
        assert!(m.calmar.is_none(), "no CAGR → no Calmar");
    }

    #[test]
    fn declining_curve_has_negative_cagr_and_finite_calmar() {
        // Still solvent (growth > 0) so CAGR is defined and negative; with a real drawdown Calmar
        // is defined and equals CAGR / max_drawdown.
        let m = Metrics::from_equity(&[dec!(100), dec!(90), dec!(80), dec!(70)], 365.0);
        let cagr = m.cagr.expect("solvent curve has a CAGR");
        assert!(cagr < 0.0, "declining curve has negative CAGR: {cagr}");
        let calmar = m.calmar.expect("drawdown present → Calmar defined");
        let max_dd = m.max_drawdown.to_f64().unwrap();
        assert!(approx(calmar, cagr / max_dd));
    }

    #[test]
    fn two_point_curve_has_cagr_but_no_dispersion_metrics() {
        // One return period: not enough samples for a sample stdev (n−1 = 1 denominator needs ≥2
        // samples), so volatility/Sharpe/Sortino are None, but total return and CAGR are defined.
        let m = Metrics::from_equity(&[dec!(100), dec!(110)], 365.0);
        assert_eq!(m.n_periods, 1);
        assert_eq!(m.total_return, dec!(0.1));
        assert!(m.cagr.is_some());
        assert!(m.volatility.is_none());
        assert!(m.sharpe.is_none());
        assert!(m.sortino.is_none());
    }

    #[test]
    fn n_periods_is_curve_length_minus_one() {
        for len in 2..8usize {
            let eq: Vec<Decimal> = (0..len).map(|i| Decimal::from(100 + i as i64)).collect();
            assert_eq!(Metrics::from_equity(&eq, 365.0).n_periods, len - 1);
        }
    }

    #[test]
    fn volatility_annualizes_by_sqrt_of_periods_per_year() {
        // Same return series, different annualization factor → volatility scales by √(ratio).
        let eq = [dec!(100), dec!(110), dec!(95), dec!(120), dec!(105)];
        let v1 = Metrics::from_equity(&eq, 1.0).volatility.unwrap();
        let v4 = Metrics::from_equity(&eq, 4.0).volatility.unwrap();
        // √4 / √1 = 2.
        assert!(approx(v4, v1 * 2.0), "v1={v1} v4={v4}");
    }

    #[test]
    fn from_equity_is_deterministic() {
        // Determinism is a hard project invariant: identical inputs → identical metrics.
        let eq = [dec!(100), dec!(108), dec!(97), dec!(121), dec!(115)];
        assert_eq!(
            Metrics::from_equity(&eq, 365.0),
            Metrics::from_equity(&eq, 365.0)
        );
    }

    #[test]
    fn down_trending_curve_has_negative_sharpe() {
        let eq = [dec!(100), dec!(90), dec!(95), dec!(80), dec!(70)];
        let m = Metrics::from_equity(&eq, 365.0);
        assert!(m.sharpe.unwrap() < 0.0);
    }
}
