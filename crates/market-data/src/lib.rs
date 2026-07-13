//! `market-data` — data hygiene at the edge of the research engine.
//!
//! Two responsibilities:
//! 1. **Allowlist** ([`allowlist`]): load and query the token allowlist, a first-class research
//!    artifact. No strategy may trade a token absent from it.
//! 2. **Validation** ([`validation`]): reject corrupt, unsorted, duplicate, bad-OHLC,
//!    negative-volume, or gapped/missing bar series *before* they reach the simulator. Invalid or
//!    missing data blocks a run — there is no silent forward-fill.

pub mod allowlist;
pub mod binance_csv;
pub mod intraday;
pub mod synthetic;
pub mod validation;

pub use allowlist::{Allowlist, AllowlistEntry};
pub use intraday::{
    validate_prints, validate_snapshots, validate_snapshots_contiguous, IntradayError,
};
pub use synthetic::{generate, Congestion, SyntheticError, SyntheticIntraday, SyntheticSpec};
pub use validation::{
    validate_series, validate_series_spacing, validate_token_decimals, DataError,
};
