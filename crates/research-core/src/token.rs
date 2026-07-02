//! Token identity and metadata.
//!
//! [`MintAddress`] is validated at construction (base58, plausible length) so a malformed mint can
//! never silently flow into accounting or an allowlist check.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated SPL mint address (base58, 32–44 chars).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MintAddress(String);

impl MintAddress {
    /// Validate and construct. Rejects implausible lengths and non-base58 characters.
    pub fn new(s: impl Into<String>) -> Result<Self, TokenError> {
        let s = s.into();
        let len = s.len();
        if !(32..=44).contains(&len) {
            return Err(TokenError::InvalidMintLength(len));
        }
        if let Some(bad) = s.chars().find(|c| !is_base58(*c)) {
            return Err(TokenError::InvalidMintChar(bad));
        }
        Ok(Self(s))
    }

    /// The validated mint address as a base58 string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MintAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for MintAddress {
    type Error = TokenError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<MintAddress> for String {
    fn from(m: MintAddress) -> Self {
        m.0
    }
}

/// Static metadata for an allowlisted token. Extra survivorship fields (dates, liquidity, custody)
/// live alongside the allowlist loader; this is the minimal identity used by accounting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenMeta {
    /// Stable internal id, e.g. `"SOL"`.
    pub token_id: String,
    /// Display symbol, e.g. `"SOL"`.
    pub symbol: String,
    /// On-chain mint address.
    pub mint: MintAddress,
    /// Token decimals, verified from mint metadata in real ingestion (0–18 on Solana).
    pub decimals: u32,
}

impl TokenMeta {
    /// Validate and construct token metadata.
    ///
    /// Rejects `decimals` outside Solana's `0..=18` range so a malformed decimals value can never
    /// silently distort quantization or accounting. Callers that build a [`TokenMeta`] from
    /// untrusted metadata should prefer this over the struct literal.
    ///
    /// # Errors
    /// Returns [`TokenError::InvalidDecimals`] if `decimals > 18`.
    pub fn new(
        token_id: impl Into<String>,
        symbol: impl Into<String>,
        mint: MintAddress,
        decimals: u32,
    ) -> Result<Self, TokenError> {
        if decimals > 18 {
            return Err(TokenError::InvalidDecimals(decimals));
        }
        Ok(Self {
            token_id: token_id.into(),
            symbol: symbol.into(),
            mint,
            decimals,
        })
    }
}

/// Errors constructing token primitives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenError {
    /// Mint address length (in chars) outside the plausible 32–44 range.
    InvalidMintLength(usize),
    /// First non-base58 character found in the mint address.
    InvalidMintChar(char),
    /// Token decimals outside Solana's `0..=18` range.
    InvalidDecimals(u32),
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMintLength(n) => {
                write!(
                    f,
                    "mint address has implausible length {n} (expected 32–44)"
                )
            }
            Self::InvalidMintChar(c) => write!(f, "mint address contains non-base58 char {c:?}"),
            Self::InvalidDecimals(n) => {
                write!(f, "token decimals {n} out of range (expected 0–18)")
            }
        }
    }
}

impl std::error::Error for TokenError {}

/// True if `c` is in the Bitcoin/base58 alphabet (no `0`, `O`, `I`, `l`).
fn is_base58(c: char) -> bool {
    matches!(c,
        '1'..='9'
        | 'A'..='H' | 'J'..='N' | 'P'..='Z'
        | 'a'..='k' | 'm'..='z')
}

#[cfg(test)]
mod tests {
    use super::*;

    const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    const WSOL: &str = "So11111111111111111111111111111111111111112";

    #[test]
    fn accepts_real_mints() {
        assert!(MintAddress::new(USDC).is_ok());
        assert!(MintAddress::new(WSOL).is_ok());
    }

    #[test]
    fn rejects_short_and_long() {
        assert_eq!(
            MintAddress::new("tooshort").unwrap_err(),
            TokenError::InvalidMintLength(8)
        );
        let long = "1".repeat(45);
        assert_eq!(
            MintAddress::new(long).unwrap_err(),
            TokenError::InvalidMintLength(45)
        );
    }

    #[test]
    fn rejects_non_base58() {
        // '0' is not in the base58 alphabet; pad to a valid length so the charset check fires.
        let bad = format!("0{}", "1".repeat(33));
        assert_eq!(
            MintAddress::new(bad).unwrap_err(),
            TokenError::InvalidMintChar('0')
        );
    }

    #[test]
    fn serde_roundtrips_and_validates() {
        let m = MintAddress::new(USDC).unwrap();
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, format!("\"{USDC}\""));
        let back: MintAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        // Deserialization rejects an invalid mint.
        assert!(serde_json::from_str::<MintAddress>("\"nope\"").is_err());
    }

    #[test]
    fn accepts_inclusive_length_bounds() {
        // 32 and 44 are the inclusive plausible-length bounds; 31 and 45 are not.
        assert!(MintAddress::new("1".repeat(32)).is_ok());
        assert!(MintAddress::new("1".repeat(44)).is_ok());
        assert_eq!(
            MintAddress::new("1".repeat(31)).unwrap_err(),
            TokenError::InvalidMintLength(31)
        );
    }

    #[test]
    fn as_str_and_display_agree() {
        let m = MintAddress::new(USDC).unwrap();
        assert_eq!(m.as_str(), USDC);
        assert_eq!(m.to_string(), USDC);
    }

    #[test]
    fn token_errors_display_reasons() {
        assert!(TokenError::InvalidMintLength(8)
            .to_string()
            .contains("implausible length 8"));
        assert!(TokenError::InvalidMintChar('0')
            .to_string()
            .contains("non-base58"));
        assert!(TokenError::InvalidDecimals(30)
            .to_string()
            .contains("out of range"));
    }

    #[test]
    fn token_meta_new_validates_decimals() {
        let mint = MintAddress::new(WSOL).unwrap();
        let meta = TokenMeta::new("SOL", "SOL", mint.clone(), 9).unwrap();
        assert_eq!(meta.token_id, "SOL");
        assert_eq!(meta.decimals, 9);
        // 0 and 18 are the inclusive bounds; 19 is rejected.
        assert!(TokenMeta::new("X", "X", mint.clone(), 0).is_ok());
        assert!(TokenMeta::new("X", "X", mint.clone(), 18).is_ok());
        assert_eq!(
            TokenMeta::new("X", "X", mint, 19).unwrap_err(),
            TokenError::InvalidDecimals(19)
        );
    }

    #[test]
    fn token_meta_serde_round_trips() {
        let meta = TokenMeta::new("SOL", "SOL", MintAddress::new(WSOL).unwrap(), 9).unwrap();
        let json = serde_json::to_string(&meta).unwrap();
        let back: TokenMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back, meta);
    }
}
