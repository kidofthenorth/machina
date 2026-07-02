//! Regime classifier scaffold (M3 deliverable, plan §19/§26).
//!
//! A minimal, deterministic stub that labels recent price action as trending vs. range-bound, for
//! routing strategy families (trend systems in directional regimes, rebalancers in chop). This is a
//! SCAFFOLD: the heuristic is intentionally simple and untuned. Richer classification (ADX,
//! realized-volatility percentiles, walk-forward stability) is M4+ work and implies no edge.

use research_core::{Bar, Decimal};

/// A coarse market-regime label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// Not enough history to classify.
    Unknown,
    /// Price is trending (latest close deviates from the lookback mean beyond the threshold).
    Trending,
    /// Price is range-bound / choppy.
    RangeBound,
}

/// Classify the regime from `history` using a deviation-from-mean heuristic over `lookback` bars.
///
/// If the latest close deviates from the lookback mean by more than `threshold` (a fraction, e.g.
/// `0.05` = 5%), the regime is [`Regime::Trending`]; otherwise [`Regime::RangeBound`]. Returns
/// [`Regime::Unknown`] when there is insufficient history. Scaffold only — not tuned.
#[must_use]
pub fn classify(history: &[Bar], lookback: usize, threshold: Decimal) -> Regime {
    if lookback == 0 || history.len() < lookback {
        return Regime::Unknown;
    }
    let window = &history[history.len() - lookback..];
    let sum: Decimal = window.iter().map(|b| b.close).sum();
    let mean = sum / Decimal::from(lookback);
    if mean == Decimal::ZERO {
        return Regime::Unknown;
    }
    let last = history[history.len() - 1].close;
    let deviation = ((last - mean) / mean).abs();
    if deviation > threshold {
        Regime::Trending
    } else {
        Regime::RangeBound
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
    fn unknown_when_insufficient_history() {
        assert_eq!(
            classify(&series(&[100, 101]), 3, dec!(0.05)),
            Regime::Unknown
        );
        assert_eq!(
            classify(&series(&[100, 101, 102]), 0, dec!(0.05)),
            Regime::Unknown
        );
    }

    #[test]
    fn trending_when_far_from_mean() {
        // closes 100,100,130 → mean 110, last 130, deviation ≈ 0.18 > 0.05 → Trending.
        assert_eq!(
            classify(&series(&[100, 100, 130]), 3, dec!(0.05)),
            Regime::Trending
        );
    }

    #[test]
    fn range_bound_when_near_mean() {
        // closes 100,101,102 → mean 101, last 102, deviation ≈ 0.0099 < 0.05 → RangeBound.
        assert_eq!(
            classify(&series(&[100, 101, 102]), 3, dec!(0.05)),
            Regime::RangeBound
        );
    }

    #[test]
    fn zero_mean_is_unknown() {
        // All-zero closes → mean 0 → no deviation ratio is definable → Unknown (not a divide).
        assert_eq!(
            classify(&series(&[0, 0, 0]), 3, dec!(0.05)),
            Regime::Unknown
        );
    }

    #[test]
    fn deviation_exactly_at_threshold_is_range_bound() {
        // window 95,100,105 → mean 100, last 105 → deviation exactly 0.05; strict `>` → RangeBound.
        assert_eq!(
            classify(&series(&[95, 100, 105]), 3, dec!(0.05)),
            Regime::RangeBound
        );
    }

    #[test]
    fn downtrend_far_below_mean_is_trending() {
        // closes 100,100,70 → mean 90, last 70, |deviation| ≈ 0.22 > 0.05 → Trending (abs handles down).
        assert_eq!(
            classify(&series(&[100, 100, 70]), 3, dec!(0.05)),
            Regime::Trending
        );
    }

    #[test]
    fn only_the_last_lookback_bars_are_used() {
        // The leading 9999 is outside the 3-bar window; tail 95,100,105 → deviation 0.05 → RangeBound.
        assert_eq!(
            classify(&series(&[9999, 95, 100, 105]), 3, dec!(0.05)),
            Regime::RangeBound
        );
    }
}
