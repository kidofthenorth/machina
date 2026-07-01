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
2. **[plans/m4-sweep.md](m4-sweep.md) — the ACTIVE milestone plan (M4). 12 subtasks S1–S12; S1–S9 DONE,
   next is S10 (cost/fee sensitivity ladder).** This is your working source of truth right now.
3. [plans/master-plan.md](master-plan.md) — authoritative M0–M11 roadmap & architecture.
4. [plans/task-queue.md](task-queue.md) — actionable status table (TODO/DOING/DONE/DEFERRED/BLOCKED).
5. [docs/architecture-index.md](../docs/architecture-index.md) + [docs/invariants.md](../docs/invariants.md) — module map + hard invariants.

Audit + staged-diff record: [plans/review-packet.md](review-packet.md). Chronological log:
[plans/worklog.md](worklog.md). Open operator questions: [plans/questions.md](questions.md).

## What's accomplished (M0–M2 complete; M3 scaffolds; **M4 in progress — S1–S9 done**)

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
- **M4 — sweep core + walk-forward + holdout seal (S1–S9 of 12 done).** Designed via a 3-architecture + adversarial
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
  - **S9 — HOLDOUT GATE (invariant 11's enforcement point).** `sweep::{config, partition}`:
    `PartitionSpec::{ByIndex, ByDate}` (ByDate via the S2 `parse_ymd`); `PartitionedBars::from_spec`
    validates the series via canonical `market_data::validate_series` (sorted **and unique**) then
    **physically moves** the holdout into a SEPARATE `Vec<Bar>` (`split_off`, not an aliasing borrow);
    rejects overlap/empty/out-of-bounds. `seal_holdout(self) -> (DevValidation, Sealed)` — `DevValidation`
    has NO holdout accessor and walk-forward runs over dev/val length only, so it is structurally
    incapable of reaching the holdout. `evaluate_on_holdout(sealed: Sealed, …)` consumes the seal **by
    value** (call-once; M5-only — the M4 CLI never calls it); the single read passes a `Cell<u32>`
    counter gateway. Gate green: read-counter==0 + digest unchanged after the full sweep+selection
    pipeline; 3 `compile_fail` doctests (no selection→holdout path, no seal fabrication, call-once) each
    with a `no_run` canary; overlap rejected. **Zero new deps.** Adversarially reviewed (6-lens +
    skeptics, 12 agents): **seal sound, no blockers**; 3 minor/nit fixes applied (delegate to
    `validate_series` → duplicate-ts rejection; real post-split structural assert; within-process-only
    digest doc).
- **Quality.** A 6-lens adversarial verification workflow ran on M0–M3 → **PASS, no blockers**; a
  5-lens + adjudicator workflow verified S8 → impl correct, found & fixed two test gaps; a 6-lens +
  per-finding-skeptic workflow verified the S9 seal → **sound**, 3 minor/nit hardening fixes applied.

## Verified state (re-confirm on a fresh checkout)

```bash
cargo fmt --all --check                                  # clean
cargo clippy --all-targets --all-features -- -D warnings # clean
cargo test --workspace --all-features                    # 144 passed, 0 failed (85 baseline + 59 from M4 S1–S9)
cargo run -p cli -- demo                                 # deterministic, schema-valid RunResult
# no-execution-deps scan (CI parity) — must print OK:
pattern='solana-sdk|solana-client|solana-program|solana-rpc|jupiter|jito|ed25519-dalek|keypair|bip39|tiny-bip39|secp256k1'
grep -REn --include='Cargo.toml' "$pattern" . | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#' || echo OK
```

Repo state: **no commits yet** (BASE=EMPTY); the operator commits. To stage the M4 work so far:
the new `crates/sweep/` (untracked — `git add crates/sweep`), plus modified `Cargo.toml`, `Cargo.lock`,
and `plans/*` + the S1–S3 edits in `crates/{portfolio,research-core,results,cli}`. **Never** `git add -A`;
**never** stage `.claude/`. Stage explicit paths only.

## What's next — continue M4 from S10 (see [m4-sweep.md](m4-sweep.md) §14 for the subtask table)

Pick up the **second half of M4** (research-rigor + reporting + wiring). Each is a small diff that
must keep the workspace green (fmt + clippy `-D warnings` + test). Suggested order = the plan's:

- **S9 — Holdout gate — ✅ DONE.** `sweep::{config, partition}`: `PartitionSpec::{ByIndex, ByDate}`;
  `PartitionedBars::from_spec` validates via canonical `market_data::validate_series` then **physically
  moves** the holdout into a *separate* `Vec<Bar>` (`split_off`); `seal_holdout(self) -> (DevValidation,
  Sealed)` (no holdout accessor on `DevValidation`); `evaluate_on_holdout(sealed: Sealed, …)`
  consume-by-value (call-once, M5-only). Gate: read-counter==0 + digest unchanged after the full
  pipeline; 3 `compile_fail` doctests; overlap rejected. Adversarially reviewed (6-lens + skeptics):
  seal sound, 3 minor/nit fixes applied. **S11 depends on it.**
- **S10 — Cost/fee sensitivity ladder (next)** (`sensitivity.rs`, `baseline.rs`): `before_costs=zero`,
  `base`, `doubled` (each cost field ×2, integer-exact); baselines re-run cost-matched. Depends on S6 (done).
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
- The holdout (S9, **done**) is sealed and must stay that way: parameter selection only ever sees
  `&DevValidation` (development/validation); `evaluate_on_holdout` consumes its seal by value (M5-only)
  and **must not be wired into any M4 CLI path** (S12). Don't add a holdout accessor to `DevValidation`.
- Walk-forward windows (S8) are index ranges over the dev/val array only: keep `train.end + embargo
  == test.start`, test ranges ordered + non-overlapping, and never index past `n_bars`. S9 runs
  `WalkForward::windows` over the dev/val partition length — the holdout is a *separate* allocation,
  so windows are structurally incapable of reaching it.

---

## Seed prompt for the new chat — S10, the cost/fee sensitivity ladder (paste this)

> You are the EXECUTOR continuing the **machina** (`solana-crypto-trader`) repo — a paper-first,
> Solana-focused crypto trading-**research** platform in Rust (no keys/signing/RPC/network anywhere).
> Read `plans/handoff.md`, then `plans/current-state.md`, **`plans/m4-sweep.md` — the active milestone
> plan, especially §9 "Fee/cost sensitivity" and §14 row S10**, then `plans/task-queue.md` and
> `docs/invariants.md`.
>
> **State:** M0–M2 complete, M3 scaffolds in place, and **M4 subtasks S1–S9 are DONE and green** —
> the deterministic sweep core (S1–S7, incl. the determinism gate), the walk-forward window model (S8),
> and the **sealed holdout gate (S9)**. `cargo fmt --all --check`,
> `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` =
> **144 passing**, `cargo run -p cli -- demo` byte-identical, no-execution-deps scan clean. The repo has
> **no commits** — **do not commit or push; the operator commits** (stage explicit paths only, never
> `.claude/`, never `git add -A`).
>
> **First** re-confirm the gates on a fresh checkout. **Then implement S10 — the COST/FEE SENSITIVITY
> LADDER — in small green diffs** (each keeps fmt + clippy `-D warnings` + test green), per
> `m4-sweep.md` §9/§14. New files: `crates/sweep/src/sensitivity.rs`, `crates/sweep/src/baseline.rs`.
> The design:
> - `cost_scenarios(base) -> Vec<(ScenarioId, CostModel)>` builds a fixed ladder by **exact integer
>   scaling** (u32/i64 fields only — **no f64**): `before_costs` = `CostModel::zero()`, `base` = as
>   configured, `doubled` = each cost field ×2 (the mandated doubled-costs survival test); optional
>   `doubled_slippage` / `doubled_priority` / `scaled(num, den)`.
> - Every `(family, param, window)` cell is evaluated across **all** scenarios; **baselines are re-run
>   cost-matched** under each scenario (baselines: `hold_usdc` / `buy_and_hold_sol` / `static_50_50` /
>   `dca_sol`). Reported per candidate: a `fee_sensitivity` block (`total_return` / `turnover` /
>   `n_trades` / `fees_paid_usdc` / `slippage_paid_usdc` / `priority_fees_paid_sol` under base vs
>   doubled), `return_drag_doubled = return_base - return_doubled` (a `Decimal` delta, **not** framed as
>   profit), and `survives_doubled: bool`. **No profitability claim (invariant 11).**
>
> **S10 gate:** `doubled` doubles each cost field **exactly** (integer-exact, hand-checked);
> `before_costs == CostModel::zero()`; doubled shows **≥ base fees** and **≤ base net return**; all keys
> `Decimal`. Depends on S6 (done).
>
> Honor the operating contract in `plans/handoff.md`, incl. the **M4-specific rules**: parallelism is
> `std::thread::scope` (zero new deps; rayon rejected, D-0009); turnover comes from
> `RunOutput.traded_notional_quote`, never `round_trips`; all sort/threshold/selection keys are
> `Decimal` (`f64` display-only, non-finite→`None`); walk-forward windows keep
> `train.end + embargo == test.start`, ordered/non-overlapping tests, in-bounds; the **holdout stays
> sealed** (`evaluate_on_holdout` is M5-only — never wire it into S10/S11/S12). Update
> `plans/current-state.md`, `plans/m4-sweep.md` (S10 status), `plans/task-queue.md`, and
> `plans/worklog.md` as part of "done", and consider a fresh-session/subagent review before declaring
> S10 done. **Do NOT start M8/M9** (signing/submit) — separate explicit human approval required.
