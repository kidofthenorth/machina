# Operator questions

Only **real** operator decisions live here. Each has a safe default already applied, so none of these
block autonomous M0–M3 work. Format: Question · Why it matters · Safe default (applied) · What's
blocked.

> Status: **0 blocking**, 4 non-blocking. The executor proceeded on safe defaults.

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

## Q4 — Strategy parameter values (non-blocking)
- **Why it matters.** `config/strategies/strategy-lab.example.toml` contains illustrative parameters
  (SMA period, weights, bands). These are placeholders, not tuned values, and must not be read as a
  recommendation.
- **Safe default (applied).** Ship conservative illustrative defaults; tuning/validation is M4–M5.
- **Blocked:** nothing; no profitability is claimed.

---

## Escalation policy
A genuinely blocking decision (anything touching keys, signing, submission, real funds, paid
credentials, or weakening an invariant) must STOP and be recorded here as **blocking** — never
resolved by a default.
