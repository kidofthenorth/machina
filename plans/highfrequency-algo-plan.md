# High-frequency research track (Q7 amendment draft)

**Status: DRAFT — operator review required.** This document is the deliverable for decision Q7
(plans/questions.md): the operator's actual ambition is a genuinely autonomous 24/7 bot earning
passive income on SOL/USDC or SOL/USDT at high volume — thousands of trades/day with small
per-trade edges. This draft is the planning input for the formal master-plan amendment; **nothing
here edits `plans/master-plan.md` or `solana-crypto-trader-plan.md`** (that lockstep, byte-identical
edit happens only after the operator approves this doc, and only after the M4 gate declaration,
per Q7). No card below authorizes execution work: key loading, signing, and submission remain
gated behind M8/M9 separate explicit human approval, always.

**Framing rule for everything below:** every strategy family is a **hypothesis**, never a "known
profitable" strategy. Each one must pass the *same* machinery M4 built for the low-frequency
track — determinism gate, sealed holdout, walk-forward, cost-sensitivity ladder,
advancement/rejection report — extended to intraday resolution. The plan's original economics
warning stands: at thousands of trades/day, the per-trade edge competes with round-trip costs
against colocated/MEV-aware counterparties. The S10 cost ladder (extended per §2) is the
instrument that decides whether any edge is real; the default expectation is that most candidates
**fail** it, and a clean rejection is a successful research outcome.

---

## 1. Candidate survey — HF strategy families as hypotheses

Seven families. For each: the mathematical edge mechanism, why the edge might persist on Solana
specifically, per-trade cost sensitivity (the number the S10 ladder will attack), and the data it
needs. Ordered roughly by how testable each is with data we can realistically archive.

### 1.1 Passive market making (spread capture) — `mm_spread_v1`

- **Edge mechanism.** Quote both sides of the book (or a CLMM range) around a fair-value estimate
  `m̂`. Expected PnL per round trip ≈ `spread/2 − adverse_selection − fees`. The classic
  Avellaneda–Stoikov formulation: optimal quotes are `m̂ ± (γσ²(T−t)/2 + (1/γ)ln(1+γ/κ))`,
  skewed by inventory `q` via a reservation price `r = m̂ − qγσ²(T−t)`. The edge exists iff
  uninformed (noise/retail/aggregator) flow pays the spread more often than informed flow picks
  you off.
- **Why it might persist on Solana.** SOL/USDC on Phoenix / OpenBook v2 has a real CLOB with
  maker rebates or zero maker fees; much aggressor flow is retail routed via Jupiter and is
  plausibly uninformed. CLMM venues (Orca Whirlpools, Raydium CLMM) let a passive range act as a
  standing two-sided quote. Fewer professional MMs than on CEXes → wider average spreads. Counter:
  the informed flow that *does* exist is searcher/arb flow that hits stale quotes within the same
  slot — exactly the adversarial-fill problem §2.4 models.
- **Cost sensitivity: EXTREME.** Edge per trade is a fraction of the spread (SOL/USDC spread is
  often ~1–5 bps). Taker fees, priority fees for quote updates, and adverse selection each eat
  bps. Survival requires maker-side economics; the doubled-costs rung of the ladder will likely
  be fatal unless fees are near zero. Inventory risk converts to cost when hedging.
- **Data needed.** L2 order-book snapshots + top-of-book updates at slot resolution (Phoenix/
  OpenBook market accounts), trade prints with aggressor side, own-fill simulation requires queue
  position modeling (or a conservative "filled only when price trades through" rule — see §2.3).

### 1.2 Short-horizon mean reversion — `intraday_meanrev_v1`

- **Edge mechanism.** Price deviations from a slow anchor revert over seconds-to-minutes when the
  deviation is liquidity-driven, not information-driven. Model price as Ornstein–Uhlenbeck around
  anchor `μ_t` (e.g. EWMA or a cross-venue reference): `dx = θ(μ − x)dt + σdW`; enter when
  `z = (x − μ)/σ` exceeds a band, exit at reversion or timeout. Edge per trade ≈
  `E[|z|·σ·recovery_fraction] − costs`.
- **Why it might persist on Solana.** Large single swaps against AMM curves cause mechanical,
  temporary price impact that decays as arbitrageurs restore alignment with the global SOL price;
  liquidations and NFT/memecoin-driven SOL flows create dislocations uncorrelated with SOL
  fundamentals. Retail-heavy flow overshoots. Counter: the fastest reverters are the searchers
  themselves — we'd be racing them with worse latency.
- **Cost sensitivity: HIGH.** Typical exploitable dislocation might be 5–30 bps; a taker round
  trip (2× swap fee + 2× slippage + 2× priority) can easily total 10–60 bps. Only the tail of
  large dislocations can clear costs; the ladder decides where that threshold sits.
- **Data needed.** 1-second (or per-slot) mid-price / pool-state series for the traded venue plus
  a reference price series (CEX or aggregate) to define the anchor; trade-size series to
  distinguish impact-driven from news-driven moves.

### 1.3 Cross-venue arbitrage on Solana (spot–spot) — `xvenue_arb_v1`

- **Edge mechanism.** The same SOL/USDC pair prices differently across on-chain venues
  (Orca vs Raydium vs Phoenix vs Lifinity…). When `p_A/p_B − 1 > total_cost`, buy on A, sell on
  B, ideally atomically in one transaction so there is no leg risk. Deterministic payoff:
  `qty·(p_B − p_A) − fees(A) − fees(B) − slippage(A) − slippage(B) − priority`.
- **Why it might persist on Solana.** Liquidity is genuinely fragmented across 5+ major venues;
  atomic multi-hop transactions make the arb riskless *if you win the slot*. Counter — and this
  is the strongest counter in the whole survey: this is the canonical searcher trade. Winners are
  decided by priority-fee auctions (Jito tips) and validator relationships; the marginal winner
  bids away nearly the entire spread. The hypothesis is really "is there residual dislocation
  below the searchers' attention threshold that still clears *our* costs" — plausibly no.
- **Cost sensitivity: EXTREME, and auction-shaped.** Cost isn't a constant: the priority fee
  needed to win is endogenous, ≈ the arb's own value minus the winner's margin. Modeling it as a
  fixed lamport number is exactly the mistake §2.4 exists to prevent.
- **Data needed.** Time-aligned pool states / books for ≥2 venues at slot resolution, plus
  historical priority-fee / Jito-tip distributions to model the auction (§2.4).

### 1.4 Triangular arbitrage (single- or cross-venue) — `tri_arb_v1`

- **Edge mechanism.** Around a cycle SOL→USDC→USDT→SOL, profit iff
  `∏ p_i(1 − f_i)(1 − s_i) > 1 + priority_cost`. Same math as 1.3 with three legs, so three fee
  and slippage terms.
- **Why it might persist on Solana.** USDC/USDT on-chain pools are deep but their peg wobbles a
  few bps; three-leg cycles are checked by fewer bots than two-leg ones; atomicity again removes
  leg risk. Counter: three legs triple the cost base, and the USDC/USDT leg's edge is usually
  <5 bps — the strictest cost test of any family here. Note the allowlist currently gates tokens;
  USDT participation would need an explicit operator allowlist addition (an operator decision,
  not an executor default).
- **Cost sensitivity: EXTREME** (three fee+slippage terms against single-digit-bps mispricings).
- **Data needed.** Slot-aligned pool states for all three pairs; priority-fee distributions.

### 1.5 Short-horizon momentum / order-flow imbalance — `flow_momentum_v1`

- **Edge mechanism.** Signed order flow predicts short-horizon returns: with trade imbalance
  `OFI_t = Σ signed_volume` over a short window, hypothesize `E[r_{t,t+h}] = β·OFI_t + ε` with
  `β > 0` for horizons of seconds to minutes (a well-documented microstructure regularity in
  other markets). Enter with the imbalance, exit at horizon `h` or on signal decay. Per-trade
  edge ≈ `β·|OFI| − costs`.
- **Why it might persist on Solana.** All flow is public and attributable in real time — swap
  direction, size, and even wallet identity are on-chain, a richer signal set than CEX tape.
  Retail momentum-chasing in SOL is strong around volatility events. Counter: by the time a bar
  closes and we react next-bar, faster observers have traded; the residual β after their action
  may be ≤ costs.
- **Cost sensitivity: HIGH.** Better than pure arb — positions are held minutes, so edge per
  trade can be tens of bps in volatile regimes — but signal decay means late entries pay full
  costs for a decayed edge. Sensitive mostly to slippage (entries chase moves).
- **Data needed.** Trade-level prints with direction and size (per-slot), volatility series;
  1s–1m bars derivable from them.

### 1.6 Stat-arb pairs on correlated Solana assets — `statarb_pairs_v1`

- **Edge mechanism.** For cointegrated pair (e.g. SOL vs a liquid LST like mSOL/jitoSOL, whose
  fair ratio moves slowly with staking yield): spread `s_t = ln p_A − γ ln p_B` is
  mean-reverting; trade z-score bands on `s_t` as in 1.2 but market-neutral-ish, so PnL is
  independent of SOL direction. LST/SOL is the cleanest cointegration on Solana because the
  fair ratio is *computable* from the stake pool state, not just estimated.
- **Why it might persist on Solana.** LST/SOL pools are second-tier liquidity where deviations
  of 10–50 bps from the redeemable ratio appear and persist for minutes because unstake
  arbitrage is capital- and time-costly (epoch delays) — a structural friction, not just speed.
  Counter: exit liquidity in the LST leg is thin, so realized slippage may consume the deviation;
  and holding a deviating spread has real tail risk (depeg events).
- **Cost sensitivity: MEDIUM-HIGH.** Deviations are larger and slower than in 1.3/1.4, so
  fewer, bigger-edge trades; slippage on the thin leg is the dominant cost. This is plausibly the
  most cost-robust family in the survey — which is a statement about testability, not
  profitability. Requires allowlist addition for the LST (operator decision).
- **Data needed.** Bar or pool-state series for both legs plus the on-chain fair ratio (stake
  pool exchange rate per epoch); depth/liquidity series for the thin leg.

### 1.7 Liquidity-provision rebalancing on CLMMs — `clmm_range_v1`

- **Edge mechanism.** Provide concentrated liquidity in a range around the price; fee income
  accrues per unit time as `fee_rate × volume_share`; the cost is adverse selection formalized as
  **loss-versus-rebalancing (LVR)** ≈ `σ²/8 × pool value` per unit time for a full-range position
  (worse when concentrated). Hypothesis: on chosen ranges/regimes,
  `fees_earned − LVR − rebalance_costs > 0`. This is market making (1.1) in AMM clothing, with
  the range width as the spread.
- **Why it might persist on Solana.** SOL/USDC CLMM volume is enormous relative to TVL in tight
  ranges (high fee APR); regime classification (M3's scaffold) could park liquidity only in
  low-σ regimes where LVR is small. Counter: LVR is paid to exactly the arb flow of 1.3, i.e. we
  would be on the *losing* side of the sharpest trade on the chain whenever σ spikes.
- **Cost sensitivity: MEDIUM** on rebalancing (infrequent), but the dominant "cost" is LVR
  itself, which must be modeled explicitly from σ — a fill-model extension, not a fee knob.
- **Data needed.** Pool state (liquidity distribution, fee rate), volume series, realized σ at
  intraday resolution; bar data alone is insufficient.

### Survey summary (hypothesis triage, not a ranking of profitability)

| Family | Edge size/trade (hypothesized) | Trades/day | Cost sensitivity | Data burden | Adversarial exposure |
|---|---|---|---|---|---|
| 1.1 mm_spread | ~spread/2 (1–3 bps) | 1000s | extreme | very high (L2 book) | high (picked off) |
| 1.2 intraday_meanrev | 5–30 bps tail | 10s–100s | high | medium (1s bars + ref) | medium |
| 1.3 xvenue_arb | <5 bps residual | 100s | extreme, auction-shaped | high (multi-venue slots) | maximal |
| 1.4 tri_arb | <5 bps | 10s–100s | extreme ×3 legs | high | maximal |
| 1.5 flow_momentum | 10–50 bps in vol regimes | 10s | high | medium (trade prints) | medium |
| 1.6 statarb_pairs | 10–50 bps, slow | 1s–10s | medium-high | low-medium | low-medium |
| 1.7 clmm_range | fee APR − LVR | continuous | medium + LVR | high (pool state) | high (LVR) |

Recommended first hypotheses to *test* (cheapest data, most structural edge story): **1.6, 1.2,
1.5** — they run on 1-second bars + trade prints rather than full L2 books, so the realism layer
of §2 can be built incrementally. 1.3/1.4 should be tested too, but primarily as *measurements of
the adversarial environment* (how fast do dislocations close? what do winners pay?) that calibrate
§2.4 — their most likely research outcome is a well-documented rejection.

**Plainly, on the goal:** the recommended starters (1.6/1.2/1.5) trade tens, not thousands, of
times a day. The families that actually match the "thousands of trades/day" ambition — 1.1 and
1.3 — are tested later **only because they need the L2/slot data layer first**, not because the
high-volume goal has been deprioritized. The ordering is a data-dependency, not a scope retreat.

---

## 2. Execution-realism requirements — what must exist before honest HF simulation

The M2 simulator (next-bar fills, daily bars, static CostModel) is honest for the low-frequency
track and **dishonest for HF by construction**: at thousands of trades/day, fills, latency, and
fees are where all the PnL lives. No HF backtest result may be reported, even internally, before
the following exist. Same invariants as everywhere: deterministic, fixed-point money, replayable.

### 2.1 Intraday/tick data foundation

- A new bar/event resolution: at minimum **1-second bars** for the traded pair(s); target
  **slot-resolution events** (~400 ms) for arb/MM families: pool-state snapshots, trade prints
  (side, size, venue), and for 1.1 top-of-book/L2 deltas.
- Same hygiene rules as M1, extended: UTC, sorted, unique per (venue, slot, seq); OHLC invariants
  per bar; **gaps block a run** unless declared a scenario — at slot resolution gaps are frequent
  (skipped slots, RPC outages), so the gap-scenario config from M1 becomes load-bearing.
- **Compute scale is a requirement, not an afterthought.** A year at 1-second resolution is
  ~31M bars; a sweep is grid × walk-forward windows × 8 ladder rungs over that, and the
  determinism gate re-runs it at five thread counts. The M4 engine materializes per-cell results
  over daily bars; the HF path must support **streaming / per-window evaluation** (bounded memory
  per cell) and a stated wall-clock budget for a full sweep on the operator's machine — otherwise
  the C8/C10 gates are physically untestable on real data.
- Storage becomes a real concern (~86k rows/day/venue at 1s; more for prints) → columnar on-disk
  fixtures with a checked-in *tiny* sample per venue for tests, full archives gitignored like all
  generated data.
- **Source selection is an operator decision (extends Q3):** candidates are Birdeye/Bitquery
  historical trade APIs, a self-run archival job against RPC (`getBlock` replay decoding DEX
  instructions), or vendor tick dumps. All need either paid credentials or sustained infra — the
  executor must not create or store either. Until decided, HF work uses **synthetic intraday
  fixtures** with realistic microstructure (documented as synthetic in every report).

### 2.2 Latency-aware fill model

- Every order carries an explicit pipeline: `t_signal → t_decide → t_submit → t_land`, with
  **landing quantized to slot boundaries** and a configurable landing-delay distribution
  (deterministic: sampled from a fixed empirical table keyed by run seed material already in the
  spec — never a runtime RNG). Minimum honest default: signal computed on data up to slot `n`
  cannot land before slot `n+2`.
- Fill price = the market state **at landing**, not at signal — the intra-latency price drift is
  where HF backtests classically lie. A "same-slot fill at signal price" mode may exist only as
  an explicitly labeled *upper-bound* scenario, never the base case.
- Landing is probabilistic in reality (transactions drop); model as deterministic landing-or-drop
  from the fee-conditional table in §2.4, so a run replays byte-identically.
- This generalizes M2's next-bar rule; the existing simulator becomes the special case
  `latency = 1 bar`.

### 2.3 Fill realism for maker/passive strategies (1.1, 1.7)

- Without our own queue position in the historical book, maker fills must be **conservative**:
  a resting quote fills only if the market *trades through* its price (not merely touches it),
  and fill size is capped by printed volume at that level. Optimistic touch-fills are forbidden
  as a base case.
- CLMM positions need LVR accounting (1.7): realized fee income from actual volume prints minus
  rebalancing-cost and adverse-selection terms derived from realized σ.

### 2.4 Per-trade cost model: fee + slippage + priority + adversarial fills — never base fee only

- **Total cost per trade** is modeled as
  `venue_fee(bps) + realized_slippage(size, depth) + priority_fee(lamports, regime) + tip(lamports)`
  — the Solana base fee (5000 lamports/sig) is a *floor* so far below real costs that using it
  alone would fabricate edges. All integer/decimal, extending the existing exact
  `scale_cost_model` machinery.
- **Slippage from depth, not a constant:** realized execution price walks the archived pool
  curve / book depth at landing time for the order's size. Constant-bps slippage remains only as
  a ladder rung, not the model.
- **Priority fees are regime-dependent and, for arbs, endogenous.** Model as a lookup keyed by
  congestion regime (calm/busy/hot), calibrated from archived priority-fee percentiles; landing
  probability is conditional on the paid fee. For 1.3/1.4, add an **auction model**: assume a
  competitor exists who bids up to `(1−margin)` of the arb value; we win only dislocations below
  their detection threshold or when our bid exceeds theirs — with the base-case margin set so
  that we win *rarely*. Pessimism is the honest default.
- **Adversarial fills (MEV) — modeled as a defender, never built as a tool.** Scope is strictly
  "what do adversaries cost *us*": (a) sandwich exposure — any taker swap with slippage tolerance
  `τ` is assumed sandwiched with probability `p(size, venue, regime)`, costing up to `τ` when it
  fires (base case: aggressive `p`); (b) stale-quote pickoff for maker strategies — resting
  quotes are hit at a loss whenever the reference price moves through them within the latency
  window; (c) back-run leakage for predictable rebalances. These are cost/risk terms in the
  simulator. Machina will not implement extraction of MEV from others' transactions; that is
  out of scope at every milestone.
- **The S10 ladder extends** with HF-specific rungs: {before_costs, base, doubled, doubled_slippage,
  doubled_priority} plus {hot_congestion, adversarial_worst (sandwich `p=1` at `τ`, pickoff on),
  latency_2x}. Advancement requires surviving the base *and* doubled rungs and showing a
  documented, bounded degradation profile across the adversarial rungs — same
  `survives_doubled`-style grounding against cost-matched baselines.

### 2.5 What is explicitly NOT required (and not allowed) for this research phase

Live data feeds wired into trading paths, key loading, signing, submission, RPC-connected order
flow of any kind — none of it. The whole of §2 is a *simulation* layer over archived/synthetic
data. Shadow execution against live quotes remains M6 (its own gate); anything that signs remains
M8/M9 (separate explicit human approval).

---

## 3. Milestone draft — HF research track cards (M-HF-C1…C10)

Provisional milestone label **M-HF** (final numbering assigned in the master-plan amendment —
likely slotted after M5 as a parallel research track, since it reuses the M4 engine and the M5
decision discipline). Same execution contract as the M4-C cards: one card per fresh session,
self-contained, runnable gate, no commits (operator commits), full-workspace gate
(`fmt --check` && `clippy -D warnings` && `test --workspace`) on every card. Guardrails carry over
verbatim from task-queue.md — Decimal/integer money, byte-identical parallel==sequential, no new
external deps without a card saying so, **no execution code anywhere**, holdout stays sealed,
never edit schemas-as-contracts without a card, never touch `master-plan.md` from a card.

**Entry conditions (all three, before C1 starts):** ① M4 gate declared (M4-C9 done);
② operator approved this document and the master-plan amendment has been made (operator-level
edit, both plan files in lockstep per Q6); ③ operator has answered the two decisions this track
needs: **HF-Q1** intraday data source + credential handling (extends Q3), **HF-Q2** any allowlist
additions (USDT, LSTs) for families 1.4/1.6.

| ID | Title | Scope sketch | Gate |
|----|-------|--------------|------|
| M-HF-C1 | Intraday types + hygiene | `market-data`: 1s-bar / slot-event / trade-print types; validation (sorted, unique, gap-blocking with scenario override); tiny synthetic checked-in fixtures | corrupt/dup/gap/unsorted fixtures all rejected; valid fixture loads; workspace green |
| M-HF-C2 | Synthetic microstructure generator | deterministic (seeded-table, no runtime RNG) generator for intraday fixtures: OU mid-price + impact-decay + regime-switching congestion; documented as synthetic in output metadata | same spec → byte-identical series; statistical shape tests (reversion, gap pattern) pass |
| M-HF-C3 | Latency-aware fill engine | `portfolio`/new `simcore`: signal→land slot pipeline, land-at-market-state pricing, deterministic landing/drop table; M2 simulator re-expressed as `latency = 1 bar` special case | M2 test suite still green unchanged; latency-2 fill priced at landing state, proven by test; parallel==sequential byte-identical |
| M-HF-C4 | HF cost model | per-trade `venue_fee + depth-walk slippage + regime priority + tip`, integer/Decimal only; congestion-regime lookup table in config template | cost of a reference trade matches hand-computed value exactly; base fee alone provably ≠ total cost (test asserts total > base floor) |
| M-HF-C5 | Adversarial-fill terms | sandwich exposure `p(size,venue,regime)·τ`, maker pickoff within latency window, conservative trade-through maker fills | worst-case rung reproduces τ-loss on every taker fill; maker touch-without-trade-through yields no fill |
| M-HF-C6 | Extended S10 ladder + HF advancement criteria | add rungs {hot_congestion, adversarial_worst, latency_2x} to `sweep::sensitivity`; **re-specify the advancement battery for HF** — the raw `turnover > turnover_budget` rejection criterion (`sweep::advance::evaluate_candidate`) inverts in meaning for HF, where huge turnover is by design and its cost is already priced by the ladder; for HF specs replace it with a cost-drag-share ceiling and/or per-trade-edge floor (numbers frozen under HF-Q3); extend `sweep-report` schema **additively** (new schema version, D-XXXX) | each rung monotone-or-documented like S10; an HF spec with design-level turnover is not auto-rejected while a zero-per-trade-edge candidate is; old reports still validate; two serializations byte-identical |
| M-HF-C7 | First hypothesis strategies | `strategies`: `statarb_pairs_v1` + `intraday_meanrev_v1` (survey 1.6, 1.2) as intent-only strategies over intraday bars; illustrative un-tuned params + grids in config template, clearly marked NOT tuned | deterministic strategy tests; strategies produce weights only; no venue/execution awareness leaks into strategy trait; **trait cadence check**: the existing intent-only `Strategy` trait runs unchanged over intraday bars — a 1s-bar run of ≥100k bars completes deterministically within the C8 wall-clock budget |
| M-HF-C8 | HF sweep wiring | `SweepSpec` accepts intraday resolution + HF cost blocks; `machina sweep` runs an HF spec end-to-end on synthetic fixtures; walk-forward windows re-sized for intraday (operator-frozen numbers, per Q5 discipline) | full sweep on synthetic data: deterministic across {1,2,3,7,8} threads; report schema-valid; holdout counter untouched |
| M-HF-C9 | Real-data ingestion (BLOCKED on HF-Q1) | archival ingestion for the operator-chosen source; hygiene gates from C1 applied; all real data gitignored | one real archived day loads, validates, and sweeps deterministically; zero credentials in tree |
| M-HF-C10 | **HF research gate declaration** | run the frozen sweep + full ladder on real data for C7 strategies (+ any 1.3/1.4 environment-measurement runs); write the advancement/rejection report; declare the gate in current-state/worklog | report validates against schema; every candidate has an explicit advance/reject verdict with reasons; **track ends here** — anything further (shadow, execution) belongs to M6+/M8/M9 and their own approvals |

**The gate this track ends at (C10) is a research decision, mirroring M5:** for each hypothesis,
either *reject with documented reasons* (expected default) or *advance to the M5-style holdout
evaluation and then to the M6 shadow question* — never directly to execution. A track that rejects
all seven families cleanly is a success: it will have answered, with real data and honest costs,
whether small-edge/high-volume trading on Solana clears round-trip costs for a non-colocated solo
operator — the question Q7 exists to settle before any capital is considered.

**Open operator decisions created by this draft:** HF-Q1 (data source/credentials), HF-Q2
(allowlist additions), HF-Q3 (freeze intraday walk-forward sizing + HF advancement thresholds —
including the C6 replacement of the raw turnover budget with a cost-drag-share ceiling /
per-trade-edge floor — before the C10 sweep, per the Q5 anti-overfitting rule). These should be added to
plans/questions.md when the amendment is made.

---

## Appendix — 2026-07-07 pre-expansion audit notes (read before approving this doc)

A plan-vs-tree audit ran before wave-1 cards were drafted (operator-directed; cards are BLOCKED in
task-queue.md pending this doc's approval + the M4 gate + the amendment). Confirmed corrections —
the wave-1 cards already implement them; the body text above was left as drafted:

1. **§2.1 / C1 "gap-scenario config from M1" — no such config exists.** The only gap override in
   the tree is choosing the looser validator: `market-data/src/validation.rs:114-115` — "Callers
   that deliberately allow gaps (an explicit scenario) should use [`validate_series`] instead."
   M-HF-C1 therefore mirrors the two-function pattern (strict + loose, choice documented); a
   config-level scenario declaration belongs to M-HF-C8's `SweepSpec` work.
2. **§2.1 "unique per (venue, slot, seq)" is a NEW key, not an extension.** `Bar` carries only
   `ts`; nothing in the tree has venue/slot/seq fields. M-HF-C1 introduces them.
3. **§2.2 "the existing simulator becomes the special case `latency = 1 bar`" is a new abstraction,
   not an existing knob.** The one-bar delay is structural in `portfolio::simulator::run`'s loop
   (`if t >= 1 { exec at bar.open }` — simulator.rs:89-94); no latency parameter exists anywhere.
   M-HF-C3 correctly scopes this as new work; read the plan sentence as intent, not as a refactor.
4. **Verified present and coherent:** the four prior review folds (C6 turnover-criterion
   replacement under HF-Q3; §2.1 ~31M-bars streaming/wall-clock requirement; §1 tens-vs-thousands
   data-dependency paragraph; C7 ≥100k-bar cadence gate); zero HF/intraday/latency code anywhere in
   `crates/`; `evaluate_candidate`'s `turnover > turnover_budget` predicate exists as named
   (advance.rs:128/182).

**Audit coverage gap:** the sweep-ladder (`ScenarioId` extension sites for C6), strategies-trait
(resolution-agnosticism for C7), and invariants/config audit areas were cut short by a session
limit. **Re-run those before expanding wave 2 (C3+).** Wave 1 is unaffected — its two crates were
audited first-hand.
