//! Fixed-point money helpers.
//!
//! Every balance, price, fee, and equity value in machina is a [`Decimal`]. Floating point is
//! forbidden for money: it is non-associative, platform-sensitive, and would break the determinism
//! invariant. The helpers here centralise the few places where rounding or unit conversion happens
//! so the rounding policy is explicit and auditable.

use rust_decimal::Decimal;

/// USDC has 6 decimals on Solana.
pub const USDC_DECIMALS: u32 = 6;
/// Native SOL / wrapped SOL has 9 decimals.
pub const SOL_DECIMALS: u32 = 9;
/// 1 SOL = 1e9 lamports.
pub const LAMPORTS_PER_SOL: i64 = 1_000_000_000;

/// Truncate `value` toward zero to `decimals` fractional places.
///
/// This is the project's standard quantization for token outputs: never credit more than the exact
/// amount a real settlement could deliver. For non-negative token amounts it is a floor.
#[must_use]
pub fn quantize_floor(value: Decimal, decimals: u32) -> Decimal {
    value.trunc_with_scale(decimals)
}

/// Convert a lamport count to a SOL [`Decimal`] exactly (e.g. 5_000 lamports → `0.000005`).
#[must_use]
pub fn lamports_to_sol(lamports: i64) -> Decimal {
    Decimal::from(lamports) / Decimal::from(LAMPORTS_PER_SOL)
}

/// Multiply `value` by a basis-point fraction: `value * bps / 10_000`.
///
/// Used for DEX-fee and slippage application. Kept exact (no rounding) — callers quantize the final
/// token amount with [`quantize_floor`].
#[must_use]
pub fn apply_bps(value: Decimal, bps: u32) -> Decimal {
    value * Decimal::from(bps) / Decimal::from(10_000_u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn quantize_floors_toward_zero() {
        assert_eq!(quantize_floor(dec!(1.2399999), 6), dec!(1.239999));
        assert_eq!(
            quantize_floor(dec!(10.123456789), SOL_DECIMALS),
            dec!(10.123456789)
        );
        assert_eq!(
            quantize_floor(dec!(10.1234567891), SOL_DECIMALS),
            dec!(10.123456789)
        );
        // Exact values are unchanged.
        assert_eq!(quantize_floor(dec!(5), USDC_DECIMALS), dec!(5));
    }

    #[test]
    fn lamports_convert_exactly() {
        assert_eq!(lamports_to_sol(5_000), dec!(0.000005));
        assert_eq!(lamports_to_sol(55_000), dec!(0.000055));
        assert_eq!(lamports_to_sol(LAMPORTS_PER_SOL), dec!(1));
    }

    #[test]
    fn bps_application_is_exact() {
        // 20 bps of 1000 = 2.
        assert_eq!(apply_bps(dec!(1000), 20), dec!(2));
        // 5 bps of 100 = 0.05.
        assert_eq!(apply_bps(dec!(100), 5), dec!(0.05));
        assert_eq!(apply_bps(dec!(100), 0), dec!(0));
    }
}
