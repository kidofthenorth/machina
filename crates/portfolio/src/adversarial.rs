//! Adversarial (MEV / adverse-selection) execution terms, priced as COSTS TO US ONLY — never a
//! benefit (m-hf-track §3/§5 row C5): a taker fill can be **sandwiched** (an adversary front-/
//! back-runs it) and/or **picked off** (transacted against a stale price), each a loss of some
//! bps of the fill notional; τ (tau) is that worst-case per-fill loss. A resting **maker** order
//! fills only when the market provably trades THROUGH its price — a mere touch fills nothing.
//!
//! Two rungs, both deterministic by construction (no RNG, no clock — pure Decimal arithmetic):
//! - **AdversarialWorst** (m-hf-track §4's ladder rung): the full τ on EVERY taker fill (p = 1).
//! - **Base rung**: the EXPECTED adverse-selection term `p·τ`, `p` an exact rational — a
//!   non-zero everyday MEV cost a candidate cannot exclude from its base economics (§3).
//!
//! Carried in its own module (like C4's [`crate::hf_cost`]), NOT wired into `run`/`run_hf` here:
//! this card defines and proves the pure pricing/fill functions. Per-fill hash realization of
//! WHICH fills get targeted (the C3 `(cell_id, event_index)` splitmix64 primitive) and the
//! fail-closed `(regime, percentile) → p` table are C8's sweep-wiring job — exactly as C4
//! supplied only the priority-fee lookup and deferred the landing-percentile table to C8.

use crate::cost::Side;
use research_core::money::apply_bps;
use research_core::Decimal;
use std::fmt;

/// The adversarial execution terms for taker fills. Every loss is a COST TO US (non-negative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdversarialModel {
    /// Worst-case sandwich loss, bps of taker-fill notional (τ's sandwich component).
    pub sandwich_bps: u32,
    /// Worst-case pickoff / adverse-selection loss, bps of notional (τ's pickoff component).
    pub pickoff_bps: u32,
    /// Base-rung adverse-event probability as an EXACT rational `num/den` (`den ≥ 1`,
    /// `num ≤ den`) — the fail-closed table value (m-hf-track §3). The full
    /// `(regime, percentile) → p` table is C8's job. An invalid rational is a pricing error,
    /// never a silent default.
    pub p_adverse_num: u64,
    pub p_adverse_den: u64,
}

/// Errors pricing adversarial terms. Fail-closed: never a silent default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdversarialError {
    /// The adverse-event probability is not a valid rational in [0,1] (`den ≥ 1`, `num ≤ den`).
    BadProbability { num: u64, den: u64 },
}

impl fmt::Display for AdversarialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadProbability { num, den } => write!(
                f,
                "adverse-selection probability {num}/{den} is not a valid rational in [0,1]"
            ),
        }
    }
}

impl std::error::Error for AdversarialError {}

/// The full worst-case adverse-selection loss τ for a taker fill of `notional_quote` (USDC):
/// sandwich + pickoff, BOTH on EVERY fill (m-hf-track §4's `AdversarialWorst` rung reproduces
/// the τ-loss on every taker fill). Pessimistic by construction — a cost to us, never a benefit.
/// Pure: no RNG, no clock, no side branching.
#[must_use]
pub fn adverse_selection_cost_worst(model: &AdversarialModel, notional_quote: Decimal) -> Decimal {
    apply_bps(notional_quote, model.sandwich_bps) + apply_bps(notional_quote, model.pickoff_bps)
}

/// The base-rung EXPECTED adverse-selection term `p·τ`, exact Decimal (m-hf-track §3 — the
/// non-zero everyday MEV cost). `p = p_adverse_num / p_adverse_den`. Fail-closed on a bad
/// rational. Always in `[0, τ]` (a cost to us, never a benefit; `p = 1` ⇒ the worst rung,
/// `p = 0` ⇒ zero).
pub fn adverse_selection_cost_expected(
    model: &AdversarialModel,
    notional_quote: Decimal,
) -> Result<Decimal, AdversarialError> {
    if model.p_adverse_den == 0 || model.p_adverse_num > model.p_adverse_den {
        return Err(AdversarialError::BadProbability {
            num: model.p_adverse_num,
            den: model.p_adverse_den,
        });
    }
    let tau = adverse_selection_cost_worst(model, notional_quote);
    Ok(tau * Decimal::from(model.p_adverse_num) / Decimal::from(model.p_adverse_den))
}

/// One resting maker (limit) order awaiting a trade-through fill. `Side::Buy` is a resting bid,
/// `Side::Sell` a resting ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MakerOrder {
    pub side: Side,
    pub limit_price: Decimal,
    pub base_size: Decimal,
}

/// The outcome of a maker order over one bar. `filled_base == 0` means NO fill — either the
/// market never reached the limit, or it only touched it without trading through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MakerFill {
    pub filled_base: Decimal,
    pub traded_through: bool,
}

/// Trade-through-only maker fill (m-hf-track §3/§5 row C5): a resting **bid** fills only if the
/// bar's low prints STRICTLY through it (`bar_low < limit_price`); a resting **ask** only if the
/// bar's high prints strictly through it (`bar_high > limit_price`). A mere touch
/// (`bar_low == bid` / `bar_high == ask`) fills NOTHING — we refuse an optimistic fill we could
/// not guarantee from queue position. Filled size is capped by the printed `volume` (base terms).
/// Pure: no RNG, no clock.
#[must_use]
pub fn maker_trade_through_fill(
    order: &MakerOrder,
    bar_low: Decimal,
    bar_high: Decimal,
    volume: Decimal,
) -> MakerFill {
    let traded_through = match order.side {
        Side::Buy => bar_low < order.limit_price,
        Side::Sell => bar_high > order.limit_price,
    };
    let filled_base = if traded_through {
        order.base_size.min(volume).max(Decimal::ZERO)
    } else {
        Decimal::ZERO
    };
    MakerFill {
        filled_base,
        traded_through,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn reference_model() -> AdversarialModel {
        AdversarialModel {
            sandwich_bps: 30,
            pickoff_bps: 10,
            p_adverse_num: 5,
            p_adverse_den: 100,
        }
    }

    #[test]
    fn adverse_selection_worst_reproduces_full_tau_on_every_fill() {
        let m = reference_model();
        // τ = sandwich(30bps) + pickoff(10bps), on EVERY fill, independent of p.
        // @1000: 500*... → 1000*30/10_000 = 3, 1000*10/10_000 = 1 → 4.
        assert_eq!(adverse_selection_cost_worst(&m, dec!(1000)), dec!(4));
        // @2000: 6 + 2 = 8 — scales with notional, still ignores p.
        assert_eq!(adverse_selection_cost_worst(&m, dec!(2000)), dec!(8));
    }

    #[test]
    fn adverse_selection_expected_is_p_times_tau() {
        let m = reference_model();
        // p = 5/100, τ@1000 = 4 → expected = 4 * 5/100 = 0.2 (exact Decimal).
        assert_eq!(
            adverse_selection_cost_expected(&m, dec!(1000)).unwrap(),
            dec!(0.2)
        );
    }

    #[test]
    fn adverse_selection_expected_boundary_probabilities() {
        let expected_at = |num, den| {
            adverse_selection_cost_expected(
                &AdversarialModel {
                    sandwich_bps: 30,
                    pickoff_bps: 10,
                    p_adverse_num: num,
                    p_adverse_den: den,
                },
                dec!(1000),
            )
            .unwrap()
        };
        assert_eq!(expected_at(0, 1), dec!(0)); // p = 0 → the base term vanishes
        assert_eq!(expected_at(1, 2), dec!(2)); // p = 1/2 → half of τ (=4)
        assert_eq!(expected_at(1, 1), dec!(4)); // p = 1 → equals the worst rung (τ)
    }

    #[test]
    fn adverse_selection_expected_rejects_bad_probability() {
        // den = 0 and num > den are both fail-closed errors (mirrors C3's build_landing_table).
        let bad_den = AdversarialModel {
            sandwich_bps: 30,
            pickoff_bps: 10,
            p_adverse_num: 1,
            p_adverse_den: 0,
        };
        assert!(matches!(
            adverse_selection_cost_expected(&bad_den, dec!(1000)),
            Err(AdversarialError::BadProbability { .. })
        ));
        let num_gt_den = AdversarialModel {
            sandwich_bps: 30,
            pickoff_bps: 10,
            p_adverse_num: 3,
            p_adverse_den: 2,
        };
        assert!(matches!(
            adverse_selection_cost_expected(&num_gt_den, dec!(1000)),
            Err(AdversarialError::BadProbability { .. })
        ));
    }

    #[test]
    fn adversarial_terms_are_costs_never_benefits() {
        let m = reference_model();
        let worst = adverse_selection_cost_worst(&m, dec!(1000));
        let expected = adverse_selection_cost_expected(&m, dec!(1000)).unwrap();
        assert!(worst >= dec!(0));
        assert!(expected >= dec!(0));
        assert!(
            expected <= worst,
            "expected {expected} must never exceed worst {worst}"
        );
    }

    #[test]
    fn maker_bid_fills_only_on_trade_through() {
        let bid = MakerOrder {
            side: Side::Buy,
            limit_price: dec!(100),
            base_size: dec!(5),
        };
        // Trade-through: low 99 < bid 100 → filled, capped at volume (min(5, 10) = 5).
        assert_eq!(
            maker_trade_through_fill(&bid, dec!(99), dec!(101), dec!(10)),
            MakerFill {
                filled_base: dec!(5),
                traded_through: true
            }
        );
        // Touch (low == bid) → NO fill (the discriminating assertion).
        assert_eq!(
            maker_trade_through_fill(&bid, dec!(100), dec!(101), dec!(10)),
            MakerFill {
                filled_base: dec!(0),
                traded_through: false
            }
        );
        // Never reached (low 101 > bid) → NO fill.
        assert_eq!(
            maker_trade_through_fill(&bid, dec!(101), dec!(103), dec!(10)),
            MakerFill {
                filled_base: dec!(0),
                traded_through: false
            }
        );
    }

    #[test]
    fn maker_ask_fills_only_on_trade_through() {
        let ask = MakerOrder {
            side: Side::Sell,
            limit_price: dec!(100),
            base_size: dec!(5),
        };
        // Trade-through: high 101 > ask 100 → filled (min(5, 10) = 5).
        assert_eq!(
            maker_trade_through_fill(&ask, dec!(99), dec!(101), dec!(10)),
            MakerFill {
                filled_base: dec!(5),
                traded_through: true
            }
        );
        // Touch (high == ask) → NO fill.
        assert_eq!(
            maker_trade_through_fill(&ask, dec!(99), dec!(100), dec!(10)),
            MakerFill {
                filled_base: dec!(0),
                traded_through: false
            }
        );
    }

    #[test]
    fn maker_fill_is_capped_by_printed_volume() {
        let bid = MakerOrder {
            side: Side::Buy,
            limit_price: dec!(100),
            base_size: dec!(50),
        };
        // Wants 50 SOL, only 8 printed through → filled 8 (the size cap).
        assert_eq!(
            maker_trade_through_fill(&bid, dec!(99), dec!(101), dec!(8)),
            MakerFill {
                filled_base: dec!(8),
                traded_through: true
            }
        );
    }

    #[test]
    fn repeated_calls_are_deterministic() {
        let m = reference_model();
        assert_eq!(
            adverse_selection_cost_worst(&m, dec!(1000)),
            adverse_selection_cost_worst(&m, dec!(1000))
        );
        assert_eq!(
            adverse_selection_cost_expected(&m, dec!(1000)),
            adverse_selection_cost_expected(&m, dec!(1000))
        );
        let bid = MakerOrder {
            side: Side::Buy,
            limit_price: dec!(100),
            base_size: dec!(5),
        };
        assert_eq!(
            maker_trade_through_fill(&bid, dec!(99), dec!(101), dec!(10)),
            maker_trade_through_fill(&bid, dec!(99), dec!(101), dec!(10))
        );
    }
}
