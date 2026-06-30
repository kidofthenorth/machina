# Current state

Optimized for fast agent parsing. Source of truth for "where are we." Updated 2026-06-29.
**New chat? Start at [plans/handoff.md](handoff.md).**

## Milestone
- **DONE:** M0 (bootstrap), M1 (data foundation), M2 (deterministic spot simulator).
- **IN PLACE (scaffolds, gate not formally declared):** M3 (Strategy Lab MVP) — trait, 4 baselines,
  trend_alloc_v1, threshold_rebalance_v1, regime scaffold, CLI comparison.
- **NEXT:** M4 (sweep / walk-forward) — not started. M5+ gated; M6 needs plan §22 source re-check;
  M8/M9 (signing/submit) need **separate explicit human approval**.

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
- `cargo test --workspace --all-features` → **85 passed, 0 failed**.
- `cargo run -p cli -- demo` → byte-identical across runs (determinism verified).
- no-execution-deps grep gate → pass (comments ignored). Dep tree = rust_decimal/serde/serde_json/
  toml (+ dev jsonschema); **no Solana/Jupiter/HTTP/wallet/signing crate anywhere**.
- Adversarial verification workflow (6 lenses) → **PASS, no blockers**; 14 findings, all resolved or
  accepted (see plans/review-packet.md and worklog).

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
- Begin M4: `/plan` a `sweep` crate (parameter sweeps + parallel canonical export + walk-forward),
  OR re-verify plan §22 sources before any M6 shadow work. Do NOT start execution (M8/M9) without
  separate human approval.
