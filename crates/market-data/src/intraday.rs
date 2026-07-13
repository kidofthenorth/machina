//! Intraday series hygiene: trade prints and slot snapshots (HF plan §2.1). Same discipline as
//! [`crate::validation`], same gap rule as daily bars: **missing data blocks a run unless the
//! caller explicitly declares a gap scenario by choosing the looser validator** — never
//! silently. At slot resolution gaps are frequent (skipped slots, outages), so snapshot series
//! treat slot gaps as the declared-scenario default ([`validate_snapshots`]) and offer a strict
//! contiguous check ([`validate_snapshots_contiguous`]) for gap-free segments.
//!
//! 1-second BARS need no new types: a 1s series is a plain `[Bar]` validated with
//! [`crate::validation::validate_series_spacing`]`(bars, 1)` (gap-blocking) or
//! [`crate::validation::validate_series`] (declared-gap scenario) — pinned by test in
//! `tests/intraday_validation.rs`.

use research_core::intraday::{IntradayItemError, SlotSnapshot, TradePrint};

/// An intraday data-hygiene failure. Every variant is a hard stop for a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntradayError {
    /// A run was asked to use an empty series.
    EmptySeries,
    /// All items in one series must come from one venue (cross-venue data = separate series).
    MixedVenue {
        index: usize,
        expected: String,
        found: String,
    },
    /// `(slot, seq)` (prints) or `slot` (snapshots) is not strictly increasing.
    OutOfOrder {
        index: usize,
        prev_slot: u64,
        cur_slot: u64,
    },
    /// Two items share the uniqueness key. `seq` is 0 for snapshot series (keyed by slot alone).
    DuplicateKey { index: usize, slot: u64, seq: u32 },
    /// Timestamps decrease while the slot advances.
    NonMonotonicTime { index: usize },
    /// A single item failed its invariants.
    Item(IntradayItemError),
    /// A hole in a contiguous-slot series (strict validator only).
    SlotGap {
        index: usize,
        prev_slot: u64,
        cur_slot: u64,
    },
}

impl std::fmt::Display for IntradayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySeries => write!(f, "intraday series is empty (a run requires data)"),
            Self::MixedVenue {
                index,
                expected,
                found,
            } => write!(f, "mixed venues at index {index}: expected {expected}, found {found}"),
            Self::OutOfOrder {
                index,
                prev_slot,
                cur_slot,
            } => write!(
                f,
                "items out of order at index {index}: slot {cur_slot} follows {prev_slot}"
            ),
            Self::DuplicateKey { index, slot, seq } => {
                write!(f, "duplicate key (slot {slot}, seq {seq}) at index {index}")
            }
            Self::NonMonotonicTime { index } => {
                write!(f, "timestamp decreases at index {index} while slot advances")
            }
            Self::Item(e) => write!(f, "{e}"),
            Self::SlotGap {
                index,
                prev_slot,
                cur_slot,
            } => write!(
                f,
                "missing slot(s) before index {index}: {cur_slot} follows {prev_slot} (expected contiguous)"
            ),
        }
    }
}

impl std::error::Error for IntradayError {}

impl From<IntradayItemError> for IntradayError {
    fn from(e: IntradayItemError) -> Self {
        Self::Item(e)
    }
}

/// Validate a single-venue trade-print series: non-empty, per-item invariants, one venue,
/// strictly increasing `(slot, seq)`, non-decreasing timestamps. Slot gaps are expected in
/// event data and are NOT an error here.
pub fn validate_prints(prints: &[TradePrint]) -> Result<(), IntradayError> {
    if prints.is_empty() {
        return Err(IntradayError::EmptySeries);
    }
    let venue = &prints[0].venue;
    for (i, p) in prints.iter().enumerate() {
        p.check()?;
        if &p.venue != venue {
            return Err(IntradayError::MixedVenue {
                index: i,
                expected: venue.clone(),
                found: p.venue.clone(),
            });
        }
        if i > 0 {
            let prev = (prints[i - 1].slot, prints[i - 1].seq);
            let cur = (p.slot, p.seq);
            if cur == prev {
                return Err(IntradayError::DuplicateKey {
                    index: i,
                    slot: p.slot,
                    seq: p.seq,
                });
            }
            if cur < prev {
                return Err(IntradayError::OutOfOrder {
                    index: i,
                    prev_slot: prints[i - 1].slot,
                    cur_slot: p.slot,
                });
            }
            if p.ts < prints[i - 1].ts {
                return Err(IntradayError::NonMonotonicTime { index: i });
            }
        }
    }
    Ok(())
}

/// Validate a single-venue snapshot series: non-empty, per-item invariants, one venue, strictly
/// increasing `slot`, non-decreasing timestamps. **Slot gaps are allowed here** — this is the
/// declared-scenario path, mirroring [`crate::validation::validate_series`] for daily bars.
pub fn validate_snapshots(snaps: &[SlotSnapshot]) -> Result<(), IntradayError> {
    if snaps.is_empty() {
        return Err(IntradayError::EmptySeries);
    }
    let venue = &snaps[0].venue;
    for (i, s) in snaps.iter().enumerate() {
        s.check()?;
        if &s.venue != venue {
            return Err(IntradayError::MixedVenue {
                index: i,
                expected: venue.clone(),
                found: s.venue.clone(),
            });
        }
        if i > 0 {
            let prev = snaps[i - 1].slot;
            if s.slot == prev {
                return Err(IntradayError::DuplicateKey {
                    index: i,
                    slot: s.slot,
                    seq: 0,
                });
            }
            if s.slot < prev {
                return Err(IntradayError::OutOfOrder {
                    index: i,
                    prev_slot: prev,
                    cur_slot: s.slot,
                });
            }
            if s.ts < snaps[i - 1].ts {
                return Err(IntradayError::NonMonotonicTime { index: i });
            }
        }
    }
    Ok(())
}

/// [`validate_snapshots`] AND require every slot advance to be exactly `+1`. The strict
/// gap-blocking path, mirroring [`crate::validation::validate_series_spacing`].
pub fn validate_snapshots_contiguous(snaps: &[SlotSnapshot]) -> Result<(), IntradayError> {
    validate_snapshots(snaps)?;
    for i in 1..snaps.len() {
        let prev = snaps[i - 1].slot;
        if snaps[i].slot != prev + 1 {
            return Err(IntradayError::SlotGap {
                index: i,
                prev_slot: prev,
                cur_slot: snaps[i].slot,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use research_core::{Side, Timestamp};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn print(venue: &str, slot: u64, seq: u32, ts: i64, price: Decimal) -> TradePrint {
        TradePrint {
            venue: venue.into(),
            slot,
            seq,
            ts: Timestamp::from_unix(ts),
            side: Side::Buy,
            price,
            size: dec!(1),
        }
    }

    fn snap(venue: &str, slot: u64, ts: i64, mid: Decimal) -> SlotSnapshot {
        SlotSnapshot {
            venue: venue.into(),
            slot,
            ts: Timestamp::from_unix(ts),
            mid,
        }
    }

    #[test]
    fn good_prints_pass() {
        let prints = vec![
            print("venue_a", 1000, 0, 1_609_459_200, dec!(100)),
            print("venue_a", 1000, 1, 1_609_459_200, dec!(100)),
            print("venue_a", 1002, 0, 1_609_459_201, dec!(100)),
            print("venue_a", 1005, 0, 1_609_459_202, dec!(100)),
        ];
        assert!(validate_prints(&prints).is_ok());
    }

    #[test]
    fn mixed_venue_rejected() {
        let prints = vec![
            print("venue_a", 1000, 0, 1_609_459_200, dec!(100)),
            print("venue_b", 1001, 0, 1_609_459_200, dec!(100)),
        ];
        assert_eq!(
            validate_prints(&prints),
            Err(IntradayError::MixedVenue {
                index: 1,
                expected: "venue_a".into(),
                found: "venue_b".into(),
            })
        );
    }

    #[test]
    fn out_of_order_and_duplicate_rejected() {
        let prints = vec![
            print("venue_a", 1002, 0, 1_609_459_200, dec!(100)),
            print("venue_a", 1000, 0, 1_609_459_200, dec!(100)),
        ];
        assert_eq!(
            validate_prints(&prints),
            Err(IntradayError::OutOfOrder {
                index: 1,
                prev_slot: 1002,
                cur_slot: 1000,
            })
        );

        let prints = vec![
            print("venue_a", 1000, 0, 1_609_459_200, dec!(100)),
            print("venue_a", 1000, 0, 1_609_459_200, dec!(100)),
        ];
        assert_eq!(
            validate_prints(&prints),
            Err(IntradayError::DuplicateKey {
                index: 1,
                slot: 1000,
                seq: 0,
            })
        );

        let prints = vec![
            print("venue_a", 1000, 1, 1_609_459_200, dec!(100)),
            print("venue_a", 1000, 0, 1_609_459_200, dec!(100)),
        ];
        assert_eq!(
            validate_prints(&prints),
            Err(IntradayError::OutOfOrder {
                index: 1,
                prev_slot: 1000,
                cur_slot: 1000,
            })
        );
    }

    #[test]
    fn non_monotonic_time_rejected() {
        let prints = vec![
            print("venue_a", 1000, 0, 1_609_459_201, dec!(100)),
            print("venue_a", 1001, 0, 1_609_459_200, dec!(100)),
        ];
        assert_eq!(
            validate_prints(&prints),
            Err(IntradayError::NonMonotonicTime { index: 1 })
        );
    }

    #[test]
    fn bad_item_rejected() {
        let mut bad = print("venue_a", 1000, 0, 1_609_459_200, dec!(100));
        bad.size = dec!(0);
        let prints = vec![bad];
        assert_eq!(
            validate_prints(&prints),
            Err(IntradayError::Item(IntradayItemError::NonPositiveSize {
                slot: 1000,
                seq: 0,
            }))
        );
    }

    #[test]
    fn snapshots_loose_allows_slot_gaps_strict_rejects() {
        let snaps = vec![
            snap("venue_a", 1000, 1_609_459_200, dec!(100)),
            snap("venue_a", 1001, 1_609_459_200, dec!(100)),
            snap("venue_a", 1003, 1_609_459_201, dec!(100)),
        ];
        assert!(validate_snapshots(&snaps).is_ok());
        assert_eq!(
            validate_snapshots_contiguous(&snaps),
            Err(IntradayError::SlotGap {
                index: 2,
                prev_slot: 1001,
                cur_slot: 1003,
            })
        );
    }

    #[test]
    fn empty_series_rejected() {
        assert_eq!(validate_prints(&[]), Err(IntradayError::EmptySeries));
        assert_eq!(validate_snapshots(&[]), Err(IntradayError::EmptySeries));
    }

    #[test]
    fn intraday_error_display_covers_variants() {
        let cases: Vec<(IntradayError, &str)> = vec![
            (IntradayError::EmptySeries, "empty"),
            (
                IntradayError::MixedVenue {
                    index: 1,
                    expected: "venue_a".into(),
                    found: "venue_b".into(),
                },
                "mixed venues",
            ),
            (
                IntradayError::OutOfOrder {
                    index: 1,
                    prev_slot: 1000,
                    cur_slot: 999,
                },
                "out of order",
            ),
            (
                IntradayError::DuplicateKey {
                    index: 1,
                    slot: 1000,
                    seq: 0,
                },
                "duplicate key",
            ),
            (
                IntradayError::NonMonotonicTime { index: 1 },
                "timestamp decreases",
            ),
            (
                IntradayError::Item(IntradayItemError::NonPositiveMid { slot: 1000 }),
                "mid must be > 0",
            ),
            (
                IntradayError::SlotGap {
                    index: 1,
                    prev_slot: 1000,
                    cur_slot: 1002,
                },
                "missing slot",
            ),
        ];
        for (err, phrase) in cases {
            assert!(
                err.to_string().contains(phrase),
                "{err:?} display missing {phrase:?}"
            );
        }
    }
}
