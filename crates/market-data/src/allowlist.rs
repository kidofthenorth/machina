//! The token allowlist: load from TOML, query by token id.
//!
//! The allowlist is a first-class research artifact (plan §11). Its `version` is embedded into every
//! run result for survivorship auditing. No strategy may trade a token absent from it.

use crate::validation::{validate_token_decimals, DataError};
use research_core::{Decimal, MintAddress, TokenMeta};
use serde::Deserialize;
use std::collections::BTreeMap;

/// A fully-parsed, validated allowlist entry with its survivorship metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowlistEntry {
    pub meta: TokenMeta,
    /// First date (YYYY-MM-DD) the token may be traded.
    pub first_allowed: String,
    /// Last allowed date, or `None` if still active.
    pub last_allowed: Option<String>,
    pub venues: Vec<String>,
    pub min_liquidity_usdc: Decimal,
    pub min_age_days: u32,
    pub risk_category: String,
    pub custody_notes: String,
}

/// An immutable, version-stamped allowlist keyed by `token_id` (ordered for determinism).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowlist {
    version: String,
    entries: BTreeMap<String, AllowlistEntry>,
}

impl Allowlist {
    /// Parse and validate an allowlist from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, AllowlistError> {
        let file: AllowlistFile = toml::from_str(s).map_err(AllowlistError::Toml)?;
        if file.version.trim().is_empty() {
            return Err(AllowlistError::MissingVersion);
        }
        let mut entries = BTreeMap::new();
        for raw in file.token {
            let entry = raw.into_entry()?;
            if entries.contains_key(&entry.meta.token_id) {
                return Err(AllowlistError::DuplicateToken(entry.meta.token_id));
            }
            entries.insert(entry.meta.token_id.clone(), entry);
        }
        if entries.is_empty() {
            return Err(AllowlistError::Empty);
        }
        Ok(Self {
            version: file.version,
            entries,
        })
    }

    /// The allowlist version string (embedded into run results).
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Look up an entry by token id.
    #[must_use]
    pub fn get(&self, token_id: &str) -> Option<&AllowlistEntry> {
        self.entries.get(token_id)
    }

    /// Whether a token id is present.
    #[must_use]
    pub fn contains(&self, token_id: &str) -> bool {
        self.entries.contains_key(token_id)
    }

    /// Require a token to be allowlisted, returning its entry or [`DataError::NotAllowlisted`].
    ///
    /// This is the gate strategies/accounting must pass before touching a token.
    pub fn require(&self, token_id: &str) -> Result<&AllowlistEntry, DataError> {
        self.get(token_id).ok_or_else(|| DataError::NotAllowlisted {
            token_id: token_id.to_owned(),
        })
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the allowlist has no entries (never true after a successful load).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Token ids in deterministic (sorted) order.
    pub fn token_ids(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

/// Errors loading an allowlist.
#[derive(Debug)]
pub enum AllowlistError {
    Toml(toml::de::Error),
    MissingVersion,
    Empty,
    DuplicateToken(String),
    InvalidMint {
        token_id: String,
        source: research_core::TokenError,
    },
    Data(DataError),
}

impl std::fmt::Display for AllowlistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Toml(e) => write!(f, "invalid allowlist TOML: {e}"),
            Self::MissingVersion => write!(f, "allowlist is missing a non-empty `version`"),
            Self::Empty => write!(f, "allowlist contains no tokens"),
            Self::DuplicateToken(t) => write!(f, "duplicate token_id {t} in allowlist"),
            Self::InvalidMint { token_id, source } => {
                write!(f, "token {token_id}: {source}")
            }
            Self::Data(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AllowlistError {}

impl From<DataError> for AllowlistError {
    fn from(e: DataError) -> Self {
        Self::Data(e)
    }
}

// ── TOML DTOs ────────────────────────────────────────────────────────────────
// Decimal-ish fields are parsed as strings then converted explicitly, so the TOML
// shape never depends on numeric-deserialization quirks.

#[derive(Deserialize)]
struct AllowlistFile {
    #[serde(default)]
    version: String,
    #[serde(default)]
    #[allow(dead_code)]
    description: Option<String>,
    #[serde(default)]
    token: Vec<TokenEntryDto>,
}

#[derive(Deserialize)]
struct TokenEntryDto {
    token_id: String,
    symbol: String,
    mint: String,
    decimals: u32,
    first_allowed: String,
    #[serde(default)]
    last_allowed: String,
    #[serde(default)]
    venues: Vec<String>,
    #[serde(default)]
    min_liquidity_usdc: String,
    #[serde(default)]
    min_age_days: u32,
    #[serde(default)]
    risk_category: String,
    #[serde(default)]
    custody_notes: String,
}

impl TokenEntryDto {
    fn into_entry(self) -> Result<AllowlistEntry, AllowlistError> {
        let mint = MintAddress::new(self.mint).map_err(|source| AllowlistError::InvalidMint {
            token_id: self.token_id.clone(),
            source,
        })?;
        let meta = TokenMeta {
            token_id: self.token_id,
            symbol: self.symbol,
            mint,
            decimals: self.decimals,
        };
        validate_token_decimals(&meta)?;
        let min_liquidity_usdc = if self.min_liquidity_usdc.trim().is_empty() {
            Decimal::ZERO
        } else {
            self.min_liquidity_usdc.parse::<Decimal>().map_err(|_| {
                AllowlistError::Data(DataError::BadDecimals {
                    token_id: meta.token_id.clone(),
                    decimals: meta.decimals,
                })
            })?
        };
        let last_allowed = if self.last_allowed.trim().is_empty() {
            None
        } else {
            Some(self.last_allowed)
        };
        Ok(AllowlistEntry {
            meta,
            first_allowed: self.first_allowed,
            last_allowed,
            venues: self.venues,
            min_liquidity_usdc,
            min_age_days: self.min_age_days,
            risk_category: self.risk_category,
            custody_notes: self.custody_notes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const SAMPLE: &str = r#"
version = "2026-06-29"
[[token]]
token_id = "USDC"
symbol = "USDC"
mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
decimals = 6
first_allowed = "2021-01-01"
last_allowed = ""
venues = ["jupiter"]
min_liquidity_usdc = "100000000"
risk_category = "stablecoin"
[[token]]
token_id = "SOL"
symbol = "SOL"
mint = "So11111111111111111111111111111111111111112"
decimals = 9
first_allowed = "2021-01-01"
venues = ["jupiter"]
min_liquidity_usdc = "50000000"
risk_category = "blue_chip"
"#;

    #[test]
    fn parses_sample() {
        let al = Allowlist::from_toml_str(SAMPLE).unwrap();
        assert_eq!(al.version(), "2026-06-29");
        assert_eq!(al.len(), 2);
        let sol = al.require("SOL").unwrap();
        assert_eq!(sol.meta.decimals, 9);
        assert_eq!(sol.min_liquidity_usdc, dec!(50000000));
        assert_eq!(sol.last_allowed, None);
        // Deterministic ordering: USDC sorts before SOL.
        assert_eq!(al.token_ids().collect::<Vec<_>>(), vec!["SOL", "USDC"]);
    }

    #[test]
    fn unknown_token_blocked() {
        let al = Allowlist::from_toml_str(SAMPLE).unwrap();
        assert!(!al.contains("WIF"));
        assert_eq!(
            al.require("WIF").unwrap_err(),
            DataError::NotAllowlisted {
                token_id: "WIF".into()
            }
        );
    }

    #[test]
    fn rejects_missing_version() {
        let toml = r#"
[[token]]
token_id = "SOL"
symbol = "SOL"
mint = "So11111111111111111111111111111111111111112"
decimals = 9
first_allowed = "2021-01-01"
"#;
        assert!(matches!(
            Allowlist::from_toml_str(toml),
            Err(AllowlistError::MissingVersion)
        ));
    }

    #[test]
    fn rejects_bad_mint() {
        let toml = r#"
version = "v1"
[[token]]
token_id = "BAD"
symbol = "BAD"
mint = "not-a-valid-mint"
decimals = 9
first_allowed = "2021-01-01"
"#;
        assert!(matches!(
            Allowlist::from_toml_str(toml),
            Err(AllowlistError::InvalidMint { .. })
        ));
    }

    #[test]
    fn rejects_bad_decimals() {
        let toml = r#"
version = "v1"
[[token]]
token_id = "BAD"
symbol = "BAD"
mint = "So11111111111111111111111111111111111111112"
decimals = 99
first_allowed = "2021-01-01"
"#;
        assert!(matches!(
            Allowlist::from_toml_str(toml),
            Err(AllowlistError::Data(DataError::BadDecimals {
                decimals: 99,
                ..
            }))
        ));
    }

    #[test]
    fn example_template_parses() {
        // Proves the shipped config template is valid and loadable.
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/tokens/allowlist.example.toml"
        );
        let text = std::fs::read_to_string(path).expect("read allowlist template");
        let al = Allowlist::from_toml_str(&text).expect("parse allowlist template");
        assert!(al.contains("SOL") && al.contains("USDC"));
        assert_eq!(al.version(), "2026-06-29");
    }
}
