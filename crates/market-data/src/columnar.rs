//! Fixed-record columnar bar storage (M-HF card C2.6, D-0013): a year of 1s bars (~31.5M rows)
//! must never materialize whole in memory. [`ColumnarWriter`] streams rows to disk in whatever
//! chunks the caller has on hand; [`ColumnarFile`] implements [`IntradaySource`] so a sweep cell
//! slices only its own window — peak memory scales with the window, never the file. `std::fs`/
//! `std::io` only: no memmap2, no arrow, no new dependency (D-0013 rejects both absent their own
//! recorded decision).

use research_core::{Bar, Decimal, Timestamp};
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::Path;

/// File magic: the first 4 bytes of every columnar file.
const MAGIC: &[u8; 4] = b"MCHC";
/// Format version. A future layout change bumps this and `open` rejects the old one.
const VERSION: u32 = 1;
/// Header layout: magic(4) + version(4) + scale(4) + reserved(4) + row_count(8).
const HEADER_LEN: usize = 24;
/// Byte offset of the row-count field within the header (patched in last, by `finish`).
const ROW_COUNT_OFFSET: u64 = 16;
/// Row layout: ts(8) + open/high/low/close/volume(8 each), little-endian.
const ROW_LEN: usize = 48;

/// Scale `d` to an integer mantissa at `scale` decimal places; `None` if it doesn't fit exactly
/// (lossy storage is a hygiene violation, not a warning).
fn to_scaled_i64(d: Decimal, scale: u32) -> Option<i64> {
    let mut scaled = d;
    scaled.rescale(scale); // rust_decimal: adjusts exponent, may round
    if scaled != d {
        return None; // rescale rounded ⇒ d does not fit exactly
    }
    scaled.mantissa().try_into().ok()
}

fn from_scaled_i64(m: i64, scale: u32) -> Decimal {
    Decimal::new(m, scale)
}

fn io_err(e: std::io::Error) -> ColumnarError {
    ColumnarError::Io(e.to_string())
}

/// Why a columnar read or write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnarError {
    Io(String),
    BadMagic,
    BadVersion(u32),
    /// A price/volume field did not fit exactly at the file's declared scale.
    ScaleOverflow {
        row: u64,
        field: &'static str,
    },
    /// The header's declared row count disagrees with the file's actual length.
    RowCountMismatch {
        header: u64,
        actual: u64,
    },
    /// A slice request read past the end of the file.
    SliceOutOfBounds {
        requested_end: usize,
        len: usize,
    },
}

impl std::fmt::Display for ColumnarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "I/O error on columnar file: {msg}"),
            Self::BadMagic => write!(f, "not a columnar file (bad magic)"),
            Self::BadVersion(v) => write!(f, "unsupported columnar format version {v}"),
            Self::ScaleOverflow { row, field } => {
                write!(
                    f,
                    "row {row}: {field} does not fit exactly at the file's scale"
                )
            }
            Self::RowCountMismatch { header, actual } => write!(
                f,
                "header declares {header} rows but the file contains {actual}"
            ),
            Self::SliceOutOfBounds { requested_end, len } => {
                write!(f, "slice end {requested_end} exceeds file length {len}")
            }
        }
    }
}

impl std::error::Error for ColumnarError {}

/// Streaming writer for the fixed-record columnar format. Chunked `append_bars` calls are the
/// point — the caller never holds more than one segment in memory at a time.
pub struct ColumnarWriter {
    writer: BufWriter<File>,
    scale: u32,
    row_count: u64,
}

impl ColumnarWriter {
    /// Create a new columnar file at `path`, writing a placeholder header (row count 0, patched
    /// in by [`Self::finish`]).
    ///
    /// # Errors
    /// Returns [`ColumnarError::Io`] if the file cannot be created or the header cannot be written.
    pub fn create(path: &Path, scale: u32) -> Result<Self, ColumnarError> {
        let file = File::create(path).map_err(io_err)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(MAGIC).map_err(io_err)?;
        writer.write_all(&VERSION.to_le_bytes()).map_err(io_err)?;
        writer.write_all(&scale.to_le_bytes()).map_err(io_err)?;
        writer.write_all(&0u32.to_le_bytes()).map_err(io_err)?; // reserved
        writer.write_all(&0u64.to_le_bytes()).map_err(io_err)?; // row count placeholder
        Ok(Self {
            writer,
            scale,
            row_count: 0,
        })
    }

    /// Append `bars` as rows, streaming through the internal `BufWriter`. Each field is scaled to
    /// an exact integer mantissa; a field that would round is a hard error — never rounded on
    /// write.
    ///
    /// # Errors
    /// Returns [`ColumnarError::ScaleOverflow`] if a field doesn't fit exactly at the file's
    /// scale, or [`ColumnarError::Io`] on a write failure.
    pub fn append_bars(&mut self, bars: &[Bar]) -> Result<(), ColumnarError> {
        for bar in bars {
            let row = self.row_count;
            let scale = self.scale;
            let open = to_scaled_i64(bar.open, scale)
                .ok_or(ColumnarError::ScaleOverflow { row, field: "open" })?;
            let high = to_scaled_i64(bar.high, scale)
                .ok_or(ColumnarError::ScaleOverflow { row, field: "high" })?;
            let low = to_scaled_i64(bar.low, scale)
                .ok_or(ColumnarError::ScaleOverflow { row, field: "low" })?;
            let close = to_scaled_i64(bar.close, scale).ok_or(ColumnarError::ScaleOverflow {
                row,
                field: "close",
            })?;
            let volume = to_scaled_i64(bar.volume, scale).ok_or(ColumnarError::ScaleOverflow {
                row,
                field: "volume",
            })?;
            self.writer
                .write_all(&bar.ts.as_unix().to_le_bytes())
                .map_err(io_err)?;
            self.writer.write_all(&open.to_le_bytes()).map_err(io_err)?;
            self.writer.write_all(&high.to_le_bytes()).map_err(io_err)?;
            self.writer.write_all(&low.to_le_bytes()).map_err(io_err)?;
            self.writer
                .write_all(&close.to_le_bytes())
                .map_err(io_err)?;
            self.writer
                .write_all(&volume.to_le_bytes())
                .map_err(io_err)?;
            self.row_count += 1;
        }
        Ok(())
    }

    /// Seek back and patch the header's row count, then flush. Returns the total rows written.
    ///
    /// # Errors
    /// Returns [`ColumnarError::Io`] if the seek, write, or flush fails.
    pub fn finish(mut self) -> Result<u64, ColumnarError> {
        self.writer.flush().map_err(io_err)?;
        self.writer
            .seek(SeekFrom::Start(ROW_COUNT_OFFSET))
            .map_err(io_err)?;
        self.writer
            .write_all(&self.row_count.to_le_bytes())
            .map_err(io_err)?;
        self.writer.flush().map_err(io_err)?;
        Ok(self.row_count)
    }
}

/// Slices a bar series without materializing more than the requested window.
pub trait IntradaySource {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Materialize exactly the requested window — never the whole file.
    ///
    /// # Errors
    /// Returns [`ColumnarError::SliceOutOfBounds`] if `range` extends past [`Self::len`], or
    /// [`ColumnarError::Io`] on a read failure.
    fn slice(&self, range: Range<usize>) -> Result<Vec<Bar>, ColumnarError>;
}

/// An opened columnar file: header already validated, ready for random-access slicing.
#[derive(Debug)]
pub struct ColumnarFile {
    file: File,
    scale: u32,
    row_count: u64,
}

impl ColumnarFile {
    /// Open and validate a columnar file's header: magic, version, and that the file length
    /// matches the declared row count exactly.
    ///
    /// # Errors
    /// Returns [`ColumnarError::BadMagic`], [`ColumnarError::BadVersion`],
    /// [`ColumnarError::RowCountMismatch`], or [`ColumnarError::Io`].
    pub fn open(path: &Path) -> Result<Self, ColumnarError> {
        let mut file = File::open(path).map_err(io_err)?;
        let mut header = [0u8; HEADER_LEN];
        file.read_exact(&mut header).map_err(io_err)?;
        let magic: [u8; 4] = header[0..4].try_into().expect("4-byte slice");
        if magic != *MAGIC {
            return Err(ColumnarError::BadMagic);
        }
        let version = u32::from_le_bytes(header[4..8].try_into().expect("4-byte slice"));
        if version != VERSION {
            return Err(ColumnarError::BadVersion(version));
        }
        let scale = u32::from_le_bytes(header[8..12].try_into().expect("4-byte slice"));
        let row_count = u64::from_le_bytes(header[16..24].try_into().expect("8-byte slice"));
        let file_len = file.metadata().map_err(io_err)?.len();
        let expected_len = HEADER_LEN as u64 + ROW_LEN as u64 * row_count;
        if file_len != expected_len {
            let actual = file_len.saturating_sub(HEADER_LEN as u64) / ROW_LEN as u64;
            return Err(ColumnarError::RowCountMismatch {
                header: row_count,
                actual,
            });
        }
        Ok(Self {
            file,
            scale,
            row_count,
        })
    }
}

impl IntradaySource for ColumnarFile {
    fn len(&self) -> usize {
        self.row_count as usize
    }

    fn slice(&self, range: Range<usize>) -> Result<Vec<Bar>, ColumnarError> {
        if range.start > range.end || range.end > self.len() {
            return Err(ColumnarError::SliceOutOfBounds {
                requested_end: range.end,
                len: self.len(),
            });
        }
        let n = range.end - range.start;
        // &File implements Read + Seek, so a shared ColumnarFile can still seek+read without
        // needing an interior-mutability wrapper.
        let mut file = &self.file;
        file.seek(SeekFrom::Start(
            HEADER_LEN as u64 + ROW_LEN as u64 * range.start as u64,
        ))
        .map_err(io_err)?;
        // One bulk read sized to the window — memory scales with `n`, never with the file.
        let mut buf = vec![0u8; ROW_LEN * n];
        file.read_exact(&mut buf).map_err(io_err)?;
        let mut bars = Vec::with_capacity(n);
        for chunk in buf.chunks_exact(ROW_LEN) {
            let row: [u8; ROW_LEN] = chunk.try_into().expect("chunks_exact(ROW_LEN) chunk");
            bars.push(decode_row(&row, self.scale));
        }
        Ok(bars)
    }
}

fn decode_row(row: &[u8; ROW_LEN], scale: u32) -> Bar {
    let ts = i64::from_le_bytes(row[0..8].try_into().expect("8-byte slice"));
    let open = i64::from_le_bytes(row[8..16].try_into().expect("8-byte slice"));
    let high = i64::from_le_bytes(row[16..24].try_into().expect("8-byte slice"));
    let low = i64::from_le_bytes(row[24..32].try_into().expect("8-byte slice"));
    let close = i64::from_le_bytes(row[32..40].try_into().expect("8-byte slice"));
    let volume = i64::from_le_bytes(row[40..48].try_into().expect("8-byte slice"));
    Bar {
        ts: Timestamp::from_unix(ts),
        open: from_scaled_i64(open, scale),
        high: from_scaled_i64(high, scale),
        low: from_scaled_i64(low, scale),
        close: from_scaled_i64(close, scale),
        volume: from_scaled_i64(volume, scale),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::{generate, SyntheticSpec};
    use rust_decimal_macros::dec;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "machina-columnar-test-{}-{name}.mcol",
            std::process::id()
        ))
    }

    /// A small C2-generated series. `round_dp(9)` in the generator guarantees every price fits
    /// exactly at scale 9; volumes are plain integers, which trivially fit too.
    fn small_series() -> Vec<Bar> {
        let spec = SyntheticSpec {
            seed: 11,
            steps: 500,
            start_unix: 1_704_067_200,
            start_slot: 100_000,
            venue: "venue_a".to_string(),
            mid0: dec!(100),
            anchor: dec!(100),
            reversion: dec!(0.05),
            vol_step: dec!(0.2),
            impact_every: 0,
            impact_size: dec!(0),
            impact_decay: dec!(0.5),
            regime_period: 100,
        };
        generate(&spec).expect("generate small series").bars_1s
    }

    fn one_bar(close: Decimal) -> Vec<Bar> {
        vec![Bar {
            ts: Timestamp::from_unix(1_700_000_000),
            open: dec!(1.20),
            high: dec!(1.30),
            low: dec!(1.10),
            close,
            volume: dec!(5),
        }]
    }

    #[test]
    fn round_trip_is_exact() {
        let bars = small_series();
        let path = temp_path("roundtrip");
        let mut writer = ColumnarWriter::create(&path, 9).expect("create");
        writer.append_bars(&bars).expect("append");
        let written = writer.finish().expect("finish");
        assert_eq!(written, bars.len() as u64);

        let file = ColumnarFile::open(&path).expect("open");
        assert_eq!(file.len(), bars.len());
        let round = file.slice(0..bars.len()).expect("slice full range");
        std::fs::remove_file(&path).expect("clean up");
        assert_eq!(round, bars);
    }

    #[test]
    fn slice_returns_exactly_the_requested_window() {
        let bars = small_series();
        let path = temp_path("slice-window");
        let mut writer = ColumnarWriter::create(&path, 9).expect("create");
        writer.append_bars(&bars).expect("append");
        writer.finish().expect("finish");

        let file = ColumnarFile::open(&path).expect("open");
        let window = file.slice(200..260).expect("slice middle window");
        std::fs::remove_file(&path).expect("clean up");
        assert_eq!(window.len(), 60);
        assert_eq!(window.first().unwrap(), &bars[200]);
        assert_eq!(window.last().unwrap(), &bars[259]);
    }

    #[test]
    fn inexact_scale_is_rejected_at_write() {
        let bars = one_bar(dec!(1.234)); // 3 dp, does not fit at scale 2
        let path = temp_path("scale-overflow");
        let mut writer = ColumnarWriter::create(&path, 2).expect("create");
        let err = writer.append_bars(&bars).unwrap_err();
        drop(writer); // no finish() ⇒ the file is abandoned, never a valid columnar file
        std::fs::remove_file(&path).ok();
        assert_eq!(
            err,
            ColumnarError::ScaleOverflow {
                row: 0,
                field: "close"
            }
        );
    }

    #[test]
    fn corrupt_magic_is_rejected_at_open() {
        let path = temp_path("bad-magic");
        std::fs::write(&path, [0u8; HEADER_LEN]).expect("write junk header");
        let err = ColumnarFile::open(&path).unwrap_err();
        std::fs::remove_file(&path).expect("clean up");
        assert_eq!(err, ColumnarError::BadMagic);
    }

    #[test]
    fn truncated_file_is_rejected_at_open() {
        let path = temp_path("truncated");
        std::fs::write(&path, [0u8; 10]).expect("write short file"); // shorter than the header
        let err = ColumnarFile::open(&path).unwrap_err();
        std::fs::remove_file(&path).expect("clean up");
        assert!(matches!(err, ColumnarError::Io(_)));
    }

    #[test]
    fn slice_past_end_is_rejected() {
        let bars = small_series();
        let path = temp_path("slice-oob");
        let mut writer = ColumnarWriter::create(&path, 9).expect("create");
        writer.append_bars(&bars).expect("append");
        writer.finish().expect("finish");

        let file = ColumnarFile::open(&path).expect("open");
        let len = file.len();
        let err = file.slice(len..len + 1).unwrap_err();
        std::fs::remove_file(&path).expect("clean up");
        assert_eq!(
            err,
            ColumnarError::SliceOutOfBounds {
                requested_end: len + 1,
                len
            }
        );
    }

    /// Scale/streaming proof (m-hf-track §2, card C2.6): a full year of 1s bars (365 × 86,400 =
    /// 31,536,000 rows) written in daily segments — the writer never holds more than one day in
    /// memory — then read back via 1,000 windowed slices. Run explicitly (`--release`, once) and
    /// record wall-clock/file size/RSS in the worklog + D-0013; this is proof, not CI.
    #[test]
    #[ignore = "scale proof: ~31.5M rows, run explicitly"]
    fn scale_proof_full_year_1s() {
        const SEGMENTS: i64 = 365;
        const STEPS: usize = 86_400;
        const BASE_START: i64 = 1_704_067_200; // 2024-01-01T00:00:00Z
        const SCALE: u32 = 9;

        let path = temp_path("scale-proof");
        let write_start = std::time::Instant::now();
        let mut writer = ColumnarWriter::create(&path, SCALE).expect("create scale-proof file");
        for i in 0..SEGMENTS {
            let spec = SyntheticSpec {
                seed: 7 + i as u64,
                steps: STEPS,
                start_unix: BASE_START + i * 86_400,
                start_slot: 100_000,
                venue: "venue_a".to_string(),
                mid0: dec!(100),
                anchor: dec!(100),
                reversion: dec!(0.05),
                vol_step: dec!(0.2),
                impact_every: 0,
                impact_size: dec!(0),
                impact_decay: dec!(0.5),
                regime_period: 100,
            };
            let out = generate(&spec).expect("generate segment");
            writer.append_bars(&out.bars_1s).expect("append segment");
        }
        let row_count = writer.finish().expect("finish");
        let write_elapsed = write_start.elapsed();
        assert_eq!(row_count, 31_536_000);

        let file_len = std::fs::metadata(&path)
            .expect("stat scale-proof file")
            .len();
        println!(
            "scale proof: wrote {row_count} rows in {write_elapsed:?}, file size {file_len} bytes"
        );

        let read_start = std::time::Instant::now();
        let file = ColumnarFile::open(&path).expect("open scale-proof file");
        assert_eq!(file.len(), 31_536_000);
        const WINDOW: usize = 43_200;
        let max_start = file.len() - WINDOW;
        let step = max_start / 999;
        for i in 0..1_000usize {
            let start = (i * step).min(max_start);
            let window = file.slice(start..start + WINDOW).expect("slice window");
            assert_eq!(window.len(), WINDOW);
            assert_eq!(
                window.first().unwrap().ts.as_unix(),
                BASE_START + start as i64
            );
            assert_eq!(
                window.last().unwrap().ts.as_unix(),
                BASE_START + (start + WINDOW - 1) as i64
            );
        }
        let read_elapsed = read_start.elapsed();
        println!("scale proof: read 1000 windows of {WINDOW} rows in {read_elapsed:?}");

        std::fs::remove_file(&path).expect("clean up scale-proof fixture");
    }
}
