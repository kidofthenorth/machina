//! Advancement / rejection verdicts (plan §11, §14 S11) — the robustness battery, framed as a
//! **filter**, never a profitability claim (invariant 11).
//!
//! A candidate becomes `Advanceable` only by surviving every robustness criterion; any failure
//! `Rejects` it and is recorded as a [`RejectionReason`] carrying the observed value versus the
//! configured threshold (both as exact decimal strings). `Advanceable` means "passed the battery,
//! eligible for M5 review" — it is **not** evidence the strategy works.
//!
//! Every threshold and comparison here is an exact `rust_decimal::Decimal` — there is no floating
//! point in this module (enforced by a source-level audit test), so selection is deterministic and
//! order-stable regardless of platform.

use research_core::Decimal;
use serde::Serialize;

/// The configured robustness budgets a candidate must satisfy. These are **research policy**
/// (illustrative in M4, frozen before any M5 decision — plan §15); the engine only applies them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancementThresholds {
    /// Maximum acceptable drawdown (a `[0,1]` fraction).
    pub drawdown_budget: Decimal,
    /// Maximum acceptable un-annualized turnover ratio (traded notional / mean equity).
    pub turnover_budget: Decimal,
    /// Minimum out-of-sample margin a candidate must beat the best cost-matched baseline by.
    pub baseline_margin: Decimal,
    /// Maximum acceptable walk-forward fold dispersion (spread of out-of-sample fold returns).
    pub dispersion_budget: Decimal,
    /// Maximum acceptable degradation at neighboring grid points (parameter fragility).
    pub neighbor_tolerance: Decimal,
    /// Minimum number of valid walk-forward windows required to trust the evidence.
    pub min_windows: u32,
}

/// The measured evidence for one candidate, gathered from the sweep + sensitivity + baselines. All
/// decision-driving fields are exact `Decimal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEvidence {
    /// Stable candidate identifier (e.g. `"threshold_rebalance_v1/target=0.5;band=0"`).
    pub candidate_label: String,
    /// Worst drawdown over the evaluation (a `[0,1]` fraction).
    pub max_drawdown: Decimal,
    /// Un-annualized turnover ratio.
    pub turnover: Decimal,
    /// Out-of-sample margin over the best cost-matched baseline, at base costs
    /// (`candidate_return - best_baseline_return`).
    pub baseline_margin: Decimal,
    /// The candidate's return under **doubled** costs (from `sensitivity::FeeSensitivity`).
    pub doubled_return: Decimal,
    /// The cost-matched baseline floor to beat under doubled costs (S10's `survives_floor`). The
    /// edge is judged to vanish when `doubled_return <= doubled_baseline_floor` — the **single source
    /// of truth** for survival, matching `sensitivity`'s strict `>` rule (no separate bool that could
    /// drift from the recorded observed/threshold pair).
    pub doubled_baseline_floor: Decimal,
    /// Spread of the out-of-sample fold returns (max − min across walk-forward folds).
    pub fold_dispersion: Decimal,
    /// Worst return degradation at a neighboring grid point.
    pub neighbor_degradation: Decimal,
    /// Number of valid walk-forward windows the evidence rests on.
    pub valid_windows: u32,
}

/// The outcome of the robustness battery for one candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Passed every robustness criterion — eligible for M5 review. **Not** a profitability claim.
    Advanceable,
    /// Failed at least one robustness criterion (see the attached reasons).
    Rejected,
}

/// Which robustness criterion a candidate failed (one variant per plan §11 / §14 criterion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionKind {
    /// Did not beat the cost-matched baselines by the required out-of-sample margin.
    FailsBaselineComparison,
    /// The edge vanishes once costs are doubled (`survives_doubled == false`).
    EdgeVanishesUnderDoubledCosts,
    /// Depends on one period — walk-forward fold dispersion exceeds budget.
    DependsOnOnePeriod,
    /// Fragile to its parameters — a neighboring grid point degrades beyond tolerance.
    ParameterFragile,
    /// Drawdown exceeds the configured budget.
    DrawdownExceedsBudget,
    /// Turnover is economically implausible (exceeds the configured budget).
    TurnoverImplausible,
    /// Insufficient valid data (too few walk-forward windows / a partition failed validation).
    InsufficientData,
}

/// A single failed criterion, recording the observed value against its threshold as exact decimal
/// strings (never JSON floats), so the verdict is auditable and precision-preserving.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RejectionReason {
    pub kind: RejectionKind,
    pub observed: String,
    pub threshold: String,
}

impl RejectionReason {
    fn new(kind: RejectionKind, observed: Decimal, threshold: Decimal) -> Self {
        Self {
            kind,
            observed: observed.to_string(),
            threshold: threshold.to_string(),
        }
    }
}

/// One candidate's verdict: its label, status, and the criteria it failed (empty iff `Advanceable`).
///
/// `Ord` compares by `candidate_label` first (then status, then reasons), giving reports a **total**
/// canonical order so serialization is byte-identical even if two verdicts share a label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct CandidateVerdict {
    pub candidate_label: String,
    pub status: Verdict,
    pub failed_criteria: Vec<RejectionReason>,
}

/// Apply the robustness battery to one candidate's evidence, producing its verdict.
///
/// Insufficient data is a hard stop evaluated first: if too few valid windows exist, the other
/// metrics are not trustworthy, so the candidate is rejected on that alone. Otherwise every remaining
/// criterion is checked in a fixed, canonical order and all failures are recorded.
#[must_use]
pub fn evaluate_candidate(ev: &CandidateEvidence, th: &AdvancementThresholds) -> CandidateVerdict {
    // Data gate first — untrustworthy evidence must not "pass" the rest.
    if ev.valid_windows < th.min_windows {
        return CandidateVerdict {
            candidate_label: ev.candidate_label.clone(),
            status: Verdict::Rejected,
            failed_criteria: vec![RejectionReason {
                kind: RejectionKind::InsufficientData,
                observed: ev.valid_windows.to_string(),
                threshold: th.min_windows.to_string(),
            }],
        };
    }

    let mut failed = Vec::new();
    // Fixed, canonical order (independent of input) so the report is deterministic.
    if ev.baseline_margin < th.baseline_margin {
        failed.push(RejectionReason::new(
            RejectionKind::FailsBaselineComparison,
            ev.baseline_margin,
            th.baseline_margin,
        ));
    }
    // Single source of truth: the edge survives iff `doubled_return` strictly beats the floor
    // (matching S10's `doubled.total_return > survives_floor`), so the recorded observed/threshold
    // pair always provably justifies this reason — no separate boolean to drift.
    if ev.doubled_return <= ev.doubled_baseline_floor {
        failed.push(RejectionReason::new(
            RejectionKind::EdgeVanishesUnderDoubledCosts,
            ev.doubled_return,
            ev.doubled_baseline_floor,
        ));
    }
    if ev.fold_dispersion > th.dispersion_budget {
        failed.push(RejectionReason::new(
            RejectionKind::DependsOnOnePeriod,
            ev.fold_dispersion,
            th.dispersion_budget,
        ));
    }
    if ev.neighbor_degradation > th.neighbor_tolerance {
        failed.push(RejectionReason::new(
            RejectionKind::ParameterFragile,
            ev.neighbor_degradation,
            th.neighbor_tolerance,
        ));
    }
    if ev.max_drawdown > th.drawdown_budget {
        failed.push(RejectionReason::new(
            RejectionKind::DrawdownExceedsBudget,
            ev.max_drawdown,
            th.drawdown_budget,
        ));
    }
    if ev.turnover > th.turnover_budget {
        failed.push(RejectionReason::new(
            RejectionKind::TurnoverImplausible,
            ev.turnover,
            th.turnover_budget,
        ));
    }

    let status = if failed.is_empty() {
        Verdict::Advanceable
    } else {
        Verdict::Rejected
    };
    CandidateVerdict {
        candidate_label: ev.candidate_label.clone(),
        status,
        failed_criteria: failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Thresholds an all-passing candidate clears.
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

    /// Evidence that passes every criterion.
    fn passing() -> CandidateEvidence {
        CandidateEvidence {
            candidate_label: "threshold_rebalance_v1/target=0.5;band=0".to_string(),
            max_drawdown: dec!(0.12),
            turnover: dec!(1.4),
            baseline_margin: dec!(0.05),
            doubled_return: dec!(0.08),
            doubled_baseline_floor: dec!(0.03),
            fold_dispersion: dec!(0.20),
            neighbor_degradation: dec!(0.04),
            valid_windows: 7,
        }
    }

    #[test]
    fn a_fully_robust_candidate_is_advanceable_with_no_reasons() {
        let v = evaluate_candidate(&passing(), &thresholds());
        assert_eq!(v.status, Verdict::Advanceable);
        assert!(v.failed_criteria.is_empty());
    }

    #[test]
    fn each_criterion_yields_its_matching_reason() {
        // Drawdown over budget.
        let mut ev = passing();
        ev.max_drawdown = dec!(0.40);
        let v = evaluate_candidate(&ev, &thresholds());
        assert_eq!(v.status, Verdict::Rejected);
        assert_eq!(v.failed_criteria.len(), 1);
        assert_eq!(
            v.failed_criteria[0].kind,
            RejectionKind::DrawdownExceedsBudget
        );
        assert_eq!(v.failed_criteria[0].observed, "0.40");
        assert_eq!(v.failed_criteria[0].threshold, "0.30");

        // Turnover implausible.
        let mut ev = passing();
        ev.turnover = dec!(9);
        let v = evaluate_candidate(&ev, &thresholds());
        assert_eq!(
            v.failed_criteria[0].kind,
            RejectionKind::TurnoverImplausible
        );

        // Fails baseline comparison (margin below required).
        let mut ev = passing();
        ev.baseline_margin = dec!(0.001);
        let v = evaluate_candidate(&ev, &thresholds());
        assert_eq!(
            v.failed_criteria[0].kind,
            RejectionKind::FailsBaselineComparison
        );

        // Edge vanishes under doubled costs: doubled return no longer beats the floor.
        let mut ev = passing();
        ev.doubled_return = dec!(0.01); // <= doubled_baseline_floor (0.03)
        let v = evaluate_candidate(&ev, &thresholds());
        assert_eq!(
            v.failed_criteria[0].kind,
            RejectionKind::EdgeVanishesUnderDoubledCosts
        );
        // The recorded pair provably justifies the verdict: observed <= threshold.
        assert_eq!(v.failed_criteria[0].observed, "0.01");
        assert_eq!(v.failed_criteria[0].threshold, "0.03");

        // Depends on one period (fold dispersion over budget).
        let mut ev = passing();
        ev.fold_dispersion = dec!(0.90);
        let v = evaluate_candidate(&ev, &thresholds());
        assert_eq!(v.failed_criteria[0].kind, RejectionKind::DependsOnOnePeriod);

        // Parameter fragile (neighbor degradation over tolerance).
        let mut ev = passing();
        ev.neighbor_degradation = dec!(0.25);
        let v = evaluate_candidate(&ev, &thresholds());
        assert_eq!(v.failed_criteria[0].kind, RejectionKind::ParameterFragile);
    }

    #[test]
    fn insufficient_data_is_a_hard_stop_recorded_alone() {
        // Even with otherwise-terrible metrics, too-thin data rejects on InsufficientData ONLY —
        // the other metrics are not trusted.
        let mut ev = passing();
        ev.valid_windows = 2; // below min_windows (4)
        ev.max_drawdown = dec!(0.99);
        ev.turnover = dec!(100);
        let v = evaluate_candidate(&ev, &thresholds());
        assert_eq!(v.status, Verdict::Rejected);
        assert_eq!(v.failed_criteria.len(), 1);
        assert_eq!(v.failed_criteria[0].kind, RejectionKind::InsufficientData);
        assert_eq!(v.failed_criteria[0].observed, "2");
        assert_eq!(v.failed_criteria[0].threshold, "4");
    }

    #[test]
    fn multiple_failures_are_listed_in_canonical_order() {
        let mut ev = passing();
        ev.baseline_margin = dec!(-0.10); // fails baseline
        ev.max_drawdown = dec!(0.50); // over drawdown budget
        ev.turnover = dec!(9); // over turnover budget
        let v = evaluate_candidate(&ev, &thresholds());
        let kinds: Vec<_> = v.failed_criteria.iter().map(|r| r.kind).collect();
        // Canonical order: baseline → (doubled) → (one-period) → (fragile) → drawdown → turnover.
        assert_eq!(
            kinds,
            vec![
                RejectionKind::FailsBaselineComparison,
                RejectionKind::DrawdownExceedsBudget,
                RejectionKind::TurnoverImplausible,
            ]
        );
        // Deterministic: the same evidence yields the same verdict every time.
        assert_eq!(evaluate_candidate(&ev, &thresholds()), v);
    }

    #[test]
    fn verdict_and_kind_serialize_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&Verdict::Advanceable).unwrap(),
            "\"advanceable\""
        );
        assert_eq!(
            serde_json::to_string(&RejectionKind::EdgeVanishesUnderDoubledCosts).unwrap(),
            "\"edge_vanishes_under_doubled_costs\""
        );
    }
}
