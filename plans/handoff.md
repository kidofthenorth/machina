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
2. **[plans/m4-sweep.md](m4-sweep.md) — the ACTIVE milestone plan (M4). 12 subtasks S1–S12; S1–S10 DONE,
   next is S11 (advancement/rejection report + new sweep-report schema).** This is your working source of truth now.
3. [plans/master-plan.md](master-plan.md) — authoritative M0–M11 roadmap & architecture.
4. [plans/task-queue.md](task-queue.md) — actionable status table (TODO/DOING/DONE/DEFERRED/BLOCKED).
5. [docs/architecture-index.md](../docs/architecture-index.md) + [docs/invariants.md](../docs/invariants.md) — module map + hard invariants.

Audit + staged-diff record: [plans/review-packet.md](review-packet.md). Chronological log:
[plans/worklog.md](worklog.md). Open operator questions: [plans/questions.md](questions.md).

## What's accomplished (M0–M2 complete; M3 scaffolds; **M4 in progress — S1–S10 done**)

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
- **M4 — sweep core + walk-forward + holdout seal + cost sensitivity (S1–S10 of 12 done).** Designed via a 3-architecture + adversarial
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
  - **S10 — COST/FEE SENSITIVITY LADDER + cost-matched baselines.** `sweep::{sensitivity, baseline}`
    (+ `cell.rs` extracted a shared `pub(crate) eval_strategy` core, so candidates and baselines score
    through byte-identical logic). `cost_scenarios` = {before_costs=`zero()`, base, doubled,
    doubled_slippage, doubled_priority} via exact integer `scale_cost_model` (u32 bps / i64 lamports,
    widened, no f64); `ScenarioMetrics` / `FeeSensitivity` (Decimal `return_drag_*`, baseline-grounded
    `survives_doubled`, not a bare `>0`); the 4 required baselines re-run cost-matched, `label()` ==
    strategy `name()`, `best_baseline_return` floor. **Zero new deps.** Adversarially reviewed (3-lens +
    skeptics, 9 agents): **no code defects**; key finding — **cost monotonicity is conditional** (higher
    costs can suppress a marginal trade → fewer fees / higher return / negative drag), now documented +
    pinned by a regression test; 2 doc nits fixed; 2 findings refuted (overflow/negative-lamport both
    unreachable — `CostModel` has no `Deserialize`).
- **Quality.** A 6-lens adversarial verification workflow ran on M0–M3 → **PASS, no blockers**; a
  5-lens + adjudicator workflow verified S8 → impl correct, found & fixed two test gaps; a 6-lens +
  per-finding-skeptic workflow verified the S9 seal → **sound**, 3 minor/nit hardening fixes applied; a
  3-lens + skeptic workflow verified S10 → **no code defects**, surfaced conditional cost monotonicity
  (documented + regression-tested), 2 doc nits fixed.

## Verified state (re-confirm on a fresh checkout)

```bash
cargo fmt --all --check                                  # clean
cargo clippy --all-targets --all-features -- -D warnings # clean
cargo test --workspace --all-features                    # 153 passed, 0 failed (85 baseline + 68 from M4 S1–S10)
cargo run -p cli -- demo                                 # deterministic, schema-valid RunResult
# no-execution-deps scan (CI parity) — must print OK:
pattern='solana-sdk|solana-client|solana-program|solana-rpc|jupiter|jito|ed25519-dalek|keypair|bip39|tiny-bip39|secp256k1'
grep -REn --include='Cargo.toml' "$pattern" . | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#' || echo OK
```

Repo state: **no commits yet** (BASE=EMPTY); the operator commits. To stage the M4 work so far:
the new `crates/sweep/` (untracked — `git add crates/sweep`), plus modified `Cargo.toml`, `Cargo.lock`,
and `plans/*` + the S1–S3 edits in `crates/{portfolio,research-core,results,cli}`. **Never** `git add -A`;
**never** stage `.claude/`. Stage explicit paths only.

## What's next — continue M4 from S11 (see [m4-sweep.md](m4-sweep.md) §14 for the subtask table)

Pick up the **second half of M4** (research-rigor + reporting + wiring). Each is a small diff that
must keep the workspace green (fmt + clippy `-D warnings` + test). Suggested order = the plan's:

- **S9 — Holdout gate — ✅ DONE.** Physical seal (`sweep::{config, partition}`): holdout moved into a
  *separate* `Vec<Bar>`; `seal_holdout(self) -> (DevValidation, Sealed)` (no holdout accessor);
  call-once `evaluate_on_holdout(sealed: Sealed, …)` (M5-only). Adversarially reviewed → sound.
- **S10 — Cost/fee sensitivity ladder — ✅ DONE.** `sweep::{sensitivity, baseline}` (+ shared
  `eval_strategy` core in `cell.rs`): `cost_scenarios` ladder via exact integer `scale_cost_model`;
  `FeeSensitivity` (Decimal `return_drag_*`, baseline-grounded `survives_doubled`); 4 baselines re-run
  cost-matched. Adversarially reviewed → no code defects; **cost monotonicity is conditional** (trade
  suppression can invert it — see m4-sweep §9), documented + regression-tested.
- **S11 — Advancement/rejection report + NEW `schemas/sweep-report.schema.json` (next)** (`advance.rs`,
  `report.rs`, `tests/schema_validation.rs`). Decimal-only selection keys + an f64-comparison audit
  test; `trial_count` recorded; robustness language only (invariant 11). Depends on S7/S9/S10 (all done).
  **Adding the schema is a deliberate contract act (D-0001).** Reuse `FeeSensitivity`/`best_baseline_return`
  from S10 and the `RejectionReason`/`Verdict` design in m4-sweep §11.
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

**M4-specific (don't regress what S1–S10 established):**
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
- Cost scenarios (S10) scale by **exact integer** arithmetic only (u32/i64 fields; `scale_cost_model`).
  Cost **monotonicity is NOT a law** — higher costs can suppress a marginal trade, so doubled can be
  cheaper / higher-return and `return_drag_doubled` can go negative (see m4-sweep §9). Do **not**
  re-impose monotonicity as a runtime invariant; `fee_sensitivity` records the signed drag honestly.
  Baselines run through the shared `cell::eval_strategy` core (same pipeline as candidates — no drift).

---

## Seed prompt for the new chat — S11, the advancement/rejection report + new schema (paste this)

> You are the EXECUTOR continuing the **machina** (`solana-crypto-trader`) repo — a paper-first,
> Solana-focused crypto trading-**research** platform in Rust (no keys/signing/RPC/network anywhere).
> Read `plans/handoff.md`, then `plans/current-state.md`, **`plans/m4-sweep.md` — the active milestone
> plan, especially §11 "Rejection report" and §12 "Canonical export & schema impact" and §14 row S11**,
> then `plans/task-queue.md` and `docs/invariants.md`.
>
> **State:** M0–M2 complete, M3 scaffolds in place, and **M4 subtasks S1–S10 are DONE and green** —
> the deterministic sweep core (S1–S7, incl. the determinism gate), the walk-forward window model (S8),
> the **sealed holdout gate (S9)**, and the **cost/fee sensitivity ladder (S10)**. `cargo fmt --all
> --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace
> --all-features` = **153 passing**, `cargo run -p cli -- demo` byte-identical, no-execution-deps scan
> clean. The repo has **no commits** — **do not commit or push; the operator commits** (stage explicit
> paths only, never `.claude/`, never `git add -A`).
>
> **First** re-confirm the gates on a fresh checkout. **Then implement S11 — the ADVANCEMENT/REJECTION
> REPORT + the new `schemas/sweep-report.schema.json` — in small green diffs** (each keeps fmt + clippy
> `-D warnings` + test green), per `m4-sweep.md` §11/§12/§14. New files:
> `crates/sweep/src/advance.rs`, `crates/sweep/src/report.rs`, `schemas/sweep-report.schema.json`,
> `crates/sweep/tests/schema_validation.rs`. The design:
> - `RejectionReport { thresholds, trial_count, verdicts: Vec<CandidateVerdict> }`;
>   `CandidateVerdict { candidate_label, status: Verdict::{Advanceable, Rejected}, failed_criteria:
>   Vec<RejectionReason> }`. One `RejectionReason` variant per §11 criterion (fails-baseline,
>   edge-vanishes-under-doubled-costs, depends-on-one-period, parameter-fragile, drawdown-exceeds-budget,
>   turnover-implausible, insufficient-data), each carrying observed `Decimal` vs threshold as strings.
> - **All selection/threshold keys are `Decimal`** (reuse S10's `FeeSensitivity` / `best_baseline_return`
>   and the un-annualized `turnover` ratio); `f64` is display-only. `trial_count` = total cells evaluated
>   (for M5 multiple-testing accounting). Language is **robustness-only** — `Advanceable` means "passed
>   the battery, eligible for M5 review", never "this works" (invariant 11); include a JSON `note`.
> - `SweepReport::to_json()` canonical (money/turnover as exact decimal strings, f64 via
>   `results::stat_to_string` `{:.10}`, collections pre-sorted / `BTreeMap`). **New schema** is
>   Draft 2020-12, `additionalProperties:false`, `decimalString` for money — a **deliberate additive
>   contract act (D-0001)**. `run-result.schema.json` stays UNCHANGED.
>
> **S11 gate:** report validates against the new schema; each §11 criterion yields its matching reason;
> `trial_count` recorded; an **f64-comparison audit test** forbids `f64` in the selection/advancement
> module; two serializations are byte-identical. Depends on S7/S9/S10 (all done). The **holdout stays
> sealed** — S11 selects only over dev/val results; never call `evaluate_on_holdout`.
>
> Honor the operating contract in `plans/handoff.md`, incl. the **M4-specific rules**: parallelism is
> `std::thread::scope` (zero new deps; rayon rejected, D-0009); turnover comes from
> `RunOutput.traded_notional_quote`, never `round_trips`; all sort/threshold/selection keys are
> `Decimal` (`f64` display-only, non-finite→`None`); cost monotonicity is **not** a runtime invariant
> (S10 §9). Update
> `plans/current-state.md`, `plans/m4-sweep.md` (S11 status), `plans/task-queue.md`, and
> `plans/worklog.md` as part of "done", and consider a fresh-session/subagent adversarial review before
> declaring S11 done. **Do NOT start M8/M9** (signing/submit) — separate explicit human approval required.
