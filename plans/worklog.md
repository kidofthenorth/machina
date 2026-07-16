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
- M4 S10 (COST/FEE SENSITIVITY LADDER + cost-matched baselines): refactored `sweep::cell` to extract a shared
  `pub(crate) eval_strategy(&dyn Strategy, …)` core (eval_cell delegates — candidates and baselines now score through
  byte-identical logic). New `sweep::sensitivity`: `ScenarioId` {before_costs, base, doubled, doubled_slippage,
  doubled_priority}, `scale_cost_model(base, num, den)` exact integer scaling (u32 bps / i64 lamports, widened, no f64;
  Doubled = ×2/1), `cost_scenarios` ladder, `ScenarioMetrics`, `FeeSensitivity` + `fee_sensitivity(before, base,
  doubled, survives_floor)` (Decimal `return_drag_costs`/`return_drag_doubled`; baseline-grounded `survives_doubled`).
  New `sweep::baseline`: `BaselineId` {hold_usdc, buy_and_hold_sol, static_50_50, dca_sol}, `eval_baseline` /
  `eval_all_baselines` (cost-matched, canonical order), `best_baseline_return` (the "return to beat" floor). Zero new
  deps. +8 tests (4 sensitivity + 4 baseline).
- M4 S10 verification: ran a 3-lens adversarial review (scaling-exactness / cost-monotonicity / baseline-consistency-
  scope) + per-finding skeptics (9 agents). Verdict: **no code defects, no blockers**; baseline-consistency-scope
  clean. Key finding (minor): the S10 gate's "doubled ≥ base fees, ≤ base return" is **NOT a universal law** — higher
  costs can suppress a marginal trade (`min_trade_notional`/`InsufficientSolForGas` skips in the simulator), so a
  suppressed net-losing trade can leave doubled *cheaper* and *higher-return* (`return_drag_doubled` negative).
  Fixed: (1) reworded the monotonicity test + added an explicit `n_trades`-equality guard so it asserts the property
  only under an unchanged trade set; (2) added a regression test pinning the honest inversion (sub-cent account: base
  1 trade, doubled 0 trades, negative drag); (3) corrected the `return_drag_doubled` field doc (can be negative;
  trade-set change visible via `n_trades`); (4) documented `scale_cost_model`'s toward-zero truncation for fractional
  scaling; (5) added a conditional-monotonicity note to m4-sweep §9. 2 findings refuted (unchecked-cast overflow and
  negative-lamport doubling — both unreachable: CostModel has no Deserialize; all constructions are hardcoded positive
  literals). +1 regression test.
- Re-gate after S10: fmt clean; clippy -D warnings clean; **153 tests pass**; demo byte-identical; no-execution-deps
  OK. Cost/fee sensitivity complete. Next: S11 (advancement/rejection report + new sweep-report schema).
- M4 S11 (ADVANCEMENT/REJECTION REPORT + new schema, D-0001): new `sweep::advance` — `AdvancementThresholds` /
  `CandidateEvidence` (all Decimal), `Verdict::{Advanceable,Rejected}`, `RejectionKind` (7 §11 criteria),
  `RejectionReason{kind,observed,threshold}` (observed/threshold as exact decimal strings), `evaluate_candidate`
  (InsufficientData hard-stop first; then a fixed canonical order of criteria; Advanceable iff no failures). New
  `sweep::report` — `SweepReport{schema_version,note,trial_count,thresholds,verdicts}` with a robustness-only `note`
  (invariant 11), canonical `to_json`/`to_value`, verdicts sorted. New `schemas/sweep-report.schema.json`
  (Draft 2020-12, additionalProperties:false, decimalString; **run-result schema UNCHANGED**) + `tests/
  schema_validation.rs` mirroring `results` — a REAL built report validates; rejects numeric budget / unknown
  status / unknown reason kind / extra props; source-level f64-audit proves advance.rs + report.rs are Decimal-only;
  trial_count recorded; two serializations byte-identical. Zero new deps (jsonschema/rust_decimal_macros pre-existing
  dev-deps). +15 tests.
- M4 S11 verification: ran a 3-lens adversarial review (schema-fidelity / advancement-logic / invariant-scope) +
  per-finding skeptics (7 agents). Verdict: **schema fidelity clean, no blockers** (real report validates; decimalString
  covers all reachable Decimal.to_string outputs; run-result untouched). 3 confirmed (all nit), all fixed:
  (1) `EdgeVanishesUnderDoubledCosts` predicate decoupled from its recorded observed/threshold → collapsed to a single
  source of truth (`doubled_return <= doubled_baseline_floor`, matching S10's strict `>`; dropped the redundant
  `survives_doubled` bool); (2) verdict sort keyed only on label (duplicate labels — reachable via scale-variant grid
  points — could make byte-output input-order-dependent) → derived `Ord` on the verdict chain + total-order
  `verdicts.sort()`, pinned by a duplicate-label determinism test; (3) status⇔failed_criteria coupling is code-only →
  added a schema/plan note that it's deliberately code-enforced (house convention, cf. run-result `unitInterval`).
  1 refuted (f64-audit's two-file scope is intentional — sibling modules use f64 only for display-only stats, already
  normalized to None at `CellResult`). +1 test.
- Re-gate after S11: fmt clean; clippy -D warnings clean; **169 tests pass**; demo byte-identical; no-execution-deps
  OK; run-result.schema.json unchanged. Report + schema complete. Next: S12 (CLI + DECISIONS D-0009 + docs — last M4).

## 2026-07-01 — Operator committed through S11; post-S11 test hardening (logged retroactively 2026-07-06)
- Operator committed the M0–M4 tree: S1–S7 = `311aedb`, S8 = `08d5647`, S9 = `632fdad`, S10 = `9f591bb`,
  S11 = `5e5b0ba` (advance/report + sweep-report schema + plans refresh). Repo is no longer BASE=EMPTY.
- 9 further commits (`375288b..df18267`), all **pure test hardening** — +103 tests, 0 removed, no behavior,
  schema, config, plan, or CLI-logic change: metrics +13; cli +6; results +8; research-core +19 (incl.
  validated TokenMeta constructor); strategies +14; portfolio +15; market-data +14; sweep +14
  (walk-forward/advancement/parallel); `df18267` rustfmt-only. HEAD = `df18267`; working tree clean.

## 2026-07-06 — Plan/context reconciliation audit + S12 task-card expansion (plan/context edits only, no code)
- Ran an 8-agent read-only audit (Sonnet subagents): per-claim verification of S1–S11 against source with
  verbatim signature capture; full gate; stale-pointer sweep; git-vs-worklog diff.
- Gate from this tree (2026-07-06): `cargo fmt --all --check` clean; `cargo clippy --all-targets
  --all-features -- -D warnings` clean; `cargo test --workspace --all-features` → **272 passed, 0 failed**
  (169 at S11 + 103 hardening); `cargo run -q -p cli -- demo | shasum` byte-identical twice (`ae064f79…`);
  no-execution-deps scan OK.
- **Drift table** (claim → verified reality → action):

| # | Claim / pointer (where) | Verified reality | Status | Action |
|---|--------------------------|------------------|--------|--------|
| 1 | S1–S8, S10, S11 DONE (m4-sweep.md §14; current-state.md) | Every named module/type/test exists and passes; signatures matched verbatim | CONFIRMED | none |
| 2 | S9 DONE: "3 `compile_fail` doctests **each with a `no_run` canary**" (m4-sweep S9 row; current-state; handoff; worklog 2026-06-29) | 3 `compile_fail` exist (partition.rs:288, :356, :427) but only 2 have `no_run` canaries — the `Sealed` no-forgery doctest has none. Seal itself sound; counter/digest/overlap gates green | PARTIAL (doc overstatement) | wording corrected; missing canary queued as card **M4-C1** |
| 3 | "169 tests" (current-state; task-queue; handoff) and "85 tests" (AGENTS.md) | 272 pass at `df18267` | STALE | updated everywhere |
| 4 | "committed through S10 (`9f591bb`); S11 uncommitted" (handoff.md); "no commits — BASE=EMPTY" (current-state; review-packet) | S11 committed (`5e5b0ba`) + 9 test commits; tree clean | STALE | updated |
| 5 | Worklog had no 2026-07-01 entries | 9 unlogged commits | GAP | logged above |
| 6 | CLAUDE.md "M4 … not started"; AGENTS.md "M1–M2 complete with M3 scaffolds", "7 research crates", `sweep` listed **Deferred** | M4 S1–S11 built; 8 crates; `crates/sweep` has 12 modules + 4 test targets | STALE | updated |
| 7 | README status "Milestone M2, with M3 scaffolds"; crate table omits `sweep`; deferred list says `sweep` "not yet created" | as row 6 | STALE | updated |
| 8 | docs/architecture-index.md: "Crates that exist (M0–M3)" omits `sweep`; deferred table lists `sweep`; schemas line omits `sweep-report.schema.json` | sweep crate + schema exist | STALE | updated |
| 9 | questions.md Q5 "safe default (applied): `[walk_forward]`/`[advancement]` illustrative defaults in strategy-lab.example.toml; the engine reads them from config" | Template has NO `[walk_forward]`, NO `[advancement]`, NO grid arrays; sweep crate has NO `Deserialize` anywhere (`toml` dep declared but unused) | STALE (aspirational) | Q5 reworded; work queued as card **M4-C2** |
| 10 | m4-sweep §13 / handoff: CLI loads "embedded `strategy-lab.example.toml` + `allowlist.example.toml`" | Real paths: `config/strategies/strategy-lab.example.toml`, `config/tokens/allowlist.example.toml` | IMPRECISE | exact paths pinned on cards |
| 11 | m4-sweep §13 "builds a `SweepSpec` … runs `run_sweep`" | No `SweepSpec`, no `run_sweep`, no orchestrator anywhere in the tree — the CLI must compose ~12 primitives; evidence aggregation (fold dispersion, neighbor degradation, per-window baseline floors) exists nowhere | S12 UNDERSPECIFIED | S12 expanded into cards **M4-C1…M4-C10** in task-queue.md |
| 12 | review-packet.md header/gates (BASE=EMPTY; 85 tests; STEP=M2+M3) | Historical snapshot of the M0–M3 review, superseded | STALE (historical) | supersession banner added; body kept as record |
| 13 | docs/invariants.md:70 "full validation battery … out of scope until M4–M5" | Battery engine now exists (S1–S11); the invariant's intent (no profit claims before the M5 decision) still holds | ACCEPTED | no action |

- Refreshed in this pass: current-state.md, task-queue.md (S12 → cards M4-C1…C10), handoff.md, CLAUDE.md,
  AGENTS.md, README.md, docs/architecture-index.md, m4-sweep.md (S9 wording; §13 paths + orchestration note),
  questions.md (Q5), review-packet.md (banner). **master-plan.md untouched** (authoritative roadmap).
- M4 remaining work now lives as small-model task cards **M4-C1…M4-C10** in task-queue.md, each sized
  ≤3 files / ~150 lines with inline signatures, exact steps, gates, guardrails, and escalate-ifs.
  Queue ends at the M4 gate declaration + M5 handoff pointer; **no M5 implementation is queued** (M5 needs
  real data — Q3 — and its own explicit go decision).

## 2026-07-06 — Mission-language sharpening + operator-chat handoff (plan/context edits only, no code)

- Operator feedback: the earlier reconciliation pass had preserved "it does not promise passive
  income, and live trading is never automatic" verbatim. The operator corrected this: machina is a
  private system for the operator's own capital, not a public product — the real goal is **genuine
  autonomous passive income**, and it's critical the goal itself not be stigmatized in the project's
  own language. The actual safety mechanism is structural (the milestone-gate + explicit-approval
  requirements in `AGENTS.md`'s "Never do" section), not the framing — naming the goal plainly
  doesn't loosen those gates.
- Sharpened the mission paragraph (identical sentence, three places) in `AGENTS.md`, `README.md`,
  and `plans/handoff.md`'s 30-second orientation: "it does not promise passive income, and live
  trading is never automatic" → "built toward one goal: genuine autonomous passive income … no
  strategy is guaranteed to clear that bar, and live trading starts only once its own milestone gate
  is explicitly approved." The uncertainty hedge now attaches to *whether a strategy proves out*,
  not to the goal. The gate-mechanism paragraph immediately below (money-moving capability gated by
  milestone + explicit approval) was left verbatim — that's the real mechanism, not framing.
- **`plans/master-plan.md` line 18 and the root `solana-crypto-trader-plan.md` (declared
  byte-identical to each other, confirmed via `diff`) still carry the old sentence — flagged as
  **Q6** in `plans/questions.md` rather than edited**, since master-plan.md is elsewhere marked
  "authoritative roadmap, don't reword" and the pair would need lockstep edits to stay identical.
- Gate: no code/schema touched this pass — `cargo test`/`clippy`/`fmt` unaffected, still green at the
  272-test baseline. `git status` after this pass: `AGENTS.md`, `CLAUDE.md`, `README.md`,
  `docs/architecture-index.md`, `plans/{current-state,handoff,m4-sweep,questions,review-packet,
  task-queue,worklog}.md` modified; nothing committed (the operator commits).

## 2026-07-06 — Q6 resolved: goal stated plainly in the authoritative plan pair (plan edits only, no code)

- Operator affirmed Q6: the new statement IS the true goal — "autonomous passive income. truly
  autonomous so its truly passive income." Swept the last old-framing sentence out of the
  authoritative pair: `plans/master-plan.md` §1 Mission and the root `solana-crypto-trader-plan.md`
  edited **identically** ("This plan does not promise passive income. It creates the machine…" →
  "The goal is genuine autonomous passive income — truly autonomous operation, so the income is
  truly passive. This plan builds the machine that earns that autonomy… advances toward live trading
  only through the explicit milestone gates below. No strategy is guaranteed to clear that bar.").
  Pair re-verified **byte-identical** via `cmp`. Milestone definitions/gate criteria untouched;
  line 1764 (goal used as a rejection criterion) untouched.
- Carried the goal into the working files so it isn't hedged out of status/queue language:
  one goal line each in `plans/current-state.md` (header), `plans/task-queue.md` (header), and the
  handoff seed prompt. Structural safety language unchanged everywhere (invariant 11 robustness-only
  wording, M8/M9 explicit-approval gates, holdout seal).
- `AGENTS.md` provenance note updated: the root plan is "kept byte-identical" to master-plan.md —
  the rule is now **edit both or neither** (Q6). Also fixed a duplicate pointer in
  `plans/handoff.md`'s read-first list (item 4 now points at m4-sweep.md, not task-queue twice).
- `plans/questions.md`: Q6 marked RESOLVED with the resolution recorded; status line now
  "0 blocking, 5 non-blocking open, 1 resolved".
- Gate: no code/schema touched; `cmp plans/master-plan.md solana-crypto-trader-plan.md` → identical;
  repo grep confirms no "does not promise passive income" remains outside historical worklog/Q6
  records. Nothing committed (the operator commits).

## 2026-07-07 — M4-C1: restored the missing `Sealed` no-forgery doctest canary
- `crates/sweep/src/partition.rs`: added the `no_run` canary doc block (exercising
  `holdout_read_count`/`holdout_digest`/`holdout_len`) right after `Sealed`'s `compile_fail` block, so
  all three holdout no-forgery doctests now have their canary (matches the existing `DevValidation`
  pattern). Gate: `cargo test -p sweep --doc` → 6 passed, 0 failed (was 5); full-workspace gate
  (`cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test
  --workspace --all-features`) green, **273 passed, 0 failed**. Nothing committed (the operator commits).

## 2026-07-07 — M4-C2: `sweep::spec` parses the strategy-lab TOML
- `config/strategies/strategy-lab.example.toml`: added grid axes to `[trend_alloc_v1]` and
  `[threshold_rebalance_v1]`, plus new `[walk_forward]` (rolling, train 365 / test 90 / step 90 /
  embargo 5) and `[advancement]` (illustrative budgets, NOT tuned — Q5) tables.
- `crates/sweep/src/spec.rs` (new): `SweepSpec::from_toml_str` — pure TOML → `SweepSpec` (partition,
  walk-forward, advancement thresholds, enabled param grids in fixed family order), `SpecError` for
  parse/date/window/unknown-kind failures. `rust_decimal`'s `serde` feature deserializes the quoted
  Decimal strings from TOML directly, no workaround needed. 3 new unit tests (example template
  resolves; unknown `walk_forward.kind` rejected; disabled family omitted).
- `crates/sweep/src/lib.rs`: `pub mod spec;` + `pub use spec::{SpecError, SweepSpec};`.
- Gate: `cargo test -p sweep spec` → 3 new tests pass; full-workspace gate (fmt/clippy/test) green,
  **276 passed, 0 failed**; `cargo run -q -p cli -- demo | shasum` twice → identical (`ae064f79…`,
  unaffected — the demo doesn't read this template). Nothing committed (the operator commits).

## 2026-07-07 — M4-C3: `sweep::runner` canonical cell enumeration
- `crates/sweep/src/runner.rs` (new): `enumerate_cells(points, scenarios, windows) -> (Vec<SweepCell>,
  Vec<CellKey>)` — canonical order window (outer) → scenario → point (inner), `cells[i].index == i`
  assigned before any thread spawns. 1 new unit test asserting the exact 8-cell key sequence over 2
  windows × 2 scenarios × 2 points, plus per-cell test-range/cost equality to its window/scenario.
- `crates/sweep/src/lib.rs`: `pub mod runner;` + `pub use runner::{enumerate_cells, CellKey};`.
- Gate: `cargo test -p sweep runner` → new test passes; full-workspace gate (fmt/clippy/test) green,
  **277 passed, 0 failed**. Nothing committed (the operator commits).

## 2026-07-07 — M4-C4: `sweep::runner` aggregates cell results into `CandidateEvidence`
- `crates/sweep/src/runner.rs`: added `aggregate_evidence(grids, keys, results, n_windows,
  base_floors, doubled_floors) -> Vec<CandidateEvidence>` — one pass over Base/Doubled cells per
  point (means, worst-case max_drawdown/turnover), plus `neighbor_indices`/`in_grid_neighbors`
  helpers reconstructing each point's row-major grid position (last axis fastest, matching
  `ParamGrid::points()`) to find axis-neighbors for `neighbor_degradation` (floored at 0). Only
  division is by window/floor count, exact Decimal throughout. 3 new unit tests: exact means/
  dispersion/baseline-margin/doubled-floor over hand-built `CellResult`s; worst-axis-neighbor
  degradation over a 3-point 1-D grid (middle point 0.4, edges floor at 0); empty-windows path
  yields all-zero evidence with `valid_windows == 0`.
- `crates/sweep/src/lib.rs`: added `aggregate_evidence` to the `pub use runner::…` line.
- Gate: `cargo test -p sweep runner` → all 4 runner tests (C3 + C4) pass; full-workspace gate
  (fmt/clippy/test) green, **280 passed, 0 failed**. Nothing committed (the operator commits).

## 2026-07-07 — M4-C5: `run_sweep` end-to-end + report-level determinism test (M4 gate)
- `crates/sweep/src/runner.rs`: added `run_sweep(spec, bars, base_cost, initial_cash_usdc,
  periods_per_year, parallelism) -> Result<SweepOutcome, SweepError>` — the single orchestration
  entry point: `PartitionedBars::from_spec` → `seal_holdout` → `walk_forward_windows` →
  `cost_scenarios` (first 3 rungs: BeforeCosts/Base/Doubled) → `enumerate_cells` → `run_cells` →
  per-window baseline floors (base + doubled cost) → `aggregate_evidence` → `evaluate_candidate` →
  `SweepReport::new`. Returns `SweepOutcome { report, sealed }` with the seal unconsumed;
  `evaluate_on_holdout` is never called or referenced (grepped clean). `SweepError` wraps
  `PartitionError`/`SimError` via `From`.
- `crates/sweep/src/lib.rs`: added `run_sweep`, `SweepError`, `SweepOutcome` to the
  `pub use runner::…` line.
- `crates/sweep/tests/sweep_runner.rs` (new): 4 tests on a 160-bar sawtooth series (same fixture
  family as `holdout_sealing.rs`), 2-family grid (TrendAlloc × ThresholdRebalance, 4 points total) —
  (a) report `to_json()` byte-identical across Sequential (×2), Threads(2), Threads(8); (b) all four
  runs have `holdout_read_count() == 0`, `holdout_len() == 30`, matching `holdout_digest()`; (c)
  exactly 4 verdicts, labels sorted+unique; (d) `report.to_value()` validates against
  `schemas/sweep-report.schema.json`.
- Gate: `cargo test -p sweep --test sweep_runner` → 4/4 pass; `cargo test -p sweep` → all sweep
  tests (unit+determinism+holdout_sealing+walk_forward+schema_validation+sweep_runner+doctests)
  pass; full-workspace gate (fmt/clippy -D warnings/test) green, **284 passed, 0 failed**;
  `cargo run -p cli -- demo` hash unchanged (`ae064f79…`, run twice). This closes the M4-C5 card and
  discharges all three M4 gate criteria (parallel==sequential, repeated-identical, holdout sealed)
  at the `run_sweep` orchestration level. Nothing committed (the operator commits).

## 2026-07-07 — Operator direction decision recorded (Q7; plans/questions.md — no code)

- The operator surfaced that the true ambition is a **high-volume autonomous Solana trading bot**
  (small per-trade edges, thousands of trades/day) — a direction the plan's "Rejected or notes-only
  directions" section had shelved. Decision (recorded as **Q7** in questions.md): **finish the
  M4-C1…C10 queue unchanged**, then amend master-plan.md + the root plan file (in lockstep,
  byte-identical) to add a **high-frequency research track** (intraday/tick data, latency-aware
  adversarial fill model, HF strategy family). HF is now *deferred-then-planned*, not rejected.
  The M4 executor queue, gate criteria, and M8/M9 approval gates are all unchanged.

## 2026-07-07 — C5 adversarial review (5 lenses + per-finding skeptics, Sonnet subagents) → PASS
- Reviewed the committed C5 diff (`721e21b`): orchestration / determinism / holdout-seal /
  evidence-math / test-quality+card-conformance lenses, 2 skeptics per finding. Verdict: **no
  blockers, no majors**; implementation matches card M4-C5 verbatim; ladder[..3] cost-matched to
  the doubled floor path; no unordered iteration or f64 in decision paths; holdout lens re-run
  standalone → all 5 seal checks PASS (Sealed unconsumed; audit hooks provably non-reading;
  partition.rs untouched by the commit).
- 1 confirmed minor (survived both skeptics): no test pinned `trial_count`/the scenario set — a
  `ladder[..2]`, swapped-floor, or points-vs-cells mutant would survive the 4 tests. **Fixed**: added
  `trial_count_is_windows_times_three_scenarios_times_points` to tests/sweep_runner.rs (expected
  count recomputed from the fixture's own windows()/points() APIs, not hardcoded). Note: a
  swapped-base/doubled-floors mutant is still not directly pinned (thresholds in this fixture are
  deliberately loose); acceptable — the C9 gate declaration re-runs the full battery.
- Re-gate: fmt clean; clippy -D warnings clean; **285 tests, 0 failed** (5/5 sweep_runner); demo
  unchanged. Next: M4-C6 (CLI `machina sweep`).
- 2026-07-07: Drafted plans/highfrequency-algo-plan.md (operator renamed from hf-track.md) (Q7 HF research-track amendment draft: 7-family candidate survey, execution-realism requirements, M-HF-C1…C10 card draft ending at a research gate). Operator review pending; no plan-pair or code edits.
- 2026-07-07: M4-C6 done — `machina sweep [--threads N] [--out PATH]` (added `sweep = { workspace = true }` to crates/cli/Cargo.toml; embedded strategy-lab template; `sweep_report_json` asserts `holdout_read_count() == 0`). Gate: `cargo test -p cli` 8/8; `sweep` shasum ×2 and `--threads 8` all byte-identical (`7d385d59…`); `demo` shasum unchanged (`ae064f79…`); fmt/clippy clean; full workspace **0 failed**; no-execution-deps scan OK. Next: M4-C7 (`sweep-verify`).
- 2026-07-07: M4-C7 done — `machina sweep-verify` (local mirror of the CI determinism gate: Sequential vs Threads(2)/Threads(8) vs repeat, raw-string comparison, no re-parsing). Gate: `cargo run -q -p cli -- sweep-verify` → `sweep-verify: OK — byte-identical across sequential, 2 and 8 threads, and repeat (5446 bytes)`, exit 0; `cargo test -p cli` 9/9 (new `sweep_verify_inputs_agree`); fmt/clippy clean; full workspace `cargo test --workspace --all-features` **0 failed**; `sweep` shasum unchanged (`7d385d59…`). Next: M4-C8 (DECISIONS D-0009 + docs refresh).
- 2026-07-07: M4-C8 done — docs-only: recorded DECISIONS D-0009 (thread::scope parallelism/zero new deps, traded-notional turnover, sweep-report schema, sealed holdout — M5-only `evaluate_on_holdout`); updated docs/architecture-index.md's `cli` row (dependency cell → "all of the above"; responsibility cell now lists demo/sweep/sweep-verify subcommands); added the sweep/sweep-verify commands to AGENTS.md Commands. No code/schema/plan-structure changes. Gate: `grep D-0009 DECISIONS.md` and `grep sweep-verify docs/architecture-index.md AGENTS.md` all hit; fmt clean; clippy `-D warnings` clean; full workspace `cargo test --workspace --all-features` **0 failed**. Next: M4-C9 (M4 gate declaration).

## 2026-07-07 — M4-C9 pre-declaration adversarial review → STOP, gate NOT declared (1 confirmed major)
- Per operator addendum, ran a pre-declaration adversarial-review workflow (Sonnet subagents,
  `model: 'sonnet'` on every agent call) BEFORE running the C9 evidence battery: 5 parallel lenses
  (determinism, holdout-seal, evidence-math, invariant-sweep, master-plan-conformance) over
  `crates/sweep/src/*.rs`, `crates/sweep/tests/*.rs`, the sweep parts of `crates/cli/src/main.rs`,
  and `schemas/sweep-report.schema.json` (read-only), then 2 skeptic agents per raised finding
  (refute with file:line evidence; a finding survives unless BOTH skeptics refute it). 11 agents
  total; 3 findings raised, 0 refuted — all 3 confirmed.
- **Confirmed MAJOR** (master-plan-conformance lens): the "Turnover and fee-sensitivity reporting"
  M4 deliverable (master-plan.md:873-889; m4-sweep.md §9 specifies a per-candidate `fee_sensitivity`
  block — total_return/turnover/n_trades/fees_paid_usdc/slippage_paid_usdc/priority_fees_paid_sol
  under base vs doubled, `return_drag_doubled`, `survives_doubled`) is only enforced as an internal
  pass/fail gate. `sensitivity::fee_sensitivity()`/`FeeSensitivity`/`ScenarioMetrics` are re-exported
  from `sweep::lib.rs` (lib.rs:57-58) but have ZERO production callers — confirmed independently via
  `grep -rn "fee_sensitivity\|FeeSensitivity\|ScenarioMetrics" crates/`, whose only call sites outside
  `sensitivity.rs` are doc-comment mentions in `runner.rs`/`advance.rs`; `run_sweep` never calls
  `fee_sensitivity`. `SweepReport`/`CandidateVerdict` (report.rs:48-54, advance.rs:116-120) and
  `schemas/sweep-report.schema.json` carry no fee-sensitivity/scenario-metrics field — verified
  directly by reading both. `machina sweep`'s exported JSON therefore has no candidate-level
  fee-sensitivity reporting, only the single worst-case `turnover: Decimal` feeding the
  `turnover_implausible` rejection criterion. Real gap between the shipped artifact and the
  master-plan Deliver bullet, not a refuted false positive.
- 2 confirmed MINORs (survived both skeptics):
  (1) evidence-math: `InsufficientData` (`valid_windows < min_windows`) is wired and unit-tested via
  hand-built `CandidateEvidence`, but never exercised end-to-end through `run_sweep` with a real
  under-populated walk-forward schedule (the only `run_sweep`-based fixture, `sweep_runner::spec()`,
  sets `min_windows: 1`, below its realized window count, so the hard-stop can't fire there).
  (2) master-plan-conformance: `cli::sweep_cmd` (the `--threads`/`--out` argument parser and file-write
  path — the CLI half of "canonical result export") has no automated test; the test module calls
  `sweep_report_json` directly and never exercises `sweep_cmd`'s arg loop, error branches, or the
  `--out` file write.
- **Decision (operator's decision rule): STOP — M4 gate NOT declared.** A confirmed major means the
  evidence-checklist row for "Turnover and fee-sensitivity reporting" lacks a real artifact for its
  reporting half (only the gating half is real). Per the rule, a confirmed major/blocker overrides the
  minors-handling branch, so the minors' ≤10-line test-fix allowance was intentionally NOT used this
  pass. No code changed (review was read-only); no plan files flipped beyond this entry —
  `m4-sweep.md`/`current-state.md`/`task-queue.md` are untouched and **M4-C9 remains `TODO`**. Step-1
  gate commands were not re-run as "declaration evidence" since the review already established the
  gate cannot be declared this pass.
- Escalating to the operator / next session: either (a) wire `fee_sensitivity` into `SweepReport` +
  `schemas/sweep-report.schema.json` (code + schema change, out of C9's plan-files-only file scope —
  needs its own card) so the deliverable is real before re-attempting C9, or (b) the operator
  explicitly accepts the internal-gate-only scope as satisfying "reporting" and directs the executor to
  proceed with C9 as-is. This is a scope call reserved for escalation, not an executor decision.

## 2026-07-07 — Scope decision on the pre-declaration review major: wire fee sensitivity into the report
- Operator decision (option a): the review's confirmed major is accepted as real — "turnover and
  fee-sensitivity reporting" must be satisfied IN THE EXPORTED ARTIFACT, not by an internal-gate
  technicality. Declaring M4 as-is would be the self-deception the battery exists to prevent.
- Queue expanded: cards **M4-C8b** (runner-side `aggregate_fee_sensitivity` + single-source
  `FeeSensitivity::from_scenarios`), **M4-C8c** (`SweepReport.candidates[]` with per-candidate
  worst-window drawdown/turnover + fee-sensitivity block; `sweep-report.schema.json` 1.0.0→1.1.0 —
  a deliberate D-0001 contract change with this review as recorded cause; DECISIONS **D-0010**),
  **M4-C8d** (the review's two minors: InsufficientData end-to-end via run_sweep; CLI
  `--threads/--out` parser extracted pure + tested). C9's evidence table and preconditions updated
  to require C8b–C8d.
- Design pins: aggregation matches evidence rules (mean returns / worst-window turnover+drawdown /
  summed costs); `survives_doubled` shares floor and comparison with the edge-vanishes criterion via
  one constructor so report and verdict cannot drift; candidates label-sorted for byte-identity; the
  sweep output hash will change at C8c (report grows) — byte-identity across runs/threads remains
  the invariant, and C8c's gate re-proves it via sweep-verify.
- M4 gate remains UNDECLARED; C9 unchanged otherwise and still TODO.

## 2026-07-07 — Card M4-C8b
- `sensitivity.rs`: added `FeeSensitivity::from_scenarios` (sole owner of the drag/survival rules);
  `fee_sensitivity` now delegates to it, unchanged signature/behavior.
- `runner.rs`: added `aggregate_fee_sensitivity(grids, keys, results, doubled_floors) ->
  Vec<FeeSensitivity>` — mean returns, worst-window turnover, summed trades/fees/slippage/priority
  per scenario slot, `survives_floor = mean(doubled_floors)` (same value `aggregate_evidence`
  records as `doubled_baseline_floor`). Re-exported from `lib.rs`. Nothing calls it yet (C8c wires it
  into `SweepReport`).
- Gate: `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --workspace --all-features` all green, 0 failed (sweep unit tests 70→72). `cargo run -p
  cli -- sweep | shasum` unchanged at `7d385d59…` (twice, identical); `demo | shasum` unchanged at
  `ae064f79…` — confirms no behavior change on this card.
- Card flipped to DONE. Next: C8c (fresh session) wires `aggregate_fee_sensitivity` into
  `SweepReport.candidates[]`, schema 1.1.0, D-0010.

## 2026-07-07 — Card M4-C8c → ESCALATED (BLOCKED): zero-cost scenario reports nonzero slippage (1 ulp)
- **Built per card (all 6 files, diff left in working tree, uncommitted):** `report.rs`
  (SWEEP_SCHEMA_VERSION 1.1.0; ScenarioMetricsDto/FeeSensitivityDto/CandidateMetricsDto;
  `SweepReport.candidates` sorted beside verdicts), `runner.rs` (run_sweep wires
  `aggregate_fee_sensitivity` → `candidates`; BeforeCosts comment reworded), schema 1.1.0
  (`candidates` required + 3 `$defs`, additionalProperties:false), schema_validation.rs (builder passes
  a real candidate; 2 new rejection tests), sweep_runner.rs (first-class test), DECISIONS.md (D-0010).
- **Blocker — card step 6's assert "every before_costs block reports zero costs" is false for the sim
  as built.** `portfolio::cost::fill_buy` computes the reporting-only figure
  `slippage_quote = quote_in − (quote_in · price / eff)` (cost.rs:77). With `slippage_bps = 0`,
  `eff == price`, so it is mathematically zero — but `rust_decimal` rounds `quote_in · price` at 28
  significant digits when `quote_in` carries high scale (balances do mid-run), leaving a ±1-ulp
  residue: the 160-bar test fixture reports `slippage_paid_quote = 0.0000000000000000000000001` on the
  BeforeCosts rung. Fees/priority are exact zeros (their identities multiply by 0; no division). The
  CLI template fixture happens to produce value-zero blocks, so the exported artifact doesn't show it
  today — but the property is fixture-dependent, not guaranteed. Pre-existing M2 behavior; C8c is
  merely the first card to EXPORT these fields.
- Secondary card nit found on the way: zero Decimals stringify with inherited scale
  (`"0.0000000000000000000000000"`), so step 6's literal `== "0"` string assert can't hold with the
  card-mandated `.to_string()` DTO encoding (house style — verdicts already export full-scale strings).
  Adapted the assert to value-equality (`parse::<Decimal>() == ZERO`), which is what then exposed the
  real residue above.
- **Why escalated, not adapted:** the honest fix is in `crates/portfolio/src/cost.rs` — a file the
  card doesn't list — and every in-scope alternative either weakens the carded zero-cost assertion or
  masks exported money values (rounding/normalizing in the DTO). Queue rule: escalate, don't improvise.
  Suggested resolution for a follow-up card: compute buy slippage exactly (e.g. return `Decimal::ZERO`
  when `eff == price`, mirroring the sell side's exact `base_in · (price − eff)`); demo hash unaffected
  (demo runs 20 bps slippage), sweep hash would change again (BeforeCosts slippage fields).
- **State left:** fmt + clippy green; full workspace test run has exactly ONE failure — the new
  `fee_sensitivity_is_reported_first_class_per_candidate` zero-cost assert. The report⇔verdict
  coupling assert (survives_doubled ⇔ edge-vanishes) PASSES unweakened. Determinism green: sweep
  `e94e10c0f25b40675cd9d3523b4d07c0f9caf39f` twice and `--threads 8` identical (report grew as
  expected); `sweep-verify` OK exit 0 (19456 bytes); demo unchanged `ae064f79…`; `git status` shows
  only the six carded files modified (no other schema touched).

## 2026-07-07 — HF track (Q7 follow-on): entry-condition check, plan audit, wave-1 cards (Fable)
- **Step 0 — entry conditions: NONE of the three met.** (a) M4 gate undeclared (queue was mid
  C8b–C8d); (b) highfrequency-algo-plan.md is DRAFT, untracked, unapproved; no master-plan
  amendment exists (pair verified byte-identical via `cmp` — invariant holds, amendment absent);
  (c) HF-Q1/Q2/Q3 absent from questions.md. Stopped and asked per instruction; **operator chose:
  audit now + draft wave 1 marked BLOCKED** rather than wait.
- **Step 1 — audit of the HF plan vs the tree** (Sonnet workflow: 7 readers + adversarial refuters;
  3 of 7 areas completed before a session limit killed the rest; market-data findings re-verified
  first-hand by the drafting session). Confirmed: the plan's "gap-scenario config from M1" does not
  exist (the only override is choosing `validate_series` over `validate_series_spacing` —
  validation.rs:114-115); "(venue, slot, seq)" is a brand-new key (nothing carries venue/slot/seq);
  §2.2's "simulator becomes latency = 1 bar" is a new abstraction, not an existing knob (delay is
  structural in simulator.rs:89-94). Verified: all four prior review folds present in the plan
  text; zero HF/intraday/latency code in crates/. Findings + the coverage gap (sweep-ladder,
  strategies-trait, invariants-config areas NOT audited — **re-run before wave-2 expansion**) are
  recorded in highfrequency-algo-plan.md's new Appendix.
- **Step 2 — wave-1 cards drafted BLOCKED.** task-queue.md gained §"M-HF wave 1" with the three
  entry conditions as hard preconditions, a per-card baseline-gate Step 0, the shared
  escalate-and-STOP list, and cards **M-HF-C1** (intraday types + hygiene: TradePrint/SlotSnapshot
  in research-core, validators + 6 new fixtures in market-data, 1s-bars-are-plain-Bars pinned by
  test) and **M-HF-C2** (deterministic synthetic microstructure generator: OU + impact-decay +
  regime congestion; SplitMix64 noise table seeded from the spec — no RNG, no new deps; output
  self-identifies `synthetic: true`). C3–C10 deliberately not drafted (each wave expands against
  what landed; adversarial checkpoint after C5). current-state.md pointer updated.
- **Card review (3 Sonnet agents): worktree execution rehearsal PASSED** — both cards executed
  verbatim by a fresh agent in an isolated worktree; every card gate green (research-core 40
  passed; market-data 36+7 unit / 7+6 integration; fmt+clippy clean; demo `ae064f79…` unchanged
  twice; no Cargo.toml/lock changes). Verbatim-fidelity + contract-fidelity reviewers: 8 findings
  (1 major — a stale "see worklog" pointer — plus minor/nits incl. all six fixture JSONs now pasted
  verbatim, C2 thread-count set restated, invariant-11 citation reworded); **all 8 applied** to the
  card text.
- **Repo bug found by the rehearsal (operator action needed):** `schemas/wallet-snapshot.schema.json`
  was never tracked — `.gitignore` line `wallet*.json` (D-0008 secret pattern) silently caught it,
  so a **clean checkout fails** `crates/results/tests/schema_validation.rs` (2 tests: missing
  file). This would fail CI from a clean clone and block M4-C9's evidence run on any fresh tree.
  Fix in working tree: `.gitignore` negation `!schemas/wallet-snapshot.schema.json` (wallet-file
  patterns untouched). **Operator: stage `.gitignore` + `schemas/wallet-snapshot.schema.json`.**
  (Unrelated to, and discovered independently of, the C8c 1-ulp escalation above.)
- HF track next actions, in order: operator reviews/approves highfrequency-algo-plan.md (read its
  Appendix first) → M4 queue finishes through C9 (incl. resolving the C8c escalation) → operator
  makes the lockstep amendment + records HF-Q1/Q2/Q3 in questions.md → flip M-HF-C1/C2 from
  BLOCKED and execute, one per fresh session → re-run the three unfinished audit areas before
  expanding wave 2.

## 2026-07-08 — Ruling on the C8c escalation: fix the accounting (card M4-C8e), never weaken the assert
- The C8c executor's stop was correct. Verified the root cause first-hand: cost.rs:77 buy-side
  `slippage_quote = quote_in - (quote_in * price / eff)` carries a ±1-ulp Decimal residue when
  `eff == price` (28-digit division), while the sell side (`base_in * (price - eff)`, cost.rs:94)
  is structurally exact. Pre-existing M2 reporting behavior, first made visible by C8c's
  first-class export — and fixture-dependent, which makes it worse, not better.
- Operator ruling: a zero-cost scenario reporting nonzero slippage in exported money values is a
  real defect (invariant 7). Amending the assertion would mask it. Inserted card **M4-C8e**:
  exact-zero guard in `fill_buy` (reporting-only field — balances/fees/gas untouched) + high-scale
  regression test, then re-gate C8c in the same session and flip both.
- Two C8c deviations blessed: (1) the executor's value-equality form of the zero asserts (Decimal
  zeros stringify with inherited scale — the card's literal `== "0"` was the imprecision, not the
  code); (2) C8e additionally amends C8c's DTO helpers to `.normalize().to_string()` (scale-canonical
  export strings, matching `param_id`'s convention — zeros export as "0").
- C9's preconditions now include C8e. M4 gate remains UNDECLARED. Sweep hash will change again at
  C8e/C8c re-gate (recorded there); demo hash must stay `ae064f79…`.

## 2026-07-08 — M4-C8e executed; C8c re-gated and flipped
- Applied the exact-zero guard to `fill_buy` (cost.rs:77) + high-scale regression test; `cargo test
  -p portfolio` → 33 passed, 0 failed. Amended C8c's DTO helpers in `report.rs` to
  `.normalize().to_string()`. Full-workspace gate (fmt+clippy+test) → 0 failed. Demo hash unchanged
  `ae064f79242f823ffd8f55bf9104e3e1b45d425a`; new sweep hash (sequential/threads-8/repeat, all
  identical) `7ad3df7de2e2c1139be427e9c953b57d4e289cb3`; `sweep-verify` → OK, exit 0. `git status`:
  only C8c's six files + `crates/portfolio/src/cost.rs` modified, no schema besides sweep-report.
  Both **M4-C8e** and **M4-C8c** flipped to DONE.

## 2026-07-09 — M4-C8d executed: two review minors closed
- Added `under_populated_schedule_is_rejected_as_insufficient_data_end_to_end` to
  `sweep_runner.rs` (clones `spec()`, sets `min_windows: 99`, asserts every verdict
  `Rejected`/`InsufficientData` only) — `cargo test -p sweep --test sweep_runner` → 7 passed.
  Extracted `parse_sweep_args` (pure) out of `sweep_cmd` in `crates/cli/src/main.rs`, same
  messages/exit(2) on Err; 6 new unit tests — `cargo test -p cli` → 15 passed. Full-workspace
  gate (fmt+clippy+test) → 0 failed. Demo hash unchanged `ae064f79242f823ffd8f55bf9104e3e1b45d425a`;
  sweep hash unchanged `7ad3df7de2e2c1139be427e9c953b57d4e289cb3` (refactor is behavior-preserving);
  `sweep-verify` → OK, exit 0. **M4-C8d** flipped to DONE.

## 2026-07-09 — M4-C9 pre-declaration re-review (addendum): CONFIRMED MAJOR — M4 gate NOT declared
- Before attempting C9, ran the mandated pre-declaration re-review: a 4-lens Sonnet workflow (report/
  schema fidelity; aggregation-math consistency; the cost.rs exact-zero-guard's reporting-only scope;
  master-plan conformance) over the C8b-C8e diff (`git diff 9368e95 01be9e9 --
  crates/sweep/src/{sensitivity,runner,report,lib}.rs crates/portfolio/src/cost.rs
  schemas/sweep-report.schema.json crates/sweep/tests/{schema_validation,sweep_runner}.rs
  DECISIONS.md`), every finding put through 2 independent skeptics prompted to refute with file:line
  evidence. First run hit the session token-limit mid-flight (4/6 agents errored); resumed the same
  run (cached agents replayed, only the failed calls re-ran) to a clean 8/8 completion, 0 errors.
- **report-schema-fidelity** and **aggregation-math-consistency** lenses: 0 findings (DTO<->schema
  field parity, `.normalize()` stringification, total-order sort, and the `aggregate_evidence` /
  `aggregate_fee_sensitivity` single-input consistency all checked out - both functions provably
  consume the identical `keys`/`results`/`doubled_floors` slices in the same order within one
  `run_sweep` call, so `survives_doubled` cannot drift from the edge-vanishes verdict).
- **cost-rs-exact-zero-guard** lens: 1 finding, **CONFIRMED minor** (2/2 skeptics) - the C8e regression
  test `zero_slippage_buy_reports_exactly_zero_slippage_even_at_high_scale` used `937.5/7 @ 103`, a
  pair that rounds back to an exact-zero residue even under the OLD unguarded formula, so it would not
  have caught a regression of the `eff == price` guard. **Fixed in this session** (test-only, 4 lines):
  swapped to `1000/3 @ 50`; hand-verified red (fails without the guard) -> green (passes with it) via a
  temporary local revert + restore. `cargo test -p portfolio --lib cost::` -> 9 passed; full-workspace
  gate (fmt+clippy+test) -> 301 passed, 0 failed; demo hash unchanged
  `ae064f79242f823ffd8f55bf9104e3e1b45d425a`; sweep hash unchanged
  `7ad3df7de2e2c1139be427e9c953b57d4e289cb3` (sequential/threads-8/repeat identical); `sweep-verify` ->
  OK, exit 0. `git status`: only `crates/portfolio/src/cost.rs` modified (test literals + comment).
- **master-plan-conformance** lens: 1 finding, **CONFIRMED major** (2/2 skeptics) - master-plan.md's M4
  deliverable "Strategy-family comparison" is elaborated in `plans/m4-sweep.md` §10 as a per-family
  "comparison row: best/median/worst across param neighbors, baseline deltas (cost-matched),
  walk-forward fold consistency, fee-sensitivity degradation." No such aggregation exists anywhere in
  the tree - `SweepReport` exports only a flat, label-sorted `candidates`/`verdicts` list (one row per
  param point, never grouped or reduced by family); `grep -rniE
  "median|FamilyComparison|ComparisonRow|per_family|by_family" crates/ schemas/` -> no hits. The
  M4-C9 evidence checklist below (the "Strategy-family comparison" row) maps the deliverable to "one
  report covering both families' candidates, scored via the shared `eval_strategy` core against the
  same 4 cost-matched baselines" - a materially weaker reading than §10, adopted with no DECISIONS.md
  entry recording the descoping (unlike D-0010, which did exactly that for the analogous
  fee-sensitivity gap). One illustrative detail in the finding was wrong (candidates do not literally
  interleave by family in a real run - they cluster, since "threshold_rebalance_v1" < "trend_alloc_v1"
  lexicographically) but both skeptics judged this a cosmetic slip, not a defect in the core claim, and
  did not refute on it.
- **Per the addendum's decision rule (confirmed major -> STOP, do not declare): the M4 gate is NOT
  declared this session.** M4-C9 stays `TODO` below; `plans/m4-sweep.md` and `plans/current-state.md`
  are untouched (S12 row, milestone status, gate-declared line all unchanged). The full step-1 gate
  battery was not run for declaration purposes (would be moot); the fmt/clippy/test/demo/sweep/
  sweep-verify runs above were solely to confirm the minor fix above didn't regress anything.
- **Operator decision needed before C9 can proceed:** either (a) implement a real per-family comparison
  artifact (a new aggregation type + schema block + tests - a genuine feature addition, not a
  same-session fix) and re-run this review, or (b) explicitly amend `plans/m4-sweep.md` §10 (descoping
  "Strategy-family comparison" to the flat candidate/verdict list, deferring true per-family rollup to
  M5's "Strategy-family ranking," master-plan.md:902) with a recorded DECISIONS.md entry, master-plan.md
  edited in lockstep with the root `solana-crypto-trader-plan.md` per Q6, then re-run C9 in a fresh
  session.

## 2026-07-09 — Ruling on the second C9 stop: descope with rigor (D-0011), C9 cleared for a third run
- The re-review's major is confirmed and fair: m4-sweep §10's per-family best/median/worst row has no
  artifact, and C9's evidence table had silently substituted a weaker reading — the exact move D-0010
  exists to prevent. Ruling = option (b), recorded not silent: the authoritative master plan puts
  "Strategy-family comparison" in M4 but **"Strategy-family ranking" in M5** (master-plan.md:900);
  what M4 ships (both families scored through one shared eval core against identical cost-matched
  baselines/windows, side-by-side in `candidates[]` + verdicts) IS a cross-family comparison, and the
  per-family aggregate row is M5 ranking input, trivially derivable from `candidates[]`. Recorded as
  **D-0011**; m4-sweep §10 amended; C9's evidence row now cites the descope explicitly. Master-plan
  pair untouched (no reword needed — its own M4/M5 split already says this).
- Blessed the review's minor fix already in the tree: the C8e regression literals (937.5/7 @ 103)
  were non-discriminating (rounded to exact zero even pre-fix); swapped to 1000/3 @ 50 and
  red→green verified. Gate re-confirmed by the operator side: 301 tests 0 failed; sweep `7ad3df7d…`;
  demo `ae064f79…` — both unchanged.
- C9 remains TODO and may now run fresh: no new full re-review required — the session must verify the
  two resolutions exist (D-0011 in DECISIONS.md; m4-sweep §10 amendment; the discriminating cost.rs
  test) and then execute the card's step-1 battery + evidence table as written.

## 2026-07-09 — M4-C9 executed: M4 gate declared
- Pre-checks verified fresh before running the card: `D-0011` is DECISIONS.md's top entry; m4-sweep
  §10 carries the `AMENDED 2026-07-09 (D-0011)` note; `cargo test -p portfolio zero_slippage_buy`
  passes (1 passed).
- Step-1 battery re-run fresh, all pass: `cargo fmt --all --check` exit 0; `cargo clippy --all-targets
  --all-features -- -D warnings` exit 0; `cargo test --workspace --all-features` → **301 passed, 0
  failed**, exit 0; `cargo run -p cli -- demo | shasum` ×2 → identical `ae064f79242f823ffd8f55bf9104e
  3e1b45d425a`; `cargo run -p cli -- sweep | shasum` ×2 + `--threads 8` → identical
  `7ad3df7de2e2c1139be427e9c953b57d4e289cb3`; `cargo run -p cli -- sweep-verify; echo $?` →
  `sweep-verify: OK — byte-identical across sequential, 2 and 8 threads, and repeat (18990 bytes)`,
  exit 0; no-execution-deps grep scan → `OK`.
- Evidence checklist walked against real artifacts (not just the card's prose): confirmed
  `sweep::param`, `sweep::parallel` (`thread::scope`), `determinism.rs`
  `parallel_equals_sequential_across_thread_counts` (K∈{1,2,3,7,8}) + `repeated_runs_are_byte_identical`,
  `SweepReport::to_json`, `schemas/sweep-report.schema.json`, `sweep::window` +
  `walk_forward.rs`, `report.rs` candidate `turnover`/`max_drawdown`/`fee_sensitivity` fields,
  `advance.rs` `RejectionKind` (7 variants), `holdout_sealing.rs` (`Cell<u32>` read-counter,
  `holdout_read_count() == 0` assertion) — all present and matching the card's claims; no row lacked
  its artifact.
- Declared M4 complete: `m4-sweep.md` S12 row → `✅ DONE (as cards M4-C1…C10)`, §1 gains
  `**GATE DECLARED 2026-07-09**`; `current-state.md` milestone section moves M4 DONE→gate declared,
  DOING→between milestones, Gates-run section refreshed to 301 tests / today's hashes; `task-queue.md`
  M4-C9 flipped to `DONE`, `M4·S1–S11` row notes appended with the declaration. No code, schema, or
  config touched — plan files only, per the card's guardrail. **M4-C10 (M5 handoff pointer) is
  deliberately left for its own fresh session, per the card.**
- 2026-07-09: M4-C10 done — the M4 queue is complete; `current-state.md` and `handoff.md` now point at the M5 operator decision and STOP (freeze questions.md Q5 walk-forward sizing + rejection thresholds; choose the Q3 real-data source; explicit go against master-plan.md's M5 gate), with the seed prompt replaced by a maintenance-only version and questions.md Q7's HF plan amendment noted as a pending operator-level task, not a card. Plan files only; full-workspace gate re-confirmed green (fmt clean, clippy clean, 301 tests 0 failed) with no code touched. **The M4 task-queue card sequence (M4-C1…C10) is now fully DONE.**
- 2026-07-09 (foreman, M5 GO + Q7 amendment): gates re-verified fresh at `bd0b3e2` before anything
  else (fmt/clippy clean; 301 tests 0 failed; demo `ae064f79…` ×2; sweep `7ad3df7d…` at 1+8
  threads; sweep-verify OK 18990 bytes). Operator decisions recorded VERBATIM in questions.md: Q3
  RESOLVED (Binance data.binance.vision daily SOLUSDC one-time snapshot; CEX-proxy caveat →
  provenance notes; shrink-never-patch hygiene rule; GeckoTerminal cross-check; Birdeye pencilled
  as HF-Q1), Q5 RESOLVED (rolling 365/90/90/5; 0.35/12/0.02/0.40/0.15/min_windows 6; holdout final
  ~20% by date; numbers final-frozen at span confirmation, one-way ratchet), M5 GO. task-queue.md
  gains §M5 with cards M5-C1…C6 (ingestion script → data-validate loader/hygiene → Q5 freeze
  sitting → --config/--data sweep wiring + provenance note → decisive sweep → decision card with
  the single call-once evaluate_on_holdout; reject-all reads the holdout ZERO times and routes to
  the HF track); signatures copied verbatim from source at `bd0b3e2`. Q7 amendment executed:
  3-design + 3-adversarial-lens + adjudicator workflow (7 Sonnet agents) → reuse-first design won
  (only design preserving the holdout-seal pattern at intraday granularity; adversary-first
  carried an unrefuted seal blocker and was disqualified); grafts + all 10 must-address findings
  folded into plans/m-hf-track.md (typed Provenance; C2.5 reuse-proof + C2.6 streaming/scale-proof
  gates; splitmix64 determinism proven at introduction with global fixture-relative event_index;
  fail-closed landing tables; base-rung adverse-selection pricing; depth_curve required for
  HF-kind specs; HF+turnover_budget spec lint; competitor_floor sensitivity sweep at C10; named
  targeting-decorrelation limitation; storage-format decision deferred to C2.6 on the record).
  master-plan.md §19 gains the M-HF track section, Arbitrage/XEMM/MEV disposition amended;
  solana-crypto-trader-plan.md updated in the same pass, `cmp` → byte-identical; M5 gate text and
  line numbers (903-909) unchanged. D-0012 recorded; HF-Q1/Q2/Q3 added to questions.md;
  current-state.md + handoff.md repointed at the M5 queue (STOP lifted; executor seed prompt
  restored). Plan/docs files only — no code, schema, config, or fixture touched.
- 2026-07-09 (foreman, review fixes on aaa2e28): a fresh-session review of the Phase-2 commit
  returned two findings, both applied. **F1**: M5-C6 (the M5 gate declaration — the highest-stakes
  card in the project) lacked its own mandatory pre-declaration adversarial review, unlike M4-C9
  which was correctly stopped twice by exactly such a review. Added Step 0 to M5-C6: a 4-lens
  Sonnet-subagent workflow (verdict-vs-report fidelity; frozen-threshold integrity — diffs
  m5-frozen.toml against the Q5 resolution; holdout-path call-site audit — zero calls for Branch A,
  exactly one for Branch B; declaration-vs-master-plan:903-909 conformance), 2 skeptics per finding,
  fixed decision rule (confirmed blocker/major → do not declare). **F2**: the M-HF wave-1 preamble
  in task-queue.md still cited `highfrequency-algo-plan.md` as design authority and listed the
  amendment as unmet. Corrected: design authority is now `m-hf-track.md` §5 (the adjudicated card
  sequence, which inserts C2.5/C2.6 before the renumbered C3 and changes C1's/C6's required
  surface); entry condition 2 marked satisfied (D-0012, 2026-07-09, cmp-verified). Also **verified
  by direct inspection, not inferred**: M-HF-C1/C2's card bodies are the pre-adjudication draft,
  unmodified since 2026-07-07 (`e821591`) — C2's `SyntheticIntraday` carries a plain `synthetic:
  bool`, not the typed `Provenance` enum `m-hf-track.md` §2/§5 requires, and C1 lacks the
  `IntraBar` alias §5 names as its required addition. A reconciliation note was added to the
  preamble: a planner session must patch C1's card text (fold in `IntraBar`, replace `synthetic:
  bool` with typed `Provenance`) before M-HF-C1 executes; everything else in C1/C2 (types, hygiene
  split, C2's `Congestion` enum + splitmix64 primitive) already lines up with the adjudicated
  design and needs no rework. current-state.md's HF blocker line updated to match. Plan files
  only; no code/schema/config touched.
- 2026-07-10 (executor, M5-C1): added `scripts/ingest-binance-solusdc-1d.sh` (new `scripts/` dir),
  the operator-run, one-time Binance SOLUSDC daily-kline snapshot script — no Rust touched. Gate
  verified: `bash -n` syntax-clean; `--dry-run 2021-01 2021-02` printed exactly the two expected
  monthly zip URLs and fetched nothing; `git check-ignore -q data/raw/` confirmed matching
  `.gitignore:43-48` verbatim, so the script's own ignore-guard is live; full-workspace gate green
  (fmt clean, clippy clean, 301 tests 0 failed, no code touched); `git status` showed only
  `scripts/`. M5-C1 flipped to DONE. Next: M5-C2 (`machina data-validate`), fresh session.
- 2026-07-10 (executor, M5-C2): added `crates/market-data/src/binance_csv.rs` (new — `load_dir`,
  `fnv1a64`, dependency-free CSV parsing) + `pub mod binance_csv;` in `lib.rs`, and a
  `machina data-validate --dir PATH [--interval-secs 86400]` subcommand in `crates/cli/src/main.rs`
  (sanctioned 3-file card). Verbatim signatures (dispatch block, `validate_series_spacing`) matched
  source exactly; no escalation needed. Gate verified: 7 new `binance_csv` tests + 1 CLI test green
  (309 total workspace tests, 0 failed, up from 301); `cargo fmt --all --check` and clippy
  (`-D warnings`) clean; `demo | shasum` unchanged `ae064f79242f823ffd8f55bf9104e3e1b45d425a`;
  `sweep | shasum` unchanged `7ad3df7de2e2c1139be427e9c953b57d4e289cb3`; `sweep-verify` OK, exit 0;
  `git status` shows no `Cargo.toml`/`Cargo.lock` change (no new deps — std + existing
  `rust_decimal` only). Manual smoke test on a temp dir confirmed the OK line format and the
  Gap-failure path (prints the first offending bar's timestamp, exit 1, never patches). M5-C2
  flipped to DONE. Next: M5-C3 (Q5 number-freeze sitting — operator + planner, not an executor
  card).
- 2026-07-11 (OPERATOR + planner sitting, M5-C3): Q5 number freeze. Operator ingested by hand
  (44 months fetched / 27 skipped-404; delisting gap 2022-10..2023-11); `data-validate` FAILED on
  the gap → operator ruled shrink-to-later-island (dropped 2021-09..2022-09; Q3: never patch);
  post-shrink `data-validate: OK — 916 bars, 2023-12-28..2026-06-30, spacing 86400s, fnv1a64
  0x65a1e18b7554a1c5`; combined CSV sha256 `1c7e70bc…8e25`. Partitions frozen (holdout
  2026-01-01..2026-06-30, 181 bars / 19.8%; validation 2025-01-01; dev/val 735 bars). Conflict
  ruled by operator: 365/90/90/5 yields only 4 windows < min_windows 6 → frozen **rolling
  365/60/60/5, min_windows 6** (6 windows exactly; OOS total unchanged 360d). Thresholds frozen
  per Q5; grids as-is (Q4). Artifacts: `config/strategies/m5-frozen.toml` (immutable this cycle),
  `plans/m5-data-validation.md` (hygiene record), questions.md Q5 freeze addendum. No strategy
  result existed on real data before the freeze. `.gitignore` gained a one-line negation
  (`!config/strategies/m5-frozen.toml`) — the blanket `config/**/*.toml` secrets guard would have
  swallowed the card's checked-in artifact; the frozen file holds no secrets. TOML syntax verified
  (python tomllib; full `SweepSpec::from_toml_str` parse is M5-C4's gate, per the card). M5-C3
  flipped to DONE. Next: M5-C4
  (`--config`/`--data` sweep wiring), fresh executor session.
- 2026-07-12 (executor, M5-C4): extended `parse_sweep_args` in `crates/cli/src/main.rs` with paired
  `--config PATH`/`--data DIR` (both-or-neither, usage error otherwise); added
  `sweep_report_json_real` (reads the TOML, `SweepSpec::from_toml_str`, `market_data::binance_csv
  ::load_dir`, `validate_series_spacing`, same hardcoded M4 cost model, same `run_sweep`, same
  `holdout_read_count() == 0` assert). Added additive `SweepReport::with_provenance(self, &str) ->
  Self` in `crates/sweep/src/report.rs` (consumes an already-built report, appends
  ` | data: <provenance>` to the note; `new` untouched). Gate: full workspace green (no test-count
  regression); `machina sweep | shasum` unchanged `7ad3df7de2e2c1139be427e9c953b57d4e289cb3` twice;
  `machina demo | shasum` unchanged `ae064f79242f823ffd8f55bf9104e3e1b45d425a`; `sweep-verify: OK`
  (18990 bytes); real-data run `machina sweep --config config/strategies/m5-frozen.toml --data
  data/raw/binance/SOLUSDC-1d` → `b2bbcc8dceadd6034759f0e102aec9882fc7d342` twice (byte-identical)
  and matching at `--threads 8`; provenance note carries `fnv1a64 0x65a1e18b7554a1c5`, matching the
  M5-C3 freeze record exactly; `trial_count` 180; no `Cargo.toml`/`Cargo.lock` changes. M5-C4
  flipped to DONE. Next: M5-C5 (the decisive sweep run — mechanical, evidence captured).
- 2026-07-12 (executor, M5-C5): preconditions confirmed — `data-validate --dir
  data/raw/binance/SOLUSDC-1d` reprinted `OK — 916 bars, 2023-12-28..2026-06-30, spacing 86400s,
  fnv1a64 0x65a1e18b7554a1c5`, matching `plans/m5-data-validation.md` exactly; full-workspace gate
  green (fmt/clippy/test, 313 tests, 0 failed) before running. Decisive run: `machina sweep
  --config config/strategies/m5-frozen.toml --data data/raw/binance/SOLUSDC-1d --out
  plans/m5-sweep-report.json`, sha256
  `424e713f330e763d0016cb4fcf92b41e4c8d02bd12ba24517d0eaf12ecc0fb40`; repeated to a scratch temp
  path (`cmp` byte-identical) and again with `--threads 8` to a second scratch temp path (`cmp`
  byte-identical) — three-run identity confirmed, parallel == sequential on real data. Report:
  `schema_version` 1.1.0, `trial_count` 180, thresholds match the M5-C3 freeze verbatim
  (drawdown_budget 0.35, turnover_budget 12, baseline_margin 0.02, dispersion_budget 0.40,
  neighbor_tolerance 0.15, min_windows 6). Verdicts: **10 rejected, 0 advanceable** (all 10
  candidates from `m5-frozen.toml`'s grids). `cargo test -p sweep schema` green (2 passed) —
  schema-validator path confirmed. No result interpreted here (M5-C6's call). `git status` shows
  only `plans/m5-sweep-report.json` new. M5-C5 flipped to DONE. Next: M5-C6 (operator + planner
  decision card — holdout read gated on an advanceable candidate; with 0 advanceable, the holdout
  is never read).
- 2026-07-12 (planner verify, M5-C4/C5): both cards re-verified fresh (313 tests 0 failed;
  no-flag sweep `7ad3df7d…` + demo `ae064f79…` unchanged; sweep-verify OK; real-data sweep ×3
  byte-identical incl. `--threads 8`; checked-in report byte-identical to a fresh `--out` run;
  schema suite 8/8 green; `data-validate` matches the freeze record exactly).
- 2026-07-12 (M5-C6 step 0): mandatory pre-declaration adversarial review — **PASS (6 agents)**.
  4 Sonnet lenses (verdict-vs-report fidelity, frozen-threshold integrity, holdout-path audit,
  declaration-vs-master-plan conformance) + 2 skeptics per finding. 1 raw finding (claimed
  malformed report sha256) refuted by both skeptics via fresh shasum (64 hex chars, exact match).
  Zero confirmed findings → proceed to declaration per the fixed decision rule.
- 2026-07-12 (OPERATOR + planner, M5-C6): **M5 GATE DECLARED — Branch A: ALL CANDIDATES REJECTED;
  the project returns to research (the HF track, Q7/D-0012).** Judged against master-plan.md:903-909
  verbatim ("One candidate is selected for mainnet shadow because it satisfies predefined
  robustness and drawdown criteria, **or all candidates are rejected and the project returns to
  research**. No execution work starts merely because the software exists."). Evidence:
  `plans/m5-sweep-report.json` (sha256 `424e713f330e763d0016cb4fcf92b41e4c8d02bd12ba24517d0eaf12ecc0fb40`,
  three-run byte-identical), 180 trials, 10 verdicts. Failed criteria per candidate
  (observed vs frozen threshold; full-precision decimals live in the report):
  every candidate failed `fails_baseline_comparison` (margin vs best cost-matched baseline,
  required ≥ +0.02): threshold_rebalance_v1 t=.25/b=.05 −0.0982 · t=.25/b=.1 −0.1006 ·
  t=.5/b=.05 −0.1144 · t=.5/b=.1 −0.1166 · t=.75/b=.05 −0.1341 · t=.75/b=.1 −0.1328;
  trend_alloc_v1 sma20/a=.5 −0.1197 · sma20/a=.75 −0.1382 · sma50/a=.5 −0.0620 ·
  sma50/a=.75 −0.0522. All 10 also failed `edge_vanishes_under_doubled_costs` (threshold
  0.0804780668820700082261097082). Both t=.75 rebalancers additionally failed
  `drawdown_exceeds_budget` (0.3797 / 0.3877 vs 0.35) and `depends_on_one_period`
  (0.5727 / 0.5637 vs 0.40). **Holdout read count: 0** — zero `evaluate_on_holdout` call sites in
  the tree (audited); the seal and the 2026-01-01..2026-06-30 holdout survive unseen for a future
  cycle. A clean reject-all is a success of the gates: neither daily-bar family earns mainnet
  shadow this cycle; no execution work starts. The goal stands as stated — genuine autonomous
  passive income earned through gates; this cycle's evidence says these two families on daily bars
  are not the strategy that earns it. M5-C6 DONE; queue §M5 CLOSED. Next: M-HF research track
  entry, gated on HF-Q1/HF-Q2 (questions.md) + the wave-1 card reconciliation (m-hf-track.md §5).
- 2026-07-12 (OPERATOR + planner, M-HF entry sitting): three rulings recorded in questions.md —
  (1) C1–C2.6 unblocked ahead of HF-Q1 by written operator note (synthetic-only; the safe-default
  provision); (2) **HF-Q2 RESOLVED: USDT + jitoSOL research-allowlist additions** (tri_arb_v1 and
  statarb_pairs_v1 both testable at C7+; jitoSOL over mSOL for liquidity; file edit lands with the
  wave-2 card that needs it); (3) HF-Q1 deferred to wave 2 (re-verify Birdeye at decision time;
  blocks C9/C10 only). Wave-1 reconciliation patch DONE (planner): C1 gains the `IntraBar = Bar`
  alias + typed `Provenance::{Synthetic, Real}` enum (+ a serde/alias test, re-exports); C2's
  `synthetic: bool` replaced by `provenance: Provenance` with a pure-integer FNV-1a `spec_hash`
  (no serde_json in src — it's a dev-dep); C1's verbatim block re-verified at `3365907` (M5-C2
  had added `binance_csv` to market-data's lib.rs — quote refreshed). §M-HF header ACTIVE; C1/C2
  flipped BLOCKED→TODO. Next: M-HF-C1, fresh executor session.
- 2026-07-13 (executor, M-HF-C1): **DONE.** Step 0 gate was green before touching any file (313
  tests, demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a`). All verbatim blocks (research-core
  lib.rs:12-25, bar.rs:12-21, time.rs:11-12; market-data lib.rs:10-17, validation.rs:84/113-115/116,
  fixture_validation.rs load pattern; both Cargo.tomls) matched source exactly — no adaptation
  needed. Added `crates/research-core/src/intraday.rs` (Side/Provenance/IntraBar=Bar alias/
  TradePrint/SlotSnapshot/IntradayItemError + 5 unit tests — the card's prose said "exactly these
  four tests" but enumerated 5 including `provenance_serde_round_trips_tagged`; implemented all 5
  as individually specified, noting the count mismatch here rather than dropping a named test) and
  its two-line lib.rs addition; `crates/market-data/src/intraday.rs` (IntradayError +
  validate_prints/validate_snapshots/validate_snapshots_contiguous + 8 unit tests) and its two-line
  lib.rs addition; six new fixtures under `crates/market-data/fixtures/`
  (prints_good/unsorted/duplicate/bad_size, snapshots_good/slot_gap); and
  `crates/market-data/tests/intraday_validation.rs` (6 integration tests). Gate: `cargo test
  -p research-core` 41/41 (incl. the 5 new); `cargo test -p market-data` unit 43/43 (incl. 8 new) +
  integration 6/6 new + 7/7 pre-existing unchanged; `cargo fmt --all --check` clean; `cargo clippy
  --all-targets --all-features -- -D warnings` clean; `cargo test --workspace --all-features`
  0 failed; `cargo run -p cli -- demo | shasum` unchanged (`ae064f79242f823ffd8f55bf9104e3e1b45d425a`,
  twice); `cargo run -p cli -- sweep-verify` OK (byte-identical across 1/2/8 threads + repeat).
  `git status` shows exactly the card's files: 2 modified lib.rs, 6 new fixtures, 2 new src files,
  1 new test file — no existing fixture or file touched. No new dependencies; no Cargo.toml edited;
  no execution code; `crates/sweep` untouched. Nothing staged/committed (operator commits). Next:
  M-HF-C2, fresh executor session.
- 2026-07-13 (executor, M-HF-C2): **DONE.** Step 0 gate was green before touching any file (332
  tests, 0 failed; demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a`; sweep shasum
  `7ad3df7de2e2c1139be427e9c953b57d4e289cb3`). C1's exports (Provenance/Side/SlotSnapshot/TradePrint,
  validate_prints/validate_snapshots/validate_snapshots_contiguous/IntradayError,
  validate_series_spacing) matched the card's "Current state" block exactly. Added
  `crates/market-data/src/synthetic.rs` (SyntheticSpec/Congestion/SyntheticError/SyntheticIntraday +
  `generate`/`spec_hash` (pure-integer FNV-1a, no serde_json)/`splitmix64`/`noise_table` + 7 unit
  tests) verbatim per the card, with one adaptation: the card's step-2 test prose still described
  the pre-reconciliation `out.synthetic: bool` field (`output_self_identifies_as_synthetic` checking
  `"synthetic":true`), which the 2026-07-12 reconciliation had already replaced with the typed
  `Provenance` enum in the card's own step-1 code block — rewrote that one test to assert
  `matches!(out.provenance, Provenance::Synthetic { .. })` and that the JSON contains
  `"kind":"synthetic"`, preserving the test's intent (self-identification) against the corrected
  struct; the other six tests typed in unmodified. Two-line `lib.rs` addition
  (`pub mod synthetic;` + the `pub use synthetic::{...}` line). Gate: `cargo test -p market-data
  synthetic` 7/7 new; `cargo test -p market-data` unit+integration all green (C1's tests
  unchanged); `cargo fmt --all --check` clean; `cargo clippy --all-targets --all-features
  -- -D warnings` clean; `cargo test --workspace --all-features` 339 passed, 0 failed;
  `cargo run -q -p cli -- demo | shasum` unchanged twice; `cargo run -q -p cli -- sweep | shasum`
  unchanged. `git status` shows exactly the card's files: 1 modified lib.rs, 1 new src file — no
  Cargo.toml/Cargo.lock changes, no existing fixture/schema/plan-pair file touched. Nothing
  staged/committed (operator commits). Next: C2.5 (reuse-proof), planner-drafted, not this session's
  scope.
- 2026-07-13 (planner verify + card-draft): M-HF-C2 re-verified fresh (339 passed 0 failed; fmt/
  clippy clean; demo/sweep hashes unchanged; no manifests; no rand; serde_json under cfg(test)
  only at synthetic.rs:275; provenance = typed Synthetic{spec_hash} at :264; the executor's one
  test adaptation was correct — the card's step-2 prose was pre-reconciliation, its step-1 code
  authoritative). **Card M-HF-C2.5 drafted** (queue §M-HF wave 1): reuse-proof, ONE new test file
  `crates/sweep/tests/hf_reuse_proof.rs`, ZERO src/manifest changes (sweep already deps on
  market-data; jsonschema already a dev-dep); 4,000 synthetic 1s bars, by_index(3000,3600)
  partition, rolling 900/300/300/5 ⇒ 8 windows, 8 candidate points; gates: 8 verdicts +
  determinism across {1,2,3,7,8} + schema-valid + holdout counter 0. Escalate-if pre-commits the
  bet-fails outcome (any needed src change ⇒ STOP, re-plan on record). Stale M5 table row in the
  queue tail fixed (ACTIVE → CLOSED). Next: M-HF-C2.5, fresh executor session; adversarial review
  follows it per m-hf-track §5.
- 2026-07-13 (executor, M-HF-C2.5): **DONE — reuse-first bet holds, zero `src/` changes.** Step 0
  gate was green before touching any file (339 passed, 0 failed; fmt/clippy clean; demo shasum
  `ae064f79242f823ffd8f55bf9104e3e1b45d425a`; template sweep shasum
  `7ad3df7de2e2c1139be427e9c953b57d4e289cb3`). Verified every signature the card cites
  (`run_sweep`, `SweepOutcome`, `SweepSpec`, `PartitionSpec::by_index`, `WalkForward::new`,
  `AdvancementThresholds`, `ParamGrid::{TrendAlloc,ThresholdRebalance}`,
  `market_data::synthetic::{generate,SyntheticSpec}`, `SweepReport::{to_json,to_value,trial_count,
  verdicts}`, `Sealed::holdout_read_count`) against source before writing anything — all matched
  verbatim. Added `crates/sweep/tests/hf_reuse_proof.rs` (new, test-only) with the card's 4 tests
  typed in as specified. Confirmed the card's own arithmetic against `WalkForward::windows`
  directly: rolling train=900/test=300/step=300/embargo=5 over the 3,600-bar dev+val series
  yields exactly 8 windows (`floor((3600-900-5-300)/300)+1 = 8`), 2 grids of 4 points each = 8
  candidate points ⇒ 8 verdicts, trial_count = 3 scenarios × 8 windows × 8 points = 192 ≥ the
  card's 64 floor — no test-expectation correction was needed, all four assertions passed as
  drafted on the first run. One incidental fix: removed an unused `Decimal` import flagged by the
  compiler (not present in the card's prose, a mechanical warning-to-error cleanup only). Gate:
  `cargo fmt --all --check` clean; `cargo clippy --all-targets --all-features -- -D warnings`
  clean; `cargo test -p sweep --test hf_reuse_proof` 4/4 new, all green; `cargo test --workspace
  --all-features` 343 passed, 0 failed (339 + 4 new); `cargo run -p cli -- demo | shasum`
  unchanged (`ae064f79…`); `cargo run -p cli -- sweep | shasum` unchanged (`7ad3df7d…`); `cargo
  run -p cli -- sweep-verify` OK (byte-identical sequential/2/8 threads + repeat). `git status`
  shows exactly `crates/sweep/tests/hf_reuse_proof.rs` (untracked) plus this worklog/queue edit —
  no `src/`, no `Cargo.toml`/`Cargo.lock`, no schema/fixture/plan-pair file touched. Holdout read
  count 0 throughout (asserted by a dedicated test). Nothing staged/committed (operator commits).
  Next: adversarial review checkpoint (m-hf-track §5) before C2.6 or C3 is drafted — this is the
  fill/cost/adversarial-terms checkpoint's predecessor, not itself the checkpoint.
- 2026-07-13 (planner verify + post-C2.5 adversarial review): C2.5 re-verified fresh (343 passed
  0 failed; fmt/clippy clean; demo/sweep hashes unchanged; commit a6a9e35's own stat proves
  test-file-only). Note: a6a9e35 was committed with C2's reused message — content correct, message
  amended by the operator on planner instruction (unpushed; main ahead of origin). **Review — the
  m-hf-track §5 post-C2.5 risk point — ran (6 Sonnet agents, 4 lenses + 2 skeptics/finding): the
  reuse-first bet HOLDS; 1 confirmed MINOR** — `verdicts.len()==8` derives from grid cardinality
  (runner.rs `points` from `spec.grids`), so a silently skipped family's cells wouldn't move it;
  skeptics measured the true cell count 192 (8 windows × 3 scenarios × 8 points). Per the fixed
  decision rule (minor + ≤10 lines of test code): the `trial_count >= 64` floor was replaced with
  exact `assert_eq!(trial_count, 192)` in hf_reuse_proof.rs — a family skip now fails the test
  (96 ≠ 192). Re-run: 4/4 green, workspace 343, 0 failed. The other 3 lenses returned zero
  findings (zero-src-change integrity; test quality; plan conformance incl. the §7 honest-limits
  caveat — no overclaim in the records). **Card M-HF-C2.6 drafted** (same session): columnar
  format (24-byte header + 48-byte exact scaled-integer rows), `ColumnarWriter`/`ColumnarFile` +
  `IntradaySource` trait in `market-data/src/columnar.rs`, std-only (memmap2/arrow rejected →
  D-0013); unit tests + an `#[ignore]`d 31.5M-row scale proof run once with wall-clock + max-RSS
  recorded; fixture in temp dir, never `data/`, never checked in. Next: M-HF-C2.6, fresh executor
  session; then C3 (latency pipeline) drafting.
- 2026-07-13 (correction): the prior entry says a6a9e35's reused commit message "was amended" —
  it was NOT: the operator committed the review fix (34e2ca2) before amending, burying a6a9e35.
  Ruling: leave it; content is correct, both this log and the message-content mismatch are now on
  the record, and a 2-deep rebase for a cosmetic message isn't worth the history rewrite.
  a6a9e35's actual content = the M-HF-C2.5 execution (hf_reuse_proof.rs + queue/worklog only).
  *(Superseded 2026-07-13: the operator chose to reword after all — non-interactive rebase;
  a6a9e35→a4e0523, 34e2ca2→a78ec42, a891ea1→c5281b7; all three tree hashes verified pairwise
  identical, working tree clean. History now reads correctly.)*
- 2026-07-13 (planner verify, M-HF-C2.6): all executor claims re-verified fresh — 349 passed, 0
  failed, 1 ignored; fmt/clippy clean; demo/sweep hashes unchanged; exact-or-error scaling and
  window-only slice confirmed by code read (seek + single bulk read); D-0013 present with measured
  numbers; scope exact; no temp-fixture leftovers; `data/` untouched. **Wave 1 (C1–C2.6) is
  COMPLETE.** Next: planner drafts C3 (run_hf/LatencyPipeline) against the landed surface.
- 2026-07-13 (executor, M-HF-C2.6): **DONE.** Step 0 gate was green before touching any file (343
  passed 0 failed; fmt/clippy clean; demo `ae064f79…`/sweep `7ad3df7d…` unchanged). Added
  `crates/market-data/src/columnar.rs`: fixed-record columnar format (24-byte header + 48-byte
  rows), `ColumnarWriter`/`ColumnarFile`, and the `IntradaySource` trait (`len`/`slice(Range) ->
  Vec<Bar>`), exact-or-error scaled-integer mantissas (never rounded on write), std::fs/io only;
  6 unit tests (round-trip, windowed slice, scale-overflow rejection, bad-magic/truncated-file
  rejection, out-of-bounds slice) plus one `#[ignore]`d scale proof; one `pub mod`/re-export line
  in `lib.rs`. Full-workspace gate green after: 349 passed, 0 failed, 1 ignored; fmt/clippy clean;
  demo/sweep shasums unchanged. Scale proof run once (release, `/usr/bin/time -l cargo test -p
  market-data --release scale_proof_full_year_1s -- --ignored --nocapture`): wrote 31,536,000 rows
  in 5.72s, file size 1,513,728,024 bytes (≈1.41 GiB, exact); read 1,000 windowed slices of 43,200
  rows in 0.84s; maximum resident set size 304,168,960 bytes (≈290 MiB) — about a fifth of the
  file, confirming the writer/reader never materialize the whole year. Recorded as D-0013
  (memmap2/arrow rejected, no new deps). Fixture lived under system temp dir, deleted at test end
  (verified no leftover file); `data/` untouched. `git status` scoped to exactly
  `crates/market-data/src/columnar.rs` (new), `crates/market-data/src/lib.rs`, `DECISIONS.md` +
  this queue/worklog pair. Next: planner drafts C3 (latency pipeline — new surface begins); M6 has
  no candidate and does not start.
- 2026-07-13 (planner, wave-2 audit re-run + card M-HF-C3 drafted): **audit re-run** (mandated by
  the queue's wave-discipline note + hf-plan Appendix:342-344), verified against `b0af75c` by
  direct read + 3 read-only Sonnet agents — **no blocking drift**:
  | # | Area | Verdict | Evidence |
  |---|------|---------|----------|
  | 1 | sweep-ladder | NO DRIFT | `sensitivity.rs:24-40,83,95` ScenarioId/scale_cost_model ("never f64")/cost_scenarios; `advance.rs:19-32,75-90,128` thresholds incl. `turnover_budget: Decimal`, 7 RejectionKinds incl. TurnoverImplausible; no timescale/thread/clock hardcodes in production paths; zero latency/intraday refs in `crates/sweep/src/` |
  | 2 | strategies-trait | NO DRIFT | `strategies/src/lib.rs:28-37` `target_weight(&[Bar], Decimal) -> Decimal`, pure/deterministic/clock-free (:24-27); warmups are bar counts; simulator consumes `FnMut(&[Bar], Decimal) -> Decimal` (`simulator.rs:65-72`, call :93-94) |
  | 3 | invariants-config | COMPATIBLE | `docs/invariants.md:22-26` (#3 determinism; splitmix64 = hash, not entropy) and `:35-38` (#5 next-bar). Notes: (a) #5's "executed at bar t+1's open" generalizes to `t+offset, offset ≥ 1` in `run_hf` — invariants.md wording amendment deferred to C8, on the record; (b) no explicit UTC line in invariants.md — pre-existing, unrelated |
  | 4 | §3 "same cell-identity primitive M4 §5.8 uses for run_id" | **WORDING DRIFT** | `run_id` is a tuple-formatted String (m4-sweep §5.8; `results/src/lib.rs:17`); no u64 hash primitive exists; `splitmix64` is private in `market-data/src/synthetic.rs:127-133`. C3 **defines** the primitive (verbatim 7-line twin, provenance comment); C8 wires `cell_id` derivation |
  **Card M-HF-C3 drafted** (queue §M-HF wave 1): `run_hf`/`LatencyPipeline` in
  `crates/portfolio/src/latency.rs` + `lib.rs` wiring + `tests/hf_regression.rs`;
  **`simulator.rs` READ-ONLY** (private helpers duplicated verbatim, pinned together by the
  regression); pinned semantics recorded on the card (signal-indexed landing table; `target_fn`
  called at landing time with signal-time history; un-landed = counted not priced — C4 prices it;
  off-end dropped per run's final-bar rule; `min_latency ≥ 1`). Gate: `run_hf(fixed_latency(1))
  == run` bar-for-bar (exact Decimal, 3 strategy shapes × 4 series × 2 cost models); landing
  table byte-identical across explicit threads {1,2,3,7,8}; M2 suite untouched; demo/sweep hashes
  unchanged. **Checkpoint reminder:** per m-hf-track §5, C3 is a named risk point (review after it
  executes) and the C3–C5 block ends in a mandatory adversarial review before C6+ is expanded.
- 2026-07-14 (planner verify + post-C3 adversarial review): C3 re-verified fresh (363 passed 0
  failed 1 ignored; fmt/clippy clean; demo/sweep hashes unchanged; sweep-verify OK; scope exact —
  simulator.rs untouched; no rand/Instant/SystemTime; executor's one deviation, `div_ceil` for
  clippy `manual_div_ceil`, arithmetic-identical + test-only, accepted). **Review — the m-hf-track
  §5 post-C3 risk point — ran (6 Sonnet agents, 4 lenses + 2 skeptics/finding): C3 STANDS; 1
  confirmed MINOR** — no test pinned the legal probability boundaries p=0/p=1 (all-un-landed /
  all-landed). Equivalence lens (argument-stream identity, helper-twin char-diff, non-vacuous
  regressions), semantics lens (all pinned decisions honored, no lookahead), and scope lens
  returned zero findings. Fix per the fixed rule (≤10 lines test code):
  `probability_boundaries_are_legal_and_exact` added in latency.rs. Re-run: workspace **364
  passed, 0 failed, 1 ignored**. Next: draft M-HF-C4 (HF cost-model fields); the C3–C5 block
  still ends in its mandatory block-level review before C6+.
- 2026-07-14 (planner draft + Foreman verify, card M-HF-C4): drafting session wrote the card;
  Foreman verified every verbatim block against source (cost.rs:24-33, money.rs helpers,
  lib.rs:13-24 post-C3, scale_cost_model pattern) and re-derived the reference-trade arithmetic
  by hand against the real exact `apply_bps`/`lamports_to_sol` (0.5+1+0.1+0.0015 = 1.6015 vs
  floor 0.05 — correct). Two recorded deviations from m-hf-track's letter, both logged on the
  card, intent preserved: (1) **HfCostModel wrapper** instead of adding fields to `CostModel` —
  46 existing `CostModel{..}` literals incl. simulator.rs's tests would break, violating the
  standing simulator.rs-untouched rule; reversible internal fork. (2) **tip_bps (bps of
  notional)** not `tip(lamports)` — a fixed-lamport tip can't scale with trade size; same unit
  convention as dex_fee_bps. The card's `den.max(1)` scaling clamp mirrors sweep's scale_u32/
  scale_i64 precedent exactly (sensitivity.rs:63-68). Card is TODO; next executor session.
- 2026-07-13 (executor, M-HF-C3): **DONE.** Step 0 gate green before touching any file (349
  passed, 0 failed, 1 ignored; demo `ae064f…`, sweep `7ad3df…` verified). Built per card:
  `crates/portfolio/src/latency.rs` (LatencyPipeline/LandingOutcome/HfError, splitmix64 +
  landing_draw + build_landing_table, verbatim simulator.rs helper twins, `run_hf` +
  10 unit tests), `lib.rs` wiring (module + re-exports + doc line),
  `tests/hf_regression.rs` (3 × `fixed_latency_one_equals_run_*` full-struct equality over
  4 shapes × 2 cost models, all green name-for-name; `landing_table_identical_across_thread_counts`
  with explicit {1,2,3,7,8}, green). simulator.rs untouched (`git diff --name-only`: only the 3
  card files + queue/worklog). Full gate: fmt --check clean; clippy -D warnings clean (one
  test-only `div_ceil` lint fixed in hf_regression.rs); workspace **363 passed, 0 failed,
  1 ignored**; demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a` and sweep shasum
  `7ad3df7de2e2c1139be427e9c953b57d4e289cb3` unchanged; sweep-verify OK (byte-identical across
  sequential/2/8 threads + repeat). No Cargo.toml/Cargo.lock change; no RNG/clock anywhere.
  Per m-hf-track §5, C3 is a named risk point — adversarial review before C4 is drafted.
- 2026-07-14 (executor, card M-HF-C4): **STOPPED — escalate-if triggered, card left `TODO`.**
  Step 0 gate green before touching any file (364 passed, 0 failed, 1 ignored — the corrected
  baseline; matches expected). Typed in `crates/portfolio/src/hf_cost.rs` (new) and
  `crates/portfolio/src/lib.rs` wiring verbatim from the card; `git diff --name-only` confirms
  only those 2 files (no `cost.rs`/`simulator.rs`/other-crate edits). `cargo test -p portfolio
  hf_cost` → **7/9 green**; the two named gate tests fail exactly as the card's own escalate-if
  anticipates: `hf_trade_cost_matches_hand_computed_reference_trade` and
  `total_cost_exceeds_base_fee_floor`, both on `base_fee_floor_quote`: code produces
  `dec!(0.0005)`, tests (copied verbatim from the card) assert `dec!(0.05)`.
  Recomputed by hand per the card's own instruction, twice, independently of the code: (a)
  `LAMPORTS_PER_SOL = 1_000_000_000`; `5_000 / 1_000_000_000 = 0.000005`, matching
  `money.rs`'s own doc example *and* its pre-existing, already-green
  `lamports_convert_exactly` test (`money.rs:63`, `lamports_to_sol(5_000) == dec!(0.000005)`);
  (b) `0.000005 * price(100) = 0.0005`. The card's hand-derivation
  (task-queue.md:4070-4071, and the drafting worklog entry above) drops a zero at the
  `lamports_to_sol(5_000)` step — writes `dec!(0.0005)` there instead of `dec!(0.000005)` — so
  its stated final answer `dec!(0.05)` is a transcription slip; correct value is `dec!(0.0005)`.
  Every other reference-trade figure on the card (`0.5`, `1`, `0.1`, `15_000`, `0.0015`,
  `1.6015`) checks out exactly against the code. Per the card's escalate-if list ("recompute by
  hand before concluding... do not adjust the assertion to match the code's output"): did not
  touch the assertions. Left `hf_cost.rs`/`lib.rs` exactly as typed from the card (2 failing
  tests included) for the next session to fix at the source (card text + the two `dec!(0.05)`
  literals in `hf_cost.rs`'s test module) rather than silently patched. No further steps run
  (no full-workspace gate, no fmt/clippy pass, no shasum re-check, card not flipped to `DONE`).
  Not committed/staged.
- 2026-07-15 (planner RULING on the M-HF-C4 escalation — FIX THE CAUSE): the executor is right;
  the card's derivation dropped a zero at `lamports_to_sol(5_000)` (the planner's own
  "hand-verification" repeated the slip — recorded as such). The assertion MECHANISM was never
  wrong and was not weakened: `total > floor` holds as `1.6015 > 0.0005`. Corrections applied at
  the source: card text (task-queue.md, two literals, with a correction note) and the two
  `dec!(0.05)` → `dec!(0.0005)` test literals in `hf_cost.rs`. Full gate re-run by the planner:
  **373 passed, 0 failed, 1 ignored** (+9 new incl. both named gate assertions); fmt/clippy
  clean; demo `ae064f79…`/sweep `7ad3df7d…` unchanged; sweep-verify OK; `git status` scope =
  hf_cost.rs (new) + lib.rs + queue/worklog only. The escalation protocol worked exactly as
  designed — the executor's hand-recompute caught what two planner passes missed. M-HF-C4
  flipped to DONE. Next: draft M-HF-C5 (adversarial terms); the C3–C5 block review follows C5.
- 2026-07-15 (planner, DRAFT M-HF-C5): card **M-HF-C5 — adversarial execution terms (sandwich,
  pickoff, maker trade-through, base-rung expected adverse-selection)** drafted and appended after
  M-HF-C4 in task-queue.md; status TODO. Verbatim source re-verified at `a7b3235` (clean tree):
  lib.rs:18-36 (post-C4 wiring), money.rs:37-39 `apply_bps` (exactness proven by the green
  money.rs:69 test), cost.rs:14-20 `Side`. **Numbers verified empirically, NOT by hand** (the C4
  lesson): wrote a throwaway `crates/portfolio/tests/zzz_c5_number_check.rs` exercising the real
  `apply_bps`/rust_decimal, ran it green, deleted it, confirmed `git status` clean — τ@1000=4,
  τ@2000=8, expected p=5/100→0.2, p=1/2→2, p=1/1→4, p=0/1→0, plus the min() size-cap and the
  strict-`<`/`>` trade-through boundaries. Files: `adversarial.rs` (NEW) + `lib.rs` — 2 files,
  mirrors C4. **Pinned decisions (logged on the card):** (1) **one `adversarial.rs`, not
  `maker_fill.rs`** — DRIFT from m-hf-track §3's `maker_fill.rs` naming, recorded not papered over:
  the maker predicate is one ~15-line pure fn sharing the module's single "costs to us only" theme;
  a dedicated file is premature fragmentation; reversible internal file-layout fork. (2) **C5 stays
  PURE like C4** — pure pricing/fill fns, NOT wired into run_hf; per-fill hash realization (reusing
  C3's `landing_draw`) + the fail-closed `(regime,percentile)→p` table are C8's job; ladder rung
  wiring (`AdversarialWorst`) is C6's — exactly as C4 deferred regime-derivation + the
  landing-percentile table. (3) **pure-parameter route** (m-hf-track §3's "or pure parameters"): no
  new hash primitive introduced, so nothing new to prove for determinism beyond Decimal purity;
  latency.rs untouched (no re-duplication / `pub(crate)` of `landing_draw`). The base-rung `p·τ`
  closed form is exactly what C8's hashed per-fill realization averages to. (4) **no scale helper**
  (unlike C4's `scale_hf_cost_model`) — adversarial rungs are a mode switch (expected↔worst, `p`
  forced to 1), not a numeric ×n scale; C6 selects by calling `_worst` vs `_expected`. Semantics:
  τ = sandwich_bps + pickoff_bps, both on EVERY fill (pessimistic, never a benefit); worst rung =
  τ (p≡1); base rung = `p·τ` with `p` an exact rational (fail-closed like C3's build_landing_table);
  maker fill = trade-through-only (strict `<`/`>`; touch = no fill), size-capped by printed volume.
  Gate: 382 expected (373 + 9 unit tests), both named gate properties asserted, demo/sweep shasums
  unchanged, 2-file scope. No code implemented (planner drafts, executor implements). Card is TODO;
  next an executor session. After C5 execution: the mandatory C3–C5 block adversarial review before
  C6+ is drafted.
- 2026-07-15 (executor, card M-HF-C5): Step 0 gate green before touching any file (373 passed,
  0 failed, 1 ignored — matches expected). Typed in `crates/portfolio/src/adversarial.rs` (new:
  `AdversarialModel`, `AdversarialError`, `adverse_selection_cost_worst`/`_expected`, `MakerOrder`/
  `MakerFill`/`maker_trade_through_fill`, 9 unit tests) and `crates/portfolio/src/lib.rs` wiring
  (module + re-export block + doc bullet) verbatim from the card; `git status --short` confirms
  only those 2 files touched (`M lib.rs`, `?? adversarial.rs`) — no `cost.rs`/`simulator.rs`/
  `latency.rs`/`hf_cost.rs`/`state.rs`/other-crate edits, no Cargo.toml/Cargo.lock change.
  `cargo test -p portfolio adversarial` → **9/9 green**, including both named gate properties
  (`adverse_selection_worst_reproduces_full_tau_on_every_fill`,
  `maker_bid_fills_only_on_trade_through`). Hand-recomputed every reference number independently
  before trusting the run (the C4 lesson): τ@1000 = 1000·30/10000 + 1000·10/10000 = 3+1 = 4;
  τ@2000 = 6+2 = 8; expected@5/100 = 4·5/100 = 0.2; boundary 0/1→0, 1/2→2, 1/1→4 — all matched the
  card and the code with no discrepancy, no assertion adjusted. Full gate: `cargo fmt --all --check`
  clean; `cargo clippy --all-targets --all-features -- -D warnings` clean; `cargo test --workspace
  --all-features` → **382 passed, 0 failed, 1 ignored** (373 + 9, exact match); demo shasum
  `ae064f79242f823ffd8f55bf9104e3e1b45d425a` and sweep shasum
  `7ad3df7de2e2c1139be427e9c953b57d4e289cb3` unchanged; `sweep-verify` → OK (byte-identical across
  sequential/2/8 threads + repeat, 18990 bytes). No RNG/clock anywhere; no `maker_fill.rs`, no
  scale helper, no ladder-rung/`run_hf` wiring — none of the escalate-if triggers fired. Card
  flipped to `DONE`. Not staged/committed (operator commits). Next: the mandatory C3–C5 block
  adversarial review, before any C6+ card is drafted.
- 2026-07-15 (verifier/planner): **M-HF-C5 independently verified — PASS.** Re-ran the full gate from
  scratch (not trusting the executor report): `fmt`/`clippy` clean; `cargo test --workspace
  --all-features` → 382 passed / 0 failed / 1 ignored (exact card match); `portfolio adversarial`
  9/9; demo shasum `ae064f79…` + sweep shasum `7ad3df7d…` unchanged; `sweep-verify` OK; grep
  confirms no RNG/clock/float in `portfolio`; `git status` scope is exactly `adversarial.rs`(new)
  + `lib.rs`(wiring) + queue/worklog flip. Code matches the card verbatim; every reference number
  hand-recomputed (τ@1000=4, τ@2000=8, p·τ=0.2, boundaries 0/2/4, maker strict `<`/`>`, volume cap)
  — all correct. Read C3(latency.rs)/C4(hf_cost.rs) end-to-end too; the `run_hf(fixed_latency(1))
  == run` regression (hf_regression.rs) genuinely pins the duplicated accounting helpers (3 strats ×
  4 shapes × 2 cost models, exact Decimal equality) + landing-table byte-identity across threads
  {1,2,3,7,8}.
- 2026-07-15 (planner): **Mandatory C3–C5 adversarial checkpoint (m-hf-track §5) — DONE:
  PASS-with-hardening.** Ran a fan-out review workflow (5 lenses over latency/hf_cost/adversarial;
  each finding refuted-or-confirmed by 2 diverse Sonnet skeptics). *Process note (honest):* the
  first workflow run reported "0 findings" due to a script bug (inner `parallel([...])` got eager
  `agent()` promises, not `() =>` thunks) that crashed the verify stage and dropped all findings;
  recovered the 5 review agents' 11 raw findings from journal.jsonl, fixed the thunk bug, and
  re-ran verify via resume (review cached). Result: **determinism lens = 0, contract-drift lens = 0**
  (the two load-bearing lenses clean — matches my own read); 11 raw findings → **5 survived skeptics
  → 2 real, distinct defects** after dedup (2 were the same depth-curve issue from two lenses; 2
  split-vote survivors — `landing-draw-event-index` and `run-hf-end-to-end-determinism` — are on
  direct trace already covered by hf_regression.rs's non-degeneracy assert + compositional
  determinism, so downgraded to nice-to-have tests). The 6 killed were spec-sanctioned or
  known-limits: maker fill = full printed volume is the **spec's own wording** (§3 "size-capped by
  printed volume"); depth-tail flatline is documented + a C8 spec-lint item; `scale_hf_cost_model`
  truncation mirrors the accepted `scale_cost_model` pattern and is unwired. **The 2 real defects**
  (both cost-understating, both UNREACHABLE in any wired path today — nothing constructs these types
  from external input until C6/C8; both mirror the `DepthCurve::new` guard C4 already ships):
  (a) `DepthCurve::new` accepts non-monotonic `impact_bps`; (b) `CongestionPriorityTable`/`HfCostModel`
  accept negative lamports → negative gas → fabricated benefit (breaks the module's own "never a
  benefit" / total>floor invariant). Verdict: the block PASSES the checkpoint in its load-bearing
  dimensions (determinism + contract-fidelity clean; wired-path pessimism holds), with a small
  fail-closed hardening owed **before C6 wiring**. Drafted card **M-HF-C5.1** (contained to
  `hf_cost.rs`+tests; `HfCostError` has no external matcher → new variants ripple nowhere).
  Next: execute C5.1, then draft C6.
- 2026-07-15 (executor): **M-HF-C5.1 executed → DONE.** Both checkpoint gaps closed at the type
  boundary in `crates/portfolio/src/hf_cost.rs` only: `DepthCurve::new` now rejects a strictly-
  decreasing `impact_bps` (equal/flat allowed) → `HfCostError::NonMonotonicImpact`;
  `CongestionPriorityTable`'s three lamport fields made private behind `new(calm,busy,hot) ->
  Result` rejecting any `< 0` → `NegativePriorityLamports { regime, lamports }`; `scale_hf_cost_model`
  + the two test constructions rerouted through `new()`/`priority_lamports_for`. 3 new tests
  (`depth_curve_rejects_non_monotonic_impact`, `priority_table_rejects_negative_lamports`,
  `hf_cost_cannot_go_negative_via_priority_table`). Gate: `fmt`/`clippy` clean; `cargo test
  --workspace --all-features` → **385 passed / 0 failed / 1 ignored** (382 + 3); the 3 priced-value
  asserts (`…reference_trade` / `…exceeds_base_fee_floor` / `…scales_hf_fields_exactly`) unchanged;
  demo shasum `ae064f79…` + sweep shasum `7ad3df7d…` unchanged; `sweep-verify` OK. `git status`
  scope = exactly `hf_cost.rs` + this queue/worklog flip; `lib.rs`/`cost.rs`/`Cargo.toml` untouched
  (grep confirmed the type is only re-exported by name, no external construction/field-read). Not
  staged/committed (operator commits). Next: planner drafts C6 (do NOT start C6).
- 2026-07-15 (verifier/planner): **M-HF-C5.1 independently verified — PASS.** Re-ran full gate: fmt/
  clippy clean; 385 passed/0 failed/1 ignored (382+3); hf_cost 9→12 tests; demo `ae064f79…` + sweep
  `7ad3df7d…` shasums unchanged; sweep-verify OK. Read the full `hf_cost.rs` diff: `DepthCurve::new`
  adds `windows(2).any(|w| w[0].impact_bps > w[1].impact_bps)` → `NonMonotonicImpact` (flat legal);
  `CongestionPriorityTable` fields now private behind `new() -> Result` rejecting `<0` →
  `NegativePriorityLamports{regime,lamports}`; `scale_hf_cost_model`+2 test sites rerouted via
  `new()`/`priority_lamports_for`; `hf_trade_cost` still pure/infallible. No priced-value assert
  moved; scope exactly `hf_cost.rs`+queue/worklog; `lib.rs`/`cost.rs`/`Cargo.toml` untouched. C3–C5
  checkpoint fully closed.
- 2026-07-15 (planner): **M-HF-C6 DRAFTED (narrow scope, operator decision).** Before drafting, ran a
  5-agent surface-map workflow over sweep/schema/cli (grep-verified) and found **§4/§5's C6 sketch
  drifted from the landed code**: (1) the "RejectionKind consumer-match audit (results, cli)" is
  vacuous — `RejectionKind` is referenced in NEITHER `crates/results` NOR `crates/cli`, the CLI dumps
  opaque JSON, and no exhaustive `match RejectionKind` exists anywhere; (2) the "HF-kind spec-lint"
  has no discriminator — `SweepSpec` has no HF/standard `kind` field (born in C8); (3) new HF
  `ScenarioId` rungs don't reach the report (`FeeSensitivity`/schema hard-locked to 3 named slots —
  even today's DoubledSlippage/DoubledPriority never surface). **Operator chose NARROW C6** =
  turnover-criterion replacement ONLY; all ladder/scenario/`data_provenance`/spec-lint moved to C8.
  Card spec: `AdvancementThresholds.turnover_budget: Decimal`→`Option` (TOML stays required, mapping
  Some-wraps — config TOMLs untouched); +`cost_drag_share_ceiling`/`per_trade_edge_floor` Option
  thresholds + `cost_drag_share`/`per_trade_edge` Option evidence (C8 computes, `None` in M4);
  +`RejectionKind::{CostDragExcessive,PerTradeEdgeInsufficient}` appended + a new exhaustive
  `RejectionKind::label()` as the real compiler-forcing function replacing the vacuous audit; schema
  additive minor bump 1.1.0→1.2.0 (append 2 kind strings, turnover_budget out of `required`, 2 new
  optional props); `skip_serializing_if=none` so an M4 report is byte-identical except the version
  line. **Primary gate = byte-identical-to-M4 regression; the `sweep` shasum MOVES by exactly the one
  `schema_version` line** (first HF card to change sweep output — the card requires diffing to confirm
  version-only, then records the new shasum). 7 files (advance/spec/report + 3 tests + the
  sweep-report schema — the "never touch schemas" guardrail explicitly lifted for that one file).
  **OWED (separate small planner edit, flagged for a future session): correct m-hf-track §5's C6 row
  + §4/§5 gate language, which cite the non-existent consumer-audit/spec-lint surfaces.** Next:
  execute C6 (fresh session).
- 2026-07-15 (executor): **M-HF-C6 executed → DONE.** Step 0 baseline matched exactly (385/0/1,
  fmt/clippy clean, demo `ae064f79…`, sweep `7ad3df7d…`) before any edit. Implemented the card
  verbatim: `AdvancementThresholds.turnover_budget`→`Option<Decimal>` (+`cost_drag_share_ceiling`,
  `per_trade_edge_floor`); `CandidateEvidence` +`cost_drag_share`/`per_trade_edge` (both `Option`);
  `RejectionKind` +`CostDragExcessive`/`PerTradeEdgeInsufficient` appended last + new exhaustive
  `RejectionKind::label()`; `evaluate_candidate` turnover check `if let Some`-gated, two new checks
  appended after it in canonical order, each gated on `if let (Some, Some)` so an M4 candidate
  (both `None`) takes neither branch; `spec.rs` Some-wraps the TOML turnover (TOML/`AdvancementToml`
  untouched); `report.rs` `ThresholdsDto` fields → `Option<String>` + `skip_serializing_if`,
  `SWEEP_SCHEMA_VERSION` 1.1.0→1.2.0; schema additive (2 new `kind` enum strings, `turnover_budget`
  moved out of `required`, 2 new optional threshold props). 6 new tests (4 in `advance.rs`: inactive-
  when-`None`, cost-drag fires+gated, per-trade-edge fires+gated, `label()`≡serde-form≡schema-enum
  via `include_str!`; 2 in `schema_validation.rs`: M4-shape and HF-shape both validate under 1.2.0).
  **Scope note (escalated to the operator mid-session, not improvised):** the card's 7-file list and
  its "six construction-sites" audit tracked only `AdvancementThresholds` sites and missed that
  `crates/sweep/src/runner.rs:126` (`aggregate_evidence`, called directly by `run_sweep` — real
  production code, not a test) also constructs `CandidateEvidence` and would not compile once the
  struct gained two new required fields. Flagged this as a card gap hitting the card's own "edit
  outside the 7 listed files → STOP" rule; the operator chose to extend scope by the same mechanical
  `cost_drag_share: None, per_trade_edge: None` addition and continue rather than block on a planner
  round-trip for a 2-line additive fix. **card gap, flag for planner:** M-HF-C6's file list should
  have been 8 files (add `runner.rs`); note this if similar cards are drafted for C8.
  Gate: `cargo fmt --all --check` clean; `cargo clippy --all-targets --all-features -- -D warnings`
  clean; `cargo test --workspace --all-features` → **391 passed / 0 failed / 1 ignored** (385 + 6);
  every PRE-C6 assertion verified unchanged by diff review (only mechanical `Some(dec!(5))`/
  `.unwrap()` wrapping, no observed/threshold/order value touched). Sweep report diff before→after
  is the single `"schema_version": "1.1.0"` → `"1.2.0"` line, confirmed by `diff`; **new sweep shasum
  `85d06e5be4b1a2ac09713a30b56ba794624dc260`** (replaces `7ad3df7d…`) — recorded here per the card.
  Empirically validated (not just reasoned): the real captured pre-C6 1.1.0 report parses and passes
  the new 1.2.0 schema with zero errors (throwaway integration test, deleted before finishing —
  never landed in the diff). Demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a` **unchanged**.
  `cargo run -p cli -- sweep-verify` → OK, byte-identical across sequential/2/8 threads and repeat at
  1.2.0. `git status` scope = exactly `advance.rs`, `spec.rs`, `report.rs`, `runner.rs` (the approved
  8th file), the 3 test files, `schemas/sweep-report.schema.json`, + this queue/worklog flip —
  nothing else touched (`AdvancementToml`, config TOMLs, `run-result.schema.json`, `crates/results`,
  `crates/cli`, master-plan pair all untouched, confirmed by `git status`). Not staged/committed
  (operator commits). **Still OWED (unchanged from the drafting entry above): a planner session must
  correct m-hf-track §5's C6 row/gate language** (the non-existent consumer-audit/spec-lint surfaces)
  — separately, that planner pass should also add `runner.rs` to any future card's file-list template
  reasoning so the "construction sites" grep covers every type touched, not just the one most obviously
  implicated. `plans/current-state.md`'s "C6 DRAFTED (TODO)" pointer (line 5) is now stale too —
  intentionally left untouched here (this card's own gate scopes `git status` to the code files + the
  queue/worklog flip only); a planner session should refresh it alongside the m-hf-track fix. Next:
  C7/C8 drafting is a planner job, not this session's.
- 2026-07-15 (verifier/planner): **M-HF-C6 independently verified — PASS**, and the owed planner
  corrections done. Committed at `ac2b040` (operator committed the executor's 10-file diff + my
  C6-draft/C5.1-verify at `e94e229`). Re-ran full gate from clean HEAD: fmt/clippy clean; **391
  passed/0 failed/1 ignored** (385+6); demo shasum `ae064f79…` unchanged; **sweep shasum now
  `85d06e5be4b1a2ac09713a30b56ba794624dc260`** (was `7ad3df7d…`); sweep-verify OK. Diff review:
  `advance.rs` — `turnover_budget` Option-gated (`if let Some`), the two new criteria `if let
  (Some,Some)`-gated and appended AFTER turnover (`>` cost-drag, `<` per-trade-edge), new
  `RejectionKind`s appended LAST (existing 7 keep their `Ord`/discriminants), exhaustive `label()`
  added; the M4-byte-identical property is structural (M4 thresholds `Some`+`None,None` take the same
  branches). `report.rs` — `skip_serializing_if="Option::is_none"` on all three HF DTO fields, so an
  M4 report omits them ⇒ the sweep-output diff is exactly `"schema_version":"1.1.0"→"1.2.0"`.
  `runner.rs` — exactly `cost_drag_share: None, per_trade_edge: None` (the missed site). Schema — clean
  additive superset (`turnover_budget` dropped from `required` but kept as a property; 2 optional
  props + 2 enum strings appended). No pre-existing assertion changed (only mechanical `Some`/`None`
  wrapping). **Owned card-audit miss:** the C6 card enumerated `AdvancementThresholds { }` sites but
  not `CandidateEvidence { }` sites, so `runner.rs`'s `aggregate_evidence` was missing from the file
  list; the executor's escalate-if caught it, operator approved the mechanical extension. Lesson
  recorded (memory `planning-enumerate-construction-sites`): a card mutating a struct must grep EVERY
  construction site of EVERY struct it touches. **Owed corrections DONE this session:** m-hf-track
  §4/§5 C6 language corrected (vacuous consumer-audit + spec-lint + ladder/provenance all marked moved
  to C8); current-state repointed to C6 DONE + the new sweep shasum. Next executor card is **C7**
  (`statarb_pairs_v1` + `intraday_meanrev_v1`, intent-only) — but its §5-drafted text predates the
  landed code, so a planner must reconcile it (grep its construction sites, per the lesson) before it
  executes; C8 now carries all the deferred C6 ladder/scenario/spec-lint/provenance work.
