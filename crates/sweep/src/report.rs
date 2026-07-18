//! Canonical sweep report export (plan §12, §14 S11).
//!
//! [`SweepReport`] is the top-level, schema-validated artifact of a sweep: the configured robustness
//! thresholds, the total trial count (for M5 multiple-testing accounting), and one [`CandidateVerdict`]
//! per candidate, sorted canonically by label. It serializes to JSON that validates against
//! `schemas/sweep-report.schema.json` — money/threshold values as **exact decimal strings** (never
//! JSON floats), collections pre-sorted, so the output is deterministic and byte-identical on repeat.
//!
//! The report is a **research filter, not a profitability verdict** (invariant 11): a fixed `note`
//! states this in the JSON itself.

use crate::advance::{AdvancementThresholds, CandidateVerdict};
use crate::sensitivity::{FeeSensitivity, ScenarioMetrics};
use research_core::Decimal;
use serde::Serialize;

/// Version of the sweep-report schema this crate targets (matches `schema_version` in the JSON and
/// `sweep-report.schema.json`).
pub const SWEEP_SCHEMA_VERSION: &str = "1.3.0";

/// The fixed disclaimer embedded in every report (invariant 11).
const REPORT_NOTE: &str = "Robustness filter only — not a profitability verdict. 'advanceable' means a candidate survived the robustness battery (costs, doubled costs, walk-forward, baselines) and is eligible for M5 review; it is never evidence the strategy is profitable. In-sample results never establish an edge.";

/// The configured thresholds, serialized with decimal budgets as exact strings. The three HF-only
/// fields are omitted from the JSON entirely when absent, so an M4-shape report (turnover present,
/// the two new thresholds absent) serializes identically to its pre-C6 form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThresholdsDto {
    pub drawdown_budget: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turnover_budget: Option<String>,
    pub baseline_margin: String,
    pub dispersion_budget: String,
    pub neighbor_tolerance: String,
    pub min_windows: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_drag_share_ceiling: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_trade_edge_floor: Option<String>,
}

impl ThresholdsDto {
    fn from_thresholds(th: &AdvancementThresholds) -> Self {
        Self {
            drawdown_budget: th.drawdown_budget.to_string(),
            turnover_budget: th.turnover_budget.map(|d| d.to_string()),
            baseline_margin: th.baseline_margin.to_string(),
            dispersion_budget: th.dispersion_budget.to_string(),
            neighbor_tolerance: th.neighbor_tolerance.to_string(),
            min_windows: th.min_windows,
            cost_drag_share_ceiling: th.cost_drag_share_ceiling.map(|d| d.to_string()),
            per_trade_edge_floor: th.per_trade_edge_floor.map(|d| d.to_string()),
        }
    }
}

/// One scenario's reportable metrics, decimal-string encoded. The parent key
/// (before_costs/base/doubled) is the scenario discriminator — no separate field carried.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ScenarioMetricsDto {
    pub total_return: String,
    pub turnover: String,
    pub n_trades: u32,
    pub fees_paid_quote: String,
    pub slippage_paid_quote: String,
    pub priority_fees_paid_sol: String,
}

impl ScenarioMetricsDto {
    /// Scale-canonical strings (`.normalize()`, matching `param_id`'s convention) so exact zeros
    /// export as `"0"`, not `"0.0000000000000000000000000"`.
    fn from_metrics(m: &ScenarioMetrics) -> Self {
        Self {
            total_return: m.total_return.normalize().to_string(),
            turnover: m.turnover.normalize().to_string(),
            n_trades: m.n_trades,
            fees_paid_quote: m.fees_paid_quote.normalize().to_string(),
            slippage_paid_quote: m.slippage_paid_quote.normalize().to_string(),
            priority_fees_paid_sol: m.priority_fees_paid_sol.normalize().to_string(),
        }
    }
}

/// A candidate's fee-sensitivity block (m4-sweep.md §9), decimal-string encoded. Robustness
/// reporting only — never a profitability claim (invariant 11).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FeeSensitivityDto {
    pub before_costs: ScenarioMetricsDto,
    pub base: ScenarioMetricsDto,
    pub doubled: ScenarioMetricsDto,
    pub return_drag_costs: String,
    pub return_drag_doubled: String,
    pub survives_doubled: bool,
}

impl FeeSensitivityDto {
    fn from_sensitivity(fs: &FeeSensitivity) -> Self {
        Self {
            before_costs: ScenarioMetricsDto::from_metrics(&fs.before_costs),
            base: ScenarioMetricsDto::from_metrics(&fs.base),
            doubled: ScenarioMetricsDto::from_metrics(&fs.doubled),
            return_drag_costs: fs.return_drag_costs.normalize().to_string(),
            return_drag_doubled: fs.return_drag_doubled.normalize().to_string(),
            survives_doubled: fs.survives_doubled,
        }
    }
}

/// One candidate's first-class reported metrics (M4 deliverable: turnover and fee-sensitivity
/// reporting). `Ord` keys on `candidate_label` first — total canonical order, like verdicts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct CandidateMetricsDto {
    pub candidate_label: String,
    pub max_drawdown: String,
    pub turnover: String,
    pub fee_sensitivity: FeeSensitivityDto,
}

impl CandidateMetricsDto {
    /// Stringify one candidate's worst-window drawdown/turnover + fee-sensitivity block.
    #[must_use]
    pub fn new(
        candidate_label: String,
        max_drawdown: Decimal,
        turnover: Decimal,
        fee_sensitivity: &FeeSensitivity,
    ) -> Self {
        Self {
            candidate_label,
            max_drawdown: max_drawdown.normalize().to_string(),
            turnover: turnover.normalize().to_string(),
            fee_sensitivity: FeeSensitivityDto::from_sensitivity(fee_sensitivity),
        }
    }
}

/// The canonical, schema-validated sweep report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SweepReport {
    pub schema_version: String,
    pub note: String,
    /// Total cells evaluated across the sweep (for M5 multiple-testing accounting).
    pub trial_count: u32,
    pub thresholds: ThresholdsDto,
    pub candidates: Vec<CandidateMetricsDto>,
    pub verdicts: Vec<CandidateVerdict>,
    /// Computed provenance rollup over the input data actually fed to the sweep — `"synthetic"`,
    /// `"real"`, or `"mixed"` (see [`SweepReport::with_data_provenance`]). Absent (not just null)
    /// when never set, so pre-C8.2 reports serialize identically.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_provenance: Option<String>,
}

impl SweepReport {
    /// Assemble a report. `verdicts` and `candidates` are each sorted into a **total** canonical
    /// order (by `candidate_label` first, then the remaining fields — see their `Ord` derives), so
    /// the output is byte-identical regardless of the order candidates were evaluated, even if two
    /// entries happen to share a label.
    #[must_use]
    pub fn new(
        thresholds: &AdvancementThresholds,
        trial_count: u32,
        mut verdicts: Vec<CandidateVerdict>,
        mut candidates: Vec<CandidateMetricsDto>,
    ) -> Self {
        verdicts.sort();
        candidates.sort();
        Self {
            schema_version: SWEEP_SCHEMA_VERSION.to_string(),
            note: REPORT_NOTE.to_string(),
            trial_count,
            thresholds: ThresholdsDto::from_thresholds(thresholds),
            candidates,
            verdicts,
            data_provenance: None,
        }
    }

    /// Serialize to a pretty JSON string. Deterministic and byte-identical on repeat.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("SweepReport serializes")
    }

    /// Serialize to a [`serde_json::Value`] (for schema validation).
    #[must_use]
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("SweepReport serializes")
    }

    /// Rebuild the note with a provenance suffix appended (e.g. which real-data snapshot fed the
    /// sweep). Every other field is unchanged; additive only — `new`'s existing callers and output
    /// are unaffected.
    #[must_use]
    pub fn with_provenance(mut self, provenance: &str) -> Self {
        self.note = format!("{} | data: {provenance}", self.note);
        self
    }

    /// Compute the typed `data_provenance` rollup over the input data actually fed to the sweep
    /// (m-hf-track §2: "a typed field, not prose"). `"synthetic"` if every item is
    /// [`research_core::intraday::Provenance::Synthetic`], `"real"` if every item is `Real`,
    /// `"mixed"` otherwise. An empty slice is defined as `"synthetic"` (vacuously — no real data
    /// is present). Pure, no I/O; does not touch `note` (coexists with [`Self::with_provenance`]).
    #[must_use]
    pub fn with_data_provenance(mut self, items: &[research_core::intraday::Provenance]) -> Self {
        let all_synthetic = items
            .iter()
            .all(|p| matches!(p, research_core::intraday::Provenance::Synthetic { .. }));
        let all_real = items
            .iter()
            .all(|p| matches!(p, research_core::intraday::Provenance::Real { .. }));
        let rollup = if all_synthetic {
            "synthetic"
        } else if all_real {
            "real"
        } else {
            "mixed"
        };
        self.data_provenance = Some(rollup.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advance::{evaluate_candidate, AdvancementThresholds, CandidateEvidence};
    use rust_decimal_macros::dec;

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

    fn evidence(label: &str, drawdown: rust_decimal::Decimal) -> CandidateEvidence {
        CandidateEvidence {
            candidate_label: label.to_string(),
            max_drawdown: drawdown,
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

    fn sample_report() -> SweepReport {
        let th = thresholds();
        // Deliberately out of label order, and one rejected candidate.
        let verdicts = vec![
            evaluate_candidate(&evidence("zeta/target=0.9;band=0", dec!(0.40)), &th),
            evaluate_candidate(&evidence("alpha/target=0.5;band=0", dec!(0.12)), &th),
        ];
        SweepReport::new(&th, 48, verdicts, vec![])
    }

    #[test]
    fn report_is_byte_identical_on_repeat_serialization() {
        let r = sample_report();
        assert_eq!(r.to_json(), r.to_json());
        // Rebuilding from the same inputs yields the same report and the same bytes.
        assert_eq!(sample_report().to_json(), r.to_json());
    }

    #[test]
    fn verdicts_are_sorted_by_label() {
        let r = sample_report();
        let labels: Vec<_> = r
            .verdicts
            .iter()
            .map(|v| v.candidate_label.as_str())
            .collect();
        assert_eq!(
            labels,
            vec!["alpha/target=0.5;band=0", "zeta/target=0.9;band=0"]
        );
    }

    #[test]
    fn budgets_serialize_as_decimal_strings_and_trial_count_is_recorded() {
        let value = sample_report().to_value();
        assert_eq!(
            value["thresholds"]["drawdown_budget"],
            serde_json::json!("0.30")
        );
        assert!(value["thresholds"]["drawdown_budget"].is_string());
        assert_eq!(value["trial_count"], serde_json::json!(48));
        assert_eq!(
            value["schema_version"],
            serde_json::json!(SWEEP_SCHEMA_VERSION)
        );
    }

    #[test]
    fn note_disclaims_profitability() {
        let r = sample_report();
        assert!(r.note.contains("not a profitability verdict"));
        assert!(r.note.to_lowercase().contains("robustness"));
    }

    #[test]
    fn with_provenance_keeps_disclaimer_and_appends_suffix() {
        let r = sample_report().with_provenance("binance snapshot, fnv1a64 0xabc123");
        assert!(r.note.contains("not a profitability verdict"));
        assert!(r.note.to_lowercase().contains("robustness"));
        assert!(r
            .note
            .ends_with("| data: binance snapshot, fnv1a64 0xabc123"));
    }

    #[test]
    fn duplicate_labels_still_serialize_deterministically() {
        // Two verdicts sharing a label but differing in content must produce byte-identical output
        // regardless of input order — the sort is total, not just keyed on the label.
        let th = thresholds();
        let advanceable = evaluate_candidate(&evidence("dup/target=0.5;band=0", dec!(0.12)), &th);
        let rejected = evaluate_candidate(&evidence("dup/target=0.5;band=0", dec!(0.55)), &th);
        let r1 = SweepReport::new(&th, 2, vec![advanceable.clone(), rejected.clone()], vec![]);
        let r2 = SweepReport::new(&th, 2, vec![rejected, advanceable], vec![]);
        assert_eq!(r1.to_json(), r2.to_json());
    }

    #[test]
    fn data_provenance_all_synthetic_rolls_up_to_synthetic() {
        let items = vec![
            research_core::intraday::Provenance::Synthetic { spec_hash: 1 },
            research_core::intraday::Provenance::Synthetic { spec_hash: 2 },
        ];
        let r = sample_report().with_data_provenance(&items);
        assert_eq!(r.data_provenance.as_deref(), Some("synthetic"));
    }

    #[test]
    fn data_provenance_all_real_rolls_up_to_real() {
        let items = vec![
            research_core::intraday::Provenance::Real {
                source_id: "binance-snapshot-1".to_string(),
            },
            research_core::intraday::Provenance::Real {
                source_id: "binance-snapshot-2".to_string(),
            },
        ];
        let r = sample_report().with_data_provenance(&items);
        assert_eq!(r.data_provenance.as_deref(), Some("real"));
    }

    #[test]
    fn data_provenance_mixed_rolls_up_to_mixed() {
        let items = vec![
            research_core::intraday::Provenance::Synthetic { spec_hash: 1 },
            research_core::intraday::Provenance::Real {
                source_id: "binance-snapshot-1".to_string(),
            },
        ];
        let r = sample_report().with_data_provenance(&items);
        assert_eq!(r.data_provenance.as_deref(), Some("mixed"));
    }

    #[test]
    fn data_provenance_empty_slice_rolls_up_to_synthetic() {
        let r = sample_report().with_data_provenance(&[]);
        assert_eq!(r.data_provenance.as_deref(), Some("synthetic"));
    }

    #[test]
    fn report_without_data_provenance_omits_the_field_entirely() {
        let value = sample_report().to_value();
        assert!(
            value.get("data_provenance").is_none(),
            "data_provenance must be entirely absent (not null) when never set"
        );
    }
}
