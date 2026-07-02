//! Walk-forward window model (plan §7).
//!
//! A [`WalkForward`] schedule turns a single bar series of length `n_bars` into an ordered sequence
//! of [`Window`]s, each a `(train, test)` pair of **bar-index ranges** into that series. The
//! schedule never lets future data leak into training and never reuses test data across folds:
//!
//! - **Embargo.** `train.end + embargo <= test.start`, so a configurable gap of completed-but-unused
//!   bars sits between the last training bar and the first test bar — this purges look-ahead that an
//!   indicator spanning the boundary could otherwise smuggle in. The no-future-data rule *within* a
//!   run is already enforced by `portfolio::run` (next-bar execution, invariant 5); windows add the
//!   **cross-window** non-leak guarantee.
//! - **Ordered, non-overlapping tests.** Successive test ranges advance by `step` and never overlap,
//!   so each fold scores a disjoint out-of-sample slice — the precondition for the fold-dispersion
//!   reporting in S11. This is structural: [`WalkForward::windows`] only ever emits folds when
//!   `step >= test_len`.
//! - **In bounds.** Only windows whose `test.end <= n_bars` are emitted, so the schedule never
//!   indexes past the series.
//!
//! Two shapes (plan §7, §14): [`WindowKind::Rolling`] (the default) slides a fixed-length training
//! window so stale regimes drop out — the right default for crypto, where one bull-market backtest
//! is not to be trusted (plan §14); [`WindowKind::Anchored`] pins the training start at index 0 and
//! grows the window, for the low-data case.
//!
//! In Rolling mode a *later* fold's training window may revisit bars that were an *earlier* fold's
//! test window (both advance by `step`). That is standard walk-forward, not look-ahead: the no-leak
//! guarantee is within-fold (`train.end <= test.start`) plus across-fold on the *test* ranges
//! (ordered, non-overlapping) — never that a future fold ignores already-scored bars.

use std::ops::Range;

/// Whether the training window slides at fixed length or grows from a pinned start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowKind {
    /// Fixed-length training window that slides forward by `step` each fold, de-weighting stale
    /// regimes. The default (plan §7).
    #[default]
    Rolling,
    /// Training window pinned at index 0 and grown by `step` each fold, using all history to date.
    Anchored,
}

/// One walk-forward fold: a training range and a strictly-later test range, both over bar indices.
///
/// Half-open ranges (`start..end`). The schedule guarantees `train.end <= test.start` (with the
/// configured embargo gap) and that the `test` ranges of successive windows are ordered and
/// non-overlapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub train: Range<usize>,
    pub test: Range<usize>,
}

/// Refusal of a [`WalkForward`] configuration that could not yield ordered, non-overlapping,
/// leak-free folds. Mirrors the workspace's hand-rolled error style (cf. `portfolio::SimError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowError {
    /// `train_len` was zero — a fold needs training bars.
    ZeroTrainLen,
    /// `test_len` was zero — a fold needs test bars.
    ZeroTestLen,
    /// `step` was zero — the schedule would never advance.
    ZeroStep,
    /// `step < test_len`, which would make successive test ranges overlap.
    TestsOverlap { step: usize, test_len: usize },
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroTrainLen => write!(f, "walk-forward train_len must be > 0"),
            Self::ZeroTestLen => write!(f, "walk-forward test_len must be > 0"),
            Self::ZeroStep => write!(f, "walk-forward step must be > 0"),
            Self::TestsOverlap { step, test_len } => write!(
                f,
                "walk-forward step {step} < test_len {test_len} would overlap test windows"
            ),
        }
    }
}

impl std::error::Error for WindowError {}

/// A walk-forward schedule: window shape plus train/test/step/embargo lengths, all measured in
/// **bars**. Build via [`WalkForward::new`] for validation; [`WalkForward::windows`] expands it
/// against a series length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkForward {
    pub kind: WindowKind,
    pub train_len: usize,
    pub test_len: usize,
    pub step: usize,
    pub embargo: usize,
}

impl WalkForward {
    /// Build a validated schedule. This is the blessed construction path; it rejects any
    /// configuration that could not yield ordered, non-overlapping, leak-free folds, so the
    /// "bad params rejected" gate (plan §14 S8) lives here.
    ///
    /// # Errors
    /// - [`WindowError::ZeroTrainLen`], [`WindowError::ZeroTestLen`], or [`WindowError::ZeroStep`]
    ///   for a zero length;
    /// - [`WindowError::TestsOverlap`] when `step < test_len` (which would overlap the folds' test
    ///   ranges).
    pub fn new(
        kind: WindowKind,
        train_len: usize,
        test_len: usize,
        step: usize,
        embargo: usize,
    ) -> Result<Self, WindowError> {
        if train_len == 0 {
            return Err(WindowError::ZeroTrainLen);
        }
        if test_len == 0 {
            return Err(WindowError::ZeroTestLen);
        }
        if step == 0 {
            return Err(WindowError::ZeroStep);
        }
        if step < test_len {
            return Err(WindowError::TestsOverlap { step, test_len });
        }
        Ok(Self {
            kind,
            train_len,
            test_len,
            step,
            embargo,
        })
    }

    /// Expand into the ordered fold windows for a series of `n_bars` bars.
    ///
    /// Only windows lying entirely within `0..n_bars` are emitted; a series too short for even one
    /// fold yields an empty `Vec` — the honest "insufficient data" signal the partition layer (S9)
    /// acts on, never a panic or an out-of-bounds range.
    ///
    /// The method is **total and defensive**: even if this `WalkForward` was assembled by struct
    /// literal rather than [`WalkForward::new`], a degenerate configuration (a zero `train_len` /
    /// `test_len` / `step`, or `step < test_len`) yields an empty `Vec` rather than an infinite loop
    /// or overlapping folds. The ordered / non-overlapping / in-bounds invariant is therefore a
    /// structural property of the *output*, not merely of the constructor.
    #[must_use]
    pub fn windows(&self, n_bars: usize) -> Vec<Window> {
        // Defensive guard: every configuration `new` would reject produces no windows here, so the
        // invariant holds regardless of how `self` was built. `step < test_len` also subsumes
        // `step == 0`, since `test_len >= 1` once we are past the zero checks.
        if self.train_len == 0 || self.test_len == 0 || self.step < self.test_len {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut k = 0usize;
        loop {
            // Saturating arithmetic keeps an absurd configuration from panicking on overflow; the
            // break below then ends the loop. `step >= test_len >= 1` here, so each iteration
            // strictly advances `test_end` until that break fires.
            let (train_start, train_end) = match self.kind {
                WindowKind::Rolling => {
                    let start = k.saturating_mul(self.step);
                    (start, start.saturating_add(self.train_len))
                }
                WindowKind::Anchored => (
                    0,
                    self.train_len.saturating_add(k.saturating_mul(self.step)),
                ),
            };
            let test_start = train_end.saturating_add(self.embargo);
            let test_end = test_start.saturating_add(self.test_len);
            // Terminates for every reachable input: `n_bars` is a series (slice) length, always far
            // below `usize::MAX`, so a saturated bound strictly exceeds it and ends the loop. (Only
            // the unreachable `n_bars == usize::MAX` — a slice can never be that long — could keep a
            // saturated bound from exceeding `n_bars`; we do not special-case it.)
            if test_end > n_bars {
                break;
            }
            out.push(Window {
                train: train_start..train_end,
                test: test_start..test_end,
            });
            k += 1;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_degenerate_configs() {
        assert_eq!(
            WalkForward::new(WindowKind::Rolling, 0, 2, 2, 0),
            Err(WindowError::ZeroTrainLen)
        );
        assert_eq!(
            WalkForward::new(WindowKind::Rolling, 3, 0, 2, 0),
            Err(WindowError::ZeroTestLen)
        );
        assert_eq!(
            WalkForward::new(WindowKind::Rolling, 3, 2, 0, 0),
            Err(WindowError::ZeroStep)
        );
        assert_eq!(
            WalkForward::new(WindowKind::Rolling, 3, 2, 1, 0),
            Err(WindowError::TestsOverlap {
                step: 1,
                test_len: 2
            })
        );
    }

    #[test]
    fn windows_is_defensive_against_struct_literal_bypass() {
        // step == 0 must not loop forever; step < test_len must not emit overlapping folds.
        let zero_step = WalkForward {
            kind: WindowKind::Rolling,
            train_len: 3,
            test_len: 2,
            step: 0,
            embargo: 0,
        };
        assert!(zero_step.windows(100).is_empty());

        let overlapping = WalkForward {
            kind: WindowKind::Rolling,
            train_len: 3,
            test_len: 2,
            step: 1,
            embargo: 0,
        };
        assert!(overlapping.windows(100).is_empty());
    }

    #[test]
    fn default_kind_is_rolling() {
        assert_eq!(WindowKind::default(), WindowKind::Rolling);
    }

    /// Assert each fold honors the embargo gap and that test ranges are ordered & non-overlapping.
    fn assert_wellformed(folds: &[Window], embargo: usize) {
        for w in folds {
            assert!(w.train.start < w.train.end, "train range non-empty");
            assert!(w.test.start < w.test.end, "test range non-empty");
            assert_eq!(w.train.end + embargo, w.test.start, "embargo gap honored");
        }
        for pair in folds.windows(2) {
            assert!(
                pair[0].test.end <= pair[1].test.start,
                "test ranges ordered and non-overlapping"
            );
        }
    }

    #[test]
    fn rolling_generates_ordered_nonoverlapping_folds() {
        let wf = WalkForward::new(WindowKind::Rolling, 3, 2, 2, 0).unwrap();
        let folds = wf.windows(10);
        assert_eq!(
            folds,
            vec![
                Window {
                    train: 0..3,
                    test: 3..5
                },
                Window {
                    train: 2..5,
                    test: 5..7
                },
                Window {
                    train: 4..7,
                    test: 7..9
                },
            ]
        );
        assert_wellformed(&folds, 0);
    }

    #[test]
    fn embargo_inserts_a_gap_between_train_and_test() {
        let wf = WalkForward::new(WindowKind::Rolling, 3, 2, 2, 1).unwrap();
        let folds = wf.windows(10);
        assert!(!folds.is_empty());
        assert_eq!(
            folds[0],
            Window {
                train: 0..3,
                test: 4..6
            }
        );
        assert_wellformed(&folds, 1);
    }

    #[test]
    fn anchored_pins_train_start_and_grows_the_window() {
        let wf = WalkForward::new(WindowKind::Anchored, 3, 2, 2, 0).unwrap();
        let folds = wf.windows(12);
        assert!(folds.len() >= 3);
        for w in &folds {
            assert_eq!(w.train.start, 0, "anchored training always starts at 0");
        }
        for pair in folds.windows(2) {
            assert!(
                pair[1].train.end > pair[0].train.end,
                "anchored training window grows"
            );
        }
        assert_wellformed(&folds, 0);
    }

    #[test]
    fn step_larger_than_test_len_leaves_gaps_between_tests() {
        let wf = WalkForward::new(WindowKind::Rolling, 3, 2, 3, 0).unwrap();
        let folds = wf.windows(15);
        assert!(folds.len() >= 2);
        assert_wellformed(&folds, 0);
        // step (3) − test_len (2) = a 1-bar gap between successive test ranges.
        assert!(folds
            .windows(2)
            .all(|p| p[1].test.start - p[0].test.end == 1));
    }

    #[test]
    fn series_too_short_yields_no_folds() {
        let wf = WalkForward::new(WindowKind::Rolling, 3, 2, 2, 0).unwrap();
        assert!(wf.windows(4).is_empty(), "need train(3)+test(2)=5 bars");
        assert!(wf.windows(0).is_empty());
    }

    #[test]
    fn window_error_display_messages() {
        assert!(WindowError::ZeroTrainLen.to_string().contains("train_len"));
        assert!(WindowError::ZeroTestLen.to_string().contains("test_len"));
        assert!(WindowError::ZeroStep.to_string().contains("step"));
        assert!(WindowError::TestsOverlap {
            step: 1,
            test_len: 2
        }
        .to_string()
        .contains("overlap"));
    }
}
