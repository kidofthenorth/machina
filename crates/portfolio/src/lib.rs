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
//! - [`hf_cost::hf_trade_cost`]: additive HF cost terms (m-hf-track §3/§4) — depth-walk
//!   slippage, regime priority, tip; carried on [`hf_cost::HfCostModel`], not on [`cost::CostModel`]
//!   (see the module doc for why). Not yet wired into execution.
//!
//! Nothing here is a strategy: the simulator takes a `FnMut(&[Bar]) -> Decimal` target-weight
//! function. Strategies live in the `strategies` crate and produce intent only.

pub mod cost;
pub mod equity;
pub mod hf_cost;
pub mod latency;
pub mod simulator;
pub mod state;

pub use cost::{BuyFill, CostModel, SellFill, Side};
pub use equity::{EquityPoint, RoundTrip};
pub use hf_cost::{
    hf_trade_cost, scale_hf_cost_model, CongestionPriorityTable, CongestionRegime, DepthBand,
    DepthCurve, HfCostError, HfCostModel, HfTradeCost,
};
pub use latency::{
    build_landing_table, fixed_latency, run_hf, HfError, HfRunOutput, LandingOutcome,
    LatencyPipeline,
};
pub use simulator::{run, RunOutput};
pub use state::{PortfolioState, SimError, TradeOutcome};
