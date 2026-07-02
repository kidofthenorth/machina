//! Bar-series and token validation. Invalid data blocks a run; nothing is silently repaired.

use research_core::bar::BarError;
use research_core::{Bar, TokenMeta};

/// The maximum token decimals we consider sane (Solana SPL tokens are ≤ 18).
pub const MAX_TOKEN_DECIMALS: u32 = 18;

/// A data-hygiene failure. Every variant is a hard stop for a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    /// A run was asked to use an empty series.
    EmptySeries,
    /// Timestamps are not strictly increasing (out of order).
    Unsorted {
        index: usize,
        prev_unix: i64,
        cur_unix: i64,
    },
    /// Two bars share a timestamp.
    DuplicateTimestamp { index: usize, unix: i64 },
    /// A single bar failed its OHLC invariants.
    Ohlc(BarError),
    /// Token decimals are out of the sane range.
    BadDecimals { token_id: String, decimals: u32 },
    /// A token is not present in the active allowlist.
    NotAllowlisted { token_id: String },
    /// A hole in the series: the spacing between two bars is not the expected interval.
    /// Enforces "missing data blocks a run — no silent forward-fill".
    Gap {
        index: usize,
        expected_secs: i64,
        actual_secs: i64,
    },
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySeries => write!(f, "bar series is empty (a run requires data)"),
            Self::Unsorted {
                index,
                prev_unix,
                cur_unix,
            } => write!(
                f,
                "bars out of order at index {index}: {cur_unix} follows {prev_unix}"
            ),
            Self::DuplicateTimestamp { index, unix } => {
                write!(f, "duplicate timestamp {unix} at index {index}")
            }
            Self::Ohlc(e) => write!(f, "{e}"),
            Self::BadDecimals { token_id, decimals } => {
                write!(f, "token {token_id}: implausible decimals {decimals} (expected 0–{MAX_TOKEN_DECIMALS})")
            }
            Self::NotAllowlisted { token_id } => {
                write!(f, "token {token_id} is not in the active allowlist")
            }
            Self::Gap {
                index,
                expected_secs,
                actual_secs,
            } => write!(
                f,
                "missing bar(s) before index {index}: spacing {actual_secs}s != expected {expected_secs}s"
            ),
        }
    }
}

impl std::error::Error for DataError {}

impl From<BarError> for DataError {
    fn from(e: BarError) -> Self {
        Self::Ohlc(e)
    }
}

/// Validate a bar series for use in a deterministic run.
///
/// Enforces, in order: non-empty, each bar's OHLC invariants, strictly increasing timestamps
/// (which simultaneously rules out duplicates and out-of-order bars). Volume non-negativity is part
/// of the per-bar OHLC check.
pub fn validate_series(bars: &[Bar]) -> Result<(), DataError> {
    if bars.is_empty() {
        return Err(DataError::EmptySeries);
    }
    for (i, bar) in bars.iter().enumerate() {
        bar.check_ohlc()?;
        if i > 0 {
            let prev = bars[i - 1].ts.as_unix();
            let cur = bar.ts.as_unix();
            if cur == prev {
                return Err(DataError::DuplicateTimestamp {
                    index: i,
                    unix: cur,
                });
            }
            if cur < prev {
                return Err(DataError::Unsorted {
                    index: i,
                    prev_unix: prev,
                    cur_unix: cur,
                });
            }
        }
    }
    Ok(())
}

/// Validate a series AND require uniform `expected_interval_secs` spacing between consecutive bars.
///
/// This enforces the "missing data blocks a run — no silent forward-fill" rule (plan §11): a hole
/// between otherwise sorted, unique bars is rejected with [`DataError::Gap`]. Callers that
/// deliberately allow gaps (an explicit scenario) should use [`validate_series`] instead.
pub fn validate_series_spacing(bars: &[Bar], expected_interval_secs: i64) -> Result<(), DataError> {
    validate_series(bars)?;
    for i in 1..bars.len() {
        let delta = bars[i].ts.as_unix() - bars[i - 1].ts.as_unix();
        if delta != expected_interval_secs {
            return Err(DataError::Gap {
                index: i,
                expected_secs: expected_interval_secs,
                actual_secs: delta,
            });
        }
    }
    Ok(())
}

/// Sanity-check token decimals (0–18).
pub fn validate_token_decimals(meta: &TokenMeta) -> Result<(), DataError> {
    if meta.decimals > MAX_TOKEN_DECIMALS {
        return Err(DataError::BadDecimals {
            token_id: meta.token_id.clone(),
            decimals: meta.decimals,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use research_core::Timestamp;
    use rust_decimal_macros::dec;

    fn bar(unix: i64, o: i64, h: i64, l: i64, c: i64) -> Bar {
        Bar {
            ts: Timestamp::from_unix(unix),
            open: o.into(),
            high: h.into(),
            low: l.into(),
            close: c.into(),
            volume: dec!(1),
        }
    }

    #[test]
    fn good_series_passes() {
        let bars = vec![
            bar(100, 10, 12, 9, 11),
            bar(200, 11, 13, 10, 12),
            bar(300, 12, 14, 11, 13),
        ];
        assert!(validate_series(&bars).is_ok());
    }

    #[test]
    fn empty_series_rejected() {
        assert_eq!(validate_series(&[]).unwrap_err(), DataError::EmptySeries);
    }

    #[test]
    fn unsorted_rejected() {
        let bars = vec![bar(200, 10, 12, 9, 11), bar(100, 11, 13, 10, 12)];
        assert!(matches!(
            validate_series(&bars).unwrap_err(),
            DataError::Unsorted { index: 1, .. }
        ));
    }

    #[test]
    fn duplicate_rejected() {
        let bars = vec![bar(100, 10, 12, 9, 11), bar(100, 11, 13, 10, 12)];
        assert!(matches!(
            validate_series(&bars).unwrap_err(),
            DataError::DuplicateTimestamp {
                index: 1,
                unix: 100
            }
        ));
    }

    #[test]
    fn bad_ohlc_rejected() {
        // high < low
        let bars = vec![bar(100, 10, 8, 9, 9)];
        assert!(matches!(
            validate_series(&bars).unwrap_err(),
            DataError::Ohlc(_)
        ));
    }

    #[test]
    fn uniform_spacing_passes() {
        let bars = vec![
            bar(100, 10, 12, 9, 11),
            bar(200, 11, 13, 10, 12),
            bar(300, 12, 14, 11, 13),
        ];
        assert!(validate_series_spacing(&bars, 100).is_ok());
    }

    #[test]
    fn missing_bar_gap_rejected() {
        // 100, 200, then a hole (skips 300) to 400 with a 100s expected interval.
        let bars = vec![
            bar(100, 10, 12, 9, 11),
            bar(200, 11, 13, 10, 12),
            bar(400, 12, 14, 11, 13),
        ];
        assert!(matches!(
            validate_series_spacing(&bars, 100).unwrap_err(),
            DataError::Gap {
                index: 2,
                expected_secs: 100,
                actual_secs: 200
            }
        ));
    }

    #[test]
    fn bad_decimals_rejected() {
        let meta = TokenMeta {
            token_id: "WEIRD".into(),
            symbol: "WEIRD".into(),
            mint: research_core::MintAddress::new("So11111111111111111111111111111111111111112")
                .unwrap(),
            decimals: 255,
        };
        assert!(matches!(
            validate_token_decimals(&meta).unwrap_err(),
            DataError::BadDecimals { decimals: 255, .. }
        ));
    }

    #[test]
    fn single_valid_bar_passes_both_checks() {
        let one = [bar(100, 10, 12, 9, 11)];
        assert!(validate_series(&one).is_ok());
        // A one-bar series has no pairs, so spacing is trivially satisfied.
        assert!(validate_series_spacing(&one, 100).is_ok());
    }

    #[test]
    fn ohlc_failure_is_reported_for_a_later_bar() {
        // Second bar has high < low; the per-bar check fires regardless of position.
        let bars = vec![bar(100, 10, 12, 9, 11), bar(200, 10, 8, 9, 9)];
        assert!(matches!(
            validate_series(&bars).unwrap_err(),
            DataError::Ohlc(_)
        ));
    }

    #[test]
    fn bars_too_close_together_are_a_gap() {
        // Expected 100s spacing but 100→150 is only 50s — under-spacing is a gap too.
        let bars = vec![bar(100, 10, 12, 9, 11), bar(150, 11, 13, 10, 12)];
        assert!(matches!(
            validate_series_spacing(&bars, 100).unwrap_err(),
            DataError::Gap {
                index: 1,
                expected_secs: 100,
                actual_secs: 50
            }
        ));
    }

    #[test]
    fn spacing_check_catches_ordering_problems_first() {
        // Base validation (ordering/dups) runs before the spacing pass.
        let bars = vec![bar(200, 10, 12, 9, 11), bar(100, 11, 13, 10, 12)];
        assert!(matches!(
            validate_series_spacing(&bars, 100).unwrap_err(),
            DataError::Unsorted { .. }
        ));
    }

    #[test]
    fn token_decimals_boundary_18_ok_19_bad() {
        let mint =
            research_core::MintAddress::new("So11111111111111111111111111111111111111112").unwrap();
        let ok = TokenMeta {
            token_id: "T".into(),
            symbol: "T".into(),
            mint: mint.clone(),
            decimals: MAX_TOKEN_DECIMALS,
        };
        assert!(validate_token_decimals(&ok).is_ok());
        let bad = TokenMeta {
            token_id: "T".into(),
            symbol: "T".into(),
            mint,
            decimals: MAX_TOKEN_DECIMALS + 1,
        };
        assert!(matches!(
            validate_token_decimals(&bad).unwrap_err(),
            DataError::BadDecimals { decimals: 19, .. }
        ));
    }

    #[test]
    fn data_error_display_and_from_bar_error() {
        assert!(DataError::EmptySeries.to_string().contains("empty"));
        assert!(DataError::Unsorted {
            index: 1,
            prev_unix: 200,
            cur_unix: 100
        }
        .to_string()
        .contains("out of order"));
        assert!(DataError::DuplicateTimestamp { index: 1, unix: 100 }
            .to_string()
            .contains("duplicate"));
        assert!(DataError::Gap {
            index: 2,
            expected_secs: 100,
            actual_secs: 200
        }
        .to_string()
        .contains("missing bar"));
        assert!(DataError::NotAllowlisted {
            token_id: "WIF".into()
        }
        .to_string()
        .contains("allowlist"));
        assert!(DataError::BadDecimals {
            token_id: "X".into(),
            decimals: 99
        }
        .to_string()
        .contains("implausible decimals"));
        // From<BarError> wraps into Ohlc and forwards its message.
        let e: DataError = BarError::NegativeVolume {
            ts: Timestamp::from_unix(0),
        }
        .into();
        assert!(matches!(e, DataError::Ohlc(_)));
        assert!(e.to_string().contains("negative volume"));
    }
}
