# Current state

Optimized for fast agent parsing. Source of truth for "where are we." Updated 2026-07-06.
**New chat? Start at [plans/handoff.md](handoff.md).**
Goal: **genuine autonomous passive income** — truly autonomous, so truly passive — earned strictly
through the milestone gates (money-moving capability stays gated by explicit human approval).

## Milestone
- **DONE:** M0 (bootstrap), M1 (data foundation), M2 (deterministic spot simulator).
- **IN PLACE (scaffolds, gate not formally declared):** M3 (Strategy Lab MVP) — trait, 4 baselines,
  trend_alloc_v1, threshold_rebalance_v1, regime scaffold, CLI comparison.
- **DOING:** M4 (sweep / walk-forward). Plan = [m4-sweep.md](m4-sweep.md) (12 subtasks S1–S12).
  **S1–S11 DONE** — the *deterministic sweep core* (S1–S7), the **walk-forward window model (S8)**, the
  **holdout gate (S9)**, the **cost/fee sensitivity ladder (S10)**, and the **advancement/rejection
  report + new schema (S11)**: existing-crate prep (S1–S3) + `crates/sweep` skeleton (S4), param grid +
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
  regression test pins it; 2 doc nits fixed. And the **advancement/rejection report + new schema
  (S11)**: `sweep::{advance, report}` + `schemas/sweep-report.schema.json` — Decimal-only
  `evaluate_candidate` (7 `RejectionReason` criteria, InsufficientData hard-stop, canonical order),
  `SweepReport` (robustness-only `note`, `trial_count`, total-order sort) validating against the new
  Draft-2020-12 schema (additive contract, D-0001). Gate: real report validates; each criterion→reason;
  f64-audit; two serializations byte-identical; run-result schema UNCHANGED. Adversarially reviewed
  (3 lenses + skeptics): schema fidelity clean; 3 nit fixes (single-source edge-vanishes, total-order
  sort, coupling-code-enforced note). **Remaining: S12, expanded into task cards M4-C1…M4-C10 in
  [task-queue.md](task-queue.md)** (a 2026-07-06 audit found S12 underspecified: no `SweepSpec`/`run_sweep`
  orchestrator or TOML `Deserialize` exists in `crates/sweep` yet, and `strategy-lab.example.toml` has no
  `[walk_forward]`/`[advancement]`/grid blocks — the cards add spec parsing, the runner, the two CLI
  subcommands, D-0009 + docs, then the M4 gate declaration). One S9 nuance corrected: 3 `compile_fail`
  doctests exist but only 2 have `no_run` canaries (the `Sealed` no-forgery one lacks it — queued as M4-C1).
- **NEXT after M4:** M5 research-decision gate. M6 needs plan §22 source re-check; M8/M9
  (signing/submit) need **separate explicit human approval**.

## Completed artifacts
- Workspace: 8 crates — research-core, market-data, portfolio, metrics, strategies, results, sweep, cli.
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

## Gates run (2026-07-06, all from this tree at `df18267`)
- `cargo fmt --all --check` → clean (exit 0).
- `cargo clippy --all-targets --all-features -- -D warnings` → clean (exit 0).
- `cargo test --workspace --all-features` → **272 passed, 0 failed** (169 at S11 + 103 from the
  2026-07-01 test-hardening commits `375288b..df18267`; per-target counts in worklog).
- `cargo run -p cli -- demo` → byte-identical across runs (determinism verified, shasum `ae064f79…`).
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
- S11 report+schema adversarial review (3 lenses + skeptics, 7 agents) → **schema fidelity clean, no
  blockers**; 3 nit fixes applied (edge-vanishes derived from a single source; total-order verdict
  sort for byte-identical output with duplicate labels; note that status⇔criteria coupling is
  code-enforced). 1 refuted (f64-audit's two-file scope is intentional — siblings use f64 only for
  display-only stats, already normalized to None).

## Safety posture (all upheld)
- No keys / signing / submit / RPC / network in the tree (no such crate is even depended upon).
- No secrets; `*.example.toml` templates only; RPC config holds env-var NAMES, never values.
- Allowlist-gated; next-bar execution (final-bar signal cannot open a position); strategies = intent.

## Commit state
- Operator committed through S11 (`5e5b0ba`) plus 9 test-hardening commits; **HEAD = `df18267`
  (2026-07-01), working tree clean**. Operator commits only — executors never commit/push; stage
  explicit paths only, never `.claude/`, never `git add -A`.

## Known blockers / open items
- **0 blocking.** Operator questions (all non-blocking, safe defaults applied): plans/questions.md.
- Accepted nit (no action): CostModelDto lamports are `i64` vs schema `minimum:0` — latent only, no
  negative-producing path (audit JS-1).
- Deferred to later milestones: real OHLCV ingestion (needs operator data-source choice, Q3); M6
  route/shadow crates; M7 wallet-state; M8/M9 gated execution. (The M4 sweep crate exists; only its
  CLI wiring + gate declaration remain — cards M4-C1…C10.)

## Next recommended command
- Execute the **task cards M4-C1…M4-C10 in [task-queue.md](task-queue.md), in order, one card per fresh
  session**. Each card is self-contained (goal, exact files + signatures, numbered steps, runnable gate,
  guardrails, escalate-ifs) and sized for a small model with zero exploration. The queue covers: the S9
  `no_run` canary (C1), `SweepSpec` TOML parsing + config-template blocks (C2), the sweep runner
  (C3–C5), CLI `machina sweep` / `machina sweep-verify` (C6–C7), DECISIONS D-0009 + docs (C8), the
  **M4 gate declaration** (C9), and the M5 handoff pointer (C10). **The holdout stays M5-only — never
  call or wire `evaluate_on_holdout`.** Do NOT start M5 implementation (needs Q3 data + explicit go) or
  execution work (M8/M9 — separate human approval).
