//! Binance monthly-kline CSV → `Vec<Bar>` loader (M5 card C2).
//!
//! Deterministic, dependency-free parsing (std + `rust_decimal` only — no `csv` crate). Hygiene
//! failures (malformed rows, misaligned timestamps) are hard errors: this loader never patches,
//! sorts, deduplicates, or forward-fills a snapshot. Ordering/spacing hygiene is a separate concern,
//! checked by [`crate::validation::validate_series_spacing`] once all files are loaded.

use research_core::{Bar, Timestamp};
use rust_decimal::Decimal;
use std::path::{Path, PathBuf};

/// A Binance kline CSV row's column count (`open_time,open,high,low,close,volume,close_time,…`);
/// only the first six columns are used.
const BINANCE_KLINE_COLUMNS: usize = 12;

/// Why loading a Binance kline CSV snapshot failed. Every variant is a hard stop — never repaired
/// (Q3: hygiene failures shrink the span, they are never patched or forward-filled).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinanceCsvError {
    /// Reading the snapshot directory or a file inside it failed.
    Io(String),
    /// A data row did not have the expected Binance kline column count.
    UnexpectedColumnCount {
        file: String,
        line_number: usize,
        columns: usize,
    },
    /// `open_time` is not an integer (and this wasn't the file's first/header line).
    InvalidTimestamp {
        file: String,
        line_number: usize,
        field: String,
    },
    /// `open_time` is ms/µs-scaled but not an exact multiple of its unit divisor.
    MisalignedTimestamp {
        file: String,
        line_number: usize,
        raw: i64,
    },
    /// A price/volume field did not parse as a `Decimal`.
    InvalidDecimal {
        file: String,
        line_number: usize,
        field: &'static str,
        value: String,
    },
}

impl std::fmt::Display for BinanceCsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "I/O error reading Binance CSV snapshot: {msg}"),
            Self::UnexpectedColumnCount {
                file,
                line_number,
                columns,
            } => write!(
                f,
                "{file}:{line_number}: expected {BINANCE_KLINE_COLUMNS} columns, found {columns}"
            ),
            Self::InvalidTimestamp {
                file,
                line_number,
                field,
            } => write!(
                f,
                "{file}:{line_number}: open_time {field:?} is not an integer"
            ),
            Self::MisalignedTimestamp {
                file,
                line_number,
                raw,
            } => write!(
                f,
                "{file}:{line_number}: open_time {raw} is not an exact multiple of its unit divisor"
            ),
            Self::InvalidDecimal {
                file,
                line_number,
                field,
                value,
            } => write!(
                f,
                "{file}:{line_number}: {field} {value:?} is not a valid decimal"
            ),
        }
    }
}

impl std::error::Error for BinanceCsvError {}

/// Load every `*.csv` file in `dir`, sorted by file name (byte order, deterministic), into one
/// concatenated `Vec<Bar>`. Rows are kept in file-then-row order; nothing is sorted, deduplicated,
/// or forward-filled here — series-level ordering and spacing hygiene are the caller's job via
/// [`crate::validation::validate_series_spacing`].
///
/// # Errors
/// Returns [`BinanceCsvError`] on any I/O failure or malformed row.
pub fn load_dir(dir: &Path) -> Result<Vec<Bar>, BinanceCsvError> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| BinanceCsvError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| BinanceCsvError::Io(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("csv") {
            paths.push(path);
        }
    }
    paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let mut bars = Vec::new();
    for path in &paths {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unnamed>")
            .to_string();
        let content =
            std::fs::read_to_string(path).map_err(|e| BinanceCsvError::Io(e.to_string()))?;
        parse_into(&content, &file_name, &mut bars)?;
    }
    Ok(bars)
}

/// Parse one file's CSV content, appending rows onto `bars` in file order. A line whose first field
/// fails to parse as an integer is tolerated ONLY at the file's first line (a header row); anywhere
/// else that is a hard error.
fn parse_into(content: &str, file_name: &str, bars: &mut Vec<Bar>) -> Result<(), BinanceCsvError> {
    for (index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        let line_number = index + 1;
        if index == 0 && fields[0].parse::<i64>().is_err() {
            continue; // header tolerance: only the file's first line may be non-numeric.
        }
        if fields.len() != BINANCE_KLINE_COLUMNS {
            return Err(BinanceCsvError::UnexpectedColumnCount {
                file: file_name.to_string(),
                line_number,
                columns: fields.len(),
            });
        }
        let raw_open_time: i64 =
            fields[0]
                .parse()
                .map_err(|_| BinanceCsvError::InvalidTimestamp {
                    file: file_name.to_string(),
                    line_number,
                    field: fields[0].to_string(),
                })?;
        let unix = normalize_open_time_secs(raw_open_time).ok_or_else(|| {
            BinanceCsvError::MisalignedTimestamp {
                file: file_name.to_string(),
                line_number,
                raw: raw_open_time,
            }
        })?;
        let parse_decimal = |col: usize, name: &'static str| -> Result<Decimal, BinanceCsvError> {
            fields[col]
                .parse::<Decimal>()
                .map_err(|_| BinanceCsvError::InvalidDecimal {
                    file: file_name.to_string(),
                    line_number,
                    field: name,
                    value: fields[col].to_string(),
                })
        };
        bars.push(Bar {
            ts: Timestamp::from_unix(unix),
            open: parse_decimal(1, "open")?,
            high: parse_decimal(2, "high")?,
            low: parse_decimal(3, "low")?,
            close: parse_decimal(4, "close")?,
            volume: parse_decimal(5, "volume")?,
        });
    }
    Ok(())
}

/// Normalize a Binance `open_time` to whole Unix seconds. Binance emits milliseconds (13 digits) or
/// microseconds (16 digits); a value that is not an exact multiple of its implied unit is rejected
/// rather than silently truncated.
fn normalize_open_time_secs(raw: i64) -> Option<i64> {
    let divisor: i64 = if raw >= 1_000_000_000_000_000 {
        1_000_000
    } else if raw >= 1_000_000_000_000 {
        1_000
    } else {
        1
    };
    (raw % divisor == 0).then_some(raw / divisor)
}

/// FNV-1a (64-bit) digest over a bar series: each bar contributes its Unix timestamp and the
/// canonical `to_string()` of its five `Decimal` fields, in order, separated by a fixed marker byte.
/// Pure integer arithmetic — this is the record's cross-run hygiene digest.
#[must_use]
pub fn fnv1a64(bars: &[Bar]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for bar in bars {
        let fields = [
            bar.ts.as_unix().to_string(),
            bar.open.to_string(),
            bar.high.to_string(),
            bar.low.to_string(),
            bar.close.to_string(),
            bar.volume.to_string(),
        ];
        for field in &fields {
            for byte in field.as_bytes() {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(PRIME);
            }
            hash ^= 0xFF; // field separator: avoids concatenation ambiguity between fields
            hash = hash.wrapping_mul(PRIME);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{validate_series, DataError};

    /// Three valid daily rows, ms-epoch `open_time`, 12 columns (only the first 6 are used).
    const THREE_VALID_ROWS: &str = "\
1609459200000,100,110,90,105,1000,0,0,0,0,0,0
1609545600000,105,115,95,110,1200,0,0,0,0,0,0
1609632000000,110,120,100,115,1300,0,0,0,0,0,0
";

    #[test]
    fn valid_three_line_csv_parses() {
        let mut bars = Vec::new();
        parse_into(THREE_VALID_ROWS, "t.csv", &mut bars).expect("parses");
        assert_eq!(bars.len(), 3);
        assert_eq!(bars[0].ts, Timestamp::from_unix(1_609_459_200));
        assert_eq!(bars[0].open.to_string(), "100");
        assert_eq!(bars[0].high.to_string(), "110");
        assert_eq!(bars[0].low.to_string(), "90");
        assert_eq!(bars[0].close.to_string(), "105");
        assert_eq!(bars[0].volume.to_string(), "1000");
        assert_eq!(bars[2].ts, Timestamp::from_unix(1_609_632_000));
    }

    #[test]
    fn header_line_is_tolerated() {
        let content = format!(
            "open_time,open,high,low,close,volume,close_time,qav,trades,tbav,tqav,ignore\n{THREE_VALID_ROWS}"
        );
        let mut bars = Vec::new();
        parse_into(&content, "t.csv", &mut bars).expect("header is skipped");
        assert_eq!(bars.len(), 3);
    }

    #[test]
    fn ms_and_micros_timestamps_normalize_identically() {
        let ms_row = "1609459200000,1,1,1,1,1,0,0,0,0,0,0\n";
        let us_row = "1609459200000000,1,1,1,1,1,0,0,0,0,0,0\n";
        let mut ms_bars = Vec::new();
        let mut us_bars = Vec::new();
        parse_into(ms_row, "ms.csv", &mut ms_bars).expect("ms parses");
        parse_into(us_row, "us.csv", &mut us_bars).expect("us parses");
        assert_eq!(ms_bars[0].ts, Timestamp::from_unix(1_609_459_200));
        assert_eq!(ms_bars[0].ts, us_bars[0].ts);
    }

    #[test]
    fn misaligned_timestamp_is_rejected() {
        // ms-range magnitude (>= 10^12) but not an exact multiple of 1000.
        let content = "1609459200500,1,1,1,1,1,0,0,0,0,0,0\n";
        let mut bars = Vec::new();
        assert!(matches!(
            parse_into(content, "t.csv", &mut bars).unwrap_err(),
            BinanceCsvError::MisalignedTimestamp {
                raw: 1_609_459_200_500,
                line_number: 1,
                ..
            }
        ));
    }

    #[test]
    fn unexpected_column_count_is_rejected() {
        let content = "1609459200000,1,1,1,1,1\n"; // only 6 columns
        let mut bars = Vec::new();
        assert!(matches!(
            parse_into(content, "t.csv", &mut bars).unwrap_err(),
            BinanceCsvError::UnexpectedColumnCount {
                columns: 6,
                line_number: 1,
                ..
            }
        ));
    }

    #[test]
    fn out_of_order_rows_are_rejected_by_validation() {
        // The loader itself performs no reordering; only downstream validation catches this.
        let content = "\
1609545600000,105,115,95,110,1200,0,0,0,0,0,0
1609459200000,100,110,90,105,1000,0,0,0,0,0,0
";
        let mut bars = Vec::new();
        parse_into(content, "t.csv", &mut bars).expect("loader does not reject ordering");
        assert_eq!(bars.len(), 2);
        assert!(matches!(
            validate_series(&bars).unwrap_err(),
            DataError::Unsorted { index: 1, .. }
        ));
    }

    #[test]
    fn digest_is_stable() {
        let mut bars = Vec::new();
        parse_into(THREE_VALID_ROWS, "t.csv", &mut bars).expect("parses");
        assert_eq!(fnv1a64(&bars), 0xf57d_42c4_827d_80e0);
    }
}
