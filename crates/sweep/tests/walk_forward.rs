//! S8 gate (plan §7, §14): the walk-forward window model must produce folds that are
//! `train.end <= test.start` (with the embargo gap), with ordered and non-overlapping test ranges,
//! never out of bounds, in the correct rolling/anchored shape, and must reject bad parameters.

use sweep::{WalkForward, Window, WindowKind};

/// Assert every structural invariant on the folds a schedule produces over `n_bars` bars.
///
/// Each fold is pinned against the gate-derived index formula — written here independently of the
/// implementation — so a mutant that, say, grew the anchored train by `test_len` instead of `step`,
/// or placed the test window `embargo + 1` bars out, would fail. The exact-equality checks make the
/// embargo magnitude (gate bullet a) and the `step`-coupled Rolling/Anchored shapes (gate bullet d)
/// observable, not merely lower-bounded.
fn assert_fold_invariants(wf: &WalkForward, n_bars: usize) {
    let folds = wf.windows(n_bars);
    for (k, w) in folds.iter().enumerate() {
        // Train range, by fold index — the shape gate (d), with growth coupled to `step`.
        let (exp_train_start, exp_train_end) = match wf.kind {
            // Rolling: fixed-length window sliding by `step`.
            WindowKind::Rolling => (k * wf.step, k * wf.step + wf.train_len),
            // Anchored: pinned at 0, end grows by `step` each fold.
            WindowKind::Anchored => (0, wf.train_len + k * wf.step),
        };
        assert_eq!(
            w.train.start, exp_train_start,
            "train.start at fold {k}: {w:?}"
        );
        assert_eq!(w.train.end, exp_train_end, "train.end at fold {k}: {w:?}");
        // Test range: starts EXACTLY `embargo` bars after train ends (gate a), spans `test_len`.
        assert_eq!(
            w.test.start,
            w.train.end + wf.embargo,
            "embargo gap must be exactly {} at fold {k}: {w:?}",
            wf.embargo
        );
        assert_eq!(
            w.test.end,
            w.test.start + wf.test_len,
            "test length must be {} at fold {k}: {w:?}",
            wf.test_len
        );
        // Both ranges non-empty (train_len, test_len are >= 1 past validation).
        assert!(w.train.start < w.train.end, "empty train range in {w:?}");
        assert!(w.test.start < w.test.end, "empty test range in {w:?}");
        // In bounds (gate c): nothing indexes past the series.
        assert!(w.test.end <= n_bars, "test range out of bounds in {w:?}");
        assert!(w.train.end <= n_bars, "train range out of bounds in {w:?}");
    }
    // Test ranges are strictly ordered and non-overlapping across consecutive folds (gate b).
    for pair in folds.windows(2) {
        assert!(
            pair[0].test.start < pair[1].test.start,
            "test ranges must strictly advance: {:?} then {:?}",
            pair[0],
            pair[1]
        );
        assert!(
            pair[0].test.end <= pair[1].test.start,
            "test ranges must not overlap: {:?} then {:?}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn rolling_enumeration_is_exact() {
    // train=3, test=2, step=2, embargo=1 over 12 bars.
    let wf = WalkForward::new(WindowKind::Rolling, 3, 2, 2, 1).unwrap();
    let folds = wf.windows(12);
    let expected = vec![
        Window {
            train: 0..3,
            test: 4..6,
        },
        Window {
            train: 2..5,
            test: 6..8,
        },
        Window {
            train: 4..7,
            test: 8..10,
        },
        Window {
            train: 6..9,
            test: 10..12,
        },
    ];
    assert_eq!(folds, expected);
}

#[test]
fn anchored_enumeration_is_exact() {
    // Same lengths, anchored: train start pinned at 0, train end grows by step each fold.
    let wf = WalkForward::new(WindowKind::Anchored, 3, 2, 2, 1).unwrap();
    let folds = wf.windows(12);
    let expected = vec![
        Window {
            train: 0..3,
            test: 4..6,
        },
        Window {
            train: 0..5,
            test: 6..8,
        },
        Window {
            train: 0..7,
            test: 8..10,
        },
        Window {
            train: 0..9,
            test: 10..12,
        },
    ];
    assert_eq!(folds, expected);
}

#[test]
fn anchored_enumeration_is_exact_when_step_exceeds_test_len() {
    // Anchored with step (4) > test_len (2): the train end and each test window must advance by the
    // *step*, not by test_len. Pins gate bullet (d) against a "grow by test_len" mutant.
    let wf = WalkForward::new(WindowKind::Anchored, 3, 2, 4, 0).unwrap();
    let folds = wf.windows(24);
    let expected = vec![
        Window {
            train: 0..3,
            test: 3..5,
        },
        Window {
            train: 0..7,
            test: 7..9,
        },
        Window {
            train: 0..11,
            test: 11..13,
        },
        Window {
            train: 0..15,
            test: 15..17,
        },
        Window {
            train: 0..19,
            test: 19..21,
        },
    ];
    assert_eq!(folds, expected);
    assert_fold_invariants(&wf, 24);
}

#[test]
fn rolling_and_anchored_share_test_ranges_when_step_equals_test_len() {
    // The two shapes differ only in their train windows; with step == test_len the test schedule is
    // identical, which makes them directly comparable in the strategy-family report (S10/S11).
    let r = WalkForward::new(WindowKind::Rolling, 4, 3, 3, 2).unwrap();
    let a = WalkForward::new(WindowKind::Anchored, 4, 3, 3, 2).unwrap();
    let rt: Vec<_> = r.windows(40).into_iter().map(|w| w.test).collect();
    let at: Vec<_> = a.windows(40).into_iter().map(|w| w.test).collect();
    assert_eq!(rt, at);
    assert!(!rt.is_empty());
}

#[test]
fn non_adjacent_tests_have_gaps_when_step_exceeds_test_len() {
    // step > test_len leaves gaps between test ranges; still ordered and non-overlapping.
    let wf = WalkForward::new(WindowKind::Rolling, 3, 2, 3, 0).unwrap();
    let folds = wf.windows(12);
    let expected = vec![
        Window {
            train: 0..3,
            test: 3..5,
        },
        Window {
            train: 3..6,
            test: 6..8,
        },
        Window {
            train: 6..9,
            test: 9..11,
        },
    ];
    assert_eq!(folds, expected);
    assert_fold_invariants(&wf, 12);
}

#[test]
fn embargo_of_two_places_each_test_window_exactly_two_bars_after_train() {
    // embargo >= 2 over multiple folds: pins gate bullet (a) magnitude against an off-by-one (or
    // "+1 when embargo>=2") mutant that the earlier embargo-in-{0,1} cases cannot see.
    let wf = WalkForward::new(WindowKind::Rolling, 4, 3, 3, 2).unwrap();
    let folds = wf.windows(20);
    let expected = vec![
        Window {
            train: 0..4,
            test: 6..9,
        },
        Window {
            train: 3..7,
            test: 9..12,
        },
        Window {
            train: 6..10,
            test: 12..15,
        },
        Window {
            train: 9..13,
            test: 15..18,
        },
    ];
    assert_eq!(folds, expected);
    // Every fold's test starts exactly `embargo` (2) bars after its train ends.
    for w in &folds {
        assert_eq!(w.test.start - w.train.end, 2);
    }
    assert_fold_invariants(&wf, 20);
}

#[test]
fn too_short_a_series_yields_no_folds() {
    let wf = WalkForward::new(WindowKind::Rolling, 3, 2, 2, 1).unwrap();
    // Needs train(3) + embargo(1) + test(2) = 6 bars for the first fold.
    assert!(wf.windows(5).is_empty());
    assert_eq!(wf.windows(6).len(), 1);
}

#[test]
fn invariants_hold_across_a_grid_of_configs() {
    for &kind in &[WindowKind::Rolling, WindowKind::Anchored] {
        for train_len in [1usize, 3, 10] {
            for test_len in [1usize, 2, 5] {
                for step in [test_len, test_len + 1, test_len + 4] {
                    for embargo in [0usize, 1, 3] {
                        let wf =
                            WalkForward::new(kind, train_len, test_len, step, embargo).unwrap();
                        for n_bars in [0usize, 1, 7, 23, 60, 200] {
                            assert_fold_invariants(&wf, n_bars);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn windows_are_deterministic_and_pure() {
    let wf = WalkForward::new(WindowKind::Rolling, 5, 3, 4, 2).unwrap();
    let folds = wf.windows(50);
    // Anchor against real values (not just self-consistency): 11 folds, pinned endpoints.
    assert_eq!(folds.len(), 11);
    assert_eq!(
        folds[0],
        Window {
            train: 0..5,
            test: 7..10
        }
    );
    assert_eq!(
        folds[10],
        Window {
            train: 40..45,
            test: 47..50
        }
    );
    // Same schedule, same input → identical output, every call (invariant 3).
    assert_eq!(wf.windows(50), folds);
    // Two independently-built identical schedules compare equal (value type, for downstream reports).
    let same = WalkForward::new(WindowKind::Rolling, 5, 3, 4, 2).unwrap();
    assert_eq!(wf, same);
}
