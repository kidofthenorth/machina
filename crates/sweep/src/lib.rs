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

pub mod advance;
pub mod baseline;
pub mod cell;
pub mod config;
pub mod congestion;
pub mod hf_aggregate;
pub mod hf_cell;
pub mod hf_runner;
pub mod hf_scenarios;
pub mod hf_spec;
pub mod intraday_partition;
pub mod parallel;
pub mod param;
pub mod partition;
pub mod report;
pub mod runner;
pub mod sensitivity;
pub mod spec;
pub mod turnover;
pub mod window;

pub use advance::{
    evaluate_candidate, AdvancementThresholds, CandidateEvidence, CandidateVerdict, RejectionKind,
    RejectionReason, Verdict,
};
pub use baseline::{best_baseline_return, eval_all_baselines, eval_baseline, BaselineId};
pub use cell::{eval_cell, CellResult};
pub use config::PartitionSpec;
pub use congestion::classify_congestion_regimes;
pub use hf_aggregate::{aggregate_hf, HfAggregate, HfRungRollup};
pub use hf_cell::{eval_hf_cell, HfCellResult};
pub use hf_runner::{run_hf_sweep, HfSweepError, HfSweepOutcome};
pub use hf_scenarios::{hf_cost_scenarios, HfCostScenario, HfLatencyParams};
pub use hf_spec::{HfSpecError, HfSweepSpec};
pub use intraday_partition::{
    evaluate_intraday_on_holdout, IntradayDevValidation, IntradayPartitionError,
    IntradayPartitionedBars, IntradaySealed,
};
pub use parallel::{run_cells, run_in_parallel, Parallelism, SweepCell};
pub use param::{build_strategy, ParamGrid, ParamPoint};
pub use partition::{evaluate_on_holdout, DevValidation, PartitionError, PartitionedBars, Sealed};
pub use report::{HfRungsDto, SweepReport, ThresholdsDto, SWEEP_SCHEMA_VERSION};
pub use runner::{
    aggregate_evidence, aggregate_fee_sensitivity, enumerate_cells, run_sweep, CellKey, SweepError,
    SweepOutcome,
};
pub use sensitivity::{
    cost_scenarios, fee_sensitivity, scale_cost_model, CostScenario, FeeSensitivity, ScenarioId,
    ScenarioMetrics,
};
pub use spec::{SpecError, SweepSpec};
pub use turnover::turnover_ratio;
pub use window::{WalkForward, Window, WindowError, WindowKind};
