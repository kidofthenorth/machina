//! UTC time. [`Timestamp`] is Unix seconds; ordering is total and deterministic.
//!
//! Formatting uses Howard Hinnant's `civil_from_days` algorithm so we can render RFC 3339 without a
//! date/time dependency; [`parse_ymd`] adds the strict inverse (`days_from_civil`) so config date
//! ranges (e.g. walk-forward partitions) parse to `Timestamp`s with zero deps. Both directions are
//! pure integer arithmetic.

use serde::{Deserialize, Serialize};

/// A point in time, as whole seconds since the Unix epoch, interpreted as UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Construct from Unix seconds (UTC).
    #[must_use]
    pub const fn from_unix(secs: i64) -> Self {
        Self(secs)
    }

    /// The underlying Unix seconds.
    #[must_use]
    pub const fn as_unix(self) -> i64 {
        self.0
    }

    /// Render as an RFC 3339 / ISO-8601 UTC string, e.g. `2021-01-01T00:00:00Z`.
    #[must_use]
    pub fn to_rfc3339(self) -> String {
        let days = self.0.div_euclid(86_400);
        let secs_of_day = self.0.rem_euclid(86_400);
        let (y, m, d) = civil_from_days(days);
        let hh = secs_of_day / 3_600;
        let mm = (secs_of_day % 3_600) / 60;
        let ss = secs_of_day % 60;
        format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
    }
}

/// Convert a count of days since 1970-01-01 into a `(year, month, day)` civil date.
/// Valid across the proleptic Gregorian calendar. (Hinnant, "chrono-Compatible Low-Level Date
/// Algorithms".)
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (y + i64::from(m <= 2), m, d)
}

/// Convert a `(year, month, day)` civil date to days since 1970-01-01. Hinnant's `days_from_civil`,
/// the exact inverse of [`civil_from_days`]. Assumes the inputs form a real date; [`parse_ymd`]
/// validates before calling.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400; // [0, 399]
    let mi = i64::from(m);
    let doy = (153 * (if m > 2 { mi - 3 } else { mi + 9 }) + 2) / 5 + (i64::from(d) - 1); // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Midnight-UTC Unix seconds for a `(year, month, day)` civil date — the inverse of the date part of
/// [`Timestamp::to_rfc3339`]. Does **not** validate the date; use [`parse_ymd`] for checked parsing.
#[must_use]
pub fn civil_date_to_unix(y: i64, m: u32, d: u32) -> i64 {
    days_from_civil(y, m, d) * 86_400
}

/// Why a `YYYY-MM-DD` string could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateParseError {
    /// Not a strict `YYYY-MM-DD` shape, or a field was not numeric.
    Malformed(String),
    /// Fields parsed but the date is not a real calendar day (e.g. `2025-02-30`).
    OutOfRange(String),
}

impl std::fmt::Display for DateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(s) => write!(f, "malformed date (expected YYYY-MM-DD): {s}"),
            Self::OutOfRange(s) => write!(f, "not a real calendar date: {s}"),
        }
    }
}

impl std::error::Error for DateParseError {}

/// Parse a strict `YYYY-MM-DD` UTC date into a midnight [`Timestamp`]. Rejects malformed input and
/// dates that are not real calendar days. Calendar validity is checked by round-tripping the day
/// count through [`civil_from_days`], so impossible days (`2025-02-30`, `2025-04-31`) are rejected.
///
/// # Errors
/// Returns [`DateParseError`] if the string is not strict `YYYY-MM-DD` or is not a real date.
pub fn parse_ymd(s: &str) -> Result<Timestamp, DateParseError> {
    let parts: Vec<&str> = s.split('-').collect();
    let well_formed = parts.len() == 3
        && parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit()));
    if !well_formed {
        return Err(DateParseError::Malformed(s.to_string()));
    }
    // Lengths are bounded above (4/2/2 digits) so these parses cannot overflow i64/u32.
    let y: i64 = parts[0].parse().expect("4 ascii digits fit i64");
    let m: u32 = parts[1].parse().expect("2 ascii digits fit u32");
    let d: u32 = parts[2].parse().expect("2 ascii digits fit u32");
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(DateParseError::OutOfRange(s.to_string()));
    }
    let days = days_from_civil(y, m, d);
    if civil_from_days(days) != (y, m, d) {
        return Err(DateParseError::OutOfRange(s.to_string()));
    }
    Ok(Timestamp::from_unix(days * 86_400))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_renders_correctly() {
        assert_eq!(Timestamp::from_unix(0).to_rfc3339(), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn known_dates_render_correctly() {
        // 2021-01-01T00:00:00Z = 1609459200
        assert_eq!(
            Timestamp::from_unix(1_609_459_200).to_rfc3339(),
            "2021-01-01T00:00:00Z"
        );
        // 2021-01-01T13:37:42Z = 1609508262
        assert_eq!(
            Timestamp::from_unix(1_609_508_262).to_rfc3339(),
            "2021-01-01T13:37:42Z"
        );
        // Leap day 2024-02-29T12:00:00Z = 1709208000
        assert_eq!(
            Timestamp::from_unix(1_709_208_000).to_rfc3339(),
            "2024-02-29T12:00:00Z"
        );
    }

    #[test]
    fn civil_date_to_unix_pinned_to_known_seconds() {
        // 2021-01-01T00:00:00Z and the leap day 2024-02-29 (midnight = the existing 12:00 value − 12h).
        assert_eq!(civil_date_to_unix(2021, 1, 1), 1_609_459_200);
        assert_eq!(civil_date_to_unix(2024, 2, 29), 1_709_208_000 - 43_200);
        // 2025-12-31T00:00:00Z.
        assert_eq!(civil_date_to_unix(2025, 12, 31), 1_767_139_200);
    }

    #[test]
    fn parse_ymd_is_inverse_of_to_rfc3339() {
        for s in ["2021-01-01", "2024-02-29", "2025-12-31", "1970-01-01"] {
            let ts = parse_ymd(s).unwrap();
            assert_eq!(ts.to_rfc3339(), format!("{s}T00:00:00Z"));
        }
    }

    #[test]
    fn parse_ymd_rejects_malformed_and_impossible_dates() {
        for bad in [
            "2025-13-01",
            "2025-00-10",
            "2025-02-30",
            "2025-04-31",
            "2025-01-00",
        ] {
            assert!(
                matches!(parse_ymd(bad), Err(DateParseError::OutOfRange(_))),
                "expected OutOfRange for {bad}"
            );
        }
        for bad in [
            "2025-1-1",
            "2025/01/01",
            "not-a-date",
            "2025-01",
            "20250101",
            "2025-aa-01",
        ] {
            assert!(
                matches!(parse_ymd(bad), Err(DateParseError::Malformed(_))),
                "expected Malformed for {bad}"
            );
        }
    }

    #[test]
    fn ordering_is_total_and_chronological() {
        let a = Timestamp::from_unix(100);
        let b = Timestamp::from_unix(200);
        assert!(a < b);
        let mut v = vec![b, a];
        v.sort();
        assert_eq!(v, vec![a, b]);
    }

    #[test]
    fn pre_epoch_timestamps_render_correctly() {
        // Negative Unix seconds rely on Euclidean div/rem for the right civil date AND time-of-day.
        assert_eq!(Timestamp::from_unix(-1).to_rfc3339(), "1969-12-31T23:59:59Z");
        assert_eq!(
            Timestamp::from_unix(-86_400).to_rfc3339(),
            "1969-12-31T00:00:00Z"
        );
        // 1960-01-01T00:00:00Z = -315_619_200 (10 years, 3 leap days, before epoch).
        assert_eq!(
            Timestamp::from_unix(-315_619_200).to_rfc3339(),
            "1960-01-01T00:00:00Z"
        );
    }

    #[test]
    fn far_future_and_pre_epoch_dates_round_trip() {
        for s in ["1900-01-01", "9999-12-31", "1969-12-31", "2000-02-29"] {
            let ts = parse_ymd(s).unwrap();
            assert_eq!(ts.to_rfc3339(), format!("{s}T00:00:00Z"));
        }
    }

    #[test]
    fn civil_date_to_unix_handles_pre_epoch() {
        assert_eq!(civil_date_to_unix(1970, 1, 1), 0);
        assert_eq!(civil_date_to_unix(1969, 12, 31), -86_400);
    }

    #[test]
    fn parse_ymd_applies_gregorian_leap_rules() {
        // Divisible by 400 → leap; divisible by 100 (not 400) → common; ordinary common year.
        assert!(parse_ymd("2000-02-29").is_ok());
        assert!(matches!(
            parse_ymd("1900-02-29"),
            Err(DateParseError::OutOfRange(_))
        ));
        assert!(matches!(
            parse_ymd("2023-02-29"),
            Err(DateParseError::OutOfRange(_))
        ));
    }

    #[test]
    fn date_parse_error_displays_reason() {
        assert!(parse_ymd("nope")
            .unwrap_err()
            .to_string()
            .contains("malformed"));
        assert!(parse_ymd("2025-02-30")
            .unwrap_err()
            .to_string()
            .contains("not a real calendar date"));
    }

    #[test]
    fn as_unix_round_trips_from_unix() {
        for s in [-1_000_000_i64, -1, 0, 1, 1_609_459_200, i64::from(u32::MAX)] {
            assert_eq!(Timestamp::from_unix(s).as_unix(), s);
        }
    }
}
