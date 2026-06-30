//! Strategy parameters, parameter grids, and the bridge to the `strategies` crate.
//!
//! A [`ParamPoint`] is one concrete parameterization of one strategy family. A [`ParamGrid`] is a
//! set of axis value-lists that expands, via [`ParamGrid::points`], into a **fixed-order** Cartesian
//! product of `ParamPoint`s — deterministic and identical across runs. [`build_strategy`] turns a
//! point into a `Box<dyn Strategy>`; strategies remain intent-only (invariant 6), so this is the
//! only coupling between the sweep and the strategy implementations.

use research_core::Decimal;
use strategies::{Strategy, ThresholdRebalanceV1, TrendAllocV1};

/// One concrete parameterization of one strategy family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamPoint {
    /// `trend_alloc_v1` — SMA-bucketed allocation.
    TrendAlloc {
        sma_period: usize,
        weight_above: Decimal,
        weight_below: Decimal,
    },
    /// `threshold_rebalance_v1` — drift-band rebalance toward a target weight.
    ThresholdRebalance {
        target_sol_weight: Decimal,
        band: Decimal,
    },
}

impl ParamPoint {
    /// Stable family name, matching the `Strategy::name()` of the built strategy.
    #[must_use]
    pub fn family(&self) -> &'static str {
        match self {
            Self::TrendAlloc { .. } => "trend_alloc_v1",
            Self::ThresholdRebalance { .. } => "threshold_rebalance_v1",
        }
    }

    /// A stable, **scale-canonical** identifier for this point, unique within a family.
    ///
    /// Decimals are `.normalize()`d so `0.5` and `0.50` (equal in value but different `rust_decimal`
    /// scale) produce the *same* id — the string therefore sorts numerically-stably and two grids
    /// that differ only in literal scale collapse to one cell.
    #[must_use]
    pub fn param_id(&self) -> String {
        match self {
            Self::TrendAlloc {
                sma_period,
                weight_above,
                weight_below,
            } => format!(
                "sma={};above={};below={}",
                sma_period,
                weight_above.normalize(),
                weight_below.normalize()
            ),
            Self::ThresholdRebalance {
                target_sol_weight,
                band,
            } => format!(
                "target={};band={}",
                target_sol_weight.normalize(),
                band.normalize()
            ),
        }
    }
}

/// Build the intent-only strategy for a parameter point. The only place sweep params become a
/// `Strategy`; the returned object owns no keys and calls no RPC (invariant 6).
#[must_use]
pub fn build_strategy(point: &ParamPoint) -> Box<dyn Strategy> {
    match *point {
        ParamPoint::TrendAlloc {
            sma_period,
            weight_above,
            weight_below,
        } => Box::new(TrendAllocV1 {
            sma_period,
            weight_above,
            weight_below,
        }),
        ParamPoint::ThresholdRebalance {
            target_sol_weight,
            band,
        } => Box::new(ThresholdRebalanceV1 {
            target_sol_weight,
            band,
        }),
    }
}

/// A grid of parameter axes for one strategy family. [`points`](ParamGrid::points) expands it into a
/// fixed-order Cartesian product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamGrid {
    /// `trend_alloc_v1` axes, iterated `sma_period` (outer) → `weight_above` → `weight_below` (inner).
    TrendAlloc {
        sma_periods: Vec<usize>,
        weights_above: Vec<Decimal>,
        weights_below: Vec<Decimal>,
    },
    /// `threshold_rebalance_v1` axes, iterated `target_sol_weight` (outer) → `band` (inner).
    ThresholdRebalance {
        target_sol_weights: Vec<Decimal>,
        bands: Vec<Decimal>,
    },
}

impl ParamGrid {
    /// Expand into a fixed-order Cartesian product of [`ParamPoint`]s.
    ///
    /// The iteration order is part of the contract: axes are nested in declared order, so the same
    /// grid yields the byte-identical `Vec` on every run (determinism, invariant 3). Duplicate or
    /// scale-variant axis values are **not** de-duplicated here; the canonical cell key collapses
    /// equal points downstream.
    #[must_use]
    pub fn points(&self) -> Vec<ParamPoint> {
        match self {
            Self::TrendAlloc {
                sma_periods,
                weights_above,
                weights_below,
            } => {
                let mut out = Vec::with_capacity(
                    sma_periods.len() * weights_above.len() * weights_below.len(),
                );
                for &sma_period in sma_periods {
                    for &weight_above in weights_above {
                        for &weight_below in weights_below {
                            out.push(ParamPoint::TrendAlloc {
                                sma_period,
                                weight_above,
                                weight_below,
                            });
                        }
                    }
                }
                out
            }
            Self::ThresholdRebalance {
                target_sol_weights,
                bands,
            } => {
                let mut out = Vec::with_capacity(target_sol_weights.len() * bands.len());
                for &target_sol_weight in target_sol_weights {
                    for &band in bands {
                        out.push(ParamPoint::ThresholdRebalance {
                            target_sol_weight,
                            band,
                        });
                    }
                }
                out
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use research_core::{Bar, Timestamp};
    use rust_decimal_macros::dec;

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
                    volume: dec!(1),
                }
            })
            .collect()
    }

    #[test]
    fn trend_grid_is_fixed_order_cartesian_product() {
        let grid = ParamGrid::TrendAlloc {
            sma_periods: vec![3, 5],
            weights_above: vec![dec!(0.5), dec!(0.75)],
            weights_below: vec![dec!(0)],
        };
        let expected = vec![
            ParamPoint::TrendAlloc {
                sma_period: 3,
                weight_above: dec!(0.5),
                weight_below: dec!(0),
            },
            ParamPoint::TrendAlloc {
                sma_period: 3,
                weight_above: dec!(0.75),
                weight_below: dec!(0),
            },
            ParamPoint::TrendAlloc {
                sma_period: 5,
                weight_above: dec!(0.5),
                weight_below: dec!(0),
            },
            ParamPoint::TrendAlloc {
                sma_period: 5,
                weight_above: dec!(0.75),
                weight_below: dec!(0),
            },
        ];
        assert_eq!(grid.points(), expected);
        // Identical across two independent expansions.
        assert_eq!(grid.points(), grid.points());
    }

    #[test]
    fn threshold_grid_is_fixed_order_cartesian_product() {
        let grid = ParamGrid::ThresholdRebalance {
            target_sol_weights: vec![dec!(0.4), dec!(0.6)],
            bands: vec![dec!(0.05), dec!(0.1)],
        };
        let pts = grid.points();
        assert_eq!(pts.len(), 4);
        assert_eq!(
            pts[0],
            ParamPoint::ThresholdRebalance {
                target_sol_weight: dec!(0.4),
                band: dec!(0.05)
            }
        );
        assert_eq!(
            pts[3],
            ParamPoint::ThresholdRebalance {
                target_sol_weight: dec!(0.6),
                band: dec!(0.1)
            }
        );
    }

    #[test]
    fn build_strategy_round_trips_family_and_behavior() {
        let point = ParamPoint::TrendAlloc {
            sma_period: 3,
            weight_above: dec!(0.75),
            weight_below: dec!(0),
        };
        let built = build_strategy(&point);
        assert_eq!(built.name(), "trend_alloc_v1");
        assert_eq!(point.family(), "trend_alloc_v1");
        // Same parameters → same target as a directly-constructed strategy.
        let direct = TrendAllocV1 {
            sma_period: 3,
            weight_above: dec!(0.75),
            weight_below: dec!(0),
        };
        let hist = series(&[10, 11, 15]);
        assert_eq!(
            built.target_weight(&hist, dec!(0)),
            direct.target_weight(&hist, dec!(0))
        );

        let tpoint = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0.1),
        };
        assert_eq!(build_strategy(&tpoint).name(), "threshold_rebalance_v1");
        assert_eq!(tpoint.family(), "threshold_rebalance_v1");
    }

    #[test]
    fn param_id_is_scale_canonical() {
        let a = ParamPoint::TrendAlloc {
            sma_period: 50,
            weight_above: dec!(0.5),
            weight_below: dec!(0.00),
        };
        let b = ParamPoint::TrendAlloc {
            sma_period: 50,
            weight_above: dec!(0.50),
            weight_below: dec!(0),
        };
        assert_eq!(a.param_id(), b.param_id());
        assert_eq!(a.param_id(), "sma=50;above=0.5;below=0");

        let t1 = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.50),
            band: dec!(0.10),
        };
        let t2 = ParamPoint::ThresholdRebalance {
            target_sol_weight: dec!(0.5),
            band: dec!(0.1),
        };
        assert_eq!(t1.param_id(), t2.param_id());
        assert_eq!(t1.param_id(), "target=0.5;band=0.1");
    }
}
