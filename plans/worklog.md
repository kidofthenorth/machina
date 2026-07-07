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
