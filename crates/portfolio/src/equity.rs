//! Equity-curve and round-trip records produced by the simulator.

use research_core::{Decimal, Timestamp};

/// One point on the equity curve: mark-to-market portfolio value at a bar close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquityPoint {
    pub ts: Timestamp,
    pub equity_quote: Decimal,
}

/// A completed round trip: flat → long → flat.
///
/// `pnl_quote` is the *exact realized* USDC change across the trip (cash at close − cash at open),
/// so it already includes every fee, slippage, and gas cost paid while the position was open. The
/// price fields are the execution prices at entry/exit and are descriptive only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundTrip {
    pub entry_ts: Timestamp,
    pub exit_ts: Timestamp,
    pub entry_price: Decimal,
    pub exit_price: Decimal,
    /// SOL quantity bought when the position opened.
    pub qty_base: Decimal,
    /// Realized USDC P&L over the trip (already net of all costs).
    pub pnl_quote: Decimal,
}
