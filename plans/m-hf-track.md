# M-HF — High-frequency research track (milestone plan)

**Status: AUTHORED 2026-07-09 (Q7 amendment, operator GO of the same date).** This is the formal
milestone plan the Q7 amendment points at — the successor to the idea draft
[highfrequency-algo-plan.md](highfrequency-algo-plan.md) (whose §1 strategy survey and §2
execution-realism requirements remain the reference text; this file supersedes its §3 card sketch).
Designed via a 3-design + 3-lens adversarial-review + adjudication workflow (7 Sonnet agents,
2026-07-09; verdict in worklog): **reuse-first** base design, with grafts from the data-first and
adversary-first designs recorded inline below.

**The goal, plainly:** genuine autonomous passive income at high volume — thousands of small-edge
trades/day on Solana — earned strictly through gates. Every strategy family is a hypothesis; the
expected default outcome of this track is a well-documented **rejection**, and that is a success:
it answers with real data and honest costs whether small-edge/high-volume trading clears round-trip
costs for a non-colocated solo operator. Signing/submission remain gated behind M8/M9 separate
explicit human approval, always. **Zero execution code in this track** — it is a simulation layer
over archived/synthetic data.

---

## 1. The reuse-first architecture bet

M4's engine is mostly resolution-agnostic; the track reuses it and isolates the genuinely new
surface. Reused **verbatim, no code change**:

- `sweep::window::{WindowKind, Window, WalkForward}` — pure index arithmetic; intraday windows are
  the same struct with 1s-scaled lengths (e.g. `train_len = 6*3600`), numbers frozen under HF-Q3.
- `sweep::sensitivity::{ScenarioId, CostScenario, scale_cost_model, cost_scenarios}` — the
  exact-integer scaling container; HF adds fields and rungs, never a new scaling mechanism.
- `sweep::runner::{enumerate_cells, aggregate_evidence, aggregate_fee_sensitivity, run_sweep}` —
  window→scenario→point canonical order and `thread::scope` dispatch, unchanged, provided the HF
  `eval_cell` sibling stays pure over `(slice, &CostModel, &ParamPoint, cash, ppy)`.
- `sweep::advance` — reused with one **additive** change (§4); all other criteria are
  frequency-independent.
- `sweep::report` + `schemas/sweep-report.schema.json` — additive schema versioning only (D-0001
  precedent).

Reused at the **pattern** level, deliberately duplicated rather than made generic:

- `sweep::partition` → a new, structurally identical `sweep::intraday_partition` module: own
  `Sealed`-equivalent, own by-value call-once `evaluate_on_holdout` sibling, own 3 `compile_fail`
  doctests with `no_run` canaries. The S9 holdout-sealing proof is load-bearing ("the M5 entry
  point"); reopening it for generics buys nothing since intraday and daily partitions never mix in
  one sweep. A leaked holdout is categorically worse than duplicated code. **Embargo and gap
  semantics at intraday resolution are defined in slot/event units, never wall-clock durations**
  (adjudication residual-risk item).

The **single highest-leverage reuse fact:** 1s bars need no new bar type (`IntraBar =
research_core::Bar`, a series-level interval fact asserted by `validate_series_spacing`, not a
type-level one). The recommended starter families (survey 1.2/1.5/1.6) run through the existing
`portfolio::run` + `sweep::cell::eval_cell` **unmodified** — a 1s signal executing at the next 1s
bar's open IS next-bar execution at finer grain. The new latency machinery is required only for
slot-resolution families (1.1/1.3/1.4/1.7), which the survey already defers.

**What genuinely breaks (honestly scoped as new):**
1. `portfolio::simulator::run`'s next-bar rule is hardcoded (`if t >= 1`, simulator.rs:89-94) — a
   new `run_hf`/`LatencyPipeline` entry point generalizes the loop **without touching `run`**,
   proven equivalent by regression (`run_hf(bars, fixed_latency(1)) == run(bars, …)` bar-for-bar).
2. `evaluate_candidate`'s `TurnoverImplausible` criterion inverts in meaning for HF (§4).

## 2. Data model, provenance, hygiene, scale

- **Types** are wave-1 M-HF-C1's as drafted (`research_core::intraday::{Side, TradePrint,
  SlotSnapshot}`, `market_data::intraday` validators), plus the `IntraBar = Bar` alias.
- **Provenance is a typed field, not prose** *(graft from the data-first design)*:
  `Provenance::Synthetic { spec_hash }` / `Provenance::Real { source_id }` carried on every
  fixture/series, rolled up into a computed `data_provenance: "synthetic"|"real"|"mixed"` field on
  the HF sweep report (additive schema change, lands with §4's version bump). "This result is
  real" becomes a computed fact — a report with any synthetic input can never render as real.
- **Hygiene** mirrors the C1 strict/loose validator-choice pattern (no new config layer): UTC,
  sorted + unique per `(venue, slot, seq)`, OHLC invariants via the reused `Bar` checks,
  gap-blocks-a-run unless a scenario is declared **in the `SweepSpec`** (C8), gaps counted in slot
  units.
- **Scale.** A year of 1s bars ≈ 31M rows; sweeps must not materialize it per cell. Contract: an
  `IntradaySource` trait (`fn slice(&self, Range<usize>) -> Vec<Bar>; fn len(&self)`) — cells
  slice only their window (HF-Q3 sizing keeps train+test at tens of thousands of bars); peak
  memory ≈ `n_threads × max_window_bars`. Storage: fixed-record columnar binary (ts + scaled
  integer OHLCV, ~40 bytes/row) read via `std::fs` — **zero new deps**; adopting `memmap2`/arrow
  instead requires its own recorded decision (D-0002 precedent). The storage-format decision is
  recorded as a DECISIONS entry at the scale-proof card (C2.6), not left implicit *(adjudication
  must-address)*.
- **Lookback bound.** The HF spec carries `max_lookback_bars`, validated at spec-parse time —
  unbounded warm-up lookback (EWMA/OU anchors) would silently break the windowed-memory bound
  *(graft from the adversary-first design)*.

## 3. Latency-aware adversarial fill model (deterministic by construction)

- **Pipeline.** `LatencyPipeline { min_latency, landing: Vec<LandingOutcome> }`;
  `LandingOutcome { offset_bars, landed }`. `run_hf` executes at
  `events[t + landing[t].offset_bars].open` — **market state at landing, never at signal**; a
  same-slot-at-signal-price mode exists only as an explicitly labeled upper-bound scenario.
  Un-landed orders change no balances but their attempted cost is accounted.
- **Deterministic landing/drop, no RNG ever.** (1) A congestion-regime series
  (`Calm|Busy|Hot`) is a pure non-lookahead function of trailing realized volatility/print
  density. (2) A checked-in, operator-reviewed TOML percentile table maps
  `(regime, fee_percentile) → (offset_bars, p_land as exact rational)`. (3) Probability without
  RNG: `landed = (splitmix64(cell_id, event_index) % D) < N` — the same cell-identity primitive
  M4 §5.8 uses for `run_id`. **`event_index` is the stable, global, fixture-relative index,
  computed once before windowing — never window-local** (else the same physical slot's outcome
  could differ per cell) *(adjudication must-address)*. **The hash key provably excludes
  evaluation order, thread id, and wall clock; a dedicated C3 test asserts byte-identical table
  lookups across thread counts {1,2,3,7,8}** — determinism of the primitive is proven where it is
  introduced, not inferred from aggregate sweep output *(graft from the adversary-first design)*.
- **Fail-closed table gaps.** A `(regime, percentile)` bucket absent from the table is a
  spec-validation error; `interpolate_landing: bool` defaults to `false` and, when enabled, is
  recorded in run metadata *(graft)*.
- **Cost model (C4).** `CostModel` gains `depth_curve`, `congestion_priority_table`, `tip_bps` —
  additive, integer/Decimal, scaled by the existing `scale_cost_model` pattern.
  **For a spec declared HF-kind, `depth_curve` is REQUIRED (parse error if absent)** — constant-bps
  slippage as an HF base case is the classic fabricated-edge failure and is structurally forbidden;
  it survives only as a ladder rung *(adjudication must-address)*.
- **Adverse selection is priced at the BASE rung, not only worst-case.** The base cost model
  carries a non-zero expected sandwich/pickoff term from the fail-closed probability table; a
  candidate cannot clear the §4 thresholds while excluding MEV costs from its everyday economics
  *(adjudication must-address)*. `AdversarialWorst` remains the τ-loss-on-every-fill bound.
- **Auction model (families 1.3/1.4).** `our_bid = min(margin_fraction × dislocation, cap)`;
  `competitor_bid` from the same hash+table primitive over an archived priority-fee percentile
  distribution (C9), pessimistic base margin so we win rarely. One primitive — hash + table —
  reused for landing, sandwich, pickoff, auction; never four bespoke mechanisms.
- **Maker fills (1.1/1.7, post-wave-1).** Trade-through-only predicate, size-capped by printed
  volume; touch-without-trade-through fills nothing. 100% new surface (`maker_fill.rs`), honestly
  scoped.
- **Named structural limitation (must appear in the C10 declaration verbatim):** hash-based
  deterministic targeting decorrelates "who gets sandwiched/picked off" from real predictability
  signals (size, repetition). HF strategies are by construction patterned, so this is an
  **optimism bias for exactly the strategies under test**; the AdversarialWorst rung and the base
  expected-cost term bound it but do not remove it *(adjudication must-address)*.

## 4. Ladder extension + the turnover-criterion replacement

- `ScenarioId` gains `HotCongestion`, `AdversarialWorst`, `Latency2x` (additive). `Latency2x` is
  not cost-shaped — an additive sibling `hf_cost_scenarios(base_cost, base_pipeline)` returns
  `(ScenarioId, CostModel, LatencyPipeline)` triples; the existing `cost_scenarios` signature is
  untouched and LF specs keep calling it.
- **Turnover.** `TurnoverImplausible` is backwards for HF (high turnover is the design, its cost
  already priced by the ladder). Additively: `RejectionKind::{CostDragExcessive,
  PerTradeEdgeInsufficient}`; `AdvancementThresholds.turnover_budget` widens to `Option<Decimal>`
  plus `cost_drag_share_ceiling` / `per_trade_edge_floor` (numbers frozen under HF-Q3, before any
  real-data result — the Q5 ratchet discipline verbatim). Computed from evidence already collected
  (fee-sensitivity sums); no new collection pass.
- **Provably additive:** the C6 card's primary gate is a regression proving an M4-era
  `CandidateEvidence`+thresholds (new fields `None`) yields byte-identical verdicts pre/post-C6.
  **[CORRECTED 2026-07-15 — this paragraph drifted from the landed code; C6 executed under the
  corrected scope, DONE.]** Two premises here were false against the code and moved to **C8**:
  (1) *Spec lint (HF-kind spec sets `turnover_budget` → parse error)* presupposes an HF/standard
  **spec-kind discriminator that does not exist** in `SweepSpec` — it is born in C8, so the lint
  lives there. (2) *"greps and re-verifies every `RejectionKind` consumer (results, cli) —
  exhaustive-match audit"* is **vacuous**: `RejectionKind` is referenced in neither `crates/results`
  nor `crates/cli`, and no exhaustive `match RejectionKind` exists anywhere (the CLI emits opaque
  JSON; `evaluate_candidate` uses `if`-chains). C6 instead added an exhaustive `RejectionKind::label()`
  as a real compiler-forcing function. **What C6 actually did (narrow, per operator decision):** the
  turnover-criterion replacement only — `turnover_budget → Option`, opt-in
  `cost_drag_share`/`per_trade_edge` criteria + their two `RejectionKind`s, additive schema minor
  bump. All ladder/`ScenarioId`/`hf_cost_scenarios`/`data_provenance` work also moved to **C8**.
- Schema: two new kind strings (C6, done) + `data_provenance` (§2, **moved to C8**) in additive minor
  version(s); old reports
  still validate.

## 5. Card sequence (gates on every card; workspace green at every step)

Wave-1 cards M-HF-C1/C2 stand as drafted in task-queue.md (C1 additionally folds in the `IntraBar`
alias — zero logic). Entry conditions for execution are unchanged from the draft: M4 gate ✓
(2026-07-09) + this amendment ✓ (2026-07-09) + **HF-Q1/HF-Q2 answered (still open)** — except that
C1–C2.6 are synthetic-only and need no data source; the operator may unblock them ahead of HF-Q1
by a written note in questions.md.

| # | Card | New/changed surface | Gate (beyond the full-workspace gate) |
|---|------|--------------------|----------------------------------------|
| 1 | **C1** (drafted) | intraday types + hygiene (+ `IntraBar` alias, `Provenance` field) | corrupt/dup/gap/unsorted fixtures rejected; valid loads |
| 2 | **C2** (drafted) | deterministic synthetic microstructure generator | same spec → byte-identical; shape tests |
| 3 | **C2.5 — reuse proof (load-bearing, cheap)** | ZERO source changes: existing families through existing `run`+`run_sweep` on C2's synthetic 1s series | valid `SweepReport`, deterministic across {1,2,3,7,8}, on 1s bars; **if this fails, the reuse-first bet is wrong — escalate, re-plan** |
| 4 | **C2.6 — streaming/scale proof** *(graft)* | `IntradaySource` + columnar reader; storage-format DECISIONS entry | bounded memory (measured) + recorded wall-clock on a full ~31M-row annual synthetic fixture, BEFORE the fill engine exists |
| 5 | **C3** | `run_hf`/`LatencyPipeline` (new file; `simulator.rs` untouched) | `run_hf(fixed_latency(1)) == run` bar-for-bar; splitmix64 lookup table byte-identical across thread counts (dedicated test); M2 suite untouched |
| 6 | **C4** | HF cost model fields (depth-walk, regime priority, tip; HF-kind requires `depth_curve`) | hand-computed reference trade exact; `total > base-fee floor` asserted |
| 7 | **C5** | adversarial terms: sandwich, pickoff, maker trade-through; base-rung expected adverse-selection term | worst-case rung reproduces τ-loss every taker fill; touch-without-trade-through → no fill |
| 8 | **C6** *(DONE 2026-07-15, narrow)* | **turnover replacement ONLY** (`turnover_budget→Option`, opt-in cost-drag/per-trade-edge criteria + 2 `RejectionKind`s + exhaustive `label()`; additive schema 1.1.0→1.2.0) | byte-identical-to-M4 regression (PRIMARY gate, met); version-only sweep diff. **Ladder rungs / `ScenarioId` / `hf_cost_scenarios` / `data_provenance` / HF-kind spec-lint all MOVED TO C8** (their C6 premises didn't exist in code — see §4 correction) |
| 9 | **C7** (drafted) | `statarb_pairs_v1` + `intraday_meanrev_v1` (intent-only) | deterministic; weights only; ≥100k-bar cadence run within the C2.6 budget |
| 10 | **C8** | `SweepSpec` HF-kind (resolution, cost blocks, `max_lookback_bars`, scenario declarations); windowed sweep wiring | full synthetic HF sweep deterministic across {1,2,3,7,8}; schema-valid; holdout counter 0; wall-clock re-measured and recorded |
| 11 | **C9** (drafted; BLOCKED on HF-Q1) | real intraday ingestion → columnar snapshot, gitignored, credential-free | one real archived day loads/validates/sweeps deterministically |
| 12 | **C10** | HF research gate declaration | full ladder on real data; **sensitivity sweep over `competitor_floor`/`margin_fraction` showing verdicts robust to it**; the §3 targeting-decorrelation limitation stated verbatim; explicit advance/reject per candidate; track ends at a research decision — anything further is M6+/M8/M9 |

Risk points earning an adversarial review (FOREMAN §3): after C2.5 (the bet), after C3 (the
determinism primitive), after C6 (the load-bearing `advance.rs` seam), before the C10 declaration.

## 6. Open operator decisions (recorded in questions.md)

- **HF-Q1** — intraday data source + credential handling (extends Q3; Birdeye pencilled, re-verify
  depth + redistribution terms). Blocks C9/C10 only.
- **HF-Q2** — allowlist additions (USDT for 1.4, LSTs for 1.6). Blocks those families' C7+ runs.
- **HF-Q3** — freeze intraday walk-forward sizing, the landing/priority percentile tables, and the
  HF advancement numbers (`cost_drag_share_ceiling`, `per_trade_edge_floor`, auction margins) —
  BEFORE the C10 sweep, same one-way ratchet as Q5.

## 7. Honest limits (kept from the design's own risk register)

- The cheap C2.5 win covers OHLC-shaped 1s-bar families only; the headline thousands-of-trades/day
  families (1.1/1.3) require the full new latency/maker surface — the early win must not be read
  as "the HF track is easy."
- C2.6/C8 produce *measurements* of compute feasibility, not guarantees; if the wall-clock number
  is unacceptable, the fallback (gate-representative smaller fixture vs. full-scale) is decided on
  the record at C8, not assumed here.
- The whole fill model is pessimistic-by-construction but still a model; M6 shadow (its own gate)
  is where modeled execution meets observed quotes.
