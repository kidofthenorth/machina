//! `sweep` — deterministic parameter sweeps and walk-forward evaluation (M4).
//!
//! This crate turns the single-run research pipeline
//! (`portfolio::run` → `metrics::Metrics::from_equity` → `results::RunResult`) into a **parameter
//! sweep**: a Cartesian product of strategy parameters × cost scenarios × walk-forward windows,
//! evaluated in parallel, with canonical (thread-order-independent) output, turnover and
//! fee-sensitivity reporting, strategy-family comparison, and a robustness rejection report.
//!
//! ## Hard guarantees (see `docs/invariants.md` and `plans/m4-sweep.md`)
//! - **Determinism (invariant 3).** Same input → identical results, byte-for-byte. Parallel output
//!   equals sequential output equals repeated runs. Parallelism is `std::thread::scope` over a
//!   precomputed work list with disjoint writes; results are canonically sorted, independent of
//!   thread scheduling. No RNG, no wall-clock in canonical output.
//! - **Intent only (invariant 6).** Sweeps build `Box<dyn strategies::Strategy>` and read market
//!   history; they own no keys, call no RPC, and submit nothing. This crate depends on **no**
//!   network/SDK/wallet crate.
//! - **Sealed holdout.** Parameter selection runs only over development/validation partitions; the
//!   holdout is physically separated and reachable only through a single consume-by-value entry
//!   point used in M5, never from the M4 CLI.
//! - **Fixed-point money (invariant 7).** All money and the turnover ratio are `rust_decimal::Decimal`;
//!   `f64` appears only in display-only statistics, normalized so it never affects ordering.
//! - **No profitability claims (invariant 11).** Comparison and rejection are framed as robustness
//!   filtering, never as evidence a strategy works.
//!
//! The crate is built in small milestone-ordered diffs (plan §14, subtasks S4–S12). Public items
//! are introduced as each subtask lands.

pub mod cell;
pub mod parallel;
pub mod param;
pub mod turnover;

pub use cell::{eval_cell, CellResult};
pub use parallel::{run_cells, run_in_parallel, Parallelism, SweepCell};
pub use param::{build_strategy, ParamGrid, ParamPoint};
pub use turnover::turnover_ratio;
