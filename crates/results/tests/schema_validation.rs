//! M0 gate: the JSON schemas are valid, and our models / representative instances validate against
//! them. Proves `schemas/` are real contracts and that `RunResult` honors `run-result.schema.json`.

use metrics::Metrics;
use portfolio::{run, CostModel};
use research_core::{Bar, Decimal, Timestamp};
use results::{RunInputs, RunResult};
use rust_decimal_macros::dec;
use serde_json::{json, Value};

fn schema(name: &str) -> Value {
    let path = format!("{}/../../schemas/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

fn compile(name: &str) -> jsonschema::Validator {
    jsonschema::validator_for(&schema(name))
        .unwrap_or_else(|e| panic!("invalid schema {name}: {e}"))
}

const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const WSOL: &str = "So11111111111111111111111111111111111111112";

#[test]
fn all_schemas_are_valid_json_schema() {
    for name in [
        "run-result.schema.json",
        "route-quote.schema.json",
        "execution-event.schema.json",
        "wallet-snapshot.schema.json",
    ] {
        let _ = compile(name); // panics if the schema itself is malformed
    }
}

#[test]
fn built_run_result_validates_against_schema() {
    let bars = vec![
        bar(1_609_459_200, 100),
        bar(1_609_545_600, 110),
        bar(1_609_632_000, 105),
        bar(1_609_718_400, 120),
    ];
    let cost = CostModel {
        dex_fee_bps: 5,
        slippage_bps: 20,
        base_fee_lamports: 5000,
        priority_fee_lamports: 50000,
    };
    let out = run(&bars, dec!(10000), &cost, |_h, _w| dec!(0.5)).unwrap();
    let eq: Vec<Decimal> = out.equity_curve.iter().map(|p| p.equity_quote).collect();
    let m = Metrics::from_equity(&eq, 365.0);
    let inputs = RunInputs {
        run_id: "demo-static_50_50-0001".to_string(),
        created_at: bars.last().unwrap().ts.to_rfc3339(),
        mode: "research",
        allowlist_version: "2026-06-29".to_string(),
        strategy: "static_50_50".to_string(),
        base_symbol: "SOL".to_string(),
        quote_symbol: "USDC".to_string(),
        bar_interval: "1d".to_string(),
        initial_cash_usdc: dec!(10000),
        cost: &cost,
        final_price: bars.last().unwrap().close,
        include_series: true,
    };
    let rr = RunResult::build(&inputs, &out, &m);
    let value = rr.to_value();
    let validator = compile("run-result.schema.json");
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "RunResult failed schema validation: {errors:?}\n{}",
        rr.to_json()
    );
}

#[test]
fn run_result_rejects_money_as_number() {
    // Hand-build a minimally-valid instance, then violate the decimal-string contract.
    let mut value = minimal_run_result();
    assert!(compile("run-result.schema.json").is_valid(&value));
    value["config"]["initial_cash_usdc"] = json!(10000); // number, not a decimal string
    assert!(
        !compile("run-result.schema.json").is_valid(&value),
        "schema must reject a numeric money field"
    );
}

#[test]
fn run_result_rejects_unknown_mode() {
    let mut value = minimal_run_result();
    value["mode"] = json!("live"); // not in the enum
    assert!(!compile("run-result.schema.json").is_valid(&value));
}

#[test]
fn route_quote_instance_validates() {
    let v = json!({
        "schema_version": "1.0.0",
        "router": "jupiter-swap-v2",
        "input_mint": USDC,
        "input_amount": "1000.000000",
        "output_mint": WSOL,
        "expected_output_amount": "9.950000000",
        "min_received_amount": "9.900000000",
        "price_impact_pct": "0.05",
        "slippage_mode": "fixed",
        "slippage_bps": 20,
        "correlation_id": "req-abc-123",
        "observed_at": "2026-06-29T00:00:00Z",
        "final_status": "quoted",
        "tx_signature": null
    });
    assert_valid("route-quote.schema.json", &v);
}

#[test]
fn execution_event_instance_validates() {
    let v = json!({
        "schema_version": "1.0.0",
        "event_id": "evt-0001",
        "decision_ts": "2026-06-29T00:00:00Z",
        "mode": "shadow",
        "state": "Quoted",
        "strategy_version": "trend_alloc_v1",
        "wallet_address": null,
        "tx_signature": null,
        "confirmation_status": null
    });
    assert_valid("execution-event.schema.json", &v);
}

#[test]
fn execution_event_rejects_bad_state() {
    let v = json!({
        "schema_version": "1.0.0",
        "event_id": "evt-0001",
        "decision_ts": "2026-06-29T00:00:00Z",
        "mode": "shadow",
        "state": "YOLO",
        "strategy_version": "trend_alloc_v1"
    });
    assert!(!compile("execution-event.schema.json").is_valid(&v));
}

#[test]
fn wallet_snapshot_instance_validates() {
    let v = json!({
        "schema_version": "1.0.0",
        "mode": "shadow",
        "wallet_address": WSOL,
        "slot": 250_000_000_u64,
        "commitment": "confirmed",
        "provider_id": "dedicated-primary",
        "observed_at": "2026-06-29T00:00:00Z",
        "sol_balance": "1.234567890",
        "token_accounts": [
            {
                "mint": USDC,
                "symbol": "USDC",
                "decimals": 6,
                "amount_base_units": "1000000000",
                "amount_decimal": "1000.000000",
                "allowlisted": true
            }
        ]
    });
    assert_valid("wallet-snapshot.schema.json", &v);
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn bar(unix: i64, price: i64) -> Bar {
    let p = Decimal::from(price);
    Bar {
        ts: Timestamp::from_unix(unix),
        open: p,
        high: p,
        low: p,
        close: p,
        volume: dec!(1),
    }
}

fn assert_valid(schema_name: &str, value: &Value) {
    let validator = compile(schema_name);
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "{schema_name} validation failed: {errors:?}"
    );
}

fn minimal_run_result() -> Value {
    json!({
        "schema_version": "1.0.0",
        "run_id": "min-0001",
        "created_at": "2021-01-01T00:00:00Z",
        "mode": "research",
        "allowlist_version": "2026-06-29",
        "config": {
            "strategy": "hold_usdc",
            "base_symbol": "SOL",
            "quote_symbol": "USDC",
            "bar_interval": "1d",
            "initial_cash_usdc": "10000",
            "cost_model": {
                "dex_fee_bps": 5,
                "slippage_bps": 20,
                "base_fee_lamports": 5000,
                "priority_fee_lamports": 50000
            }
        },
        "metrics": {
            "total_return": "0",
            "max_drawdown": "0",
            "n_trades": 0
        },
        "final_balances": {
            "base": "0",
            "quote": "10000",
            "equity_quote": "10000"
        }
    })
}
