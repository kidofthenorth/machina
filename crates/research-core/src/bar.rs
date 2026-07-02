//! OHLCV bars and their single-bar invariants.
//!
//! Series-level checks (sorted, unique, no gaps) live in `market-data`; a [`Bar`] only knows how to
//! validate itself.

use crate::time::Timestamp;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A single OHLCV bar. All price/volume fields are exact decimals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bar {
    /// Bar open time (UTC). Bars are keyed and sorted by this.
    pub ts: Timestamp,
    /// Opening price; must lie within `[low, high]`.
    pub open: Decimal,
    /// Highest traded price; must be ≥ `low`.
    pub high: Decimal,
    /// Lowest traded price; must be ≤ `high`.
    pub low: Decimal,
    /// Closing price; must lie within `[low, high]`. The simulator marks equity at this price.
    pub close: Decimal,
    /// Base-asset volume; must be ≥ 0.
    pub volume: Decimal,
}

impl Bar {
    /// Validate this bar's OHLC invariants:
    /// `low ≤ high`, `low ≤ open ≤ high`, `low ≤ close ≤ high`, and `volume ≥ 0`.
    pub fn check_ohlc(&self) -> Result<(), BarError> {
        if self.high < self.low {
            return Err(BarError::HighBelowLow { ts: self.ts });
        }
        if self.open < self.low || self.open > self.high {
            return Err(BarError::OpenOutOfRange { ts: self.ts });
        }
        if self.close < self.low || self.close > self.high {
            return Err(BarError::CloseOutOfRange { ts: self.ts });
        }
        if self.volume < Decimal::ZERO {
            return Err(BarError::NegativeVolume { ts: self.ts });
        }
        Ok(())
    }
}

/// A single-bar validation failure, tagged with the offending timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarError {
    /// `high` is below `low`.
    HighBelowLow {
        /// Open time of the offending bar.
        ts: Timestamp,
    },
    /// `open` lies outside `[low, high]`.
    OpenOutOfRange {
        /// Open time of the offending bar.
        ts: Timestamp,
    },
    /// `close` lies outside `[low, high]`.
    CloseOutOfRange {
        /// Open time of the offending bar.
        ts: Timestamp,
    },
    /// `volume` is negative.
    NegativeVolume {
        /// Open time of the offending bar.
        ts: Timestamp,
    },
}

impl std::fmt::Display for BarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HighBelowLow { ts } => write!(f, "bar {}: high < low", ts.to_rfc3339()),
            Self::OpenOutOfRange { ts } => {
                write!(f, "bar {}: open outside [low, high]", ts.to_rfc3339())
            }
            Self::CloseOutOfRange { ts } => {
                write!(f, "bar {}: close outside [low, high]", ts.to_rfc3339())
            }
            Self::NegativeVolume { ts } => write!(f, "bar {}: negative volume", ts.to_rfc3339()),
        }
    }
}

impl std::error::Error for BarError {}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn bar(o: Decimal, h: Decimal, l: Decimal, c: Decimal, v: Decimal) -> Bar {
        Bar {
            ts: Timestamp::from_unix(1_609_459_200),
            open: o,
            high: h,
            low: l,
            close: c,
            volume: v,
        }
    }

    #[test]
    fn valid_bar_passes() {
        assert!(bar(dec!(10), dec!(12), dec!(9), dec!(11), dec!(100))
            .check_ohlc()
            .is_ok());
        // Degenerate flat bar is valid.
        assert!(bar(dec!(10), dec!(10), dec!(10), dec!(10), dec!(0))
            .check_ohlc()
            .is_ok());
    }

    #[test]
    fn high_below_low_rejected() {
        let err = bar(dec!(10), dec!(8), dec!(9), dec!(9), dec!(1))
            .check_ohlc()
            .unwrap_err();
        assert!(matches!(err, BarError::HighBelowLow { .. }));
    }

    #[test]
    fn open_or_close_out_of_range_rejected() {
        assert!(matches!(
            bar(dec!(13), dec!(12), dec!(9), dec!(11), dec!(1))
                .check_ohlc()
                .unwrap_err(),
            BarError::OpenOutOfRange { .. }
        ));
        assert!(matches!(
            bar(dec!(10), dec!(12), dec!(9), dec!(8), dec!(1))
                .check_ohlc()
                .unwrap_err(),
            BarError::CloseOutOfRange { .. }
        ));
    }

    #[test]
    fn negative_volume_rejected() {
        assert!(matches!(
            bar(dec!(10), dec!(12), dec!(9), dec!(11), dec!(-1))
                .check_ohlc()
                .unwrap_err(),
            BarError::NegativeVolume { .. }
        ));
    }

    #[test]
    fn open_and_close_exactly_at_bounds_are_valid() {
        // open == low and close == high (and the mirror) are in range — the checks use ≤/≥.
        assert!(bar(dec!(9), dec!(12), dec!(9), dec!(12), dec!(1))
            .check_ohlc()
            .is_ok());
        assert!(bar(dec!(12), dec!(12), dec!(9), dec!(9), dec!(1))
            .check_ohlc()
            .is_ok());
    }

    #[test]
    fn zero_volume_ok_but_tiny_negative_rejected() {
        assert!(bar(dec!(10), dec!(10), dec!(10), dec!(10), dec!(0))
            .check_ohlc()
            .is_ok());
        assert!(matches!(
            bar(dec!(10), dec!(10), dec!(10), dec!(10), dec!(-0.000000001))
                .check_ohlc()
                .unwrap_err(),
            BarError::NegativeVolume { .. }
        ));
    }

    #[test]
    fn bar_error_display_includes_timestamp_and_reason() {
        let msg = bar(dec!(10), dec!(8), dec!(9), dec!(9), dec!(1))
            .check_ohlc()
            .unwrap_err()
            .to_string();
        assert!(
            msg.contains("2021-01-01"),
            "renders the offending ts: {msg}"
        );
        assert!(msg.contains("high < low"), "states the reason: {msg}");
    }

    #[test]
    fn bar_serde_round_trips() {
        let b = bar(dec!(10), dec!(12), dec!(9), dec!(11), dec!(100));
        let json = serde_json::to_string(&b).unwrap();
        let back: Bar = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
    }
}
