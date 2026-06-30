# Operator questions

Only **real** operator decisions live here. Each has a safe default already applied, so none of these
block autonomous M0–M4 *engine* work. Format: Question · Why it matters · Safe default (applied) ·
What's blocked.

> Status: **0 blocking**, 5 non-blocking. The executor proceeded on safe defaults. Note: Q5's
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
- **Safe default (applied).** Build the M4 engine + all gate tests against illustrative defaults in
  `strategy-lab.example.toml` (`[walk_forward]` `train=365,test=90,step=90,embargo=5,kind=rolling`;
  `[advancement]` illustrative budgets, clearly marked NOT tuned). The engine reads them from config;
  none are hard-coded.
- **Blocked:** the M5 *research decision* (advance/reject) must use values **frozen by the operator
  before** the sweep that informs it. The M4 engine, determinism gate, and holdout seal are
  unaffected — they don't depend on the numbers.

---

## Escalation policy
A genuinely blocking decision (anything touching keys, signing, submission, real funds, paid
credentials, or weakening an invariant) must STOP and be recorded here as **blocking** — never
resolved by a default.
