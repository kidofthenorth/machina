//! Fee/cost sensitivity (plan §9, §14 S10) — a first-class output, not an afterthought.
//!
//! Real edges die to costs. So every candidate is evaluated across a **fixed ladder of cost
//! scenarios** built from the configured base model by **exact integer scaling** (the `CostModel`
//! fields are `u32`/`i64` only — never `f64`, so a scenario is bit-exact and order-stable):
//!
//! - [`ScenarioId::BeforeCosts`] — `CostModel::zero()`, the "same strategy before costs" reference.
//! - [`ScenarioId::Base`] — costs exactly as configured.
//! - [`ScenarioId::Doubled`] — every cost field ×2, the mandated doubled-costs survival test.
//! - [`ScenarioId::DoubledSlippage`] / [`ScenarioId::DoubledPriority`] — one axis ×2, to see which
//!   cost the edge is most fragile to.
//!
//! The per-candidate [`FeeSensitivity`] block reports base-vs-doubled metrics plus the `Decimal`
//! **return drag** from costs and from doubling — framed strictly as robustness, never profit
//! (invariant 11). `survives_doubled` is grounded in the cost-matched baselines (`baseline.rs`), not
//! a bare `total_return > 0`.

use crate::cell::CellResult;
use portfolio::CostModel;
use research_core::Decimal;

/// A named cost scenario in the sensitivity ladder. Serialized as its stable snake_case id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ScenarioId {
    /// Zero costs — the frictionless "same strategy before costs" reference.
    #[serde(rename = "before_costs")]
    BeforeCosts,
    /// Costs exactly as configured.
    #[serde(rename = "base")]
    Base,
    /// Every cost field ×2 — the mandated doubled-costs survival test.
    #[serde(rename = "doubled")]
    Doubled,
    /// Only slippage ×2 (isolates the slippage axis).
    #[serde(rename = "doubled_slippage")]
    DoubledSlippage,
    /// Only the priority fee ×2 (isolates the priority-fee axis).
    #[serde(rename = "doubled_priority")]
    DoubledPriority,
}

impl ScenarioId {
    /// Stable snake_case identifier, matching the serde form — used for canonical cell/report keys.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::BeforeCosts => "before_costs",
            Self::Base => "base",
            Self::Doubled => "doubled",
            Self::DoubledSlippage => "doubled_slippage",
            Self::DoubledPriority => "doubled_priority",
        }
    }
}

/// One scenario of the ladder: its id and the concrete [`CostModel`] to run under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostScenario {
    pub id: ScenarioId,
    pub cost: CostModel,
}

/// Scale a `u32` cost field by `num/den`, exactly (widened so the multiply cannot overflow). `den` is
/// clamped to `>= 1` so a degenerate `den == 0` cannot divide by zero.
fn scale_u32(x: u32, num: u32, den: u32) -> u32 {
    let den = den.max(1);
    ((u64::from(x) * u64::from(num)) / u64::from(den)) as u32
}

/// Scale an `i64` cost field by `num/den`, exactly (widened to `i128`). `den` clamped to `>= 1`.
fn scale_i64(x: i64, num: u32, den: u32) -> i64 {
    let den = i128::from(den.max(1));
    ((i128::from(x) * i128::from(num)) / den) as i64
}

/// Scale every field of a [`CostModel`] by `num/den`, in integer space. `Doubled` is
/// `scale_cost_model(base, 2, 1)` — exact, since `den == 1`; the general form also serves an optional
/// `scaled(num, den)` scenario (plan §9). When `den` does not divide a field the result **truncates
/// toward zero** (matching `research_core::money::quantize_floor`'s convention — floor for the
/// non-negative cost fields the repo uses). Slippage and DEX-fee are `bps` (`u32`); base/priority
/// fees are lamports (`i64`).
#[must_use]
pub fn scale_cost_model(base: &CostModel, num: u32, den: u32) -> CostModel {
    CostModel {
        dex_fee_bps: scale_u32(base.dex_fee_bps, num, den),
        slippage_bps: scale_u32(base.slippage_bps, num, den),
        base_fee_lamports: scale_i64(base.base_fee_lamports, num, den),
        priority_fee_lamports: scale_i64(base.priority_fee_lamports, num, den),
    }
}

/// The fixed cost-scenario ladder for `base`, in canonical order. Every `(family, param, window)`
/// cell — and every baseline — is evaluated across all of these (plan §9).
#[must_use]
pub fn cost_scenarios(base: &CostModel) -> Vec<CostScenario> {
    vec![
        CostScenario {
            id: ScenarioId::BeforeCosts,
            cost: CostModel::zero(),
        },
        CostScenario {
            id: ScenarioId::Base,
            cost: base.clone(),
        },
        CostScenario {
            id: ScenarioId::Doubled,
            cost: scale_cost_model(base, 2, 1),
        },
        CostScenario {
            id: ScenarioId::DoubledSlippage,
            cost: CostModel {
                slippage_bps: scale_u32(base.slippage_bps, 2, 1),
                ..base.clone()
            },
        },
        CostScenario {
            id: ScenarioId::DoubledPriority,
            cost: CostModel {
                priority_fee_lamports: scale_i64(base.priority_fee_lamports, 2, 1),
                ..base.clone()
            },
        },
    ]
}

/// The metrics a candidate posts under one cost scenario — the reportable projection of a
/// [`CellResult`]. All money fields are exact `Decimal` (`quote` == USDC).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ScenarioMetrics {
    pub scenario: ScenarioId,
    pub total_return: Decimal,
    pub turnover: Decimal,
    pub n_trades: u32,
    pub fees_paid_quote: Decimal,
    pub slippage_paid_quote: Decimal,
    pub priority_fees_paid_sol: Decimal,
}

impl ScenarioMetrics {
    /// Project the reportable cost/return fields of a cell result under `scenario`.
    #[must_use]
    pub fn from_cell(scenario: ScenarioId, cell: &CellResult) -> Self {
        Self {
            scenario,
            total_return: cell.total_return,
            turnover: cell.turnover,
            n_trades: cell.n_trades,
            fees_paid_quote: cell.fees_paid_quote,
            slippage_paid_quote: cell.slippage_paid_quote,
            priority_fees_paid_sol: cell.priority_fees_paid_sol,
        }
    }
}

/// A candidate's fee-sensitivity block (plan §9): how its return holds up as costs rise, framed as
/// robustness — never profit (invariant 11).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FeeSensitivity {
    pub before_costs: ScenarioMetrics,
    pub base: ScenarioMetrics,
    pub doubled: ScenarioMetrics,
    /// `base.total_return - doubled.total_return`: the `Decimal` return **drag** from doubling costs.
    /// Usually `>= 0` (higher costs cost return), but it **can be negative** when doubled costs
    /// suppress a marginal trade (via the simulator's `min_trade_notional` / `InsufficientSolForGas`
    /// skips) that the base scenario made at a *loss* — a negative drag means the two scenarios
    /// executed a *different trade set*, visible as `base.n_trades != doubled.n_trades`. It is a
    /// robustness diagnostic, never framed as profit (invariant 11).
    pub return_drag_doubled: Decimal,
    /// `before_costs.total_return - base.total_return`: the frictionless→base cost drag (same
    /// trade-set caveat as [`Self::return_drag_doubled`]).
    pub return_drag_costs: Decimal,
    /// Whether the candidate under **doubled** costs still strictly beats `survives_floor`. The caller
    /// supplies that floor from the cost-matched baselines under the doubled scenario (`baseline.rs`),
    /// so this is a baseline-grounded robustness check — **not** a bare `total_return > 0` (plan §15).
    pub survives_doubled: bool,
}

impl FeeSensitivity {
    /// Assemble from already-aggregated per-scenario metrics. Single source of truth for the
    /// drag and survival rules — the single-cell path [`fee_sensitivity`] delegates here, so
    /// the report and the edge-vanishes criterion can never drift apart.
    #[must_use]
    pub fn from_scenarios(
        before_costs: ScenarioMetrics,
        base: ScenarioMetrics,
        doubled: ScenarioMetrics,
        survives_floor: Decimal,
    ) -> Self {
        let return_drag_doubled = base.total_return - doubled.total_return;
        let return_drag_costs = before_costs.total_return - base.total_return;
        let survives_doubled = doubled.total_return > survives_floor;
        Self {
            before_costs,
            base,
            doubled,
            return_drag_doubled,
            return_drag_costs,
            survives_doubled,
        }
    }
}

/// Assemble a [`FeeSensitivity`] from the same candidate's cell results under the before-costs, base,
/// and doubled scenarios, plus the cost-matched baseline return floor to beat under doubled costs.
#[must_use]
pub fn fee_sensitivity(
    before_costs: &CellResult,
    base: &CellResult,
    doubled: &CellResult,
    survives_floor: Decimal,
) -> FeeSensitivity {
    FeeSensitivity::from_scenarios(
        ScenarioMetrics::from_cell(ScenarioId::BeforeCosts, before_costs),
        ScenarioMetrics::from_cell(ScenarioId::Base, base),
        ScenarioMetrics::from_cell(ScenarioId::Doubled, doubled),
        survives_floor,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::eval_cell;
    use crate::param::ParamPoint;
    use research_core::{Bar, Timestamp};
    use rust_decimal_macros::dec;

    fn base_cost() -> CostModel {
        CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        }
    }

    /// A rising/pulling-back synthetic path so a rebalancer trades and pays real costs.
    fn series(closes: &[i64]) -> Vec<Bar> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let p = Decimal::from(c);
                Bar {
                    ts: Timestamp::from_unix(i as i64 * 86_400),
                    open: p,
                    high: p,
                    low: p,
                    close: p,
                    volume: dec!(1000),
                }
            })
            .collect()
    }

    #[test]
    fn scale_cost_model_doubles_every_field_exactly() {
        let doubled = scale_cost_model(&base_cost(), 2, 1);
        assert_eq!(doubled.dex_fee_bps, 10);
        assert_eq!(doubled.slippage_bps, 40);
        assert_eq!(doubled.base_fee_lamports, 10_000);
        assert_eq!(doubled.priority_fee_lamports, 100_000);
        // Exact fractional scaling stays in integer space: 3/2 of the base.
        let one_and_half = scale_cost_model(&base_cost(), 3, 2);
        assert_eq!(one_and_half.slippage_bps, 30); // 20 * 3 / 2
        assert_eq!(one_and_half.priority_fee_lamports, 75_000); // 50_000 * 3 / 2
                                                                // A zero denominator cannot divide by zero (clamped to 1 → no scaling change vs num=1).
        assert_eq!(scale_cost_model(&base_cost(), 1, 0), base_cost());
    }

    #[test]
    fn scaling_by_zero_num_zeros_all_fields() {
        assert_eq!(scale_cost_model(&base_cost(), 0, 1), CostModel::zero());
    }

    #[test]
    fn scenario_metrics_projects_the_cell_fields() {
        let bars = series(&[100, 108, 96, 112]);
        let point = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };
        let cell = eval_cell(&point, &bars, &base_cost(), dec!(10_000), 365.0).unwrap();
        let sm = ScenarioMetrics::from_cell(ScenarioId::Base, &cell);
        assert_eq!(sm.scenario, ScenarioId::Base);
        assert_eq!(sm.total_return, cell.total_return);
        assert_eq!(sm.turnover, cell.turnover);
        assert_eq!(sm.n_trades, cell.n_trades);
        assert_eq!(sm.fees_paid_quote, cell.fees_paid_quote);
        assert_eq!(sm.slippage_paid_quote, cell.slippage_paid_quote);
        assert_eq!(sm.priority_fees_paid_sol, cell.priority_fees_paid_sol);
    }

    #[test]
    fn cost_scenarios_are_the_fixed_ladder_in_canonical_order() {
        let base = base_cost();
        let ladder = cost_scenarios(&base);
        let ids: Vec<_> = ladder.iter().map(|s| s.id).collect();
        assert_eq!(
            ids,
            vec![
                ScenarioId::BeforeCosts,
                ScenarioId::Base,
                ScenarioId::Doubled,
                ScenarioId::DoubledSlippage,
                ScenarioId::DoubledPriority,
            ]
        );
        assert_eq!(ladder[0].cost, CostModel::zero());
        assert_eq!(ladder[1].cost, base);
        assert_eq!(ladder[2].cost, scale_cost_model(&base, 2, 1));
        // Isolated axes double exactly one field, leaving the rest at base.
        assert_eq!(ladder[3].cost.slippage_bps, 40);
        assert_eq!(ladder[3].cost.dex_fee_bps, base.dex_fee_bps);
        assert_eq!(
            ladder[3].cost.priority_fee_lamports,
            base.priority_fee_lamports
        );
        assert_eq!(ladder[4].cost.priority_fee_lamports, 100_000);
        assert_eq!(ladder[4].cost.slippage_bps, base.slippage_bps);
    }

    #[test]
    fn scenario_label_matches_serde_form() {
        for id in [
            ScenarioId::BeforeCosts,
            ScenarioId::Base,
            ScenarioId::Doubled,
            ScenarioId::DoubledSlippage,
            ScenarioId::DoubledPriority,
        ] {
            let json = serde_json::to_string(&id).unwrap();
            assert_eq!(json, format!("\"{}\"", id.label()));
        }
    }

    #[test]
    fn doubled_costs_are_a_strict_drag_when_the_trade_set_is_unchanged() {
        // A well-capitalized rebalancer on a moving price, so it trades on every bar under any cost.
        // Cost monotonicity ("doubled >= base fees, <= base net return") is NOT a universal law: it
        // holds only when the *trade set is unchanged*. Higher costs can suppress a marginal trade
        // (the `min_trade_notional` / `InsufficientSolForGas` skips in `portfolio::simulator`), and a
        // suppressed net-losing trade can even raise the net return. We keep the account large so the
        // trade set is identical across scenarios, then assert that identity explicitly and only then
        // assert monotonicity — so this stays a real property, not a coincidence a future fixture edit
        // could silently break.
        let bars = series(&[100, 108, 96, 112, 104, 120, 110, 130]);
        let point = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };
        let base = base_cost();
        let doubled = scale_cost_model(&base, 2, 1);

        let before_c = eval_cell(&point, &bars, &CostModel::zero(), dec!(10_000), 365.0).unwrap();
        let base_c = eval_cell(&point, &bars, &base, dec!(10_000), 365.0).unwrap();
        let doubled_c = eval_cell(&point, &bars, &doubled, dec!(10_000), 365.0).unwrap();

        // The frictionless scenario pays nothing.
        assert_eq!(before_c.fees_paid_quote, dec!(0));
        assert_eq!(before_c.slippage_paid_quote, dec!(0));
        assert_eq!(before_c.priority_fees_paid_sol, dec!(0));

        // Monotonicity's precondition, asserted explicitly: the same trades fire in every scenario.
        assert!(base_c.n_trades > 0, "test needs a trading candidate");
        assert_eq!(
            base_c.n_trades, doubled_c.n_trades,
            "monotonicity below assumes an unchanged trade set"
        );
        assert_eq!(before_c.n_trades, base_c.n_trades, "unchanged trade set");

        // Given that unchanged trade set, doubled costs are a strict drag: never cheaper, never a
        // higher net return.
        assert!(doubled_c.fees_paid_quote >= base_c.fees_paid_quote);
        assert!(doubled_c.slippage_paid_quote >= base_c.slippage_paid_quote);
        assert!(doubled_c.priority_fees_paid_sol >= base_c.priority_fees_paid_sol);
        assert!(doubled_c.total_return <= base_c.total_return);

        // The block records the drags as Decimals and the survives verdict against a supplied floor.
        let fs = fee_sensitivity(&before_c, &base_c, &doubled_c, dec!(0));
        assert_eq!(
            fs.return_drag_doubled,
            base_c.total_return - doubled_c.total_return
        );
        assert_eq!(
            fs.return_drag_costs,
            before_c.total_return - base_c.total_return
        );
        // Non-negative here *because* the trade set is unchanged (asserted above); a negative drag is
        // legitimate in general when higher costs suppress a net-losing trade (see the field doc).
        assert!(
            fs.return_drag_doubled >= dec!(0),
            "with an unchanged trade set, doubling costs cannot raise returns"
        );
        assert_eq!(fs.survives_doubled, doubled_c.total_return > dec!(0));
    }

    #[test]
    fn cost_monotonicity_inverts_when_higher_costs_suppress_a_losing_trade() {
        // The honest converse of the previous test, pinned as a regression: on a sub-cent account the
        // base scenario makes a marginal deploy that loses to costs, but DOUBLED costs suppress it
        // entirely (the deploy can no longer cover doubled gas → `InsufficientSolForGas` skip). So
        // doubled trades LESS, pays LESS, and posts a HIGHER net return — and `return_drag_doubled`
        // goes negative. `fee_sensitivity` records this faithfully (no clamp, no panic); the
        // trade-set change is visible via `n_trades`. This is not a bug — it is why monotonicity is
        // documented as conditional on an unchanged trade set.
        let bars = series(&[100, 100, 100, 100]);
        let point = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };
        let base = base_cost();
        let doubled = scale_cost_model(&base, 2, 1);
        let cash = dec!(0.02); // sub-cent account: one deploy sits right on the gas boundary

        let before_c = eval_cell(&point, &bars, &CostModel::zero(), cash, 365.0).unwrap();
        let base_c = eval_cell(&point, &bars, &base, cash, 365.0).unwrap();
        let doubled_c = eval_cell(&point, &bars, &doubled, cash, 365.0).unwrap();

        // Different trade sets: doubled costs suppress the marginal deploy that base executes.
        assert_eq!(base_c.n_trades, 1);
        assert_eq!(doubled_c.n_trades, 0);

        // The inversion: doubled is cheaper and higher-return than base here.
        assert!(doubled_c.fees_paid_quote < base_c.fees_paid_quote);
        assert!(doubled_c.total_return > base_c.total_return);

        // `return_drag_doubled` is legitimately negative, and the trade-set change is observable.
        let fs = fee_sensitivity(&before_c, &base_c, &doubled_c, dec!(0));
        assert!(fs.return_drag_doubled < dec!(0));
        assert_ne!(fs.base.n_trades, fs.doubled.n_trades);
    }

    #[test]
    fn fee_sensitivity_delegates_to_from_scenarios() {
        let bars = series(&[100, 108, 96, 112]);
        let point = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0),
        };
        let base = base_cost();
        let doubled = scale_cost_model(&base, 2, 1);
        let a = eval_cell(&point, &bars, &CostModel::zero(), dec!(10_000), 365.0).unwrap();
        let b = eval_cell(&point, &bars, &base, dec!(10_000), 365.0).unwrap();
        let c = eval_cell(&point, &bars, &doubled, dec!(10_000), 365.0).unwrap();
        let floor = dec!(0);

        assert_eq!(
            fee_sensitivity(&a, &b, &c, floor),
            FeeSensitivity::from_scenarios(
                ScenarioMetrics::from_cell(ScenarioId::BeforeCosts, &a),
                ScenarioMetrics::from_cell(ScenarioId::Base, &b),
                ScenarioMetrics::from_cell(ScenarioId::Doubled, &c),
                floor,
            )
        );
    }
}
