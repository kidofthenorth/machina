//! `hf_cost_scenarios` — the 6-rung HF cost-scenario ladder (m-hf-track §4), assembled from the
//! same `HfCostModel` / `AdversarialModel` / latency-params inputs `run_hf_priced` will need per
//! cell. Standalone and independently testable: nothing here is wired into `run_sweep`, `run_hf`,
//! or `run_hf_priced` (C8.7f is the first real caller).
//!
//! `hf.base.slippage_bps` is scaled here but dead weight on the priced path — `run_hf_priced`
//! always synthesizes `slippage_bps: 0` (`portfolio::hf_priced.rs:11-12`); scaling it is harmless
//! but never read.

use crate::sensitivity::{scale_cost_model, ScenarioId};
use portfolio::{scale_hf_cost_model, AdversarialModel, CongestionRegime, CostModel, HfCostModel};

/// Flat wave-1 latency/landing parameters (the (regime, percentile) table is C9+ work — see the
/// C8.7 reconciliation, drift item 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfLatencyParams {
    pub offset_bars: usize,
    pub p_land_num: u64,
    pub p_land_den: u64,
}

/// One rung of the HF ladder: everything `run_hf_priced` needs, minus the per-cell pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfCostScenario {
    pub id: ScenarioId,
    pub hf: HfCostModel,
    pub adversarial: AdversarialModel,
    pub latency: HfLatencyParams,
    /// `Some(r)` = classification is overridden with a constant regime (the `HotCongestion` rung).
    pub force_regime: Option<CongestionRegime>,
}

/// The fixed 6-rung HF cost-scenario ladder, in canonical order:
/// `[BeforeCosts, Base, Doubled, HotCongestion, AdversarialWorst, Latency2x]`.
#[must_use]
pub fn hf_cost_scenarios(
    base_hf: &HfCostModel,
    base_adversarial: &AdversarialModel,
    base_latency: &HfLatencyParams,
) -> Vec<HfCostScenario> {
    let before_costs_hf = {
        let mut z = scale_hf_cost_model(base_hf, 0, 1);
        z.base = CostModel::zero();
        z
    };
    let before_costs_adversarial = AdversarialModel {
        sandwich_bps: 0,
        pickoff_bps: 0,
        p_adverse_num: 0,
        p_adverse_den: 1,
    };

    let base_bundle_hf = base_hf.clone();
    let base_bundle_adversarial = *base_adversarial;
    let base_bundle_latency = base_latency.clone();

    let doubled_hf = {
        let mut z = scale_hf_cost_model(base_hf, 2, 1);
        z.base = scale_cost_model(&base_hf.base, 2, 1);
        z
    };
    let doubled_adversarial = AdversarialModel {
        sandwich_bps: base_adversarial.sandwich_bps.saturating_mul(2),
        pickoff_bps: base_adversarial.pickoff_bps.saturating_mul(2),
        p_adverse_num: base_adversarial.p_adverse_num,
        p_adverse_den: base_adversarial.p_adverse_den,
    };

    let adversarial_worst = AdversarialModel {
        sandwich_bps: base_adversarial.sandwich_bps,
        pickoff_bps: base_adversarial.pickoff_bps,
        p_adverse_num: 1,
        p_adverse_den: 1,
    };

    let latency_2x = HfLatencyParams {
        offset_bars: base_latency.offset_bars.saturating_mul(2),
        p_land_num: base_latency.p_land_num,
        p_land_den: base_latency.p_land_den,
    };

    vec![
        HfCostScenario {
            id: ScenarioId::BeforeCosts,
            hf: before_costs_hf,
            adversarial: before_costs_adversarial,
            latency: base_latency.clone(),
            force_regime: None,
        },
        HfCostScenario {
            id: ScenarioId::Base,
            hf: base_bundle_hf,
            adversarial: base_bundle_adversarial,
            latency: base_bundle_latency,
            force_regime: None,
        },
        HfCostScenario {
            id: ScenarioId::Doubled,
            hf: doubled_hf,
            adversarial: doubled_adversarial,
            latency: base_latency.clone(),
            force_regime: None,
        },
        HfCostScenario {
            id: ScenarioId::HotCongestion,
            hf: base_hf.clone(),
            adversarial: *base_adversarial,
            latency: base_latency.clone(),
            force_regime: Some(CongestionRegime::Hot),
        },
        HfCostScenario {
            id: ScenarioId::AdversarialWorst,
            hf: base_hf.clone(),
            adversarial: adversarial_worst,
            latency: base_latency.clone(),
            force_regime: None,
        },
        HfCostScenario {
            id: ScenarioId::Latency2x,
            hf: base_hf.clone(),
            adversarial: *base_adversarial,
            latency: latency_2x,
            force_regime: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use portfolio::{CongestionPriorityTable, DepthBand, DepthCurve};
    use research_core::Decimal;

    fn base_hf() -> HfCostModel {
        HfCostModel {
            base: CostModel {
                dex_fee_bps: 10,
                slippage_bps: 5,
                base_fee_lamports: 5_000,
                priority_fee_lamports: 1_000,
            },
            depth_curve: DepthCurve::new(vec![DepthBand {
                notional_upto: Decimal::from(1_000_000),
                impact_bps: 20,
            }])
            .expect("valid depth curve"),
            congestion_priority_table: CongestionPriorityTable::new(1_000, 5_000, 20_000)
                .expect("valid priority table"),
            tip_bps: 3,
        }
    }

    fn base_adversarial() -> AdversarialModel {
        AdversarialModel {
            sandwich_bps: 5,
            pickoff_bps: 5,
            p_adverse_num: 1,
            p_adverse_den: 20,
        }
    }

    fn base_latency() -> HfLatencyParams {
        HfLatencyParams {
            offset_bars: 1,
            p_land_num: 9,
            p_land_den: 10,
        }
    }

    #[test]
    fn ladder_has_six_rungs_in_canonical_order() {
        let ladder = hf_cost_scenarios(&base_hf(), &base_adversarial(), &base_latency());
        let ids: Vec<ScenarioId> = ladder.iter().map(|s| s.id).collect();
        assert_eq!(
            ids,
            vec![
                ScenarioId::BeforeCosts,
                ScenarioId::Base,
                ScenarioId::Doubled,
                ScenarioId::HotCongestion,
                ScenarioId::AdversarialWorst,
                ScenarioId::Latency2x,
            ]
        );
    }

    #[test]
    fn before_costs_is_zeroed() {
        let ladder = hf_cost_scenarios(&base_hf(), &base_adversarial(), &base_latency());
        let rung = &ladder[0];
        assert_eq!(rung.hf.base, CostModel::zero());
        assert_eq!(rung.hf.tip_bps, 0);
        assert_eq!(
            rung.hf
                .congestion_priority_table
                .priority_lamports_for(CongestionRegime::Calm),
            0
        );
        assert_eq!(
            rung.hf
                .congestion_priority_table
                .priority_lamports_for(CongestionRegime::Busy),
            0
        );
        assert_eq!(
            rung.hf
                .congestion_priority_table
                .priority_lamports_for(CongestionRegime::Hot),
            0
        );
        assert_eq!(rung.adversarial.p_adverse_num, 0);
        assert_eq!(rung.adversarial.sandwich_bps, 0);
        assert_eq!(rung.adversarial.pickoff_bps, 0);
    }

    #[test]
    fn doubled_doubles_costs_but_not_probabilities() {
        let hf = base_hf();
        let adversarial = base_adversarial();
        let ladder = hf_cost_scenarios(&hf, &adversarial, &base_latency());
        let rung = &ladder[2];
        assert_eq!(rung.hf.base.dex_fee_bps, hf.base.dex_fee_bps * 2);
        assert_eq!(rung.hf.tip_bps, hf.tip_bps * 2);
        assert_eq!(
            rung.hf
                .congestion_priority_table
                .priority_lamports_for(CongestionRegime::Hot),
            hf.congestion_priority_table
                .priority_lamports_for(CongestionRegime::Hot)
                * 2
        );
        assert_eq!(rung.adversarial.sandwich_bps, adversarial.sandwich_bps * 2);
        assert_eq!(rung.adversarial.pickoff_bps, adversarial.pickoff_bps * 2);
        assert_eq!(rung.adversarial.p_adverse_num, adversarial.p_adverse_num);
        assert_eq!(rung.adversarial.p_adverse_den, adversarial.p_adverse_den);
    }

    #[test]
    fn adversarial_worst_is_certain_with_base_bps() {
        let adversarial = base_adversarial();
        let ladder = hf_cost_scenarios(&base_hf(), &adversarial, &base_latency());
        let rung = &ladder[4];
        assert_eq!(rung.adversarial.p_adverse_num, 1);
        assert_eq!(rung.adversarial.p_adverse_den, 1);
        assert_eq!(rung.adversarial.sandwich_bps, adversarial.sandwich_bps);
        assert_eq!(rung.adversarial.pickoff_bps, adversarial.pickoff_bps);
    }

    #[test]
    fn latency_2x_doubles_offset_only() {
        let latency = base_latency();
        let ladder = hf_cost_scenarios(&base_hf(), &base_adversarial(), &latency);
        let rung = &ladder[5];
        assert_eq!(rung.latency.offset_bars, latency.offset_bars * 2);
        assert_eq!(rung.latency.p_land_num, latency.p_land_num);
        assert_eq!(rung.latency.p_land_den, latency.p_land_den);
    }

    #[test]
    fn hot_congestion_differs_from_base_only_in_force_regime() {
        let ladder = hf_cost_scenarios(&base_hf(), &base_adversarial(), &base_latency());
        let base_rung = &ladder[1];
        let hot_rung = &ladder[3];
        assert_eq!(hot_rung.hf, base_rung.hf);
        assert_eq!(hot_rung.adversarial, base_rung.adversarial);
        assert_eq!(hot_rung.latency, base_rung.latency);
        assert_eq!(base_rung.force_regime, None);
        assert_eq!(hot_rung.force_regime, Some(CongestionRegime::Hot));
    }

    #[test]
    fn ladder_is_deterministic() {
        let a = hf_cost_scenarios(&base_hf(), &base_adversarial(), &base_latency());
        let b = hf_cost_scenarios(&base_hf(), &base_adversarial(), &base_latency());
        assert_eq!(a, b);
    }
}
