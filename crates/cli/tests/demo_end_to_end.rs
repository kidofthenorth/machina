//! Offline end-to-end tests of the `machina` binary itself: spawn the compiled executable and
//! assert the load-bearing invariants at the process boundary — byte-identical determinism across
//! runs, a schema-valid embedded `RunResult`, and internal accounting consistency of the emitted
//! JSON.
//!
//! No network, no keys: the demo is self-contained synthetic data by construction, and these tests
//! only run the compiled binary with no environment beyond what Cargo provides.

use research_core::Decimal;
use results::RunResult;
use std::process::{Command, Output};

/// Path to the compiled `machina` binary (provided by Cargo for integration tests).
const BIN: &str = env!("CARGO_BIN_EXE_machina");

/// Marker line the demo prints immediately before the embedded `RunResult` JSON.
const JSON_MARKER: &str = "--- RunResult (trend_alloc_v1), schema-valid ---";

fn run_machina(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("spawn machina binary")
}

/// Run `machina demo`, asserting a clean exit, and return its stdout as UTF-8.
fn demo_stdout() -> String {
    let out = run_machina(&["demo"]);
    assert!(out.status.success(), "demo exited non-zero: {}", out.status);
    assert!(
        out.stderr.is_empty(),
        "demo wrote to stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("demo output is UTF-8")
}

/// Slice the embedded `RunResult` JSON object out of the demo's stdout.
///
/// The JSON is the only brace-delimited block after the marker line (the trailing note paragraph
/// contains no braces), so first-`{` .. last-`}` is exact. A parse failure in the callers would
/// flag any drift in this framing.
fn extract_run_result_json(stdout: &str) -> &str {
    let after =
        &stdout[stdout.find(JSON_MARKER).expect("JSON marker present") + JSON_MARKER.len()..];
    let start = after.find('{').expect("JSON object follows the marker");
    let end = after.rfind('}').expect("JSON object closes");
    &after[start..=end]
}

#[test]
fn demo_stdout_is_byte_identical_across_runs() {
    // The whole pipeline — allowlist load, synthetic data, simulator, metrics, serialization —
    // must be a pure function: two independent processes produce byte-for-byte identical output.
    // This would catch wall-clock stamps, hash-order iteration, or locale leaking into the demo.
    let a = run_machina(&["demo"]);
    let b = run_machina(&["demo"]);
    assert!(a.status.success(), "first run failed: {}", a.status);
    assert!(b.status.success(), "second run failed: {}", b.status);
    assert!(!a.stdout.is_empty(), "demo printed nothing");
    assert_eq!(a.stdout, b.stdout, "demo stdout differs between runs");
    assert!(a.stderr.is_empty() && b.stderr.is_empty());
}

#[test]
fn demo_embeds_a_schema_valid_run_result() {
    // The printed JSON must honor the checked-in run-result contract — the same gate the results
    // crate enforces on built values, here asserted on the binary's actual output text.
    let stdout = demo_stdout();
    let value: serde_json::Value =
        serde_json::from_str(extract_run_result_json(&stdout)).expect("embedded JSON parses");
    let schema_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/run-result.schema.json"
    );
    let schema_text = std::fs::read_to_string(schema_path).expect("read run-result schema");
    let schema: serde_json::Value = serde_json::from_str(&schema_text).expect("parse schema");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "demo RunResult failed schema validation: {errors:?}"
    );
}

#[test]
fn demo_run_result_is_internally_consistent() {
    let stdout = demo_stdout();
    let rr: RunResult =
        serde_json::from_str(extract_run_result_json(&stdout)).expect("parses as RunResult");

    // Identity stamps: research mode only, current schema, the selected strategy.
    assert_eq!(rr.schema_version, results::SCHEMA_VERSION);
    assert_eq!(rr.mode, "research");
    assert_eq!(rr.config.strategy, "trend_alloc_v1");
    // The human-readable header and the embedded JSON must agree on the allowlist version.
    assert!(
        stdout.contains(&format!("allowlist {} |", rr.allowlist_version)),
        "header allowlist version disagrees with RunResult"
    );

    // include_series wiring: one equity point per synthetic daily bar (24), and created_at is
    // derived from the data — it equals the last bar's timestamp, never the wall clock.
    assert_eq!(rr.equity_curve.len(), 24);
    let last_point = rr.equity_curve.last().expect("non-empty curve");
    assert_eq!(last_point.timestamp, rr.created_at);

    // Accounting/equity consistency, recomputed from the JSON's own decimal strings:
    // the curve starts at initial cash and its final mark equals the reported final balances.
    let initial: Decimal = rr.config.initial_cash_usdc.parse().expect("initial cash");
    let first: Decimal = rr.equity_curve[0].equity_quote.parse().expect("first mark");
    let last: Decimal = last_point.equity_quote.parse().expect("last mark");
    let final_equity: Decimal = rr
        .final_balances
        .equity_quote
        .parse()
        .expect("final equity");
    assert_eq!(first, initial, "curve must start at initial cash");
    assert_eq!(last, final_equity, "final mark must equal final balances");

    // total_return is defined as exactly last/first − 1 in Decimal (metrics::total_return);
    // recomputing the same expression from the serialized values must reproduce it exactly.
    let total: Decimal = rr.metrics.total_return.parse().expect("total_return");
    assert_eq!(total, last / first - Decimal::ONE);

    // Round trips are chronologically ordered and non-overlapping. RFC 3339 UTC strings of equal
    // length compare chronologically as plain strings.
    assert!(
        !rr.round_trips.is_empty(),
        "trend_alloc_v1 closes trips on this series"
    );
    for t in &rr.round_trips {
        assert!(t.entry_ts < t.exit_ts, "trip must open before it closes");
    }
    for pair in rr.round_trips.windows(2) {
        assert!(
            pair[0].exit_ts <= pair[1].entry_ts,
            "trips must not overlap"
        );
    }
}

#[test]
fn usage_and_unknown_command_paths() {
    // No-args and --help print the same deterministic usage text and exit cleanly.
    let no_args = run_machina(&[]);
    let help = run_machina(&["--help"]);
    assert!(no_args.status.success());
    assert!(help.status.success());
    assert_eq!(no_args.stdout, help.stdout);
    let text = String::from_utf8(help.stdout).expect("usage is UTF-8");
    assert!(text.contains("machina demo"));
    // The usage text carries the safety invariant: no key/RPC/submission path exists.
    assert!(text.contains("no key"));

    // Unknown commands are rejected with exit code 2 and a diagnostic on stderr.
    let bad = run_machina(&["frobnicate"]);
    assert_eq!(bad.status.code(), Some(2));
    let err = String::from_utf8(bad.stderr).expect("stderr is UTF-8");
    assert!(err.contains("unknown command: frobnicate"));
}
