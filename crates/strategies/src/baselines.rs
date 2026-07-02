//! Required baselines (plan §8). Every experiment is compared against these.

use crate::Strategy;
use research_core::{Bar, Decimal};

/// Always hold USDC (target SOL weight 0). The risk-free-of-SOL benchmark.
#[derive(Debug, Clone, Copy, Default)]
pub struct HoldUsdc;

impl Strategy for HoldUsdc {
    fn name(&self) -> &str {
        "hold_usdc"
    }
    fn target_weight(&self, _history: &[Bar], _current_weight: Decimal) -> Decimal {
        Decimal::ZERO
    }
}

/// Always fully invested in SOL (target weight 1). The "did you beat just holding SOL?" benchmark.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuyAndHoldSol;

impl Strategy for BuyAndHoldSol {
    fn name(&self) -> &str {
        "buy_and_hold_sol"
    }
    fn target_weight(&self, _history: &[Bar], _current_weight: Decimal) -> Decimal {
        Decimal::ONE
    }
}

/// Static 50/50 SOL/USDC, continuously rebalanced toward the target by the simulator.
#[derive(Debug, Clone, Copy, Default)]
pub struct Static5050;

impl Strategy for Static5050 {
    fn name(&self) -> &str {
        "static_50_50"
    }
    fn target_weight(&self, _history: &[Bar], _current_weight: Decimal) -> Decimal {
        Decimal::new(5, 1) // 0.5
    }
}

/// Dollar-cost-average into SOL: a linear ramp of target SOL weight from 0 → 1 over `ramp_bars`
/// bars, then fully invested. This is a *weight-space proxy* for periodic fixed-USDC buys — a
/// benchmark required by the M3 gate, not a tuned strategy.
#[derive(Debug, Clone, Copy)]
pub struct DcaIntoSol {
    /// Number of bars over which the target ramps from 0 to 1.
    pub ramp_bars: usize,
}

impl DcaIntoSol {
    /// Illustrative default: ramp into SOL over 10 bars. NOT a recommendation.
    #[must_use]
    pub fn illustrative() -> Self {
        Self { ramp_bars: 10 }
    }
}

impl Strategy for DcaIntoSol {
    fn name(&self) -> &str {
        "dca_sol"
    }
    fn target_weight(&self, history: &[Bar], _current_weight: Decimal) -> Decimal {
        if self.ramp_bars == 0 {
            return Decimal::ONE;
        }
        let elapsed = history.len().min(self.ramp_bars);
        Decimal::from(elapsed) / Decimal::from(self.ramp_bars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use research_core::Timestamp;
    use rust_decimal_macros::dec;

    /// `n` trivial bars; only the length matters to DCA.
    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                ts: Timestamp::from_unix(i as i64),
                open: dec!(100),
                high: dec!(100),
                low: dec!(100),
                close: dec!(100),
                volume: dec!(1),
            })
            .collect()
    }

    #[test]
    fn baselines_return_constant_targets() {
        assert_eq!(HoldUsdc.target_weight(&[], dec!(0.3)), dec!(0));
        assert_eq!(BuyAndHoldSol.target_weight(&[], dec!(0.3)), dec!(1));
        assert_eq!(Static5050.target_weight(&[], dec!(0.9)), dec!(0.5));
    }

    #[test]
    fn baseline_names_are_stable() {
        assert_eq!(HoldUsdc.name(), "hold_usdc");
        assert_eq!(BuyAndHoldSol.name(), "buy_and_hold_sol");
        assert_eq!(Static5050.name(), "static_50_50");
        assert_eq!(DcaIntoSol::illustrative().name(), "dca_sol");
    }

    #[test]
    fn dca_ramps_linearly_then_caps() {
        let s = DcaIntoSol { ramp_bars: 4 };
        assert_eq!(s.target_weight(&bars(0), dec!(0)), dec!(0)); // 0/4
        assert_eq!(s.target_weight(&bars(1), dec!(0)), dec!(0.25)); // 1/4
        assert_eq!(s.target_weight(&bars(2), dec!(0)), dec!(0.5)); // 2/4
        assert_eq!(s.target_weight(&bars(4), dec!(0)), dec!(1)); // 4/4
        assert_eq!(s.target_weight(&bars(9), dec!(0)), dec!(1)); // capped at 1
    }

    #[test]
    fn dca_zero_ramp_is_fully_invested() {
        // Guard: ramp_bars == 0 means "already fully in" and avoids a divide-by-zero.
        let s = DcaIntoSol { ramp_bars: 0 };
        assert_eq!(s.target_weight(&bars(0), dec!(0)), dec!(1));
        assert_eq!(s.target_weight(&bars(5), dec!(0)), dec!(1));
    }

    #[test]
    fn dca_illustrative_default_ramp() {
        assert_eq!(DcaIntoSol::illustrative().ramp_bars, 10);
    }

    #[test]
    fn baselines_ignore_history_and_current_weight() {
        // Constant-target baselines are pure constants regardless of inputs.
        let hist = bars(20);
        assert_eq!(HoldUsdc.target_weight(&hist, dec!(0.9)), dec!(0));
        assert_eq!(BuyAndHoldSol.target_weight(&hist, dec!(0.1)), dec!(1));
        assert_eq!(Static5050.target_weight(&hist, dec!(0.0)), dec!(0.5));
    }
}
