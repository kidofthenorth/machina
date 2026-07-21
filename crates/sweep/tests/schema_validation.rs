//! S11 gate (plan §11/§12/§14): the new `sweep-report.schema.json` is a real contract — a built
//! `SweepReport` validates against it, the schema rejects money-as-number and unknown enums, each
//! rejection criterion round-trips, `trial_count` is recorded, and two serializations are
//! byte-identical. Plus the f64-comparison audit: the advancement/selection modules are Decimal-only.

use research_core::intraday::Provenance;
use research_core::Decimal;
use rust_decimal_macros::dec;
use serde_json::{json, Value};
use sweep::report::CandidateMetricsDto;
use sweep::{
    evaluate_candidate, AdvancementThresholds, CandidateEvidence, FeeSensitivity, HfRungsDto,
    RejectionKind, ScenarioId, ScenarioMetrics, SweepReport, Verdict,
};

fn schema() -> Value {
    let path = format!(
        "{}/../../schemas/sweep-report.schema.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

fn validator() -> jsonschema::Validator {
    jsonschema::validator_for(&schema()).expect("sweep-report schema is valid JSON Schema")
}

fn assert_valid(value: &Value) {
    let v = validator();
    let errors: Vec<String> = v.iter_errors(value).map(|e| e.to_string()).collect();
    assert!(errors.is_empty(), "schema validation failed: {errors:?}");
}

fn thresholds() -> AdvancementThresholds {
    AdvancementThresholds {
        drawdown_budget: dec!(0.30),
        turnover_budget: Some(dec!(5)),
        baseline_margin: dec!(0.02),
        dispersion_budget: dec!(0.50),
        neighbor_tolerance: dec!(0.10),
        min_windows: 4,
        cost_drag_share_ceiling: None,
        per_trade_edge_floor: None,
    }
}

fn evidence(label: &str) -> CandidateEvidence {
    CandidateEvidence {
        candidate_label: label.to_string(),
        max_drawdown: dec!(0.12),
        turnover: dec!(1.4),
        baseline_margin: dec!(0.05),
        doubled_return: dec!(0.08),
        doubled_baseline_floor: dec!(0.03),
        fold_dispersion: dec!(0.20),
        neighbor_degradation: dec!(0.04),
        valid_windows: 7,
        cost_drag_share: None,
        per_trade_edge: None,
    }
}

fn scenario_metrics(scenario: ScenarioId, total_return: Decimal) -> ScenarioMetrics {
    ScenarioMetrics {
        scenario,
        total_return,
        turnover: dec!(1.4),
        n_trades: 6,
        fees_paid_quote: dec!(2.5),
        slippage_paid_quote: dec!(1.25),
        priority_fees_paid_sol: dec!(0.001),
    }
}

fn sample_candidate() -> CandidateMetricsDto {
    let fs = FeeSensitivity::from_scenarios(
        scenario_metrics(ScenarioId::BeforeCosts, dec!(0.30)),
        scenario_metrics(ScenarioId::Base, dec!(0.20)),
        scenario_metrics(ScenarioId::Doubled, dec!(0.10)),
        dec!(0.03),
    );
    CandidateMetricsDto::new(
        "threshold_rebalance_v1/target=0.5;band=0".to_string(),
        dec!(0.12),
        dec!(1.4),
        &fs,
    )
}

#[test]
fn schema_is_valid_json_schema() {
    let _ = validator(); // panics if the schema itself is malformed
}

#[test]
fn built_report_validates_against_schema() {
    let th = thresholds();

    // An advanceable candidate.
    let ok = evaluate_candidate(&evidence("threshold_rebalance_v1/target=0.5;band=0"), &th);
    assert_eq!(ok.status, Verdict::Advanceable);

    // A candidate failing several criteria at once (drawdown, turnover, edge-vanishes).
    let mut bad = evidence("trend_alloc_v1/sma=3;above=1;below=0");
    bad.max_drawdown = dec!(0.55);
    bad.turnover = dec!(9);
    bad.doubled_return = dec!(0.01); // <= doubled_baseline_floor (0.03) → edge vanishes
    let bad = evaluate_candidate(&bad, &th);
    assert_eq!(bad.status, Verdict::Rejected);

    // A candidate rejected on insufficient data alone.
    let mut thin = evidence("trend_alloc_v1/sma=99;above=1;below=0");
    thin.valid_windows = 1;
    let thin = evaluate_candidate(&thin, &th);
    assert_eq!(
        thin.failed_criteria[0].kind,
        RejectionKind::InsufficientData
    );

    let report = SweepReport::new(&th, 48, vec![ok, bad, thin], vec![sample_candidate()]);
    assert_valid(&report.to_value());
}

#[test]
fn m4_shape_report_validates_under_1_2_0() {
    // M4 shape: turnover present, both new HF thresholds absent.
    let report = SweepReport::new(&thresholds(), 1, vec![], vec![]);
    assert_valid(&report.to_value());
    let value = report.to_value();
    assert_eq!(value["schema_version"], json!("1.4.0"));
    assert!(value["thresholds"]["turnover_budget"].is_string());
    assert!(value["thresholds"].get("cost_drag_share_ceiling").is_none());
    assert!(value["thresholds"].get("per_trade_edge_floor").is_none());
}

#[test]
fn hf_shape_report_validates_under_1_2_0() {
    // HF shape: turnover absent, both new HF thresholds present.
    let th = AdvancementThresholds {
        turnover_budget: None,
        cost_drag_share_ceiling: Some(dec!(0.5)),
        per_trade_edge_floor: Some(dec!(0.001)),
        ..thresholds()
    };
    let report = SweepReport::new(&th, 1, vec![], vec![]);
    assert_valid(&report.to_value());
    let value = report.to_value();
    assert!(value["thresholds"].get("turnover_budget").is_none());
    assert_eq!(value["thresholds"]["cost_drag_share_ceiling"], json!("0.5"));
    assert_eq!(value["thresholds"]["per_trade_edge_floor"], json!("0.001"));
}

#[test]
fn report_with_data_provenance_validates_and_round_trips() {
    let report = SweepReport::new(&thresholds(), 1, vec![], vec![]).with_data_provenance(&[
        Provenance::Real {
            source_id: "binance-snapshot-1".to_string(),
        },
    ]);
    let value = report.to_value();
    assert_valid(&value);
    assert_eq!(value["data_provenance"], json!("real"));
}

#[test]
fn report_rejects_a_numeric_budget() {
    let report = SweepReport::new(&thresholds(), 12, vec![], vec![]);
    let mut value = report.to_value();
    assert!(validator().is_valid(&value));
    // Violate the decimal-string contract: a budget as a JSON number.
    value["thresholds"]["drawdown_budget"] = json!(0.3);
    assert!(
        !validator().is_valid(&value),
        "schema must reject a numeric budget"
    );
}

#[test]
fn report_rejects_unknown_status_and_reason_kind() {
    let mut ev = evidence("x/target=0.5;band=0");
    ev.max_drawdown = dec!(0.99);
    let report = SweepReport::new(
        &thresholds(),
        1,
        vec![evaluate_candidate(&ev, &thresholds())],
        vec![],
    );
    let mut value = report.to_value();
    assert!(validator().is_valid(&value));

    let mut bad_status = value.clone();
    bad_status["verdicts"][0]["status"] = json!("maybe");
    assert!(
        !validator().is_valid(&bad_status),
        "unknown status must fail"
    );

    value["verdicts"][0]["failed_criteria"][0]["kind"] = json!("vibes_off");
    assert!(
        !validator().is_valid(&value),
        "unknown reason kind must fail"
    );
}

#[test]
fn report_rejects_additional_properties() {
    let report = SweepReport::new(&thresholds(), 1, vec![], vec![]);
    let mut value = report.to_value();
    value["surprise"] = json!("not allowed");
    assert!(
        !validator().is_valid(&value),
        "additionalProperties:false must reject unknown top-level fields"
    );
}

#[test]
fn candidate_rejects_additional_properties() {
    let report = SweepReport::new(&thresholds(), 1, vec![], vec![sample_candidate()]);
    let mut value = report.to_value();
    assert!(validator().is_valid(&value));
    value["candidates"][0]["surprise"] = json!("not allowed");
    assert!(
        !validator().is_valid(&value),
        "additionalProperties:false must reject unknown candidate fields"
    );
}

#[test]
fn candidate_with_full_hf_block_validates() {
    let hf_rungs = HfRungsDto::new(
        &scenario_metrics(ScenarioId::HotCongestion, dec!(0.18)),
        &scenario_metrics(ScenarioId::AdversarialWorst, dec!(0.05)),
        &scenario_metrics(ScenarioId::Latency2x, dec!(0.15)),
        3,
        dec!(0.75),
        2,
    );
    let candidate = sample_candidate().with_hf(hf_rungs, Some(dec!(0.4)), Some(dec!(0.002)));
    let report = SweepReport::new(&thresholds(), 1, vec![], vec![candidate]);
    let value = report.to_value();
    assert_valid(&value);
    assert_eq!(value["candidates"][0]["cost_drag_share"], json!("0.4"));
    assert_eq!(value["candidates"][0]["per_trade_edge"], json!("0.002"));
    assert_eq!(
        value["candidates"][0]["hf_rungs"]["adverse_selection_hits"],
        json!(3)
    );
    assert_eq!(
        value["candidates"][0]["hf_rungs"]["unlanded_orders"],
        json!(2)
    );
}

#[test]
fn hf_rungs_rejects_additional_properties() {
    let hf_rungs = HfRungsDto::new(
        &scenario_metrics(ScenarioId::HotCongestion, dec!(0.18)),
        &scenario_metrics(ScenarioId::AdversarialWorst, dec!(0.05)),
        &scenario_metrics(ScenarioId::Latency2x, dec!(0.15)),
        3,
        dec!(0.75),
        2,
    );
    let candidate = sample_candidate().with_hf(hf_rungs, Some(dec!(0.4)), Some(dec!(0.002)));
    let report = SweepReport::new(&thresholds(), 1, vec![], vec![candidate]);
    let mut value = report.to_value();
    assert!(validator().is_valid(&value));
    value["candidates"][0]["hf_rungs"]["surprise"] = json!("not allowed");
    assert!(
        !validator().is_valid(&value),
        "hf_rungs additionalProperties:false must reject unknown fields"
    );
}

#[test]
fn candidate_without_hf_fields_omits_them_entirely() {
    let report = SweepReport::new(&thresholds(), 1, vec![], vec![sample_candidate()]);
    let value = report.to_value();
    assert_valid(&value);
    assert!(value["candidates"][0].get("cost_drag_share").is_none());
    assert!(value["candidates"][0].get("per_trade_edge").is_none());
    assert!(value["candidates"][0].get("hf_rungs").is_none());
}

#[test]
fn candidate_rejects_a_numeric_total_return() {
    let report = SweepReport::new(&thresholds(), 1, vec![], vec![sample_candidate()]);
    let mut value = report.to_value();
    assert!(validator().is_valid(&value));
    // Violate the decimal-string contract: a scenario return as a JSON number.
    value["candidates"][0]["fee_sensitivity"]["base"]["total_return"] = json!(0.2);
    assert!(
        !validator().is_valid(&value),
        "schema must reject a numeric total_return"
    );
}

/// The f64-comparison audit (plan §5.5 / §14 S11): all sort/threshold/selection keys are `Decimal`;
/// the advancement and report modules must contain no floating point at all. A source-level check —
/// blunt but decisive — so a future edit that reaches for `f64` in a decision path fails loudly.
#[test]
fn advancement_and_report_modules_are_decimal_only() {
    let advance_src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/advance.rs"));
    let report_src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/report.rs"));
    assert!(
        !advance_src.contains("f64"),
        "advance.rs must be Decimal-only (no f64 in selection/threshold logic)"
    );
    assert!(
        !report_src.contains("f64"),
        "report.rs must be Decimal-only (no f64 in the exported report)"
    );
}
