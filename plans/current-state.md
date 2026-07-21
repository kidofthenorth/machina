# Current state

Optimized for fast agent parsing. Source of truth for "where are we." Updated 2026-07-18 (M-HF
wave 2: C1–C7 and **C8.1–C8.5 + CARD-HYG-1 all DONE + planner-verified**. C8.1–C8.4 committed at
`3757d2a`/`ddd4512` (ScenarioId HF rungs; `data_provenance` — sweep schema **1.3.0**, sweep shasum
`94e90c3c…`; `ParamGrid::IntradayMeanRev`; `sweep::intraday_partition` holdout seal; gnhf e2e
port). C8.5 DONE 2026-07-18 + planner re-verified same day (`sweep::hf_spec` — HF-kind TOML
parse + fail-closed spec-lint, parse-only; new `hf-strategy-lab.example.toml` template, NOT
tuned; staged for operator commit). **C8.6 planner reconciliation pass DONE 2026-07-18** (plan
files only, no code, HEAD `3af2dbb`): C8.6 split by real crate-boundary dependency into
**C8.6a** (`sweep::congestion` — non-lookahead `CongestionRegime` classifier, small, 2 files) and
**C8.6b** (`portfolio::hf_priced::run_hf_priced` — wires `HfCostModel`+`AdversarialModel` into a
new priced execution entry via a per-trade synthesized `CostModel`, reusing `apply_buy`/
`apply_sell` unchanged, 4 files) — both are now **EXECUTOR-READY** and order-independent (C8.6b
only needs the already-`pub` `CongestionRegime` type, not C8.6a's classifier function). A real
narrowing was logged: the original sketch's regime-conditioned "(regime, percentile) → p" table is
scoped OUT (`AdversarialModel` as landed by C5 has no such axis — building one now would be new
type design, not wiring). Gate re-run at HEAD `3af2dbb`: **441 passed / 0 failed / 1 ignored**;
demo `ae064f79…` and sweep `94e90c3c…` UNCHANGED; sweep-verify OK. **Fresh-session review of C8.5
(sol5.6, 2026-07-18): accept-with-one-fix** — one MEDIUM (`resolution_secs` parsed unvalidated;
a card gap, not executor drift), carded as **C8.5.1** (parse-time lint + 3 tests, task-queue.md).
**C8.5.1 DONE 2026-07-19** (landed `7beef71`; planner re-verified: gate 444/0/1, shasums
unchanged — parse-only held). **C8.6a DONE 2026-07-19** (`sweep::congestion::
classify_congestion_regimes` — pure non-lookahead classifier, two-direction mutation proof,
pessimistic Hot default; planner re-verified: gate **450 / 0 / 1** (444 + 6), demo + sweep
shasums unchanged, re-export is the only caller; staged, awaiting operator commit — note the
operator's first C8.6a commit was mislabeled with the C8.5.1-pointer message and was soft-reset
on request 2026-07-19, work preserved staged). **C8.6b DONE 2026-07-20** (landed `bb88831`;
planner re-verified: gate **457 / 0 / 1** (450 + 7), demo + sweep shasums unchanged, C3/C7
regression suites byte-untouched — `run_hf` provably unmoved; `hf_priced.rs` clean on the
forbidden-pattern scan: `slippage_bps` always 0, adverse-selection reference fns doc-only, f64
only in a non-money test fraction. One confirmed-minor finding: an unneeded 8th
`latency::rebalance` `pub(crate)` widening beyond the card's pinned 7, zero callers, recorded
as **CARD-HYG-3** per FOREMAN §7 — now FOLDED INTO C8.7a). **C8.7 planner reconciliation pass
DONE 2026-07-20** (plan files only, HEAD `44d483c`, gate re-run this pass: **457/0/1**, demo
`ae064f79…` ×2, sweep `94e90c3c…`, sweep-verify OK): C8.7 split into seven signature-pinned
executor cards **C8.7a–C8.7g** (task-queue.md) with pinned decisions D-a…D-h and six drift
corrections on the record — headline pins: the report EXTENDS `SweepReport` (schema 1.3.0→1.4.0
at C8.7d, where the sweep shasum MOVES once, version-only diff, expected); `HfSweepSpec` gains
`[latency]`/`[adversarial]`/`[congestion]` blocks (C8.7b — the spec can't feed `run_hf_priced`
today); the §3 (regime,percentile) landing table is DEFERRED to C9+ (flat wave-1 params);
landing tables use global dev-val event indices with window-excluded cell ids. **C8.7a DONE
2026-07-20** (committed `8d187ef`; planner re-verified: gate **464 / 0 / 1** (457 + 7), demo +
sweep shasums unchanged, `hf_cost_scenarios` has zero callers outside its tests + lib.rs
re-export — unwired as required; CARD-HYG-3 rode along and is CLOSED, `latency.rs:202` back to
private with zero external callers). **C8.7b DONE 2026-07-20** (staged, awaiting operator
commit; gate re-run: **470 / 0 / 1** (464 + 6), demo `ae064f79…` ×2 and sweep `94e90c3c…`
unchanged — parse-only held; `HfSweepSpec` gains `pub latency: hf_scenarios::HfLatencyParams`,
`pub adversarial: portfolio::AdversarialModel`, `pub congestion_lookback: usize` resolved from
new `[latency]`/`[adversarial]`/`[congestion]` TOML blocks with 4 fail-closed lints
(`ZeroLatencyOffset`, `InvalidLandingProbability`, `InvalidAdversarialProbability`,
`ZeroCongestionLookback`); `spec.rs` and all LF fixtures byte-untouched; nothing wired into
execution; planner-verified same day — gate re-run at the staged tree, counts reproduced, both
plan-pinned types confirmed, no duplicate types defined). **C8.7c DONE 2026-07-21** (landed
`02afbeb`; gate re-run: **475 / 0 / 1** (470 + 5), demo `ae064f79…` ×2 and sweep
`94e90c3c…` unchanged; `sweep::hf_cell` adds `HfCellResult`, `eval_hf_strategy` (private),
`eval_hf_cell` mirroring `cell.rs` line-for-line with `run_hf_priced` in place of `run`; `cell.rs`
and `portfolio` byte-untouched; `finite()` duplicated, not exported; zero callers outside its own
tests + the lib.rs re-export). **C8.7d DONE 2026-07-21** (landed `a05880a`;
planner re-verified: gate **478 / 0 / 1** (475 + 3), demo `ae064f79…` ×2 unchanged, sweep shasum
**moved by design** `94e90c3c…` → `f6e1ab5132754d69c3a2be23fc549df36c07909f` with a version-only
diff — `schema_version` 1.3.0→1.4.0, decision D-a honored; `HfRungsDto` + three optional
`skip_serializing_if`-absent fields on `CandidateMetricsDto` + `with_hf` builder; schema additive
only, `required` untouched). **C8.7e DONE 2026-07-21** (staged, awaiting operator commit; planner
re-verified: gate **485 / 0 / 1** (478 + 7), demo `ae064f79…` ×2 AND sweep `f6e1ab51…` both
UNCHANGED; new `sweep::hf_aggregate::aggregate_hf` — one-pass HF sibling of the LF aggregation
per D-b, shared-field recipes copied token-identical from `runner.rs`, D-g `cost_drag_share`/
`per_trade_edge` finally `Some` with exact-Decimal `None` guards, D-h rung rollups with BASE-rung
counter sums; `runner.rs` touched by exactly one visibility token on `neighbor_indices`; LF
`aggregate_evidence`/`aggregate_fee_sensitivity` byte-untouched). Next up: **⛔ REVIEW-C8.4-SEAL**
(the 7-lens seal review — operator go-ahead REQUIRED, multi-agent spend; blocks C8.7f). The FOREMAN §3 7-lens review of C8.4's
intraday holdout seal is now
ORDERED IN THE QUEUE as ⛔ REVIEW-C8.4-SEAL, between C8.7e and C8.7f — operator go-ahead
required; C8.7f's escalate-if enforces it.
`M-HF-C8-PAIR` stays deferred (HF-Q4/HF-Q2).
**2026-07-19 — FOREMAN kit ACTIVATED (planner reconcile pass, no code):** GATE is now
**`bash scripts/gate.sh`** (fresh run at `fcd75e4`: **441 / 0 / 1**; demo `ae064f79…` ×2
identical; sweep `94e90c3c…`; sweep-verify OK; no-exec-deps OK — zero drift vs the 07-18
baselines); handoff baselines generated by `scripts/handoff-baselines.sh`; cadence declared in
AGENTS.md §Cadence (`project-management: off`); `.claude/agents/` gains `card-executor` +
`review-lens` (model: sonnet; committed by the operator at `4a0848c` — an explicit operator
exception to never-stage-`.claude/`; agents themselves still never stage it). AGENTS.md's stale
"M4 in progress / 402 tests" status corrected.
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
- **DONE:** M5 (research decision) — **GATE DECLARED 2026-07-12 (card M5-C6): Branch A — ALL 10
  CANDIDATES REJECTED; the project returns to research.** Judged against master-plan.md:903-909
  verbatim. Evidence: [m5-sweep-report.json](m5-sweep-report.json) (sha256 `424e713f…fb40`, ×3
  byte-identical; 180 trials; every candidate fails the +0.02 baseline margin and
  edge-vanishes-under-doubled-costs). Thresholds were frozen 2026-07-11 BEFORE any real-data
  result ([m5-data-validation.md](m5-data-validation.md); `config/strategies/m5-frozen.toml`).
  **Holdout read count: 0** — the seal and the 2026-01-01..2026-06-30 holdout survive unseen.
  Step-0 pre-declaration review: PASS (6 agents). A clean reject-all is a success of the gates:
  neither daily-bar family earns mainnet shadow; **no execution work starts.**
- **DOING (park lifted 2026-07-21 by operator action):** the **M-HF research track** (Q7/D-0012,
  [m-hf-track.md](m-hf-track.md)) — parked 2026-07-20 at C8.7b DONE, then **resumed 2026-07-21**:
  the operator fired executor card C8.7c (planner seed), landed + planner-verified (475/0/1,
  shasums unchanged). Card execution continues at **C8.7d**.
  The operator's other build (Drift funding-rate shadow harvester, PLAN-001) lives in the
  **sibling repo `../drift-harvester`** — 2026-07-20 operator decision: each build keeps its own
  repo; machina's purpose statement stays as-is. Track history: entered
  2026-07-12: C1–C2.6 unblocked by operator note; HF-Q2 resolved (USDT + jitoSOL research
  allowlist); wave-1 cards C1/C2 reconciled against m-hf-track §1/§2 (IntraBar alias, typed
  Provenance) and flipped TODO. **Wave 1 C1–C2.6 ALL DONE 2026-07-13** (C1 intraday types +
  hygiene; C2 deterministic synthetic generator; C2.5 reuse proof — bet holds, post-card
  adversarial review PASS, trial_count pinned 192; C2.6 columnar storage + `IntradaySource`,
  D-0013, 31.5M rows / ~290 MiB peak RSS measured). **Wave-2 audit re-run DONE 2026-07-13** (no
  blocking drift; table in worklog). **C3/C4 DONE** (2026-07-13/07-15): C3 `run_hf`/`LatencyPipeline`
  (simulator.rs untouched, adversarially reviewed, stands); C4 `HfCostModel`/`hf_trade_cost` — one
  escalation (a card transcription slip caught + fixed at source, gate re-run green). **C5 DONE +
  independently verified 2026-07-15** (adversarial terms: sandwich/pickoff τ at worst + base-rung
  expected `p·τ` rungs, and a trade-through-only maker fill; pure pricing/fill functions like C4,
  NOT wired into run_hf — that is C6/C8): fmt/clippy clean, 382 passed/0 failed/1 ignored,
  demo+sweep shasums unchanged, scope exactly `adversarial.rs`+`lib.rs`, every reference number
  hand-recomputed. **Mandatory C3–C5 adversarial checkpoint DONE 2026-07-15 (m-hf-track §5):
  PASS-with-hardening.** 5 lenses, each finding refuted-or-confirmed by 2 skeptics: determinism = 0,
  contract-drift = 0 (the load-bearing lenses clean); 11 raw findings → 5 survived → **2 real,
  distinct fail-closed gaps on C4's cost types** (both cost-understating, both UNREACHABLE in any
  wired path today, both mirroring the `DepthCurve::new` guard C4 already ships): (a) `DepthCurve::new`
  doesn't reject non-monotonic `impact_bps` (a larger trade can price cheaper than a smaller one);
  (b) `CongestionPriorityTable`/`HfCostModel` accept negative lamports → negative gas → a fabricated
  benefit. **Card M-HF-C5.1 DONE + verified 2026-07-15** (both closed fail-closed in `hf_cost.rs`:
  `DepthCurve::new` rejects strictly-decreasing `impact_bps`; `CongestionPriorityTable` fields private
  behind `new()` rejecting negatives; 3 new tests; 385/0/1; demo/sweep shasums unchanged; no
  priced-value moved). **C3–C5 block hardened; checkpoint fully closed.**
  **C6 DONE + independently verified 2026-07-15 — NARROW scope by operator decision:**
  turnover-criterion replacement ONLY (`advance.rs`: `turnover_budget`→`Option`,
  +`cost_drag_share_ceiling`/`per_trade_edge_floor` gating
  +`RejectionKind::{CostDragExcessive,PerTradeEdgeInsufficient}` + exhaustive `label()`; DTO+schema
  additive minor bump 1.1.0→1.2.0). Gate: **391/0/1**; primary byte-identical-to-M4 regression met;
  sweep report diff is exactly the one `schema_version` line ⇒ **new sweep shasum
  `85d06e5be4b1a2ac09713a30b56ba794624dc260`** (replaces `7ad3df7d…`); demo shasum unchanged; old
  1.1.0 report validates against 1.2.0. A 5-agent surface map proved §4/§5's C6 sketch drifted from
  code — the "RejectionKind consumer audit" is vacuous (no consumers in results/cli), the HF-kind
  spec-lint has no discriminator (born in C8), HF `ScenarioId` rungs don't reach the report without
  C8 — so **all ladder/scenario/provenance/spec-lint work moved to C8**; **m-hf-track §4/§5 corrected
  2026-07-15**. Card-audit miss (owned): the card listed `AdvancementThresholds` construction sites
  but not `CandidateEvidence`'s, missing `runner.rs` — executor's escalate-if caught it, operator
  approved the mechanical `None,None` extension (lesson recorded). HF-Q1
  (deferred to wave 2) blocks C9/C10;
  HF-Q3 blocks C10. M6+ does NOT start (reject-all ⇒ no shadow candidate); M8/M9 (signing/submit)
  need **separate explicit human approval**.

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
- `cargo run -p cli -- sweep` → byte-identical across 2 runs and `--threads 8` (shasum `7ad3df7d…`
  at M4/M5; **now `85d06e5b…` after M-HF-C6's schema bump to 1.2.0**);
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
- M5 gate declaration (card M5-C6, 2026-07-12) → fresh battery: fmt/clippy clean; **313 passed, 0
  failed**; demo `ae064f79…` + template sweep `7ad3df7d…` unchanged; sweep-verify OK; real-data
  sweep three-run byte-identical (incl. `--threads 8`); checked-in report sha256 `424e713f…fb40`
  byte-identical to a fresh run; `data-validate` matches the freeze record exactly; step-0
  adversarial review PASS (6 agents); holdout read count 0.

## Safety posture (all upheld)
- No keys / signing / submit / RPC / network in the tree (no such crate is even depended upon).
- No secrets; `*.example.toml` templates only; RPC config holds env-var NAMES, never values.
- Allowlist-gated; next-bar execution (final-bar signal cannot open a position); strategies = intent.

## Commit state
- Operator committed through M5-C5; the M5-C6 declaration diff (plan files only) is staged pending
  operator commit. Operator commits only — executors never commit/push; stage explicit paths only,
  never `.claude/`, never `git add -A`.

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
  operator may unblock them ahead of HF-Q1 by written note. *(Both since resolved: C1/C2
  reconciled + unblocked 2026-07-12; the sweep/strategies/invariants audit re-run **satisfied
  2026-07-13** — drift table in the worklog, no blocking drift.)* Wave-2+ cards are drafted
  against `m-hf-track.md` §5, not the superseded sketch in `highfrequency-algo-plan.md` §3.
- Accepted nit (no action): CostModelDto lamports are `i64` vs schema `minimum:0` — latent only, no
  negative-producing path (audit JS-1).
- Deferred to later milestones: M6
  route/shadow crates; M7 wallet-state; M8/M9 gated execution. (M4 is DONE — sweep crate, CLI wiring,
  and gate declaration all complete, cards M4-C1…C10.)

## Next recommended command
**C8.1–C8.4 + CARD-HYG-1 are DONE + planner-verified (2026-07-17), committed at
`3757d2a`+`ddd4512`.** Per-card verification evidence is in worklog 2026-07-17 ("Planner
verification wrap"). Gate freshly re-verified 2026-07-18 at HEAD `ddd4512` (clean tree):
fmt/clippy clean; **433/0/1**; demo `ae064f79…` unchanged; sweep `94e90c3c…` (moved once, at
C8.2's schema 1.2.0→1.3.0 bump, byte-diff-explained); sweep-verify OK.

**M-HF-C8.5 is DONE (2026-07-18) + planner re-verified the same day.** `sweep::hf_spec`
(`HfSweepSpecToml`/`HfSweepSpec`/`HfSpecError`, parse-only, LF `spec.rs` byte-untouched);
spec-lint fail-closed at parse time (`turnover_budget` → `deny_unknown_fields` rejection;
cost-drag/per-trade pair required → always `Some`; depth curve required, `portfolio`'s
constructors surfaced not re-checked; `max_lookback_bars > 0`); new
`config/strategies/hf-strategy-lab.example.toml` (illustrative, NOT tuned) + 8 tests. Planner
gate re-run at the staged tree: **441/0/1**; demo `ae064f79…` + sweep `94e90c3c…` unchanged;
sweep-verify OK; staged scope exactly the 4 card files + queue/worklog flip. All 3 pre-ruled
drifts held; no escalations.

**M-HF-C8.6 planner reconciliation pass is DONE (2026-07-18, plan files only, HEAD `3af2dbb`).**
C8.6 split into two executor-ready cards, task-queue.md: **C8.6a** (`sweep::congestion::
classify_congestion_regimes` — non-lookahead trailing-volume tercile classifier over `&[Bar]`,
pessimistic `Hot` default below `lookback+1` bars; 2 files) and **C8.6b**
(`portfolio::hf_priced::run_hf_priced` — a new sibling to `run_hf` that synthesizes a per-trade
`CostModel` from `HfCostModel`+`AdversarialModel`+the per-bar regime+a distinguishably-keyed
adverse-selection draw, and calls the EXISTING `apply_buy`/`apply_sell` unmodified — zero new
accounting code in `state.rs`/`cost.rs`; 4 files, including one visibility-only touch to
`latency.rs`). Both preserve every hard constraint from the original scoping (`run_hf`/
`simulator::run` byte-identical; priced costs never cheaper than plain `CostModel`, proven by a
pinned reference-config test; deterministic, no RNG/clock; the adverse-selection draw reuses the
`(cell_id, event_index)` splitmix64 pattern keyed distinguishably from `landing_draw`). One design
was narrowed and logged: the "(regime, percentile) → p" adverse-selection table is scoped OUT
(`AdversarialModel` has no such axis as landed by C5) — C8.6b uses `AdversarialModel`'s existing
flat probability instead.
- **C8.6a and C8.6b are DONE** (2026-07-19 / 2026-07-20, planner-verified; see the top block).
- **M-HF-C8.7 planner reconciliation pass is DONE (2026-07-20, plan files only, HEAD `44d483c`).**
  The row-C8 gate is now seven executor-ready cards in task-queue.md: **C8.7a** (`sweep::
  hf_scenarios` 6-rung ladder + the CARD-HYG-3 fold), **C8.7b** (`HfSweepSpec` gains `[latency]`/
  `[adversarial]`/`[congestion]`, parse-only), **C8.7c** (`sweep::hf_cell` priced eval core),
  **C8.7d** (`SweepReport` additive HF fields, schema **1.3.0→1.4.0** — the sweep shasum MOVES
  once here, version-only diff, pinned expected), **C8.7e** (`sweep::hf_aggregate` — evidence
  with `cost_drag_share`/`per_trade_edge` finally `Some`, redeeming C6), **⛔ REVIEW-C8.4-SEAL**
  (the standing 7-lens seal review, operator-gated, MUST precede C8.7f), **C8.7f**
  (`run_hf_sweep` capstone over `IntradaySource` + the C8.4 seal), **C8.7g** (the gate battery:
  determinism ×{1,2,3,7,8}, holdout counter 0, schema-valid, wall-clock recorded via a second
  `#[ignore]` test — gate ignored-count goes 1→2 there, sanctioned). Six drift corrections
  recorded in the section preamble (no latency/adversarial params in the spec; `Vec<Bar>` does
  NOT implement `IntradaySource`; the seal is materialized-Vec-shaped; `hf_cost_scenarios`
  didn't exist; `ColumnarFile` carries no provenance; the §3 percentile table is deferred to
  C9+, on the record). **C8.7a is DONE** (2026-07-20, `8d187ef`, planner-verified: 464/0/1,
  shasums unchanged, ladder unwired, CARD-HYG-3 closed). **C8.7b is DONE** (2026-07-20, landed
  `fedeb7c`; gate 470/0/1, shasums unchanged, parse-only held, resolved into
  C8.7a's `HfLatencyParams` + `portfolio::AdversarialModel` — no duplicate types). **C8.7c is
  DONE** (2026-07-21, landed `02afbeb`; 475/0/1, shasums unchanged). **C8.7d is DONE**
  (2026-07-21, landed `a05880a`; 478/0/1, sweep shasum moved by design to
  `f6e1ab51…`, version-only diff per D-a). **C8.7e is DONE** (2026-07-21, staged awaiting
  operator commit; 485/0/1, both shasums unchanged, `sweep::hf_aggregate` pure, LF aggregation
  byte-untouched). **The next step is ⛔ REVIEW-C8.4-SEAL — operator go-ahead required before
  C8.7f.**
- **Operator ruling HF-Q4 (this session):** `statarb_pairs_v1` gets a real two-leg engine (the
  precomputed-spread-series shortcut was rejected as a fabricated-edge risk). Because that is roughly
  as large as C1–C7 combined, it is scoped OUT of C8's gate into a deferred mini-track,
  **`M-HF-C8-PAIR`** (task-queue.md) — its own future card sequence, not drafted yet. Blocked
  additionally on HF-Q2's still-unsupplied USDT/jitoSOL `[[token]]` fields (exact list in
  task-queue.md's `M-HF-C8-PAIR` section — external, migration-sensitive, never invented).

M6 has no candidate and does not start.

Do NOT start M6+ (network), and never M8/M9 (signing/submission — separate explicit human approval).
