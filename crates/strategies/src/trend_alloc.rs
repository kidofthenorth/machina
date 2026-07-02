//! `trend_alloc_v1` — SOL/USDC trend allocation by simple-moving-average filter (M3 scaffold).
//!
//! When the latest close is above the SMA, target `weight_above` SOL; otherwise `weight_below`.
//! This is the simplest possible trend/risk-off filter and exists to exercise the engine
//! deterministically. It is **not tuned** and implies no edge. Candidate signals (EMA/ADX/Donchian)
//! and validation (walk-forward, regime splits) are M4–M5 work.

use crate::Strategy;
use research_core::{Bar, Decimal};

/// Trend allocation: above SMA → `weight_above`, below → `weight_below`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrendAllocV1 {
    /// SMA lookback in bars.
    pub sma_period: usize,
    /// Target SOL weight when price is above the SMA.
    pub weight_above: Decimal,
    /// Target SOL weight when price is at/below the SMA.
    pub weight_below: Decimal,
}

impl TrendAllocV1 {
    /// Illustrative default: 50-bar SMA, 75% SOL above / 0% below. NOT a recommendation.
    #[must_use]
    pub fn illustrative() -> Self {
        Self {
            sma_period: 50,
            weight_above: Decimal::new(75, 2),
            weight_below: Decimal::ZERO,
        }
    }
}

impl Strategy for TrendAllocV1 {
    fn name(&self) -> &str {
        "trend_alloc_v1"
    }

    fn target_weight(&self, history: &[Bar], _current_weight: Decimal) -> Decimal {
        // Not enough history to form the SMA → stay defensive (no new exposure).
        if self.sma_period == 0 || history.len() < self.sma_period {
            return self.weight_below;
        }
        let window = &history[history.len() - self.sma_period..];
        let sum: Decimal = window.iter().map(|b| b.close).sum();
        let sma = sum / Decimal::from(self.sma_period);
        let last_close = history[history.len() - 1].close;
        if last_close > sma {
            self.weight_above
        } else {
            self.weight_below
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
        let s = TrendAllocV1 {
            sma_period: 3,
            weight_above: dec!(1),
            weight_below: dec!(0),
        };
        assert_eq!(s.target_weight(&series(&[10, 11]), dec!(0)), dec!(0));
    }

    #[test]
    fn long_when_above_sma() {
        let s = TrendAllocV1 {
            sma_period: 3,
            weight_above: dec!(0.75),
            weight_below: dec!(0),
        };
        // closes 10,11,15 → SMA = 12, last = 15 > 12 → above.
        assert_eq!(s.target_weight(&series(&[10, 11, 15]), dec!(0)), dec!(0.75));
    }

    #[test]
    fn defensive_when_below_sma() {
        let s = TrendAllocV1 {
            sma_period: 3,
            weight_above: dec!(0.75),
            weight_below: dec!(0),
        };
        // closes 15,12,9 → SMA = 12, last = 9 < 12 → below.
        assert_eq!(s.target_weight(&series(&[15, 12, 9]), dec!(0)), dec!(0));
    }

    #[test]
    fn deterministic() {
        let s = TrendAllocV1::illustrative();
        let bars = series(&(0..60).collect::<Vec<_>>());
        assert_eq!(
            s.target_weight(&bars, dec!(0)),
            s.target_weight(&bars, dec!(0.5))
        );
    }

    #[test]
    fn zero_period_stays_defensive() {
        // A zero SMA period can't form a mean → return weight_below (avoids div-by-zero).
        let s = TrendAllocV1 {
            sma_period: 0,
            weight_above: dec!(1),
            weight_below: dec!(0.25),
        };
        assert_eq!(s.target_weight(&series(&[10, 20, 30]), dec!(0)), dec!(0.25));
    }

    #[test]
    fn exactly_at_sma_is_defensive() {
        // last_close == SMA is NOT "above" (strict `>`), so target is weight_below.
        let s = TrendAllocV1 {
            sma_period: 3,
            weight_above: dec!(0.75),
            weight_below: dec!(0),
        };
        assert_eq!(s.target_weight(&series(&[10, 10, 10]), dec!(0)), dec!(0));
    }

    #[test]
    fn only_the_last_sma_period_bars_matter() {
        let s = TrendAllocV1 {
            sma_period: 3,
            weight_above: dec!(1),
            weight_below: dec!(0),
        };
        // A huge early close is outside the 3-bar window: tail 10,11,15 → SMA 12, last 15 > 12.
        assert_eq!(
            s.target_weight(&series(&[1000, 1, 10, 11, 15]), dec!(0)),
            dec!(1)
        );
        // Same leading noise, tail ends below its own SMA → defensive.
        assert_eq!(
            s.target_weight(&series(&[1, 1000, 15, 12, 9]), dec!(0)),
            dec!(0)
        );
    }

    #[test]
    fn illustrative_defaults_are_stable() {
        let s = TrendAllocV1::illustrative();
        assert_eq!(s.sma_period, 50);
        assert_eq!(s.weight_above, dec!(0.75));
        assert_eq!(s.weight_below, dec!(0));
    }
}
