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
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
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
    HighBelowLow { ts: Timestamp },
    OpenOutOfRange { ts: Timestamp },
    CloseOutOfRange { ts: Timestamp },
    NegativeVolume { ts: Timestamp },
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
}
