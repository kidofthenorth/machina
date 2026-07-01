# Task queue

Compact, actionable. Status: TODO / DOING / DONE / BLOCKED / DEFERRED.
Gate = the check that proves the task is complete.

| ID | Status | Milestone | File scope | Gate | Notes |
|----|--------|-----------|------------|------|-------|
| T01 | DONE | M0 | `.gitignore`, `README.md`, `DECISIONS.md`, `docs/*`, `plans/*` | files exist, links resolve | Secret-safe gitignore replaces cadence-payload one (D-0008). |
| T02 | DONE | M0 | `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml` | `cargo metadata` ok | Workspace; deps are pure-math+serde only (D-0002). |
| T03 | DONE | M0 | `.github/workflows/ci.yml` | yaml valid; jobs defined | fmt+clippy+test + `no-execution-deps` scan. |
| T04 | DONE | M0 | `config/**/*.example.toml` | no secrets; parse in T08 | allowlist/rpc/costs/strategy templates. |
| T05 | DONE | M0 | `schemas/*.schema.json` | valid Draft 2020-12 | run-result/route-quote/execution-event/wallet-snapshot. |
| T06 | DONE | M0 | `crates/results` | schema-validation tests pass | RunResult validates against run-result schema (8 tests). |
| T07 | DONE | M1 | `crates/research-core` | unit tests pass | money/token/bar/time; fixed-point money (14 tests). |
| T08 | DONE | M1 | `crates/market-data` (+ `fixtures/`) | corrupt/dup/unsorted/missing/bad-decimal/disallowed-token tests pass | bar validation (incl. gap) + allowlist loader. |
| T09 | DONE | M2 | `crates/portfolio` | accounting + determinism tests pass | state, fills, simulator, equity, round trips (14 tests). |
| T10 | DONE | M2/M3 | `crates/metrics` | metrics tests pass; drawdown ∈ [0,1] | return/dd in Decimal; Sharpe/Sortino/vol f64 (7 tests). |
| T11 | DONE | M3 | `crates/strategies` | deterministic strategy tests pass | trait + 4 baselines (incl. dca_sol) + regime scaffold + trend_alloc_v1 + threshold_rebalance_v1. |
| T12 | DONE | M2/M3 | `crates/cli` | `cargo run -p cli -- demo` runs | deterministic demo wiring; emits validated RunResult. |
| T13 | DONE | M0–M3 | (gates) | fmt+clippy+test green | fmt/clippy clean; 85 tests pass; demo byte-identical. |
| T14 | DONE | M0–M3 | (review) | verification fan-out recorded | 6-lens adversarial workflow → PASS; 14 findings fixed/accepted (review-packet). |
| M4 | DOING | M4 | `crates/sweep` (+ portfolio/results/research-core additive) | parallel==sequential; repeated identical; holdout untouched | Plan: [m4-sweep.md](m4-sweep.md), 12 subtasks S1–S12. **S1–S11 DONE** (deterministic sweep core; S7 determinism gate; S8 walk-forward windows; S9 holdout gate; S10 cost/fee sensitivity; S11 advancement/rejection report + new sweep-report schema, D-0001 — all adversarially reviewed). Next & last: S12 (CLI `sweep`/`sweep-verify` + DECISIONS D-0009 + docs). 169 tests. |
| — | DEFERRED | M5 | research decision | — | trial count, stability, advance/reject gate. Needs real data (Q3). |
| — | DEFERRED | M6 | `crates/route-model`, `crates/risk`, `crates/solana-execution` (no-sign) | — | Jupiter shadow quote collector. Re-verify plan §22 sources first. |
| — | DEFERRED | M7 | `crates/wallet-state` | — | read-only mainnet shadow. No signing key. |
| — | BLOCKED | M8/M9 | devnet/canary signing+submit | — | **Requires separate explicit human approval.** Do not start. |
