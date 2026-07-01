# Worklog

Chronological, terse. One line per meaningful event. Newest at the bottom of each day. Gate results
recorded inline (not full logs).

## 2026-06-29 — M0 bootstrap → M2/M3
- Recon: fresh repo, **no commits** (BASE=EMPTY). Toolchain rustc/cargo 1.95.0 + clippy + rustfmt present.
- Verified crates.io reachable; dep set resolves+builds: rust_decimal 1.42, serde 1.0, serde_json 1.0, toml 1.1, jsonschema 0.46.
- M0: wrote secret-safe `.gitignore` (replaces cadence-payload gitignore; D-0008), `README.md`, `DECISIONS.md`.
- M0: `Cargo.toml` workspace (7 research crates), `rust-toolchain.toml`, `rustfmt.toml`; deps pure-math+serde only (D-0002).
- M0: `.github/workflows/ci.yml` — fmt+clippy(-D warnings)+test, plus `no-execution-deps` grep gate.
- M0: 4 config templates (allowlist/rpc/costs/strategy) — no secrets; RPC keys via env-var NAME only.
- M0: 4 JSON schemas (run-result/route-quote/execution-event/wallet-snapshot), Draft 2020-12.
- M0: `docs/invariants.md`, `docs/architecture-index.md`; copied plan → `plans/master-plan.md` (1826 lines).
- Verified rust_decimal/jsonschema APIs in scratchpad; dropped `maths` feature; pinned rust_decimal_macros=1.
- M1: `research-core` (money/token/bar/time) — 14 tests green. `market-data` (allowlist + validation + fixtures) — green.
- M2: `portfolio` (cost/state/equity/simulator) — 14 tests; buy/sell/oversell/unaffordable/fees/final-bar/determinism.
- M2/M3: `metrics` (return/dd in Decimal; Sharpe/Sortino f64) — 7 tests; drawdown ∈ [0,1] asserted.
- M3: `strategies` (trait + baselines + trend_alloc_v1 + threshold_rebalance_v1) — green; sim integration tests.
- M0: `results` DTOs + 8 schema-validation tests — built RunResult validates against run-result.schema.json.
- M2/M3: `cli` demo wires it all; `cargo run -p cli -- demo` deterministic (byte-identical) + schema-valid.
- Gate: fmt clean; clippy -D warnings clean; **76 tests pass**. Fixed no-execution-deps CI grep to ignore TOML comments.
- Audit: ran 6-lens adversarial verification workflow (22 agents) → **PASS, no blockers**; 14 findings (4 major/7 minor/3 nit).
- Fix (M1): added `validate_series_spacing` + `DataError::Gap` + `bars_gap` fixture + missing/disallowed-token tests.
- Fix (M3): added `dca_sol` baseline (wired into CLI) + `regime::classify` scaffold, each with tests; updated DECISIONS D-0007.
- Fix (docs): corrected architecture-index deps; refreshed AGENTS/CLAUDE status+pointers; canonical roadmap = plans/master-plan.md.
- Created `plans/current-state.md` + `plans/review-packet.md` (resolves broken links). Accepted JS-1 nit (CostModelDto i64) — no action.
- Re-gate after fixes: fmt clean; clippy -D warnings clean; **85 tests pass**; demo still deterministic. Ready for operator review.

## 2026-06-29 — M4 planning (sweep / walk-forward)
- Re-confirmed gates on fresh checkout: `cargo build`/`test`/`clippy -D warnings`/`fmt --check` green; **85 tests pass**.
- Ran a 3-design + adversarial-review design workflow (5 agents) for M4. Adversary caught a real trap:
  turnover from `round_trips` reports ~0 for zero-round-trip rebalancers (Static5050 / threshold_rebalance) →
  fix is an additive `RunOutput.traded_notional_quote` (Decimal), turnover = traded_notional / mean_equity.
- Decisions: parallelism = `std::thread::scope` (ZERO new deps; rayon rejected — record D-0009); holdout sealed by
  **physical partition** (separate `Vec<Bar>` allocation) + consume-by-value `evaluate_on_holdout` (M5-only).
- Wrote `plans/m4-sweep.md` (active milestone plan): 12 small-diff subtasks S1–S12; S7 = determinism gate, S9 = holdout gate.
- Logged operator policy decisions (walk-forward sizing, rejection thresholds) as non-blocking with illustrative
  defaults — to be FROZEN before M5 (questions.md Q4 extended + Q5). No code changed yet; next diff = S1.
- M4 S1: added `RunOutput.traded_notional_quote` (Decimal) + `traded_notional()` helper in `portfolio::simulator`;
  buy = USDC spent, sell = mid value of SOL sold (gas excluded). Tests: hand-computed 2000 (buy+sell), 937.5
  (zero-round-trip rebalancer — the case round-trip turnover misses), 0 (no-trade). +3 tests.
- M4 S2: added `civil_date_to_unix` + `parse_ymd` (+ `DateParseError`) to `research-core::time` — Hinnant
  `days_from_civil` inverse, pure integer, zero deps; calendar-validity via round-trip (rejects 2025-02-30 etc.).
  Pinned 2021-01-01 / leap 2024-02-29 / 2025-12-31 to exact secs. +3 tests.
- M4 S3: added `RunInputs.turnover: Option<Decimal>`; `RunResult::build` fills `MetricsReport.turnover` (was
  hard-None). cli demo passes `None`. New test: populated turnover serializes as decimal string + validates
  against the UNCHANGED run-result schema. +1 test.
- Re-gate after S1–S3: fmt clean; clippy -D warnings clean; **92 tests pass**; demo byte-identical. Next: S4.
- M4 S4: created `crates/sweep` (skeleton lib + manifest); wired into workspace members + deps. Deps pure-math/serde/toml
  only; no-execution-deps scan passes; sweep dep tree has no solana/jupiter/http/async/signing crate.
- M4 S5: `sweep::param` — `ParamPoint`/`ParamGrid` (fixed-order Cartesian product, both families), `build_strategy`
  (intent-only bridge), scale-canonical `param_id` (0.50→0.5 via `.normalize()`). +4 tests.
- M4 S6: `sweep::turnover` (`turnover_ratio` = traded_notional/mean_equity, zero/empty guard, order-independent) +
  `sweep::cell` (`eval_cell` pure wrapper of run+Metrics; CellResult with Decimal keys; non-finite f64→None). +8 tests.
- M4 S7 (DETERMINISM GATE): `sweep::parallel` — `run_in_parallel` (std::thread::scope, contiguous index chunks,
  disjoint writes into pre-sized Vec<Option<R>>, read back in index order), `Parallelism`, `SweepCell`, `run_cells`.
  tests/determinism.rs: parallel==sequential structural + byte-identical for K∈{1,2,3,7,8}; repeated-run identical;
  anti-vacuous guard (most cells trade). Zero new deps (rayon rejected, D-0009). +5 tests.
- Re-gate after S4–S7: fmt clean; clippy -D warnings clean; **109 tests pass**; no-execution-deps OK; demo
  byte-identical. Deterministic sweep core complete. Next: S8 (walk-forward windows).
- Refreshed `plans/handoff.md` for a new chat: M4-in-progress (S1–S7 done), m4-sweep.md promoted to the active
  plan, verified state = 109 tests + no-exec-deps, S8-first "what's next", M4-specific guardrails (determinism
  gate / traded_notional turnover / Decimal keys / sealed holdout), and an updated seed prompt.
- M4 S8 (walk-forward windows): `sweep::window` — `Window { train, test: Range<usize> }`,
  `WalkForward { kind, train_len, test_len, step, embargo }`, `WindowKind::{Rolling default, Anchored}`,
  `WindowError` (hand-rolled, SimError-style). `new()` validates (rejects zero lengths + `step<test_len`
  overlap); `windows(n_bars)` is pure + total + defensive (saturating math, struct-literal bypass yields
  empty not infinite-loop). Invariants structural: `train.end + embargo == test.start`, ordered/non-overlapping
  tests, in-bounds. Zero new deps. +10 tests (3 unit + 7 integration).
- M4 S8 verification: ran a 5-lens adversarial workflow (boundary/invariants/determinism/edge/tests) + adjudicator
  (6 agents). Verdict: **impl correct** (formula hand-derived across configs; gate bullets a–e hold; pure/
  terminating), but **two real test-coverage gaps**: embargo only asserted `<=` (a too-large-embargo mutant
  survived) and Anchored shape only tested at `step==test_len` (a "grow by test_len" mutant survived). Closed both:
  strengthened `assert_fold_invariants` to pin every fold against the gate formula by index; added exact tests for
  `embargo≥2` and Anchored `step>test_len`; anchored the determinism test to real values; honest doc on the
  `usize::MAX` termination edge (unreachable). +2 tests (now 9 integration).
- Re-gate after S8: fmt clean; clippy -D warnings clean; **121 tests pass**; demo byte-identical; no-execution-deps
  OK. Walk-forward model complete. Next: S9 (holdout gate — physical partition + consume-by-value seal).
- M4 S9 (HOLDOUT GATE — invariant 11 enforcement): new `sweep::config` (`PartitionSpec::{ByIndex, ByDate}`;
  `by_date()` via the S2 `parse_ymd`) + `sweep::partition` — `PartitionedBars::from_spec` validates the series and
  **physically moves** the holdout into a SEPARATE `Vec<Bar>` (`split_off`, not an aliasing borrow); rejects
  overlap/empty/out-of-bounds. `seal_holdout(self) -> (DevValidation, Sealed)`: `DevValidation` exposes
  development/validation/dev_validation/walk_forward_windows and has NO holdout accessor (windows run over dev/val
  length only → structurally can't reach holdout). `evaluate_on_holdout(sealed: Sealed, …)` consumes by value
  (call-once; M5-only, never the M4 CLI); the single read goes through a `Cell<u32>` counter gateway. Gate
  (tests/holdout_sealing.rs + doctests): read-counter==0 + digest unchanged after the full sweep+selection pipeline
  (holds only `&DevValidation`); 3 `compile_fail` doctests (no selection→holdout path, no seal fabrication, call-once
  consume-by-value) each with a `no_run` canary so they fail for the RIGHT reason; overlap rejected (ByIndex+ByDate).
  Zero new deps. +23 tests (3 config + 10 partition unit + 5 integration + 5 doctests).
- M4 S9 verification: ran a 6-lens adversarial review (seal-bypass / counter-soundness / compile_fail-soundness /
  determinism / validation-edges / invariant-scope) + per-finding skeptics (12 agents). Verdict: **seal SOUND, no
  blockers** (physical separation, no DevValidation→holdout path, call-once reader all verified). 3 confirmed
  (minor/nit), all fixed: (1) `from_spec` now delegates to canonical `market_data::validate_series` so duplicate
  timestamps are rejected (was only non-decreasing — drifted from the platform's sorted+unique contract);
  `PartitionError` folds empty/OHLC/unsorted/dup into `InvalidSeries(DataError)`; +1 dup-rejection test. (2) replaced
  the tautological pre-split `debug_assert` with a real post-`split_off` structural assert. (3) tightened the digest
  doc to "within-process witness only, not a durable cross-toolchain identifier" (DefaultHasher isn't version-stable).
  3 findings refuted (digest is an audit hook unreachable from selection; CLI doesn't depend on sweep yet; the
  `holdout()` compile_fail is sound alongside the seal-bypass lens).
- Re-gate after S9: fmt clean; clippy -D warnings clean; **144 tests pass**; demo byte-identical; no-execution-deps
  OK. Holdout gate complete. Next: S10 (cost/fee sensitivity ladder).
