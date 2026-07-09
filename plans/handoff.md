# Handoff — for a new chat continuing machina

You are picking up a build where **M4 is DONE and gate-declared**; M5 requires an explicit operator
decision first. This is the single entry point. Read the five pointer files below, confirm the gates
are green, then follow the STOP block in [current-state.md](current-state.md) — do not proceed into
M5 without it. **Do not commit or push — the operator commits.**

---

## 30-second orientation

**machina** (codename `solana-crypto-trader`) is a paper-first, Solana-focused crypto **trading
research platform** in Rust, built toward one goal: **genuine autonomous passive income** —
low-touch, hands-off on-chain spot trading for a solo operator, once a strategy has earned that
trust. Getting there means first proving a strategy has a robust edge after costs, latency,
slippage, and operational failure; no strategy is guaranteed to clear that bar, and **live trading
starts only once its own milestone gate is explicitly approved**.

**The one rule that cannot be broken:** money-moving capability is gated by milestone **and** by
explicit human approval. No key loading, signing, or transaction submission exists — or may be added
— before the research phase (M0–M5) is complete and separately approved. Building the software never
authorizes trading. Full invariants: [docs/invariants.md](../docs/invariants.md).

## Read these first (source of truth)

1. [plans/current-state.md](current-state.md) — live per-milestone status, gates, blockers, next command.
2. **[plans/task-queue.md](task-queue.md) — the M4 task queue, CLOSED**: cards M4-C1…M4-C10 are all
   DONE. Nothing in it is active; do not resume it. Design rationale lives in
   [plans/m4-sweep.md](m4-sweep.md) (S1–S11 DONE).
3. [plans/master-plan.md](master-plan.md) — authoritative M0–M11 roadmap & architecture.
4. [plans/m4-sweep.md](m4-sweep.md) — the M4 design record (S1–S11 rationale the cards build on).
5. [docs/architecture-index.md](../docs/architecture-index.md) + [docs/invariants.md](../docs/invariants.md) — module map + hard invariants.

Audit + staged-diff record: [plans/review-packet.md](review-packet.md). Chronological log:
[plans/worklog.md](worklog.md). Open operator questions: [plans/questions.md](questions.md).

## What's accomplished (M0–M2 complete; M3 scaffolds; **M4 in progress — S1–S11 done**)

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
- **M4 — sweep core + walk-forward + holdout seal + cost sensitivity + advancement report (S1–S11 of 12 done).** Designed via a 3-architecture + adversarial
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
    pipeline; 3 `compile_fail` doctests (no selection→holdout path, no seal fabrication, call-once) —
    2 of 3 carry `no_run` canaries; the `Sealed` one is queued as card M4-C1 (2026-07-06 audit);
    overlap rejected. **Zero new deps.** Adversarially reviewed (6-lens +
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
  - **S11 — ADVANCEMENT/REJECTION REPORT + new schema (D-0001).** `sweep::{advance, report}` +
    `schemas/sweep-report.schema.json`. `advance.rs`: Decimal-only `evaluate_candidate` — 7
    `RejectionKind` criteria, InsufficientData hard-stop, fixed canonical order, `RejectionReason`
    records observed/threshold as exact decimal strings. `report.rs`: `SweepReport` with a
    robustness-only `note` (invariant 11), `trial_count`, and a **total-order** verdict sort
    (byte-identical even with duplicate labels). New Draft-2020-12 schema (additive contract);
    **run-result schema UNCHANGED**; `tests/schema_validation.rs` mirrors `results` (real report
    validates; rejects numeric budget / unknown enums / extra props; source-level f64-audit).
    **Zero new deps.** Adversarially reviewed (3-lens + skeptics, 7 agents): **schema fidelity clean**;
    3 nit fixes (single-source edge-vanishes; total-order sort; coupling-code-enforced note); 1 refuted.
- **Quality.** A 6-lens adversarial verification workflow ran on M0–M3 → **PASS, no blockers**; a
  5-lens + adjudicator workflow verified S8 → impl correct, found & fixed two test gaps; a 6-lens +
  per-finding-skeptic workflow verified the S9 seal → **sound**, 3 minor/nit hardening fixes applied; a
  3-lens + skeptic workflow verified S10 → **no code defects**, surfaced conditional cost monotonicity
  (documented + regression-tested), 2 doc nits fixed; a 3-lens + skeptic workflow verified S11 → **schema
  fidelity clean**, 3 nit fixes applied.

## Verified state (re-confirm on a fresh checkout)

```bash
cargo fmt --all --check                                  # clean
cargo clippy --all-targets --all-features -- -D warnings # clean
cargo test --workspace --all-features                    # 301 passed, 0 failed (2026-07-09, card M4-C9 gate declaration)
cargo run -p cli -- demo                                 # deterministic, schema-valid RunResult (shasum ae064f79242f823ffd8f55bf9104e3e1b45d425a, identical on repeat)
cargo run -p cli -- sweep                                # deterministic sweep report (shasum 7ad3df7de2e2c1139be427e9c953b57d4e289cb3, identical ×2 and at --threads 8)
cargo run -p cli -- sweep-verify                         # sweep-verify: OK — byte-identical across sequential, 2 and 8 threads, and repeat (18990 bytes); exit 0
# no-execution-deps scan (CI parity) — must print OK:
pattern='solana-sdk|solana-client|solana-program|solana-rpc|jupiter|jito|ed25519-dalek|keypair|bip39|tiny-bip39|secp256k1'
grep -REn --include='Cargo.toml' "$pattern" . | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#' || echo OK
```

Repo state (2026-07-09): **M4 gate declared (card M4-C9).** The operator has committed through card
M4-C9 (HEAD = `98724e2`, "GATE DECLARED 2026-07-09 — M4 sweep/walk-forward complete"); card M4-C10
(this handoff pointer) is this session's plan-only edit, pending operator commit. **Never** `git add
-A`; **never** stage `.claude/`. Stage explicit paths only. (Run `git status` to confirm current HEAD
and what's staged.)

## What's next — STOP: M5 is an operator decision

**M4 is DONE — gate declared 2026-07-09 (card M4-C9); all cards M4-C1…M4-C10 are complete.** The
task queue's M4 section is closed. Do not resume it and do not queue M5 implementation.

**STOP — operator decisions required before M5 (the research decision):**
1. Freeze the walk-forward sizing and rejection thresholds (questions.md Q5) — they are illustrative
   in `strategy-lab.example.toml` and must be frozen BEFORE the decisive sweep (post-hoc choice =
   overfitting).
2. Choose the real OHLCV data source for SOL/USDC (questions.md Q3) — all M4 runs used synthetic
   series; no statistical claim is valid until real multi-year data is ingested and validated.
3. Give an explicit M5 go decision. M5's gate (master-plan.md:903-907): select ONE candidate for
   mainnet shadow because it satisfies predefined criteria, or reject all and return to research.

Do NOT start M6+ (network), and never M8/M9 (signing/submission — separate explicit human approval).
Agents picking up this repo may do **maintenance only** (fix a failing gate, refresh docs) until the
operator records a written go in `plans/questions.md`.

Separately, questions.md Q7 records the operator's high-frequency direction decision (2026-07-07): the
plan amendment adding an HF research track is a **pending operator-level planning task**, not an
executor card — it waits on this same M5 gate and is out of scope for any agent session until the
operator makes that edit.

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

**M4-specific (don't regress what S1–S11 established):**
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
- The advancement report (S11) is **robustness-only** (invariant 11): `Advanceable` = "eligible for M5
  review", never "profitable". All threshold/selection keys are `Decimal` (`advance.rs`/`report.rs` are
  f64-free, source-audited). `SweepReport` verdicts sort by a **total** order so output is byte-identical;
  `schemas/sweep-report.schema.json` is an additive contract (D-0001) — **`run-result.schema.json` stays
  UNCHANGED**. Schema checks shape; code enforces semantics (status⇔`failed_criteria`).

---

## Seed prompt for the new chat — maintenance only (M4 is done; M5 needs the operator)

> You are picking up the **machina** (`solana-crypto-trader`) repo — a paper-first, Solana-focused
> crypto trading-**research** platform in Rust built toward **genuine autonomous passive income**
> (truly autonomous, so truly passive), earned through milestone gates. The current phase is
> research: no keys/signing/RPC/network anywhere.
>
> **M4 is DONE — gate declared 2026-07-09 (card M4-C9); M4-C1…M4-C10 are all complete.** Read
> `plans/handoff.md` → `plans/current-state.md` → the STOP block there before doing anything else.
> There is no active M4 task queue left to execute, and M5 cannot start without the operator's
> written go.
>
> **Your task this session is maintenance only**: fix a failing gate (a fmt/clippy/test regression)
> or refresh stale docs. You may **not** start M5 implementation, real-data ingestion, or any M6+
> scaffolding — those require the operator to (1) freeze questions.md Q5's walk-forward sizing and
> rejection thresholds, (2) choose the Q3 real-data source, and (3) record an explicit M5 go in
> `plans/questions.md`. If none of that is recorded yet, do not queue or sketch M5 work — say so and
> stop.
>
> Non-negotiables (unchanged): `Decimal`/integer only for money — never f64; the holdout stays sealed
> — **never call `evaluate_on_holdout`**; never edit `schemas/*.json` or `plans/master-plan.md`;
> never `git commit`/`git push` (stage explicit paths only, never `.claude/`, never `git add -A`);
> never start M8/M9 (signing/submit) — separate explicit human approval required.
