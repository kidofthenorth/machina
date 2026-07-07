# Operator questions

Only **real** operator decisions live here. Each has a safe default already applied, so none of these
block autonomous M0–M4 *engine* work. Format: Question · Why it matters · Safe default (applied) ·
What's blocked.

> Status: **0 blocking**, 5 non-blocking open, 1 resolved (Q6). The executor proceeded on safe defaults. Note: Q5's
> walk-forward sizing + rejection thresholds must be **frozen by the operator before the M5 research
> decision** (post-hoc choice = overfitting); they do not block the M4 engine.

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

## Q3 — Real OHLCV data source for SOL/USDC (non-blocking for now)
- **Why it matters.** Honest backtests (M4–M5) need archived candles. Options in the plan: Birdeye,
  CoinGecko/GeckoTerminal, Bitquery. Some need an API key (a paid credential the executor must not
  create or store).
- **Safe default (applied).** M1–M3 use **tiny synthetic, checked-in fixtures only**; no network and
  no credentials. Real-data ingestion is deferred to a later, operator-approved task.
- **Blocked:** real-data backtests and any M4+ statistical claims. M0–M3 are unaffected.

## Q4 — Strategy parameter values + sweep grids (non-blocking)
- **Why it matters.** `config/strategies/strategy-lab.example.toml` contains illustrative parameters
  (SMA period, weights, bands). These are placeholders, not tuned values, and must not be read as a
  recommendation. M4 also adds parameter *grids* (sets of values to sweep) over these.
- **Safe default (applied).** Ship conservative illustrative defaults/grids; tuning/validation is
  M4–M5. No value here is a recommendation.
- **Blocked:** nothing; no profitability is claimed.

## Q5 — M4 research policy: walk-forward sizing + rejection thresholds (non-blocking for the engine; FREEZE before M5)
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

## Escalation policy
A genuinely blocking decision (anything touching keys, signing, submission, real funds, paid
credentials, or weakening an invariant) must STOP and be recorded here as **blocking** — never
resolved by a default.
