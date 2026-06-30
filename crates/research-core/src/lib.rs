//! `research-core` — the domain foundation shared by every research crate.
//!
//! Nothing here performs I/O, touches the network, or signs anything. It exists to make the
//! determinism and accounting invariants (see `docs/invariants.md`) structurally true:
//!
//! - **Fixed-point money.** All balances, prices, fees, and equity are [`rust_decimal::Decimal`].
//!   Never use floating point for money. See [`money`].
//! - **UTC time.** [`time::Timestamp`] is Unix seconds (UTC); ordering is total and deterministic.
//! - **Validated primitives.** [`token::MintAddress`] and [`bar::Bar`] reject malformed inputs at
//!   construction / via explicit checks rather than silently accepting bad data.

pub mod bar;
pub mod money;
pub mod time;
pub mod token;

pub use bar::{Bar, BarError};
pub use money::{
    apply_bps, lamports_to_sol, quantize_floor, LAMPORTS_PER_SOL, SOL_DECIMALS, USDC_DECIMALS,
};
pub use time::{civil_date_to_unix, parse_ymd, DateParseError, Timestamp};
pub use token::{MintAddress, TokenError, TokenMeta};

// Re-export Decimal so downstream crates use exactly one Decimal type/version.
pub use rust_decimal::Decimal;
