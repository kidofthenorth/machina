# Current state

Optimized for fast agent parsing. Source of truth for "where are we." Updated 2026-07-09 (M5 GO).
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
- **DOING:** M5 (research decision) — **operator GO recorded 2026-07-09**. Cards **M5-C1…C6** in
  [task-queue.md](task-queue.md) §M5: **C1 (ingestion script), C2 (`data-validate`), C3 (Q5 freeze
  sitting) DONE**. Real data confirmed 2026-07-11: 916 bars 2023-12-28..2026-06-30, fnv1a64
  `0x65a1e18b7554a1c5` ([m5-data-validation.md](m5-data-validation.md)); policy **frozen** in
  `config/strategies/m5-frozen.toml` (rolling 365/60/60/5, min_windows 6, holdout 2026-01-01
  onward — one-way ratchet closed, immutable this cycle).
- **NEXT:** execute M5-C4 (`--config`/`--data` sweep wiring) → C5 (decisive sweep) → M5-C6
  (decision card: the single call-once
  `evaluate_on_holdout`, advance-or-reject-all vs master-plan.md:903-909; reject-all routes to the
  HF track). M6 needs plan §22 source re-check; M8/M9 (signing/submit) need **separate explicit
  human approval**.

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
- Operator committed through M5-C2; **HEAD = `0d8efaa` (2026-07-10, "M5-C2 done")**; the M5-C3
  freeze-sitting diff is staged pending operator commit. Operator commits only — executors never
  commit/push; stage explicit paths only, never `.claude/`, never `git add -A`.

## Known blockers / open items
- **0 blocking.** 2026-07-09: Q3 + Q5 RESOLVED and M5 GO recorded (plans/questions.md). Remaining
  open questions (Q1, Q2, Q4) are cosmetic/non-blocking.
- **HF research track (Q7): amendment MADE 2026-07-09 (D-0012).** Master-plan pair amended in
  lockstep (cmp-verified); formal milestone plan authored at [m-hf-track.md](m-hf-track.md) via a
  3-design + adversarial-review + adjudication workflow (7 Sonnet agents; reuse-first won).
  Wave-1 cards M-HF-C1/C2 are the **pre-adjudication draft, unmodified** — confirmed 2026-07-09
  they lack the typed `Provenance` enum and `IntraBar` alias `m-hf-track.md` §5 requires; a planner
  session must patch C1's card text before it executes (task-queue.md's reconciliation note). Entry
  otherwise waits only on **HF-Q1/HF-Q2** (questions.md) — C1–C2.6 are synthetic-only and the
  operator may unblock them ahead of HF-Q1 by written note. Wave-2+ card drafting still requires
  the sweep/strategies/invariants audit re-run flagged 2026-07-07, and must be drafted against
  `m-hf-track.md` §5, not the superseded sketch in `highfrequency-algo-plan.md` §3.
- Accepted nit (no action): CostModelDto lamports are `i64` vs schema `minimum:0` — latent only, no
  negative-producing path (audit JS-1).
- Deferred to later milestones: M6
  route/shadow crates; M7 wallet-state; M8/M9 gated execution. (M4 is DONE — sweep crate, CLI wiring,
  and gate declaration all complete, cards M4-C1…C10.)

## Next recommended command
Execute **M5-C4** (task-queue.md §M5) in a fresh executor session — wire `--config`/`--data` into
`machina sweep`. The Q5 freeze (M5-C3) is DONE (2026-07-11): frozen numbers are immutable this
cycle; the M4 no-flag sweep path must stay byte-identical (`7ad3df7d…`).

Do NOT start M6+ (network), and never M8/M9 (signing/submission — separate explicit human approval).
