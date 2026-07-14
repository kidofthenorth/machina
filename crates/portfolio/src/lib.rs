//! `portfolio` — deterministic portfolio accounting and the spot research simulator.
//!
//! Layers, bottom-up:
//! - [`cost::CostModel`] + `fill_buy`/`fill_sell`: the pure, deterministic fill model.
//! - [`state::PortfolioState`]: balances and the `apply_buy`/`apply_sell` accounting primitives that
//!   refuse to oversell, overspend, or run out of gas.
//! - [`simulator::run`]: next-bar execution over a bar series, producing an equity curve, round
//!   trips, and cost aggregates.
//! - [`latency::run_hf`]: the latency-aware HF entry point (m-hf-track §3); next-bar
//!   [`simulator::run`] is its `fixed_latency(1)` special case, proven by regression.
//!
//! Nothing here is a strategy: the simulator takes a `FnMut(&[Bar]) -> Decimal` target-weight
//! function. Strategies live in the `strategies` crate and produce intent only.

pub mod cost;
pub mod equity;
pub mod latency;
pub mod simulator;
pub mod state;

pub use cost::{BuyFill, CostModel, SellFill, Side};
pub use equity::{EquityPoint, RoundTrip};
pub use latency::{
    build_landing_table, fixed_latency, run_hf, HfError, HfRunOutput, LandingOutcome,
    LatencyPipeline,
};
pub use simulator::{run, RunOutput};
pub use state::{PortfolioState, SimError, TradeOutcome};
