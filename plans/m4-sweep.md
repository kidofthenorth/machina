# M4 — Sweep / Walk-Forward (active milestone plan)

> Research-only. M4 adds **no** keys, signing, RPC, or network. It adds one new crate
> `crates/sweep`, one additive field on `portfolio::RunOutput`, one date→unix parser in
> `research-core`, and one new schema `schemas/sweep-report.schema.json`. **Zero new external
> dependencies** (parallelism is `std::thread::scope`).
>
> Source of truth for the M4 build. Derived from a 3-design + adversarial-review design workflow
> (2026-06-29). Status pointer lives in [current-state.md](current-state.md); subtask status in the
> table in §14 below and in [task-queue.md](task-queue.md).

## 1. Goal & gate

Turn the existing single-run pipeline (`portfolio::run` → `metrics::Metrics::from_equity` →
`results::RunResult`) into a **deterministic, parallel parameter sweep** with walk-forward windows,
fee/cost sensitivity, baseline comparison, turnover reporting, and a rejection report — exporting
canonical JSON.

**M4 gate (plan §19):**
- Parallel results == sequential results (byte-for-byte and structurally).
- Repeated runs identical.
- Untouched holdout remains untouched.

## 2. Scope (plan §19, §25 line ~1420)

Deliver: parameter sweeps for `trend_alloc_v1` + `threshold_rebalance_v1`; parallel execution;
canonical export; walk-forward windows; strategy-family comparison; **turnover & fee-sensitivity as
first-class outputs**; rejection report for failed candidate families.

Out of scope (deferred to M5): the *decision* (which candidate advances), holdout scoring as a
research conclusion, real multi-year data, dense parameter surfaces.

## 3. Dependency decision (D-0002, confirmed by adversary → record as D-0009)

**Zero new external deps.** `std::thread::scope` (stable since Rust 1.63; workspace MSRV 1.80)
supplies the embarrassingly-parallel, index-chunked, disjoint-write pattern with no crates. **rayon
is rejected**: although the `no-execution-deps` CI scan does not list it (it would pass), D-0002's
minimalism is the bar, and a work-stealing scheduler buys nothing for a precomputed `Vec` of
independent cells.

`crates/sweep` deps: `research-core`, `market-data`, `portfolio`, `metrics`, `strategies`,
`results`, `rust_decimal`, `serde`, `serde_json`, `toml`. Dev: `rust_decimal_macros`, `jsonschema`
(already an allowed workspace dev-dep). No new internal-graph additions beyond the `sweep` crate
itself (D-0003: added in milestone order).

## 4. The turnover correctness fix (adversary verdict — round-trip turnover is WRONG)

Verified against `crates/portfolio/src/equity.rs` + `simulator.rs`: `RoundTrip` is pushed **only** on
a flat→long→flat cycle (`record_round_trip` fires `Side::Sell` when `!now_long`); the unclosed
`OpenPosition` is a local var never exported. `Static5050` / `ThresholdRebalanceV1` rebalance without
ever going flat → `n_trades>0` but **zero** `round_trips`. A round-trip-based turnover would report
~0 turnover for the highest-churn strategies — silently undercounting exactly what §25 demands as a
first-class output.

**Fix (additive, behavior-preserving):** add one field to `portfolio::RunOutput`:
```rust
pub traded_notional_quote: Decimal,   // sum of USDC notional moved per executed trade
```
Accumulate it in `simulator::run`'s existing `if let Some(outcome) = rebalance(...)` branch, next to
`n_trades += 1`, from the already-computed `TradeOutcome`:
- **Buy:** notional = `-outcome.quote_delta` (== `quote_in`, exact USDC spent).
- **Sell:** notional = `(outcome.base_delta.abs() - outcome.gas_sol) * outcome.exec_price`
  (== `base_in * mid_price`, gas leg removed), consistent with how the simulator values slippage
  (`base_in*(price-eff)`), staying in `Decimal`.

This counts rebalancer churn that never closes a round trip. **All `Decimal`, exact, order-stable.**
(Verify `TradeOutcome` exposes `quote_delta`/`base_delta` when implementing S1; adjust the exact
expression to the real field names if they differ, preserving the `base_in * mid_price` semantics.)

Turnover ratio (in `sweep::turnover`):
```
turnover = traded_notional_quote / mean_equity
mean_equity = sum(equity_curve[i].equity_quote) / equity_curve.len()
```
with a zero/empty guard returning `Decimal::ZERO`. **Annualization is NOT applied to the budget
check** — the §14 "economically plausible turnover" rejection runs on the un-annualized exact
`Decimal` ratio, keeping the decision off f64. (An annualized display value may be reported
separately but never feeds a sort/threshold.)

Thread the `Decimal` into `MetricsReport.turnover` (currently hard-coded `None`) by adding
`turnover: Option<Decimal>` to `RunInputs` and using it in `RunResult::build`. Populates the
schema's already-present `turnover` field — **no schema change** for turnover.

## 5. Determinism strategy (the M4 gate)

1. **Canonical enumeration.** Materialize the full work list as `Vec<Cell>` in fixed order:
   **window index → cost-scenario id → param-axis order** (each axis in declared order). Param keys
   are built from `Decimal`s **normalized to a canonical scale** so the string sort key is
   numeric-stable — guards against `0.5` vs `0.50` scale drift that `rust_decimal` preserves.
2. **Parallelism = `std::thread::scope` only.** Pre-size `Vec<Option<CellResult>>` of length N. Split
   by **contiguous index chunks** via `split_at_mut`. Each scoped thread maps the pure `eval_cell`
   over its `&[Cell]` slice and writes **only** its disjoint `&mut [Option<…>]` region. No mutex, no
   atomics, no shared accumulator. After join, `unwrap` in index order.
3. **`eval_cell` is pure** over `(bars-slice, &CostModel, &ParamPoint, cash, ppy)` — wraps the
   already-deterministic `portfolio::run`. Output **value** at each index is identical regardless of
   scheduling; output **order** is the pre-assigned cell index, fixed before any thread spawns.
4. **Two-level determinism test:** assert `run_cells(cells, K) == run_cells_sequential(cells)` by
   **full structural equality** (`CellResult: PartialEq`) for `K ∈ {1,2,3,7,8}` **AND** byte-identical
   `to_json()`. Structural equality catches divergence that `{:.10}` f64 formatting could mask.
5. **All sort/threshold/ranking/selection keys are `Decimal`** (`total_return`, `max_drawdown`,
   `turnover`) — never raw f64. Enforced by an **audit test** forbidding f64 comparisons in the
   selection/advancement module.
6. **Gate test pins explicit thread counts** (never `available_parallelism()` in the test path). The
   CLI may use `available_parallelism()` since values are count-invariant; the gate must not.
7. **NaN guard:** normalize non-finite f64 stats to `None` at `CellResult` construction (matching
   `results::stat_to_string`, which already filters `is_finite`). Do this at the cell boundary, not
   just at serialization, so structural equality cannot flake on `NaN != NaN`.
8. **No wall-clock in canonical output.** `created_at` derives from the window's last bar
   `ts.to_rfc3339()`; `run_id` from a deterministic `(family, param_id, scenario_id, window_index)`
   tuple. A repeated-serialization test asserts byte-identity.

## 6. Holdout sealing (physical partition + consume-by-value)

- `PartitionedBars::from_spec(bars, &PartitionSpec)` validates the full series, then **physically
  moves** the holdout bars into a separately owned `Holdout(Vec<Bar>)` — **not** a `&[Bar]` borrow
  into the shared backing array.
- `from_spec` **asserts the three partitions are disjoint and ordered**:
  `dev.end <= val.start && val.end <= holdout.start`. Overlapping config is rejected, not bled.
- `seal_holdout(self) -> (DevValidation, Sealed)` consumes `PartitionedBars`. `DevValidation` exposes
  `development()` / `validation()` / walk-forward windows and has **no** holdout accessor.
- Because holdout bars are a **different allocation not in the dev/val array**, `walk_forward` over
  `n_bars = dev+val length` is **structurally incapable** of indexing into holdout.
- **The single holdout reader consumes the seal by value:**
  `evaluate_on_holdout(sealed: Sealed, chosen: &ParamPoint, cost: &CostModel, cash, ppy) -> CellResult`.
  Taking `Sealed` by value makes it **call-once**. This is the M5 entry point; **the M4 CLI never
  calls it.**
- **Date→unix parser:** add a pure UTC `civil_date_to_unix(y, m, d) -> i64` (and `parse_ymd`) **in
  `research-core::time`, next to `civil_from_days`** — the inverse of the existing `civil_from_days`.
  Pure integer arithmetic, zero deps. Unit-test against `2021-01-01`, leap day `2024-02-29`,
  `2025-12-31`.

**Seal tests:** (a) `Cell<u32>`/digest read-counter around holdout — run the FULL sweep+selection
pipeline (holds only `&DevValidation`), assert counter still 0 and holdout digest unchanged;
(b) `compile_fail` doc-test proving selection/`DevValidation` has no path to holdout bars and cannot
fabricate the seal; (c) `evaluate_on_holdout` consumes `Sealed` by value (call-once).

## 7. Walk-forward window model

`Window { train: Range<usize>, test: Range<usize> }` over **bar indices** into the dev/val array.
Invariant `train.end <= test.start` (embargo gap `>= embargo_bars`), tests ordered and
non-overlapping. `WalkForward { kind: WindowKind, train_len, test_len, step, embargo }` with
`WindowKind::{Rolling, Anchored}`.

**Default = Rolling** (operator-confirmable, §16): crypto regime changes are severe (plan §14: "do
not trust one bull-market backtest"); a fixed-length rolling train de-weights stale regimes.
`Anchored` (train start fixed at 0, end grows) is offered for the low-data case. No-future-data
(invariant 5) is already enforced inside `portfolio::run`; windows add the *cross-window* non-leak
guarantee.

## 8. Turnover definition (see §4) — canonical, least-gameable

`turnover = traded_notional_quote / mean_equity`, all `Decimal`. Sourced from the new
`RunOutput.traded_notional_quote` aggregate (NOT `round_trips`). Forward-vs-reverse summation
identical (no f64 path) — tested. Fills `MetricsReport.turnover` via the new `RunInputs.turnover`.

## 9. Fee/cost sensitivity (first-class, plan §14/§25)

`cost_scenarios(base) -> Vec<(ScenarioId, CostModel)>` building a fixed ladder by **exact integer
scaling** (u32/i64 fields only, no f64):
- `before_costs` = `CostModel::zero()` (the "same strategy before costs" baseline).
- `base` = as configured.
- `doubled` = each cost field ×2 (the mandated "doubled costs/slippage" survival test).
- optional `doubled_slippage` / `doubled_priority` isolating axes; optional `scaled(num, den)`.

Every `(family, param, window)` cell is evaluated across **all** scenarios; **baselines are re-run
cost-matched** under each scenario. Reported per candidate: a `fee_sensitivity` block with
`total_return`/`turnover`/`n_trades`/`fees_paid_usdc`/`slippage_paid_usdc`/`priority_fees_paid_sol`
under base vs doubled, plus `return_drag_doubled = return_base - return_doubled` (a `Decimal` delta,
**not** framed as profit) and `survives_doubled: bool`. No profitability claim (invariant 11).

> **Monotonicity is conditional (S10 review).** "Doubled costs ⇒ ≥ base fees and ≤ base return" holds
> only when the **trade set is unchanged**. Higher costs can *suppress* a marginal trade (the
> simulator's `min_trade_notional` / `InsufficientSolForGas` skips), so a suppressed net-losing trade
> can leave doubled costs *cheaper* and *higher-return* than base — `return_drag_doubled` then goes
> negative. This is honest simulator behavior, not a bug: `fee_sensitivity` records it faithfully (the
> trade-set change is visible as `base.n_trades != doubled.n_trades`), pinned by a regression test.
> Do **not** re-impose monotonicity as a runtime invariant.

## 10. Strategy-family comparison

Per family, aggregate cells into a comparison row: best/median/worst across param neighbors, baseline
deltas (cost-matched), walk-forward fold consistency, fee-sensitivity degradation. Keyed and sorted
on `Decimal` fields only.

## 11. Rejection report (robustness only — invariant 11)

`RejectionReport { thresholds, trial_count, verdicts: Vec<CandidateVerdict> }`;
`CandidateVerdict { candidate_label, status: Verdict, failed_criteria: Vec<RejectionReason> }`;
`Verdict::{Advanceable, Rejected}`. `RejectionReason` enum, one variant per §14 criterion, each
carrying observed `Decimal` vs threshold (as strings):
- `FailsBaselineComparison` (doesn't beat hold_usdc / buy_and_hold_sol / static_50_50 / dca_sol after
  base costs, out-of-sample).
- `EdgeVanishesUnderDoubledCosts` (`survives_doubled = false`).
- `DependsOnOnePeriod` (walk-forward fold dispersion / worst-window beyond budget).
- `ParameterFragile` (neighboring grid points degrade beyond tolerance).
- `DrawdownExceedsBudget` (`max_drawdown` > configured `Decimal` budget).
- `TurnoverImplausible` (un-annualized turnover `Decimal` > configured budget).
- `InsufficientData` (a window/partition failed validation).

`trial_count` (total cells evaluated) recorded for M5 multiple-testing accounting. **Language is
strictly robustness-failure**; `Advanceable` means "passed the robustness battery, eligible for M5
review," never "this works." A JSON `note` states the report is a research filter, not a
profitability verdict. Sorted canonically.

## 12. Canonical export & schema impact

- `SweepReport::to_json()` / `to_value()` via `serde_json::to_string_pretty`, money/turnover as exact
  **decimal strings**, f64 stats via `{:.10}` (`results::stat_to_string`), collections pre-sorted by
  the canonical cell key (`BTreeMap` for any keyed maps).
- **New schema** `schemas/sweep-report.schema.json` (Draft 2020-12, `additionalProperties:false`,
  `decimalString` for money) — an **additive** contract (D-0001). Validated by
  `crates/sweep/tests/schema_validation.rs` mirroring `results/tests/schema_validation.rs`.
- **`run-result.schema.json` is UNCHANGED**: its `turnover` field already exists; M4 only populates
  it. Per-cell records may embed a `results::RunResult` behind an opt-in flag for large grids.

## 13. CLI wiring

Add to `crates/cli/src/main.rs` alongside `demo`, same hand-rolled `args.next()` match (no clap,
D-0002), add `sweep = { workspace = true }` to `crates/cli/Cargo.toml`:
- `machina sweep [--threads N] [--out PATH]` — loads embedded `strategy-lab.example.toml` +
  `allowlist.example.toml`, builds a `SweepSpec` (families from the `[trend_alloc_v1]` /
  `[threshold_rebalance_v1]` grids — **note config key `rebalance_band` maps to struct field
  `band`** — cost scenarios before/base/doubled, walk-forward over the development partition), runs
  `run_sweep` with `Parallelism::Threads(N)` (default `Sequential`), prints/writes canonical JSON.
- `machina sweep-verify` — runs the sweep both `Sequential` and `Threads(N)`, asserts byte-identical
  output, exits non-zero on mismatch (local mirror of the CI gate).
- **Holdout is NOT reachable from any M4 CLI path** — `evaluate_on_holdout` is M5-only.

## 14. Ordered subtasks (each keeps the workspace green: fmt + clippy -D warnings + test)

S5 (S7 here) is the determinism gate; S9 is the holdout gate. The three earliest touch **existing**
crates (portfolio additive field, results turnover, research-core date parser), isolated with their
own gates so the frozen M1/M2 surfaces change in tiny, asserted steps.

| ID | Status | Title | File scope | Depends | Gate |
|----|--------|-------|------------|---------|------|
| S1 | ✅ DONE | `traded_notional_quote` on `RunOutput` | `portfolio/src/simulator.rs` | — | DONE: hand-computed buy+sell (2000); zero-round-trip rebalancer (937.5); no-trade run (0); determinism green |
| S2 | ✅ DONE | Pure `civil_date_to_unix` / `parse_ymd` | `research-core/src/time.rs`, `lib.rs` | — | DONE: pinned 2021-01-01 / leap 2024-02-29 / 2025-12-31 exact secs; round-trips `to_rfc3339`; rejects malformed + impossible dates |
| S3 | ✅ DONE | Thread turnover into `RunInputs` → `MetricsReport.turnover` | `results/src/lib.rs`, `results/tests/schema_validation.rs`, `cli/src/main.rs` | S1 | DONE: turnover serializes as decimal string (not null) and still validates against UNCHANGED run-result schema |
| S4 | ✅ DONE | `crates/sweep` skeleton + workspace wiring | `Cargo.toml`, `sweep/Cargo.toml`, `sweep/src/lib.rs` | — | DONE: workspace builds; clippy/fmt green; no-execution-deps passes; sweep dep tree clean |
| S5 | ✅ DONE | Param model + grid + strategy bridge | `sweep/src/param.rs`, `lib.rs` | S4 | DONE: fixed-order Cartesian product (both families); `build_strategy` round-trips family+behavior; `param_id` scale-canonical (0.50→0.5) |
| S6 | ✅ DONE | Single-cell runner + exact Decimal turnover + NaN normalization | `sweep/src/cell.rs`, `sweep/src/turnover.rs` | S1,S3,S5 | DONE: `eval_cell` matches direct `run`+`Metrics`; turnover exact `0.5` hand value; fwd-vs-rev sum identical; non-finite f64→None |
| S7 | ✅ DONE | **Determinism gate** — `std::thread::scope` scheduler | `sweep/src/parallel.rs`, `sweep/tests/determinism.rs` | S6 | DONE: parallel==sequential structural + byte-identical for K∈{1,2,3,7,8}; repeated-run byte-identity; explicit thread counts; anti-vacuous guard |
| S8 | ✅ DONE | Walk-forward window model | `sweep/src/window.rs`, `sweep/tests/walk_forward.rs` | S4 | DONE: `Window`/`WalkForward`/`WindowKind::{Rolling default,Anchored}`; exact-enumeration tests pin embargo magnitude (≥2) + `step`-coupled shapes; ordered+non-overlapping tests; in-bounds; `new()` rejects bad params + `windows()` total/defensive; zero deps. Adversarially verified (5-lens + adjudicator): impl correct, two test-coverage gaps found & closed |
| S9 | ✅ DONE | **Holdout gate** — partition + physical seal + consume-by-value | `sweep/src/partition.rs`, `sweep/src/config.rs`, `sweep/tests/holdout_sealing.rs` | S2,S6,S8 | DONE: `PartitionSpec::{ByIndex,ByDate}` (ByDate via S2 parser); `from_spec` validates via canonical `market_data::validate_series` (sorted+unique) then `split_off`s holdout into a SEPARATE `Vec<Bar>`; rejects overlap/empty/out-of-bounds; `seal_holdout(self)->(DevValidation,Sealed)` (DevValidation has NO holdout accessor); call-once `evaluate_on_holdout(sealed: Sealed,…)` (M5-only). Gate green: `Cell<u32>` read-counter==0 + digest unchanged after full sweep+selection pipeline; 3 `compile_fail` doctests (no selection→holdout path, no seal fabrication, call-once consume-by-value) + `no_run` canaries; overlap rejected (ByIndex+ByDate). Adversarially reviewed (6 lenses + skeptics): seal sound; 3 minor/nit hardening fixes applied (delegate to `validate_series`→duplicate-ts rejection; real post-split structural assert; within-process-only digest doc) |
| S10 | ✅ DONE | Cost/fee sensitivity ladder + cost-matched baselines | `sweep/src/sensitivity.rs`, `sweep/src/baseline.rs`, `sweep/src/cell.rs` (extract `eval_strategy`) | S6 | DONE: `cost_scenarios` ladder {before_costs=`zero()`, base, doubled, doubled_slippage, doubled_priority} via exact integer `scale_cost_model` (u32/i64, no f64); `ScenarioMetrics`/`FeeSensitivity` (Decimal `return_drag_*`, baseline-grounded `survives_doubled`); `baseline.rs` runs all 4 baselines cost-matched through the shared `eval_strategy` core (label==strategy name; `best_baseline_return` floor). Doubling exact (hand-checked); before_costs pays nothing; doubled ≥ base fees / ≤ base return **when the trade set is unchanged** (asserted via an `n_trades`-equality guard). Adversarially reviewed (3-lens + skeptics): no code defects; **key finding — cost monotonicity is NOT a universal law** (higher costs can suppress a marginal trade via `min_trade_notional`/gas skips → fewer fees, higher return, negative drag), now documented + pinned by a regression test; +2 doc nits fixed |
| S11 | TODO | Advancement/rejection + `SweepReport` export + new schema | `sweep/src/advance.rs`, `report.rs`, `schemas/sweep-report.schema.json`, `sweep/tests/schema_validation.rs` | S7,S9,S10 | validates against new schema; each criterion yields matching reason; `trial_count` recorded; f64-comparison audit; two serializations byte-identical |
| S12 | TODO | CLI `sweep`/`sweep-verify` + refresh plans/docs/DECISIONS | `cli/src/main.rs`, `cli/Cargo.toml`, `plans/*`, `docs/architecture-index.md`, `DECISIONS.md` (D-0009) | S11 | `machina sweep` prints deterministic schema-valid report; `sweep-verify` exits 0; full workspace green; current-state M4 pointer updated |

## 15. Operator decisions (illustrative defaults applied; freeze before M5 — see questions.md Q4–Q5)

These are **research policy**, not code constraints. M4 ships illustrative defaults in the config
templates; the **real values must be frozen before any M5 research decision** (choosing them post-hoc
is overfitting, plan §14). They do **not** block building the engine — the gate is determinism +
holdout sealing + turnover/fee reporting, none of which depend on the numbers.

- **Walk-forward sizing & kind** → new `[walk_forward]` block in `strategy-lab.example.toml`
  (illustrative `train=365, test=90, step=90, embargo=5, kind=rolling`). Default Rolling.
- **Rejection thresholds** (drawdown budget, turnover ceiling, baseline margin, walk-forward
  dispersion, neighbor tolerance) → new `[advancement]` table in `strategy-lab.example.toml`,
  illustrative values, clearly marked NOT tuned. Turnover budget stated against the un-annualized
  `Decimal` ratio §8 computes.
- **Per-cell RunResult export** → default OFF (aggregate `SweepReport` only); opt-in `--emit-cells`.
- **Doubled-cost survival floor** → "still beats the relevant baselines under the SAME doubled-cost
  scenario" (cost-matched), baseline set from config. Not a bare `total_return>0`.
- **Real data vs fixtures** → M4 engine + all gate tests on synthetic/fixture series; defer real
  multi-year SOL/USDC ingestion to the M5 data question (questions.md Q3).

## 16. Invariant references

Honors: 3 (determinism — §5), 4 (allowlist — CLI still loads/requires it), 5 (no future data —
next-bar already enforced; windows add cross-window non-leak), 6 (intent-only — sweep builds
`Box<dyn Strategy>`, owns no execution state), 7 (Decimal money — turnover/notional all Decimal),
11 (no profit claims — rejection/comparison framed as robustness, never profitability).
