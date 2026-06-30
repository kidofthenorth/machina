# Handoff — for a new chat continuing machina

You are picking up an in-progress build. This is the single entry point. Read the four pointer files
below, confirm the gates are green, then continue in milestone order. **Do not commit or push — the
operator commits.**

---

## 30-second orientation

**machina** (codename `solana-crypto-trader`) is a paper-first, Solana-focused crypto **trading
research platform** in Rust. It exists to discover whether a strategy has a robust edge after costs,
latency, slippage, and operational failure. It does **not** promise income, and **live trading is
never automatic**.

**The one rule that cannot be broken:** money-moving capability is gated by milestone **and** by
explicit human approval. No key loading, signing, or transaction submission exists — or may be added
— before the research phase (M0–M5) is complete and separately approved. Building the software never
authorizes trading. Full invariants: [docs/invariants.md](../docs/invariants.md).

## Read these first (source of truth)

1. [plans/current-state.md](current-state.md) — live per-milestone status, gates, blockers, next command.
2. **[plans/m4-sweep.md](m4-sweep.md) — the ACTIVE milestone plan (M4). 12 subtasks S1–S12; S1–S8 DONE,
   next is S9 (holdout gate).** This is your working source of truth right now.
3. [plans/master-plan.md](master-plan.md) — authoritative M0–M11 roadmap & architecture.
4. [plans/task-queue.md](task-queue.md) — actionable status table (TODO/DOING/DONE/DEFERRED/BLOCKED).
5. [docs/architecture-index.md](../docs/architecture-index.md) + [docs/invariants.md](../docs/invariants.md) — module map + hard invariants.

Audit + staged-diff record: [plans/review-packet.md](review-packet.md). Chronological log:
[plans/worklog.md](worklog.md). Open operator questions: [plans/questions.md](questions.md).

## What's accomplished (M0–M2 complete; M3 scaffolds; **M4 in progress — S1–S8 done**)

- **M0 Bootstrap.** 7-crate Cargo workspace; pinned toolchain; rustfmt; CI (`fmt + clippy -D warnings
  + test + no-execution-deps` scan); README/DECISIONS/AGENTS/CLAUDE; 4 secret-safe `*.example.toml`
  templates; 4 JSON Schemas (run-result/route-quote/execution-event/wallet-snapshot) + validation tests.
- **M1 Data foundation.** `research-core` (fixed-point money via `rust_decimal`, token/mint, OHLCV bar,
  UTC time) + `market-data` (allowlist loader; series validation incl. empty/OHLC/duplicate/unsorted/
  **gap-missing**; token-decimal + disallowed-token checks; checked-in fixtures).
- **M2 Deterministic spot simulator.** `portfolio` (deterministic `CostModel` fill model;
  `PortfolioState` accounting with oversell/unaffordable/gas guards; next-bar `simulator::run`; equity
  curve; round trips) + `metrics` (return/drawdown in `Decimal`; Sharpe/Sortino/vol in display-only f64).
- **M3 Strategy Lab (scaffolds).** `Strategy` trait (intent only); baselines `hold_usdc` /
  `buy_and_hold_sol` / `static_50_50` / `dca_sol`; `regime::classify` scaffold; `trend_alloc_v1`;
  `threshold_rebalance_v1`; `results` RunResult model; `cli` deterministic demo.
- **M4 — sweep core + walk-forward (S1–S8 of 12 done).** Designed via a 3-architecture + adversarial
  workflow (see [m4-sweep.md](m4-sweep.md)). Shipped, each a small green diff:
  - **S1** additive `portfolio::RunOutput.traded_notional_quote` (the correct turnover base — the
    adversary proved round-trip turnover reads ~0 for zero-round-trip rebalancers).
  - **S2** `research-core::time::{parse_ymd, civil_date_to_unix}` (Hinnant inverse; pure-int; zero deps).
  - **S3** `results` turnover plumbed into `MetricsReport.turnover` (unchanged run-result schema).
  - **S4** new `crates/sweep` crate, wired into the workspace (no-exec-deps scan still clean).
  - **S5** `sweep::param` — `ParamPoint`/`ParamGrid` fixed-order Cartesian product + intent-only
    `build_strategy` bridge + scale-canonical `param_id`.
  - **S6** `sweep::{cell, turnover}` — pure `eval_cell` (wraps `run`+`Metrics`), `CellResult`
    (Decimal keys, non-finite f64→None), exact `turnover_ratio`.
  - **S7 — DETERMINISM GATE PROVEN.** `sweep::parallel` — `run_in_parallel` over `std::thread::scope`
    (contiguous index chunks, disjoint writes, read back in index order), `Parallelism`, `SweepCell`,
    `run_cells`. Gate test: a 48-cell sweep is byte-identical across thread counts {1,2,3,7,8},
    parallel==sequential, repeated-run identical, + anti-vacuous guard. **Zero new deps** (rayon
    rejected; record as D-0009 in S12).
  - **S8 — Walk-forward windows.** `sweep::window` — `Window { train, test: Range<usize> }`,
    `WalkForward { kind, train_len, test_len, step, embargo }`, `WindowKind::{Rolling (default),
    Anchored}`, `WindowError`. `new()` rejects bad params (zero lengths, `step<test_len` overlap);
    `windows(n_bars)` is pure + total + defensive (saturating math; struct-literal bypass → empty,
    never an infinite loop). Invariants structural: `train.end + embargo == test.start`, ordered +
    non-overlapping test ranges, in-bounds. **Zero new deps.** Adversarially verified (5-lens +
    adjudicator): impl correct; two test-coverage gaps (embargo magnitude, Anchored `step` coupling)
    found and closed.
- **Quality.** A 6-lens adversarial verification workflow ran on M0–M3 → **PASS, no blockers**; a
  5-lens + adjudicator workflow verified S8 → impl correct, found & fixed two test gaps.

## Verified state (re-confirm on a fresh checkout)

```bash
cargo fmt --all --check                                  # clean
cargo clippy --all-targets --all-features -- -D warnings # clean
cargo test --workspace --all-features                    # 121 passed, 0 failed (85 baseline + 36 from M4 S1–S8)
cargo run -p cli -- demo                                 # deterministic, schema-valid RunResult
# no-execution-deps scan (CI parity) — must print OK:
pattern='solana-sdk|solana-client|solana-program|solana-rpc|jupiter|jito|ed25519-dalek|keypair|bip39|tiny-bip39|secp256k1'
grep -REn --include='Cargo.toml' "$pattern" . | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#' || echo OK
```

Repo state: **no commits yet** (BASE=EMPTY); the operator commits. To stage the M4 work so far:
the new `crates/sweep/` (untracked — `git add crates/sweep`), plus modified `Cargo.toml`, `Cargo.lock`,
and `plans/*` + the S1–S3 edits in `crates/{portfolio,research-core,results,cli}`. **Never** `git add -A`;
**never** stage `.claude/`. Stage explicit paths only.

## What's next — continue M4 from S9 (see [m4-sweep.md](m4-sweep.md) §14 for the subtask table)

Pick up the **second half of M4** (research-rigor + reporting + wiring). Each is a small diff that
must keep the workspace green (fmt + clippy `-D warnings` + test). Suggested order = the plan's:

- **S8 — Walk-forward window model — ✅ DONE.** `sweep::window` shipped: `Window`, `WalkForward`,
  `WindowKind::{Rolling (default), Anchored}`, `WindowError`; exact-enumeration + grid tests pin the
  embargo gap, the `step`-coupled shapes, ordering/non-overlap, and bad-param rejection. Adversarially
  verified. **S9 below depends on it.**
- **S9 — HOLDOUT GATE (next)** (`sweep/src/partition.rs`, `config.rs`, `tests/holdout_sealing.rs`). Physical
  partition (holdout moved into a *separate* `Vec<Bar>`), `seal_holdout(self) -> (DevValidation, Sealed)`,
  `evaluate_on_holdout(sealed: Sealed, …)` **consume-by-value (call-once, M5-only)**. Tests:
  read-counter==0 + digest unchanged after the full pipeline; `compile_fail` doctest; overlap rejected.
  Depends on S2 (done), S6 (done), S8 (done).
- **S10 — Cost/fee sensitivity ladder** (`sensitivity.rs`, `baseline.rs`): `before_costs=zero`, `base`,
  `doubled` (each cost field ×2, integer-exact); baselines re-run cost-matched. Depends on S6 (done).
- **S11 — Advancement/rejection report + NEW `schemas/sweep-report.schema.json`** (`advance.rs`,
  `report.rs`, `tests/schema_validation.rs`). Decimal-only selection keys + an f64-comparison audit
  test; `trial_count` recorded; robustness language only (invariant 11). Depends on S7 (done), S9, S10.
  **Adding the schema is a deliberate contract act (D-0001).**
- **S12 — CLI `sweep`/`sweep-verify`** + refresh `plans/*`, `docs/architecture-index.md`, and record
  **DECISIONS D-0009** (sweep determinism, rayon rejection, the additive `RunOutput` field, new schema).
  Depends on S11.

Beyond M4: **M5** research-decision gate (needs the operator to FREEZE walk-forward sizing + rejection
thresholds first — questions.md Q5); **M6** Jupiter shadow (re-verify plan §22 sources; no signing).
**BLOCKED — M8/M9** devnet/canary signing + submission: **separate explicit human approval. Do not start.**
Real OHLCV ingestion is deferred pending an operator data-source decision (questions.md Q3); M1–M4 use
tiny synthetic checked-in fixtures only.

## Operating contract (executor rules — keep these)

- Never `git commit`/`git push`; stage **explicit paths only**, never `git add -A`; never stage `.claude/`.
- Never create/load/store private keys, seed phrases, wallet files, API secrets, or paid credentials.
- No signing, no mainnet (or any) submit path, no real-money execution path before the gated +
  approved milestones. Setting an RPC URL alone must never enable signing/submission.
- Determinism is non-negotiable: same input → identical orders/fills/balances/equity/metrics;
  fixed-point money (never f64 for money); ordered collections; no RNG/clock in canonical runs.
- Strategies emit intent (target weights) only — no keys, no RPC. Never trade a non-allowlisted token.
- Work in milestone order; small diffs; refresh `plans/current-state.md` + `worklog.md` in the same diff.
- No profitability claims from in-sample results.

**M4-specific (don't regress what S1–S8 established):**
- Parallelism is `std::thread::scope` only — **no new dependency** (rayon is rejected, D-0009).
  Any sweep output must stay byte-identical parallel==sequential==repeated; the gate lives in
  `crates/sweep/tests/determinism.rs` and must use **explicit** thread counts, never `available_parallelism`.
- Turnover comes from `RunOutput.traded_notional_quote`, **never** from `round_trips` (rebalancers
  close zero round trips while churning — round-trip turnover undercounts them to ~0).
- All sort/threshold/selection keys are `Decimal`; `f64` is display-only and non-finite values are
  normalized to `None` at `CellResult` construction (so structural equality can't flake on `NaN`).
- The holdout (S9) must be sealed: parameter selection only ever sees development/validation; the
  single holdout reader consumes its seal by value (M5-only). The M4 CLI never touches the holdout.
- Walk-forward windows (S8) are index ranges over the dev/val array only: keep `train.end + embargo
  == test.start`, test ranges ordered + non-overlapping, and never index past `n_bars`. S9 runs
  `WalkForward::windows` over the dev/val partition length — the holdout is a *separate* allocation,
  so windows are structurally incapable of reaching it.

---

## Seed prompt for the new chat (paste this)

> You are the EXECUTOR continuing the **machina** (`solana-crypto-trader`) repo. Read
> `plans/handoff.md`, then `plans/current-state.md`, **`plans/m4-sweep.md` (the active milestone plan)**,
> `plans/task-queue.md`, and `docs/invariants.md`. M0–M2 are complete, M3 scaffolds are in place, and
> **M4 is underway: subtasks S1–S8 (the deterministic sweep core + determinism gate + walk-forward
> window model) are DONE and green** — `cargo fmt --check`, `cargo clippy --all-targets --all-features
> -- -D warnings`, `cargo test --workspace --all-features` = **121 passing**, demo byte-identical,
> no-execution-deps scan clean. The repo has no commits — **do not commit or push; the operator commits**
> (stage explicit paths only, never `.claude/`). First re-confirm the gates on a fresh checkout, then
> continue M4 from **S9 (the holdout gate — physical partition + consume-by-value seal)** per
> `plans/m4-sweep.md` §14, in small green diffs. Honor the
> operating contract in `plans/handoff.md` — including the M4-specific rules: parallelism is
> `std::thread::scope` (zero new deps; rayon rejected); turnover comes from `RunOutput.traded_notional_quote`
> not `round_trips`; selection keys are `Decimal`; and the S9 holdout must be physically sealed and
> consume-by-value (M5-only). Update `plans/current-state.md`, `plans/m4-sweep.md` (subtask status),
> and `plans/worklog.md` as part of "done." Do NOT start M8/M9 (signing/submit) — that needs separate
> explicit human approval.
