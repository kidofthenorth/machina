//! Portfolio state and the accounting primitives `apply_buy` / `apply_sell`.
//!
//! These are the lowest layer of the engine and enforce the hard accounting rules: you cannot spend
//! USDC you do not have, you cannot sell SOL you do not hold, and you must keep enough SOL to pay
//! gas. All balance changes are exact decimals.

use crate::cost::{CostModel, Side};
use research_core::Decimal;

/// The portfolio: a USDC (quote) balance and a SOL (base) balance. MVP is a single risk asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioState {
    /// USDC cash/stable balance.
    pub quote_balance: Decimal,
    /// SOL (base asset) balance.
    pub base_balance: Decimal,
}

impl PortfolioState {
    /// A fresh portfolio holding `initial_cash_usdc` in USDC and no SOL.
    #[must_use]
    pub fn new(initial_cash_usdc: Decimal) -> Self {
        Self {
            quote_balance: initial_cash_usdc,
            base_balance: Decimal::ZERO,
        }
    }

    /// Mark-to-market equity in USDC at `price` (USDC per SOL).
    #[must_use]
    pub fn equity(&self, price: Decimal) -> Decimal {
        self.quote_balance + self.base_balance * price
    }

    /// Spend `quote_in` USDC to buy SOL at mid `price`. Debits USDC, credits SOL, and pays gas in SOL.
    pub fn apply_buy(
        &mut self,
        quote_in: Decimal,
        price: Decimal,
        cost: &CostModel,
    ) -> Result<TradeOutcome, SimError> {
        if quote_in <= Decimal::ZERO {
            return Err(SimError::NonPositiveAmount);
        }
        if quote_in > self.quote_balance {
            return Err(SimError::UnaffordableBuy {
                needed: quote_in,
                available: self.quote_balance,
            });
        }
        let fill = cost.fill_buy(quote_in, price);
        let base_delta = fill.net_base - fill.gas_sol;
        let new_base = self.base_balance + base_delta;
        if new_base < Decimal::ZERO {
            // The SOL received does not even cover gas.
            return Err(SimError::InsufficientSolForGas {
                gas: fill.gas_sol,
                available: self.base_balance + fill.net_base,
            });
        }
        self.quote_balance -= quote_in;
        self.base_balance = new_base;
        Ok(TradeOutcome {
            side: Side::Buy,
            exec_price: price,
            base_delta,
            quote_delta: -quote_in,
            gas_sol: fill.gas_sol,
            dex_fee_quote: fill.dex_fee_quote,
            slippage_quote: fill.slippage_quote,
        })
    }

    /// Sell `base_in` SOL for USDC at mid `price`. Debits SOL (plus gas), credits USDC.
    pub fn apply_sell(
        &mut self,
        base_in: Decimal,
        price: Decimal,
        cost: &CostModel,
    ) -> Result<TradeOutcome, SimError> {
        if base_in <= Decimal::ZERO {
            return Err(SimError::NonPositiveAmount);
        }
        if base_in > self.base_balance {
            return Err(SimError::Oversell {
                requested: base_in,
                available: self.base_balance,
            });
        }
        let fill = cost.fill_sell(base_in, price);
        let total_base_out = base_in + fill.gas_sol;
        if total_base_out > self.base_balance {
            return Err(SimError::InsufficientSolForGas {
                gas: fill.gas_sol,
                available: self.base_balance - base_in,
            });
        }
        self.base_balance -= total_base_out;
        self.quote_balance += fill.net_quote;
        Ok(TradeOutcome {
            side: Side::Sell,
            exec_price: price,
            base_delta: -total_base_out,
            quote_delta: fill.net_quote,
            gas_sol: fill.gas_sol,
            dex_fee_quote: fill.dex_fee_quote,
            slippage_quote: fill.slippage_quote,
        })
    }
}

/// The realized effect of one applied trade (for round-trip and cost reporting).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeOutcome {
    pub side: Side,
    pub exec_price: Decimal,
    pub base_delta: Decimal,
    pub quote_delta: Decimal,
    pub gas_sol: Decimal,
    pub dex_fee_quote: Decimal,
    pub slippage_quote: Decimal,
}

/// Accounting / simulation errors. Every variant is a *refusal*, never a silent adjustment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimError {
    /// A trade amount was zero or negative.
    NonPositiveAmount,
    /// A buy would spend more USDC than is available.
    UnaffordableBuy { needed: Decimal, available: Decimal },
    /// A sell would dispose of more SOL than is held.
    Oversell {
        requested: Decimal,
        available: Decimal,
    },
    /// Not enough SOL remains to cover network gas.
    InsufficientSolForGas { gas: Decimal, available: Decimal },
    /// The simulator was given no bars.
    NoBars,
}

impl std::fmt::Display for SimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonPositiveAmount => write!(f, "trade amount must be positive"),
            Self::UnaffordableBuy { needed, available } => {
                write!(f, "unaffordable buy: need {needed} USDC, have {available}")
            }
            Self::Oversell {
                requested,
                available,
            } => {
                write!(f, "oversell: requested {requested} SOL, hold {available}")
            }
            Self::InsufficientSolForGas { gas, available } => {
                write!(f, "insufficient SOL for gas: need {gas}, have {available}")
            }
            Self::NoBars => write!(f, "simulator requires at least one bar"),
        }
    }
}

impl std::error::Error for SimError {}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn zero_cost() -> CostModel {
        CostModel::zero()
    }

    #[test]
    fn buy_debits_usdc_and_credits_sol() {
        let mut s = PortfolioState::new(dec!(1000));
        let out = s.apply_buy(dec!(1000), dec!(100), &zero_cost()).unwrap();
        assert_eq!(s.quote_balance, dec!(0));
        assert_eq!(s.base_balance, dec!(10));
        assert_eq!(out.quote_delta, dec!(-1000));
        assert_eq!(out.base_delta, dec!(10));
    }

    #[test]
    fn sell_debits_sol_and_credits_usdc() {
        let mut s = PortfolioState {
            quote_balance: dec!(0),
            base_balance: dec!(10),
        };
        s.apply_sell(dec!(10), dec!(100), &zero_cost()).unwrap();
        assert_eq!(s.base_balance, dec!(0));
        assert_eq!(s.quote_balance, dec!(1000));
    }

    #[test]
    fn unaffordable_buy_rejected() {
        let mut s = PortfolioState::new(dec!(500));
        assert_eq!(
            s.apply_buy(dec!(1000), dec!(100), &zero_cost())
                .unwrap_err(),
            SimError::UnaffordableBuy {
                needed: dec!(1000),
                available: dec!(500)
            }
        );
        // State unchanged after a rejected trade.
        assert_eq!(s.quote_balance, dec!(500));
        assert_eq!(s.base_balance, dec!(0));
    }

    #[test]
    fn oversell_impossible() {
        let mut s = PortfolioState {
            quote_balance: dec!(0),
            base_balance: dec!(5),
        };
        assert_eq!(
            s.apply_sell(dec!(10), dec!(100), &zero_cost()).unwrap_err(),
            SimError::Oversell {
                requested: dec!(10),
                available: dec!(5)
            }
        );
        assert_eq!(s.base_balance, dec!(5));
    }

    #[test]
    fn fees_reduce_equity() {
        let cost = CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5000,
            priority_fee_lamports: 50000,
        };
        let mut s = PortfolioState::new(dec!(1000));
        let before = s.equity(dec!(100));
        s.apply_buy(dec!(1000), dec!(100), &cost).unwrap();
        let after = s.equity(dec!(100));
        assert!(
            after < before,
            "equity should fall after a costed buy: {before} -> {after}"
        );
    }
}
