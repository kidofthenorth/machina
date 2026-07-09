# Current state

Optimized for fast agent parsing. Source of truth for "where are we." Updated 2026-07-06.
**New chat? Start at [plans/handoff.md](handoff.md).**
Goal: **genuine autonomous passive income** — truly autonomous, so truly passive — earned strictly
through the milestone gates (money-moving capability stays gated by explicit human approval).

## Milestone
- **DONE:** M0 (bootstrap), M1 (data foundation), M2 (deterministic spot simulator).
- **IN PLACE (scaffolds, gate not formally declared):** M3 (Strategy Lab MVP) — trait, 4 baselines,
  trend_alloc_v1, threshold_rebalance_v1, regime scaffold, CLI comparison.
- **DONE:** M4 (sweep / walk-forward) — engine S1–S11 + task cards M4-C1…C8d built (deterministic
  sweep core, walk-forward windows, holdout seal, cost/fee-sensitivity ladder, advancement/rejection
  report + schema, CLI `sweep`/`sweep-verify`, D-0009/D-0010/D-0011 recorded); **gate declared
  2026-07-09 (card M4-C9)**; 301 tests green. Plan: [m4-sweep.md](m4-sweep.md).
- **DOING:** — (between milestones; M5 requires operator decisions — see handoff).
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

## Gates run (2026-07-09, M4 gate declared — card M4-C9; prior batch 2026-07-06 at `df18267`)
- `cargo fmt --all --check` → clean (exit 0).
- `cargo clippy --all-targets --all-features -- -D warnings` → clean (exit 0).
- `cargo test --workspace --all-features` → **301 passed, 0 failed** (272 at 2026-07-06 `df18267` +
  cards M4-C8b…C8e; per-target counts in worklog).
- `cargo run -p cli -- demo` → byte-identical across runs (determinism verified, shasum `ae064f79…`).
- `cargo run -p cli -- sweep` → byte-identical across 2 runs and `--threads 8` (shasum `7ad3df7d…`);
  `cargo run -p cli -- sweep-verify` → `OK`, exit 0 (byte-identical sequential/2/8 threads + repeat).
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
- M4 gate declaration (card M4-C9, 2026-07-09) → full step-1 battery re-run fresh, all pass; two prior
  C9 pre-declaration reviews' findings resolved and reverified before declaring (D-0010 fee-sensitivity
  fix; D-0011 family-comparison descope, m4-sweep §10 amended).

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
- **HF research track (Q7 follow-on): wave-1 cards drafted, BLOCKED.** 2026-07-07: the entry-condition
  check found none of the three conditions met (M4 gate undeclared; HF doc unapproved + no amendment;
  HF-Q1/Q2/Q3 unrecorded), so on operator instruction M-HF-C1/C2 were drafted **ahead of the gate**
  into [task-queue.md](task-queue.md) §"M-HF wave 1" with the entry conditions restated as a hard
  precondition. Audit notes appended to [highfrequency-algo-plan.md](highfrequency-algo-plan.md)
  (read before approving). Waves 2+ (C3–C10) are deliberately not drafted; the sweep/strategies/
  invariants audit areas must be re-run before wave-2 expansion (see worklog 2026-07-07).
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
