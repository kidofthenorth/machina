//! Intraday market events: trade prints and per-slot state snapshots (HF plan §2.1).
//!
//! Same split as [`crate::bar`]: an item only knows how to validate itself; series-level checks
//! (single venue, sorted, unique per key, slot-gap policy) live in `market-data`. Research data
//! only — these types carry no execution capability.

use crate::time::Timestamp;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Aggressor side of a trade print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

/// Where a series came from — a typed field, not prose (m-hf-track §2). A result computed
/// over any synthetic input can never render as real: reports roll this up as a computed
/// fact, never a hand-written label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Provenance {
    /// Deterministic generated data; `spec_hash` identifies the generating spec exactly.
    Synthetic { spec_hash: u64 },
    /// Operator-ingested archived data; `source_id` names the snapshot record.
    Real { source_id: String },
}

/// 1-second bars need no new type: an intraday bar IS a [`crate::Bar`] at finer spacing —
/// a series-level interval fact asserted by `validate_series_spacing(bars, 1)`, not a
/// type-level one. Zero logic (m-hf-track §1).
pub type IntraBar = crate::Bar;

/// One executed trade on one venue. Ordered and deduplicated by `(slot, seq)` within a venue
/// (`seq` disambiguates multiple prints landing in the same slot).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradePrint {
    pub venue: String,
    /// Solana slot in which the trade landed.
    pub slot: u64,
    /// Intra-slot sequence number (0-based) — the uniqueness key together with `slot`.
    pub seq: u32,
    /// Wall-clock time of the slot (UTC). Carried for bar alignment; ordering uses (slot, seq).
    pub ts: Timestamp,
    pub side: Side,
    /// Trade price in quote units; must be > 0.
    pub price: Decimal,
    /// Trade size in base units; must be > 0.
    pub size: Decimal,
}

/// One market-state observation for one venue at one slot (pool/book mid).
/// Depth fields arrive with the HF fill/cost cards — do not add them here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotSnapshot {
    pub venue: String,
    pub slot: u64,
    pub ts: Timestamp,
    /// Mid price in quote units; must be > 0.
    pub mid: Decimal,
}

/// A single-item validation failure, tagged with the offending key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntradayItemError {
    NonPositivePrice { slot: u64, seq: u32 },
    NonPositiveSize { slot: u64, seq: u32 },
    NonPositiveMid { slot: u64 },
}

impl std::fmt::Display for IntradayItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonPositivePrice { slot, seq } => {
                write!(f, "print (slot {slot}, seq {seq}): price must be > 0")
            }
            Self::NonPositiveSize { slot, seq } => {
                write!(f, "print (slot {slot}, seq {seq}): size must be > 0")
            }
            Self::NonPositiveMid { slot } => {
                write!(f, "snapshot (slot {slot}): mid must be > 0")
            }
        }
    }
}

impl std::error::Error for IntradayItemError {}

impl TradePrint {
    /// Validate this print's single-item invariants: `price > 0`, `size > 0`.
    pub fn check(&self) -> Result<(), IntradayItemError> {
        if self.price <= Decimal::ZERO {
            return Err(IntradayItemError::NonPositivePrice {
                slot: self.slot,
                seq: self.seq,
            });
        }
        if self.size <= Decimal::ZERO {
            return Err(IntradayItemError::NonPositiveSize {
                slot: self.slot,
                seq: self.seq,
            });
        }
        Ok(())
    }
}

impl SlotSnapshot {
    /// Validate this snapshot's single-item invariant: `mid > 0`.
    pub fn check(&self) -> Result<(), IntradayItemError> {
        if self.mid <= Decimal::ZERO {
            return Err(IntradayItemError::NonPositiveMid { slot: self.slot });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn print(slot: u64, seq: u32, price: Decimal, size: Decimal) -> TradePrint {
        TradePrint {
            venue: "venue_a".into(),
            slot,
            seq,
            ts: Timestamp::from_unix(1_609_459_200),
            side: Side::Buy,
            price,
            size,
        }
    }

    fn snap(slot: u64, mid: Decimal) -> SlotSnapshot {
        SlotSnapshot {
            venue: "venue_a".into(),
            slot,
            ts: Timestamp::from_unix(1_609_459_200),
            mid,
        }
    }

    #[test]
    fn valid_print_and_snapshot_pass_check() {
        let p = print(1000, 0, dec!(100.25), dec!(1.5));
        assert!(p.check().is_ok());
        let s = snap(1000, dec!(100));
        assert!(s.check().is_ok());
    }

    #[test]
    fn non_positive_price_size_mid_rejected() {
        let p = print(1000, 0, dec!(0), dec!(1.5));
        assert!(matches!(
            p.check(),
            Err(IntradayItemError::NonPositivePrice { slot: 1000, seq: 0 })
        ));

        let p = print(1000, 0, dec!(100.25), dec!(-1));
        assert!(matches!(
            p.check(),
            Err(IntradayItemError::NonPositiveSize { slot: 1000, seq: 0 })
        ));

        let s = snap(1000, dec!(0));
        assert!(matches!(
            s.check(),
            Err(IntradayItemError::NonPositiveMid { slot: 1000 })
        ));
    }

    #[test]
    fn print_serde_round_trips_with_lowercase_side() {
        let p = print(1000, 0, dec!(100.25), dec!(1.5));
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"side\":\"buy\""));
        let back: TradePrint = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn snapshot_serde_round_trips() {
        let s = snap(1000, dec!(100.25));
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"ts\":1609459200"));
        let back: SlotSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn provenance_serde_round_trips_tagged() {
        let synthetic = Provenance::Synthetic { spec_hash: 42 };
        let json = serde_json::to_string(&synthetic).unwrap();
        assert!(json.contains("\"kind\":\"synthetic\""));
        let back: Provenance = serde_json::from_str(&json).unwrap();
        assert_eq!(back, synthetic);

        let real = Provenance::Real {
            source_id: "binance-solusdc-1d".into(),
        };
        let json = serde_json::to_string(&real).unwrap();
        let back: Provenance = serde_json::from_str(&json).unwrap();
        assert_eq!(back, real);

        let alias_bar: IntraBar = crate::Bar {
            ts: Timestamp::from_unix(0),
            open: dec!(1),
            high: dec!(1),
            low: dec!(1),
            close: dec!(1),
            volume: dec!(0),
        };
        assert_eq!(alias_bar, alias_bar.clone());
    }
}
