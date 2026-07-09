# DECISIONS

Architecture decision log. Newest first. Each entry: context → decision → consequences. Keep terse.
These are *implementation* decisions made by the executor within the bounds of the master plan; they
do not authorize any new capability (no signing, no submission — see
[docs/invariants.md](docs/invariants.md)).

---

## D-0011 — M4 "strategy-family comparison" = the cross-family canonical report; per-family aggregation rows move to M5 (M4)
**Context.** The 2026-07-09 pre-declaration re-review confirmed a major: `plans/m4-sweep.md` §10
sketched a per-family aggregation row (best/median/worst across param neighbors) with no
corresponding artifact anywhere, and the C9 evidence checklist had silently substituted a weaker
reading — the failure mode D-0010 was recorded to prevent.
**Decision.** The master plan lists "Strategy-family comparison" under M4 but **"Strategy-family
ranking" under M5** (master-plan.md:900). M4's deliverable is satisfied by what actually ships: both
families' candidates scored through the identical shared `eval_strategy` core, against the same
cost-matched baselines and walk-forward windows, exported side-by-side (label-sorted) in
`SweepReport.candidates[]` with per-candidate metrics, fee sensitivity, and verdicts — a genuine
cross-family comparison under one cost model. The per-family *aggregate* row is descoped from M4 and
folded into M5's ranking deliverable, where it is trivially derivable from `candidates[]` and where
choosing its aggregation semantics belongs (after real data, Q3, and frozen thresholds, Q5).
`plans/m4-sweep.md` §10 is amended accordingly; the master-plan pair is untouched (its own M4/M5
split already says this).
**Consequences.** The M4 gate evidence cites the artifact that exists, with the descope recorded
rather than implied. M5's ranking work starts from `candidates[]`, not from a bespoke M4 structure
chosen before real data existed.

## D-0010 — SweepReport carries first-class per-candidate turnover + fee-sensitivity (M4)
**Context.** The pre-declaration adversarial review of the M4 surface (2026-07-07, 11 agents)
confirmed a major: the "turnover and fee-sensitivity reporting" deliverable never reached the
EXPORTED artifact — `sensitivity::FeeSensitivity` had zero production callers and `SweepReport`
surfaced turnover only inside failure reasons. Plan §25 requires both as first-class outputs.
**Decision.** `SweepReport` (schema_version 1.0.0 → 1.1.0) gains a required `candidates` array:
per candidate, worst-window `max_drawdown`/`turnover` plus a `fee_sensitivity` block
(before_costs/base/doubled aggregated ScenarioMetrics, both return drags, baseline-grounded
`survives_doubled`). Aggregation matches the evidence rules (mean returns, worst-window
turnover/drawdown, summed costs); `survives_doubled` shares its floor and comparison with the
edge-vanishes criterion via the single constructor `FeeSensitivity::from_scenarios`, so report
and verdict cannot drift. The schema edit is a deliberate, reviewed contract change (D-0001);
the review finding is its recorded cause.
**Consequences.** The M4 deliverable is satisfied in the artifact itself; BeforeCosts cells have
a production consumer; reports stay byte-deterministic (candidates sorted by label; exact
decimal strings; no f64).

## D-0009 — M4 sweep: std::thread::scope parallelism (zero new deps); traded-notional turnover; sweep-report schema; sealed holdout (M4)
**Context.** M4 needs a deterministic parallel sweep (plan §19) whose parallel output is
byte-identical to sequential, a turnover base that rebalancers cannot undercount, a canonical
report artifact, and a holdout that selection cannot touch.
**Decision.** (a) Parallelism is `std::thread::scope` over contiguous index chunks with disjoint
writes — **zero new dependencies; rayon rejected** (D-0002 minimalism; a work-stealing scheduler
buys nothing for a precomputed `Vec` of independent cells). The gate
(`crates/sweep/tests/determinism.rs`) pins explicit thread counts {1,2,3,7,8}, never
`available_parallelism`. (b) Turnover derives from the additive
`portfolio::RunOutput.traded_notional_quote` (`Decimal`; buy = USDC spent, sell = mid value of SOL
sold, gas excluded) — round-trip-based turnover reads ~0 for rebalancers that never go flat.
(c) The sweep exports `SweepReport` validating against the additive
`schemas/sweep-report.schema.json` (Draft 2020-12; D-0001); `run-result.schema.json` is unchanged.
(d) The holdout is physically partitioned and sealed (`seal_holdout`); the single reader
`evaluate_on_holdout` consumes the seal by value and is **M5-only — never wired into the M4 CLI**,
which instead asserts `holdout_read_count() == 0` on every run.
**Consequences.** Sweep results are reproducible across thread counts and runs (`machina
sweep-verify` mirrors the CI gate locally); turnover is exact and shape-independent; the report
contract evolves additively; selection code is structurally unable to read the holdout.

## D-0008 — `.gitignore` replaced with a project secret-safe gitignore (M0)
**Context.** The repo root previously held the cadence-kit's *payload* `.gitignore` (it ignored
`AGENTS.md`, `CLAUDE.md`, and the plan; its own comment noted "the repo root may also have its own
.gitignore"). M0 requires a real secret-safe gitignore.
**Decision.** Replace it with a project gitignore that (a) ignores keys/wallets/seeds/secrets/env,
(b) ignores real `config/**/*.toml` while keeping `*.example.toml` templates, (c) ignores `/target`,
generated `/data` and `/results`, (d) keeps `Cargo.lock` tracked for reproducible CI, (e) keeps
`AGENTS.md`/`CLAUDE.md`/`plans`/`docs`/schemas tracked, (f) ignores only `.claude/settings.local.json`
rather than all of `.claude/`.
**Consequences.** Doc/plan files become stageable (they are M0 deliverables). The operator still
stages explicit paths only.

## D-0007 — Strategies are scaffolds with deterministic tests, not tuned models (M3)
**Context.** M3 allows `trend_alloc_v1` / `threshold_rebalance_v1` "scaffold or minimal
implementation only if deterministic tests are in place," requires baselines (incl. DCA) and a
regime-classifier scaffold (plan §19/§26), and forbids profitability claims.
**Decision.** Implement a `Strategy` trait (history-in → target-weight-out, intent only); the four
required baselines — hold-USDC, buy-and-hold-SOL, static 50/50, and `dca_sol` (a weight-space DCA
ramp proxy); a minimal `regime::classify` scaffold (deviation-from-mean → Trending/RangeBound/
Unknown); and minimal deterministic implementations of `trend_alloc_v1` (SMA-bucketed allocation)
and `threshold_rebalance_v1` (drift-band rebalance). Each has unit tests asserting *behavior*
(bucketing, band logic, ramp, classification, determinism), **not** performance.
**Consequences.** No backtest in this repo constitutes evidence a strategy works. Tuning/validation
is M4–M5 and out of scope here. The regime scaffold's real consumer (`bounded_grid_experiment`) is
M4+; it ships now only to satisfy the M3 deliverable.

## D-0006 — Next-bar execution; final-bar signal cannot trade
**Context.** Plan requires "final-bar signal cannot open a new position" and "next-bar / quote-time
execution, never signal-price execution."
**Decision.** The simulator feeds the strategy bars `[0..=t]` and executes the resulting target at
bar `t+1`'s **open** price. The last bar produces a signal that is never executed.
**Consequences.** No same-bar lookahead. The invariant "final-bar signal cannot open a position"
holds by construction, and a test asserts it.

## D-0005 — Deterministic fill model: explicit slippage + DEX fee + network/priority fee
**Context.** M2 requires fee/slippage/priority-fee accounting as *configured assumptions* (no
randomness in canonical runs).
**Decision.** Fill model is a pure function of `(side, input_amount, exec_price, CostModel)`:
slippage shifts the effective price against the trader (`±slippage_bps`); DEX fee reduces the output
(`output × (1 − dex_fee_bps)`); Solana base fee + priority fee are charged in SOL from the SOL
balance (`(base+priority) lamports ÷ 1e9`). Token outputs are quantized to token decimals by
truncation toward zero (never over-credit). Failed-route probability is a *scenario stressor*, not
random behavior in canonical runs.
**Consequences.** Buy debits USDC + credits SOL; sell debits SOL + credits USDC; fees strictly
reduce equity; oversell/unaffordable trades are rejected. All asserted by tests.

## D-0004 — Money is `rust_decimal::Decimal`; statistical metrics are `f64` (display-only)
**Context.** Determinism is a hard requirement; the plan mandates decimal/integer fixed-point for
money and forbids floating point for balances/quotes.
**Decision.** All balances, prices, fees, costs, and equity use `rust_decimal::Decimal` (exact,
platform-deterministic, no float). Statistical *reporting* metrics that need `sqrt` (volatility,
Sharpe, Sortino) are computed in `f64` and are **never** fed back into accounting. Total return and
max drawdown are computed in `Decimal` so the `[0,1]` drawdown invariant is exact.
**Consequences.** No `f64` in any money path. A future lint/audit can grep for `f64` outside the
metrics statistical functions.

## D-0003 — Crate set tracks the plan's names; defer non-research crates
**Context.** The plan lists 12 crates spanning research and (gated) execution.
**Decision.** Create only the crates with real M0–M3 content: `research-core`, `market-data`,
`portfolio`, `metrics`, `strategies`, `results`, `cli`. Defer `sweep` (M4), `route-model` (M6),
`solana-execution` (M6/M8), `wallet-state` (M7), `risk` (M6) — named in the plan and the
architecture index, but **not** created as empty crates (avoids dead code and keeps the workspace
green). Shared domain types live in `research-core` (the plan's name for the research foundation).
**Consequences.** No execution-capable crate exists in the tree at all — the strongest form of the
"no live path" invariant. New crates are added in milestone order as content arrives.

## D-0002 — Dependencies: pure-math + serialization only; no SDKs, no network clients
**Context.** Executor rules forbid signing/submission SDKs and paid/credentialed clients.
**Decision.** Workspace dependencies are limited to `rust_decimal` (fixed-point math), `serde` +
`serde_json` (result serialization), `toml` (config templates), and dev-only `jsonschema` (schema
tests). **No** Solana SDK, **no** Jupiter client, **no** HTTP client, **no** wallet/keypair crate is
present in any `Cargo.toml`.
**Consequences.** It is impossible to sign or submit a transaction from this workspace because the
capability simply is not depended upon. A dependency audit (`cargo tree`) confirms this.

## D-0001 — JSON schemas are the cross-language contract (M0)
**Context.** The plan reuses the "JSON schema boundary" discipline from prior projects.
**Decision.** Author `run-result`, `route-quote`, `execution-event`, and `wallet-snapshot` schemas as
JSON Schema Draft 2020-12 under `schemas/`. The `results` crate's `RunResult`/`RunConfig` models
serialize to JSON that validates against `run-result.schema.json`, enforced by a test.
**Consequences.** Schemas are contracts: changing them is a deliberate, reviewed act. Execution-event
and route-quote schemas describe the *future* shadow/execution surface but ship now as the agreed
shape; no code produces live execution events yet.
