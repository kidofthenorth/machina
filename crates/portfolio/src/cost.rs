//! The deterministic fill model.
//!
//! A fill is a *pure function* of `(side, input_amount, exec_price, CostModel)` — no randomness, no
//! clock. Costs are applied against the trader: slippage shifts the effective price the wrong way,
//! the DEX fee reduces the output, and the Solana base + priority fees are charged in SOL. Token
//! outputs are floored to token decimals so we never over-credit.

use research_core::money::{
    apply_bps, lamports_to_sol, quantize_floor, SOL_DECIMALS, USDC_DECIMALS,
};
use research_core::Decimal;

/// Which direction a swap goes. Base = SOL, quote = USDC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// USDC → SOL.
    Buy,
    /// SOL → USDC.
    Sell,
}

/// Configured execution-cost assumptions. These are modeled, not measured (plan §10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostModel {
    /// Aggregator/DEX fee, basis points, taken on the output side.
    pub dex_fee_bps: u32,
    /// Adverse price movement between decision and fill, basis points.
    pub slippage_bps: u32,
    /// Solana base fee per signature (lamports).
    pub base_fee_lamports: i64,
    /// Optional prioritization fee (lamports).
    pub priority_fee_lamports: i64,
}

impl CostModel {
    /// A zero-cost model (useful for "same strategy before costs" baselines).
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            dex_fee_bps: 0,
            slippage_bps: 0,
            base_fee_lamports: 0,
            priority_fee_lamports: 0,
        }
    }

    /// Total network gas (base + priority) charged per transaction, in SOL.
    #[must_use]
    pub fn gas_sol(&self) -> Decimal {
        lamports_to_sol(self.base_fee_lamports + self.priority_fee_lamports)
    }

    /// Priority-fee portion only, in SOL (for the metrics report).
    #[must_use]
    pub fn priority_sol(&self) -> Decimal {
        lamports_to_sol(self.priority_fee_lamports)
    }

    /// Effective buy price (USDC per SOL), worsened by slippage.
    fn buy_price(&self, price: Decimal) -> Decimal {
        price + apply_bps(price, self.slippage_bps)
    }

    /// Effective sell price (USDC per SOL), worsened by slippage.
    fn sell_price(&self, price: Decimal) -> Decimal {
        price - apply_bps(price, self.slippage_bps)
    }

    /// Simulate spending `quote_in` USDC to buy SOL at mid `price`.
    #[must_use]
    pub fn fill_buy(&self, quote_in: Decimal, price: Decimal) -> BuyFill {
        let eff = self.buy_price(price);
        let gross_base = quote_in / eff;
        let dex_fee_base = apply_bps(gross_base, self.dex_fee_bps);
        let net_base = quantize_floor(gross_base - dex_fee_base, SOL_DECIMALS);
        // Reporting figures in quote terms (valued at mid price).
        let slippage_quote = quote_in - (quote_in * price / eff);
        let dex_fee_quote = dex_fee_base * price;
        BuyFill {
            net_base,
            gas_sol: self.gas_sol(),
            slippage_quote,
            dex_fee_quote,
        }
    }

    /// Simulate selling `base_in` SOL for USDC at mid `price`.
    #[must_use]
    pub fn fill_sell(&self, base_in: Decimal, price: Decimal) -> SellFill {
        let eff = self.sell_price(price);
        let gross_quote = base_in * eff;
        let dex_fee_quote = apply_bps(gross_quote, self.dex_fee_bps);
        let net_quote = quantize_floor(gross_quote - dex_fee_quote, USDC_DECIMALS);
        let slippage_quote = base_in * (price - eff);
        SellFill {
            net_quote,
            gas_sol: self.gas_sol(),
            slippage_quote,
            dex_fee_quote,
        }
    }
}

/// Result of a simulated buy (USDC → SOL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuyFill {
    /// SOL credited (net of DEX fee, floored to SOL decimals). Gas is charged separately.
    pub net_base: Decimal,
    /// Network + priority gas, in SOL.
    pub gas_sol: Decimal,
    /// Slippage cost, valued in USDC (reporting only).
    pub slippage_quote: Decimal,
    /// DEX fee, valued in USDC (reporting only).
    pub dex_fee_quote: Decimal,
}

/// Result of a simulated sell (SOL → USDC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SellFill {
    /// USDC credited (net of DEX fee, floored to USDC decimals).
    pub net_quote: Decimal,
    /// Network + priority gas, in SOL.
    pub gas_sol: Decimal,
    /// Slippage cost, valued in USDC (reporting only).
    pub slippage_quote: Decimal,
    /// DEX fee, valued in USDC (reporting only).
    pub dex_fee_quote: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn zero_cost_buy_is_exact_inverse_of_price() {
        let c = CostModel::zero();
        // Spend 1000 USDC at price 100 → 10 SOL exactly, no fees, no gas.
        let f = c.fill_buy(dec!(1000), dec!(100));
        assert_eq!(f.net_base, dec!(10));
        assert_eq!(f.gas_sol, dec!(0));
        assert_eq!(f.slippage_quote, dec!(0));
        assert_eq!(f.dex_fee_quote, dec!(0));
    }

    #[test]
    fn costs_reduce_buy_output() {
        let c = CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5000,
            priority_fee_lamports: 50000,
        };
        let f = c.fill_buy(dec!(1000), dec!(100));
        // With slippage+fee you get strictly less than the 10 SOL a frictionless fill would give.
        assert!(f.net_base < dec!(10), "net_base={}", f.net_base);
        assert!(f.slippage_quote > dec!(0));
        assert!(f.dex_fee_quote > dec!(0));
        assert_eq!(f.gas_sol, dec!(0.000055));
    }

    #[test]
    fn sell_credits_less_than_mid_value_after_costs() {
        let c = CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5000,
            priority_fee_lamports: 50000,
        };
        let f = c.fill_sell(dec!(10), dec!(100));
        // Mid value would be 1000 USDC; after slippage + fee it is strictly less.
        assert!(f.net_quote < dec!(1000), "net_quote={}", f.net_quote);
        assert!(f.slippage_quote > dec!(0));
        assert_eq!(f.gas_sol, dec!(0.000055));
        // USDC output is floored to 6 decimals.
        assert!(f.net_quote.scale() <= USDC_DECIMALS);
    }

    #[test]
    fn zero_model_and_gas_helpers() {
        let z = CostModel::zero();
        assert_eq!(z.gas_sol(), dec!(0));
        assert_eq!(z.priority_sol(), dec!(0));
        let c = CostModel {
            dex_fee_bps: 0,
            slippage_bps: 0,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        };
        assert_eq!(c.gas_sol(), dec!(0.000055)); // (5_000 + 50_000) / 1e9
        assert_eq!(c.priority_sol(), dec!(0.00005)); // 50_000 / 1e9
    }

    #[test]
    fn dex_fee_only_reduces_output_exactly() {
        // 100 bps = 1% fee, no slippage: 1000 USDC / 100 = 10 gross SOL, minus 1% = 9.9 net.
        let c = CostModel {
            dex_fee_bps: 100,
            slippage_bps: 0,
            base_fee_lamports: 0,
            priority_fee_lamports: 0,
        };
        let f = c.fill_buy(dec!(1000), dec!(100));
        assert_eq!(f.net_base, dec!(9.9));
        assert_eq!(f.slippage_quote, dec!(0)); // no slippage
        assert_eq!(f.dex_fee_quote, dec!(10)); // 0.1 SOL * 100 USDC/SOL
        assert_eq!(f.gas_sol, dec!(0));
    }

    #[test]
    fn slippage_only_worsens_effective_price() {
        // 10% slippage, no fee: effective buy price 110 → fewer than 10 SOL and a slippage charge.
        let c = CostModel {
            dex_fee_bps: 0,
            slippage_bps: 1_000,
            base_fee_lamports: 0,
            priority_fee_lamports: 0,
        };
        let f = c.fill_buy(dec!(1000), dec!(100));
        assert!(f.net_base < dec!(10) && f.net_base > dec!(9), "net_base={}", f.net_base);
        assert!(f.slippage_quote > dec!(0));
        assert_eq!(f.dex_fee_quote, dec!(0));
    }

    #[test]
    fn zero_input_fills_credit_nothing_but_still_report_gas() {
        let c = CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        };
        let b = c.fill_buy(dec!(0), dec!(100));
        assert_eq!(b.net_base, dec!(0));
        assert_eq!(b.gas_sol, dec!(0.000055));
        let s = c.fill_sell(dec!(0), dec!(100));
        assert_eq!(s.net_quote, dec!(0));
        assert_eq!(s.gas_sol, dec!(0.000055));
    }

    #[test]
    fn gas_is_independent_of_trade_size() {
        let c = CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        };
        assert_eq!(
            c.fill_buy(dec!(1), dec!(100)).gas_sol,
            c.fill_buy(dec!(1_000_000), dec!(100)).gas_sol
        );
    }
}
