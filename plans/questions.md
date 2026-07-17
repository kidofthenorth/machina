# Operator questions

Only **real** operator decisions live here. Each has a safe default already applied, so none of these
block autonomous M0–M4 *engine* work. Format: Question · Why it matters · Safe default (applied) ·
What's blocked.

> Status: **0 blocking**, 5 non-blocking open (Q1, Q2, Q4, HF-Q1, HF-Q3), 6 resolved
> (Q3, Q5, Q6, Q7, HF-Q2, HF-Q4). **M5 is CLOSED — gate declared 2026-07-12: reject-all, holdout unread**
> (see the M5 OUTCOME block after Q7). **M-HF wave 1 is ACTIVE:** C1–C2.6 unblocked by operator
> note (HF-Q1, 2026-07-12); HF-Q2 resolved (USDT + jitoSOL research allowlist, exact per-token fields
> still owed — now blocks only the deferred `M-HF-C8-PAIR` mini-track, not core C8); HF-Q1 deferred to
> wave 2 (blocks C9/C10); HF-Q3 gates the decisive HF sweep (C10), same ratchet as Q5. **HF-Q4
> (2026-07-16): statarb_pairs_v1 gets a real two-leg engine, scoped into its own future mini-track,
> not folded into C8.**

---

## Q1 — Repo/product name: "machina" vs "solana-crypto-trader" (non-blocking)
- **Why it matters.** The on-disk repo is `machina`; the plan and first-packet header use
  `solana-crypto-trader`. Wanted consistent naming in README/manifests.
- **Safe default (applied).** Treat `machina` as the repo/product name and `solana-crypto-trader` as
  the project codename; packets use `REPO=solana-crypto-trader` per the plan's contract.
- **Blocked:** nothing. Rename is cosmetic if the operator prefers one name.

## Q2 — Should the `.claude/` cadence engine be committed? (non-blocking)
- **Why it matters.** Committing it makes the loop travel with the repo; not committing keeps the
  repo lean. Original gitignore ignored all of `.claude/`.
- **Safe default (applied).** Ignore only `.claude/settings.local.json` (machine-local); leave the
  rest stageable but **do not** stage it (operator stages explicit paths only).
- **Blocked:** nothing.

## Q3 — Real OHLCV data source for SOL/USDC — **RESOLVED 2026-07-09**
- **Why it matters.** Honest backtests (M4–M5) need archived candles. Options in the plan: Birdeye,
  CoinGecko/GeckoTerminal, Bitquery. Some need an API key (a paid credential the executor must not
  create or store).
- **Safe default (previously applied).** M1–M4 used **tiny synthetic, checked-in fixtures only**; no
  network and no credentials.
- **Resolution (operator, 2026-07-09, verbatim).** Binance data.binance.vision daily SOLUSDC klines
  are the M5 data source, ingested as a one-time operator-run snapshot into gitignored files with a
  checked-in validation record (date range, row counts, content hash). The CEX-proxy caveat is
  acknowledged and must be recorded in the sweep report's provenance notes; execution realism is
  owned by M6 shadow + the S10 cost ladder. If early-history hygiene fails (gaps/thin candles),
  shrink the span to where hygiene passes — never patch or forward-fill. GeckoTerminal daily pool
  data is a shape/sanity cross-check only, not an equality check. Birdeye is pencilled as the HF
  track's intraday source (HF-Q1, decided later; re-verify its depth and redistribution terms then).
- **Blocked:** nothing. The M5 queue's ingestion cards implement this.

## Q4 — Strategy parameter values + sweep grids (non-blocking)
- **Why it matters.** `config/strategies/strategy-lab.example.toml` contains illustrative parameters
  (SMA period, weights, bands). These are placeholders, not tuned values, and must not be read as a
  recommendation. M4 also adds parameter *grids* (sets of values to sweep) over these.
- **Safe default (applied).** Ship conservative illustrative defaults/grids; tuning/validation is
  M4–M5. No value here is a recommendation.
- **Blocked:** nothing; no profitability is claimed.

## Q5 — M4 research policy: walk-forward sizing + rejection thresholds — **RESOLVED 2026-07-09 (rules now, numbers at span confirmation)**

- **Resolution (operator, 2026-07-09, verbatim).** RESOLVED (rules now, numbers at span
  confirmation): the proposed set is approved — rolling 365/90/90/5; drawdown_budget 0.35;
  turnover_budget 12; baseline_margin 0.02; dispersion_budget 0.40; neighbor_tolerance 0.15;
  min_windows 6; holdout = final ~20% by date. Final partition date and min_windows are to be
  confirmed against the ingested span and frozen in the same sitting as the hygiene check, before
  any strategy result is computed. One-way ratchet: after that freeze, immutable for this M5 cycle.
- **FREEZE ADDENDUM (operator sitting, 2026-07-11 — card M5-C3).** Confirmed span: **916 bars,
  2023-12-28..2026-06-30** (fnv1a64 `0x65a1e18b7554a1c5`; combined CSV sha256 `1c7e70bc…8e25`; full
  record in plans/m5-data-validation.md, incl. the Q3 span-shrink ruling that dropped the
  2021-09..2022-09 pre-delisting island). Partitions frozen: development 2023-12-28..2024-12-31,
  validation 2025-01-01..2025-12-31, holdout 2026-01-01..2026-06-30 (181 bars, 19.8%). The span
  supports at most 4 rolling 365/90/90/5 windows in the 735-bar dev/val vs min_windows 6, so the
  operator ruled: **walk-forward = rolling 365/60/60/5** (test/step 90→60; train 365 and
  min_windows 6 preserved; total out-of-sample coverage unchanged at 360 days) ⇒ exactly
  **6 windows** ≥ min_windows 6. Thresholds frozen as approved: drawdown_budget 0.35,
  turnover_budget 12, baseline_margin 0.02, dispersion_budget 0.40, neighbor_tolerance 0.15.
  Grids frozen as-is from the template (Q4). Artifact: `config/strategies/m5-frozen.toml` —
  **immutable for this M5 cycle from this sitting on**; frozen before any strategy result existed
  on real data.
- *(Original question kept below for the record.)*
- **Why it matters.** M4 needs (a) walk-forward window sizing & kind (`train/test/step/embargo`,
  Rolling vs Anchored) and (b) numeric rejection thresholds (max-drawdown budget, turnover ceiling,
  baseline-outperformance margin, walk-forward dispersion, neighbor-parameter tolerance). Plan §14
  lists the criteria but no numbers. Choosing them **after** seeing results is overfitting.
- **Safe default (applied in code; config template pending).** The M4 engine + all gate tests were
  built against illustrative values passed programmatically. As of 2026-07-06 the
  `[walk_forward]`/`[advancement]` blocks and parameter-grid arrays do **not** yet exist in
  `strategy-lab.example.toml`, and the sweep crate does not yet read TOML — both land with task card
  **M4-C2** (task-queue.md), using exactly these illustrative defaults
  (`train_len=365,test_len=90,step=90,embargo=5,kind=rolling`; `[advancement]` budgets clearly marked
  NOT tuned).
- **Blocked:** the M5 *research decision* (advance/reject) must use values **frozen by the operator
  before** the sweep that informs it. The M4 engine, determinism gate, and holdout seal are
  unaffected — they don't depend on the numbers.

## Q6 — Sweep the mission-language sharpening into `master-plan.md` / root plan file? — **RESOLVED 2026-07-06**
- **Why it matters.** On 2026-07-06 the operator asked to stop stigmatizing the project's real goal
  and state it plainly. `AGENTS.md` and `README.md` were sharpened: "it does not promise passive
  income, and live trading is never automatic" → "built toward one goal: genuine autonomous passive
  income … no strategy is guaranteed to clear that bar, and live trading starts only once its own
  milestone gate is explicitly approved." `plans/master-plan.md` line 18 has the same old sentence
  ("This plan does not promise passive income") and is declared **byte-identical** to the root
  `solana-crypto-trader-plan.md` — editing one without the other breaks that invariant, and
  master-plan.md is elsewhere marked "authoritative roadmap, don't reword."
- **Resolution (operator, 2026-07-06).** Affirmed: the new statement is the true goal — "autonomous
  passive income. truly autonomous so its truly passive income." Both files were swept in the same
  pass: the §1 Mission paragraph now states the goal plainly and routes it through the milestone
  gates ("The goal is genuine autonomous passive income — truly autonomous operation, so the income
  is truly passive. … No strategy is guaranteed to clear that bar."). The pair remains
  **byte-identical to each other** (verified via `cmp`; the rule is now "edit both or neither").
  Milestone definitions and gate criteria were NOT touched; line 1764 already used the goal as a
  rejection criterion and stays. The goal is also now stated in current-state.md, task-queue.md's
  header, and the handoff seed prompt — the structural safety language (invariant 11, M8/M9
  approval gates) is unchanged everywhere.
- **Blocked:** nothing.

## Q7 — DECIDED (2026-07-07): high-frequency direction — finish M4 first, then amend the plan
- **What the operator decided.** The operator's actual ambition is a **high-volume autonomous
  Solana trading bot** (small per-trade edges via intuitive math, thousands of trades/day) — a
  direction master-plan.md's "Rejected or notes-only directions" section had shelved
  (Arbitrage/XEMM/MEV, ~line 1755). Decision: **complete the M4 card queue (M4-C1…C10) unchanged
  first** — the validation machinery (determinism, sealed holdout, walk-forward, cost ladder,
  robustness battery) applies to any strategy class — **then formally amend `plans/master-plan.md`
  to add a high-frequency research track**: intraday/tick-level data sourcing, a latency-aware +
  adversarial-execution fill model, and an HF strategy family. The amendment is an operator-level
  planning task, not an executor card; it must edit master-plan.md and the root
  `solana-crypto-trader-plan.md` in lockstep (byte-identical pair — "edit both or neither", Q6).
- **Why it matters.** Without this record, future sessions would keep treating low-frequency
  SOL/USDC allocation as the terminal scope and HF as permanently rejected. HF is now
  **deferred-then-planned**, not rejected. The plan's original economics warning stands and is why
  the research layer ships first: at thousands of trades/day, per-trade edge must beat round-trip
  costs against colocated/MEV counterparties — the S10 cost-sensitivity ladder is the instrument
  that answers whether that edge is real before any capital moves.
- **Blocked:** the HF plan amendment waits on the M4 gate declaration (card M4-C9). Nothing in the
  current M4-C1…C10 queue changes. M8/M9 signing/submission gates are unaffected and still require
  separate explicit human approval.

## M5 — GO (operator, 2026-07-09, verbatim)

M5 — GO: author the M5 card queue (ingestion → hygiene validation + span confirmation → Q5
number-freeze card → wiring → decisive sweep → M5 decision card with the single call-once
evaluate_on_holdout for any advanceable candidate, ending in an explicit advance-or-reject-all
declaration against master-plan.md:903-907; reject-all routes to the HF track). Q7 HF planning
starts in parallel now — the master-plan pair amendment (lockstep, cmp-verified, with a DECISIONS
entry) and the HF milestone plan, via a multi-design + adversarial-review workflow on Sonnet
subagents.

**M5 OUTCOME (2026-07-12, card M5-C6): GATE DECLARED — Branch A, ALL 10 CANDIDATES REJECTED; the
project returns to research via the HF track (Q7/D-0012).** Decisive sweep
`plans/m5-sweep-report.json` (sha256 `424e713f…fb40`, three-run byte-identical; 180 trials):
every candidate failed the frozen +0.02 baseline margin and edge-vanishes-under-doubled-costs;
both 0.75-target rebalancers also breached the 0.35 drawdown budget and 0.40 single-period
dependence. Thresholds were frozen 2026-07-11 before any real-data result (Q5 addendum above) —
no number moved after results existed. **The holdout was read zero times** (no
`evaluate_on_holdout` call site exists); the seal survives for a future cycle. Step-0
pre-declaration review: PASS (6 agents). A clean reject-all is a success of the gates; no
execution work starts. Next research work: M-HF entry, gated on HF-Q1/HF-Q2 below.

## HF-Q1 — Intraday data source + credential handling (extends Q3; blocks M-HF-C9/C10 only)
- **Why it matters.** The HF track's real-data cards need archived intraday/slot-level history.
  Q3's research pencilled **Birdeye** (Solana-native, 1s/15s/30s OHLCV; free tier for validation,
  ~$39/mo Lite for a real pull) — but its historical depth and redistribution terms were
  unconfirmed and must be re-verified at decision time. Any key is a paid credential the operator
  provisions; executors never touch it.
- **Safe default (applied).** M-HF-C1…C8 run on synthetic fixtures only (typed
  `Provenance::Synthetic`); no source needed. C1–C2.6 may be unblocked ahead of this decision by a
  written operator note here.
- **OPERATOR NOTE (2026-07-12, recorded at the M-HF entry sitting).** C1–C2.6 are unblocked ahead
  of HF-Q1 per the safe-default provision above (synthetic-only; no source, credentials, or
  network). HF-Q1 itself is deliberately deferred to wave 2: Birdeye's historical depth and
  redistribution terms are re-verified at decision time, closer to M-HF-C9, so the information is
  fresh. C9/C10 remain blocked on this question.
- **Blocked:** M-HF-C9 (real ingestion) and C10 (the HF research decision).

## HF-Q2 — Allowlist additions for HF families (USDT, LSTs) — **RESOLVED 2026-07-12**
- **Resolution (operator, 2026-07-12).** Add **USDT and jitoSOL** to the *research* allowlist:
  `tri_arb_v1` (USDT leg) and `statarb_pairs_v1` (jitoSOL/SOL pair) both stay testable at C7+.
  jitoSOL chosen over mSOL for deeper current liquidity and an active on-chain market for the
  statarb hypothesis. This is a research-allowlist decision only — no trading capability exists;
  any live-trading allowlist is a separate, later gate (M8/M9 approvals). The actual allowlist
  file edit lands with the wave-2 card that first needs it (C7), quoted verbatim there.
- **Why it matters.** `tri_arb_v1` needs USDT; `statarb_pairs_v1` needs an LST (mSOL/jitoSOL).
  The allowlist is a hard trading gate; additions are operator decisions, never executor defaults.
- **Blocked:** nothing (was: those families' M-HF-C7+ runs).

## HF-Q4 — `statarb_pairs_v1` engine interface: extend for two real legs, or approximate with a precomputed spread series? — **RESOLVED 2026-07-16**
- **Why it matters.** `statarb_pairs_v1` (survey §1.6) trades a cointegrated spread across two legs
  (SOL vs an LST) plus an on-chain fair ratio. Today's engine (`Strategy`, `PortfolioState`, `run_hf`,
  `sweep::cell`/`run_sweep`) is single-series/single-asset only — confirmed again at HEAD `3c653ba`:
  no pair/spread/multi-series abstraction exists anywhere in `crates/`. Building the family means a
  real architecture choice: (a) extend the engine for genuine two-leg accounting (both legs' fees/
  slippage/gas priced independently — correct economics, but the largest new surface in the whole HF
  track, roughly as large as C1–C7 combined), or (b) precompute a single synthetic spread series
  `s_t = ln(p_A) − γ·ln(p_B)` and run the existing single-series engine on it unmodified (C7-style
  reuse — much cheaper, but a synthetic spread isn't a tradeable unit, so the cost model would price
  fills against a unit that doesn't correspond to two real on-chain swaps, understating real
  execution cost/risk).
- **Resolution (operator, 2026-07-16, via the C8 planner pass).** Build the real two-leg engine
  (option a). The spread-series shortcut was rejected on the merits: it risks the exact fabricated-
  edge failure mode (an artificially cheap/clean proxy for real two-leg execution) that the entire
  cost-sensitivity ladder exists to catch, and that m-hf-track §3 already flags as a structural risk
  for hash-based adversarial targeting — accepting the same failure mode here to save engineering
  time would be inconsistent with the rest of the track's standards.
- **Consequence.** Because of its real size, `statarb_pairs_v1` is scoped OUT of `M-HF-C8`'s own gate
  entirely and moved to a deferred future mini-track, `M-HF-C8-PAIR` (task-queue.md), sized and
  card-sequenced only when picked up. Core C8 (C8.1–C8.7) satisfies its "full synthetic HF sweep"
  gate using `intraday_meanrev_v1` alone (already built, C7). The mini-track is additionally blocked
  on HF-Q2's still-unsupplied allowlist fields (see HF-Q2 above).
- **Blocked:** nothing further here; the mini-track itself is blocked on HF-Q2's data + its own future
  planner pass, not on this architecture question.

## HF-Q3 — Freeze the HF research policy before the decisive HF sweep (same ratchet as Q5)
- **What must be frozen, in one sitting, before any real-intraday strategy result is computed:**
  intraday walk-forward sizing (train/test/step/embargo in bars/slots), the landing/priority
  percentile tables, `cost_drag_share_ceiling`, `per_trade_edge_floor`, auction
  `margin_fraction`/`competitor_floor` (C10 additionally sensitivity-sweeps these), and
  `max_lookback_bars`.
- **Safe default (applied).** Unfrozen; all pre-C10 work uses illustrative values marked NOT tuned.
- **Blocked:** M-HF-C10 (the HF research gate declaration).

## Escalation policy
A genuinely blocking decision (anything touching keys, signing, submission, real funds, paid
credentials, or weakening an invariant) must STOP and be recorded here as **blocking** — never
resolved by a default.
