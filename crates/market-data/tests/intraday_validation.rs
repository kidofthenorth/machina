//! Integration tests over the checked-in intraday fixtures (HF plan §2.1, card M-HF-C1).
//! Tiny synthetic data only — no network.

use market_data::{
    validate_prints, validate_series, validate_series_spacing, validate_snapshots,
    validate_snapshots_contiguous, IntradayError,
};
use research_core::{Bar, IntradayItemError, SlotSnapshot, Timestamp, TradePrint};

fn load_prints(name: &str) -> Vec<TradePrint> {
    let path = format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

fn load_snapshots(name: &str) -> Vec<SlotSnapshot> {
    let path = format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

#[test]
fn good_prints_fixture_validates() {
    assert!(validate_prints(&load_prints("prints_good.json")).is_ok());
}

#[test]
fn unsorted_prints_fixture_rejected() {
    assert!(matches!(
        validate_prints(&load_prints("prints_unsorted.json")).unwrap_err(),
        IntradayError::OutOfOrder { .. }
    ));
}

#[test]
fn duplicate_prints_fixture_rejected() {
    assert!(matches!(
        validate_prints(&load_prints("prints_duplicate.json")).unwrap_err(),
        IntradayError::DuplicateKey { .. }
    ));
}

#[test]
fn bad_size_prints_fixture_rejected() {
    assert!(matches!(
        validate_prints(&load_prints("prints_bad_size.json")).unwrap_err(),
        IntradayError::Item(IntradayItemError::NonPositiveSize { .. })
    ));
}

#[test]
fn snapshot_fixtures_good_and_gap() {
    let good = load_snapshots("snapshots_good.json");
    assert!(validate_snapshots(&good).is_ok());
    assert!(validate_snapshots_contiguous(&good).is_ok());

    // Gaps allowed only by explicit scenario, never silently: the loose validator accepts a
    // slot gap, the strict contiguous validator rejects it.
    let gap = load_snapshots("snapshots_slot_gap.json");
    assert!(validate_snapshots(&gap).is_ok());
    assert!(matches!(
        validate_snapshots_contiguous(&gap).unwrap_err(),
        IntradayError::SlotGap { .. }
    ));
}

#[test]
fn one_second_bars_use_existing_validators() {
    fn bar(unix: i64) -> Bar {
        Bar {
            ts: Timestamp::from_unix(unix),
            open: 10.into(),
            high: 12.into(),
            low: 9.into(),
            close: 11.into(),
            volume: 1.into(),
        }
    }

    let bars = vec![bar(0), bar(1), bar(2)];
    assert!(validate_series_spacing(&bars, 1).is_ok());

    let gapped = vec![bar(0), bar(1), bar(3)];
    assert!(matches!(
        validate_series_spacing(&gapped, 1).unwrap_err(),
        market_data::DataError::Gap { .. }
    ));
    // The declared-gap scenario path still accepts it.
    assert!(validate_series(&gapped).is_ok());
}
