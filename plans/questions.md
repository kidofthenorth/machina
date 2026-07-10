# Operator questions

Only **real** operator decisions live here. Each has a safe default already applied, so none of these
block autonomous M0–M4 *engine* work. Format: Question · Why it matters · Safe default (applied) ·
What's blocked.

> Status: **0 blocking**, 6 non-blocking open (Q1, Q2, Q4, HF-Q1, HF-Q2, HF-Q3), 4 resolved
> (Q3, Q5, Q6, Q7) + **M5 GO recorded 2026-07-09** (see the M5 block after Q7). Q5's numbers get
> their final freeze at span confirmation (same sitting as the ingestion hygiene check, before any
> strategy result) — the one-way ratchet is stated in the Q5 resolution. The Q7 amendment was made
> 2026-07-09 (D-0012, plans/m-hf-track.md); HF-Q1/Q2/Q3 gate the HF track's later cards.

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

## HF-Q1 — Intraday data source + credential handling (extends Q3; blocks M-HF-C9/C10 only)
- **Why it matters.** The HF track's real-data cards need archived intraday/slot-level history.
  Q3's research pencilled **Birdeye** (Solana-native, 1s/15s/30s OHLCV; free tier for validation,
  ~$39/mo Lite for a real pull) — but its historical depth and redistribution terms were
  unconfirmed and must be re-verified at decision time. Any key is a paid credential the operator
  provisions; executors never touch it.
- **Safe default (applied).** M-HF-C1…C8 run on synthetic fixtures only (typed
  `Provenance::Synthetic`); no source needed. C1–C2.6 may be unblocked ahead of this decision by a
  written operator note here.
- **Blocked:** M-HF-C9 (real ingestion) and C10 (the HF research decision).

## HF-Q2 — Allowlist additions for HF families (USDT, LSTs)
- **Why it matters.** `tri_arb_v1` needs USDT; `statarb_pairs_v1` needs an LST (mSOL/jitoSOL).
  The allowlist is a hard trading gate; additions are operator decisions, never executor defaults.
- **Safe default (applied).** No additions; families needing them are skipped until recorded here.
- **Blocked:** those families' M-HF-C7+ runs only.

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
