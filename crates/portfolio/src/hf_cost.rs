//! Additive HF cost-model fields (m-hf-track §3/§4/§5 row C4): depth-walk slippage,
//! congestion-regime-keyed priority fee, and a tip. Carried on a separate [`HfCostModel`]
//! wrapper rather than added to [`CostModel`] directly (planner decision, logged on the
//! M-HF-C4 card: extending `CostModel`'s own fields would force edits to ~30 existing
//! construction sites, including inside `simulator.rs`'s tests, breaking the reuse-first
//! bet's standing rule that `simulator.rs` is never touched).
//!
//! Nothing here is wired into `run`/`run_hf` yet — this card defines the types and a pure
//! cost function, proven by a hand-computed reference trade. Wiring into execution and the
//! SweepSpec-level "HF-kind requires `depth_curve`" validation are later cards (C5/C8).

use crate::cost::CostModel;
use research_core::money::{apply_bps, lamports_to_sol};
use research_core::Decimal;
use std::fmt;

/// One step of a piecewise-constant depth curve: trades up to `notional_upto` (quote terms)
/// incur `impact_bps` of slippage. Bands are sorted ascending by `notional_upto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepthBand {
    pub notional_upto: Decimal,
    pub impact_bps: u32,
}

/// A depth-walk slippage curve (m-hf-track §3: "constant-bps slippage ... is structurally
/// forbidden [as the HF base case]; it survives only as a ladder rung"). Never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepthCurve {
    bands: Vec<DepthBand>,
}

impl DepthCurve {
    /// `bands` must be non-empty and sorted strictly ascending by `notional_upto`.
    pub fn new(bands: Vec<DepthBand>) -> Result<Self, HfCostError> {
        if bands.is_empty() {
            return Err(HfCostError::EmptyDepthCurve);
        }
        if bands
            .windows(2)
            .any(|w| w[0].notional_upto >= w[1].notional_upto)
        {
            return Err(HfCostError::UnsortedDepthCurve);
        }
        if bands.windows(2).any(|w| w[0].impact_bps > w[1].impact_bps) {
            return Err(HfCostError::NonMonotonicImpact);
        }
        Ok(Self { bands })
    }

    /// Impact in bps for a trade of `notional` (quote terms): the first band whose
    /// `notional_upto >= notional`, or the deepest (last) band if `notional` walks off the
    /// end of the curve — walking past the quoted depth never costs less than the deepest
    /// quoted level.
    #[must_use]
    pub fn impact_bps_for(&self, notional: Decimal) -> u32 {
        self.bands
            .iter()
            .find(|b| b.notional_upto >= notional)
            .unwrap_or_else(|| self.bands.last().expect("DepthCurve is never empty"))
            .impact_bps
    }
}

/// Congestion regime a trade lands in (m-hf-track §3). Deriving this from trailing
/// volatility/print density is NOT built here — that is C8's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionRegime {
    Calm,
    Busy,
    Hot,
}

/// Regime-keyed priority-fee lookup, in lamports (m-hf-track §3's fee-conditional table,
/// priority-fee side only — the landing-percentile side is deferred to C8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CongestionPriorityTable {
    calm_lamports: i64,
    busy_lamports: i64,
    hot_lamports: i64,
}

impl CongestionPriorityTable {
    /// Each priority fee must be `>= 0` — a priority fee is a cost to us, never a benefit.
    /// Fail-closed at construction (mirrors [`DepthCurve::new`]), so `hf_trade_cost` can never
    /// see a table that would drive `total_quote` below the base-fee floor.
    pub fn new(
        calm_lamports: i64,
        busy_lamports: i64,
        hot_lamports: i64,
    ) -> Result<Self, HfCostError> {
        for (regime, lamports) in [
            ("calm", calm_lamports),
            ("busy", busy_lamports),
            ("hot", hot_lamports),
        ] {
            if lamports < 0 {
                return Err(HfCostError::NegativePriorityLamports { regime, lamports });
            }
        }
        Ok(Self {
            calm_lamports,
            busy_lamports,
            hot_lamports,
        })
    }

    #[must_use]
    pub fn priority_lamports_for(&self, regime: CongestionRegime) -> i64 {
        match regime {
            CongestionRegime::Calm => self.calm_lamports,
            CongestionRegime::Busy => self.busy_lamports,
            CongestionRegime::Hot => self.hot_lamports,
        }
    }
}

/// Errors constructing HF cost types. Fail-closed: never a silent default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HfCostError {
    /// `DepthCurve` must have at least one band.
    EmptyDepthCurve,
    /// `DepthCurve` bands must be sorted strictly ascending by `notional_upto`.
    UnsortedDepthCurve,
    /// `DepthCurve` `impact_bps` must be non-decreasing across bands — a larger trade must
    /// never cost fewer bps than a smaller one (a strict decrease inverts the depth-walk and
    /// understates slippage at scale).
    NonMonotonicImpact,
    /// A `CongestionPriorityTable` priority fee was negative — a cost to us can never be a
    /// benefit. `regime` names the offending field; `lamports` is its value.
    NegativePriorityLamports { regime: &'static str, lamports: i64 },
}

impl fmt::Display for HfCostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDepthCurve => write!(f, "depth curve must have at least one band"),
            Self::UnsortedDepthCurve => {
                write!(
                    f,
                    "depth curve bands must be sorted ascending by notional_upto"
                )
            }
            Self::NonMonotonicImpact => {
                write!(
                    f,
                    "depth curve impact_bps must be non-decreasing across bands"
                )
            }
            Self::NegativePriorityLamports { regime, lamports } => {
                write!(
                    f,
                    "priority-fee lamports must be >= 0, got {lamports} for {regime}"
                )
            }
        }
    }
}

impl std::error::Error for HfCostError {}

/// The additive HF cost terms, carried alongside the existing [`CostModel`] rather than
/// extending it (see module doc). `tip_bps` is bps of trade notional, not lamports (deviation
/// from m-hf-track §2.4's literal wording — logged on the M-HF-C4 card).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfCostModel {
    pub base: CostModel,
    pub depth_curve: DepthCurve,
    pub congestion_priority_table: CongestionPriorityTable,
    pub tip_bps: u32,
}

/// The priced breakdown of one trade under [`HfCostModel`]. All quote-terms fields are USDC;
/// gas is converted to quote at `price` so `total_quote` and `base_fee_floor_quote` are
/// comparable in one unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HfTradeCost {
    pub venue_fee_quote: Decimal,
    pub depth_slippage_quote: Decimal,
    pub tip_quote: Decimal,
    pub gas_lamports: i64,
    pub gas_quote: Decimal,
    pub total_quote: Decimal,
    /// The Solana base fee alone, converted to quote at `price` — the fabricated-edge floor
    /// a real HF cost model must exceed (m-hf-track §5 row C4's gate).
    pub base_fee_floor_quote: Decimal,
}

/// Price one trade's HF cost terms. Pure: no RNG, no clock, no side branching (m-hf-track §3;
/// depth-walk and tip apply symmetrically at the notional level, matching how `dex_fee_bps`
/// and gas already don't branch on side in `CostModel`).
#[must_use]
pub fn hf_trade_cost(
    hf: &HfCostModel,
    notional_quote: Decimal,
    price: Decimal,
    regime: CongestionRegime,
) -> HfTradeCost {
    let venue_fee_quote = apply_bps(notional_quote, hf.base.dex_fee_bps);
    let depth_slippage_quote = apply_bps(
        notional_quote,
        hf.depth_curve.impact_bps_for(notional_quote),
    );
    let tip_quote = apply_bps(notional_quote, hf.tip_bps);
    let gas_lamports =
        hf.base.base_fee_lamports + hf.congestion_priority_table.priority_lamports_for(regime);
    let gas_quote = lamports_to_sol(gas_lamports) * price;
    let base_fee_floor_quote = lamports_to_sol(hf.base.base_fee_lamports) * price;
    HfTradeCost {
        venue_fee_quote,
        depth_slippage_quote,
        tip_quote,
        gas_lamports,
        gas_quote,
        total_quote: venue_fee_quote + depth_slippage_quote + tip_quote + gas_quote,
        base_fee_floor_quote,
    }
}

/// Scale `base`'s HF-only fields by `num/den`, in integer/exact-Decimal space — the same
/// pattern `sweep::sensitivity::scale_cost_model` uses for `CostModel` (C6 will fold this
/// into the ladder; not wired here). `CostModel`'s own fields are scaled by the existing
/// function, untouched.
#[must_use]
pub fn scale_hf_cost_model(hf: &HfCostModel, num: u32, den: u32) -> HfCostModel {
    let den_nonzero = den.max(1);
    let scale_bps = |bps: u32| ((u64::from(bps) * u64::from(num)) / u64::from(den_nonzero)) as u32;
    let scale_lamports =
        |l: i64| ((i128::from(l) * i128::from(num)) / i128::from(den_nonzero)) as i64;
    HfCostModel {
        base: hf.base.clone(),
        depth_curve: DepthCurve {
            bands: hf
                .depth_curve
                .bands
                .iter()
                .map(|b| DepthBand {
                    notional_upto: b.notional_upto,
                    impact_bps: scale_bps(b.impact_bps),
                })
                .collect(),
        },
        congestion_priority_table: CongestionPriorityTable::new(
            scale_lamports(
                hf.congestion_priority_table
                    .priority_lamports_for(CongestionRegime::Calm),
            ),
            scale_lamports(
                hf.congestion_priority_table
                    .priority_lamports_for(CongestionRegime::Busy),
            ),
            scale_lamports(
                hf.congestion_priority_table
                    .priority_lamports_for(CongestionRegime::Hot),
            ),
        )
        .expect("scaling non-negative lamports by num/den stays non-negative"),
        tip_bps: scale_bps(hf.tip_bps),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn reference_depth_curve() -> DepthCurve {
        DepthCurve::new(vec![
            DepthBand {
                notional_upto: dec!(100),
                impact_bps: 5,
            },
            DepthBand {
                notional_upto: dec!(1_000),
                impact_bps: 20,
            },
            DepthBand {
                notional_upto: dec!(10_000),
                impact_bps: 50,
            },
        ])
        .expect("valid depth curve")
    }

    fn reference_priority_table() -> CongestionPriorityTable {
        CongestionPriorityTable::new(1_000, 10_000, 100_000).expect("valid priority table")
    }

    #[test]
    fn depth_curve_rejects_empty() {
        assert_eq!(DepthCurve::new(vec![]), Err(HfCostError::EmptyDepthCurve));
    }

    #[test]
    fn depth_curve_rejects_unsorted() {
        let equal = vec![
            DepthBand {
                notional_upto: dec!(100),
                impact_bps: 5,
            },
            DepthBand {
                notional_upto: dec!(100),
                impact_bps: 20,
            },
        ];
        assert_eq!(DepthCurve::new(equal), Err(HfCostError::UnsortedDepthCurve));

        let descending = vec![
            DepthBand {
                notional_upto: dec!(1_000),
                impact_bps: 20,
            },
            DepthBand {
                notional_upto: dec!(100),
                impact_bps: 5,
            },
        ];
        assert_eq!(
            DepthCurve::new(descending),
            Err(HfCostError::UnsortedDepthCurve)
        );
    }

    #[test]
    fn depth_curve_walks_bands_by_notional() {
        let curve = reference_depth_curve();
        assert_eq!(curve.impact_bps_for(dec!(50)), 5);
        assert_eq!(curve.impact_bps_for(dec!(100)), 5);
        assert_eq!(curve.impact_bps_for(dec!(500)), 20);
        assert_eq!(curve.impact_bps_for(dec!(50_000)), 50);
    }

    #[test]
    fn congestion_priority_table_looks_up_by_regime() {
        let table = reference_priority_table();
        assert_eq!(table.priority_lamports_for(CongestionRegime::Calm), 1_000);
        assert_eq!(table.priority_lamports_for(CongestionRegime::Busy), 10_000);
        assert_eq!(table.priority_lamports_for(CongestionRegime::Hot), 100_000);
    }

    #[test]
    fn hf_trade_cost_matches_hand_computed_reference_trade() {
        let hf = HfCostModel {
            base: CostModel {
                dex_fee_bps: 10,
                slippage_bps: 0,
                base_fee_lamports: 5_000,
                priority_fee_lamports: 0,
            },
            depth_curve: reference_depth_curve(),
            congestion_priority_table: reference_priority_table(),
            tip_bps: 2,
        };
        let cost = hf_trade_cost(&hf, dec!(500), dec!(100), CongestionRegime::Busy);

        assert_eq!(cost.venue_fee_quote, dec!(0.5));
        assert_eq!(cost.depth_slippage_quote, dec!(1));
        assert_eq!(cost.tip_quote, dec!(0.1));
        assert_eq!(cost.gas_lamports, 15_000);
        assert_eq!(cost.gas_quote, dec!(0.0015));
        assert_eq!(cost.total_quote, dec!(1.6015));
        assert_eq!(cost.base_fee_floor_quote, dec!(0.0005));
    }

    #[test]
    fn total_cost_exceeds_base_fee_floor() {
        let hf = HfCostModel {
            base: CostModel {
                dex_fee_bps: 10,
                slippage_bps: 0,
                base_fee_lamports: 5_000,
                priority_fee_lamports: 0,
            },
            depth_curve: reference_depth_curve(),
            congestion_priority_table: reference_priority_table(),
            tip_bps: 2,
        };
        let cost = hf_trade_cost(&hf, dec!(500), dec!(100), CongestionRegime::Busy);
        assert!(cost.total_quote > cost.base_fee_floor_quote);
        assert_eq!(cost.total_quote, dec!(1.6015));
        assert_eq!(cost.base_fee_floor_quote, dec!(0.0005));
    }

    #[test]
    fn zero_hf_terms_still_exceeds_floor_when_gas_priority_is_nonzero() {
        let hf = HfCostModel {
            base: CostModel {
                dex_fee_bps: 0,
                slippage_bps: 0,
                base_fee_lamports: 5_000,
                priority_fee_lamports: 0,
            },
            depth_curve: DepthCurve::new(vec![DepthBand {
                notional_upto: dec!(1_000_000),
                impact_bps: 0,
            }])
            .expect("valid depth curve"),
            congestion_priority_table: CongestionPriorityTable::new(0, 10_000, 0)
                .expect("valid priority table"),
            tip_bps: 0,
        };
        let cost = hf_trade_cost(&hf, dec!(500), dec!(100), CongestionRegime::Busy);
        assert!(cost.total_quote > cost.base_fee_floor_quote);
    }

    #[test]
    fn scale_hf_cost_model_scales_hf_fields_exactly() {
        let hf = HfCostModel {
            base: CostModel {
                dex_fee_bps: 10,
                slippage_bps: 0,
                base_fee_lamports: 5_000,
                priority_fee_lamports: 0,
            },
            depth_curve: reference_depth_curve(),
            congestion_priority_table: reference_priority_table(),
            tip_bps: 2,
        };
        let scaled = scale_hf_cost_model(&hf, 2, 1);

        assert_eq!(scaled.base, hf.base);
        assert_eq!(scaled.depth_curve.impact_bps_for(dec!(50)), 10);
        assert_eq!(scaled.depth_curve.impact_bps_for(dec!(500)), 40);
        assert_eq!(scaled.depth_curve.impact_bps_for(dec!(50_000)), 100);
        assert_eq!(
            scaled
                .congestion_priority_table
                .priority_lamports_for(CongestionRegime::Calm),
            2_000
        );
        assert_eq!(
            scaled
                .congestion_priority_table
                .priority_lamports_for(CongestionRegime::Busy),
            20_000
        );
        assert_eq!(
            scaled
                .congestion_priority_table
                .priority_lamports_for(CongestionRegime::Hot),
            200_000
        );
        assert_eq!(scaled.tip_bps, 4);
    }

    #[test]
    fn repeated_calls_are_deterministic() {
        let hf = HfCostModel {
            base: CostModel {
                dex_fee_bps: 10,
                slippage_bps: 0,
                base_fee_lamports: 5_000,
                priority_fee_lamports: 0,
            },
            depth_curve: reference_depth_curve(),
            congestion_priority_table: reference_priority_table(),
            tip_bps: 2,
        };
        let a = hf_trade_cost(&hf, dec!(500), dec!(100), CongestionRegime::Busy);
        let b = hf_trade_cost(&hf, dec!(500), dec!(100), CongestionRegime::Busy);
        assert_eq!(a, b);
    }

    #[test]
    fn depth_curve_rejects_non_monotonic_impact() {
        // A larger trade priced cheaper (bps) than a smaller one inverts the depth-walk.
        let decreasing = vec![
            DepthBand {
                notional_upto: dec!(100),
                impact_bps: 50,
            },
            DepthBand {
                notional_upto: dec!(1_000),
                impact_bps: 20,
            },
        ];
        assert_eq!(
            DepthCurve::new(decreasing),
            Err(HfCostError::NonMonotonicImpact)
        );

        // A flat step (equal impacts) is legal — reject only a strict decrease.
        let flat = vec![
            DepthBand {
                notional_upto: dec!(100),
                impact_bps: 5,
            },
            DepthBand {
                notional_upto: dec!(1_000),
                impact_bps: 5,
            },
        ];
        assert!(DepthCurve::new(flat).is_ok());
    }

    #[test]
    fn priority_table_rejects_negative_lamports() {
        assert_eq!(
            CongestionPriorityTable::new(-1, 0, 0),
            Err(HfCostError::NegativePriorityLamports {
                regime: "calm",
                lamports: -1,
            })
        );

        let table = CongestionPriorityTable::new(0, 10_000, 100_000).expect("valid priority table");
        assert_eq!(table.priority_lamports_for(CongestionRegime::Busy), 10_000);
    }

    #[test]
    fn hf_cost_cannot_go_negative_via_priority_table() {
        // Regression pin for the C3–C5 checkpoint finding: the fabricated-benefit input
        // (a negative priority fee driving `total_quote` below `base_fee_floor_quote`) is now
        // un-constructible at the type boundary, so `hf_trade_cost` can never be handed it.
        assert!(CongestionPriorityTable::new(-10_000, 0, 0).is_err());
    }
}
