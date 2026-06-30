//! UTC time. [`Timestamp`] is Unix seconds; ordering is total and deterministic.
//!
//! Output formatting uses Howard Hinnant's `civil_from_days` algorithm so we can render RFC 3339
//! without pulling in a date/time dependency. Only seconds→string is needed (fixtures carry Unix
//! seconds), so no string→seconds parser lives here.

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
    fn ordering_is_total_and_chronological() {
        let a = Timestamp::from_unix(100);
        let b = Timestamp::from_unix(200);
        assert!(a < b);
        let mut v = vec![b, a];
        v.sort();
        assert_eq!(v, vec![a, b]);
    }
}
