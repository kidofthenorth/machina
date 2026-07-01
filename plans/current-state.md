# Current state

Optimized for fast agent parsing. Source of truth for "where are we." Updated 2026-06-29.
**New chat? Start at [plans/handoff.md](handoff.md).**

## Milestone
- **DONE:** M0 (bootstrap), M1 (data foundation), M2 (deterministic spot simulator).
- **IN PLACE (scaffolds, gate not formally declared):** M3 (Strategy Lab MVP) — trait, 4 baselines,
  trend_alloc_v1, threshold_rebalance_v1, regime scaffold, CLI comparison.
- **DOING:** M4 (sweep / walk-forward). Plan = [m4-sweep.md](m4-sweep.md) (12 subtasks S1–S12).
  **S1–S10 DONE** — the *deterministic sweep core* (S1–S7), the **walk-forward window model (S8)**, the
  **holdout gate (S9)**, and the **cost/fee sensitivity ladder (S10)**: existing-crate prep (S1–S3) +
  `crates/sweep` skeleton (S4), param grid +
  strategy bridge (S5), single-cell runner + exact turnover (S6), the determinism gate (S7,
  `std::thread::scope`; parallel==sequential byte-identical across thread counts {1,2,3,7,8};
  repeated-run identical), walk-forward windows (S8: `Window` / `WalkForward` /
  `WindowKind::{Rolling (default), Anchored}`; exact embargo gap; ordered, non-overlapping test ranges;
  in-bounds; bad params rejected; pure + total), and the **holdout seal (S9)**: `sweep::{partition,
  config}` — `PartitionSpec::{ByIndex, ByDate}` (ByDate via the S2 date parser), `PartitionedBars::
  from_spec` (validates the series via canonical `market_data::validate_series` — sorted+unique — then
  **physically moves** the holdout into a separate `Vec<Bar>` via `split_off`; rejects overlap/empty/
  out-of-bounds), `seal_holdout(self) -> (DevValidation, Sealed)`, and the call-once
  `evaluate_on_holdout(sealed: Sealed, …)` (M5-only, never the M4 CLI). Gate: a `Cell<u32>`
  read-counter + digest stay untouched after the full sweep+selection pipeline (holds only
  `&DevValidation`); `compile_fail` doctests prove no selection→holdout path, no seal fabrication, and
  call-once consume-by-value; overlap rejected. Adversarially reviewed (6 lenses + per-finding
  skeptics): seal sound; 3 minor/nit hardening fixes applied (delegate to `validate_series` so
  duplicate timestamps are rejected; real post-split structural assert; within-process-only digest
  doc). And the **cost/fee sensitivity ladder (S10)**: `sweep::{sensitivity, baseline}` (+ `cell.rs`
  extracted a shared `eval_strategy` core) — `cost_scenarios` = {before_costs=`zero()`, base, doubled,
  doubled_slippage, doubled_priority} via exact integer `scale_cost_model` (u32 bps/i64 lamports, no
  f64); `ScenarioMetrics`/`FeeSensitivity` (Decimal `return_drag_*`, baseline-grounded
  `survives_doubled` — not a bare `>0`); the 4 required baselines re-run cost-matched through the same
  `eval_strategy` (label==strategy `name()`; `best_baseline_return` floor). Adversarially reviewed
  (3 lenses + skeptics): no code defects — surfaced that **cost monotonicity is conditional** (higher
  costs can suppress a marginal trade → fewer fees/higher return/negative drag), now documented + a
  regression test pins it; 2 doc nits fixed. **Next diff: S11** — advancement/rejection report + new
  `schemas/sweep-report.schema.json`. Remaining: S11 report+schema, S12 CLI.
- **NEXT after M4:** M5 research-decision gate. M6 needs plan §22 source re-check; M8/M9
  (signing/submit) need **separate explicit human approval**.

## Completed artifacts
- Workspace: 7 crates — research-core, market-data, portfolio, metrics, strategies, results, cli.
- Money: `rust_decimal::Decimal` everywhere; f64 only in metrics stats (display-only).
- Determinism: next-bar execution; ordered collections; no RNG/clock in canonical runs.
- M0: README, DECISIONS, AGENTS, CLAUDE, master-plan, 4 config templates, 4 JSON schemas,
  rust-toolchain, rustfmt, CI (fmt+clippy+test + no-execution-deps scan).
- M1: bar/token/time types; allowlist loader; series validation (empty/OHLC/dup/unsorted/**gap**);
  token-decimal + allowlist checks; tiny checked-in fixtures (incl. bars_gap).
- M2: PortfolioState, deterministic CostModel fill model, simulator (equity curve, round trips,
  cost aggregates), apply_buy/apply_sell with oversell/unaffordable/gas guards.
- M3: Strategy trait (intent only); baselines hold_usdc / buy_and_hold_sol / static_50_50 / dca_sol;
  regime::classify scaffold; trend_alloc_v1 (SMA bucket); threshold_rebalance_v1 (drift band).
- results: RunResult/RunConfig DTOs (money as decimal strings) validating run-result.schema.json.
- cli: `machina demo` — deterministic, schema-valid RunResult; no network, no keys.

## Gates run (2026-06-29, all from this tree)
- `cargo fmt --all --check` → clean (exit 0).
- `cargo clippy --all-targets --all-features -- -D warnings` → clean (exit 0).
- `cargo test --workspace --all-features` → **153 passed, 0 failed** (85 baseline + 68 from M4 S1–S10).
- `cargo run -p cli -- demo` → byte-identical across runs (determinism verified).
- no-execution-deps grep gate → pass (comments ignored). Dep tree = rust_decimal/serde/serde_json/
  toml (+ dev jsonschema); **no Solana/Jupiter/HTTP/wallet/signing crate anywhere**.
- Adversarial verification workflow (6 lenses) → **PASS, no blockers**; 14 findings, all resolved or
  accepted (see plans/review-packet.md and worklog).
- S9 holdout-seal adversarial review (6 lenses + per-finding skeptics, 12 agents) → **seal sound, no
  blockers**; 3 confirmed minor/nit findings, all fixed (duplicate-ts rejection via `validate_series`;
  real post-split assert; within-process-only digest doc). 3 findings refuted.
- S10 cost-sensitivity adversarial review (3 lenses + skeptics, 9 agents) → **no code defects**;
  key finding: cost monotonicity is conditional (trade suppression can invert it), now documented +
  pinned by a regression test; 2 doc nits fixed. 2 findings refuted (overflow/negative-lamport both
  unreachable — CostModel has no Deserialize; all constructions are hardcoded positive literals).

## Safety posture (all upheld)
- No keys / signing / submit / RPC / network in the tree (no such crate is even depended upon).
- No secrets; `*.example.toml` templates only; RPC config holds env-var NAMES, never values.
- Allowlist-gated; next-bar execution (final-bar signal cannot open a position); strategies = intent.

## Staged paths
- Nothing committed (fresh repo, no commits — BASE=EMPTY). Operator commits only.
- Explicit paths to stage (NOT `.claude/`): see plans/review-packet.md "Staged paths".

## Known blockers / open items
- **0 blocking.** Operator questions (all non-blocking, safe defaults applied): plans/questions.md.
- Accepted nit (no action): CostModelDto lamports are `i64` vs schema `minimum:0` — latent only, no
  negative-producing path (audit JS-1).
- Deferred to later milestones: real OHLCV ingestion (needs operator data-source choice, Q3); M4
  sweep crate; M6 route/shadow crates; M7 wallet-state; M8/M9 gated execution.

## Next recommended command
- Continue M4 per [m4-sweep.md](m4-sweep.md): S1–S10 are DONE (deterministic sweep core + walk-forward
  windows + sealed holdout + cost/fee sensitivity). Next is **S11** — the advancement/rejection report
  + **new `schemas/sweep-report.schema.json`** (`sweep/src/advance.rs` + `report.rs`; Decimal-only
  selection keys + an f64-comparison audit test; `trial_count` recorded; robustness language only,
  invariant 11; adding the schema is a deliberate contract act, D-0001) — then S12 (CLI
  `sweep`/`sweep-verify` + plans/docs/DECISIONS refresh). Each subtask is a small diff that keeps the
  workspace green. Do NOT start execution (M8/M9) without separate human approval.
