//! Integration tests over the checked-in bar fixtures, covering the §19 M1 gate data cases:
//! corrupt (bad-OHLC), duplicate, unsorted, negative-volume, missing (gap), and disallowed-token.
//! Tiny synthetic data only — no network.

use market_data::{validate_series, validate_series_spacing, Allowlist, DataError};
use research_core::Bar;

/// The active allowlist template, embedded so the disallowed-token gate is testable without files.
const ALLOWLIST_TEMPLATE: &str = include_str!("../../../config/tokens/allowlist.example.toml");

/// Daily bar spacing in seconds (the fixtures use daily bars).
const DAILY: i64 = 86_400;

fn load(name: &str) -> Vec<Bar> {
    let path = format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

#[test]
fn good_fixture_validates() {
    assert!(validate_series(&load("bars_good.json")).is_ok());
    // bars_good is daily-spaced, so the stricter spacing check also passes.
    assert!(validate_series_spacing(&load("bars_good.json"), DAILY).is_ok());
}

#[test]
fn unsorted_fixture_rejected() {
    assert!(matches!(
        validate_series(&load("bars_unsorted.json")).unwrap_err(),
        DataError::Unsorted { .. }
    ));
}

#[test]
fn duplicate_fixture_rejected() {
    assert!(matches!(
        validate_series(&load("bars_duplicate.json")).unwrap_err(),
        DataError::DuplicateTimestamp { .. }
    ));
}

#[test]
fn bad_ohlc_fixture_rejected() {
    assert!(matches!(
        validate_series(&load("bars_bad_ohlc.json")).unwrap_err(),
        DataError::Ohlc(_)
    ));
}

#[test]
fn negative_volume_fixture_rejected() {
    assert!(matches!(
        validate_series(&load("bars_negative_volume.json")).unwrap_err(),
        DataError::Ohlc(_)
    ));
}

#[test]
fn missing_bar_fixture_rejected() {
    // bars_gap has a one-day hole; against a daily expected interval it is rejected as a gap.
    assert!(matches!(
        validate_series_spacing(&load("bars_gap.json"), DAILY).unwrap_err(),
        DataError::Gap { .. }
    ));
    // The looser validator (no expected interval) accepts it — gaps are allowed only by explicit
    // scenario, never silently forward-filled.
    assert!(validate_series(&load("bars_gap.json")).is_ok());
}

#[test]
fn disallowed_token_rejected() {
    // A run may only touch allowlisted tokens (invariant #4). Loading the active allowlist and
    // requiring an absent token yields NotAllowlisted — the gate that precedes any run.
    let allowlist =
        Allowlist::from_toml_str(ALLOWLIST_TEMPLATE).expect("allowlist template parses");
    assert!(allowlist.require("SOL").is_ok());
    assert_eq!(
        allowlist.require("WIF").unwrap_err(),
        DataError::NotAllowlisted {
            token_id: "WIF".into()
        }
    );
}
