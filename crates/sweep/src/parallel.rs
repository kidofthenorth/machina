//! Deterministic parallel execution — the M4 determinism gate (invariant 3).
//!
//! [`run_in_parallel`] is a parallel `map` whose output is **byte-identical to the sequential map**,
//! independent of thread count or OS scheduling. It achieves this without a work-stealing scheduler
//! (so, zero external dependencies — see plan §3 / D-0009): the items are split into **contiguous
//! index chunks**, each chunk is handed to one scoped thread that writes only its **disjoint** region
//! of a pre-sized output buffer, and the buffer is then read back in index order. No shared mutable
//! accumulator, no atomics, no per-completion ordering — so `output[i] == f(&items[i])` always.
//!
//! Determinism therefore rests on one requirement: the mapped function must be **pure** (a function
//! of its argument only). [`SweepCell::evaluate`] satisfies this because `portfolio::run` is
//! deterministic and uses no clock or RNG.

use crate::cell::{eval_cell, CellResult};
use crate::param::ParamPoint;
use portfolio::{CostModel, SimError};
use research_core::{Bar, Decimal};
use std::num::NonZeroUsize;
use std::ops::Range;

/// How to execute a batch of cells. Both paths produce identical canonical output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parallelism {
    /// Single-threaded reference path.
    Sequential,
    /// Up to N worker threads via `std::thread::scope`. Result is independent of N.
    Threads(NonZeroUsize),
}

/// One unit of sweep work: evaluate `point` under `cost` over the `test` bar-index range.
///
/// `index` is the cell's canonical position, assigned at enumeration; the executor preserves it.
/// Before walk-forward windows (S8) a cell spans the whole series; later, `test` is a window's test
/// range. The cell borrows no bars — they are shared by reference at evaluation time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweepCell {
    pub index: usize,
    pub point: ParamPoint,
    pub cost: CostModel,
    pub test: Range<usize>,
}

impl SweepCell {
    /// Evaluate this cell over its slice of `bars`. Pure and deterministic.
    ///
    /// # Errors
    /// Propagates `portfolio::SimError` (e.g. `NoBars` if `test` selects an empty range).
    pub fn evaluate(
        &self,
        bars: &[Bar],
        initial_cash_usdc: Decimal,
        periods_per_year: f64,
    ) -> Result<CellResult, SimError> {
        eval_cell(
            &self.point,
            &bars[self.test.clone()],
            &self.cost,
            initial_cash_usdc,
            periods_per_year,
        )
    }
}

/// Deterministic parallel `map`: `output[i] == f(&items[i])` for every `i`, regardless of
/// `parallelism` or thread scheduling.
///
/// The mapped closure MUST be pure for the output to be meaningful; purity is what makes the
/// parallel and sequential results identical.
#[must_use]
pub fn run_in_parallel<T, R, F>(items: &[T], parallelism: Parallelism, f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    let n = items.len();
    let threads = match parallelism {
        Parallelism::Sequential => 1,
        Parallelism::Threads(t) => t.get(),
    };
    if threads <= 1 || n <= 1 {
        return items.iter().map(&f).collect();
    }

    // Pre-size the output; each thread writes a disjoint contiguous chunk of it.
    let mut out: Vec<Option<R>> = Vec::with_capacity(n);
    out.resize_with(n, || None);
    let chunk = n.div_ceil(threads); // contiguous index chunks; count = ceil(n/chunk) <= threads
    let f = &f;

    std::thread::scope(|scope| {
        let mut remaining: &mut [Option<R>] = &mut out;
        let mut start = 0usize;
        let mut handles = Vec::new();
        while start < n {
            let end = (start + chunk).min(n);
            let (head, tail) = remaining.split_at_mut(end - start);
            remaining = tail;
            let items_chunk = &items[start..end];
            handles.push(scope.spawn(move || {
                for (slot, item) in head.iter_mut().zip(items_chunk) {
                    *slot = Some(f(item));
                }
            }));
            start = end;
        }
        for h in handles {
            h.join().expect("sweep worker thread panicked");
        }
    });

    out.into_iter()
        .map(|slot| slot.expect("every cell index was written exactly once"))
        .collect()
}

/// Evaluate a batch of cells over shared `bars`, in canonical (cell `index`) order.
///
/// # Errors
/// Returns the first cell evaluation error, by lowest cell index, so the error is deterministic.
pub fn run_cells(
    cells: &[SweepCell],
    bars: &[Bar],
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
    parallelism: Parallelism,
) -> Result<Vec<CellResult>, SimError> {
    run_in_parallel(cells, parallelism, |c| {
        c.evaluate(bars, initial_cash_usdc, periods_per_year)
    })
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use research_core::Timestamp;
    use rust_decimal_macros::dec;

    #[test]
    fn parallel_map_preserves_index_order() {
        let items: Vec<usize> = (0..100).collect();
        let seq = run_in_parallel(&items, Parallelism::Sequential, |x| x * 3 + 1);
        for k in [1usize, 2, 3, 7, 8, 16] {
            let par = run_in_parallel(
                &items,
                Parallelism::Threads(NonZeroUsize::new(k).unwrap()),
                |x| x * 3 + 1,
            );
            assert_eq!(par, seq, "thread count {k} changed the result");
        }
        assert_eq!(seq, (0..100).map(|x| x * 3 + 1).collect::<Vec<_>>());
    }

    #[test]
    fn empty_and_singleton_inputs_are_handled() {
        let empty: Vec<usize> = vec![];
        assert_eq!(
            run_in_parallel(
                &empty,
                Parallelism::Threads(NonZeroUsize::new(4).unwrap()),
                |x| *x
            ),
            Vec::<usize>::new()
        );
        let one = [42usize];
        assert_eq!(
            run_in_parallel(
                &one,
                Parallelism::Threads(NonZeroUsize::new(4).unwrap()),
                |x| *x
            ),
            vec![42]
        );
    }

    #[test]
    fn more_threads_than_items_still_index_ordered() {
        // chunk = ceil(3/8) = 1 → three single-item chunks; output must stay in index order.
        let items: Vec<usize> = (0..3).collect();
        let par = run_in_parallel(
            &items,
            Parallelism::Threads(NonZeroUsize::new(8).unwrap()),
            |x| x * 2,
        );
        assert_eq!(par, vec![0, 2, 4]);
    }

    fn cell_series(n: usize) -> Vec<Bar> {
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

    fn reb_cell(index: usize, test: Range<usize>) -> SweepCell {
        SweepCell {
            index,
            point: ParamPoint::ThresholdRebalance {
                target_sol_weight: dec!(0.5),
                band: dec!(0),
            },
            cost: CostModel::zero(),
            test,
        }
    }

    #[test]
    fn run_cells_matches_sequential_and_preserves_order() {
        let bars = cell_series(12);
        let cells: Vec<SweepCell> = (0..4).map(|i| reb_cell(i, 0..12)).collect();
        let seq = run_cells(&cells, &bars, dec!(1000), 365.0, Parallelism::Sequential).unwrap();
        let par = run_cells(
            &cells,
            &bars,
            dec!(1000),
            365.0,
            Parallelism::Threads(NonZeroUsize::new(3).unwrap()),
        )
        .unwrap();
        assert_eq!(seq, par, "parallel == sequential");
        assert_eq!(seq.len(), 4);
    }

    #[test]
    fn run_cells_propagates_error_from_an_empty_test_range() {
        let bars = cell_series(12);
        // An empty test range selects an empty slice → NoBars, propagated deterministically.
        let cells = vec![reb_cell(0, 5..5)];
        assert_eq!(
            run_cells(&cells, &bars, dec!(1000), 365.0, Parallelism::Sequential).unwrap_err(),
            SimError::NoBars
        );
    }
}
