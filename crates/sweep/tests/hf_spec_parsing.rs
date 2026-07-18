//! Integration tests for `sweep::hf_spec` (M-HF card C8.5): the checked-in
//! `hf-strategy-lab.example.toml` template parses and resolves to the values written in it,
//! and the spec-lint rules reject the forbidden shapes (turnover budget set, empty depth
//! curve, zero lookback bound). Parse-only — nothing here executes a sweep.

use portfolio::{hf_trade_cost, CongestionRegime, HfCostError};
use rust_decimal_macros::dec;
use sweep::{HfSpecError, HfSweepSpec, ParamGrid, PartitionSpec, WalkForward, WindowKind};

const TEMPLATE: &str = include_str!("../../../config/strategies/hf-strategy-lab.example.toml");

mod hf_spec_parsing {
    use super::*;

    #[test]
    fn example_template_parses_and_resolves() {
        let spec = HfSweepSpec::from_toml_str(TEMPLATE).unwrap();

        assert_eq!(spec.allowlist_version, "2026-06-29");
        assert_eq!(spec.resolution_secs, 1);
        assert_eq!(spec.max_lookback_bars, 500);
        assert_eq!(
            spec.partition,
            PartitionSpec::by_date("2024-01-01", "2025-01-01").unwrap()
        );
        assert_eq!(
            spec.walk_forward,
            WalkForward::new(WindowKind::Rolling, 3600, 900, 900, 60).unwrap()
        );

        // HF thresholds: turnover budget OFF, both HF criteria ON (C6's pairing rule).
        assert_eq!(spec.thresholds.turnover_budget, None);
        assert_eq!(spec.thresholds.cost_drag_share_ceiling, Some(dec!(0.50)));
        assert_eq!(spec.thresholds.per_trade_edge_floor, Some(dec!(0)));
        assert_eq!(spec.thresholds.drawdown_budget, dec!(0.35));
        assert_eq!(spec.thresholds.min_windows, 3);

        // The one HF-buildable family, with the axes as written (2×2×1×1 = 4 points).
        assert_eq!(spec.grids.len(), 1);
        assert_eq!(
            spec.grids[0],
            ParamGrid::IntradayMeanRev {
                anchor_periods: vec![60, 300],
                bands: vec![dec!(0.005), dec!(0.01)],
                weights_in: vec![dec!(0.50)],
                weights_out: vec![dec!(0.00)],
            }
        );
        assert_eq!(spec.grids[0].points().len(), 4);

        // The cost model resolved through portfolio's constructors, values as written.
        assert_eq!(spec.hf_cost.tip_bps, 2);
        assert_eq!(spec.hf_cost.base.dex_fee_bps, 10);
        assert_eq!(spec.hf_cost.base.base_fee_lamports, 5000);
        assert_eq!(spec.hf_cost.depth_curve.impact_bps_for(dec!(50)), 5);
        assert_eq!(spec.hf_cost.depth_curve.impact_bps_for(dec!(500)), 20);
        assert_eq!(spec.hf_cost.depth_curve.impact_bps_for(dec!(50_000)), 50);
        assert_eq!(
            spec.hf_cost
                .congestion_priority_table
                .priority_lamports_for(CongestionRegime::Busy),
            10_000
        );
        // The resolved model prices hf_cost.rs's hand-computed reference trade identically.
        let cost = hf_trade_cost(&spec.hf_cost, dec!(500), dec!(100), CongestionRegime::Busy);
        assert_eq!(cost.total_quote, dec!(1.6015));
    }

    #[test]
    fn turnover_budget_is_rejected_for_hf_specs() {
        let with_turnover = TEMPLATE.replace(
            "drawdown_budget = \"0.35\"",
            "drawdown_budget = \"0.35\"\nturnover_budget = \"12\"",
        );
        assert!(with_turnover.contains("turnover_budget"));
        assert!(matches!(
            HfSweepSpec::from_toml_str(&with_turnover),
            Err(HfSpecError::TurnoverBudgetNotAllowed)
        ));
    }

    #[test]
    fn empty_depth_curve_is_rejected_with_underlying_error() {
        // Anchor after the depth-curve table header at line start — `[intraday_meanrev_v1]` also
        // has a `bands` key, and the template's comment header mentions the table name mid-line.
        let curve = TEMPLATE
            .find("\n[hf_cost.depth_curve]")
            .expect("template has a depth-curve block");
        let start = TEMPLATE[curve..]
            .find("bands = [")
            .expect("template has depth-curve bands")
            + curve;
        let end = TEMPLATE[start..].find(']').expect("bands array closes") + start + 1;
        let empty_bands = format!("{}bands = []{}", &TEMPLATE[..start], &TEMPLATE[end..]);
        assert!(matches!(
            HfSweepSpec::from_toml_str(&empty_bands),
            Err(HfSpecError::MissingDepthCurve(HfCostError::EmptyDepthCurve))
        ));
    }

    #[test]
    fn zero_max_lookback_bars_is_rejected() {
        let zero_lookback = TEMPLATE.replace("max_lookback_bars = 500", "max_lookback_bars = 0");
        assert!(zero_lookback.contains("max_lookback_bars = 0"));
        assert!(matches!(
            HfSweepSpec::from_toml_str(&zero_lookback),
            Err(HfSpecError::ZeroMaxLookback)
        ));
    }
}
