//! `intraday_meanrev_v1` — SOL/USDC intraday mean-reversion by SMA deviation band (M-HF scaffold).
//!
//! Deterministic Decimal reformulation of the OU-process sketch (highfrequency-algo-plan.md:55):
//! anchor = SMA over `anchor_period` bars of the *same* series (no external reference series);
//! `deviation = (last_close - anchor) / anchor`. When strictly cheap (`deviation < -band`), go long
//! `weight_in`; otherwise stay at `weight_out`. No sqrt/stddev/z-score, no f64, no RNG — this is
//! **not tuned** and implies no edge.

use crate::Strategy;
use research_core::{Bar, Decimal};

/// Intraday mean reversion: below the SMA by more than `band` → `weight_in`, else `weight_out`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntradayMeanRevV1 {
    /// SMA anchor lookback in bars.
    pub anchor_period: usize,
    /// Deviation band (fraction, e.g. `0.01` = 1%) below which the price counts as "cheap".
    pub band: Decimal,
    /// Target SOL weight when the price is strictly below the band.
    pub weight_in: Decimal,
    /// Target SOL weight otherwise (defensive).
    pub weight_out: Decimal,
}

impl IntradayMeanRevV1 {
    /// Illustrative default: 60-bar (1-minute on 1s bars) anchor, 1% band, 50% SOL when cheap,
    /// flat otherwise. NOT a recommendation.
    #[must_use]
    pub fn illustrative() -> Self {
        Self {
            anchor_period: 60,
            band: Decimal::new(1, 2),
            weight_in: Decimal::new(5, 1),
            weight_out: Decimal::ZERO,
        }
    }
}

impl Strategy for IntradayMeanRevV1 {
    fn name(&self) -> &str {
        "intraday_meanrev_v1"
    }

    fn target_weight(&self, history: &[Bar], _current_weight: Decimal) -> Decimal {
        if self.anchor_period == 0 || history.len() < self.anchor_period {
            return self.weight_out;
        }
        let window = &history[history.len() - self.anchor_period..];
        let sum: Decimal = window.iter().map(|b| b.close).sum();
        let anchor = sum / Decimal::from(self.anchor_period);
        if anchor == Decimal::ZERO {
            return self.weight_out;
        }
        let last_close = history[history.len() - 1].close;
        let deviation = (last_close - anchor) / anchor;
        if deviation < -self.band {
            self.weight_in
        } else {
            self.weight_out
        }
    }
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
                    ts: Timestamp::from_unix(i as i64),
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
    fn defensive_until_enough_history() {
        let s = IntradayMeanRevV1 {
            anchor_period: 3,
            band: dec!(0.01),
            weight_in: dec!(1),
            weight_out: dec!(0),
        };
        assert_eq!(s.target_weight(&series(&[10, 11]), dec!(0)), dec!(0));
    }

    #[test]
    fn zero_period_stays_defensive() {
        let s = IntradayMeanRevV1 {
            anchor_period: 0,
            band: dec!(0.01),
            weight_in: dec!(1),
            weight_out: dec!(0.25),
        };
        assert_eq!(s.target_weight(&series(&[10, 20, 30]), dec!(0)), dec!(0.25));
    }

    #[test]
    fn zero_anchor_stays_defensive() {
        let s = IntradayMeanRevV1 {
            anchor_period: 3,
            band: dec!(0.01),
            weight_in: dec!(1),
            weight_out: dec!(0.25),
        };
        assert_eq!(s.target_weight(&series(&[0, 0, 0]), dec!(0)), dec!(0.25));
    }

    #[test]
    fn long_when_cheap_below_band() {
        let s = IntradayMeanRevV1 {
            anchor_period: 3,
            band: dec!(0.01),
            weight_in: dec!(0.5),
            weight_out: dec!(0),
        };
        // closes 100,100,97 → anchor ~99, last 97, deviation ≈ -0.0202 < -0.01 → weight_in.
        assert_eq!(
            s.target_weight(&series(&[100, 100, 97]), dec!(0)),
            dec!(0.5)
        );
    }

    #[test]
    fn defensive_when_at_or_above_anchor() {
        let s = IntradayMeanRevV1 {
            anchor_period: 3,
            band: dec!(0.01),
            weight_in: dec!(0.5),
            weight_out: dec!(0),
        };
        // closes 100,100,103 → anchor 101, last 103, deviation > 0 → not cheap → weight_out.
        assert_eq!(s.target_weight(&series(&[100, 100, 103]), dec!(0)), dec!(0));
    }

    #[test]
    fn band_edge_is_defensive() {
        let s = IntradayMeanRevV1 {
            anchor_period: 3,
            band: dec!(0.05),
            weight_in: dec!(1),
            weight_out: dec!(0),
        };
        // closes 95,100,105 → anchor 100, last 105 → deviation exactly 0.05 (not negative) →
        // weight_out. Symmetric edge check below covers exactly -band.
        assert_eq!(s.target_weight(&series(&[95, 100, 105]), dec!(0)), dec!(0));
        // closes 105,100,95 → anchor 100, last 95 → deviation exactly -0.05; strict `<` → weight_out.
        assert_eq!(s.target_weight(&series(&[105, 100, 95]), dec!(0)), dec!(0));
    }

    #[test]
    fn only_last_anchor_period_bars_matter() {
        let s = IntradayMeanRevV1 {
            anchor_period: 3,
            band: dec!(0.01),
            weight_in: dec!(1),
            weight_out: dec!(0),
        };
        // Leading noise outside the 3-bar window; tail 100,100,97 → cheap → weight_in.
        assert_eq!(
            s.target_weight(&series(&[9999, 100, 100, 97]), dec!(0)),
            dec!(1)
        );
        // Same leading noise, tail ends at/above anchor → defensive.
        assert_eq!(
            s.target_weight(&series(&[9999, 100, 100, 103]), dec!(0)),
            dec!(0)
        );
    }

    #[test]
    fn deterministic() {
        let s = IntradayMeanRevV1::illustrative();
        let bars = series(&(0..120).collect::<Vec<_>>());
        assert_eq!(
            s.target_weight(&bars, dec!(0)),
            s.target_weight(&bars, dec!(0.5))
        );
    }

    #[test]
    fn illustrative_defaults_are_stable() {
        let s = IntradayMeanRevV1::illustrative();
        assert_eq!(s.anchor_period, 60);
        assert_eq!(s.band, dec!(0.01));
        assert_eq!(s.weight_in, dec!(0.5));
        assert_eq!(s.weight_out, dec!(0));
    }
}
