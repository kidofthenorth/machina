//! Latency-aware HF entry point (m-hf-track §3): `run_hf` generalizes the simulator loop
//! WITHOUT touching `simulator::run` — next-bar execution is the special case
//! `fixed_latency(1)`, proven equivalent by regression (tests/hf_regression.rs).
//!
//! Event ordering: the signal decided from completed bars `[0..=t]` lands at bar
//! `t + offset_bars` (offset ≥ min_latency ≥ 1) and executes at that bar's OPEN — market
//! state at landing, never at signal. Un-landed orders change no balances; their attempted
//! count is recorded (the attempted-cost term is priced by the C4 cost fields). Latency is
//! measured in BARS (slots at 1s resolution), never wall-clock. Landing outcomes are
//! deterministic by construction: a splitmix64 hash stream keyed by `(cell_id,
//! event_index)` — never RNG, thread id, evaluation order, or clock.

use crate::cost::{CostModel, Side};
use crate::equity::{EquityPoint, RoundTrip};
use crate::simulator::RunOutput;
use crate::state::{PortfolioState, SimError, TradeOutcome};
use research_core::{Bar, Decimal, Timestamp};
use std::fmt;

/// Landing outcome for one signal index; `offset_bars` ≥ the pipeline's `min_latency`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandingOutcome {
    pub offset_bars: usize,
    pub landed: bool,
}

/// Deterministic latency pipeline: one [`LandingOutcome`] per signal index (m-hf-track §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatencyPipeline {
    min_latency: usize,
    landing: Vec<LandingOutcome>,
}

impl LatencyPipeline {
    /// `min_latency` ≥ 1 always: the same-slot-at-signal-price mode is NOT built here —
    /// it may only ever exist as an explicitly labeled upper-bound scenario (§3).
    pub fn new(min_latency: usize, landing: Vec<LandingOutcome>) -> Result<Self, HfError> {
        if min_latency == 0 {
            return Err(HfError::ZeroMinLatency);
        }
        if let Some(index) = landing.iter().position(|l| l.offset_bars < min_latency) {
            return Err(HfError::OffsetBelowMin { index });
        }
        Ok(Self {
            min_latency,
            landing,
        })
    }

    #[must_use]
    pub fn min_latency(&self) -> usize {
        self.min_latency
    }

    #[must_use]
    pub fn landing(&self) -> &[LandingOutcome] {
        &self.landing
    }
}

/// Every signal lands, `offset_bars` later — `fixed_latency(1, n)` IS next-bar execution.
#[must_use]
pub fn fixed_latency(offset_bars: usize, n_signals: usize) -> LatencyPipeline {
    assert!(
        offset_bars >= 1,
        "same-slot execution is not modeled (m-hf-track §3)"
    );
    LatencyPipeline {
        min_latency: offset_bars,
        landing: vec![
            LandingOutcome {
                offset_bars,
                landed: true
            };
            n_signals
        ],
    }
}

/// Errors from the HF entry point. `SimError` stays untouched (simulator.rs is read-only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HfError {
    /// Underlying simulation/accounting error.
    Sim(SimError),
    /// One landing outcome per potential signal index: `landing.len() == bars.len()`.
    PipelineLength { landing: usize, bars: usize },
    /// `min_latency` must be ≥ 1 (no same-slot execution).
    ZeroMinLatency,
    /// `landing[index].offset_bars` < `min_latency`.
    OffsetBelowMin { index: usize },
    /// Landing probability must be an exact rational with `den ≥ 1` and `num ≤ den`.
    BadProbability { num: u64, den: u64 },
    /// One [`crate::hf_cost::CongestionRegime`] per bar: `regimes.len() == bars.len()`.
    RegimesLength { regimes: usize, bars: usize },
}

impl fmt::Display for HfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sim(e) => write!(f, "simulation error: {e}"),
            Self::PipelineLength { landing, bars } => {
                write!(f, "pipeline has {landing} landing outcomes for {bars} bars")
            }
            Self::ZeroMinLatency => write!(f, "min_latency must be >= 1"),
            Self::OffsetBelowMin { index } => {
                write!(f, "landing[{index}].offset_bars is below min_latency")
            }
            Self::BadProbability { num, den } => {
                write!(
                    f,
                    "landing probability {num}/{den} is not a valid rational in [0,1]"
                )
            }
            Self::RegimesLength { regimes, bars } => {
                write!(f, "regimes has {regimes} entries for {bars} bars")
            }
        }
    }
}

impl std::error::Error for HfError {}

impl From<SimError> for HfError {
    fn from(e: SimError) -> Self {
        Self::Sim(e)
    }
}

/// SplitMix64 — a deterministic integer hash sequence, NOT an entropy source. Verbatim
/// twin of the C2 generator's private fn (market-data/src/synthetic.rs:127-133),
/// duplicated deliberately (pattern-level reuse, m-hf-track §1): a shared home would
/// couple portfolio to market-data for seven lines of integer arithmetic.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One landing draw — a pure function of `(cell_id, event_index)` and NOTHING else:
/// provably independent of thread id, evaluation order, and wall clock (m-hf-track §3).
fn landing_draw(cell_id: u64, event_index: u64) -> u64 {
    let mut state = cell_id;
    let mixed_cell = splitmix64(&mut state);
    let mut keyed = mixed_cell ^ event_index;
    splitmix64(&mut keyed)
}

/// Probability without RNG: `landed ⇔ draw % den < num`, `p = num/den` exact (§3).
/// Fail-closed: an invalid rational is an error, never a default.
///
/// `event_indices` are the stable, GLOBAL, fixture-relative indices, computed once before
/// windowing — never window-local (else one physical slot's outcome could differ per
/// cell). C8 is responsible for passing fixture-global ranges; the signature makes
/// window-local indexing impossible to do silently.
pub fn build_landing_table(
    cell_id: u64,
    event_indices: std::ops::Range<u64>,
    offset_bars: usize,
    p_land_num: u64,
    p_land_den: u64,
) -> Result<Vec<LandingOutcome>, HfError> {
    if p_land_den == 0 || p_land_num > p_land_den {
        return Err(HfError::BadProbability {
            num: p_land_num,
            den: p_land_den,
        });
    }
    Ok(event_indices
        .map(|event_index| LandingOutcome {
            offset_bars,
            landed: (landing_draw(cell_id, event_index) % p_land_den) < p_land_num,
        })
        .collect())
}

// ── Verbatim twins of simulator.rs's private accounting helpers ──────────────────────
// simulator.rs is READ-ONLY by design (m-hf-track §1: "without touching run"), so its
// private helpers are duplicated here rather than made pub(crate). The fixed_latency(1)
// equality regression (tests/hf_regression.rs) pins the two copies together: if either
// copy drifts, that gate breaks.

/// Below this SOL amount the position is treated as flat.
pub(crate) fn dust() -> Decimal {
    Decimal::new(1, 9) // 1e-9 SOL
}

/// Skip rebalances whose notional is below this (USDC) to avoid sub-cent churn.
pub(crate) fn min_trade_notional() -> Decimal {
    Decimal::new(1, 2) // 0.01 USDC
}

pub(crate) struct OpenPosition {
    entry_ts: Timestamp,
    entry_price: Decimal,
    qty_base: Decimal,
    quote_at_open: Decimal,
}

/// Move the portfolio toward `target` SOL weight at `price`. Returns the executed trade, if any.
fn rebalance(
    state: &mut PortfolioState,
    target: Decimal,
    price: Decimal,
    cost: &CostModel,
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

    if delta > Decimal::ZERO {
        // Buy: spend the smaller of the gap notional and available cash.
        let quote_in = (delta * price).min(state.quote_balance);
        if quote_in <= Decimal::ZERO {
            return Ok(None);
        }
        match state.apply_buy(quote_in, price, cost) {
            Ok(o) => Ok(Some(o)),
            // Trade too small to cover gas — skip rather than fail the run.
            Err(SimError::InsufficientSolForGas { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    } else {
        // Sell: dispose of the gap, but keep enough SOL to pay gas.
        let sellable = state.base_balance - cost.gas_sol();
        if sellable <= Decimal::ZERO {
            return Ok(None);
        }
        let base_in = (-delta).min(sellable);
        if base_in <= Decimal::ZERO {
            return Ok(None);
        }
        match state.apply_sell(base_in, price, cost) {
            Ok(o) => Ok(Some(o)),
            Err(SimError::InsufficientSolForGas { .. } | SimError::Oversell { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_round_trip(
    open: &mut Option<OpenPosition>,
    round_trips: &mut Vec<RoundTrip>,
    state: &PortfolioState,
    outcome: &TradeOutcome,
    ts: Timestamp,
    was_flat: bool,
    quote_before: Decimal,
) {
    let now_long = state.base_balance > dust();
    match outcome.side {
        Side::Buy if was_flat && now_long => {
            *open = Some(OpenPosition {
                entry_ts: ts,
                entry_price: outcome.exec_price,
                qty_base: state.base_balance,
                quote_at_open: quote_before,
            });
        }
        Side::Sell if !now_long => {
            if let Some(op) = open.take() {
                round_trips.push(RoundTrip {
                    entry_ts: op.entry_ts,
                    exit_ts: ts,
                    entry_price: op.entry_price,
                    exit_price: outcome.exec_price,
                    qty_base: op.qty_base,
                    // Exact realized P&L: cash at close − cash at open (includes all costs).
                    pnl_quote: state.quote_balance - op.quote_at_open,
                });
            }
        }
        _ => {}
    }
}

pub(crate) fn clamp01(w: Decimal) -> Decimal {
    w.max(Decimal::ZERO).min(Decimal::ONE)
}

/// USDC notional moved by one trade, at mid price, excluding the gas leg.
///
/// Buy: the exact USDC spent (`-quote_delta == quote_in`). Sell: the mid value of the SOL disposed;
/// `base_delta` on a sell is `-(base_in + gas_sol)`, so `base_in = |base_delta| - gas_sol` and the
/// mid notional is `(|base_delta| - gas_sol) * exec_price`. Both are exact `Decimal`.
pub(crate) fn traded_notional(outcome: &TradeOutcome) -> Decimal {
    match outcome.side {
        Side::Buy => -outcome.quote_delta,
        Side::Sell => (outcome.base_delta.abs() - outcome.gas_sol) * outcome.exec_price,
    }
}

/// Current SOL weight = base value / equity at `price`. Zero when flat or equity is non-positive.
pub(crate) fn current_weight(state: &PortfolioState, price: Decimal) -> Decimal {
    let equity = state.equity(price);
    if equity <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    (state.base_balance * price) / equity
}

/// Everything one HF run produces: the base accounting plus HF-only counters. With
/// `fixed_latency(1)`, `base` equals `run`'s output exactly (the regression gate) and
/// the HF counters are zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfRunOutput {
    pub base: RunOutput,
    /// In-range attempts that did not land (`landed == false`). Balances untouched; the
    /// attempted COST (a cost to us, never a benefit) is priced when C4's fields land.
    pub unlanded_orders: u32,
}

/// Run the deterministic latency-aware simulation (m-hf-track §3).
///
/// The signal decided from completed bars `[0..=t]` executes at bar
/// `t + landing[t].offset_bars`'s OPEN — market state at landing, never at signal.
pub fn run_hf<F>(
    bars: &[Bar],
    initial_cash_usdc: Decimal,
    cost: &CostModel,
    pipeline: &LatencyPipeline,
    mut target_fn: F,
) -> Result<HfRunOutput, HfError>
where
    F: FnMut(&[Bar], Decimal) -> Decimal,
{
    if bars.is_empty() {
        return Err(HfError::Sim(SimError::NoBars));
    }
    if pipeline.landing.len() != bars.len() {
        return Err(HfError::PipelineLength {
            landing: pipeline.landing.len(),
            bars: bars.len(),
        });
    }

    // Landing schedule: land_at[l] = signal indices t with t + offset == l, ascending t
    // (ties execute in signal order; overtaking — a later signal landing earlier — is
    // legal). Off-end signals never land: the generalization of run's final-bar rule.
    let mut land_at: Vec<Vec<usize>> = vec![Vec::new(); bars.len()];
    let mut unlanded_orders = 0u32;
    for (t, outcome) in pipeline.landing.iter().enumerate() {
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
    let mut slippage_paid_quote = Decimal::ZERO;
    let mut gas_paid_sol = Decimal::ZERO;
    let mut bars_in_market = 0u32;

    for (l, bar) in bars.iter().enumerate() {
        // 1. Execute every order landing at this bar, at ITS open — the target is decided
        //    from the signal-time history `bars[..=t]` but priced at landing.
        for &t in &land_at[l] {
            let exec_price = bar.open;
            let weight_now = current_weight(&state, exec_price);
            let target = clamp01(target_fn(&bars[..t + 1], weight_now));
            let was_flat = state.base_balance <= dust();
            let quote_before = state.quote_balance;
            if let Some(outcome) = rebalance(&mut state, target, exec_price, cost)? {
                n_trades += 1;
                traded_notional_quote += traded_notional(&outcome);
                fees_paid_quote += outcome.dex_fee_quote;
                slippage_paid_quote += outcome.slippage_quote;
                gas_paid_sol += outcome.gas_sol;
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
        // 2. Mark equity at this bar's CLOSE (identical to `run`).
        equity_curve.push(EquityPoint {
            ts: bar.ts,
            equity_quote: state.equity(bar.close),
        });
        if state.base_balance > dust() {
            bars_in_market += 1;
        }
    }

    let priority_fees_paid_sol = cost.priority_sol() * Decimal::from(n_trades);
    Ok(HfRunOutput {
        base: RunOutput {
            equity_curve,
            round_trips,
            final_state: state,
            n_trades,
            traded_notional_quote,
            fees_paid_quote,
            slippage_paid_quote,
            gas_paid_sol,
            priority_fees_paid_sol,
            bars_in_market,
        },
        unlanded_orders,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// 1s-spaced bars with open ≠ close so "executes at OPEN" is distinguishable
    /// (OHLC-valid: open = p, close = p + 2, high = p + 3, low = p − 1).
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

    #[test]
    fn fixed_latency_one_is_all_landed_offset_one() {
        let p = fixed_latency(1, 3);
        assert_eq!(p.min_latency(), 1);
        assert_eq!(p.landing().len(), 3);
        for outcome in p.landing() {
            assert_eq!(
                *outcome,
                LandingOutcome {
                    offset_bars: 1,
                    landed: true
                }
            );
        }
    }

    #[test]
    fn zero_min_latency_rejected() {
        assert_eq!(
            LatencyPipeline::new(0, vec![]).unwrap_err(),
            HfError::ZeroMinLatency
        );
    }

    #[test]
    fn offset_below_min_rejected() {
        assert_eq!(
            LatencyPipeline::new(
                2,
                vec![LandingOutcome {
                    offset_bars: 1,
                    landed: true
                }]
            )
            .unwrap_err(),
            HfError::OffsetBelowMin { index: 0 }
        );
    }

    #[test]
    fn pipeline_length_mismatch_rejected() {
        let bars = series(&[100, 110, 120, 130]);
        assert_eq!(
            run_hf(
                &bars,
                dec!(10000),
                &CostModel::zero(),
                &fixed_latency(1, 3),
                |_h, _w| dec!(1)
            )
            .unwrap_err(),
            HfError::PipelineLength {
                landing: 3,
                bars: 4
            }
        );
    }

    #[test]
    fn empty_bars_rejected() {
        assert_eq!(
            run_hf(
                &[],
                dec!(10000),
                &CostModel::zero(),
                &fixed_latency(1, 0),
                |_h, _w| dec!(1)
            )
            .unwrap_err(),
            HfError::Sim(SimError::NoBars)
        );
    }

    #[test]
    fn bad_probability_rejected() {
        assert!(matches!(
            build_landing_table(1, 0..10, 1, 1, 0),
            Err(HfError::BadProbability { .. })
        ));
        assert!(matches!(
            build_landing_table(1, 0..10, 1, 3, 2),
            Err(HfError::BadProbability { .. })
        ));
    }

    #[test]
    fn probability_boundaries_are_legal_and_exact() {
        // p = 0 (num == 0) and p = 1 (num == den) are legal boundary cases, not errors:
        // all-un-landed and all-landed respectively (post-C3 review finding).
        let none = build_landing_table(1, 0..100, 1, 0, 10).expect("p=0 is legal");
        assert!(none.iter().all(|l| !l.landed));
        let all = build_landing_table(1, 0..100, 1, 10, 10).expect("p=1 is legal");
        assert!(all.iter().all(|l| l.landed));
    }

    #[test]
    fn landing_table_is_pure_and_keyed_by_cell() {
        let a = build_landing_table(1, 0..1000, 2, 1, 2).unwrap();
        let b = build_landing_table(1, 0..1000, 2, 1, 2).unwrap();
        assert_eq!(a, b);
        let other_cell = build_landing_table(2, 0..1000, 2, 1, 2).unwrap();
        assert_ne!(a, other_cell);
    }

    #[test]
    fn latency_two_executes_at_landing_open() {
        let bars = series(&[100, 200, 250, 400]);
        let out = run_hf(
            &bars,
            dec!(10000),
            &CostModel::zero(),
            &fixed_latency(2, 4),
            |_h, _w| dec!(1),
        )
        .unwrap();
        // t=0 lands at bar 2 and buys at ITS open 250 (NOT bar 1's 200) → exactly 40 SOL;
        // t=1 lands at bar 3 with weight already 1 → no trade; t=2,3 fall off-end.
        assert_eq!(out.base.n_trades, 1);
        assert_eq!(out.base.final_state.base_balance, dec!(40));
        assert_eq!(out.unlanded_orders, 0);
        assert_eq!(out.base.equity_curve[1].equity_quote, dec!(10000));
        assert_eq!(out.base.equity_curve[2].equity_quote, dec!(10080));
        assert_eq!(out.base.equity_curve[3].equity_quote, dec!(16080));
    }

    #[test]
    fn unlanded_changes_no_balances_but_is_counted() {
        let bars = series(&[100, 200, 250, 400]);
        let pipeline = LatencyPipeline::new(
            1,
            vec![
                LandingOutcome {
                    offset_bars: 1,
                    landed: false,
                },
                LandingOutcome {
                    offset_bars: 1,
                    landed: true,
                },
                LandingOutcome {
                    offset_bars: 1,
                    landed: false,
                },
                LandingOutcome {
                    offset_bars: 1,
                    landed: true,
                },
            ],
        )
        .unwrap();
        let out = run_hf(
            &bars,
            dec!(10000),
            &CostModel::zero(),
            &pipeline,
            |_h, _w| dec!(1),
        )
        .unwrap();
        // t=0 and t=2 are in-range drops → counted; t=1 lands at bar 2 (buy 40 SOL at 250);
        // t=3 is off-end (landed=true but dropped, NOT counted).
        assert_eq!(out.unlanded_orders, 2);
        assert_eq!(out.base.n_trades, 1);
        assert_eq!(out.base.final_state.base_balance, dec!(40));
        assert_eq!(out.base.equity_curve[1].equity_quote, dec!(10000));
    }

    #[test]
    fn repeated_runs_are_deterministic() {
        let bars = series(&[100, 200, 250, 400]);
        let cost = CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        };
        let strat = |_h: &[Bar], _w: Decimal| dec!(0.5);
        let a = run_hf(&bars, dec!(10000), &cost, &fixed_latency(2, 4), strat).unwrap();
        let b = run_hf(&bars, dec!(10000), &cost, &fixed_latency(2, 4), strat).unwrap();
        assert_eq!(a, b);
    }
}
