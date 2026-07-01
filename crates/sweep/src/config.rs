//! Partition configuration — how to cut a single bar series into development / validation / holdout.
//!
//! A [`PartitionSpec`] names the two cut points of a three-way split. It is resolved against a real
//! series — and rigorously validated — by [`crate::partition::PartitionedBars::from_spec`], which
//! rejects any spec that would leave a partition empty or out of order (the overlap rejection,
//! plan §6). Date-bounded specs are parsed with the zero-dependency S2 date parser
//! (`research_core::time::parse_ymd`), so config dates need no `chrono`-style dependency.

use research_core::{parse_ymd, DateParseError, Timestamp};

/// How to split one bar series into three contiguous, ordered partitions: **development**
/// `[0, val_start)`, **validation** `[val_start, holdout_start)`, and **holdout**
/// `[holdout_start, n)`.
///
/// The split is resolved into bar indices by [`crate::partition::PartitionedBars::from_spec`]. The
/// holdout is then physically moved into its own allocation so parameter selection can never reach
/// it (invariant 11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionSpec {
    /// Cut at two explicit bar indices.
    ByIndex {
        /// First index of the validation partition (equivalently, the development length).
        val_start: usize,
        /// First index of the holdout partition (equivalently, development + validation length).
        holdout_start: usize,
    },
    /// Cut at two UTC instants. A bar joins **development** if `ts < val_start`, **validation** if
    /// `val_start <= ts < holdout_start`, and **holdout** if `ts >= holdout_start` — the boundaries
    /// are half-open, so a bar landing exactly on a boundary falls into the later partition. The
    /// series must be sorted ascending (checked when the spec is resolved).
    ByDate {
        /// Inclusive start of the validation partition.
        val_start: Timestamp,
        /// Inclusive start of the holdout partition.
        holdout_start: Timestamp,
    },
}

impl PartitionSpec {
    /// Cut at two explicit bar indices.
    #[must_use]
    pub fn by_index(val_start: usize, holdout_start: usize) -> Self {
        Self::ByIndex {
            val_start,
            holdout_start,
        }
    }

    /// Cut at two strict `YYYY-MM-DD` UTC dates, parsed with the zero-dependency date parser
    /// (`research_core::time::parse_ymd`). Each date is the inclusive *start* of its partition.
    ///
    /// # Errors
    /// Returns [`DateParseError`] if either string is not a strict, real `YYYY-MM-DD` calendar date.
    pub fn by_date(val_start: &str, holdout_start: &str) -> Result<Self, DateParseError> {
        Ok(Self::ByDate {
            val_start: parse_ymd(val_start)?,
            holdout_start: parse_ymd(holdout_start)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_index_stores_cut_points() {
        assert_eq!(
            PartitionSpec::by_index(30, 45),
            PartitionSpec::ByIndex {
                val_start: 30,
                holdout_start: 45
            }
        );
    }

    #[test]
    fn by_date_parses_with_the_s2_parser() {
        let spec = PartitionSpec::by_date("2024-01-01", "2024-06-01").unwrap();
        assert_eq!(
            spec,
            PartitionSpec::ByDate {
                val_start: parse_ymd("2024-01-01").unwrap(),
                holdout_start: parse_ymd("2024-06-01").unwrap(),
            }
        );
    }

    #[test]
    fn by_date_rejects_a_malformed_date() {
        assert!(PartitionSpec::by_date("2024-13-01", "2024-06-01").is_err());
        assert!(PartitionSpec::by_date("2024-01-01", "not-a-date").is_err());
    }
}
