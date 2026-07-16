# Current state

Optimized for fast agent parsing. Source of truth for "where are we." Updated 2026-07-15 (M-HF
wave 2: C3/C4/C5/C5.1 DONE + verified; C3–C5 adversarial checkpoint PASS-with-hardening closed;
**C6 DONE + verified** (narrow: turnover-criterion replacement; sweep schema 1.2.0, new sweep shasum
`85d06e5b…`); next executor card is C7 (verify its drafted text vs the landed code first)).
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
- **DOING:** the **M-HF research track** (Q7/D-0012, [m-hf-track.md](m-hf-track.md)) — entered
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
**M-HF-C5 is DRAFTED (TODO)** (2026-07-15; card appended after M-HF-C4 in task-queue.md —
adversarial execution terms as costs to us only: sandwich + pickoff τ priced at a worst rung (τ on
every taker fill) and a base rung (expected `p·τ`, exact rational), plus a trade-through-only maker
fill; pure pricing/fill functions like C4, NOT wired into `run_hf` — per-fill hash realization + the
`(regime,percentile)→p` table are C8's job). Every reference number (τ@1000=4, τ@2000=8, expected
p=5/100→0.2, p=1/2→2, p=1/1→4, p=0/1→0) was verified empirically against the real `apply_bps` /
rust_decimal before drafting (throwaway test, run green, deleted; tree clean) — the C4 lesson
applied. Next: an **executor session runs M-HF-C5** (fresh chat, one card). After C5: the
**mandatory C3–C5 block adversarial review** runs before C6+ is drafted/expanded. M6 has no
candidate and does not start.

Do NOT start M6+ (network), and never M8/M9 (signing/submission — separate explicit human approval).
