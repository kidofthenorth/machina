//! `threshold_rebalance_v1` — drift-band rebalancing toward a fixed SOL weight (M3 scaffold).
//!
//! Maintain a target SOL weight; only move when the current weight has drifted past a band. Inside
//! the band, return the *current* weight so the simulator does nothing (no churn). This is the
//! defensible core of "grid"/volatility-harvesting intuition without a dense grid or martingale.
//! Band-widening under stress (volatility, poor route quality, elevated priority fees) is a later
//! version; this scaffold uses a fixed band. **Not tuned**; no profitability is implied.

use crate::Strategy;
use research_core::{Bar, Decimal};

/// Rebalance to `target_sol_weight` only when `|current − target| > band`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThresholdRebalanceV1 {
    /// The SOL weight to hold.
    pub target_sol_weight: Decimal,
    /// Drift band; rebalance fires only outside it.
    pub band: Decimal,
}

impl ThresholdRebalanceV1 {
    /// Illustrative default: 50% SOL target, ±10% band. NOT a recommendation.
    #[must_use]
    pub fn illustrative() -> Self {
        Self {
            target_sol_weight: Decimal::new(5, 1),
            band: Decimal::new(1, 1),
        }
    }
}

impl Strategy for ThresholdRebalanceV1 {
    fn name(&self) -> &str {
        "threshold_rebalance_v1"
    }

    fn target_weight(&self, _history: &[Bar], current_weight: Decimal) -> Decimal {
        let drift = (current_weight - self.target_sol_weight).abs();
        if drift > self.band {
            // Outside the band → rebalance back to target.
            self.target_sol_weight
        } else {
            // Inside the band → hold; returning current weight means the simulator won't trade.
            current_weight
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn holds_inside_band() {
        let s = ThresholdRebalanceV1 {
            target_sol_weight: dec!(0.5),
            band: dec!(0.1),
        };
        // current 0.55, drift 0.05 ≤ 0.1 → hold at current.
        assert_eq!(s.target_weight(&[], dec!(0.55)), dec!(0.55));
    }

    #[test]
    fn rebalances_outside_band_up_and_down() {
        let s = ThresholdRebalanceV1 {
            target_sol_weight: dec!(0.5),
            band: dec!(0.1),
        };
        // drifted high → sell back to target.
        assert_eq!(s.target_weight(&[], dec!(0.7)), dec!(0.5));
        // drifted low → buy back to target.
        assert_eq!(s.target_weight(&[], dec!(0.2)), dec!(0.5));
    }

    #[test]
    fn boundary_is_inclusive_hold() {
        let s = ThresholdRebalanceV1 {
            target_sol_weight: dec!(0.5),
            band: dec!(0.1),
        };
        // Exactly at the band edge (drift == band) → hold (strict `>` to fire).
        assert_eq!(s.target_weight(&[], dec!(0.6)), dec!(0.6));
    }
}
