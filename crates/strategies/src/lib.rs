//! `strategies` — the Strategy Lab (M3).
//!
//! A [`Strategy`] maps completed market history (and the current SOL weight) to a **target SOL
//! weight in `[0,1]`**. That is the entire surface: strategies produce *intent only*. They do not
//! own keys, call RPC, fetch quotes, size orders, or touch execution state (see `docs/invariants.md`
//! invariant 6). The `portfolio` simulator turns a target weight into swap intents and fills.
//!
//! ## What ships here
//! - Baselines: [`baselines::HoldUsdc`], [`baselines::BuyAndHoldSol`], [`baselines::Static5050`],
//!   [`baselines::DcaIntoSol`].
//! - Scaffolds: [`trend_alloc::TrendAllocV1`], [`threshold_rebalance::ThresholdRebalanceV1`],
//!   [`regime::classify`] (regime classifier scaffold).
//!
//! These are **scaffolds with deterministic tests**, not tuned models, and imply **no**
//! profitability (invariant 11). Parameters are illustrative.

pub mod baselines;
pub mod regime;
pub mod threshold_rebalance;
pub mod trend_alloc;

use research_core::{Bar, Decimal};

/// A strategy: completed history + current SOL weight → target SOL weight in `[0,1]`.
///
/// Implementations must be **pure and deterministic**: the same arguments always yield the same
/// target. They must not read the clock, use randomness, or look at data beyond `history`.
pub trait Strategy {
    /// Stable strategy name, `<family>_v<n>` for versioned strategies.
    fn name(&self) -> &str;

    /// Target SOL weight given completed `history` (oldest first) and the `current_weight`.
    ///
    /// The simulator clamps the result into `[0,1]`, but implementations should already return a
    /// value in range.
    fn target_weight(&self, history: &[Bar], current_weight: Decimal) -> Decimal;
}

pub use baselines::{BuyAndHoldSol, DcaIntoSol, HoldUsdc, Static5050};
pub use regime::{classify as classify_regime, Regime};
pub use threshold_rebalance::ThresholdRebalanceV1;
pub use trend_alloc::TrendAllocV1;
