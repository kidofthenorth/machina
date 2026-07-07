//! Sweep run specification parsed from the strategy-lab TOML template (M4 card C2).
//! Read-only research config: no secrets, no network. Unknown TOML keys are ignored
//! (the template also carries single-value demo keys the sweep does not use).

use crate::advance::AdvancementThresholds;
use crate::config::PartitionSpec;
use crate::param::ParamGrid;
use crate::window::{WalkForward, WindowError, WindowKind};
use research_core::{DateParseError, Decimal};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct SweepSpecToml {
    allowlist_version: String,
    partitions: PartitionsToml,
    walk_forward: WalkForwardToml,
    advancement: AdvancementToml,
    trend_alloc_v1: TrendAllocToml,
    threshold_rebalance_v1: ThresholdRebalanceToml,
}
#[derive(Debug, Clone, Deserialize)]
struct PartitionsToml {
    validation: DateRangeToml,
    holdout: DateRangeToml,
}
#[derive(Debug, Clone, Deserialize)]
struct DateRangeToml {
    start: String,
}
#[derive(Debug, Clone, Deserialize)]
struct WalkForwardToml {
    kind: String,
    train_len: usize,
    test_len: usize,
    step: usize,
    embargo: usize,
}
#[derive(Debug, Clone, Deserialize)]
struct AdvancementToml {
    drawdown_budget: Decimal,
    turnover_budget: Decimal,
    baseline_margin: Decimal,
    dispersion_budget: Decimal,
    neighbor_tolerance: Decimal,
    min_windows: u32,
}
#[derive(Debug, Clone, Deserialize)]
struct TrendAllocToml {
    enabled: bool,
    sma_periods: Vec<usize>,
    weights_above: Vec<Decimal>,
    weights_below: Vec<Decimal>,
}
#[derive(Debug, Clone, Deserialize)]
struct ThresholdRebalanceToml {
    enabled: bool,
    target_sol_weights: Vec<Decimal>,
    #[serde(rename = "rebalance_bands")]
    bands: Vec<Decimal>,
}

/// A fully resolved sweep specification (validated views of the TOML).
#[derive(Debug, Clone)]
pub struct SweepSpec {
    pub allowlist_version: String,
    pub partition: PartitionSpec,
    pub walk_forward: WalkForward,
    pub thresholds: AdvancementThresholds,
    /// Enabled family grids, fixed order: trend_alloc_v1 first, then threshold_rebalance_v1.
    pub grids: Vec<ParamGrid>,
}

/// Why a spec failed to parse/resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecError {
    Toml(String),
    Date(DateParseError),
    Window(WindowError),
    UnknownWalkForwardKind(String),
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Toml(msg) => write!(f, "sweep spec TOML parse error: {msg}"),
            Self::Date(e) => write!(f, "sweep spec date error: {e}"),
            Self::Window(e) => write!(f, "sweep spec walk-forward window error: {e}"),
            Self::UnknownWalkForwardKind(kind) => {
                write!(
                    f,
                    "unknown walk_forward.kind: {kind:?} (expected \"rolling\" or \"anchored\")"
                )
            }
        }
    }
}

impl std::error::Error for SpecError {}

impl SweepSpec {
    /// Parse and resolve the strategy-lab TOML. Pure; no I/O.
    ///
    /// # Errors
    /// Returns [`SpecError`] if the TOML fails to parse, a date fails to parse, the walk-forward
    /// window configuration is invalid, or `walk_forward.kind` is neither `"rolling"` nor
    /// `"anchored"`.
    pub fn from_toml_str(s: &str) -> Result<Self, SpecError> {
        let t: SweepSpecToml = toml::from_str(s).map_err(|e| SpecError::Toml(e.to_string()))?;
        let kind = match t.walk_forward.kind.as_str() {
            "rolling" => WindowKind::Rolling,
            "anchored" => WindowKind::Anchored,
            other => return Err(SpecError::UnknownWalkForwardKind(other.to_string())),
        };
        let walk_forward = WalkForward::new(
            kind,
            t.walk_forward.train_len,
            t.walk_forward.test_len,
            t.walk_forward.step,
            t.walk_forward.embargo,
        )
        .map_err(SpecError::Window)?;
        let partition =
            PartitionSpec::by_date(&t.partitions.validation.start, &t.partitions.holdout.start)
                .map_err(SpecError::Date)?;
        let thresholds = AdvancementThresholds {
            drawdown_budget: t.advancement.drawdown_budget,
            turnover_budget: t.advancement.turnover_budget,
            baseline_margin: t.advancement.baseline_margin,
            dispersion_budget: t.advancement.dispersion_budget,
            neighbor_tolerance: t.advancement.neighbor_tolerance,
            min_windows: t.advancement.min_windows,
        };
        let mut grids = Vec::new();
        if t.trend_alloc_v1.enabled {
            grids.push(ParamGrid::TrendAlloc {
                sma_periods: t.trend_alloc_v1.sma_periods,
                weights_above: t.trend_alloc_v1.weights_above,
                weights_below: t.trend_alloc_v1.weights_below,
            });
        }
        if t.threshold_rebalance_v1.enabled {
            grids.push(ParamGrid::ThresholdRebalance {
                target_sol_weights: t.threshold_rebalance_v1.target_sol_weights,
                bands: t.threshold_rebalance_v1.bands,
            });
        }
        Ok(Self {
            allowlist_version: t.allowlist_version,
            partition,
            walk_forward,
            thresholds,
            grids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn example_template_parses_and_resolves() {
        let toml_str = include_str!("../../../config/strategies/strategy-lab.example.toml");
        let spec = SweepSpec::from_toml_str(toml_str).unwrap();
        assert_eq!(
            spec.walk_forward,
            WalkForward::new(WindowKind::Rolling, 365, 90, 90, 5).unwrap()
        );
        assert_eq!(spec.thresholds.min_windows, 3);
        assert_eq!(spec.thresholds.drawdown_budget, dec!(0.35));
        assert_eq!(spec.grids.len(), 2);
        assert_eq!(spec.grids[0].points().len(), 4);
        assert_eq!(spec.grids[1].points().len(), 6);
        assert_eq!(
            spec.partition,
            PartitionSpec::by_date("2024-01-01", "2025-01-01").unwrap()
        );
    }

    #[test]
    fn unknown_walk_forward_kind_is_rejected() {
        let toml_str = r#"
            allowlist_version = "test"
            [partitions]
            validation = { start = "2024-01-01" }
            holdout = { start = "2025-01-01" }
            [walk_forward]
            kind = "sideways"
            train_len = 10
            test_len = 5
            step = 5
            embargo = 0
            [advancement]
            drawdown_budget = "0.5"
            turnover_budget = "10"
            baseline_margin = "0"
            dispersion_budget = "0.5"
            neighbor_tolerance = "0.5"
            min_windows = 1
            [trend_alloc_v1]
            enabled = false
            sma_periods = []
            weights_above = []
            weights_below = []
            [threshold_rebalance_v1]
            enabled = false
            target_sol_weights = []
            rebalance_bands = []
        "#;
        assert!(matches!(
            SweepSpec::from_toml_str(toml_str),
            Err(SpecError::UnknownWalkForwardKind(k)) if k == "sideways"
        ));
    }

    #[test]
    fn disabled_family_is_omitted() {
        let toml_str = r#"
            allowlist_version = "test"
            [partitions]
            validation = { start = "2024-01-01" }
            holdout = { start = "2025-01-01" }
            [walk_forward]
            kind = "rolling"
            train_len = 10
            test_len = 5
            step = 5
            embargo = 0
            [advancement]
            drawdown_budget = "0.5"
            turnover_budget = "10"
            baseline_margin = "0"
            dispersion_budget = "0.5"
            neighbor_tolerance = "0.5"
            min_windows = 1
            [trend_alloc_v1]
            enabled = false
            sma_periods = []
            weights_above = []
            weights_below = []
            [threshold_rebalance_v1]
            enabled = true
            target_sol_weights = ["0.5"]
            rebalance_bands = ["0.1"]
        "#;
        let spec = SweepSpec::from_toml_str(toml_str).unwrap();
        assert_eq!(spec.grids.len(), 1);
    }
}
