//! Holdout sealing — the physical partition that makes invariant 11 structural.
//!
//! Parameter selection (the sweep grid, walk-forward folds, and the S11 advancement report) must
//! never see the holdout. This module enforces that not by convention but by ownership and types:
//!
//! 1. [`PartitionedBars::from_spec`] validates the full series and splits it into three contiguous,
//!    ordered partitions, **physically moving** the holdout bars into their own [`Vec`] — a
//!    *different allocation* from the development+validation bars, not a `&[Bar]` borrow into a
//!    shared backing array.
//! 2. [`PartitionedBars::seal_holdout`] consumes the partition and returns a [`DevValidation`] (all
//!    the selection pipeline ever holds) and a [`Sealed`] (the holdout, locked away). `DevValidation`
//!    exposes development / validation / walk-forward windows and has **no** holdout accessor; and
//!    because the holdout is a separate allocation, walk-forward windows expanded over the dev/val
//!    length are *structurally incapable* of indexing into it.
//! 3. [`evaluate_on_holdout`] is the single holdout reader. It consumes the [`Sealed`] **by value**,
//!    so the holdout is scored at most once — and only in M5. The M4 CLI never calls it.
//!
//! A `Cell<u32>` read counter wraps the holdout and a content digest witnesses its bytes, so a test
//! can prove the full sweep+selection pipeline neither reads nor mutates the holdout (the "untouched
//! holdout remains untouched" half of the M4 gate, plan §1/§19).

use std::cell::Cell;

use crate::cell::{eval_cell, CellResult};
use crate::config::PartitionSpec;
use crate::param::ParamPoint;
use crate::window::{WalkForward, Window};
use market_data::{validate_series, DataError};
use portfolio::{CostModel, SimError};
use research_core::{Bar, Decimal};

/// Why a [`PartitionSpec`] could not be applied to a series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionError {
    /// The series failed the platform's data-hygiene contract — empty, bad OHLC, out-of-order, or
    /// **duplicate** timestamps (see [`market_data::DataError`]). Partitioning requires the same
    /// sorted-**and-unique** series the rest of the platform enforces, so the seal admits no series
    /// that ingestion would reject; strict ordering is also what makes a `ByDate` cut well-defined.
    InvalidSeries(DataError),
    /// A `ByIndex` cut point lay past the end of the series.
    IndexOutOfBounds { index: usize, n_bars: usize },
    /// The development partition `[0, val_start)` would be empty (`val_start == 0`).
    DevelopmentEmpty,
    /// The validation partition `[val_start, holdout_start)` would be empty or reversed
    /// (`holdout_start <= val_start`). This is the overlap / mis-order rejection (plan §6).
    ValidationEmptyOrOverlapping {
        val_start: usize,
        holdout_start: usize,
    },
    /// The holdout partition `[holdout_start, n)` would be empty (`holdout_start >= n_bars`).
    HoldoutEmpty { holdout_start: usize, n_bars: usize },
}

impl std::fmt::Display for PartitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSeries(e) => write!(f, "series failed validation: {e}"),
            Self::IndexOutOfBounds { index, n_bars } => {
                write!(f, "partition cut point {index} is past the series end ({n_bars} bars)")
            }
            Self::DevelopmentEmpty => write!(f, "development partition would be empty (val_start == 0)"),
            Self::ValidationEmptyOrOverlapping {
                val_start,
                holdout_start,
            } => write!(
                f,
                "validation partition empty or overlapping: holdout_start {holdout_start} <= val_start {val_start}"
            ),
            Self::HoldoutEmpty {
                holdout_start,
                n_bars,
            } => write!(
                f,
                "holdout partition would be empty: holdout_start {holdout_start} >= {n_bars} bars"
            ),
        }
    }
}

impl std::error::Error for PartitionError {}

/// The holdout bars, sealed in their own allocation behind a read counter.
///
/// **Private on purpose.** The only way to obtain holdout bars is [`PartitionedBars::seal_holdout`]
/// (which produces a [`Sealed`]) and the only way to read them is [`Holdout::read`], reached solely
/// through [`evaluate_on_holdout`]. Every read bumps `reads`, so a test can assert the counter is
/// still `0` after the selection pipeline has run.
struct Holdout {
    bars: Vec<Bar>,
    reads: Cell<u32>,
}

impl Holdout {
    fn new(bars: Vec<Bar>) -> Self {
        Self {
            bars,
            reads: Cell::new(0),
        }
    }

    /// The sole gateway to the holdout bars; counts every access. Saturating so a (impossible) wrap
    /// cannot silently reset the counter to 0 and mask a read.
    fn read(&self) -> &[Bar] {
        self.reads.set(self.reads.get().saturating_add(1));
        &self.bars
    }
}

/// A content digest over bars, used as a **within-process** tamper witness — compare `before` vs
/// `after` inside a single run. It is **not** a durable cross-run identifier: it hashes via the std
/// `DefaultHasher` (SipHash), whose output is not guaranteed stable across Rust/std versions, so it
/// must not be persisted or compared across runs or toolchains. Hashes the canonical JSON form
/// because [`Bar`] holds `Decimal`, which is not `Hash`. Computed without going through
/// [`Holdout::read`], so taking a digest does not count as a strategy read.
fn digest_bars(bars: &[Bar]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let json = serde_json::to_string(bars).expect("bars serialize to JSON");
    json.hash(&mut h);
    h.finish()
}

/// A validated, physically-partitioned series, *pre-seal*. The holdout already lives in its own
/// allocation; [`seal_holdout`](PartitionedBars::seal_holdout) is the only way to consume it, and
/// there is intentionally no accessor that hands out the holdout bars.
pub struct PartitionedBars {
    /// Development ++ validation, contiguous, in one allocation.
    dev_val: Vec<Bar>,
    /// Boundary inside `dev_val`: development = `[0, val_start)`, validation = `[val_start, len)`.
    val_start: usize,
    /// The holdout — a **separate** allocation.
    holdout: Vec<Bar>,
}

impl std::fmt::Debug for PartitionedBars {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Redact bar contents; expose only partition shape.
        f.debug_struct("PartitionedBars")
            .field("development_len", &self.val_start)
            .field("validation_len", &(self.dev_val.len() - self.val_start))
            .field("holdout_len", &self.holdout.len())
            .finish_non_exhaustive()
    }
}

impl PartitionedBars {
    /// Validate `bars` and split it per `spec` into development / validation / holdout, **moving**
    /// the holdout into its own allocation.
    ///
    /// Validation covers the full series (delegated to the platform's canonical
    /// [`market_data::validate_series`]: non-empty, per-bar OHLC invariants, and strictly increasing
    /// — i.e. sorted **and unique** — timestamps) and the resolved cut points: contiguous, in
    /// bounds, correctly ordered, and leaving none of the three partitions empty. Contiguous cut
    /// points make the partitions disjoint and ordered
    /// (`dev.end <= val.start && val.end <= holdout.start`) by construction.
    ///
    /// # Errors
    /// [`PartitionError`] if the series fails data-hygiene validation; or if the resolved cut points
    /// are out of bounds, out of order (the overlap rejection), or would leave a partition empty.
    pub fn from_spec(bars: Vec<Bar>, spec: &PartitionSpec) -> Result<Self, PartitionError> {
        // Validate the full series against the platform's canonical contract before trusting it:
        // non-empty, OHLC-valid, and strictly increasing timestamps (sorted AND unique — the same
        // rule `market-data` enforces at ingestion, so the seal admits no series the rest of the
        // platform would reject). Strict monotonicity is also what makes a date cut via
        // `partition_point` well-defined and order-independent.
        validate_series(&bars).map_err(PartitionError::InvalidSeries)?;
        let n = bars.len(); // >= 1: `validate_series` rejects an empty series.

        let (val_start, holdout_start) = match spec {
            PartitionSpec::ByIndex {
                val_start,
                holdout_start,
            } => (*val_start, *holdout_start),
            // `partition_point` is valid because the series is sorted (checked above): bars with
            // `ts < boundary` form the prefix. Half-open boundaries: a bar exactly on the boundary
            // joins the later partition.
            PartitionSpec::ByDate {
                val_start,
                holdout_start,
            } => (
                bars.partition_point(|b| b.ts < *val_start),
                bars.partition_point(|b| b.ts < *holdout_start),
            ),
        };

        // Bounds: a `ByIndex` cut point can name an index past the end; a resolved date cannot.
        if val_start > n {
            return Err(PartitionError::IndexOutOfBounds {
                index: val_start,
                n_bars: n,
            });
        }
        if holdout_start > n {
            return Err(PartitionError::IndexOutOfBounds {
                index: holdout_start,
                n_bars: n,
            });
        }
        // Every one of the three partitions must be non-empty and correctly ordered.
        if val_start == 0 {
            return Err(PartitionError::DevelopmentEmpty);
        }
        if holdout_start <= val_start {
            return Err(PartitionError::ValidationEmptyOrOverlapping {
                val_start,
                holdout_start,
            });
        }
        if holdout_start >= n {
            return Err(PartitionError::HoldoutEmpty {
                holdout_start,
                n_bars: n,
            });
        }
        let mut dev_val = bars;
        // `split_off` moves `[holdout_start, n)` into a fresh allocation and truncates `dev_val` to
        // `[0, holdout_start)` — the physical separation the seal relies on.
        let holdout = dev_val.split_off(holdout_start);
        // Disjoint-and-ordered post-condition (plan §6), checked against the ACTUAL post-split shapes
        // rather than restating the `if` ladder above: the two partitions reconstruct the validated
        // cut, the development boundary sits strictly inside dev/val, and the holdout is non-empty.
        // A future change to the split offset or ordering would trip here.
        debug_assert_eq!(
            dev_val.len(),
            holdout_start,
            "dev/val length must equal the cut point"
        );
        debug_assert_eq!(
            holdout.len(),
            n - holdout_start,
            "holdout length must be the tail"
        );
        debug_assert!(
            val_start > 0 && val_start < dev_val.len() && !holdout.is_empty(),
            "all three partitions must be non-empty and ordered"
        );
        Ok(Self {
            dev_val,
            val_start,
            holdout,
        })
    }

    /// Development partition length (bars).
    #[must_use]
    pub fn development_len(&self) -> usize {
        self.val_start
    }

    /// Validation partition length (bars).
    #[must_use]
    pub fn validation_len(&self) -> usize {
        self.dev_val.len() - self.val_start
    }

    /// Holdout partition length (bars).
    #[must_use]
    pub fn holdout_len(&self) -> usize {
        self.holdout.len()
    }

    /// Consume the partition, sealing the holdout away from the selection pipeline.
    ///
    /// Returns the [`DevValidation`] handle the sweep/selection code works with and the [`Sealed`]
    /// holdout, which only [`evaluate_on_holdout`] can read — once, by value, in M5.
    #[must_use]
    pub fn seal_holdout(self) -> (DevValidation, Sealed) {
        (
            DevValidation {
                bars: self.dev_val,
                val_start: self.val_start,
            },
            Sealed {
                holdout: Holdout::new(self.holdout),
            },
        )
    }
}

/// The development+validation bars — everything parameter selection is allowed to see.
///
/// Exposes the development slice, the validation slice, the combined dev/val slice that walk-forward
/// windows index into, and a window expander. It deliberately has **no** holdout accessor: the
/// holdout is a different allocation held only by [`Sealed`], so windows over [`len`](Self::len)
/// cannot reach it. Selection code that holds only a `&DevValidation` has no path to the holdout
/// bars:
///
/// ```compile_fail
/// let dv: sweep::DevValidation = unimplemented!();
/// let _leak = dv.holdout(); // no such method → does not compile
/// ```
///
/// The accessors it *does* expose compile and type-check:
///
/// ```no_run
/// let dv: sweep::DevValidation = unimplemented!();
/// let _dev = dv.development();
/// let _val = dv.validation();
/// let _all = dv.dev_validation();
/// ```
#[derive(Debug, Clone)]
pub struct DevValidation {
    bars: Vec<Bar>,
    val_start: usize,
}

impl DevValidation {
    /// The development partition `[0, val_start)` — where parameters are fit.
    #[must_use]
    pub fn development(&self) -> &[Bar] {
        &self.bars[..self.val_start]
    }

    /// The validation partition `[val_start, len)` — the out-of-sample selection slice.
    #[must_use]
    pub fn validation(&self) -> &[Bar] {
        &self.bars[self.val_start..]
    }

    /// The combined development+validation series that walk-forward windows index into. This is the
    /// **only** bar array selection touches; the holdout is not part of it.
    #[must_use]
    pub fn dev_validation(&self) -> &[Bar] {
        &self.bars
    }

    /// Number of dev+val bars — the `n_bars` a walk-forward schedule is expanded against.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Whether the dev+val series is empty. (It never is when built via
    /// [`PartitionedBars::from_spec`], which rejects an empty development partition; this mirrors
    /// [`len`](Self::len) for the `clippy::len_without_is_empty` contract.)
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    /// Expand a walk-forward schedule over the dev+val length. Every emitted fold lies within
    /// `0..len()`, so no fold can index into the sealed holdout (a separate allocation).
    #[must_use]
    pub fn walk_forward_windows(&self, wf: &WalkForward) -> Vec<Window> {
        wf.windows(self.bars.len())
    }
}

/// The sealed holdout. The only constructor is [`PartitionedBars::seal_holdout`] and the only reader
/// is [`evaluate_on_holdout`], which consumes it **by value** — so the holdout is read at most once,
/// and never by code that holds only a [`DevValidation`].
///
/// A seal cannot be forged from outside the crate: the field is private and its type is private, so
/// only `seal_holdout` can mint one.
///
/// ```compile_fail
/// let _forged = sweep::Sealed { holdout: unimplemented!() }; // private field → does not compile
/// ```
pub struct Sealed {
    holdout: Holdout,
}

impl std::fmt::Debug for Sealed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Redact bar contents; expose only audit-relevant shape.
        f.debug_struct("Sealed")
            .field("holdout_len", &self.holdout.bars.len())
            .field("reads", &self.holdout.reads.get())
            .finish_non_exhaustive()
    }
}

impl Sealed {
    /// Audit hook: how many times the holdout bars have been read — `0` until [`evaluate_on_holdout`]
    /// runs. Lets a test prove the selection pipeline never touched the holdout. Does **not** read
    /// the bars.
    #[must_use]
    pub fn holdout_read_count(&self) -> u32 {
        self.holdout.reads.get()
    }

    /// Audit hook: a **within-process** content digest of the holdout for tamper-detection (compare
    /// `before` vs `after` within one run). Not a durable cross-run identifier — see `digest_bars`.
    /// Computed without going through the read gateway, so taking a digest does not count as a
    /// strategy read.
    #[must_use]
    pub fn holdout_digest(&self) -> u64 {
        digest_bars(&self.holdout.bars)
    }

    /// Holdout length (bars). Diagnostic only; does not expose the bars.
    #[must_use]
    pub fn holdout_len(&self) -> usize {
        self.holdout.bars.len()
    }
}

/// Score the chosen parameters on the sealed holdout — the single, call-once holdout read (M5 only).
///
/// Consumes `sealed` **by value**: there is no way to evaluate the same holdout twice, and selection
/// code (which only ever holds a [`DevValidation`]) can never reach this entry point. **The M4 CLI
/// never calls it** (plan §6, §13).
///
/// # Errors
/// Propagates `portfolio::SimError` from the underlying run. (The holdout is guaranteed non-empty by
/// [`PartitionedBars::from_spec`], so `NoBars` cannot occur here.)
///
/// A single call type-checks (this also pins the argument types, so the `compile_fail` below can
/// only be failing for the move, not for an unrelated naming error):
///
/// ```no_run
/// fn one_call_is_fine(
///     sealed: sweep::Sealed,
///     point: &sweep::ParamPoint,
///     cost: &portfolio::CostModel,
///     cash: research_core::Decimal,
/// ) {
///     let _ = sweep::evaluate_on_holdout(sealed, point, cost, cash, 365.0);
/// }
/// ```
///
/// …but because the seal is taken by value, a second call does not compile (call-once). The seal
/// here is a real (reachable) binding — a function parameter rather than a diverging
/// `unimplemented!()` — so the move on the second call is a genuine borrow-check error, not
/// dead-code that the checker would skip:
///
/// ```compile_fail
/// fn second_call_is_rejected(
///     sealed: sweep::Sealed,
///     point: &sweep::ParamPoint,
///     cost: &portfolio::CostModel,
///     cash: research_core::Decimal,
/// ) {
///     let _ = sweep::evaluate_on_holdout(sealed, point, cost, cash, 365.0);
///     let _ = sweep::evaluate_on_holdout(sealed, point, cost, cash, 365.0); // `sealed` moved → no compile
/// }
/// ```
pub fn evaluate_on_holdout(
    sealed: Sealed,
    chosen: &ParamPoint,
    cost: &CostModel,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
) -> Result<CellResult, SimError> {
    // The sole legitimate read of the holdout — bumps the read counter to 1.
    let bars = sealed.holdout.read();
    eval_cell(chosen, bars, cost, initial_cash_usdc, periods_per_year)
}

#[cfg(test)]
mod tests {
    use super::*;
    use research_core::Timestamp;
    use rust_decimal_macros::dec;

    /// `n` sorted, OHLC-valid daily bars (flat each bar; price drifts up by index so strategies trade).
    fn series(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| {
                let p = Decimal::from(100 + i as i64);
                Bar {
                    ts: Timestamp::from_unix(i as i64 * 86_400),
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
    fn from_spec_by_index_splits_into_three_contiguous_partitions() {
        let pb = PartitionedBars::from_spec(series(40), &PartitionSpec::by_index(20, 30)).unwrap();
        assert_eq!(pb.development_len(), 20);
        assert_eq!(pb.validation_len(), 10);
        assert_eq!(pb.holdout_len(), 10);

        let (dev, sealed) = pb.seal_holdout();
        assert_eq!(dev.development().len(), 20);
        assert_eq!(dev.validation().len(), 10);
        assert_eq!(dev.dev_validation().len(), 30);
        assert_eq!(dev.len(), 30);
        assert!(!dev.is_empty());
        assert_eq!(sealed.holdout_len(), 10);
    }

    #[test]
    fn by_date_resolves_cut_points_with_half_open_boundaries() {
        // 10 daily bars from the epoch. Validation starts at day 4, holdout at day 7.
        let spec = PartitionSpec::ByDate {
            val_start: Timestamp::from_unix(4 * 86_400),
            holdout_start: Timestamp::from_unix(7 * 86_400),
        };
        let pb = PartitionedBars::from_spec(series(10), &spec).unwrap();
        assert_eq!(pb.development_len(), 4); // days 0..4
        assert_eq!(pb.validation_len(), 3); // days 4..7
        assert_eq!(pb.holdout_len(), 3); // days 7..10
    }

    #[test]
    fn holdout_is_a_separate_allocation_holding_the_tail_bars() {
        let pb = PartitionedBars::from_spec(series(10), &PartitionSpec::by_index(4, 7)).unwrap();
        let (dev, sealed) = pb.seal_holdout();
        // Different allocations (the physical move): the dev/val and holdout buffers do not alias.
        assert_ne!(dev.dev_validation().as_ptr(), sealed.holdout.bars.as_ptr());
        // Content boundary: dev/val ends at day 6, holdout begins at day 7.
        assert_eq!(
            dev.dev_validation().last().unwrap().ts,
            Timestamp::from_unix(6 * 86_400)
        );
        assert_eq!(sealed.holdout.bars[0].ts, Timestamp::from_unix(7 * 86_400));
    }

    #[test]
    fn read_gateway_increments_the_counter_but_audit_hooks_do_not() {
        let pb = PartitionedBars::from_spec(series(10), &PartitionSpec::by_index(4, 7)).unwrap();
        let (_dev, sealed) = pb.seal_holdout();
        assert_eq!(sealed.holdout_read_count(), 0);

        let _ = sealed.holdout.read();
        assert_eq!(sealed.holdout_read_count(), 1);
        let _ = sealed.holdout.read();
        assert_eq!(sealed.holdout_read_count(), 2);

        // Digest and length are audits, not reads: they must not bump the counter.
        let before = sealed.holdout_read_count();
        let _ = sealed.holdout_digest();
        let _ = sealed.holdout_len();
        assert_eq!(sealed.holdout_read_count(), before);
    }

    #[test]
    fn digest_is_stable_and_distinguishes_content() {
        let pb = PartitionedBars::from_spec(series(10), &PartitionSpec::by_index(4, 7)).unwrap();
        let (_dev, sealed) = pb.seal_holdout();
        assert_eq!(sealed.holdout_digest(), sealed.holdout_digest());

        // A different holdout (different tail bars) yields a different digest.
        let other = PartitionedBars::from_spec(series(11), &PartitionSpec::by_index(4, 7)).unwrap();
        let (_d2, other_sealed) = other.seal_holdout();
        assert_ne!(sealed.holdout_digest(), other_sealed.holdout_digest());
    }

    #[test]
    fn empty_series_rejected() {
        assert_eq!(
            PartitionedBars::from_spec(vec![], &PartitionSpec::by_index(1, 2)).unwrap_err(),
            PartitionError::InvalidSeries(DataError::EmptySeries)
        );
    }

    #[test]
    fn unsorted_series_rejected() {
        let mut bars = series(6);
        bars.swap(1, 4); // break the timestamp ordering
        assert!(matches!(
            PartitionedBars::from_spec(bars, &PartitionSpec::by_index(2, 4)).unwrap_err(),
            PartitionError::InvalidSeries(DataError::Unsorted { .. })
        ));
    }

    #[test]
    fn duplicate_timestamp_series_rejected() {
        // The platform contract is sorted AND unique: a duplicate timestamp is a hard stop, so the
        // seal cannot admit a degenerate series the rest of the platform rejects (and a `ByDate` cut
        // over a run of equal timestamps stays well-defined). Two bars share day 2.
        let mut bars = series(6);
        bars[2].ts = bars[1].ts; // now two bars at the same timestamp
        assert!(matches!(
            PartitionedBars::from_spec(bars, &PartitionSpec::by_index(2, 4)).unwrap_err(),
            PartitionError::InvalidSeries(DataError::DuplicateTimestamp { .. })
        ));
    }

    #[test]
    fn overlapping_and_empty_partitions_rejected() {
        // holdout_start before val_start → overlap / mis-order (the headline rejection).
        assert_eq!(
            PartitionedBars::from_spec(series(10), &PartitionSpec::by_index(7, 3)).unwrap_err(),
            PartitionError::ValidationEmptyOrOverlapping {
                val_start: 7,
                holdout_start: 3
            }
        );
        // val_start == holdout_start → empty validation.
        assert!(matches!(
            PartitionedBars::from_spec(series(10), &PartitionSpec::by_index(5, 5)).unwrap_err(),
            PartitionError::ValidationEmptyOrOverlapping { .. }
        ));
        // val_start == 0 → empty development.
        assert_eq!(
            PartitionedBars::from_spec(series(10), &PartitionSpec::by_index(0, 5)).unwrap_err(),
            PartitionError::DevelopmentEmpty
        );
        // holdout_start == n → empty holdout.
        assert_eq!(
            PartitionedBars::from_spec(series(10), &PartitionSpec::by_index(4, 10)).unwrap_err(),
            PartitionError::HoldoutEmpty {
                holdout_start: 10,
                n_bars: 10
            }
        );
        // A cut point past the end.
        assert_eq!(
            PartitionedBars::from_spec(series(10), &PartitionSpec::by_index(4, 99)).unwrap_err(),
            PartitionError::IndexOutOfBounds {
                index: 99,
                n_bars: 10
            }
        );
    }

    #[test]
    fn evaluate_on_holdout_scores_exactly_the_sealed_holdout_bars() {
        let bars = series(12);
        let expected_holdout: Vec<Bar> = bars[8..].to_vec();
        let pb = PartitionedBars::from_spec(bars, &PartitionSpec::by_index(4, 8)).unwrap();
        let (_dev, sealed) = pb.seal_holdout();

        let point = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };
        let got =
            evaluate_on_holdout(sealed, &point, &CostModel::zero(), dec!(1000), 365.0).unwrap();
        let expected = eval_cell(
            &point,
            &expected_holdout,
            &CostModel::zero(),
            dec!(1000),
            365.0,
        )
        .unwrap();
        assert_eq!(got, expected);
    }
}
