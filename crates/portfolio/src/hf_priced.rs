//! Priced HF execution entry point (m-hf-track §3/§5 row C8.6b): wires [`crate::hf_cost::HfCostModel`]
//! and [`crate::adversarial::AdversarialModel`] into a `run_hf`-shaped loop.
//!
//! A pure structural sibling of [`crate::latency::run_hf`], not a wrapper around it:
//! `run_hf`/`simulator::run` are not edited, called, or depended on here (this card's guardrail).
//! `PortfolioState::apply_buy`/`apply_sell` and `CostModel::fill_buy`/`fill_sell` are reused
//! completely unchanged — each trade synthesizes a per-trade `CostModel` whose `dex_fee_bps` folds
//! in the venue fee, depth-walk impact, tip, and any realized adverse-selection hit (all bps-of-
//! notional taken from the fill's output side, `apply_bps`'s linearity making the combined single
//! deduction exactly equal to summing each term separately — the same math `hf_trade_cost` uses).
//! `CostModel.slippage_bps` is always `0` on this path: HF pricing never uses `CostModel`'s own
//! price-shift mechanism.

use crate::adversarial::AdversarialModel;
use crate::cost::CostModel;
use crate::equity::EquityPoint;
use crate::hf_cost::{CongestionRegime, HfCostModel};
use crate::latency::{
    clamp01, current_weight, dust, min_trade_notional, record_round_trip, traded_notional, HfError,
    LatencyPipeline, OpenPosition,
};
use crate::simulator::RunOutput;
use crate::state::{PortfolioState, SimError, TradeOutcome};
use research_core::money::lamports_to_sol;
use research_core::{Bar, Decimal};

/// SplitMix64 — deterministic integer hash sequence, NOT an entropy source. Own private copy
/// (per the project's established duplicate-don't-share convention for this primitive, matching
/// `latency.rs`'s own copy of `market-data/src/synthetic.rs`'s generator).
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Distinguishing tag XORed into `event_index` before the second mix, so the adverse-selection
/// draw stream is independent of the landing draw stream even for the same `(cell_id, event_index)`.
const ADVERSE_SELECTION_TAG: u64 = 0xA0DE_5E1E_C7A6;

/// One adverse-selection draw — a pure function of `(cell_id, event_index)` and NOTHING else,
/// keyed distinctly from `landing_draw` via [`ADVERSE_SELECTION_TAG`].
fn adverse_draw(cell_id: u64, event_index: u64) -> u64 {
    let mut state = cell_id;
    let mixed_cell = splitmix64(&mut state);
    let mut keyed = mixed_cell ^ (event_index ^ ADVERSE_SELECTION_TAG);
    splitmix64(&mut keyed)
}

/// Whether the fill at `event_index` draws an adverse-selection hit under `model`'s flat
/// base-rung probability (`p = p_adverse_num / p_adverse_den`). Realizes the base rung by draw,
/// not by a continuous per-trade haircut: over many draws this converges to `p_adverse_num /
/// p_adverse_den * (sandwich_bps + pickoff_bps)`, matching
/// [`crate::adversarial::adverse_selection_cost_expected`]'s value. That pure reference function
/// (and `adverse_selection_cost_worst`) is NOT called from this execution path — it is test-only,
/// referenced here only in this doc comment so a future reader doesn't mistake the unused import
/// for dead code.
fn adverse_selection_hit(cell_id: u64, event_index: u64, model: &AdversarialModel) -> bool {
    (adverse_draw(cell_id, event_index) % model.p_adverse_den) < model.p_adverse_num
}

/// Everything one priced HF run produces: the base HF accounting plus adverse-selection counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfPricedRunOutput {
    pub base: crate::latency::HfRunOutput,
    /// Number of landed fills that drew an adverse-selection hit.
    pub adverse_selection_hits: u32,
    /// Total adverse-selection cost paid (sandwich + pickoff bps of notional), in USDC.
    pub adverse_selection_paid_quote: Decimal,
}

/// Move the portfolio toward `target` SOL weight at `price`, pricing the trade with `hf`'s
/// depth-walk/gas terms plus `extra_bps` (caller-computed: `hf.tip_bps` + adverse-selection bps
/// if this fill drew a hit). Structural duplicate of [`crate::latency::rebalance`] with the cost
/// source swapped for a synthesized `CostModel`.
#[allow(clippy::too_many_arguments)]
fn rebalance_priced(
    state: &mut PortfolioState,
    target: Decimal,
    price: Decimal,
    hf: &HfCostModel,
    regime: CongestionRegime,
    extra_bps: u32,
) -> Result<Option<TradeOutcome>, SimError> {
    let equity = state.equity(price);
    if equity <= Decimal::ZERO || price <= Decimal::ZERO {
        return Ok(None);
    }
    let desired_base = (target * equity) / price;
    let delta = desired_base - state.base_balance;
    if (delta * price).abs() < min_trade_notional() {
        return Ok(None);
    }

    let priority_lamports = hf.congestion_priority_table.priority_lamports_for(regime);
    let gas_only = CostModel {
        dex_fee_bps: 0,
        slippage_bps: 0,
        base_fee_lamports: hf.base.base_fee_lamports,
        priority_fee_lamports: priority_lamports,
    };

    if delta > Decimal::ZERO {
        let quote_in = (delta * price).min(state.quote_balance);
        if quote_in <= Decimal::ZERO {
            return Ok(None);
        }
        let cost = CostModel {
            dex_fee_bps: hf.base.dex_fee_bps + hf.depth_curve.impact_bps_for(quote_in) + extra_bps,
            slippage_bps: 0,
            base_fee_lamports: hf.base.base_fee_lamports,
            priority_fee_lamports: priority_lamports,
        };
        match state.apply_buy(quote_in, price, &cost) {
            Ok(o) => Ok(Some(o)),
            Err(SimError::InsufficientSolForGas { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    } else {
        let sellable = state.base_balance - gas_only.gas_sol();
        if sellable <= Decimal::ZERO {
            return Ok(None);
        }
        let base_in = (-delta).min(sellable);
        if base_in <= Decimal::ZERO {
            return Ok(None);
        }
        let notional = base_in * price;
        let cost = CostModel {
            dex_fee_bps: hf.base.dex_fee_bps + hf.depth_curve.impact_bps_for(notional) + extra_bps,
            slippage_bps: 0,
            base_fee_lamports: hf.base.base_fee_lamports,
            priority_fee_lamports: priority_lamports,
        };
        match state.apply_sell(base_in, price, &cost) {
            Ok(o) => Ok(Some(o)),
            Err(SimError::InsufficientSolForGas { .. } | SimError::Oversell { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Run the deterministic, priced latency-aware simulation: identical event ordering to
/// [`crate::latency::run_hf`] (signal `t` decided from `bars[..=t]`, executes at
/// `t + landing[t].offset_bars`'s open), but each landed fill is priced through `hf` +
/// `adversarial` instead of a flat `CostModel`.
///
/// `regimes.len()` must equal `bars.len()`. `cell_id` is the SAME key the caller used to build
/// `pipeline` via `build_landing_table` — `LatencyPipeline` does not store it.
#[allow(clippy::too_many_arguments)]
pub fn run_hf_priced<F>(
    bars: &[Bar],
    initial_cash_usdc: Decimal,
    hf: &HfCostModel,
    adversarial: &AdversarialModel,
    pipeline: &LatencyPipeline,
    regimes: &[CongestionRegime],
    cell_id: u64,
    mut target_fn: F,
) -> Result<HfPricedRunOutput, HfError>
where
    F: FnMut(&[Bar], Decimal) -> Decimal,
{
    if bars.is_empty() {
        return Err(HfError::Sim(SimError::NoBars));
    }
    if pipeline.landing().len() != bars.len() {
        return Err(HfError::PipelineLength {
            landing: pipeline.landing().len(),
            bars: bars.len(),
        });
    }
    if regimes.len() != bars.len() {
        return Err(HfError::RegimesLength {
            regimes: regimes.len(),
            bars: bars.len(),
        });
    }

    let mut land_at: Vec<Vec<usize>> = vec![Vec::new(); bars.len()];
    let mut unlanded_orders = 0u32;
    for (t, outcome) in pipeline.landing().iter().enumerate() {
        match t.checked_add(outcome.offset_bars) {
            Some(l) if l < bars.len() => {
                if outcome.landed {
                    land_at[l].push(t);
                } else {
                    unlanded_orders += 1;
                }
            }
            _ => {}
        }
    }

    let mut state = PortfolioState::new(initial_cash_usdc);
    let mut equity_curve = Vec::with_capacity(bars.len());
    let mut round_trips = Vec::new();
    let mut open: Option<OpenPosition> = None;
    let mut n_trades = 0u32;
    let mut traded_notional_quote = Decimal::ZERO;
    let mut fees_paid_quote = Decimal::ZERO;
    let mut gas_paid_sol = Decimal::ZERO;
    let mut priority_fees_paid_sol = Decimal::ZERO;
    let mut bars_in_market = 0u32;
    let mut adverse_selection_hits = 0u32;
    let mut adverse_selection_paid_quote = Decimal::ZERO;

    for (l, bar) in bars.iter().enumerate() {
        let regime = regimes[l];
        for &t in &land_at[l] {
            let exec_price = bar.open;
            let weight_now = current_weight(&state, exec_price);
            let target = clamp01(target_fn(&bars[..t + 1], weight_now));
            let was_flat = state.base_balance <= dust();
            let quote_before = state.quote_balance;

            let hit = adverse_selection_hit(cell_id, t as u64, adversarial);
            let extra_bps = hf.tip_bps
                + if hit {
                    adversarial.sandwich_bps + adversarial.pickoff_bps
                } else {
                    0
                };

            if let Some(outcome) =
                rebalance_priced(&mut state, target, exec_price, hf, regime, extra_bps)?
            {
                n_trades += 1;
                traded_notional_quote += traded_notional(&outcome);
                fees_paid_quote += outcome.dex_fee_quote;
                gas_paid_sol += outcome.gas_sol;
                let priority_lamports = hf.congestion_priority_table.priority_lamports_for(regime);
                priority_fees_paid_sol += lamports_to_sol(priority_lamports);
                if hit {
                    adverse_selection_hits += 1;
                    let notional = traded_notional(&outcome);
                    let adverse_bps = adversarial.sandwich_bps + adversarial.pickoff_bps;
                    adverse_selection_paid_quote +=
                        research_core::money::apply_bps(notional, adverse_bps);
                }
                record_round_trip(
                    &mut open,
                    &mut round_trips,
                    &state,
                    &outcome,
                    bar.ts,
                    was_flat,
                    quote_before,
                );
            }
        }
        equity_curve.push(EquityPoint {
            ts: bar.ts,
            equity_quote: state.equity(bar.close),
        });
        if state.base_balance > dust() {
            bars_in_market += 1;
        }
    }

    Ok(HfPricedRunOutput {
        base: crate::latency::HfRunOutput {
            base: RunOutput {
                equity_curve,
                round_trips,
                final_state: state,
                n_trades,
                traded_notional_quote,
                fees_paid_quote,
                slippage_paid_quote: Decimal::ZERO,
                gas_paid_sol,
                priority_fees_paid_sol,
                bars_in_market,
            },
            unlanded_orders,
        },
        adverse_selection_hits,
        adverse_selection_paid_quote,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hf_cost::{hf_trade_cost, CongestionPriorityTable, DepthBand, DepthCurve};
    use research_core::Timestamp;
    use rust_decimal_macros::dec;

    fn series(prices: &[i64]) -> Vec<Bar> {
        prices
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let open = Decimal::from(*p);
                Bar {
                    ts: Timestamp::from_unix(1_700_000_000 + i as i64),
                    open,
                    high: open + dec!(3),
                    low: open - dec!(1),
                    close: open + dec!(2),
                    volume: dec!(1),
                }
            })
            .collect()
    }

    /// Landing-vs-adverse-draw independence proof (mirrors
    /// `landing_table_identical_across_thread_counts`'s scale, `0..100_000`): the two draw
    /// streams are keyed distinctly (`ADVERSE_SELECTION_TAG`), so `landed == adverse_hit` must
    /// occur neither ~0% nor ~100% of the time. Lives here (not the integration test file)
    /// because `adverse_selection_hit` is module-private, matching `adverse_draw`/
    /// `ADVERSE_SELECTION_TAG`'s scoping in the card's file-1 bullet.
    #[test]
    fn landing_and_adverse_draws_are_independent_over_100k() {
        const N: u64 = 100_000;
        let cell_id = 0x00C0_FFEE_u64;
        let landing = crate::latency::build_landing_table(cell_id, 0..N, 3, 1, 2).unwrap();
        let adversarial = AdversarialModel {
            sandwich_bps: 1,
            pickoff_bps: 1,
            p_adverse_num: 1,
            p_adverse_den: 2,
        };
        let mut agree = 0u64;
        for (i, outcome) in landing.iter().enumerate() {
            let hit = adverse_selection_hit(cell_id, i as u64, &adversarial);
            if outcome.landed == hit {
                agree += 1;
            }
        }
        let agree_frac = agree as f64 / N as f64;
        assert!(
            agree_frac > 0.05 && agree_frac < 0.95,
            "agree_frac={agree_frac}"
        );
    }

    #[test]
    fn adverse_draw_is_pure_and_keyed_by_cell() {
        let a: Vec<u64> = (0..1000).map(|i| adverse_draw(1, i)).collect();
        let b: Vec<u64> = (0..1000).map(|i| adverse_draw(1, i)).collect();
        assert_eq!(a, b);
        let other_cell: Vec<u64> = (0..1000).map(|i| adverse_draw(2, i)).collect();
        assert_ne!(a, other_cell);
    }

    #[test]
    fn probability_boundaries_are_legal_and_exact() {
        let none = AdversarialModel {
            sandwich_bps: 0,
            pickoff_bps: 0,
            p_adverse_num: 0,
            p_adverse_den: 10,
        };
        assert!((0..100).all(|i| !adverse_selection_hit(1, i, &none)));
        let all = AdversarialModel {
            sandwich_bps: 0,
            pickoff_bps: 0,
            p_adverse_num: 10,
            p_adverse_den: 10,
        };
        assert!((0..100).all(|i| adverse_selection_hit(1, i, &all)));
    }

    #[test]
    fn single_trade_matches_hf_trade_cost_exactly() {
        // Hand-computed cross-check: the synthesized-CostModel realized total for one buy must
        // match hf_trade_cost(...).total_quote to EXACT Decimal equality for the same
        // notional/price/regime.
        let hf = HfCostModel {
            base: CostModel {
                dex_fee_bps: 30,
                slippage_bps: 0,
                base_fee_lamports: 5_000,
                priority_fee_lamports: 0,
            },
            depth_curve: DepthCurve::new(vec![DepthBand {
                notional_upto: dec!(1_000_000),
                impact_bps: 15,
            }])
            .unwrap(),
            congestion_priority_table: CongestionPriorityTable::new(1_000, 2_000, 3_000).unwrap(),
            tip_bps: 5,
        };
        let adversarial = AdversarialModel {
            sandwich_bps: 0,
            pickoff_bps: 0,
            p_adverse_num: 0,
            p_adverse_den: 1,
        };
        let regime = CongestionRegime::Busy;
        let price = dec!(100);
        let bars = series(&[100, 100]);
        let pipeline = crate::latency::LatencyPipeline::new(
            1,
            vec![
                crate::latency::LandingOutcome {
                    offset_bars: 1,
                    landed: true,
                },
                crate::latency::LandingOutcome {
                    offset_bars: 1,
                    landed: false,
                },
            ],
        )
        .unwrap();
        let regimes = vec![CongestionRegime::Calm, regime];

        // target_fn drives a single full buy at bar 1's open (100).
        let out = run_hf_priced(
            &bars,
            dec!(10_000),
            &hf,
            &adversarial,
            &pipeline,
            &regimes,
            1,
            |_h, _w| dec!(1),
        )
        .unwrap();
        assert_eq!(out.base.base.n_trades, 1);

        let quote_in = dec!(10_000); // full cash spent on the single buy
        let expected = hf_trade_cost(&hf, quote_in, price, regime);
        // Realized total = dex_fee_quote (folds venue fee + depth impact + tip, no adverse hit)
        // + gas leg valued at price.
        let realized_dex_fee = out.base.base.fees_paid_quote;
        let realized_gas_quote = out.base.base.gas_paid_sol * price;
        let realized_total = realized_dex_fee + realized_gas_quote;
        assert_eq!(realized_total, expected.total_quote);
    }
}
