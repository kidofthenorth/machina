//! Equity-curve and round-trip records produced by the simulator.

use research_core::{Decimal, Timestamp};

/// One point on the equity curve: mark-to-market portfolio value at a bar close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquityPoint {
    /// Timestamp of the bar whose close this point marks.
    pub ts: Timestamp,
    /// Portfolio value in USDC at that bar's close (cash + SOL × close).
    pub equity_quote: Decimal,
}

/// A completed round trip: flat → long → flat.
///
/// `pnl_quote` is the *exact realized* USDC change across the trip (cash at close − cash at open),
/// so it already includes every fee, slippage, and gas cost paid while the position was open. The
/// price fields are the execution prices at entry/exit and are descriptive only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundTrip {
    /// Timestamp of the bar whose open executed the opening buy.
    pub entry_ts: Timestamp,
    /// Timestamp of the bar whose open executed the closing sell.
    pub exit_ts: Timestamp,
    /// Execution (mid) price of the opening buy, USDC per SOL.
    pub entry_price: Decimal,
    /// Execution (mid) price of the closing sell, USDC per SOL.
    pub exit_price: Decimal,
    /// SOL quantity bought when the position opened.
    pub qty_base: Decimal,
    /// Realized USDC P&L over the trip (already net of all costs).
    pub pnl_quote: Decimal,
}
