//! Deterministic synthetic intraday data (HF plan §2.1, card M-HF-C2). Research fixtures ONLY.
//!
//! Every output is a pure function of its [`SyntheticSpec`]: the "noise" is a fixed quantile
//! table indexed by a SplitMix64 integer hash stream seeded from the spec — no entropy source,
//! no clock, no `rand` dependency (invariant 3: no wall-clock/RNG in canonical runs). Same spec
//! → byte-identical output. Every bundle carries a typed `Provenance::Synthetic { spec_hash }`
//! into serialization, so no
//! synthetic series can masquerade as real data, and none of it may ever ground a statistical
//! claim about real markets (invariant 11's no-profitability-claims rule, extended here to
//! synthetic-vs-real labeling; real ingestion is M-HF-C9, gated on HF-Q1).

use crate::intraday::{validate_prints, validate_snapshots};
use crate::validation::validate_series_spacing;
use research_core::intraday::{Provenance, Side, SlotSnapshot, TradePrint};
use research_core::{Bar, Decimal, Timestamp};
use serde::Serialize;

/// Spec for one synthetic intraday series. The output is a pure function of this struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyntheticSpec {
    /// Seed of the SplitMix64 index stream.
    pub seed: u64,
    /// Number of 1-second steps (one bar per step); must be > 0.
    pub steps: usize,
    /// Unix seconds (UTC) of the first bar.
    pub start_unix: i64,
    /// First Solana slot. Slots advance 2–3 per step (deterministically), so snapshot series
    /// have realistic slot gaps — the declared-scenario path of M-HF-C1.
    pub start_slot: u64,
    /// Synthetic venue label (not a real venue).
    pub venue: String,
    /// Initial mid price; must be > 0.
    pub mid0: Decimal,
    /// OU anchor μ; must be > 0.
    pub anchor: Decimal,
    /// OU reversion θ per step; must be in [0, 1).
    pub reversion: Decimal,
    /// Noise step size σ in quote units; must be ≥ 0.
    pub vol_step: Decimal,
    /// Every `impact_every`-th step (i > 0) starts an impact event; 0 = never.
    pub impact_every: usize,
    /// Initial displacement magnitude of an impact event; must be ≥ 0.
    pub impact_size: Decimal,
    /// Per-step geometric decay of the outstanding impact; must be in [0, 1).
    pub impact_decay: Decimal,
    /// Steps per congestion regime segment (> 0): Calm → Busy → Hot → Calm → …
    pub regime_period: usize,
}

/// Congestion regime label (consumed by the HF cost cards).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Congestion {
    Calm,
    Busy,
    Hot,
}

/// Why a spec was rejected or generation failed its own hygiene check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntheticError {
    BadSpec(&'static str),
    /// Generated output failed M-HF-C1 validation — a generator bug by definition.
    SelfCheck(String),
}

impl std::fmt::Display for SyntheticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSpec(why) => write!(f, "bad synthetic spec: {why}"),
            Self::SelfCheck(e) => write!(f, "generator self-check failed: {e}"),
        }
    }
}

impl std::error::Error for SyntheticError {}

/// One generated bundle. `provenance` is always `Provenance::Synthetic { spec_hash }` and
/// serializes into every export — synthetic data must self-identify, typed, not prose
/// (m-hf-track §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyntheticIntraday {
    /// Always `Provenance::Synthetic { spec_hash: spec_hash(&spec) }`.
    pub provenance: Provenance,
    pub spec: SyntheticSpec,
    pub bars_1s: Vec<Bar>,
    pub prints: Vec<TradePrint>,
    pub snapshots: Vec<SlotSnapshot>,
    /// One label per step.
    pub congestion: Vec<Congestion>,
}

/// Identify the generating spec exactly: FNV-1a (64-bit) over every field's canonical string,
/// in declaration order, 0xFF-separated (same constants and separator discipline as
/// `binance_csv::fnv1a64`). Pure integer math; NO serde_json (dev-dep only in this crate).
fn spec_hash(spec: &SyntheticSpec) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let fields = [
        spec.seed.to_string(),
        spec.steps.to_string(),
        spec.start_unix.to_string(),
        spec.start_slot.to_string(),
        spec.venue.clone(),
        spec.mid0.to_string(),
        spec.anchor.to_string(),
        spec.reversion.to_string(),
        spec.vol_step.to_string(),
        spec.impact_every.to_string(),
        spec.impact_size.to_string(),
        spec.impact_decay.to_string(),
        spec.regime_period.to_string(),
    ];
    let mut hash = OFFSET_BASIS;
    for field in &fields {
        for byte in field.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(PRIME);
        }
        hash ^= 0xFF; // field separator: avoids concatenation ambiguity between fields
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// SplitMix64 — a deterministic integer hash sequence, NOT an entropy source.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Fixed symmetric noise quantile table (mean 0). `dec!` is dev-only, hence `Decimal::new`.
fn noise_table() -> [Decimal; 16] {
    [
        Decimal::new(-200, 2),
        Decimal::new(-150, 2),
        Decimal::new(-100, 2),
        Decimal::new(-75, 2),
        Decimal::new(-50, 2),
        Decimal::new(-25, 2),
        Decimal::new(-10, 2),
        Decimal::ZERO,
        Decimal::ZERO,
        Decimal::new(10, 2),
        Decimal::new(25, 2),
        Decimal::new(50, 2),
        Decimal::new(75, 2),
        Decimal::new(100, 2),
        Decimal::new(150, 2),
        Decimal::new(200, 2),
    ]
}

fn check_spec(spec: &SyntheticSpec) -> Result<(), SyntheticError> {
    if spec.steps == 0 {
        return Err(SyntheticError::BadSpec("steps must be > 0"));
    }
    if spec.mid0 <= Decimal::ZERO {
        return Err(SyntheticError::BadSpec("mid0 must be > 0"));
    }
    if spec.anchor <= Decimal::ZERO {
        return Err(SyntheticError::BadSpec("anchor must be > 0"));
    }
    if spec.reversion < Decimal::ZERO || spec.reversion >= Decimal::ONE {
        return Err(SyntheticError::BadSpec("reversion must be in [0, 1)"));
    }
    if spec.vol_step < Decimal::ZERO {
        return Err(SyntheticError::BadSpec("vol_step must be >= 0"));
    }
    if spec.impact_size < Decimal::ZERO {
        return Err(SyntheticError::BadSpec("impact_size must be >= 0"));
    }
    if spec.impact_decay < Decimal::ZERO || spec.impact_decay >= Decimal::ONE {
        return Err(SyntheticError::BadSpec("impact_decay must be in [0, 1)"));
    }
    if spec.regime_period == 0 {
        return Err(SyntheticError::BadSpec("regime_period must be > 0"));
    }
    Ok(())
}

/// Generate one synthetic bundle. Pure: same `spec` → byte-identical output, proven by test.
///
/// Per step: decay the outstanding impact, maybe start a new impact event, then
/// `next = mid + θ(μ − mid) + σ·ε + impact`, quantized to 9 dp (deterministic banker's
/// rounding) and floored at 0.01 so prices stay positive. The bar is (open = mid,
/// close = next); one print and one snapshot land on a slot that advances 2–3 per step.
pub fn generate(spec: &SyntheticSpec) -> Result<SyntheticIntraday, SyntheticError> {
    check_spec(spec)?;
    let noise = noise_table();
    let floor = Decimal::new(1, 2); // 0.01
    let mut state = spec.seed;
    let mut mid = spec.mid0;
    let mut impact = Decimal::ZERO;
    let mut slot = spec.start_slot;
    let mut bars_1s = Vec::with_capacity(spec.steps);
    let mut prints = Vec::with_capacity(spec.steps);
    let mut snapshots = Vec::with_capacity(spec.steps);
    let mut congestion = Vec::with_capacity(spec.steps);
    for i in 0..spec.steps {
        let r = splitmix64(&mut state);
        let eps = noise[(r % 16) as usize];
        impact *= spec.impact_decay;
        if spec.impact_every > 0 && i > 0 && i % spec.impact_every == 0 {
            let sign = if r & (1 << 20) == 0 {
                Decimal::ONE
            } else {
                -Decimal::ONE
            };
            impact = spec.impact_size * sign;
        }
        let mut next =
            (mid + spec.reversion * (spec.anchor - mid) + spec.vol_step * eps + impact).round_dp(9);
        if next < floor {
            next = floor;
        }
        let ts = Timestamp::from_unix(spec.start_unix + i as i64);
        let (high, low) = if next >= mid {
            (next, mid)
        } else {
            (mid, next)
        };
        let volume = Decimal::from(1 + (r >> 8) % 9);
        bars_1s.push(Bar {
            ts,
            open: mid,
            high,
            low,
            close: next,
            volume,
        });
        slot += 2 + ((r >> 16) & 1); // 2–3 slots per second: slot gaps are the honest default
        let side = if next >= mid { Side::Buy } else { Side::Sell };
        prints.push(TradePrint {
            venue: spec.venue.clone(),
            slot,
            seq: 0,
            ts,
            side,
            price: next,
            size: volume,
        });
        snapshots.push(SlotSnapshot {
            venue: spec.venue.clone(),
            slot,
            ts,
            mid: next,
        });
        congestion.push(match (i / spec.regime_period) % 3 {
            0 => Congestion::Calm,
            1 => Congestion::Busy,
            _ => Congestion::Hot,
        });
        mid = next;
    }
    // Self-check: generator output must satisfy the M-HF-C1 hygiene it will be tested against.
    validate_series_spacing(&bars_1s, 1).map_err(|e| SyntheticError::SelfCheck(e.to_string()))?;
    validate_prints(&prints).map_err(|e| SyntheticError::SelfCheck(e.to_string()))?;
    validate_snapshots(&snapshots).map_err(|e| SyntheticError::SelfCheck(e.to_string()))?;
    Ok(SyntheticIntraday {
        provenance: Provenance::Synthetic {
            spec_hash: spec_hash(spec),
        },
        spec: spec.clone(),
        bars_1s,
        prints,
        snapshots,
        congestion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intraday::validate_snapshots_contiguous;
    use rust_decimal_macros::dec;

    fn spec() -> SyntheticSpec {
        SyntheticSpec {
            seed: 42,
            steps: 600,
            start_unix: 1_609_459_200,
            start_slot: 100_000,
            venue: "venue_a".to_string(),
            mid0: dec!(100),
            anchor: dec!(100),
            reversion: dec!(0.05),
            vol_step: dec!(0.2),
            impact_every: 50,
            impact_size: dec!(1.5),
            impact_decay: dec!(0.5),
            regime_period: 100,
        }
    }

    #[test]
    fn same_spec_is_byte_identical() {
        let a = serde_json::to_string(&generate(&spec()).unwrap()).unwrap();
        let b = serde_json::to_string(&generate(&spec()).unwrap()).unwrap();
        assert_eq!(a, b);

        let mut other = spec();
        other.seed = 43;
        let c = serde_json::to_string(&generate(&other).unwrap()).unwrap();
        assert_ne!(a, c);
    }

    #[test]
    fn outputs_pass_c1_validation() {
        let out = generate(&spec()).unwrap();
        assert!(validate_series_spacing(&out.bars_1s, 1).is_ok());
        assert!(validate_prints(&out.prints).is_ok());
        assert!(validate_snapshots(&out.snapshots).is_ok());
        assert!(matches!(
            validate_snapshots_contiguous(&out.snapshots),
            Err(crate::intraday::IntradayError::SlotGap { .. })
        ));
    }

    #[test]
    fn output_self_identifies_as_synthetic() {
        let out = generate(&spec()).unwrap();
        assert!(matches!(out.provenance, Provenance::Synthetic { .. }));
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains("\"kind\":\"synthetic\""));
    }

    #[test]
    fn mean_reversion_pulls_toward_anchor() {
        let spec = SyntheticSpec {
            mid0: dec!(110),
            anchor: dec!(100),
            reversion: dec!(0.2),
            vol_step: dec!(0.05),
            impact_every: 0,
            steps: 100,
            ..spec()
        };
        let out = generate(&spec).unwrap();
        let dev: Vec<Decimal> = out.bars_1s.iter().map(|b| b.close - dec!(100)).collect();
        for i in 0..spec.steps - 1 {
            if dev[i].abs() > dec!(1) {
                assert!(dev[i + 1].abs() < dev[i].abs());
            }
        }
        assert!(dev[spec.steps - 1].abs() <= dec!(1));
    }

    #[test]
    fn impact_fires_and_decays_geometrically() {
        let spec = SyntheticSpec {
            vol_step: dec!(0),
            reversion: dec!(0),
            impact_every: 5,
            impact_size: dec!(2),
            impact_decay: dec!(0.5),
            steps: 10,
            ..spec()
        };
        let out = generate(&spec).unwrap();
        let d: Vec<Decimal> = out.bars_1s.iter().map(|b| b.close - b.open).collect();
        for item in d.iter().take(5) {
            assert_eq!(*item, Decimal::ZERO);
        }
        assert_eq!(d[5].abs(), dec!(2));
        assert_eq!(d[6] * dec!(2), d[5]);
        assert_eq!(d[7] * dec!(2), d[6]);
    }

    #[test]
    fn regimes_cycle_deterministically() {
        let out = generate(&spec()).unwrap();
        assert_eq!(out.congestion[0], Congestion::Calm);
        assert_eq!(out.congestion[100], Congestion::Busy);
        assert_eq!(out.congestion[200], Congestion::Hot);
        assert_eq!(out.congestion[300], Congestion::Calm);
        for start in [0usize, 100, 200, 300] {
            let label = out.congestion[start];
            for offset in 0..100 {
                assert_eq!(out.congestion[start + offset], label);
            }
        }
    }

    #[test]
    fn bad_specs_rejected() {
        assert_eq!(
            generate(&SyntheticSpec { steps: 0, ..spec() }),
            Err(SyntheticError::BadSpec("steps must be > 0"))
        );
        assert_eq!(
            generate(&SyntheticSpec {
                mid0: dec!(0),
                ..spec()
            }),
            Err(SyntheticError::BadSpec("mid0 must be > 0"))
        );
        assert_eq!(
            generate(&SyntheticSpec {
                reversion: dec!(1),
                ..spec()
            }),
            Err(SyntheticError::BadSpec("reversion must be in [0, 1)"))
        );
        assert_eq!(
            generate(&SyntheticSpec {
                regime_period: 0,
                ..spec()
            }),
            Err(SyntheticError::BadSpec("regime_period must be > 0"))
        );
    }
}
