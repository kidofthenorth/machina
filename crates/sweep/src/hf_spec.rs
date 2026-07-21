//! HF-kind sweep specification parsed from the hf-strategy-lab TOML template (M-HF card C8.5).
//! Read-only research config: no secrets, no network. A **separate** struct from
//! [`crate::spec::SweepSpec`] — `SweepSpec` is a plain struct whose fields every LF call site
//! reads directly, so the HF shape lives here and leaves the LF spec and its fixtures untouched.
//!
//! Parse-only: nothing consumes [`HfSweepSpec`] yet — wiring into execution is C8.7's job.
//!
//! Spec-lint (fail-closed at parse time):
//! - `[advancement]` must **not** set `turnover_budget` — HF specs go through the `None` path
//!   (high turnover is the design, priced via cost drag instead). The key is absent from
//!   [`HfAdvancementToml`] and rejected via `#[serde(deny_unknown_fields)]`, surfaced as
//!   [`HfSpecError::TurnoverBudgetNotAllowed`].
//! - `cost_drag_share_ceiling` and `per_trade_edge_floor` are **required** (C6's pairing rule:
//!   an HF spec always resolves both to `Some`).
//! - `[hf_cost]` is **required**, and its depth curve must be non-empty (m-hf-track §3:
//!   constant-bps slippage is structurally forbidden as the HF base case) — construction
//!   delegates to `portfolio`'s fail-closed constructors and surfaces their errors.
//! - `max_lookback_bars` must be `> 0` (m-hf-track §2's lookback bound), so an unbounded
//!   warm-up can't silently break the windowed-memory bound.
//!
//! `resolution_secs` is carried as a field (not hardcoded) and lint-checked here at parse time:
//! a non-positive value is always rejected, and a value other than `1` is rejected whenever any
//! HF family is enabled (every family that exists today is 1-second) — surfaced as
//! [`HfSpecError::UnsupportedResolutionSecs`]. A coarser resolution with all families disabled
//! still parses, so the field stays genuinely carried rather than hardcoded.

use crate::advance::AdvancementThresholds;
use crate::config::PartitionSpec;
use crate::param::ParamGrid;
use crate::window::{WalkForward, WindowError, WindowKind};
use portfolio::{CongestionPriorityTable, DepthBand, DepthCurve, HfCostError, HfCostModel};
use research_core::{DateParseError, Decimal};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct HfSweepSpecToml {
    allowlist_version: String,
    partitions: PartitionsToml,
    walk_forward: WalkForwardToml,
    advancement: HfAdvancementToml,
    resolution_secs: i64,
    max_lookback_bars: usize,
    intraday_meanrev_v1: IntradayMeanRevToml,
    hf_cost: HfSweepCostToml,
    latency: LatencyToml,
    adversarial: AdversarialToml,
    congestion: CongestionToml,
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
/// `turnover_budget` is deliberately absent; `deny_unknown_fields` makes setting it a
/// parse-time rejection (the cheapest, most fail-closed enforcement of the HF `None` path).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct HfAdvancementToml {
    drawdown_budget: Decimal,
    baseline_margin: Decimal,
    dispersion_budget: Decimal,
    neighbor_tolerance: Decimal,
    min_windows: u32,
    cost_drag_share_ceiling: Decimal,
    per_trade_edge_floor: Decimal,
}
#[derive(Debug, Clone, Deserialize)]
struct IntradayMeanRevToml {
    enabled: bool,
    anchor_periods: Vec<usize>,
    bands: Vec<Decimal>,
    weights_in: Vec<Decimal>,
    weights_out: Vec<Decimal>,
}
#[derive(Debug, Clone, Deserialize)]
struct HfSweepCostToml {
    base: BaseCostToml,
    depth_curve: DepthCurveToml,
    congestion_priority_table: PriorityTableToml,
    tip_bps: u32,
}
#[derive(Debug, Clone, Deserialize)]
struct BaseCostToml {
    dex_fee_bps: u32,
    slippage_bps: u32,
    base_fee_lamports: i64,
    priority_fee_lamports: i64,
}
#[derive(Debug, Clone, Deserialize)]
struct DepthCurveToml {
    bands: Vec<DepthBandToml>,
}
#[derive(Debug, Clone, Deserialize)]
struct DepthBandToml {
    notional_upto: Decimal,
    impact_bps: u32,
}
#[derive(Debug, Clone, Deserialize)]
struct PriorityTableToml {
    calm_lamports: i64,
    busy_lamports: i64,
    hot_lamports: i64,
}
#[derive(Debug, Clone, Deserialize)]
struct LatencyToml {
    offset_bars: usize,
    p_land_num: u64,
    p_land_den: u64,
}
#[derive(Debug, Clone, Deserialize)]
struct AdversarialToml {
    sandwich_bps: u32,
    pickoff_bps: u32,
    p_adverse_num: u64,
    p_adverse_den: u64,
}
#[derive(Debug, Clone, Deserialize)]
struct CongestionToml {
    lookback: usize,
}

/// A fully resolved HF-kind sweep specification (validated views of the TOML). Not consumed by
/// `run_sweep`/`run_hf`/the CLI yet — C8.7 wires it.
#[derive(Debug, Clone)]
pub struct HfSweepSpec {
    pub allowlist_version: String,
    pub partition: PartitionSpec,
    pub walk_forward: WalkForward,
    /// Always `turnover_budget: None` with both HF criteria `Some` (spec-lint above).
    pub thresholds: AdvancementThresholds,
    /// Enabled family grids; `intraday_meanrev_v1` is the only HF-buildable family today.
    pub grids: Vec<ParamGrid>,
    /// Bar resolution in seconds (`1` for the families that exist; carried, not hardcoded).
    pub resolution_secs: i64,
    /// Strategy warm-up bound in bars; validated `> 0` at parse time.
    pub max_lookback_bars: usize,
    /// The HF cost model, built through `portfolio`'s fail-closed constructors.
    pub hf_cost: HfCostModel,
    /// Flat wave-1 latency/landing parameters (see `hf_scenarios`'s doc comment).
    pub latency: crate::hf_scenarios::HfLatencyParams,
    /// Base-rung adversarial-selection parameters (worst-case is a ladder rung).
    pub adversarial: portfolio::AdversarialModel,
    /// Trailing-volume lookback (bars) for the congestion-regime classifier.
    pub congestion_lookback: usize,
}

/// Why an HF spec failed to parse/resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HfSpecError {
    Toml(String),
    Date(DateParseError),
    Window(WindowError),
    UnknownWalkForwardKind(String),
    /// The `[advancement]` block set `turnover_budget` — forbidden for HF specs, which must
    /// use the `None` path (turnover is the design; cost drag prices it instead).
    TurnoverBudgetNotAllowed,
    /// The `[hf_cost.depth_curve]` bands failed [`DepthCurve::new`]'s fail-closed construction
    /// (empty, unsorted, or non-monotonic); the underlying error is surfaced, not re-checked.
    /// An HF spec without a usable depth curve is structurally forbidden (m-hf-track §3).
    MissingDepthCurve(HfCostError),
    /// The `[hf_cost.congestion_priority_table]` failed [`CongestionPriorityTable::new`]
    /// (negative lamports); the underlying error is surfaced.
    InvalidPriorityTable(HfCostError),
    /// `max_lookback_bars` was `0` — an unbounded/absent warm-up bound is rejected.
    ZeroMaxLookback,
    /// `resolution_secs` (the offending value is carried) was non-positive, or was `!= 1` while
    /// at least one HF family was enabled (every family that exists today is 1-second). A
    /// coarser resolution parses only when all families are disabled.
    UnsupportedResolutionSecs(i64),
    /// `[latency].offset_bars` was `0` — `LatencyPipeline::new` requires at least 1.
    ZeroLatencyOffset,
    /// `[latency]`'s landing-probability rational was degenerate (denominator `0`) or `> 1`
    /// (numerator exceeded denominator). The offending values are carried.
    InvalidLandingProbability(u64, u64),
    /// `[adversarial]`'s adverse-selection-probability rational was degenerate (denominator
    /// `0`) or `> 1` (numerator exceeded denominator). The offending values are carried.
    InvalidAdversarialProbability(u64, u64),
    /// `[congestion].lookback` was `0` — a degenerate all-`Hot` classification is never
    /// configured silently.
    ZeroCongestionLookback,
}

impl std::fmt::Display for HfSpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Toml(msg) => write!(f, "hf sweep spec TOML parse error: {msg}"),
            Self::Date(e) => write!(f, "hf sweep spec date error: {e}"),
            Self::Window(e) => write!(f, "hf sweep spec walk-forward window error: {e}"),
            Self::UnknownWalkForwardKind(kind) => {
                write!(
                    f,
                    "unknown walk_forward.kind: {kind:?} (expected \"rolling\" or \"anchored\")"
                )
            }
            Self::TurnoverBudgetNotAllowed => {
                write!(
                    f,
                    "hf sweep spec must not set advancement.turnover_budget \
                     (HF turnover is priced via cost drag, not budgeted)"
                )
            }
            Self::MissingDepthCurve(e) => {
                write!(f, "hf sweep spec depth curve invalid: {e}")
            }
            Self::InvalidPriorityTable(e) => {
                write!(f, "hf sweep spec congestion priority table invalid: {e}")
            }
            Self::ZeroMaxLookback => {
                write!(f, "hf sweep spec max_lookback_bars must be > 0")
            }
            Self::UnsupportedResolutionSecs(v) => {
                write!(
                    f,
                    "hf sweep spec resolution_secs = {v} is unsupported: it must be >= 1, \
                     and must be exactly 1 while any HF family is enabled"
                )
            }
            Self::ZeroLatencyOffset => {
                write!(f, "hf sweep spec latency.offset_bars must be > 0")
            }
            Self::InvalidLandingProbability(num, den) => {
                write!(
                    f,
                    "hf sweep spec latency landing probability {num}/{den} is invalid: \
                     denominator must be > 0 and numerator must be <= denominator"
                )
            }
            Self::InvalidAdversarialProbability(num, den) => {
                write!(
                    f,
                    "hf sweep spec adversarial probability {num}/{den} is invalid: \
                     denominator must be > 0 and numerator must be <= denominator"
                )
            }
            Self::ZeroCongestionLookback => {
                write!(f, "hf sweep spec congestion.lookback must be > 0")
            }
        }
    }
}

impl std::error::Error for HfSpecError {}

impl HfSweepSpec {
    /// Parse and resolve the hf-strategy-lab TOML. Pure; no I/O.
    ///
    /// # Errors
    /// Returns [`HfSpecError`] if the TOML fails to parse, `[advancement]` sets
    /// `turnover_budget`, a date fails to parse, the walk-forward window configuration is
    /// invalid, `walk_forward.kind` is unknown, the depth curve or priority table fails
    /// `portfolio`'s fail-closed construction, `max_lookback_bars` is `0`, or
    /// `resolution_secs` is non-positive or `!= 1` with any HF family enabled.
    pub fn from_toml_str(s: &str) -> Result<Self, HfSpecError> {
        let t: HfSweepSpecToml = toml::from_str(s).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("unknown field `turnover_budget`") {
                HfSpecError::TurnoverBudgetNotAllowed
            } else {
                HfSpecError::Toml(msg)
            }
        })?;
        if t.max_lookback_bars == 0 {
            return Err(HfSpecError::ZeroMaxLookback);
        }
        let kind = match t.walk_forward.kind.as_str() {
            "rolling" => WindowKind::Rolling,
            "anchored" => WindowKind::Anchored,
            other => return Err(HfSpecError::UnknownWalkForwardKind(other.to_string())),
        };
        let walk_forward = WalkForward::new(
            kind,
            t.walk_forward.train_len,
            t.walk_forward.test_len,
            t.walk_forward.step,
            t.walk_forward.embargo,
        )
        .map_err(HfSpecError::Window)?;
        let partition =
            PartitionSpec::by_date(&t.partitions.validation.start, &t.partitions.holdout.start)
                .map_err(HfSpecError::Date)?;
        let thresholds = AdvancementThresholds {
            drawdown_budget: t.advancement.drawdown_budget,
            turnover_budget: None,
            baseline_margin: t.advancement.baseline_margin,
            dispersion_budget: t.advancement.dispersion_budget,
            neighbor_tolerance: t.advancement.neighbor_tolerance,
            min_windows: t.advancement.min_windows,
            cost_drag_share_ceiling: Some(t.advancement.cost_drag_share_ceiling),
            per_trade_edge_floor: Some(t.advancement.per_trade_edge_floor),
        };
        let depth_curve = DepthCurve::new(
            t.hf_cost
                .depth_curve
                .bands
                .iter()
                .map(|b| DepthBand {
                    notional_upto: b.notional_upto,
                    impact_bps: b.impact_bps,
                })
                .collect(),
        )
        .map_err(HfSpecError::MissingDepthCurve)?;
        let congestion_priority_table = CongestionPriorityTable::new(
            t.hf_cost.congestion_priority_table.calm_lamports,
            t.hf_cost.congestion_priority_table.busy_lamports,
            t.hf_cost.congestion_priority_table.hot_lamports,
        )
        .map_err(HfSpecError::InvalidPriorityTable)?;
        let hf_cost = HfCostModel {
            base: portfolio::CostModel {
                dex_fee_bps: t.hf_cost.base.dex_fee_bps,
                slippage_bps: t.hf_cost.base.slippage_bps,
                base_fee_lamports: t.hf_cost.base.base_fee_lamports,
                priority_fee_lamports: t.hf_cost.base.priority_fee_lamports,
            },
            depth_curve,
            congestion_priority_table,
            tip_bps: t.hf_cost.tip_bps,
        };
        let mut grids = Vec::new();
        if t.intraday_meanrev_v1.enabled {
            grids.push(ParamGrid::IntradayMeanRev {
                anchor_periods: t.intraday_meanrev_v1.anchor_periods,
                bands: t.intraday_meanrev_v1.bands,
                weights_in: t.intraday_meanrev_v1.weights_in,
                weights_out: t.intraday_meanrev_v1.weights_out,
            });
        }
        if t.resolution_secs < 1 || (t.resolution_secs != 1 && !grids.is_empty()) {
            return Err(HfSpecError::UnsupportedResolutionSecs(t.resolution_secs));
        }
        if t.latency.offset_bars == 0 {
            return Err(HfSpecError::ZeroLatencyOffset);
        }
        if t.latency.p_land_den == 0 || t.latency.p_land_num > t.latency.p_land_den {
            return Err(HfSpecError::InvalidLandingProbability(
                t.latency.p_land_num,
                t.latency.p_land_den,
            ));
        }
        if t.adversarial.p_adverse_den == 0
            || t.adversarial.p_adverse_num > t.adversarial.p_adverse_den
        {
            return Err(HfSpecError::InvalidAdversarialProbability(
                t.adversarial.p_adverse_num,
                t.adversarial.p_adverse_den,
            ));
        }
        if t.congestion.lookback == 0 {
            return Err(HfSpecError::ZeroCongestionLookback);
        }
        let latency = crate::hf_scenarios::HfLatencyParams {
            offset_bars: t.latency.offset_bars,
            p_land_num: t.latency.p_land_num,
            p_land_den: t.latency.p_land_den,
        };
        let adversarial = portfolio::AdversarialModel {
            sandwich_bps: t.adversarial.sandwich_bps,
            pickoff_bps: t.adversarial.pickoff_bps,
            p_adverse_num: t.adversarial.p_adverse_num,
            p_adverse_den: t.adversarial.p_adverse_den,
        };
        Ok(Self {
            allowlist_version: t.allowlist_version,
            partition,
            walk_forward,
            thresholds,
            grids,
            resolution_secs: t.resolution_secs,
            max_lookback_bars: t.max_lookback_bars,
            hf_cost,
            latency,
            adversarial,
            congestion_lookback: t.congestion.lookback,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_walk_forward_kind_is_rejected() {
        let toml_str = template_with(("kind = \"rolling\"", "kind = \"sideways\""));
        assert!(matches!(
            HfSweepSpec::from_toml_str(&toml_str),
            Err(HfSpecError::UnknownWalkForwardKind(k)) if k == "sideways"
        ));
    }

    #[test]
    fn disabled_family_yields_no_grids() {
        let toml_str = template_with(("enabled = true", "enabled = false"));
        let spec = HfSweepSpec::from_toml_str(&toml_str).unwrap();
        assert!(spec.grids.is_empty());
    }

    #[test]
    fn missing_hf_cost_block_is_a_parse_error() {
        let template = include_str!("../../../config/strategies/hf-strategy-lab.example.toml");
        let truncated: String = template
            .lines()
            .take_while(|l| !l.starts_with("[hf_cost]"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(matches!(
            HfSweepSpec::from_toml_str(&truncated),
            Err(HfSpecError::Toml(_))
        ));
    }

    #[test]
    fn error_display_is_informative() {
        assert!(HfSpecError::TurnoverBudgetNotAllowed
            .to_string()
            .contains("turnover_budget"));
        assert!(HfSpecError::MissingDepthCurve(HfCostError::EmptyDepthCurve)
            .to_string()
            .contains("depth curve"));
        assert!(HfSpecError::ZeroMaxLookback
            .to_string()
            .contains("max_lookback_bars"));
        let msg = HfSpecError::UnsupportedResolutionSecs(60).to_string();
        assert!(msg.contains("resolution_secs"));
        assert!(msg.contains("60"));
        assert!(HfSpecError::ZeroLatencyOffset
            .to_string()
            .contains("offset_bars"));
        let msg = HfSpecError::InvalidLandingProbability(11, 10).to_string();
        assert!(msg.contains("11/10"));
        let msg = HfSpecError::InvalidAdversarialProbability(21, 20).to_string();
        assert!(msg.contains("21/20"));
        assert!(HfSpecError::ZeroCongestionLookback
            .to_string()
            .contains("lookback"));
    }

    #[test]
    fn template_resolves_latency_adversarial_congestion() {
        let template = include_str!("../../../config/strategies/hf-strategy-lab.example.toml");
        let spec = HfSweepSpec::from_toml_str(template).unwrap();
        assert_eq!(spec.latency.offset_bars, 1);
        assert_eq!(spec.latency.p_land_num, 9);
        assert_eq!(spec.latency.p_land_den, 10);
        assert_eq!(spec.adversarial.sandwich_bps, 5);
        assert_eq!(spec.adversarial.pickoff_bps, 5);
        assert_eq!(spec.adversarial.p_adverse_num, 1);
        assert_eq!(spec.adversarial.p_adverse_den, 20);
        assert_eq!(spec.congestion_lookback, 300);
    }

    #[test]
    fn zero_latency_offset_is_rejected() {
        let toml_str = template_with(("offset_bars = 1", "offset_bars = 0"));
        assert!(matches!(
            HfSweepSpec::from_toml_str(&toml_str),
            Err(HfSpecError::ZeroLatencyOffset)
        ));
    }

    #[test]
    fn zero_landing_probability_denominator_is_rejected() {
        let toml_str = template_with(("p_land_den = 10", "p_land_den = 0"));
        assert!(matches!(
            HfSweepSpec::from_toml_str(&toml_str),
            Err(HfSpecError::InvalidLandingProbability(9, 0))
        ));
    }

    #[test]
    fn landing_probability_over_one_is_rejected() {
        let toml_str = template_with(("p_land_num = 9", "p_land_num = 11"));
        assert!(matches!(
            HfSweepSpec::from_toml_str(&toml_str),
            Err(HfSpecError::InvalidLandingProbability(11, 10))
        ));
    }

    #[test]
    fn adversarial_probability_over_one_is_rejected() {
        let toml_str = template_with(("p_adverse_num = 1", "p_adverse_num = 21"));
        assert!(matches!(
            HfSweepSpec::from_toml_str(&toml_str),
            Err(HfSpecError::InvalidAdversarialProbability(21, 20))
        ));
    }

    #[test]
    fn zero_congestion_lookback_is_rejected() {
        let toml_str = template_with(("lookback = 300", "lookback = 0"));
        assert!(matches!(
            HfSweepSpec::from_toml_str(&toml_str),
            Err(HfSpecError::ZeroCongestionLookback)
        ));
    }

    /// The checked-in template with one exact-match line substitution applied.
    fn template_with((from, to): (&str, &str)) -> String {
        let template = include_str!("../../../config/strategies/hf-strategy-lab.example.toml");
        assert!(template.contains(from), "template must contain {from:?}");
        template.replace(from, to)
    }
}
