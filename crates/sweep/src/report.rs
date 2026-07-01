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
use serde::Serialize;

/// Version of the sweep-report schema this crate targets (matches `schema_version` in the JSON and
/// `sweep-report.schema.json`).
pub const SWEEP_SCHEMA_VERSION: &str = "1.0.0";

/// The fixed disclaimer embedded in every report (invariant 11).
const REPORT_NOTE: &str = "Robustness filter only — not a profitability verdict. 'advanceable' means a candidate survived the robustness battery (costs, doubled costs, walk-forward, baselines) and is eligible for M5 review; it is never evidence the strategy is profitable. In-sample results never establish an edge.";

/// The configured thresholds, serialized with decimal budgets as exact strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThresholdsDto {
    pub drawdown_budget: String,
    pub turnover_budget: String,
    pub baseline_margin: String,
    pub dispersion_budget: String,
    pub neighbor_tolerance: String,
    pub min_windows: u32,
}

impl ThresholdsDto {
    fn from_thresholds(th: &AdvancementThresholds) -> Self {
        Self {
            drawdown_budget: th.drawdown_budget.to_string(),
            turnover_budget: th.turnover_budget.to_string(),
            baseline_margin: th.baseline_margin.to_string(),
            dispersion_budget: th.dispersion_budget.to_string(),
            neighbor_tolerance: th.neighbor_tolerance.to_string(),
            min_windows: th.min_windows,
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
    pub verdicts: Vec<CandidateVerdict>,
}

impl SweepReport {
    /// Assemble a report. `verdicts` are sorted into a **total** canonical order (by
    /// `candidate_label` first, then status and reasons — see `CandidateVerdict`'s `Ord`), so the
    /// output is byte-identical regardless of the order candidates were evaluated, even if two
    /// verdicts happen to share a label.
    #[must_use]
    pub fn new(
        thresholds: &AdvancementThresholds,
        trial_count: u32,
        mut verdicts: Vec<CandidateVerdict>,
    ) -> Self {
        verdicts.sort();
        Self {
            schema_version: SWEEP_SCHEMA_VERSION.to_string(),
            note: REPORT_NOTE.to_string(),
            trial_count,
            thresholds: ThresholdsDto::from_thresholds(thresholds),
            verdicts,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advance::{evaluate_candidate, AdvancementThresholds, CandidateEvidence};
    use rust_decimal_macros::dec;

    fn thresholds() -> AdvancementThresholds {
        AdvancementThresholds {
            drawdown_budget: dec!(0.30),
            turnover_budget: dec!(5),
            baseline_margin: dec!(0.02),
            dispersion_budget: dec!(0.50),
            neighbor_tolerance: dec!(0.10),
            min_windows: 4,
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
        }
    }

    fn sample_report() -> SweepReport {
        let th = thresholds();
        // Deliberately out of label order, and one rejected candidate.
        let verdicts = vec![
            evaluate_candidate(&evidence("zeta/target=0.9;band=0", dec!(0.40)), &th),
            evaluate_candidate(&evidence("alpha/target=0.5;band=0", dec!(0.12)), &th),
        ];
        SweepReport::new(&th, 48, verdicts)
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
    fn duplicate_labels_still_serialize_deterministically() {
        // Two verdicts sharing a label but differing in content must produce byte-identical output
        // regardless of input order — the sort is total, not just keyed on the label.
        let th = thresholds();
        let advanceable = evaluate_candidate(&evidence("dup/target=0.5;band=0", dec!(0.12)), &th);
        let rejected = evaluate_candidate(&evidence("dup/target=0.5;band=0", dec!(0.55)), &th);
        let r1 = SweepReport::new(&th, 2, vec![advanceable.clone(), rejected.clone()]);
        let r2 = SweepReport::new(&th, 2, vec![rejected, advanceable]);
        assert_eq!(r1.to_json(), r2.to_json());
    }
}
