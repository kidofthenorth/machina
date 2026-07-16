# Task queue

Compact, actionable. Status: TODO / DOING / DONE / BLOCKED / DEFERRED.
Gate = the check that proves the task is complete.
This queue serves machina's goal — **genuine autonomous passive income**, earned through milestone
gates: M4 proves the sweep engine is deterministic and honest; live trading remains gated behind
M8/M9 explicit human approval.

## Completed (M0–M4 S11)

| ID | Status | Milestone | File scope | Gate | Notes |
|----|--------|-----------|------------|------|-------|
| T01 | DONE | M0 | `.gitignore`, `README.md`, `DECISIONS.md`, `docs/*`, `plans/*` | files exist, links resolve | Secret-safe gitignore replaces cadence-payload one (D-0008). |
| T02 | DONE | M0 | `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml` | `cargo metadata` ok | Workspace; deps are pure-math+serde only (D-0002). |
| T03 | DONE | M0 | `.github/workflows/ci.yml` | yaml valid; jobs defined | fmt+clippy+test + `no-execution-deps` scan. |
| T04 | DONE | M0 | `config/**/*.example.toml` | no secrets; parse in T08 | allowlist/rpc/costs/strategy templates. |
| T05 | DONE | M0 | `schemas/*.schema.json` | valid Draft 2020-12 | run-result/route-quote/execution-event/wallet-snapshot. |
| T06 | DONE | M0 | `crates/results` | schema-validation tests pass | RunResult validates against run-result schema. |
| T07 | DONE | M1 | `crates/research-core` | unit tests pass | money/token/bar/time; fixed-point money. |
| T08 | DONE | M1 | `crates/market-data` (+ `fixtures/`) | corrupt/dup/unsorted/missing/bad-decimal/disallowed-token tests pass | bar validation (incl. gap) + allowlist loader. |
| T09 | DONE | M2 | `crates/portfolio` | accounting + determinism tests pass | state, fills, simulator, equity, round trips. |
| T10 | DONE | M2/M3 | `crates/metrics` | metrics tests pass; drawdown ∈ [0,1] | return/dd in Decimal; Sharpe/Sortino/vol f64. |
| T11 | DONE | M3 | `crates/strategies` | deterministic strategy tests pass | trait + 4 baselines (incl. dca_sol) + regime scaffold + trend_alloc_v1 + threshold_rebalance_v1. |
| T12 | DONE | M2/M3 | `crates/cli` | `cargo run -p cli -- demo` runs | deterministic demo wiring; emits validated RunResult. |
| T13 | DONE | M0–M3 | (gates) | fmt+clippy+test green | fmt/clippy clean; demo byte-identical. |
| T14 | DONE | M0–M3 | (review) | verification fan-out recorded | 6-lens adversarial workflow → PASS; 14 findings fixed/accepted (review-packet). |
| M4·S1–S11 | DONE | M4 | `crates/sweep` (+ portfolio/results/research-core additive), `schemas/sweep-report.schema.json` | parallel==sequential byte-identical {1,2,3,7,8}; repeated identical; holdout sealed | Plan: [m4-sweep.md](m4-sweep.md). Deterministic sweep core (S1–S7), walk-forward (S8), holdout gate (S9), cost/fee sensitivity (S10), advancement report + schema (S11) — all adversarially reviewed. **Verified against source 2026-07-06 (audit in worklog): all claims confirmed; one S9 nuance → card M4-C1.** Workspace green at `df18267`: **272 tests, 0 failed**. **GATE DECLARED 2026-07-09 (card M4-C9): 301 tests, 0 failed; demo `ae064f79…`; sweep `7ad3df7d…`.** |

---

## M4 remaining — task cards (execute in order, one card per fresh session)

These cards replace the former single subtask **S12** (a 2026-07-06 audit found it underspecified: no
`SweepSpec`/`run_sweep`/TOML-`Deserialize` existed, and `strategy-lab.example.toml` had no
`[walk_forward]`/`[advancement]`/grid blocks). Every signature quoted below was **copied verbatim from
source on 2026-07-06 at commit `df18267`** — if what you find differs, escalate, don't adapt.

**How to execute a card (applies to every card):**
- Fresh session. Do only what the card says. Do not open files the card doesn't name.
- After the card's gate passes: flip this card's Status to DONE in this file and append one line to
  `plans/worklog.md` (these two bookkeeping edits are expected on every card and don't count against
  its file budget). Do **not** `git commit` or `git push` — the operator commits.
- The full-workspace gate for every card is:
  `cargo fmt --all --check` && `cargo clippy --all-targets --all-features -- -D warnings` &&
  `cargo test --workspace --all-features` (expect **0 failed**).
- Card snippets are correct but not always rustfmt-clean as pasted (some quoted lines exceed the
  100-column width). After typing in a card's code, run `cargo fmt --all` once BEFORE checking the
  gate — reflowing is expected and is not a deviation from the card.

**Guardrails (restated on each card; violating any one = STOP):**
- Money/thresholds/ordering keys are `rust_decimal::Decimal` or integers — **never `f64`** (f64 is
  display-only stats, already normalized to `None`).
- Parallel output must be **byte-identical** to sequential and to repeated runs.
- **No new dependencies** (no crates added to any `Cargo.toml` beyond what a card explicitly lists —
  and cards only ever add the *internal* `sweep = { workspace = true }` to `crates/cli`).
- **No execution code**: no keys, signing, submission, RPC, network, or HTTP — anywhere.
- **The holdout stays sealed**: never call `evaluate_on_holdout`, never wire it into any CLI path,
  never add a holdout accessor to `DevValidation`.
- Never edit `schemas/*.json`, `plans/master-plan.md`, or anything under `fixtures/`.

**Escalate and STOP (report in plans/worklog.md instead of improvising) if:**
- a file, line, or signature named on the card doesn't match what you find;
- any pre-existing test fails, or a gate fails for a reason unrelated to your change;
- the task seems to need a file the card doesn't list;
- anything ambiguous touches money types, determinism, the holdout, or schemas.

---

### M4-C1 — Add the missing `no_run` canary to the `Sealed` no-forgery doctest — `DONE`

**Goal.** Restore the S9-claimed property "every `compile_fail` doctest has a `no_run` canary proving
it fails only for the intended reason" (serves M4 gate: *untouched holdout remains untouched* — keeps
the seal's type-level proofs honest). Audit found 2 of 3 canaries present; the `Sealed` one is missing.

**Files.** `crates/sweep/src/partition.rs` — only this file.

**Current state (verbatim, partition.rs:349-361).**
```rust
/// The sealed holdout. The only constructor is [`PartitionedBars::seal_holdout`] and the only reader
/// is [`evaluate_on_holdout`], which consumes it **by value** — so the holdout is read at most once,
/// and never by code that holds only a [`DevValidation`].
///
/// A seal cannot be forged from outside the crate: the field is private and its type is private, so
/// only `seal_holdout` can mint one.
///
/// ```compile_fail
/// let _forged = sweep::Sealed { holdout: unimplemented!() }; // private field → does not compile
/// ```
pub struct Sealed {
    holdout: Holdout,
}
```
`Sealed`'s pub methods (partition.rs:373-396): `holdout_read_count(&self) -> u32`,
`holdout_digest(&self) -> u64`, `holdout_len(&self) -> usize`.

**Steps.**
1. In the doc comment above, immediately after the closing ```` /// ``` ```` of the `compile_fail`
   block and before `pub struct Sealed {`, insert exactly:
   ```rust
   ///
   /// The audit hooks on a legitimately obtained seal compile and type-check (a canary proving the
   /// block above fails only on the private field, not on a naming error):
   ///
   /// ```no_run
   /// let sealed: sweep::Sealed = unimplemented!();
   /// let _reads = sealed.holdout_read_count();
   /// let _digest = sealed.holdout_digest();
   /// let _len = sealed.holdout_len();
   /// ```
   ```
   (This mirrors the existing `DevValidation` canary pattern at partition.rs:295-300.)
2. Run the gate.

**Gate.**
- `cargo test -p sweep --doc` → **6 passed** (was 5; the new canary is the 6th), 0 failed.
- Full-workspace gate (fmt + clippy + test) green, 0 failed.

**Guardrails.** Decimal/integer only for money (no f64); parallel==sequential byte-identical; no new
dependencies; no execution/signing/RPC code; holdout stays sealed — `evaluate_on_holdout` is M5-only
and never wired into the M4 CLI; no schema or master-plan edits.

**Escalate-if.** The doc comment at partition.rs:349-361 doesn't match the verbatim block above; the
doctest count is not 5 before / 6 after; any other test fails; anything else needs changing.

---

### M4-C2 — `sweep::spec`: parse the strategy-lab TOML + add its missing blocks — `DONE`

**Goal.** Give the sweep engine a config-file front door: add the `[walk_forward]`, `[advancement]`,
and grid-array keys to the strategy-lab template, and a `SweepSpec` that parses them (serves M4
deliverable *parameter sweeps for the MVP strategy families*; unblocks C5–C7). This also discharges
questions.md Q5's "illustrative defaults in config" promise.

**Files.**
1. `config/strategies/strategy-lab.example.toml`
2. `crates/sweep/src/spec.rs` (new)
3. `crates/sweep/src/lib.rs`

**Current state (verified).** The template ends with the `[threshold_rebalance_v1]` table whose last
line is `rebalance_band = "0.10"             # rebalance only when |current − target| > band`. It has
NO `[walk_forward]`, NO `[advancement]`, NO grid arrays. Its `[partitions]` table is:
```toml
[partitions]
development = { start = "2021-01-01", end = "2023-12-31" }
validation  = { start = "2024-01-01", end = "2024-12-31" }
holdout     = { start = "2025-01-01", end = "2025-12-31" }
```
`crates/sweep/src` has **no** `Deserialize` anywhere; the `toml` dep in `crates/sweep/Cargo.toml` is
declared but unused (this card is its intended user — do NOT add any dep).
Existing types this card maps onto (verbatim):
```rust
// crates/sweep/src/config.rs:54
pub fn by_date(val_start: &str, holdout_start: &str) -> Result<Self, DateParseError>   // on PartitionSpec
// crates/sweep/src/window.rs:105-111
pub fn new(kind: WindowKind, train_len: usize, test_len: usize, step: usize, embargo: usize) -> Result<Self, WindowError>   // on WalkForward
// crates/sweep/src/window.rs:32-40
pub enum WindowKind { #[default] Rolling, Anchored }
// crates/sweep/src/advance.rs:19-32
pub struct AdvancementThresholds { pub drawdown_budget: Decimal, pub turnover_budget: Decimal,
    pub baseline_margin: Decimal, pub dispersion_budget: Decimal, pub neighbor_tolerance: Decimal,
    pub min_windows: u32 }
// crates/sweep/src/param.rs:95-107
pub enum ParamGrid {
    TrendAlloc { sma_periods: Vec<usize>, weights_above: Vec<Decimal>, weights_below: Vec<Decimal> },
    ThresholdRebalance { target_sol_weights: Vec<Decimal>, bands: Vec<Decimal> },
}
```
`crates/sweep/src/lib.rs:28-38` lists modules alphabetically: `advance, baseline, cell, config,
parallel, param, partition, report, sensitivity, turnover, window`.

**Steps.**
1. In the template, inside `[trend_alloc_v1]` immediately after the line `weight_below = "0.00"`, add:
   ```toml
   # M4 sweep grid axes (sets of values to sweep). Illustrative, NOT tuned (Q4/Q5).
   sma_periods = [20, 50]
   weights_above = ["0.75", "0.50"]
   weights_below = ["0.00"]
   ```
2. Inside `[threshold_rebalance_v1]` immediately after the `rebalance_band = "0.10"` line, add:
   ```toml
   # M4 sweep grid axes (sets of values to sweep). Illustrative, NOT tuned (Q4/Q5).
   target_sol_weights = ["0.25", "0.50", "0.75"]
   rebalance_bands = ["0.05", "0.10"]
   ```
3. At the end of the template file, append:
   ```toml
   # ── walk-forward schedule (M4) ───────────────────────────────────────────────
   # Illustrative defaults (Q5); the operator must FREEZE real values before M5.
   # kind = "rolling" (fixed-length train, de-weights stale regimes) or "anchored".
   [walk_forward]
   kind = "rolling"
   train_len = 365
   test_len = 90
   step = 90
   embargo = 5

   # ── advancement thresholds (M4 robustness battery) ───────────────────────────
   # Illustrative budgets, clearly NOT tuned (Q5); freeze before the M5 decision.
   # Decimal strings; turnover budget is against the UN-annualized ratio.
   [advancement]
   drawdown_budget = "0.35"
   turnover_budget = "12"
   baseline_margin = "0"
   dispersion_budget = "0.40"
   neighbor_tolerance = "0.15"
   min_windows = 3
   ```
4. Create `crates/sweep/src/spec.rs` with exactly this shape (fill in the obvious bodies; hand-rolled
   error in the house `WindowError`/`SimError` style):
   ```rust
   //! Sweep run specification parsed from the strategy-lab TOML template (M4 card C2).
   //! Read-only research config: no secrets, no network. Unknown TOML keys are ignored
   //! (the template also carries single-value demo keys the sweep does not use).

   use crate::advance::AdvancementThresholds;
   use crate::config::PartitionSpec;
   use crate::param::ParamGrid;
   use crate::window::{WalkForward, WindowError, WindowKind};
   use research_core::{DateParseError, Decimal};
   use serde::Deserialize;

   #[derive(Debug, Clone, Deserialize)]
   struct SweepSpecToml {
       allowlist_version: String,
       partitions: PartitionsToml,
       walk_forward: WalkForwardToml,
       advancement: AdvancementToml,
       trend_alloc_v1: TrendAllocToml,
       threshold_rebalance_v1: ThresholdRebalanceToml,
   }
   #[derive(Debug, Clone, Deserialize)]
   struct PartitionsToml { validation: DateRangeToml, holdout: DateRangeToml }
   #[derive(Debug, Clone, Deserialize)]
   struct DateRangeToml { start: String }
   #[derive(Debug, Clone, Deserialize)]
   struct WalkForwardToml { kind: String, train_len: usize, test_len: usize, step: usize, embargo: usize }
   #[derive(Debug, Clone, Deserialize)]
   struct AdvancementToml {
       drawdown_budget: Decimal, turnover_budget: Decimal, baseline_margin: Decimal,
       dispersion_budget: Decimal, neighbor_tolerance: Decimal, min_windows: u32,
   }
   #[derive(Debug, Clone, Deserialize)]
   struct TrendAllocToml { enabled: bool, sma_periods: Vec<usize>, weights_above: Vec<Decimal>, weights_below: Vec<Decimal> }
   #[derive(Debug, Clone, Deserialize)]
   struct ThresholdRebalanceToml {
       enabled: bool,
       target_sol_weights: Vec<Decimal>,
       #[serde(rename = "rebalance_bands")]
       bands: Vec<Decimal>,
   }

   /// A fully resolved sweep specification (validated views of the TOML).
   #[derive(Debug, Clone)]
   pub struct SweepSpec {
       pub allowlist_version: String,
       pub partition: PartitionSpec,
       pub walk_forward: WalkForward,
       pub thresholds: AdvancementThresholds,
       /// Enabled family grids, fixed order: trend_alloc_v1 first, then threshold_rebalance_v1.
       pub grids: Vec<ParamGrid>,
   }

   /// Why a spec failed to parse/resolve.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum SpecError {
       Toml(String),
       Date(DateParseError),
       Window(WindowError),
       UnknownWalkForwardKind(String),
   }
   // impl Display + std::error::Error for SpecError (match, one line per variant).

   impl SweepSpec {
       /// Parse and resolve the strategy-lab TOML. Pure; no I/O.
       pub fn from_toml_str(s: &str) -> Result<Self, SpecError> {
           let t: SweepSpecToml = toml::from_str(s).map_err(|e| SpecError::Toml(e.to_string()))?;
           let kind = match t.walk_forward.kind.as_str() {
               "rolling" => WindowKind::Rolling,
               "anchored" => WindowKind::Anchored,
               other => return Err(SpecError::UnknownWalkForwardKind(other.to_string())),
           };
           let walk_forward = WalkForward::new(kind, t.walk_forward.train_len, t.walk_forward.test_len,
               t.walk_forward.step, t.walk_forward.embargo).map_err(SpecError::Window)?;
           let partition = PartitionSpec::by_date(&t.partitions.validation.start, &t.partitions.holdout.start)
               .map_err(SpecError::Date)?;
           let thresholds = AdvancementThresholds { /* copy the six fields from t.advancement */ };
           let mut grids = Vec::new();
           if t.trend_alloc_v1.enabled {
               grids.push(ParamGrid::TrendAlloc { sma_periods: t.trend_alloc_v1.sma_periods,
                   weights_above: t.trend_alloc_v1.weights_above, weights_below: t.trend_alloc_v1.weights_below });
           }
           if t.threshold_rebalance_v1.enabled {
               grids.push(ParamGrid::ThresholdRebalance { target_sol_weights: t.threshold_rebalance_v1.target_sol_weights,
                   bands: t.threshold_rebalance_v1.bands });
           }
           Ok(Self { allowlist_version: t.allowlist_version, partition, walk_forward, thresholds, grids })
       }
   }
   ```
5. Unit tests in `spec.rs` (`#[cfg(test)] mod tests`, use `rust_decimal_macros::dec`):
   - `example_template_parses_and_resolves`: `include_str!("../../../config/strategies/strategy-lab.example.toml")`
     → `from_toml_str` Ok; assert `walk_forward == WalkForward::new(WindowKind::Rolling, 365, 90, 90, 5).unwrap()`;
     `thresholds.min_windows == 3`; `thresholds.drawdown_budget == dec!(0.35)`; `grids.len() == 2`;
     `grids[0].points().len() == 4`; `grids[1].points().len() == 6`;
     `partition == PartitionSpec::by_date("2024-01-01", "2025-01-01").unwrap()`.
   - `unknown_walk_forward_kind_is_rejected`: feed a minimal TOML string with `kind = "sideways"` →
     `Err(SpecError::UnknownWalkForwardKind(_))`.
   - `disabled_family_is_omitted`: minimal TOML with `enabled = false` on one family → `grids.len() == 1`.
6. In `crates/sweep/src/lib.rs`: add `pub mod spec;` alphabetically (between `pub mod sensitivity;`
   and `pub mod turnover;`) and add `pub use spec::{SpecError, SweepSpec};` to the re-export block
   (after the `sensitivity` re-export).
7. Run the gate.

**Gate.**
- `cargo test -p sweep spec` → the 3 new tests pass.
- Full-workspace gate green, 0 failed. `cargo run -p cli -- demo` still byte-identical (run twice,
  compare `shasum`) — the demo does not read this template, so it must be unaffected.

**Guardrails.** Decimal/integer only for money (weights/budgets parse into `Decimal`, never f64); no
new dependencies (`serde`/`toml` are already sweep deps); parallel==sequential byte-identical
untouched; no execution/signing/RPC code; holdout stays sealed (`evaluate_on_holdout` never called);
no schema or master-plan edits.

**Escalate-if.** The template's `[partitions]`/family tables differ from the verbatim block above; any
quoted signature differs; `toml::from_str` cannot deserialize `Decimal` from the quoted strings (do
NOT switch weights to bare numbers — stop and report); any pre-existing test fails.

---

### M4-C3 — `sweep::runner` part 1: canonical cell enumeration — `DONE`

**Goal.** One fixed, documented enumeration of the whole sweep — window (outer) → cost scenario →
param point (inner), index preassigned before any thread spawns (serves M4 gate: *parallel and
sequential results match* — order must be scheduling-independent).

**Files.**
1. `crates/sweep/src/runner.rs` (new)
2. `crates/sweep/src/lib.rs`

**Current state (verbatim signatures this card composes).**
```rust
// crates/sweep/src/parallel.rs:35-41
pub struct SweepCell { pub index: usize, pub point: ParamPoint, pub cost: CostModel, pub test: Range<usize> }
// crates/sweep/src/sensitivity.rs:57-61
pub struct CostScenario { pub id: ScenarioId, pub cost: CostModel }
// crates/sweep/src/sensitivity.rs:23-40 — ScenarioId variants (Copy, Eq):
// BeforeCosts, Base, Doubled, DoubledSlippage, DoubledPriority
// crates/sweep/src/window.rs:47-51
pub struct Window { pub train: Range<usize>, pub test: Range<usize> }
// crates/sweep/src/param.rs — ParamPoint (Clone, Eq); ParamPoint::family(&self) -> &'static str;
// ParamPoint::param_id(&self) -> String
```

**Steps.**
1. Create `crates/sweep/src/runner.rs` with:
   ```rust
   //! Sweep orchestration (M4 cards C3–C5): canonical enumeration → parallel evaluation →
   //! evidence aggregation → SweepReport. Deterministic by construction: the cell order is fixed
   //! before any thread spawns (m4-sweep.md §5: window → cost-scenario → param order).

   use crate::parallel::SweepCell;
   use crate::param::ParamPoint;
   use crate::sensitivity::{CostScenario, ScenarioId};
   use crate::window::Window;

   /// Where a cell sits in the canonical enumeration. `point_index` indexes the caller's
   /// concatenated `points` list (grids in fixed family order).
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct CellKey {
       pub window_index: usize,
       pub scenario: ScenarioId,
       pub point_index: usize,
   }

   /// Materialize the full work list in canonical order: window (outer) → scenario → point (inner).
   /// `cells[i].index == i` and `keys[i]` describes `cells[i]`.
   #[must_use]
   pub fn enumerate_cells(
       points: &[ParamPoint],
       scenarios: &[CostScenario],
       windows: &[Window],
   ) -> (Vec<SweepCell>, Vec<CellKey>) {
       let n = windows.len() * scenarios.len() * points.len();
       let mut cells = Vec::with_capacity(n);
       let mut keys = Vec::with_capacity(n);
       for (wi, w) in windows.iter().enumerate() {
           for s in scenarios {
               for (pi, p) in points.iter().enumerate() {
                   cells.push(SweepCell {
                       index: cells.len(),
                       point: p.clone(),
                       cost: s.cost.clone(),
                       test: w.test.clone(),
                   });
                   keys.push(CellKey { window_index: wi, scenario: s.id, point_index: pi });
               }
           }
       }
       (cells, keys)
   }
   ```
2. Unit tests in `runner.rs` (`#[cfg(test)] mod tests`): build 2 windows (`Window { train: 0..4, test: 5..8 }`,
   `Window { train: 3..7, test: 8..11 }`), 2 scenarios (`cost_scenarios(&some CostModel)` truncated to
   the first 2), and a 2-point grid (`ParamGrid::ThresholdRebalance { target_sol_weights:
   vec![dec!(0.25), dec!(0.5)], bands: vec![dec!(0.1)] }.points()`), then assert:
   - `cells.len() == 8` and `keys.len() == 8`;
   - `cells[i].index == i` for all i;
   - the exact key sequence: (w0,s0,p0), (w0,s0,p1), (w0,s1,p0), (w0,s1,p1), (w1,s0,p0), … (w1,s1,p1);
   - `cells[i].test` equals the window's `test` range and `cells[i].cost` the scenario's cost.
3. In `crates/sweep/src/lib.rs`: add `pub mod runner;` alphabetically (between `report` and
   `sensitivity`) and `pub use runner::{enumerate_cells, CellKey};` in the re-export block.
4. Run the gate.

**Gate.**
- `cargo test -p sweep runner` → new tests pass.
- Full-workspace gate green, 0 failed.

**Guardrails.** Decimal/integer only for money (no f64); enumeration order fixed before any thread —
parallel must stay byte-identical to sequential; no new dependencies; no execution/signing/RPC code;
holdout stays sealed (`evaluate_on_holdout` never called or referenced here); no schema/master-plan edits.

**Escalate-if.** Any quoted signature differs; `pub use runner::…` collides with an existing name;
any pre-existing test fails.

---

### M4-C4 — `sweep::runner` part 2: aggregate cell results into `CandidateEvidence` — `DONE`

**Goal.** Turn raw per-cell results into the exact evidence `evaluate_candidate` consumes — means,
fold dispersion, worst drawdown/turnover, per-window baseline floors, neighbor degradation — all
`Decimal` (serves M4 deliverables *strategy-family comparison* and *turnover and fee-sensitivity
reporting*; feeds the *rejection report*).

**Files.**
1. `crates/sweep/src/runner.rs`
2. `crates/sweep/src/lib.rs`

**Current state (verbatim, the target struct and its semantics — advance.rs:37-60).**
```rust
pub struct CandidateEvidence {
    pub candidate_label: String,            // e.g. "threshold_rebalance_v1/target=0.5;band=0"
    pub max_drawdown: Decimal,              // worst drawdown over the evaluation ([0,1] fraction)
    pub turnover: Decimal,                  // un-annualized turnover ratio
    pub baseline_margin: Decimal,           // candidate_return - best_baseline_return, base costs, OOS
    pub doubled_return: Decimal,            // candidate return under DOUBLED costs
    pub doubled_baseline_floor: Decimal,    // cost-matched baseline floor under doubled costs
    pub fold_dispersion: Decimal,           // max − min of OOS fold returns
    pub neighbor_degradation: Decimal,      // worst return degradation at a neighboring grid point
    pub valid_windows: u32,
}
```
Predicate directions in `evaluate_candidate` (advance.rs:128-188, verified): fails when
`baseline_margin < th.baseline_margin`; `doubled_return <= doubled_baseline_floor`;
`fold_dispersion > th.dispersion_budget`; `neighbor_degradation > th.neighbor_tolerance`;
`max_drawdown > th.drawdown_budget`; `turnover > th.turnover_budget`; hard-stop when
`valid_windows < th.min_windows`.
`CellResult` fields used here (cell.rs:24-43): `total_return: Decimal`, `max_drawdown: Decimal`,
`turnover: Decimal`. `ParamGrid::points()` returns the fixed-order Cartesian product;
`ParamPoint::{family, param_id}` as in C3. `Decimal` is `Copy + Ord`; use
`Decimal::from(count as u64)` for divisors.

**Steps.**
1. In `runner.rs`, add (exact formulas; all Decimal, division only by window count):
   ```rust
   /// Aggregate raw cell results into one CandidateEvidence per param point.
   ///
   /// Definitions (all exact Decimal; f64 never enters):
   /// - candidate returns come from Base-scenario cells; means divide by the window count;
   /// - fold_dispersion = max − min of the Base-scenario window returns (0 with < 2 windows);
   /// - max_drawdown / turnover = the WORST (max) across windows (conservative);
   /// - baseline_margin = mean(Base returns) − mean(base_floors);
   /// - doubled_return = mean(Doubled-scenario returns); doubled_baseline_floor = mean(doubled_floors);
   /// - neighbor_degradation = max over axis-neighbors of (own mean − neighbor mean), floored at 0;
   ///   axis-neighbors differ by exactly one position on exactly one grid axis;
   /// - valid_windows = the window count (run_cells is total — any SimError aborts the sweep).
   #[must_use]
   pub fn aggregate_evidence(
       grids: &[ParamGrid],
       keys: &[CellKey],
       results: &[CellResult],
       n_windows: usize,
       base_floors: &[Decimal],      // per-window best cost-matched baseline return, Base costs
       doubled_floors: &[Decimal],   // per-window best cost-matched baseline return, Doubled costs
   ) -> Vec<CandidateEvidence>
   ```
   Implementation outline (write it exactly like this):
   a. `let points: Vec<ParamPoint> = grids.iter().flat_map(|g| g.points()).collect();`
   b. For each point index `pi`, collect `base_returns[pi]: Vec<Decimal>` and
      `doubled_returns[pi]: Vec<Decimal>` ordered by `window_index`, plus running `max` of
      `max_drawdown` and `turnover` over Base cells, by one pass over `keys.iter().zip(results)`
      matching `k.scenario == ScenarioId::Base` / `ScenarioId::Doubled`.
   c. `let mean = |xs: &[Decimal]| if xs.is_empty() { Decimal::ZERO } else { xs.iter().copied().sum::<Decimal>() / Decimal::from(xs.len() as u64) };`
   d. `fold_dispersion` = `max − min` via `iter().copied().max()/min()` (`Decimal: Ord`), `ZERO` if empty.
   e. Mean floors: `mean(base_floors)`, `mean(doubled_floors)`.
   f. Neighbor means: first compute every point's Base mean return into `means: Vec<Decimal>` (same
      order as `points`). Then use these two private helpers verbatim (the row-major decomposition
      matches `ParamGrid::points()` declared order — last axis fastest — verified against param.rs):
      ```rust
      /// Concatenated-list indices of `pi`'s axis neighbors (±1 on exactly one grid axis).
      fn neighbor_indices(grids: &[ParamGrid], pi: usize) -> Vec<usize> {
          let mut offset = 0;
          for g in grids {
              let len = g.points().len();
              if pi < offset + len {
                  return in_grid_neighbors(g, pi - offset).into_iter().map(|n| n + offset).collect();
              }
              offset += len;
          }
          Vec::new()
      }

      /// Axis neighbors of the in-grid row-major index `local` (last axis fastest).
      fn in_grid_neighbors(grid: &ParamGrid, local: usize) -> Vec<usize> {
          let dims: Vec<usize> = match grid {
              ParamGrid::TrendAlloc { sma_periods, weights_above, weights_below } => {
                  vec![sma_periods.len(), weights_above.len(), weights_below.len()]
              }
              ParamGrid::ThresholdRebalance { target_sol_weights, bands } => {
                  vec![target_sol_weights.len(), bands.len()]
              }
          };
          let mut idx = vec![0usize; dims.len()];
          let mut rem = local;
          for a in (0..dims.len()).rev() {
              idx[a] = rem % dims[a];
              rem /= dims[a];
          }
          let compose = |ix: &[usize]| ix.iter().zip(&dims).fold(0, |acc, (i, d)| acc * d + i);
          let mut out = Vec::new();
          for a in 0..dims.len() {
              if idx[a] > 0 {
                  let mut nix = idx.clone();
                  nix[a] -= 1;
                  out.push(compose(&nix));
              }
              if idx[a] + 1 < dims[a] {
                  let mut nix = idx.clone();
                  nix[a] += 1;
                  out.push(compose(&nix));
              }
          }
          out
      }
      ```
      then, per point: `neighbor_degradation = neighbor_indices(grids, pi).into_iter()
      .map(|ni| means[pi] - means[ni]).max().unwrap_or(Decimal::ZERO).max(Decimal::ZERO);`.
   g. `candidate_label = format!("{}/{}", point.family(), point.param_id())`.
   h. `valid_windows = u32::try_from(n_windows).unwrap_or(u32::MAX)`.
   Evidence order = the concatenated point order (already canonical — do not sort here; the report
   sorts verdicts itself).
2. Unit tests in `runner.rs` with hand-built `CellResult`s (construct the struct literally; set the
   fields you assert, zero the rest, `None` for the five `Option<f64>` stats):
   - `evidence_means_and_dispersion_are_exact`: 1-point grid, 2 windows, scenarios Base+Doubled;
     Base returns 0.10 / 0.30 → mean `0.2`, dispersion `0.2`; Doubled returns 0.05 / 0.15 →
     doubled_return `0.1`; base_floors [0.05, 0.05] → baseline_margin `0.15`; drawdowns 0.1 / 0.4 →
     max_drawdown `0.4`; turnovers 1 / 3 → turnover `3`; valid_windows 2. Assert every field with `dec!`.
   - `neighbor_degradation_picks_the_worst_axis_neighbor`: ThresholdRebalance grid
     `target_sol_weights: [0.25, 0.5, 0.75] × bands: [0.1]`, 1 window, Base returns 0.1 / 0.5 / 0.2
     → middle point degradation `0.4` (vs the 0.1 neighbor), edge points floor at their single
     neighbor; and a *better* neighbor yields `0` (the `.max(ZERO)` floor).
   - `empty_windows_yield_zeroed_insufficient_evidence`: `n_windows = 0` → all-zero evidence,
     `valid_windows == 0` (which `evaluate_candidate` hard-stops as InsufficientData).
3. Add `aggregate_evidence` to the `pub use runner::…` line in `lib.rs`.
4. Run the gate.

**Gate.**
- `cargo test -p sweep runner` → all C3+C4 tests pass.
- Full-workspace gate green, 0 failed.

**Guardrails.** Decimal/integer only — the ONLY division is by the window count, in Decimal; never
f64 for any evidence field; deterministic iteration order only (no HashMap — use Vec/BTreeMap);
no new dependencies; no execution/signing/RPC code; holdout stays sealed; no schema/master-plan edits.

**Escalate-if.** `CandidateEvidence`/`CellResult` fields differ from the verbatim blocks; the
row-major reconstruction disagrees with `ParamGrid::points()` order in your neighbor test (that means
the formula note is wrong — stop, don't guess); any pre-existing test fails.

---

### M4-C5 — `sweep::runner` part 3: `run_sweep` end-to-end + report-level determinism test — `DONE`

**Goal.** The single orchestration entry point: spec + bars → sealed partitions → windows → cells →
parallel evaluation → evidence → verdicts → `SweepReport` (serves ALL THREE M4 gate criteria:
parallel==sequential, repeated identical, holdout untouched).

**Files.**
1. `crates/sweep/src/runner.rs`
2. `crates/sweep/src/lib.rs`
3. `crates/sweep/tests/sweep_runner.rs` (new)

**Current state (verbatim signatures this card composes).**
```rust
// partition.rs:160  (on PartitionedBars)
pub fn from_spec(bars: Vec<Bar>, spec: &PartitionSpec) -> Result<Self, PartitionError>
// partition.rs:266-267
pub fn seal_holdout(self) -> (DevValidation, Sealed)
// partition.rs:343-344  (on DevValidation)
pub fn walk_forward_windows(&self, wf: &WalkForward) -> Vec<Window>
// partition.rs:322-323
pub fn dev_validation(&self) -> &[Bar]
// sensitivity.rs:94-95 — ladder order pinned by test: [BeforeCosts(zero), Base, Doubled, DoubledSlippage, DoubledPriority]
pub fn cost_scenarios(base: &CostModel) -> Vec<CostScenario>
// parallel.rs:121-127
pub fn run_cells(cells: &[SweepCell], bars: &[Bar], initial_cash_usdc: Decimal,
    periods_per_year: f64, parallelism: Parallelism) -> Result<Vec<CellResult>, SimError>
// baseline.rs:96-101
pub fn eval_all_baselines(bars: &[Bar], cost: &CostModel, initial_cash_usdc: Decimal,
    periods_per_year: f64) -> Result<Vec<(BaselineId, CellResult)>, SimError>
// baseline.rs:115-116
pub fn best_baseline_return(results: &[(BaselineId, CellResult)]) -> Decimal
// sensitivity.rs:82-83
pub fn scale_cost_model(base: &CostModel, num: u32, den: u32) -> CostModel
// advance.rs:128
pub fn evaluate_candidate(ev: &CandidateEvidence, th: &AdvancementThresholds) -> CandidateVerdict
// report.rs:63-67  (on SweepReport)
pub fn new(thresholds: &AdvancementThresholds, trial_count: u32, mut verdicts: Vec<CandidateVerdict>) -> Self
// Sealed audit hooks: holdout_read_count() -> u32, holdout_digest() -> u64, holdout_len() -> usize
```

**Steps.**
1. In `runner.rs`, add:
   ```rust
   /// Everything a sweep produces. The seal is returned UNCONSUMED so callers can PROVE the holdout
   /// was never read (`sealed.holdout_read_count() == 0`). M4 code must never consume it.
   #[derive(Debug)]
   pub struct SweepOutcome { pub report: SweepReport, pub sealed: Sealed }

   /// Why a sweep failed.
   #[derive(Debug)]
   pub enum SweepError { Partition(PartitionError), Sim(SimError) }
   // impl Display + std::error::Error; impl From<PartitionError> and From<SimError>.

   /// Run the full deterministic sweep. Scenario set = the first three rungs of the cost ladder:
   /// BeforeCosts, Base, Doubled (m4-sweep.md §13). `trial_count` = total cells evaluated.
   pub fn run_sweep(
       spec: &SweepSpec,
       bars: Vec<Bar>,
       base_cost: &CostModel,
       initial_cash_usdc: Decimal,
       periods_per_year: f64,
       parallelism: Parallelism,
   ) -> Result<SweepOutcome, SweepError> {
       let (dev, sealed) = PartitionedBars::from_spec(bars, &spec.partition)?.seal_holdout();
       let windows = dev.walk_forward_windows(&spec.walk_forward);
       let ladder = cost_scenarios(base_cost);
       // BeforeCosts, Base, Doubled — ladder order is test-pinned. BeforeCosts cells are evaluated
       // and counted in trial_count but not read by aggregate_evidence: they are the "same strategy
       // before costs" sensitivity rung m4-sweep.md §13 mandates, kept for M5's FeeSensitivity use
       // and honest multiple-testing accounting. Deliberate — do not narrow to [1..3].
       let scenarios = &ladder[..3];
       let points: Vec<ParamPoint> = spec.grids.iter().flat_map(|g| g.points()).collect();
       let (cells, keys) = enumerate_cells(&points, scenarios, &windows);
       let results = run_cells(&cells, dev.dev_validation(), initial_cash_usdc, periods_per_year, parallelism)?;
       let doubled = scale_cost_model(base_cost, 2, 1);
       let mut base_floors = Vec::with_capacity(windows.len());
       let mut doubled_floors = Vec::with_capacity(windows.len());
       for w in &windows {
           let slice = &dev.dev_validation()[w.test.clone()];
           base_floors.push(best_baseline_return(&eval_all_baselines(slice, base_cost, initial_cash_usdc, periods_per_year)?));
           doubled_floors.push(best_baseline_return(&eval_all_baselines(slice, &doubled, initial_cash_usdc, periods_per_year)?));
       }
       let evidence = aggregate_evidence(&spec.grids, &keys, &results, windows.len(), &base_floors, &doubled_floors);
       let verdicts = evidence.iter().map(|ev| evaluate_candidate(ev, &spec.thresholds)).collect();
       let trial_count = u32::try_from(cells.len()).unwrap_or(u32::MAX);
       Ok(SweepOutcome { report: SweepReport::new(&spec.thresholds, trial_count, verdicts), sealed })
   }
   ```
2. Extend the `lib.rs` runner re-export to `pub use runner::{aggregate_evidence, enumerate_cells, run_sweep, CellKey, SweepError, SweepOutcome};`.
3. Create `crates/sweep/tests/sweep_runner.rs`:
   - Series builder (copy the proven sawtooth from tests/holdout_sealing.rs:24-39):
     `close = 100 + (i % 7) * 3 + (i / 7) * 2` over `n = 160` daily bars from unix 0.
   - Spec built directly (fields are pub — no TOML here). `SweepSpec` has FIVE fields; supply them
     all: `allowlist_version: "test".to_string()` (unread by `run_sweep`, required by the literal);
     `PartitionSpec::by_index(100, 130)`; `WalkForward::new(WindowKind::Rolling, 20, 10, 10, 0).unwrap()`;
     thresholds `drawdown_budget 0.5, turnover_budget 50, baseline_margin -1, dispersion_budget 10, neighbor_tolerance 10, min_windows 1` (loose — verdict *content* is not this card's subject);
     grids: `TrendAlloc { sma_periods: vec![3, 5], weights_above: vec![dec!(0.75)], weights_below: vec![dec!(0)] }` and
     `ThresholdRebalance { target_sol_weights: vec![dec!(0.25), dec!(0.5)], bands: vec![dec!(0.05)] }`.
   - Cost = the fixture model `CostModel { dex_fee_bps: 5, slippage_bps: 20, base_fee_lamports: 5_000, priority_fee_lamports: 50_000 }`; cash `dec!(10000)`; ppy `365.0`.
   - Tests:
     a. `report_is_byte_identical_across_parallelism_and_repeats`: run Sequential twice, Threads(2),
        Threads(8) → all four `outcome.report.to_json()` strings equal.
     b. `holdout_is_never_read_and_survives_intact`: every outcome above has
        `sealed.holdout_read_count() == 0`, `holdout_len() == 30`, and equal `holdout_digest()`
        across two runs.
     c. `every_candidate_gets_exactly_one_verdict`: `report.verdicts.len() == 4` (2 + 2 points) and
        labels strictly increasing (sorted, unique).
     d. `report_validates_against_schema`: `jsonschema` is already a dev-dep; load the schema exactly
        as `tests/schema_validation.rs:15` does —
        `format!("{}/../../schemas/sweep-report.schema.json", env!("CARGO_MANIFEST_DIR"))` →
        `std::fs::read_to_string` → `serde_json::from_str` → `jsonschema::validator_for` (copy that
        file's helper shape) — and assert `outcome.report.to_value()` validates.
4. Run the gate.

**Gate.**
- `cargo test -p sweep --test sweep_runner` → 4 tests pass.
- `cargo test -p sweep` → all sweep tests (unit + determinism + holdout_sealing + walk_forward +
  schema_validation + sweep_runner + doctests) pass.
- Full-workspace gate green, 0 failed.

**Guardrails.** Decimal/integer only for money; the report must be byte-identical across
Sequential/Threads(k)/repeats — if test (a) fails, the BUG IS IN YOUR CODE (find the nondeterminism;
never weaken the assert); **`run_sweep` returns the seal unconsumed and must never call
`evaluate_on_holdout`**; no new dependencies; no execution/signing/RPC code; no schema/master-plan edits.

**Escalate-if.** Any composed signature differs; test (a) or (b) fails and you cannot find a cause in
code YOU wrote this session; `ladder[..3]` is not BeforeCosts/Base/Doubled (check the sensitivity
ladder test — if the order changed, stop); any pre-existing test fails.

---

### M4-C6 — CLI `machina sweep [--threads N] [--out PATH]` — `DONE`

**Goal.** The user-facing sweep runner: embedded templates → `SweepSpec` → `run_sweep` → canonical
`SweepReport` JSON on stdout or `--out` (serves M4 deliverables *parallel execution* and *canonical
result export*).

**Files.**
1. `crates/cli/Cargo.toml`
2. `crates/cli/src/main.rs`

**Current state (verbatim).** `crates/cli/Cargo.toml` `[dependencies]` lists exactly: research-core,
market-data, portfolio, metrics, strategies, results, rust_decimal (all `{ workspace = true }`) — no
`sweep`. The workspace root already declares `sweep = { path = "crates/sweep" }`. `main.rs` structure:
```rust
const ALLOWLIST_TEMPLATE: &str = include_str!("../../../config/tokens/allowlist.example.toml");
const PERIODS_PER_YEAR: f64 = 365.0;
fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("demo") => demo(),
        Some("--help" | "-h" | "help") | None => usage(),
        Some(other) => { eprintln!("unknown command: {other}\n"); usage(); std::process::exit(2); }
    }
}
```
The demo's cost model (main.rs, `demo()` and `demo_cost()` in tests) is
`CostModel { dex_fee_bps: 5, slippage_bps: 20, base_fee_lamports: 5_000, priority_fee_lamports: 50_000 }`
(mirrors `config/costs/solana-mainnet.example.toml`). Sweep API from C2/C5:
`SweepSpec::from_toml_str(&str) -> Result<SweepSpec, SpecError>`;
`run_sweep(&SweepSpec, Vec<Bar>, &CostModel, Decimal, f64, Parallelism) -> Result<SweepOutcome, SweepError>`;
`Parallelism::{Sequential, Threads(NonZeroUsize)}`; `SweepOutcome { report, sealed }`;
`report.to_json() -> String`; `sealed.holdout_read_count() -> u32`.

**Steps.**
1. In `crates/cli/Cargo.toml`, after the `results = { workspace = true }` line add
   `sweep = { workspace = true }`.
2. In `main.rs`: add
   `const STRATEGY_LAB_TEMPLATE: &str = include_str!("../../../config/strategies/strategy-lab.example.toml");`
   next to `ALLOWLIST_TEMPLATE`; add `use std::num::NonZeroUsize;` and
   `use sweep::{run_sweep, Parallelism, SweepSpec};` to the imports.
3. Add a `Some("sweep") => sweep_cmd(args),` arm to the `match` (before the help arm). Hand-rolled
   options loop (NO clap — D-0002):
   ```rust
   fn sweep_cmd(mut args: impl Iterator<Item = String>) {
       let mut threads: Option<usize> = None;
       let mut out: Option<String> = None;
       while let Some(a) = args.next() {
           match a.as_str() {
               "--threads" => {
                   let v = args.next().unwrap_or_else(|| { eprintln!("--threads needs a value"); std::process::exit(2) });
                   threads = Some(v.parse().unwrap_or_else(|_| { eprintln!("--threads must be a positive integer"); std::process::exit(2) }));
               }
               "--out" => out = Some(args.next().unwrap_or_else(|| { eprintln!("--out needs a path"); std::process::exit(2) })),
               other => { eprintln!("unknown sweep option: {other}"); std::process::exit(2); }
           }
       }
       let parallelism = match threads {
           None => Parallelism::Sequential,
           Some(0) => { eprintln!("--threads must be >= 1"); std::process::exit(2); }
           Some(n) => Parallelism::Threads(NonZeroUsize::new(n).expect("n >= 1")),
       };
       let json = sweep_report_json(parallelism);
       match out {
           Some(p) => { std::fs::write(&p, &json).expect("write report file"); eprintln!("wrote {p}"); }
           None => println!("{json}"),
       }
   }
   ```
4. Add the shared core (also used by C7's `sweep-verify`; stdout stays pure JSON — status goes to stderr):
   ```rust
   /// Run the canonical M4 sweep on the embedded templates + synthetic series; return report JSON.
   /// Proves the holdout stayed sealed before returning. NEVER calls evaluate_on_holdout (M5-only).
   fn sweep_report_json(parallelism: Parallelism) -> String {
       let allowlist = Allowlist::from_toml_str(ALLOWLIST_TEMPLATE).expect("allowlist template parses");
       allowlist.require("SOL").expect("SOL allowlisted");
       allowlist.require("USDC").expect("USDC allowlisted");
       let spec = SweepSpec::from_toml_str(STRATEGY_LAB_TEMPLATE).expect("strategy-lab template parses");
       let bars = sweep_series();
       validate_series_spacing(&bars, 86_400).expect("sweep series is valid and gap-free");
       let cost = CostModel { dex_fee_bps: 5, slippage_bps: 20, base_fee_lamports: 5_000, priority_fee_lamports: 50_000 };
       let outcome = run_sweep(&spec, bars, &cost, Decimal::from(10_000), PERIODS_PER_YEAR, parallelism)
           .expect("sweep runs");
       assert_eq!(outcome.sealed.holdout_read_count(), 0, "M4 must never read the holdout");
       outcome.report.to_json()
   }

   /// Deterministic synthetic daily series spanning the template's [partitions] dates
   /// (2021-01-01..2025-12-31 = 1826 bars). Same sawtooth family as the sweep test fixtures.
   /// NOT real market data (research demo only; real ingestion is the M5 data question, Q3).
   fn sweep_series() -> Vec<Bar> {
       const DAY: i64 = 86_400;
       const START: i64 = 1_609_459_200; // 2021-01-01T00:00:00Z
       (0..1826)
           .map(|i| {
               let p = Decimal::from(100 + (i as i64 % 7) * 3 + (i as i64 / 7) * 2);
               Bar { ts: Timestamp::from_unix(START + i as i64 * DAY), open: p, high: p, low: p, close: p, volume: Decimal::from(1_000) }
           })
           .collect()
   }
   ```
5. In `usage()`, add a line under the `machina demo` line:
   `machina sweep [--threads N] [--out PATH]    Deterministic parameter sweep (research only)`.
6. Tests (in the existing `#[cfg(test)] mod tests`):
   - `sweep_series_is_valid_and_spans_the_partitions`: `len() == 1826`;
     `validate_series_spacing(&bars, 86_400)` Ok; `bars[0].ts == Timestamp::from_unix(1_609_459_200)`.
   - `sweep_report_is_deterministic_and_parallel_equal`:
     `sweep_report_json(Parallelism::Sequential)` twice and
     `sweep_report_json(Parallelism::Threads(NonZeroUsize::new(2).unwrap()))` — all three strings equal.
7. Run the gate.

**Gate.**
- `cargo test -p cli` → all cli tests (6 existing + 2 new) pass.
- `cargo run -q -p cli -- sweep | shasum` run twice → identical hashes.
- `cargo run -q -p cli -- sweep --threads 8 | shasum` → same hash as sequential.
- `cargo run -q -p cli -- demo | shasum` twice → identical (demo untouched).
- Full-workspace gate green, 0 failed. No-execution-deps scan still prints OK (see handoff.md §Verified state).

**Guardrails.** Decimal/integer only for money; stdout carries ONLY the canonical JSON (all status to
stderr) so parallel==sequential stays byte-checkable; `--threads` may vary the schedule but NEVER the
bytes; no new external dependencies (`sweep` is an internal workspace path dep, explicitly allowed
here); no execution/signing/RPC code; **never call `evaluate_on_holdout` — the `assert_eq!(…, 0)` on
the read count is mandatory**; no schema/master-plan edits.

**Escalate-if.** `crates/cli/Cargo.toml` or the `main()` match differs from the verbatim block; C2/C5
exports are missing or differently named; the two shasum runs differ (nondeterminism — stop, report);
any pre-existing test fails.

---

### M4-C7 — CLI `machina sweep-verify` — `DONE`

**Goal.** A local, operator-runnable mirror of the CI determinism gate: run the sweep Sequential,
Threads(2), and Threads(8); exit 0 iff all three reports are byte-identical and the holdout was never
read (serves M4 gate criteria *parallel==sequential* and *repeated runs identical*).

**Files.** `crates/cli/src/main.rs` — only this file.

**Current state.** After C6, `main.rs` has the `sweep` arm, `sweep_report_json(Parallelism) -> String`
(which already asserts `holdout_read_count() == 0`), `usage()` with demo+sweep lines, and imports
`Parallelism`, `NonZeroUsize`.

**Steps.**
1. Add a `Some("sweep-verify") => sweep_verify(),` arm to the `match` in `main()` (after the `sweep` arm).
2. Add:
   ```rust
   /// Local mirror of the CI determinism gate: the canonical sweep must be byte-identical across
   /// Sequential and explicit thread counts. Exits non-zero on any mismatch.
   fn sweep_verify() {
       let sequential = sweep_report_json(Parallelism::Sequential);
       for n in [2usize, 8] {
           let parallel = sweep_report_json(Parallelism::Threads(NonZeroUsize::new(n).expect("n >= 1")));
           if parallel != sequential {
               eprintln!("sweep-verify: MISMATCH at {n} threads (parallel output != sequential)");
               std::process::exit(1);
           }
       }
       let repeat = sweep_report_json(Parallelism::Sequential);
       if repeat != sequential {
           eprintln!("sweep-verify: MISMATCH on repeated sequential run");
           std::process::exit(1);
       }
       println!("sweep-verify: OK — byte-identical across sequential, 2 and 8 threads, and repeat ({} bytes)", sequential.len());
   }
   ```
3. In `usage()`, add under the sweep line:
   `machina sweep-verify    Assert parallel == sequential byte-identical (CI-gate mirror)`.
4. Test (in `mod tests`): `sweep_verify_inputs_agree` — assert the three strings
   `sweep_report_json(Sequential)`, `…Threads(2)`, `…Threads(8)` are equal (the fn itself calls
   `process::exit`, so test the comparison inputs, not the fn).
5. Run the gate.

**Gate.**
- `cargo run -q -p cli -- sweep-verify` → prints `sweep-verify: OK …`, exit code 0 (check `echo $?`).
- `cargo test -p cli` → all pass.
- Full-workspace gate green, 0 failed.

**Guardrails.** Decimal/integer only for money; byte-identity is the whole point — never loosen the
comparison (no JSON re-parsing, compare the raw strings); explicit thread counts here (2, 8), never
`available_parallelism`, so the check is a property of the engine not the host; no new dependencies;
no execution/signing/RPC code; holdout stays sealed (inherited assert in `sweep_report_json`); no
schema/master-plan edits.

**Escalate-if.** `sweep-verify` reports MISMATCH (determinism regression — stop and report
immediately; do not debug by weakening); C6's helpers are missing; any pre-existing test fails.

---

### M4-C8 — Record DECISIONS D-0009 + refresh docs for the sweep CLI — `DONE`

**Goal.** Persist the M4 decisions the code already embodies and point the context files at the new
CLI surface (serves the M4 deliverable *canonical result export* being discoverable; closes the
documentation half of the old S12).

**Files.**
1. `DECISIONS.md`
2. `docs/architecture-index.md`
3. `AGENTS.md`

**Current state.** `DECISIONS.md` entries run D-0008 (top, newest) → D-0001; format:
`## D-NNNN — <title> (M<n>)` then `**Context.** … **Decision.** … **Consequences.** …`. There is no
D-0009. `docs/architecture-index.md`'s crate table `cli` row ends with
`all of the above except sweep (sweep dep arrives with the M4 CLI cards)`. `AGENTS.md` Commands lists
`cargo run -p cli -- demo` only.

**Steps.**
1. In `DECISIONS.md`, insert directly under the `---` separator (above `## D-0008 …`), verbatim:
   ```markdown
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
   ```
2. In `docs/architecture-index.md`, replace the `cli` row's dependency cell text
   `all of the above except sweep (sweep dep arrives with the M4 CLI cards)` with `all of the above`,
   and extend that row's Responsibility cell with: `Subcommands: demo, sweep [--threads N] [--out
   PATH] (canonical SweepReport JSON), sweep-verify (parallel==sequential byte-check).`
3. In `AGENTS.md` Commands, after the `cargo run -p cli -- demo` line add:
   `- Sweep: cargo run -p cli -- sweep [--threads N] [--out PATH]; verify determinism: cargo run -p cli -- sweep-verify` *(research only; no network, no keys)*.
4. Run the full-workspace gate (docs-only change — it must stay green).

**Gate.**
- `grep -n "D-0009" DECISIONS.md` → hit at the top entry; `grep -n "sweep-verify" docs/architecture-index.md AGENTS.md` → one hit each.
- Full-workspace gate green, 0 failed.

**Guardrails.** Docs only — NO code, schema, config, or plan-structure changes on this card;
Decimal/f64 wording stays exactly as written above (it restates invariants); no new dependencies; no
execution/signing/RPC anything; holdout wording stays "M5-only"; no master-plan edits.

**Escalate-if.** DECISIONS.md's format/ordering differs from the description; the architecture-index
cli row text differs (an earlier card may have changed it — stop and report); anything beyond these
three files seems to need editing.

---

### M4-C8b — Aggregate per-candidate fee sensitivity in the runner — `DONE`

*(C8b–C8d inserted 2026-07-07 after the pre-declaration adversarial review confirmed a major: the
"turnover and fee-sensitivity reporting" deliverable never reaches the exported report —
`FeeSensitivity` had zero production callers. C8b builds the aggregation; C8c wires it into
`SweepReport` + schema; C8d closes the review's two confirmed minors. Operator approved this scope
2026-07-07 — see worklog.)*

**Goal.** Give `FeeSensitivity` a production path: aggregate each candidate's BeforeCosts/Base/Doubled
cells into one `FeeSensitivity` per candidate (serves M4 deliverable *turnover and fee-sensitivity
reporting*).

**Files.**
1. `crates/sweep/src/sensitivity.rs`
2. `crates/sweep/src/runner.rs`
3. `crates/sweep/src/lib.rs`

**Current state (verbatim, sensitivity.rs:181-195).**
```rust
pub fn fee_sensitivity(
    before_costs: &CellResult,
    base: &CellResult,
    doubled: &CellResult,
    survives_floor: Decimal,
) -> FeeSensitivity {
    FeeSensitivity {
        before_costs: ScenarioMetrics::from_cell(ScenarioId::BeforeCosts, before_costs),
        base: ScenarioMetrics::from_cell(ScenarioId::Base, base),
        doubled: ScenarioMetrics::from_cell(ScenarioId::Doubled, doubled),
        return_drag_doubled: base.total_return - doubled.total_return,
        return_drag_costs: before_costs.total_return - base.total_return,
        survives_doubled: doubled.total_return > survives_floor,
    }
}
```
`ScenarioMetrics` (sensitivity.rs:128-137): `scenario: ScenarioId, total_return: Decimal, turnover:
Decimal, n_trades: u32, fees_paid_quote: Decimal, slippage_paid_quote: Decimal,
priority_fees_paid_sol: Decimal`. `runner.rs` already has `CellKey { window_index, scenario:
ScenarioId, point_index }` (:20-25), `aggregate_evidence` (:70-141, `mean` closure at :98-104), and
run_sweep's `doubled_floors: Vec<Decimal>` (one entry per window). `Decimal` implements `Default`
(ZERO), `Ord`, `Sum`.

**Steps.**
1. In `sensitivity.rs`, add directly after the `FeeSensitivity` struct (before `pub fn
   fee_sensitivity`):
   ```rust
   impl FeeSensitivity {
       /// Assemble from already-aggregated per-scenario metrics. Single source of truth for the
       /// drag and survival rules — the single-cell path [`fee_sensitivity`] delegates here, so
       /// the report and the edge-vanishes criterion can never drift apart.
       #[must_use]
       pub fn from_scenarios(
           before_costs: ScenarioMetrics,
           base: ScenarioMetrics,
           doubled: ScenarioMetrics,
           survives_floor: Decimal,
       ) -> Self {
           let return_drag_doubled = base.total_return - doubled.total_return;
           let return_drag_costs = before_costs.total_return - base.total_return;
           let survives_doubled = doubled.total_return > survives_floor;
           Self {
               before_costs,
               base,
               doubled,
               return_drag_doubled,
               return_drag_costs,
               survives_doubled,
           }
       }
   }
   ```
2. Replace `fee_sensitivity`'s body so it delegates (signature and doc comment unchanged):
   ```rust
   FeeSensitivity::from_scenarios(
       ScenarioMetrics::from_cell(ScenarioId::BeforeCosts, before_costs),
       ScenarioMetrics::from_cell(ScenarioId::Base, base),
       ScenarioMetrics::from_cell(ScenarioId::Doubled, doubled),
       survives_floor,
   )
   ```
3. In `runner.rs`, extend the `use crate::sensitivity::…` line with `FeeSensitivity,
   ScenarioMetrics`, and add after `aggregate_evidence` (before `neighbor_indices`):
   ```rust
   /// Per-candidate aggregated fee-sensitivity blocks, in candidate (point) order — the reportable
   /// projection of the sweep (m4-sweep.md §9). Aggregation across windows, all exact Decimal:
   /// total_return = mean (matching `aggregate_evidence`); turnover = max (worst window);
   /// n_trades / fees / slippage / priority fees = sums. `survives_floor` = mean of the per-window
   /// doubled-cost baseline floors — the same value `aggregate_evidence` records as
   /// `doubled_baseline_floor`, so `survives_doubled` cannot drift from the edge-vanishes verdict.
   #[must_use]
   pub fn aggregate_fee_sensitivity(
       grids: &[ParamGrid],
       keys: &[CellKey],
       results: &[CellResult],
       doubled_floors: &[Decimal],
   ) -> Vec<FeeSensitivity> {
       let n_points: usize = grids.iter().map(|g| g.points().len()).sum();

       #[derive(Clone, Default)]
       struct Acc {
           returns: Vec<Decimal>,
           turnover: Decimal,
           n_trades: u32,
           fees: Decimal,
           slippage: Decimal,
           priority: Decimal,
       }
       let mut acc: Vec<[Acc; 3]> = (0..n_points).map(|_| Default::default()).collect();
       for (k, r) in keys.iter().zip(results) {
           let slot = match k.scenario {
               ScenarioId::BeforeCosts => 0,
               ScenarioId::Base => 1,
               ScenarioId::Doubled => 2,
               _ => continue,
           };
           let a = &mut acc[k.point_index][slot];
           a.returns.push(r.total_return);
           a.turnover = a.turnover.max(r.turnover);
           a.n_trades += r.n_trades;
           a.fees += r.fees_paid_quote;
           a.slippage += r.slippage_paid_quote;
           a.priority += r.priority_fees_paid_sol;
       }

       let mean = |xs: &[Decimal]| {
           if xs.is_empty() {
               Decimal::ZERO
           } else {
               xs.iter().copied().sum::<Decimal>() / Decimal::from(xs.len() as u64)
           }
       };
       let survives_floor = mean(doubled_floors);
       let metrics = |scenario: ScenarioId, a: &Acc| ScenarioMetrics {
           scenario,
           total_return: mean(&a.returns),
           turnover: a.turnover,
           n_trades: a.n_trades,
           fees_paid_quote: a.fees,
           slippage_paid_quote: a.slippage,
           priority_fees_paid_sol: a.priority,
       };
       acc.iter()
           .map(|slots| {
               FeeSensitivity::from_scenarios(
                   metrics(ScenarioId::BeforeCosts, &slots[0]),
                   metrics(ScenarioId::Base, &slots[1]),
                   metrics(ScenarioId::Doubled, &slots[2]),
                   survives_floor,
               )
           })
           .collect()
   }
   ```
4. In `lib.rs`, add `aggregate_fee_sensitivity` to the `pub use runner::{…}` list.
5. Tests. In `sensitivity.rs`'s `mod tests`: `fee_sensitivity_delegates_to_from_scenarios` — build
   three `CellResult`s with the module's existing test helpers and assert
   `fee_sensitivity(&a, &b, &c, floor) == FeeSensitivity::from_scenarios(ScenarioMetrics::from_cell(
   ScenarioId::BeforeCosts, &a), ScenarioMetrics::from_cell(ScenarioId::Base, &b),
   ScenarioMetrics::from_cell(ScenarioId::Doubled, &c), floor)`. In `runner.rs`'s `mod tests`:
   `fee_sensitivity_aggregation_is_exact` — 1-point grid, 2 windows, all three scenarios per window,
   hand-built `CellResult` literals with non-zero returns/turnover/n_trades/fees; assert per-scenario
   mean return, max turnover, summed n_trades/fees, both drags, and `survives_doubled` against
   `mean(doubled_floors)` — every value with `dec!`.
6. `cargo fmt --all`, then the gate.

**Gate.**
- `cargo test -p sweep` → all green including the two new tests.
- `cargo run -q -p cli -- sweep | shasum` → **unchanged** (nothing calls the new fn yet; the report
  changes in C8c). Demo unchanged. Full-workspace gate green, 0 failed.

**Guardrails.** Decimal/integer only — the ONLY division is by window count, in Decimal; no f64; no
new dependencies; deterministic iteration only (Vec + fixed slots — no HashMap); no execution/RPC
code; holdout untouched (`evaluate_on_holdout` never called); **no schema edits on this card**
(schema is C8c's, with recorded cause); no master-plan edits.

**Escalate-if.** `fee_sensitivity`'s current body differs from the verbatim block above;
`ScenarioMetrics` fields differ; the sweep shasum changes; any pre-existing test fails.

---

### M4-C8c — Export per-candidate turnover + fee sensitivity in SweepReport (schema 1.1.0, D-0010) — `DONE` *(2026-07-07: escalated — zero-cost scenario reports a 1-ulp nonzero slippage; fix needed `portfolio/src/cost.rs`, off-card. Resolved 2026-07-08 by M4-C8e's exact-zero guard; re-gated in the same session — 0 failed.)*

**Goal.** Make turnover and fee sensitivity **first-class outputs of the exported artifact** — the
deliverable the pre-declaration review found unmet. `SweepReport` gains a required `candidates`
array; the schema moves to 1.1.0; D-0010 records why. **Sanctioned exception to the 3-file budget
(6 files): report code, schema, and validation tests must land in one green diff.**

**Files.**
1. `crates/sweep/src/report.rs`
2. `crates/sweep/src/runner.rs` (run_sweep wiring only)
3. `schemas/sweep-report.schema.json` — **deliberate contract change (D-0001); cause = the
   2026-07-07 pre-declaration review major, operator-approved**
4. `crates/sweep/tests/schema_validation.rs`
5. `crates/sweep/tests/sweep_runner.rs`
6. `DECISIONS.md` (D-0010, verbatim text in step 7)

**Current state (verbatim).** `report.rs:17`: `pub const SWEEP_SCHEMA_VERSION: &str = "1.0.0";`.
`report.rs:48-55`: `SweepReport { schema_version: String, note: String, trial_count: u32,
thresholds: ThresholdsDto, verdicts: Vec<CandidateVerdict> }`. `report.rs:63-67`:
`pub fn new(thresholds: &AdvancementThresholds, trial_count: u32, mut verdicts:
Vec<CandidateVerdict>) -> Self` (does `verdicts.sort()`). Schema top-level `required` is
`["schema_version", "note", "trial_count", "thresholds", "verdicts"]`; `$defs` currently
`decimalString`, `Thresholds`, `CandidateVerdict`, `RejectionReason`; `schema_version` is
pattern-checked (`^[0-9]+\.[0-9]+\.[0-9]+$`), not a const. run_sweep currently ends
(runner.rs:293-301) with `verdicts` → `SweepReport::new(&spec.thresholds, trial_count, verdicts)`.
C8b provides `aggregate_fee_sensitivity` and `FeeSensitivity::from_scenarios`.

**Steps.**
1. `report.rs`: bump `SWEEP_SCHEMA_VERSION` to `"1.1.0"`. Add these DTOs (import
   `crate::sensitivity::{FeeSensitivity, ScenarioMetrics}` and `research_core::Decimal`):
   ```rust
   /// One scenario's reportable metrics, decimal-string encoded. The parent key
   /// (before_costs/base/doubled) is the scenario discriminator — no separate field carried.
   #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
   pub struct ScenarioMetricsDto {
       pub total_return: String,
       pub turnover: String,
       pub n_trades: u32,
       pub fees_paid_quote: String,
       pub slippage_paid_quote: String,
       pub priority_fees_paid_sol: String,
   }

   /// A candidate's fee-sensitivity block (m4-sweep.md §9), decimal-string encoded. Robustness
   /// reporting only — never a profitability claim (invariant 11).
   #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
   pub struct FeeSensitivityDto {
       pub before_costs: ScenarioMetricsDto,
       pub base: ScenarioMetricsDto,
       pub doubled: ScenarioMetricsDto,
       pub return_drag_costs: String,
       pub return_drag_doubled: String,
       pub survives_doubled: bool,
   }

   /// One candidate's first-class reported metrics (M4 deliverable: turnover and fee-sensitivity
   /// reporting). `Ord` keys on `candidate_label` first — total canonical order, like verdicts.
   #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
   pub struct CandidateMetricsDto {
       pub candidate_label: String,
       pub max_drawdown: String,
       pub turnover: String,
       pub fee_sensitivity: FeeSensitivityDto,
   }
   ```
   with private `ScenarioMetricsDto::from_metrics(&ScenarioMetrics)` and
   `FeeSensitivityDto::from_sensitivity(&FeeSensitivity)` helpers (field-by-field `.to_string()` on
   every Decimal; copy `n_trades`/`survives_doubled` as-is), and one public constructor:
   ```rust
   impl CandidateMetricsDto {
       /// Stringify one candidate's worst-window drawdown/turnover + fee-sensitivity block.
       #[must_use]
       pub fn new(
           candidate_label: String,
           max_drawdown: Decimal,
           turnover: Decimal,
           fee_sensitivity: &FeeSensitivity,
       ) -> Self {
           Self {
               candidate_label,
               max_drawdown: max_drawdown.to_string(),
               turnover: turnover.to_string(),
               fee_sensitivity: FeeSensitivityDto::from_sensitivity(fee_sensitivity),
           }
       }
   }
   ```
2. `SweepReport`: add `pub candidates: Vec<CandidateMetricsDto>,` between `thresholds` and
   `verdicts` (field order = serialization order). `SweepReport::new` gains a fourth parameter
   `mut candidates: Vec<CandidateMetricsDto>`, does `candidates.sort();` next to `verdicts.sort();`,
   stores it. Update report.rs's own unit tests to pass `vec![]` (an empty array is schema-valid).
3. `runner.rs` — in run_sweep after the `evidence` binding:
   ```rust
   let fee = aggregate_fee_sensitivity(&spec.grids, &keys, &results, &doubled_floors);
   let candidates = evidence
       .iter()
       .zip(&fee)
       .map(|(ev, fs)| {
           CandidateMetricsDto::new(ev.candidate_label.clone(), ev.max_drawdown, ev.turnover, fs)
       })
       .collect();
   ```
   change the construction to `SweepReport::new(&spec.thresholds, trial_count, verdicts,
   candidates)`; import `crate::report::CandidateMetricsDto`. Reword run_sweep's BeforeCosts comment
   (runner.rs:253-256): BeforeCosts cells now have a production consumer —
   "consumed by `aggregate_fee_sensitivity` for the reported before-costs rung" (drop
   "kept for M5's FeeSensitivity use").
4. `schemas/sweep-report.schema.json`: add `"candidates"` to the top-level `required`; add under
   `properties`:
   ```json
   "candidates": {
     "description": "Per-candidate first-class metrics: worst-window drawdown/turnover and the fee-sensitivity block (before/base/doubled). Robustness reporting only (invariant 11).",
     "type": "array",
     "items": { "$ref": "#/$defs/CandidateMetrics" }
   }
   ```
   and three new `$defs`, each `"additionalProperties": false`:
   - `CandidateMetrics`: required `["candidate_label", "max_drawdown", "turnover",
     "fee_sensitivity"]`; `candidate_label` string minLength 1; `max_drawdown`/`turnover` →
     `#/$defs/decimalString`; `fee_sensitivity` → `#/$defs/FeeSensitivity`.
   - `FeeSensitivity`: required `["before_costs", "base", "doubled", "return_drag_costs",
     "return_drag_doubled", "survives_doubled"]`; the three scenario keys → `#/$defs/ScenarioMetrics`;
     both drags → `#/$defs/decimalString`; `survives_doubled` `{ "type": "boolean" }`.
   - `ScenarioMetrics`: required `["total_return", "turnover", "n_trades", "fees_paid_quote",
     "slippage_paid_quote", "priority_fees_paid_sol"]`; `n_trades` `{ "type": "integer",
     "minimum": 0 }`; all others → `#/$defs/decimalString`.
5. `tests/schema_validation.rs`: the real-report builder now also passes a non-empty `candidates`
   vec (build one `CandidateMetricsDto::new(...)` from a `FeeSensitivity::from_scenarios(...)` over
   hand-built `ScenarioMetrics`). Add two rejection tests in the existing style: an extra property
   inside a candidates object → invalid; a numeric (non-string) `total_return` → invalid. The
   f64-audit test must stay green (introduce no `f64` token into advance.rs/report.rs).
6. `tests/sweep_runner.rs`: add `fee_sensitivity_is_reported_first_class_per_candidate` — run_sweep
   sequential on the existing fixture; assert `report.candidates.len() == 4`; candidate labels equal
   verdict labels pairwise (both are label-sorted); every `before_costs` block has
   `fees_paid_quote == "0"`, `slippage_paid_quote == "0"`, `priority_fees_paid_sol == "0"`; and for
   every candidate, `fee_sensitivity.survives_doubled == false` **iff** the same-label verdict's
   `failed_criteria` contains `kind == RejectionKind::EdgeVanishesUnderDoubledCosts`
   (report⇔verdict single-source coupling; import `RejectionKind` from `sweep`).
7. `DECISIONS.md` — insert directly under the `---` separator (above `## D-0009 …`), verbatim:
   ```markdown
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
   ```
8. `cargo fmt --all`, then the gate.

**Gate.**
- `cargo test -p sweep` → all green, including the new schema-rejection and first-class tests.
- `cargo run -q -p cli -- sweep | shasum` twice → identical to each other AND to `--threads 8`
  (the hash **will** differ from `7d385d59…` — the report grew; byte-identity across runs/threads
  is the invariant, not the old hash value).
- `cargo run -q -p cli -- sweep-verify; echo $?` → OK, exit 0.
- Full-workspace gate green, 0 failed. Demo hash unchanged (`ae064f79…`).
- `git status` shows NO schema file modified other than `sweep-report.schema.json`.

**Guardrails.** Decimal strings only in the JSON (never numbers for money); `survives_doubled` is
computed ONLY via `from_scenarios` — never recomputed inline; candidates sorted for total-order
byte-identity; no new dependencies; no execution/RPC code; holdout untouched; the ONLY schema
touched is sweep-report; no master-plan edits.

**Escalate-if.** Any quoted current-state line differs; `sweep-verify` reports MISMATCH
(determinism regression — stop); the f64-audit fails; any other schema shows as modified; the
report⇔verdict coupling test fails in a way you'd "fix" by weakening either side.

---

### M4-C8e — Exact-zero buy-side slippage when slippage is zero (unblocks C8c) — `DONE`

*(Inserted 2026-07-08. C8c's escalation was CORRECT and the operator ruled: fix the accounting,
never weaken the assertion. `fill_buy`'s reporting identity leaves a ±1-ulp Decimal residue in
`slippage_quote` when `slippage_bps = 0` — a real money-reporting defect that C8c's first-class
export made visible. Pre-existing M2 behavior; the sell side is already structurally exact.
Execute this card, then re-gate C8c in the SAME session.)*

**Goal.** A zero-slippage cost model must report exactly zero slippage (serves M4 deliverable
*turnover and fee-sensitivity reporting* — exported money values must be exact, invariant 7).

**Files.**
1. `crates/portfolio/src/cost.rs` — **money-accounting file: change EXACTLY what this card says,
   nothing else in the crate**
2. `crates/sweep/src/report.rs` — DTO stringification only (an approved amendment to C8c's
   still-uncommitted build)
(then re-run C8c's gate and flip BOTH cards.)

**Current state (verbatim, cost.rs:69-85; the residue is line 77).**
```rust
    /// Simulate spending `quote_in` USDC to buy SOL at mid `price`.
    #[must_use]
    pub fn fill_buy(&self, quote_in: Decimal, price: Decimal) -> BuyFill {
        let eff = self.buy_price(price);
        let gross_base = quote_in / eff;
        let dex_fee_base = apply_bps(gross_base, self.dex_fee_bps);
        let net_base = quantize_floor(gross_base - dex_fee_base, SOL_DECIMALS);
        // Reporting figures in quote terms (valued at mid price).
        let slippage_quote = quote_in - (quote_in * price / eff);
        let dex_fee_quote = dex_fee_base * price;
        BuyFill {
            net_base,
            gas_sol: self.gas_sol(),
            slippage_quote,
            dex_fee_quote,
        }
    }
```
The sell side (cost.rs:94) is `let slippage_quote = base_in * (price - eff);` — structurally exact
when `eff == price`. `slippage_quote` is a **reporting-only** field: `net_base`/`gas_sol` drive
balances, so this fix cannot change fills, balances, or equity.

**Steps.**
1. Replace line 77 with:
   ```rust
        // Exact-zero guard: with zero slippage `eff == price` and the identity below is
        // mathematically zero, but Decimal's 28-digit division can leave a ±1-ulp residue once
        // balances carry high scale. The sell side (`base_in * (price - eff)`) is structurally
        // exact; this makes the buy side match (invariant 7 — exported money is exact).
        let slippage_quote = if eff == price {
            Decimal::ZERO
        } else {
            quote_in - (quote_in * price / eff)
        };
   ```
2. Regression test in cost.rs's `mod tests`:
   ```rust
   #[test]
   fn zero_slippage_buy_reports_exactly_zero_slippage_even_at_high_scale() {
       // High-scale quote_in (as produced by prior fractional fills) used to leave a ±1-ulp
       // residue through the quote_in * price / eff rounding. Must be exactly zero.
       let cost = CostModel {
           dex_fee_bps: 5,
           slippage_bps: 0,
           base_fee_lamports: 5_000,
           priority_fee_lamports: 50_000,
       };
       let quote_in = dec!(937.5) / dec!(7); // deliberately non-terminating scale
       let f = cost.fill_buy(quote_in, dec!(103));
       assert!(f.slippage_quote.is_zero());
   }
   ```
   (Red-green: this SHOULD fail if run before step 1 — verify if convenient; if these particular
   numbers happen to pass pre-fix, keep the test anyway — C8c's end-to-end assert is the real proof.)
3. In `report.rs` (C8c's uncommitted DTO helpers): stringify every Decimal via
   `.normalize().to_string()` instead of `.to_string()` (the six `ScenarioMetricsDto` money fields,
   both drags in `FeeSensitivityDto`, and `max_drawdown`/`turnover` in `CandidateMetricsDto`), with
   a one-line doc note: scale-canonical strings, matching `param_id`'s `.normalize()` convention —
   so exact zeros export as `"0"`, not `"0.0000000000000000000000000"`. Keep C8c's value-equality
   zero assertions (they pass either way; they are the approved form).
4. `cargo test -p portfolio` → all green including the new test; every pre-existing slippage
   assertion (cost.rs:142, :157, :173, :205, :225) unchanged and green.
5. **Re-run C8c's full gate** (its build is already in the tree): `cargo fmt --all`; full-workspace
   gate → **0 failed** (the previously-failing before_costs assert must now pass);
   `cargo run -q -p cli -- sweep | shasum` twice + `--threads 8` → all three identical (hash will
   differ from `e94e10c0…` — record the new value); `sweep-verify; echo $?` → OK, 0;
   `cargo run -q -p cli -- demo | shasum` → **unchanged `ae064f79…`** (demo uses `slippage_bps: 20`,
   so `eff != price` — untouched by the guard); `git status` → only C8c's six files + cost.rs +
   report.rs modified, no schema other than sweep-report.
6. Flip **both C8e and C8c** to DONE; one worklog line with the new sweep hash.

**Guardrails.** Money-accounting crate: the ONLY behavioral change is the exact-zero guard on a
reporting-only field — `net_base`/`net_quote`/fee/gas formulas must not change by a single
character; Decimal only, no f64; no new dependencies; no execution/RPC code; holdout untouched; no
schema edits on this card (C8c's schema change is already in the tree); no master-plan edits.

**Escalate-if.** cost.rs:69-85 differs from the verbatim block; the demo hash changes (means the
guard touched a non-zero-slippage path — stop immediately); any pre-existing portfolio/strategies/
cli test fails; C8c's gate still fails after the fix.

---

### M4-C8d — Close the review's two minor test gaps — `DONE`

**Goal.** (1) Exercise the InsufficientData path end-to-end through `run_sweep` (not just hand-built
evidence); (2) put the CLI `--threads`/`--out` parser under test — the pre-declaration review's two
confirmed minors.

**Files.**
1. `crates/sweep/tests/sweep_runner.rs`
2. `crates/cli/src/main.rs`

**Steps.**
1. `tests/sweep_runner.rs` — add
   `under_populated_schedule_is_rejected_as_insufficient_data_end_to_end`: clone the fixture
   `spec()` but set `thresholds.min_windows: 99`; run_sweep sequential; assert every verdict has
   `status == Verdict::Rejected` and `failed_criteria.len() == 1` with
   `failed_criteria[0].kind == RejectionKind::InsufficientData` (import `Verdict`, `RejectionKind`
   from `sweep`).
2. `crates/cli/src/main.rs` — extract `sweep_cmd`'s option loop into a pure, unit-testable helper
   (behavior identical — same messages, same decisions):
   ```rust
   /// Parse `sweep` options. Pure so it is unit-testable; `sweep_cmd` maps Err to exit(2).
   fn parse_sweep_args(
       args: impl Iterator<Item = String>,
   ) -> Result<(Parallelism, Option<String>), String>
   ```
   `Err(message)` for: missing `--threads`/`--out` value, non-integer or `0` threads, unknown
   option; `Ok((Parallelism::Sequential, None))` with no args. `sweep_cmd` becomes: parse → on Err
   `eprintln!` + `exit(2)` → run as before. Unit tests: no args → Sequential/None;
   `--threads 2 --out x.json` → Threads(2)/Some("x.json"); `--threads 0`, `--threads abc`,
   `--out` (no value), `--bogus` → all Err.
3. `cargo fmt --all`, then the gate.

**Gate.**
- `cargo test -p sweep --test sweep_runner` and `cargo test -p cli` → new tests green.
- `cargo run -q -p cli -- sweep | shasum` twice → identical, and the hash equals C8c's recorded
  value (the parser refactor must not change output); `sweep-verify` exit 0; demo unchanged.
- Full-workspace gate green, 0 failed.

**Guardrails.** Behavior-preserving refactor only in the CLI; no new dependencies; no execution/RPC
code; holdout untouched; no schema/master-plan edits.

**Escalate-if.** `sweep_cmd`'s current shape doesn't match what C6 built; the sweep hash changes;
any pre-existing test fails.

---

### M4-C9 — M4 gate declaration (evidence checklist against master-plan.md) — `DONE`

**Goal.** Run the full battery, check every M4 deliverable and gate criterion against
`plans/master-plan.md:873-889` (quoted verbatim below — do NOT edit master-plan.md), and declare M4
complete in the plan files.

**Files.**
1. `plans/m4-sweep.md`
2. `plans/current-state.md`
3. `plans/task-queue.md` (this file)
(+ the standard one-line `plans/worklog.md` entry.)

**The authoritative M4 definition (master-plan.md:873-889; content matches word-for-word, blank lines
collapsed — your checklist):**
> ### M4: Sweep and walk-forward
> Deliver:
> - Parameter sweeps for the MVP strategy families.
> - Parallel execution.
> - Canonical result export.
> - Walk-forward windows.
> - Strategy-family comparison.
> - Turnover and fee-sensitivity reporting.
> - Rejection report for failed candidate families.
>
> Gate:
> - Parallel and sequential results match.
> - Repeated runs are identical.
> - Untouched holdout remains untouched.

**Steps.**
1. Run and record (all must pass; paste one-line results into the worklog entry):
   a. `cargo fmt --all --check` → exit 0.
   b. `cargo clippy --all-targets --all-features -- -D warnings` → exit 0.
   c. `cargo test --workspace --all-features` → 0 failed, 0 ignored.
   d. `cargo run -q -p cli -- demo | shasum` twice → identical.
   e. `cargo run -q -p cli -- sweep | shasum` twice → identical; `cargo run -q -p cli -- sweep --threads 8 | shasum` → same hash.
   f. `cargo run -q -p cli -- sweep-verify; echo $?` → prints OK, exit 0.
   g. the no-execution-deps scan from handoff.md §Verified state → prints OK.
2. Evidence checklist — confirm each maps to a real artifact (all should exist if C1–C8d are DONE):
   | Master-plan item | Evidence |
   |---|---|
   | Parameter sweeps, MVP families | `sweep::param` grids (both families) + `SweepSpec.grids` + CLI sweep over the template grids |
   | Parallel execution | `sweep::parallel` (`std::thread::scope`) + `--threads N` + determinism.rs {1,2,3,7,8} |
   | Canonical result export | `SweepReport::to_json` + `schemas/sweep-report.schema.json` validation tests (schema_validation.rs, sweep_runner.rs) |
   | Walk-forward windows | `sweep::window` + `DevValidation::walk_forward_windows` + walk_forward.rs |
   | Strategy-family comparison | cross-family canonical report: both families' candidates scored via the shared `eval_strategy` core against the same 4 cost-matched baselines and windows, exported side-by-side in `candidates[]` + verdicts. The per-family best/median/worst aggregation row is **descoped to M5's "Strategy-family ranking" deliverable — recorded as D-0011** (m4-sweep §10 amended 2026-07-09) |
   | Turnover & fee-sensitivity reporting | **first-class in the exported artifact**: `SweepReport.candidates[]` per-candidate `turnover`, `max_drawdown`, and `fee_sensitivity` block (C8b/C8c, schema 1.1.0, D-0010) + `RunOutput.traded_notional_quote` → `CellResult.turnover` + budget/doubled-costs criteria with recorded observed/threshold |
   | Rejection report | `advance` (7 criteria) + `report` verdicts, schema-validated |
   | Gate: parallel==sequential | determinism.rs + sweep_runner.rs test (a) + step 1e/1f above |
   | Gate: repeated identical | same three, plus step 1d |
   | Gate: holdout untouched | holdout_sealing.rs (counter+digest) + `compile_fail` doctests + CLI `assert_eq!(read_count, 0)` |
3. In `plans/m4-sweep.md`: set the S12 row Status to `✅ DONE (as cards M4-C1…C10)`; add one line to
   the §1 goal section: `**GATE DECLARED <today's date>** — evidence in worklog + task-queue M4 cards.`
4. In `plans/current-state.md`: Milestone section — move M4 to **DONE** (one line: engine S1–S11 +
   cards C1–C8d, gate declared, N tests green); set "DOING" to `— (between milestones; M5 requires
   operator decisions — see handoff)`; update the "Gates run" date/counts with step-1 results.
5. In this file: flip this card to DONE; update the `M4·S1–S11` row Notes with "GATE DECLARED".
6. Append the worklog line with the step-1 results.

**Gate.** All step-1 commands pass; the three plan files agree M4 is complete; no code was changed.

**Guardrails.** Plan files only — no code/schema/config edits; do NOT reword anything in
master-plan.md; determinism evidence must come from freshly run commands (step 1), not from this
card's text; no new dependencies; holdout stays sealed.

**Escalate-if.** ANY step-1 command fails (M4 is NOT complete — stop, report, do not declare); any
checklist row lacks its artifact; cards C1–C8 **and C8b/C8c/C8d/C8e** are not all DONE in this file.

---

### M4-C10 — Point current-state/handoff at the M5 decision and STOP — `DONE`

**Goal.** Leave the repo in a clean between-milestones state: the next action belongs to the
**operator**, not an agent. No M5 implementation may be queued (M5 needs real data — questions.md Q3 —
and its own explicit go decision).

**Files.**
1. `plans/current-state.md`
2. `plans/handoff.md`
(+ the standard worklog line and this file's status flip.)

**Steps.**
1. In `plans/current-state.md`, set "Next recommended command" to exactly this content (adjust
   formatting to the file, not the substance):
   > **STOP — operator decisions required before M5 (the research decision).** In order:
   > 1. Freeze the walk-forward sizing and rejection thresholds (questions.md Q5) — they are
   >    illustrative in `strategy-lab.example.toml` and must be frozen BEFORE the decisive sweep
   >    (post-hoc choice = overfitting).
   > 2. Choose the real OHLCV data source for SOL/USDC (questions.md Q3) — all M4 runs used synthetic
   >    series; no statistical claim is valid until real multi-year data is ingested and validated.
   > 3. Give an explicit M5 go decision. M5's gate (master-plan.md:903-907): select ONE candidate for
   >    mainnet shadow because it satisfies predefined criteria, or reject all and return to research.
   > Do NOT start M6+ (network), and never M8/M9 (signing/submission — separate explicit human approval).
2. In `plans/handoff.md`: update the "What's next" section and replace the seed prompt with a short
   one: read handoff → current-state → the STOP block above; agents may do maintenance (fix a failing
   gate, refresh docs) but must not start M5 work without the operator's written go in questions.md.
3. Update handoff.md's "Verified state" block to the numbers recorded by C9.
4. Worklog line + flip this card.

**Gate.** `grep -n "STOP" plans/current-state.md` hits the new block; handoff.md contains no
instruction to continue past M4; full-workspace gate still green (no code touched).

**Guardrails.** Plan files only; no code/schema/config edits; do not create M5 tasks, data-ingestion
tasks, or any M6+ scaffolding; no master-plan edits.

**Escalate-if.** C9 is not DONE; anything in handoff.md contradicts the STOP semantics after your
edit; you find yourself wanting to sketch M5 implementation steps (don't — that is the operator's gate).

---

## M5 — task cards (**CLOSED 2026-07-12** — gate declared: Branch A reject-all, holdout unread; see worklog + current-state)

The M5 research decision, per the operator's written resolutions in `plans/questions.md` (Q3, Q5,
and the M5 GO block, all 2026-07-09). Sequence: ingestion → hygiene + span confirmation → Q5
number-freeze → CLI wiring → decisive sweep → decision card. Every signature quoted below was
**copied verbatim from source on 2026-07-09 at commit `bd0b3e2`** — if what you find differs,
escalate, don't adapt.

**Common rules, guardrails, and escalate-ifs are IDENTICAL to the M4 section above** (fresh session
per card; only the card's files; flip status + one worklog line; full-workspace gate
`cargo fmt --all --check` && `cargo clippy --all-targets --all-features -- -D warnings` &&
`cargo test --workspace --all-features`, 0 failed; `cargo fmt --all` after pasting snippets;
Decimal/integer money; byte-identical parallel==sequential; **no new dependencies**; **no execution
code** — no keys/signing/submit/RPC, and **no HTTP client code in any crate** (the only network
touch in all of M5 is the operator running the C1 shell script by hand); never edit
`schemas/*.json`, `plans/master-plan.md`, `solana-crypto-trader-plan.md`, or `fixtures/`). Two
additions for M5:
- **Real data is operator-provisioned and gitignored.** `data/raw/` is ignored by `.gitignore`
  (lines 43-48). No card ever stages anything under `data/`; the checked-in artifact is the
  validation record `plans/m5-data-validation.md`.
- **The holdout seal rules stand until M5-C6**: `evaluate_on_holdout` is called at most once, only
  by the M5-C6 decision card, only for an `advanceable` candidate, only after the Q5 freeze (M5-C3).
  If the decisive sweep rejects all candidates, the holdout is **never read at all** (counter
  stays 0) and no holdout-calling code is ever written.

---

### M5-C1 — Operator ingestion script for Binance SOLUSDC daily klines — `DONE`

**Goal.** A one-time, operator-run snapshot of daily SOLUSDC klines from `data.binance.vision` into
gitignored files (Q3 resolution; serves M5 gate master-plan.md:905-909 by making a real-data
decision possible).

**Files.** `scripts/ingest-binance-solusdc-1d.sh` (new; the `scripts/` directory does not exist yet
— create it). Nothing else; no Rust changes.

**Current state (verbatim).** `.gitignore:43-48`:
```
/data/raw/
/data/normalized/
/data/**/*.parquet
/data/**/*.csv
/data/**/*.db
/data/**/*.sqlite
```

**Steps.**
1. Create `scripts/ingest-binance-solusdc-1d.sh`, `chmod +x`, with `#!/usr/bin/env bash` and
   `set -euo pipefail`. Arguments: `START_MONTH END_MONTH` (inclusive, `YYYY-MM`), plus an optional
   `--dry-run` flag that only prints the URLs it would fetch.
2. For each month in the range, download
   `https://data.binance.vision/data/spot/monthly/klines/SOLUSDC/1d/SOLUSDC-1d-<YYYY-MM>.zip`
   and its `.CHECKSUM` sibling into `data/raw/binance/SOLUSDC-1d/` (create with `mkdir -p`),
   verify with `shasum -a 256 -c`, unzip the CSV alongside, and delete the zip. A missing month
   (HTTP 404, e.g. before the pair listed) is reported and **skipped**, not fatal; any checksum
   failure IS fatal.
3. On completion print: months fetched, months skipped, CSV file count, and
   `shasum -a 256 data/raw/binance/SOLUSDC-1d/*.csv | shasum -a 256` (one combined content hash the
   operator can paste into the validation record).
4. The script must refuse to run if `git check-ignore data/raw/` fails (belt-and-braces: never
   ingest into a stageable path).

**Gate.** `bash -n scripts/ingest-binance-solusdc-1d.sh` (syntax-clean); `scripts/ingest-… 2021-01
2021-02 --dry-run` prints exactly two monthly zip URLs and fetches nothing; full-workspace gate
still green (no Rust touched); `git status` shows only the script.

**Guardrails (restated).** The script is run BY THE OPERATOR, by hand, once — no card, test, CI
job, or Rust code may invoke it. No credentials anywhere (data.binance.vision is keyless). Nothing
under `data/` is ever staged. No new dependencies; no HTTP code in any crate.

**Escalate-if.** `.gitignore:43-48` differs from the verbatim block above; the card seems to need
any Rust change; anything suggests adding an HTTP client crate.

---

### M5-C2 — `machina data-validate`: Binance CSV → `Vec<Bar>` loader + hygiene record — `DONE`

**Goal.** Deterministically load the snapshot CSVs into validated `Bar`s and print the hygiene
record (row count, date span, digest) that M5-C3's freeze sitting consumes (Q3 resolution: hygiene
failures shrink the span — never patch or forward-fill).

**Files.** `crates/market-data/src/binance_csv.rs` (new) + one `pub mod`/re-export line in
`crates/market-data/src/lib.rs`; `crates/cli/src/main.rs` (new `data-validate` subcommand).
*(Sanctioned 3-file card.)*

**Current state (verbatim).**
- `market-data/src/validation.rs:116`:
  `pub fn validate_series_spacing(bars: &[Bar], expected_interval_secs: i64) -> Result<(), DataError> {`
- `cli/src/main.rs:33-43` dispatch:
```rust
    match args.next().as_deref() {
        Some("demo") => demo(),
        Some("sweep") => sweep_cmd(args),
        Some("sweep-verify") => sweep_verify(),
        Some("--help" | "-h" | "help") | None => usage(),
        Some(other) => {
            eprintln!("unknown command: {other}\n");
            usage();
            std::process::exit(2);
        }
    }
```

**Steps.**
1. `binance_csv.rs`: `pub fn load_dir(dir: &Path) -> Result<Vec<Bar>, BinanceCsvError>` —
   read `*.csv` files in the directory **sorted by file name** (byte order, deterministic), parse
   each line by splitting on `,`: Binance kline columns are
   `open_time,open,high,low,close,volume,close_time,…` (12 columns; only the first 6 are used).
   Skip a line whose first field does not parse as `i64` **only if it is the first line of a file**
   (header tolerance); anywhere else it is an error. `open_time` may be in milliseconds (13
   digits) or microseconds (16 digits) — normalize to whole seconds by magnitude
   (`>= 10^15 → /1_000_000`, `>= 10^12 → /1_000`); a value that is not an exact multiple of its
   divisor is an error (never silently truncate a misaligned timestamp). Prices/volume parse as
   `Decimal` (`rust_decimal` is already a dependency; **std + existing deps only — no csv crate**).
2. After loading all files, sort by `ts` is **not** applied — files sorted by name and rows in file
   order must already be ascending; validation catches violations (no silent reordering).
3. Also in `binance_csv.rs`: `pub fn fnv1a64(bars: &[Bar]) -> u64` — FNV-1a over each bar's
   `ts.unix()` and the canonical `to_string()` of its five Decimal fields, in order (pure integer
   math; this is the record's cross-run digest).
4. CLI: `machina data-validate --dir PATH [--interval-secs 86400]` → `load_dir`, then
   `validate_series_spacing(&bars, interval)`; on success print exactly:
   `data-validate: OK — <n> bars, <first YYYY-MM-DD>..<last YYYY-MM-DD>, spacing <interval>s, fnv1a64 0x<hex>`
   and exit 0; on any failure print the error and exit 1. **Gap handling:** if
   `validate_series_spacing` rejects, also print the timestamp of the first offending bar so the
   operator can shrink the span (Q3 rule) — the tool never patches.
5. Tests (in `binance_csv.rs` + a CLI test): a tiny in-repo **string literal** fixture (not a file
   under `fixtures/` — do not touch that tree): valid 3-line CSV parses; header line tolerated;
   ms and µs timestamps normalize identically; misaligned timestamp rejected; out-of-order rows
   rejected by validation; digest is stable (pin the exact hex).

**Gate.** New tests green; full-workspace gate green (expect **> 301** tests, 0 failed);
`cargo run -p cli -- demo | shasum` unchanged (`ae064f79242f823ffd8f55bf9104e3e1b45d425a`);
`cargo run -p cli -- sweep | shasum` unchanged (`7ad3df7de2e2c1139be427e9c953b57d4e289cb3`);
`git status` → no `Cargo.toml`/`Cargo.lock` modified (proves zero new deps).

**Guardrails (restated).** Decimal for prices/volume — never f64; no network/HTTP; no new deps; no
silent forward-fill, reordering, or dedup — hygiene failures are errors; never edit `schemas/*`,
`fixtures/`, or the plan pair; holdout untouched.

**Escalate-if.** The dispatch block or `validate_series_spacing` signature differs from the
verbatim blocks; a real snapshot line has other than 12 columns (report the line, don't adapt);
you feel the need for a csv/serde-csv dependency; demo/sweep hashes move.

---

### M5-C3 — Q5 number-freeze sitting (OPERATOR + planner — not an executor card) — `DONE`

**Goal.** Freeze the final M5 research policy against the confirmed real span, BEFORE any strategy
result is computed (Q5 resolution's one-way ratchet).

**Files.** `config/strategies/m5-frozen.toml` (new, checked in — no secrets);
`plans/m5-data-validation.md` (new); `plans/questions.md` (Q5 freeze addendum);
`plans/worklog.md`. *(Plan/config only — no Rust.)*

**Steps.**
1. Operator runs `scripts/ingest-binance-solusdc-1d.sh` for the full listable range, then
   `cargo run -p cli -- data-validate --dir data/raw/binance/SOLUSDC-1d`. If hygiene fails at the
   early edge, re-ingest from the first clean month (shrink, never patch) until OK.
2. Record in `plans/m5-data-validation.md`: source URL pattern, months fetched/skipped, the
   script's combined sha256, `data-validate`'s exact OK line (count, span, fnv1a64), the CEX-proxy
   caveat verbatim from Q3, and the date of the sitting.
3. Compute the holdout boundary: `holdout.start` = the UTC date at the 80% point of the confirmed
   span (rounded to the 1st of the next month for legibility); `validation.start` = one year before
   `holdout.start`. Confirm the dev/val length supports ≥ `min_windows` rolling 365/90/90/5 windows
   (windows ≈ floor((dev_val_len − 365 − 5 − 90)/90) + 1); adjust `min_windows` ONLY downward-never,
   upward-if-needed is allowed before the freeze.
4. Write `config/strategies/m5-frozen.toml`: copy `strategy-lab.example.toml`, set the real
   partition dates, `[walk_forward]` rolling 365/90/90/5, `[advancement]` = drawdown_budget "0.35",
   turnover_budget "12", baseline_margin "0.02", dispersion_budget "0.40", neighbor_tolerance
   "0.15", min_windows (final), and the grids **frozen as-is** from the template (Q4: illustrative,
   now frozen for this cycle). Header comment: "FROZEN 2026-MM-DD per Q5 — immutable for the M5
   cycle."
5. Append the freeze (final dates + numbers + window count) to questions.md Q5 and one worklog line.

**Gate.** `data-validate` OK line matches the record; the frozen TOML parses
(`SweepSpec::from_toml_str` — verified implicitly by M5-C4's tests); every number matches the Q5
resolution; freeze recorded before any sweep of real data exists anywhere.

**Guardrails.** After this card, `m5-frozen.toml` is immutable for the cycle (one-way ratchet). No
strategy may be run on the real data before this card completes — if any real-data strategy result
exists first, STOP: the freeze is contaminated; escalate to the operator on the record.

---

### M5-C4 — Wire `--config` / `--data` into `machina sweep` (+ provenance note) — `DONE`

**Goal.** The sweep CLI runs the frozen spec on the real snapshot — same deterministic pipeline,
zero behavior change for the existing template/synthetic path (M4 gates must not move).

**Files.** `crates/cli/src/main.rs`; `crates/sweep/src/report.rs` (additive provenance-note
constructor only). *(2 files.)*

**Current state (verbatim).**
- `cli/src/main.rs:57-59`:
```rust
fn parse_sweep_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(Parallelism, Option<String>), String> {
```
- `cli/src/main.rs:130` `fn sweep_report_json(parallelism: Parallelism) -> String {` — builds the
  spec from the embedded `STRATEGY_LAB_TEMPLATE`, bars from `sweep_series()`, cost model
  `dex_fee_bps: 5, slippage_bps: 20, base_fee_lamports: 5_000, priority_fee_lamports: 50_000`,
  then asserts `outcome.sealed.holdout_read_count() == 0`.
- `sweep/src/spec.rs:107` `pub fn from_toml_str(s: &str) -> Result<Self, SpecError> {`
- `sweep/src/runner.rs:314-321`:
```rust
pub fn run_sweep(
    spec: &SweepSpec,
    bars: Vec<Bar>,
    base_cost: &CostModel,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
    parallelism: Parallelism,
) -> Result<SweepOutcome, SweepError> {
```
- `sweep/src/report.rs:132` `pub note: String,` (a free string in the schema; `report.rs:156` sets
  it from a fixed `REPORT_NOTE`).

**Steps.**
1. Extend `parse_sweep_args` to also accept `--config PATH` and `--data DIR` (both or neither —
   one without the other is a usage error). Return type grows accordingly; all existing arg tests
   stay green unchanged plus new ones for the pairing rule.
2. In the sweep path: with `--config/--data`, read the TOML with `std::fs::read_to_string`,
   `SweepSpec::from_toml_str`, load bars via `market_data::binance_csv::load_dir`, validate with
   `validate_series_spacing(&bars, 86_400)`, and run the SAME `run_sweep` call with the SAME
   hardcoded cost model (the M4 modeled-cost assumptions, unchanged and now frozen with the cycle);
   keep the `holdout_read_count() == 0` assert on this path too.
3. `report.rs`: add an additive constructor (e.g. `SweepReport::with_provenance(…, provenance:
   &str)`) that appends ` | data: <provenance>` to the fixed note. `SweepReport::new` delegates and
   stays byte-identical for existing callers. The CLI real-data path passes
   `binance data.binance.vision SOLUSDC 1d snapshot (CEX-proxy; see plans/m5-data-validation.md), fnv1a64 0x<hex>`.
   The schema is untouched (`note` is a free string); the existing note tests must still pass.
4. Tests: pairing-rule arg tests; a report test that `with_provenance` keeps the robustness
   disclaimer AND carries the provenance suffix; template path output byte-identical (existing
   determinism tests untouched).

**Gate.** Full-workspace gate green, 0 failed; `machina sweep | shasum` UNCHANGED
(`7ad3df7de2e2c1139be427e9c953b57d4e289cb3`) and `sweep-verify: OK` (the no-flag path must be
byte-identical to before this card); `machina sweep --config config/strategies/m5-frozen.toml
--data data/raw/binance/SOLUSDC-1d | shasum` twice → identical; no `Cargo.toml`/`Cargo.lock`
changes.

**Guardrails (restated).** No new deps; no schema edit; Decimal money; the holdout assert stays on
every CLI path; `evaluate_on_holdout` is NOT wired anywhere in this card; never stage `data/`.

**Escalate-if.** Any verbatim block above mismatches; the no-flag sweep hash moves (your change
leaked into the M4 path — stop); the frozen TOML fails to parse (C3's file is wrong — report,
don't fix silently); tempted to touch `schemas/sweep-report.schema.json`.

---

### M5-C5 — The decisive sweep run (mechanical; evidence captured) — `DONE`

**Goal.** Produce the one canonical real-data sweep report the M5 decision reads (master-plan
deliverables :895-903).

**Files.** `plans/m5-sweep-report.json` (new, checked in — deterministic research artifact);
`plans/worklog.md`.

**Steps.**
1. Confirm preconditions: M5-C3 + C4 DONE; `data-validate` OK line matches
   `plans/m5-data-validation.md` exactly (fnv1a64 included).
2. Run, capturing shasums:
   `cargo run -q -p cli -- sweep --config config/strategies/m5-frozen.toml --data data/raw/binance/SOLUSDC-1d --out plans/m5-sweep-report.json`;
   repeat to a temp path and `cmp` (byte-identical); run again with `--threads 8` to a temp path
   and `cmp` (parallel == sequential on real data).
3. Worklog line: the report shasum, the three-run identity, trial_count, and the counts of
   `advanceable` vs `rejected` verdicts. **Do not interpret the results in this card.**

**Gate.** Three byte-identical runs; report validates against `schemas/sweep-report.schema.json`
(the existing `sweep` test suite's validator path — or `cargo test -p sweep schema` green on the
committed report if a test reads it); worklog updated.

**Guardrails.** Read-only with respect to code. No threshold, config, or data edit after seeing
results — the ratchet is closed. Never stage `data/`.

**Escalate-if.** Runs are not byte-identical (determinism regression — STOP, do not declare);
`data-validate` no longer matches the record (snapshot drifted — STOP).

---

### M5-C6 — M5 decision card: the single holdout read + advance-or-reject-all declaration (OPERATOR + planner) — `DONE` *(2026-07-12: Branch A — reject-all declared; step-0 review PASS, 6 agents; holdout read count 0)*

**Goal.** Deliver M5's gate (master-plan.md:903-909, quoted verbatim during declaration): "One
candidate is selected for mainnet shadow because it satisfies predefined robustness and drawdown
criteria, or all candidates are rejected and the project returns to research. No execution work
starts merely because the software exists."

**Files.** `plans/current-state.md`, `plans/handoff.md`, `plans/worklog.md`,
`plans/questions.md`; **only if** an advanceable candidate exists: `crates/cli/src/main.rs`
(one-shot `m5-decide` subcommand, added by a dedicated fresh-session sub-card the planner writes
at that time).

**Current state (verbatim, partition.rs:448-454).**
```rust
pub fn evaluate_on_holdout(
    sealed: Sealed,
    chosen: &ParamPoint,
    cost: &CostModel,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
) -> Result<CellResult, SimError> {
```

**Steps.**
0. **Mandatory pre-declaration adversarial review (FOREMAN §3/§7) — before touching any file below.**
   This is the single highest-stakes card in the project (the M5 gate, master-plan.md:903-909); M4's
   own gate declaration (M4-C9) was correctly stopped twice by exactly this kind of review before it
   was allowed to land, and this card gets no less scrutiny. Run a multi-agent workflow (Sonnet
   subagents) with **4 independent lenses** over the draft declaration + `plans/m5-sweep-report.json`
   + `config/strategies/m5-frozen.toml`:
   - **Verdict-vs-report fidelity** — every claim in the draft declaration text is checked against
     the actual `SweepReport` JSON (verdicts, `failed_criteria`, `trial_count`) field-by-field; no
     paraphrase may soften or drop a failed criterion.
   - **Frozen-threshold integrity** — `config/strategies/m5-frozen.toml`'s `[walk_forward]` and
     `[advancement]` blocks are diffed line-by-line against the Q5 resolution recorded in
     `plans/questions.md` (2026-07-09) and against `plans/m5-data-validation.md`'s confirmed span;
     any drift is a blocker (a silently-redefined threshold is exactly how a post-hoc rejection
     becomes a post-hoc advance).
   - **Holdout-path audit** — greps the diff/tree for every call site of `evaluate_on_holdout`;
     Branch A must show **zero** call sites anywhere; Branch B must show **exactly one**, gated by
     `read_count == 1`, with no code path that could invoke it twice.
   - **Declaration-vs-master-plan conformance** — the declaration text is checked against
     master-plan.md:903-909 quoted verbatim: does it actually assert what the gate requires (a named
     candidate satisfying predefined criteria, or an explicit reject-all), not a weaker substitute.
   Each finding gets **2 independent skeptic agents** attempting to refute it. Decision rule (fixed
   in advance, per FOREMAN §7): any confirmed blocker or major → **do not declare**; fix the cause or
   descope on the record (FOREMAN §8), then re-run this review before trying again. Zero confirmed
   findings → record "review: PASS (N agents)" in the worklog and proceed to step 1.
1. Read `plans/m5-sweep-report.json` verdicts.
2. **Branch A — all candidates `rejected` (expected default).** The holdout is NEVER read: no code
   is written, `holdout_read_count` stays 0, the seal survives intact for a future cycle. Declare
   reject-all in current-state + worklog quoting each candidate's `failed_criteria`
   (observed-vs-threshold pairs) and master-plan.md:905-907's reject arm. Point
   current-state/handoff at the **HF research track** as the operator-chosen "return to research"
   (Q7). A clean reject-all is a SUCCESS of the gates, not a failure of the project.
3. **Branch B — ≥1 `advanceable`.** The planner writes (then a fresh executor runs) a sub-card
   adding `machina m5-decide --config … --data …`: re-runs the pipeline, takes the single
   best-ranked advanceable `ParamPoint`, calls `evaluate_on_holdout` ONCE (the signature above;
   base cost model), prints the holdout `CellResult` + `read_count == 1` proof, and exits. Run it
   ONCE. Record the holdout result in the declaration; selection is judged against the FROZEN
   thresholds — the holdout result is reported as-is (pass or fail, no re-tuning, no second read,
   under any circumstances).
4. Either branch: declaration in current-state + worklog against master-plan.md:903-909 verbatim;
   questions.md gets the outcome; the M5 section of this queue flips to CLOSED.

**Guardrails.** `evaluate_on_holdout` at most once, ever, this cycle — Branch A calls it zero
times. No execution work follows from either branch without its own milestone approval (M6+ gates;
M8/M9 separate explicit human approval). Never weaken a frozen threshold to flip a verdict. The
step-0 review is not optional and not skippable by a confident planner — it runs even if the
verdicts look unambiguous.

**Escalate-if.** Any impulse to re-run the sweep with different numbers after seeing results; any
second holdout read; any verdict ambiguity (e.g. report/schema mismatch); any confirmed blocker or
major from the step-0 review — STOP and record.

---

## M-HF wave 1 — task cards (**ACTIVE 2026-07-12** — C1/C2 reconciled + unblocked by operator note; C2.5/C2.6 to be drafted after C2 lands)

*(Drafted 2026-07-07 on operator instruction after the entry-condition check found all three
conditions unmet — see worklog. These cards serve machina's goal — genuine autonomous passive
income at high volume — by building the machinery that TESTS whether small-edge/high-volume
trading clears real round-trip costs. Every HF strategy family is a hypothesis; no card claims or
implies a known-profitable algorithm; rejecting all seven families cleanly is a successful outcome.)*

**Design authority: [m-hf-track.md](m-hf-track.md), §5, not `highfrequency-algo-plan.md`.**
The Q7 amendment (D-0012, 2026-07-09) ran a 3-design + adversarial-review + adjudication workflow
and selected a **reuse-first** design; `m-hf-track.md` is the resulting, adjudicated milestone plan
and its §5 card sequence is authoritative (it inserts new cards **C2.5** and **C2.6** before C3,
and changes C1's/C6's required surface — see the reconciliation note below).
`highfrequency-algo-plan.md` remains reference text for its §1 strategy survey and §2
execution-realism requirements only; its own §3 card sketch is **superseded**.

**Entry conditions — verify directly (M4-C9's Status in this file; `cmp plans/master-plan.md
solana-crypto-trader-plan.md`; `grep -n "^## D-0012" DECISIONS.md`; `grep -n "HF-Q" plans/questions.md`):**
1. **M4 gate declared** — card M4-C9 is DONE in this file. ✅ (2026-07-09)
2. **Operator approval + amendment** — ✅ **SATISFIED 2026-07-09 (D-0012).** The master-plan
   amendment was made (`plans/master-plan.md` §19 "M-HF" + amended Arbitrage/XEMM/MEV disposition;
   root `solana-crypto-trader-plan.md` edited in lockstep, `cmp`-verified byte-identical, per Q6).
3. **HF-Q decisions recorded in plans/questions.md** — **partially resolved 2026-07-12.**
   HF-Q2 RESOLVED (USDT + jitoSOL research-allowlist additions approved). HF-Q1 deferred to wave 2
   by design (blocks only M-HF-C9/C10; Birdeye terms re-verified at decision time), and **C1–C2.6
   are unblocked ahead of it by written operator note under HF-Q1** (2026-07-12). HF-Q3 (frozen
   intraday policy, same ratchet as Q5) still blocks the M-HF-C10 sweep, as designed.

**Reconciliation note (stated, not inferred) — C1/C2 as drafted are the PRE-adjudication text,
unmodified.** The card bodies below were copied verbatim from source on 2026-07-07 at commit
`e821591`, **before** the design workflow ran; they were **not** regenerated against
`m-hf-track.md`'s adjudicated design. Confirmed by direct inspection (2026-07-09): neither card
contains the typed `Provenance::{Synthetic,Real}` enum `m-hf-track.md` §2/§5 requires — C2's
`SyntheticIntraday` carries only a plain `pub synthetic: bool` field — and neither contains the
`IntraBar = research_core::Bar` alias `m-hf-track.md` §5 names as C1's required addition ("zero
logic" — a doc comment + `pub use`). Everything else in C1/C2 (the `TradePrint`/`SlotSnapshot`
types, the strict/loose hygiene split, C2's `Congestion` enum and its `splitmix64` primitive) lines
up with the adjudicated design's §1–§3 and needs no rework. **Reconciliation patch DONE 2026-07-12
(planner session):** (a) the `IntraBar = Bar` alias folded into C1 per `m-hf-track.md` §1; (b) the
typed `Provenance::{Synthetic, Real}` enum added to C1 and C2's `synthetic: bool` replaced with it
per §2 (C2 gains a pure-integer `spec_hash` — FNV-1a over the spec's fields, no serde_json in src);
(c) C1's "Current state (verbatim)" block re-verified against source at `3365907` (market-data
lib.rs had gained `binance_csv` in M5-C2 — quote updated).

**Wave discipline.** C1–C2 are drafted now because they need only today's `crates/research-core` +
`crates/market-data`. `m-hf-track.md` §5 inserts **C2.5** (reuse-proof: existing engine on 1s bars,
zero new source) and **C2.6** (streaming/scale-proof at ~31M rows) between C2 and the (renumbered)
C3 fill engine — neither is drafted yet. **C4–C10 remain deliberately NOT drafted** (C3 drafted
2026-07-13 after the audit re-run noted below) — they must
quote types C1–C2.6 will create, so each wave is expanded against what actually landed, and against
`m-hf-track.md` §5's card-by-card gate list (not the superseded sketch in
`highfrequency-algo-plan.md` §3). A mandatory **adversarial review checkpoint follows the
fill/cost/adversarial-terms cards** (C3–C5 in the new numbering — the place an HF backtest would
silently lie) before C6+ may be expanded, per `m-hf-track.md` §5's stated risk points. The
2026-07-07 audit of the HF plan vs. the tree covered market-data/portfolio/prior-art; the
sweep-ladder, strategies-trait, and invariants-config audit areas were cut short (session limit) and
**must be re-run before wave 2 is expanded** (findings + coverage gaps recorded in the 2026-07-07
worklog entry and in [highfrequency-algo-plan.md](highfrequency-algo-plan.md)'s Appendix).
**Re-run DONE 2026-07-13** — no blocking drift; drift table in that day's worklog entry.

Execution contract and guardrails: identical to the M4 cards above (fresh session per card; flip the
card status + one worklog line when its gate passes; never `git commit`/`git push` — the operator
commits explicit paths; full-workspace gate on every card; `cargo fmt --all` once after typing in
pasted code). **Step 0 of every card: run the full-workspace gate BEFORE touching any file.** If it
is already red, STOP and report the baseline failure in the worklog instead of proceeding — never
spend the session attributing a pre-existing break to your own diff. (The 2026-07-07 card rehearsal
hit exactly this: from a clean checkout, `crates/results/tests/schema_validation.rs` fails because
`.gitignore`'s `wallet*.json` pattern kept `schemas/wallet-snapshot.schema.json` untracked — see
that day's worklog entry for the fix status.)

**Escalate and STOP (report in plans/worklog.md instead of improvising) if:** a file, line, or
signature named on the card doesn't match what you find; any pre-existing test fails, or a gate
fails for a reason unrelated to your change; the task seems to need a file the card doesn't list;
anything ambiguous touches money types, determinism, the holdout, or schemas.

Every signature below was **copied verbatim from source on 2026-07-07 at commit
`e821591`** (the tree's uncommitted M4-C8c edits touch only `crates/sweep`,
`schemas/sweep-report.schema.json`, and `DECISIONS.md` — not these crates) — if what you find
differs, escalate, don't adapt.

---

### M-HF-C1 — Intraday domain types + series hygiene (trade prints, slot snapshots, 1s bars) — `DONE` *(2026-07-13; reconciled 2026-07-12 against m-hf-track.md §1/§2: IntraBar alias + typed Provenance folded in; verbatim blocks re-verified at `3365907`; unblocked by operator note, questions.md HF-Q1)*

**Goal.** Give the research engine intraday primitives with the same hygiene guarantees daily bars
already have: `TradePrint`/`SlotSnapshot` types in `research-core` (self-checking, like `Bar`),
series-level validation in `market-data` (single venue, sorted, unique per key, explicit gap
policy), and tiny hand-written synthetic fixtures proving every rejection path (serves HF plan
§2.1; unblocks M-HF-C2 and every later HF card).

**Audit corrections this card implements (2026-07-07, confirmed against source).** (a) The HF
plan's "gap-scenario config from M1" does **not** exist in code — the only gap override today is
*calling the looser function*: validation.rs:114-115 says "Callers that deliberately allow gaps (an
explicit scenario) should use [`validate_series`] instead." This card mirrors that exact
two-function pattern for intraday data (strict + loose, choice documented); a config-level scenario
mechanism belongs to the wave that gives `SweepSpec` intraday support (M-HF-C8), where a run
actually declares scenarios. (b) "(venue, slot, seq)" is a **new** per-event ordering/uniqueness
key this card introduces — no existing Rust type in `research-core`/`market-data` carries per-event
venue/slot/seq fields (`AllowlistEntry.venues` in allowlist.rs and the wallet-snapshot schema's
`slot` are unrelated, non-colliding uses of those words); `Bar` is keyed by `ts` alone. (c) 1-second bars
need **no new type**: `validate_series_spacing(bars, 1)` already handles them — this card only adds
the test that pins that.

**Files.**
1. `crates/research-core/src/intraday.rs` (new)
2. `crates/research-core/src/lib.rs` (two added lines)
3. `crates/market-data/src/intraday.rs` (new)
4. `crates/market-data/src/lib.rs` (two added lines)
5. `crates/market-data/fixtures/` — SIX NEW files (step 5): `prints_good.json`,
   `prints_unsorted.json`, `prints_duplicate.json`, `prints_bad_size.json`, `snapshots_good.json`,
   `snapshots_slot_gap.json`
6. `crates/market-data/tests/intraday_validation.rs` (new)

Sanctioned exception to the ~3-file budget: files 1/3/5/6 are ALL NEW; the two lib.rs edits are
module + re-export lines. No existing file's behavior changes. **This card edits NO Cargo.toml**
(all needed deps exist) and touches NO existing fixture.

**Current state (verbatim, re-verified 2026-07-12 at `3365907` — M5-C2 added `binance_csv` to
market-data since the 2026-07-07 draft).**
`crates/research-core/src/lib.rs:12-15` modules (alphabetical): `pub mod bar; pub mod money;
pub mod time; pub mod token;` — re-exports at :17-25 include `pub use bar::{Bar, BarError};` and
`pub use rust_decimal::Decimal;`. `crates/market-data/src/lib.rs:10-17` (entire public surface):
```rust
pub mod allowlist;
pub mod binance_csv;
pub mod validation;

pub use allowlist::{Allowlist, AllowlistEntry};
pub use validation::{
    validate_series, validate_series_spacing, validate_token_decimals, DataError,
};
```
The daily-bar machinery this card mirrors (read-only — do NOT modify any of it):
```rust
// crates/research-core/src/bar.rs:12-21
pub struct Bar {
    /// Bar open time (UTC). Bars are keyed and sorted by this.
    pub ts: Timestamp,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    /// Base-asset volume; must be ≥ 0.
    pub volume: Decimal,
}
// crates/research-core/src/time.rs:11-12 — newtype ⇒ serializes as a bare integer in fixtures
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(i64);
// crates/market-data/src/validation.rs:84
pub fn validate_series(bars: &[Bar]) -> Result<(), DataError>
// crates/market-data/src/validation.rs:116
pub fn validate_series_spacing(bars: &[Bar], expected_interval_secs: i64) -> Result<(), DataError>
```
validation.rs:113-115 doc (the gap-scenario pattern to mirror): "a hole between otherwise sorted,
unique bars is rejected with [`DataError::Gap`]. Callers that deliberately allow gaps (an explicit
scenario) should use [`validate_series`] instead."
Fixture-loading convention (crates/market-data/tests/fixture_validation.rs:14-18):
```rust
fn load(name: &str) -> Vec<Bar> {
    let path = format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}
```
Existing fixtures (6 files, daily bars — never modify): bars_bad_ohlc / bars_duplicate / bars_gap /
bars_good / bars_negative_volume / bars_unsorted `.json`. Dependencies already present:
research-core has `rust_decimal` + `serde` (dev: `rust_decimal_macros`, `serde_json`); market-data
has `research-core`, `rust_decimal`, `serde`, `toml` (dev: `rust_decimal_macros`, `serde_json`).

**Steps.**
1. Create `crates/research-core/src/intraday.rs`:
   ```rust
   //! Intraday market events: trade prints and per-slot state snapshots (HF plan §2.1).
   //!
   //! Same split as [`crate::bar`]: an item only knows how to validate itself; series-level checks
   //! (single venue, sorted, unique per key, slot-gap policy) live in `market-data`. Research data
   //! only — these types carry no execution capability.

   use crate::time::Timestamp;
   use rust_decimal::Decimal;
   use serde::{Deserialize, Serialize};

   /// Aggressor side of a trade print.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "lowercase")]
   pub enum Side {
       Buy,
       Sell,
   }

   /// Where a series came from — a typed field, not prose (m-hf-track §2). A result computed
   /// over any synthetic input can never render as real: reports roll this up as a computed
   /// fact, never a hand-written label.
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(tag = "kind", rename_all = "lowercase")]
   pub enum Provenance {
       /// Deterministic generated data; `spec_hash` identifies the generating spec exactly.
       Synthetic { spec_hash: u64 },
       /// Operator-ingested archived data; `source_id` names the snapshot record.
       Real { source_id: String },
   }

   /// 1-second bars need no new type: an intraday bar IS a [`crate::Bar`] at finer spacing —
   /// a series-level interval fact asserted by `validate_series_spacing(bars, 1)`, not a
   /// type-level one. Zero logic (m-hf-track §1).
   pub type IntraBar = crate::Bar;

   /// One executed trade on one venue. Ordered and deduplicated by `(slot, seq)` within a venue
   /// (`seq` disambiguates multiple prints landing in the same slot).
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub struct TradePrint {
       pub venue: String,
       /// Solana slot in which the trade landed.
       pub slot: u64,
       /// Intra-slot sequence number (0-based) — the uniqueness key together with `slot`.
       pub seq: u32,
       /// Wall-clock time of the slot (UTC). Carried for bar alignment; ordering uses (slot, seq).
       pub ts: Timestamp,
       pub side: Side,
       /// Trade price in quote units; must be > 0.
       pub price: Decimal,
       /// Trade size in base units; must be > 0.
       pub size: Decimal,
   }

   /// One market-state observation for one venue at one slot (pool/book mid).
   /// Depth fields arrive with the HF fill/cost cards — do not add them here.
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub struct SlotSnapshot {
       pub venue: String,
       pub slot: u64,
       pub ts: Timestamp,
       /// Mid price in quote units; must be > 0.
       pub mid: Decimal,
   }

   /// A single-item validation failure, tagged with the offending key.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum IntradayItemError {
       NonPositivePrice { slot: u64, seq: u32 },
       NonPositiveSize { slot: u64, seq: u32 },
       NonPositiveMid { slot: u64 },
   }

   impl std::fmt::Display for IntradayItemError {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
           match self {
               Self::NonPositivePrice { slot, seq } => {
                   write!(f, "print (slot {slot}, seq {seq}): price must be > 0")
               }
               Self::NonPositiveSize { slot, seq } => {
                   write!(f, "print (slot {slot}, seq {seq}): size must be > 0")
               }
               Self::NonPositiveMid { slot } => {
                   write!(f, "snapshot (slot {slot}): mid must be > 0")
               }
           }
       }
   }

   impl std::error::Error for IntradayItemError {}

   impl TradePrint {
       /// Validate this print's single-item invariants: `price > 0`, `size > 0`.
       pub fn check(&self) -> Result<(), IntradayItemError> {
           if self.price <= Decimal::ZERO {
               return Err(IntradayItemError::NonPositivePrice {
                   slot: self.slot,
                   seq: self.seq,
               });
           }
           if self.size <= Decimal::ZERO {
               return Err(IntradayItemError::NonPositiveSize {
                   slot: self.slot,
                   seq: self.seq,
               });
           }
           Ok(())
       }
   }

   impl SlotSnapshot {
       /// Validate this snapshot's single-item invariant: `mid > 0`.
       pub fn check(&self) -> Result<(), IntradayItemError> {
           if self.mid <= Decimal::ZERO {
               return Err(IntradayItemError::NonPositiveMid { slot: self.slot });
           }
           Ok(())
       }
   }
   ```
   plus `#[cfg(test)] mod tests` in the same file with exactly these four tests (use
   `rust_decimal_macros::dec` and `serde_json` — both dev-deps):
   - `valid_print_and_snapshot_pass_check` — a print (price `dec!(100.25)`, size `dec!(1.5)`) and a
     snapshot (mid `dec!(100)`) both return `Ok(())`.
   - `non_positive_price_size_mid_rejected` — price `dec!(0)` → `NonPositivePrice`; size `dec!(-1)`
     → `NonPositiveSize`; mid `dec!(0)` → `NonPositiveMid` (match on the variants).
   - `print_serde_round_trips_with_lowercase_side` — `serde_json::to_string` of a print contains
     `"side":"buy"`; parsing it back equals the original.
   - `snapshot_serde_round_trips` — snapshot JSON round-trips to an equal value and its `ts`
     serializes as a bare integer (assert the JSON contains `"ts":1609459200`).
   - `provenance_serde_round_trips_tagged` — `Provenance::Synthetic { spec_hash: 42 }` serializes
     to JSON containing `"kind":"synthetic"` and round-trips equal; `Provenance::Real { source_id:
     "binance-solusdc-1d".into() }` round-trips equal. Also pin the alias with a one-line assert
     that an `IntraBar` value constructed as a `Bar` compares equal to itself via the alias type
     (compile-time proof the alias is `Bar`).
2. In `crates/research-core/src/lib.rs`: add `pub mod intraday;` between `pub mod bar;` and
   `pub mod money;`; add `pub use intraday::{IntraBar, IntradayItemError, Provenance, Side,
   SlotSnapshot, TradePrint};` directly after the `pub use bar::{Bar, BarError};` line.
3. Create `crates/market-data/src/intraday.rs`:
   ```rust
   //! Intraday series hygiene: trade prints and slot snapshots (HF plan §2.1). Same discipline as
   //! [`crate::validation`], same gap rule as daily bars: **missing data blocks a run unless the
   //! caller explicitly declares a gap scenario by choosing the looser validator** — never
   //! silently. At slot resolution gaps are frequent (skipped slots, outages), so snapshot series
   //! treat slot gaps as the declared-scenario default ([`validate_snapshots`]) and offer a strict
   //! contiguous check ([`validate_snapshots_contiguous`]) for gap-free segments.
   //!
   //! 1-second BARS need no new types: a 1s series is a plain `[Bar]` validated with
   //! [`crate::validation::validate_series_spacing`]`(bars, 1)` (gap-blocking) or
   //! [`crate::validation::validate_series`] (declared-gap scenario) — pinned by test in
   //! `tests/intraday_validation.rs`.

   use research_core::intraday::{IntradayItemError, SlotSnapshot, TradePrint};

   /// An intraday data-hygiene failure. Every variant is a hard stop for a run.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum IntradayError {
       /// A run was asked to use an empty series.
       EmptySeries,
       /// All items in one series must come from one venue (cross-venue data = separate series).
       MixedVenue {
           index: usize,
           expected: String,
           found: String,
       },
       /// `(slot, seq)` (prints) or `slot` (snapshots) is not strictly increasing.
       OutOfOrder {
           index: usize,
           prev_slot: u64,
           cur_slot: u64,
       },
       /// Two items share the uniqueness key. `seq` is 0 for snapshot series (keyed by slot alone).
       DuplicateKey { index: usize, slot: u64, seq: u32 },
       /// Timestamps decrease while the slot advances.
       NonMonotonicTime { index: usize },
       /// A single item failed its invariants.
       Item(IntradayItemError),
       /// A hole in a contiguous-slot series (strict validator only).
       SlotGap {
           index: usize,
           prev_slot: u64,
           cur_slot: u64,
       },
   }

   impl std::fmt::Display for IntradayError {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
           match self {
               Self::EmptySeries => write!(f, "intraday series is empty (a run requires data)"),
               Self::MixedVenue {
                   index,
                   expected,
                   found,
               } => write!(f, "mixed venues at index {index}: expected {expected}, found {found}"),
               Self::OutOfOrder {
                   index,
                   prev_slot,
                   cur_slot,
               } => write!(
                   f,
                   "items out of order at index {index}: slot {cur_slot} follows {prev_slot}"
               ),
               Self::DuplicateKey { index, slot, seq } => {
                   write!(f, "duplicate key (slot {slot}, seq {seq}) at index {index}")
               }
               Self::NonMonotonicTime { index } => {
                   write!(f, "timestamp decreases at index {index} while slot advances")
               }
               Self::Item(e) => write!(f, "{e}"),
               Self::SlotGap {
                   index,
                   prev_slot,
                   cur_slot,
               } => write!(
                   f,
                   "missing slot(s) before index {index}: {cur_slot} follows {prev_slot} (expected contiguous)"
               ),
           }
       }
   }

   impl std::error::Error for IntradayError {}

   impl From<IntradayItemError> for IntradayError {
       fn from(e: IntradayItemError) -> Self {
           Self::Item(e)
       }
   }

   /// Validate a single-venue trade-print series: non-empty, per-item invariants, one venue,
   /// strictly increasing `(slot, seq)`, non-decreasing timestamps. Slot gaps are expected in
   /// event data and are NOT an error here.
   pub fn validate_prints(prints: &[TradePrint]) -> Result<(), IntradayError> {
       if prints.is_empty() {
           return Err(IntradayError::EmptySeries);
       }
       let venue = &prints[0].venue;
       for (i, p) in prints.iter().enumerate() {
           p.check()?;
           if &p.venue != venue {
               return Err(IntradayError::MixedVenue {
                   index: i,
                   expected: venue.clone(),
                   found: p.venue.clone(),
               });
           }
           if i > 0 {
               let prev = (prints[i - 1].slot, prints[i - 1].seq);
               let cur = (p.slot, p.seq);
               if cur == prev {
                   return Err(IntradayError::DuplicateKey {
                       index: i,
                       slot: p.slot,
                       seq: p.seq,
                   });
               }
               if cur < prev {
                   return Err(IntradayError::OutOfOrder {
                       index: i,
                       prev_slot: prints[i - 1].slot,
                       cur_slot: p.slot,
                   });
               }
               if p.ts < prints[i - 1].ts {
                   return Err(IntradayError::NonMonotonicTime { index: i });
               }
           }
       }
       Ok(())
   }

   /// Validate a single-venue snapshot series: non-empty, per-item invariants, one venue, strictly
   /// increasing `slot`, non-decreasing timestamps. **Slot gaps are allowed here** — this is the
   /// declared-scenario path, mirroring [`crate::validation::validate_series`] for daily bars.
   pub fn validate_snapshots(snaps: &[SlotSnapshot]) -> Result<(), IntradayError> {
       if snaps.is_empty() {
           return Err(IntradayError::EmptySeries);
       }
       let venue = &snaps[0].venue;
       for (i, s) in snaps.iter().enumerate() {
           s.check()?;
           if &s.venue != venue {
               return Err(IntradayError::MixedVenue {
                   index: i,
                   expected: venue.clone(),
                   found: s.venue.clone(),
               });
           }
           if i > 0 {
               let prev = snaps[i - 1].slot;
               if s.slot == prev {
                   return Err(IntradayError::DuplicateKey {
                       index: i,
                       slot: s.slot,
                       seq: 0,
                   });
               }
               if s.slot < prev {
                   return Err(IntradayError::OutOfOrder {
                       index: i,
                       prev_slot: prev,
                       cur_slot: s.slot,
                   });
               }
               if s.ts < snaps[i - 1].ts {
                   return Err(IntradayError::NonMonotonicTime { index: i });
               }
           }
       }
       Ok(())
   }

   /// [`validate_snapshots`] AND require every slot advance to be exactly `+1`. The strict
   /// gap-blocking path, mirroring [`crate::validation::validate_series_spacing`].
   pub fn validate_snapshots_contiguous(snaps: &[SlotSnapshot]) -> Result<(), IntradayError> {
       validate_snapshots(snaps)?;
       for i in 1..snaps.len() {
           let prev = snaps[i - 1].slot;
           if snaps[i].slot != prev + 1 {
               return Err(IntradayError::SlotGap {
                   index: i,
                   prev_slot: prev,
                   cur_slot: snaps[i].slot,
               });
           }
       }
       Ok(())
   }
   ```
   plus `#[cfg(test)] mod tests` with exactly these eight tests (build items via small `fn print(…)`
   / `fn snap(…)` helpers with `dec!`):
   - `good_prints_pass` — 4 prints: (slot 1000, seq 0), (1000, 1), (1002, 0), (1005, 0), equal or
     rising ts → `Ok` (covers same-slot seq bump AND slot skips being fine for prints).
   - `mixed_venue_rejected` — second print with another venue → `MixedVenue { index: 1, .. }`.
   - `out_of_order_and_duplicate_rejected` — (1002,0) then (1000,0) → `OutOfOrder`; (1000,0) twice
     → `DuplicateKey`; same slot with seq 1 then 0 → `OutOfOrder`.
   - `non_monotonic_time_rejected` — rising slots, falling `ts` → `NonMonotonicTime { index: 1 }`.
   - `bad_item_rejected` — size `dec!(0)` → `Item(IntradayItemError::NonPositiveSize { .. })`.
   - `snapshots_loose_allows_slot_gaps_strict_rejects` — slots 1000, 1001, 1003:
     `validate_snapshots` → `Ok`; `validate_snapshots_contiguous` →
     `SlotGap { index: 2, prev_slot: 1001, cur_slot: 1003 }`.
   - `empty_series_rejected` — both validators on `&[]` → `EmptySeries`.
   - `intraday_error_display_covers_variants` — `to_string()` of each variant contains its key
     phrase (mirror `data_error_display_and_from_bar_error` in validation.rs).
4. In `crates/market-data/src/lib.rs`: add `pub mod intraday;` between `pub mod allowlist;` and
   `pub mod validation;`; add
   `pub use intraday::{validate_prints, validate_snapshots, validate_snapshots_contiguous, IntradayError};`
   after the `pub use allowlist::…` line.
5. Create the six fixtures under `crates/market-data/fixtures/` (tiny, hand-written, synthetic —
   `venue_a` is deliberately not a real venue name):
   `prints_good.json`:
   ```json
   [
     {"venue": "venue_a", "slot": 1000, "seq": 0, "ts": 1609459200, "side": "buy",  "price": "100.25", "size": "1.5"},
     {"venue": "venue_a", "slot": 1000, "seq": 1, "ts": 1609459200, "side": "sell", "price": "100.20", "size": "0.75"},
     {"venue": "venue_a", "slot": 1002, "seq": 0, "ts": 1609459201, "side": "buy",  "price": "100.30", "size": "2.0"},
     {"venue": "venue_a", "slot": 1005, "seq": 0, "ts": 1609459202, "side": "sell", "price": "100.10", "size": "0.5"}
   ]
   ```
   `prints_unsorted.json`:
   ```json
   [
     {"venue": "venue_a", "slot": 1002, "seq": 0, "ts": 1609459201, "side": "buy",  "price": "100.30", "size": "2.0"},
     {"venue": "venue_a", "slot": 1000, "seq": 0, "ts": 1609459200, "side": "buy",  "price": "100.25", "size": "1.5"}
   ]
   ```
   `prints_duplicate.json`:
   ```json
   [
     {"venue": "venue_a", "slot": 1000, "seq": 0, "ts": 1609459200, "side": "buy",  "price": "100.25", "size": "1.5"},
     {"venue": "venue_a", "slot": 1000, "seq": 0, "ts": 1609459200, "side": "buy",  "price": "100.25", "size": "1.5"}
   ]
   ```
   `prints_bad_size.json`:
   ```json
   [
     {"venue": "venue_a", "slot": 1000, "seq": 0, "ts": 1609459200, "side": "buy",  "price": "100.25", "size": "1.5"},
     {"venue": "venue_a", "slot": 1002, "seq": 0, "ts": 1609459201, "side": "sell", "price": "100.20", "size": "-1"}
   ]
   ```
   `snapshots_good.json`:
   ```json
   [
     {"venue": "venue_a", "slot": 1000, "ts": 1609459200, "mid": "100.25"},
     {"venue": "venue_a", "slot": 1001, "ts": 1609459200, "mid": "100.30"},
     {"venue": "venue_a", "slot": 1002, "ts": 1609459201, "mid": "100.20"}
   ]
   ```
   `snapshots_slot_gap.json`:
   ```json
   [
     {"venue": "venue_a", "slot": 1000, "ts": 1609459200, "mid": "100.25"},
     {"venue": "venue_a", "slot": 1001, "ts": 1609459200, "mid": "100.30"},
     {"venue": "venue_a", "slot": 1003, "ts": 1609459201, "mid": "100.20"}
   ]
   ```
6. Create `crates/market-data/tests/intraday_validation.rs` mirroring fixture_validation.rs's
   shape (same `load` pattern — write two small helpers `load_prints`/`load_snapshots`):
   - `good_prints_fixture_validates`; `unsorted_prints_fixture_rejected` (`OutOfOrder`);
     `duplicate_prints_fixture_rejected` (`DuplicateKey`); `bad_size_prints_fixture_rejected`
     (`Item(_)`); `snapshot_fixtures_good_and_gap` (good passes BOTH validators; gap fixture passes
     `validate_snapshots`, rejected by `validate_snapshots_contiguous` — assert both, with a
     comment restating "gaps allowed only by explicit scenario, never silently");
   - `one_second_bars_use_existing_validators` — build 3 in-line `Bar`s at unix 0, 1, 2 (reuse the
     `bar()` helper shape from validation.rs tests) → `validate_series_spacing(&bars, 1)` is `Ok`;
     with ts 0, 1, 3 → `Err(DataError::Gap { .. })`. This pins the "1s bars are plain `Bar`s" claim.
7. `cargo fmt --all`, then run the gate.

**Gate.**
- `cargo test -p research-core` → the 4 new intraday unit tests pass; all pre-existing pass.
- `cargo test -p market-data` → the 8 new unit + 6 new integration tests pass; all pre-existing
  (validation, allowlist, fixture_validation) pass unchanged.
- Full-workspace gate green: `cargo fmt --all --check` && `cargo clippy --all-targets
  --all-features -- -D warnings` && `cargo test --workspace --all-features` → **0 failed**.
- `cargo run -q -p cli -- demo | shasum` twice → identical (nothing on this card touches the CLI).
- `git status` under `crates/market-data/fixtures/` shows ONLY the six new files — no existing
  fixture modified.

**Guardrails (restated; violating any one = STOP).** Prices/sizes/money are `Decimal` — never f64.
No RNG, no clock, no I/O in library code (validation is pure; fixtures are read only by tests).
Parallel==sequential byte-identity of the sweep is untouched (this card must not touch
`crates/sweep`; the determinism gate's thread counts {1,2,3,7,8} stay green via the workspace
gate). The holdout stays sealed — `evaluate_on_holdout` is M5-only, never called or wired.
Strategies emit intent only — no strategy, cost, or execution code on this card. **No execution
code anywhere**: no keys, signing, submission, RPC, network, or HTTP (M8/M9 require separate
explicit human approval). For later HF cards (context, restated per contract): per-trade cost =
venue fee + depth-walk slippage + regime priority fee + tip — never base fee alone; adversarial/MEV
terms are modeled only as costs *to us*, never as capability. No new dependencies (workspace or
external). ADD-only under `fixtures/` — never modify existing fixtures, `schemas/*`,
`plans/master-plan.md`, or `solana-crypto-trader-plan.md`.

**Escalate-if.** This card's Status is still BLOCKED (entry conditions 1–2 above unmet — check
first, work second); any verbatim block differs from source (esp. validation.rs:84/116,
bar.rs:12-21, either lib.rs); a name this card adds collides with an existing export; `Timestamp`
does not serialize as a bare integer (newtype serde behavior changed — stop); any pre-existing test
fails; you find yourself wanting to modify `DataError`, `validation.rs`, or any existing fixture
(don't — new module + new files only).

---

### M-HF-C2 — Deterministic synthetic microstructure generator — `DONE` *(2026-07-13; reconciled 2026-07-12: `synthetic: bool` replaced by the typed `Provenance` enum per m-hf-track §2; requires M-HF-C1 DONE)*

**Goal.** A generator for synthetic intraday research data — OU mean-reverting mid price +
impact-decay events + regime-switching congestion — emitting 1s bars, trade prints, slot snapshots,
and a congestion series that all pass M-HF-C1 validation. Same spec → byte-identical output; every
output self-identifies as synthetic. This generator carries HF cards C3–C8 until the operator
decides HF-Q1 (real intraday data), exactly as Q3's synthetic-fixtures default carried M1–M4.

**Files.**
1. `crates/market-data/src/synthetic.rs` (new)
2. `crates/market-data/src/lib.rs` (two added lines)
No Cargo.toml edits; no fixture files (generation is in-memory — checked-in fixtures stay the
hand-authored C1 ones; **never write files from library code**).

**Current state (assumes M-HF-C1 landed exactly as its card specifies — verify, and escalate if
not).** `market-data::intraday` exports `validate_prints`, `validate_snapshots`,
`validate_snapshots_contiguous`, `IntradayError`; `research_core::intraday` exports `Side`,
`SlotSnapshot`, `TradePrint`, `IntradayItemError`;
`market_data::validation::validate_series_spacing(bars, 1)` validates 1s bars. The determinism
invariant this card must satisfy (docs/invariants.md §3 enforcement line, verbatim): "fixed-point
money (`rust_decimal::Decimal`), ordered collections in canonical output, no wall-clock/RNG in
canonical runs, and a repeated-run determinism test." Compliance: the generator is a **pure
function of its `SyntheticSpec`** — the "noise" is a fixed 16-entry `Decimal` quantile table
indexed by a SplitMix64 integer hash stream seeded from the spec. No entropy source, no clock, no
`rand` dependency (D-0002 minimalism upheld). NOTE: `rust_decimal_macros` is a **dev-dependency**
of market-data — library code must construct constants with `Decimal::new(mantissa, scale)` /
`Decimal::from(int)`; `dec!` only in tests.

**Steps.**
1. Create `crates/market-data/src/synthetic.rs`:
   ```rust
   //! Deterministic synthetic intraday data (HF plan §2.1, card M-HF-C2). Research fixtures ONLY.
   //!
   //! Every output is a pure function of its [`SyntheticSpec`]: the "noise" is a fixed quantile
   //! table indexed by a SplitMix64 integer hash stream seeded from the spec — no entropy source,
   //! no clock, no `rand` dependency (invariant 3: no wall-clock/RNG in canonical runs). Same spec
   //! → byte-identical output. Every bundle carries a typed `Provenance::Synthetic { spec_hash }`
   //! into serialization, so no
   //! synthetic series can masquerade as real data, and none of it may ever ground a statistical
   //! claim about real markets (invariant 11's no-profitability-claims rule, extended here to
   //! synthetic-vs-real labeling; real ingestion is M-HF-C9, gated on HF-Q1).

   use crate::intraday::{validate_prints, validate_snapshots};
   use crate::validation::validate_series_spacing;
   use research_core::intraday::{Provenance, Side, SlotSnapshot, TradePrint};
   use research_core::{Bar, Decimal, Timestamp};
   use serde::Serialize;

   /// Spec for one synthetic intraday series. The output is a pure function of this struct.
   #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
   pub struct SyntheticSpec {
       /// Seed of the SplitMix64 index stream.
       pub seed: u64,
       /// Number of 1-second steps (one bar per step); must be > 0.
       pub steps: usize,
       /// Unix seconds (UTC) of the first bar.
       pub start_unix: i64,
       /// First Solana slot. Slots advance 2–3 per step (deterministically), so snapshot series
       /// have realistic slot gaps — the declared-scenario path of M-HF-C1.
       pub start_slot: u64,
       /// Synthetic venue label (not a real venue).
       pub venue: String,
       /// Initial mid price; must be > 0.
       pub mid0: Decimal,
       /// OU anchor μ; must be > 0.
       pub anchor: Decimal,
       /// OU reversion θ per step; must be in [0, 1).
       pub reversion: Decimal,
       /// Noise step size σ in quote units; must be ≥ 0.
       pub vol_step: Decimal,
       /// Every `impact_every`-th step (i > 0) starts an impact event; 0 = never.
       pub impact_every: usize,
       /// Initial displacement magnitude of an impact event; must be ≥ 0.
       pub impact_size: Decimal,
       /// Per-step geometric decay of the outstanding impact; must be in [0, 1).
       pub impact_decay: Decimal,
       /// Steps per congestion regime segment (> 0): Calm → Busy → Hot → Calm → …
       pub regime_period: usize,
   }

   /// Congestion regime label (consumed by the HF cost cards).
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
   #[serde(rename_all = "lowercase")]
   pub enum Congestion {
       Calm,
       Busy,
       Hot,
   }

   /// Why a spec was rejected or generation failed its own hygiene check.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum SyntheticError {
       BadSpec(&'static str),
       /// Generated output failed M-HF-C1 validation — a generator bug by definition.
       SelfCheck(String),
   }

   impl std::fmt::Display for SyntheticError {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
           match self {
               Self::BadSpec(why) => write!(f, "bad synthetic spec: {why}"),
               Self::SelfCheck(e) => write!(f, "generator self-check failed: {e}"),
           }
       }
   }

   impl std::error::Error for SyntheticError {}

   /// One generated bundle. `provenance` is always `Provenance::Synthetic { spec_hash }` and
   /// serializes into every export — synthetic data must self-identify, typed, not prose
   /// (m-hf-track §2).
   #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
   pub struct SyntheticIntraday {
       /// Always `Provenance::Synthetic { spec_hash: spec_hash(&spec) }`.
       pub provenance: Provenance,
       pub spec: SyntheticSpec,
       pub bars_1s: Vec<Bar>,
       pub prints: Vec<TradePrint>,
       pub snapshots: Vec<SlotSnapshot>,
       /// One label per step.
       pub congestion: Vec<Congestion>,
   }

   /// Identify the generating spec exactly: FNV-1a (64-bit) over every field's canonical string,
   /// in declaration order, 0xFF-separated (same constants and separator discipline as
   /// `binance_csv::fnv1a64`). Pure integer math; NO serde_json (dev-dep only in this crate).
   fn spec_hash(spec: &SyntheticSpec) -> u64 {
       const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
       const PRIME: u64 = 0x0000_0100_0000_01b3;
       let fields = [
           spec.seed.to_string(),
           spec.steps.to_string(),
           spec.start_unix.to_string(),
           spec.start_slot.to_string(),
           spec.venue.clone(),
           spec.mid0.to_string(),
           spec.anchor.to_string(),
           spec.reversion.to_string(),
           spec.vol_step.to_string(),
           spec.impact_every.to_string(),
           spec.impact_size.to_string(),
           spec.impact_decay.to_string(),
           spec.regime_period.to_string(),
       ];
       let mut hash = OFFSET_BASIS;
       for field in &fields {
           for byte in field.as_bytes() {
               hash ^= u64::from(*byte);
               hash = hash.wrapping_mul(PRIME);
           }
           hash ^= 0xFF; // field separator: avoids concatenation ambiguity between fields
           hash = hash.wrapping_mul(PRIME);
       }
       hash
   }

   /// SplitMix64 — a deterministic integer hash sequence, NOT an entropy source.
   fn splitmix64(state: &mut u64) -> u64 {
       *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
       let mut z = *state;
       z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
       z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
       z ^ (z >> 31)
   }

   /// Fixed symmetric noise quantile table (mean 0). `dec!` is dev-only, hence `Decimal::new`.
   fn noise_table() -> [Decimal; 16] {
       [
           Decimal::new(-200, 2),
           Decimal::new(-150, 2),
           Decimal::new(-100, 2),
           Decimal::new(-75, 2),
           Decimal::new(-50, 2),
           Decimal::new(-25, 2),
           Decimal::new(-10, 2),
           Decimal::ZERO,
           Decimal::ZERO,
           Decimal::new(10, 2),
           Decimal::new(25, 2),
           Decimal::new(50, 2),
           Decimal::new(75, 2),
           Decimal::new(100, 2),
           Decimal::new(150, 2),
           Decimal::new(200, 2),
       ]
   }

   fn check_spec(spec: &SyntheticSpec) -> Result<(), SyntheticError> {
       if spec.steps == 0 {
           return Err(SyntheticError::BadSpec("steps must be > 0"));
       }
       if spec.mid0 <= Decimal::ZERO {
           return Err(SyntheticError::BadSpec("mid0 must be > 0"));
       }
       if spec.anchor <= Decimal::ZERO {
           return Err(SyntheticError::BadSpec("anchor must be > 0"));
       }
       if spec.reversion < Decimal::ZERO || spec.reversion >= Decimal::ONE {
           return Err(SyntheticError::BadSpec("reversion must be in [0, 1)"));
       }
       if spec.vol_step < Decimal::ZERO {
           return Err(SyntheticError::BadSpec("vol_step must be >= 0"));
       }
       if spec.impact_size < Decimal::ZERO {
           return Err(SyntheticError::BadSpec("impact_size must be >= 0"));
       }
       if spec.impact_decay < Decimal::ZERO || spec.impact_decay >= Decimal::ONE {
           return Err(SyntheticError::BadSpec("impact_decay must be in [0, 1)"));
       }
       if spec.regime_period == 0 {
           return Err(SyntheticError::BadSpec("regime_period must be > 0"));
       }
       Ok(())
   }

   /// Generate one synthetic bundle. Pure: same `spec` → byte-identical output, proven by test.
   ///
   /// Per step: decay the outstanding impact, maybe start a new impact event, then
   /// `next = mid + θ(μ − mid) + σ·ε + impact`, quantized to 9 dp (deterministic banker's
   /// rounding) and floored at 0.01 so prices stay positive. The bar is (open = mid,
   /// close = next); one print and one snapshot land on a slot that advances 2–3 per step.
   pub fn generate(spec: &SyntheticSpec) -> Result<SyntheticIntraday, SyntheticError> {
       check_spec(spec)?;
       let noise = noise_table();
       let floor = Decimal::new(1, 2); // 0.01
       let mut state = spec.seed;
       let mut mid = spec.mid0;
       let mut impact = Decimal::ZERO;
       let mut slot = spec.start_slot;
       let mut bars_1s = Vec::with_capacity(spec.steps);
       let mut prints = Vec::with_capacity(spec.steps);
       let mut snapshots = Vec::with_capacity(spec.steps);
       let mut congestion = Vec::with_capacity(spec.steps);
       for i in 0..spec.steps {
           let r = splitmix64(&mut state);
           let eps = noise[(r % 16) as usize];
           impact *= spec.impact_decay;
           if spec.impact_every > 0 && i > 0 && i % spec.impact_every == 0 {
               let sign = if r & (1 << 20) == 0 { Decimal::ONE } else { -Decimal::ONE };
               impact = spec.impact_size * sign;
           }
           let mut next = (mid + spec.reversion * (spec.anchor - mid) + spec.vol_step * eps
               + impact)
               .round_dp(9);
           if next < floor {
               next = floor;
           }
           let ts = Timestamp::from_unix(spec.start_unix + i as i64);
           let (high, low) = if next >= mid { (next, mid) } else { (mid, next) };
           let volume = Decimal::from(1 + (r >> 8) % 9);
           bars_1s.push(Bar {
               ts,
               open: mid,
               high,
               low,
               close: next,
               volume,
           });
           slot += 2 + ((r >> 16) & 1); // 2–3 slots per second: slot gaps are the honest default
           let side = if next >= mid { Side::Buy } else { Side::Sell };
           prints.push(TradePrint {
               venue: spec.venue.clone(),
               slot,
               seq: 0,
               ts,
               side,
               price: next,
               size: volume,
           });
           snapshots.push(SlotSnapshot {
               venue: spec.venue.clone(),
               slot,
               ts,
               mid: next,
           });
           congestion.push(match (i / spec.regime_period) % 3 {
               0 => Congestion::Calm,
               1 => Congestion::Busy,
               _ => Congestion::Hot,
           });
           mid = next;
       }
       // Self-check: generator output must satisfy the M-HF-C1 hygiene it will be tested against.
       validate_series_spacing(&bars_1s, 1)
           .map_err(|e| SyntheticError::SelfCheck(e.to_string()))?;
       validate_prints(&prints).map_err(|e| SyntheticError::SelfCheck(e.to_string()))?;
       validate_snapshots(&snapshots).map_err(|e| SyntheticError::SelfCheck(e.to_string()))?;
       Ok(SyntheticIntraday {
           provenance: Provenance::Synthetic {
               spec_hash: spec_hash(spec),
           },
           spec: spec.clone(),
           bars_1s,
           prints,
           snapshots,
           congestion,
       })
   }
   ```
2. `#[cfg(test)] mod tests` in the same file — exactly these seven tests (use `dec!` and
   `serde_json`; base-spec helper: `seed 42, steps 600, start_unix 1_609_459_200,
   start_slot 100_000, venue "venue_a".to_string(), mid0 dec!(100), anchor dec!(100),
   reversion dec!(0.05), vol_step dec!(0.2), impact_every 50, impact_size dec!(1.5),
   impact_decay dec!(0.5), regime_period 100`):
   - `same_spec_is_byte_identical` — `serde_json::to_string(&generate(&spec()).unwrap())` twice →
     equal strings; changing only `seed` to 43 → a different string.
   - `outputs_pass_c1_validation` — on the base spec: `validate_series_spacing(&out.bars_1s, 1)`
     Ok; `validate_prints(&out.prints)` Ok; `validate_snapshots(&out.snapshots)` Ok; and
     `validate_snapshots_contiguous(&out.snapshots)` → `Err(IntradayError::SlotGap { .. })` (slot
     skipping is the honest default; contiguity is a declared scenario — this asymmetry is the
     point, assert it).
   - `output_self_identifies_as_synthetic` — `out.synthetic` is true AND the serialized JSON
     contains `"synthetic":true`.
   - `mean_reversion_pulls_toward_anchor` — spec: `mid0 dec!(110), anchor dec!(100),
     reversion dec!(0.2), vol_step dec!(0.05), impact_every 0, steps 100`. Let
     `dev_i = bars_1s[i].close − dec!(100)` for `i in 0..steps`: for every `i` with `i + 1 < steps`
     and `|dev_i| > dec!(1)`, assert `|dev_{i+1}| < |dev_i|`; assert `|dev_{steps−1}| <= dec!(1)`.
     (Provable a priori:
     `|dev_{i+1}| ≤ 0.8·|dev_i| + 0.1 < |dev_i|` for `|dev_i| > 0.5`, and once ≤ 1 it stays ≤ 1.
     If this fails, the loop math was mistyped — fix YOUR code, do not loosen the assert.)
   - `impact_fires_and_decays_geometrically` — spec: `vol_step dec!(0), reversion dec!(0),
     impact_every 5, impact_size dec!(2), impact_decay dec!(0.5), steps 10`. Let
     `d_i = bars_1s[i].close − bars_1s[i].open`: assert `d_0 … d_4 == 0` (flat before the first
     event), `|d_5| == dec!(2)`, `d_6 * dec!(2) == d_5`, `d_7 * dec!(2) == d_6` (exact geometric
     decay, same sign).
   - `regimes_cycle_deterministically` — base spec: `congestion[0] == Calm`,
     `congestion[100] == Busy`, `congestion[200] == Hot`, `congestion[300] == Calm`; each 100-step
     segment is constant.
   - `bad_specs_rejected` — `steps 0`, `mid0 dec!(0)`, `reversion dec!(1)`, `regime_period 0` →
     each `Err(SyntheticError::BadSpec(_))`.
3. In `crates/market-data/src/lib.rs`: add `pub mod synthetic;` between `pub mod intraday;` and
   `pub mod validation;`; add
   `pub use synthetic::{generate, Congestion, SyntheticError, SyntheticIntraday, SyntheticSpec};`
   after the `pub use intraday::…` line.
4. `cargo fmt --all`, then run the gate.

**Gate.**
- `cargo test -p market-data synthetic` → the 7 new tests pass.
- `cargo test -p market-data` → everything passes (C1's tests unchanged).
- Full-workspace gate green: fmt + clippy `-D warnings` + `cargo test --workspace --all-features`
  → **0 failed**.
- `cargo run -q -p cli -- demo | shasum` twice → identical (CLI untouched).
- `git status` → NO `Cargo.toml`/`Cargo.lock` modified (proves no new dependency).

**Guardrails (restated; violating any one = STOP).** All prices/sizes `Decimal`, integer slot/seed
math only — never f64 anywhere. Determinism is the deliverable: the ONLY "randomness" is SplitMix64
on `spec.seed` — a pure integer function; no `rand`, no OS entropy, no clock, no new dependencies
(D-0002). Byte-identity is asserted by test — if it fails, the bug is in YOUR code; never weaken
the assert. Synthetic data must self-identify (the typed `Provenance::Synthetic { spec_hash }`
field stays serialized) and must
never be presented as real market data or ground a statistical claim (invariant 11's rule, extended
to synthetic-vs-real labeling). Library code writes no files. Parallel==sequential byte-identity of
the sweep untouched at thread counts {1,2,3,7,8} (don't touch
`crates/sweep`). Holdout stays sealed (`evaluate_on_holdout` is M5-only, never wired). Strategies
emit intent only — no strategy/cost/execution code here. **No execution code anywhere** — no
keys/signing/submit/RPC/HTTP (M8/M9 need separate explicit human approval). Later-card context,
restated per contract: per-trade cost = venue fee + depth-walk slippage + regime priority + tip,
never base fee alone; adversarial/MEV terms are modeled only as costs to us. Never edit
`schemas/*`, existing fixtures, `plans/master-plan.md`, or `solana-crypto-trader-plan.md`.

**Escalate-if.** This card's Status is still BLOCKED or M-HF-C1 is not DONE; C1's exports differ
from the "Current state" block above (quote what you actually find in the worklog — don't adapt);
you need `dec!` in non-test code (design misfit — stop); `same_spec_is_byte_identical` or the
mean-reversion/impact asserts fail with the pinned numbers (the card's math note is then wrong —
stop and report, do NOT tweak constants until green); clippy `-D warnings` fails on pasted code
for a reason whose mechanical fix would change behavior; any pre-existing test fails.

---

### M-HF-C2.5 — Reuse proof: existing families through the existing engine on 1s bars — `DONE` *(2026-07-13; zero src/manifest changes — the reuse-first bet holds)*

**Goal.** Prove the reuse-first bet (m-hf-track.md §1/§5 row C2.5) with ZERO source changes: the
existing daily-bar families run through the existing `run_sweep` pipeline on C2's synthetic
1-second series, producing a valid, deterministic `SweepReport`. **This is the load-bearing card
of the whole HF track — if it cannot pass without touching `src/`, the bet is wrong and the track
re-plans on the record.**

**Files.** `crates/sweep/tests/hf_reuse_proof.rs` (new — ONE file; test-only). No `src/` change in
any crate; no `Cargo.toml` change (`sweep` already depends on `market-data`, where the C2
generator lives; `jsonschema` + `rust_decimal_macros` are already dev-deps of `sweep`).

**Current state (verbatim, verified 2026-07-13 at the M-HF-C2 verification).**
- `sweep/src/runner.rs:314-321`:
```rust
pub fn run_sweep(
    spec: &SweepSpec,
    bars: Vec<Bar>,
    base_cost: &CostModel,
    initial_cash_usdc: Decimal,
    periods_per_year: f64,
    parallelism: Parallelism,
) -> Result<SweepOutcome, SweepError> {
```
- `sweep/src/runner.rs:274-277`: `pub struct SweepOutcome { pub report: SweepReport, pub sealed:
  Sealed, }` — `Sealed::holdout_read_count()` must be 0 after the sweep.
- `sweep/src/spec.rs:64-71` — `SweepSpec` is all-pub: `{ allowlist_version: String, partition:
  PartitionSpec, walk_forward: WalkForward, thresholds: AdvancementThresholds, grids:
  Vec<ParamGrid> }`.
- `sweep/src/config.rs` — `PartitionSpec::by_index(val_start, holdout_start)` constructor.
- `sweep/src/window.rs:105-111` — `WalkForward::new(kind, train_len, test_len, step, embargo) ->
  Result<Self, WindowError>`; `WindowKind::Rolling`.
- `sweep/src/advance.rs:19-32` — `AdvancementThresholds { drawdown_budget, turnover_budget,
  baseline_margin, dispersion_budget, neighbor_tolerance: Decimal…, min_windows: u32 }`.
- `sweep/src/param.rs:95-107` — `ParamGrid::TrendAlloc { sma_periods: Vec<usize>, weights_above:
  Vec<Decimal>, weights_below: Vec<Decimal> }`, `ParamGrid::ThresholdRebalance {
  target_sol_weights: Vec<Decimal>, bands: Vec<Decimal> }`.
- `market-data/src/synthetic.rs` (C2, DONE) — `pub fn generate(spec: &SyntheticSpec) ->
  Result<SyntheticIntraday, SyntheticError>`; the 1s bars are `out.bars_1s: Vec<Bar>`;
  `SyntheticSpec` fields as in card M-HF-C2.
- Schema-validation convention to mirror: `crates/sweep/tests/schema_validation.rs` (loads
  `schemas/sweep-report.schema.json`, compiles with `jsonschema`, validates `report.to_value()`).
- Determinism-test convention to mirror: `crates/sweep/tests/determinism.rs` (EXPLICIT thread
  counts, never `available_parallelism`).

**Steps.**
1. Create `crates/sweep/tests/hf_reuse_proof.rs` with a module doc stating the bet: "1s bars need
   no new bar type; a 1s signal executing at the next 1s bar's open IS next-bar execution at
   finer grain (m-hf-track §1). This test proves the existing engine runs unmodified on 1s bars —
   ZERO source changes; if it ever needs one, the reuse-first bet is wrong."
2. Helper `hf_bars() -> Vec<Bar>`: `market_data::synthetic::generate(&spec).unwrap().bars_1s`
   with `SyntheticSpec { seed: 7, steps: 4_000, start_unix: 1_609_459_200, start_slot: 100_000,
   venue: "venue_a".to_string(), mid0: dec!(100), anchor: dec!(100), reversion: dec!(0.05),
   vol_step: dec!(0.2), impact_every: 50, impact_size: dec!(1.5), impact_decay: dec!(0.5),
   regime_period: 100 }` → exactly 4,000 1-second bars.
3. Helper `hf_spec() -> SweepSpec`: `allowlist_version: "2026-06-29"`, `partition:
   PartitionSpec::by_index(3_000, 3_600)` (dev 3,000 / val 600 / holdout 400 — a holdout exists
   so the seal is exercised), `walk_forward: WalkForward::new(WindowKind::Rolling, 900, 300, 300,
   5).unwrap()` (windows over the 3,600-bar dev/val: floor((3600−900−5−300)/300)+1 = **8**),
   `thresholds: AdvancementThresholds { drawdown_budget: dec!(0.35), turnover_budget: dec!(12),
   baseline_margin: dec!(0.02), dispersion_budget: dec!(0.40), neighbor_tolerance: dec!(0.15),
   min_windows: 6 }` (illustrative, NOT tuned — this card proves plumbing, not strategy quality),
   `grids: vec![ParamGrid::TrendAlloc { sma_periods: vec![20, 50], weights_above:
   vec![dec!(0.75), dec!(0.5)], weights_below: vec![dec!(0)] }, ParamGrid::ThresholdRebalance {
   target_sol_weights: vec![dec!(0.25), dec!(0.5)], bands: vec![dec!(0.05), dec!(0.1)] }]`
   → 4 + 4 = **8 candidate points**.
4. Helper `run(parallelism) -> (String, u32)`: `run_sweep(&hf_spec(), hf_bars(), &CostModel {
   dex_fee_bps: 5, slippage_bps: 20, base_fee_lamports: 5_000, priority_fee_lamports: 50_000 },
   dec!(10_000), 31_536_000.0, parallelism).unwrap()` → return `(outcome.report.to_json(),
   outcome.sealed.holdout_read_count())`.
5. Exactly these four tests:
   - `existing_engine_runs_unmodified_on_1s_bars` — sequential run: `trial_count > 0`; **8
     verdicts** (one per candidate point), each with a status; anti-vacuous `trial_count >= 64`
     (≥ 8 windows × 8 points; the scenario ladder may multiply it further — assert `>=`, don't
     pin the exact product).
   - `hf_sweep_deterministic_across_thread_counts` — `run(Sequential)` JSON byte-identical to
     `run(Threads(N))` for N in {1, 2, 3, 7, 8} (explicit counts) and to a repeated sequential
     run.
   - `hf_report_validates_against_schema` — mirror `schema_validation.rs`: the report's
     `to_value()` validates against `schemas/sweep-report.schema.json`.
   - `holdout_stays_sealed_on_1s_bars` — `holdout_read_count == 0` from `run(Sequential)` (a
     dedicated test so the seal claim is first-class).
   If the engine's real numbers disagree with this card's arithmetic (window count, verdict
   count, trial floor): fix the TEST's expectation to the observed-and-verified value and record
   the correction in the worklog — NEVER change `src/` to make the card's arithmetic true.
6. `cargo fmt --all`; run the new tests; run the full-workspace gate.

**Gate.** New tests green; full-workspace gate green (expect **> 339** tests, 0 failed); demo
shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a` and template sweep shasum
`7ad3df7de2e2c1139be427e9c953b57d4e289cb3` unchanged; `git status` shows ONLY
`crates/sweep/tests/hf_reuse_proof.rs` (+ queue/worklog edits) — no `src/`, no manifests.

**Guardrails (restated).** ZERO source changes — this card's entire value is proving none are
needed. Decimal money; explicit thread counts; no new deps; holdout counter 0; never edit
`schemas/*`, existing fixtures, the plan pair, or `config/strategies/m5-frozen.toml`.

**Escalate-if.** ANY assertion cannot pass without changing a `src/` file — STOP: that is the
reuse-first bet failing, the single outcome this card exists to detect; record exactly which seam
broke in the worklog and report (the planner re-plans per m-hf-track §5). Also escalate if:
`run_sweep` rejects the 1s series for a reason other than your own spec arithmetic; the report
would validate only after a schema edit (forbidden); demo/sweep hashes move.

---

### M-HF-C2.6 — Streaming/scale proof: `IntradaySource` + columnar reader at ~31.5M rows — `DONE` *(2026-07-13; format + writer/reader + IntradaySource in market-data/src/columnar.rs; 6 unit tests + one `#[ignore]`d 31.5M-row proof; D-0013 recorded)*

**Goal.** Prove the compute/memory contract BEFORE the fill engine exists (m-hf-track §2/§5 row
C2.6): a year of 1s bars (~31.5M rows) lives on disk in a fixed-record columnar file; sweeps
slice only their window (`IntradaySource`), so peak memory ≈ `n_threads × max_window_bars`, never
the whole year. Deliverables: the format + reader/writer, the trait, measured wall-clock + peak
RSS on the full-scale fixture, and the storage-format decision recorded as **D-0013**.

**Files.** `crates/market-data/src/columnar.rs` (new) + one `pub mod`/re-export line in
`crates/market-data/src/lib.rs`; `DECISIONS.md` (new D-0013 entry, newest-first). *(Sanctioned
3-file card; the scale fixture itself is written to a temp dir at test time and NEVER checked in
or placed under `data/`.)*

**Current state (verbatim, verified 2026-07-13 at the post-C2.5 review).**
- `research-core/src/bar.rs:12-21` — `Bar { ts: Timestamp, open/high/low/close/volume: Decimal }`.
- `research-core/src/time.rs:17,23` — `Timestamp::from_unix(secs: i64)`, `.as_unix() -> i64`.
- `market-data/src/synthetic.rs` (C2) — `pub fn generate(spec: &SyntheticSpec) ->
  Result<SyntheticIntraday, SyntheticError>`; `SyntheticSpec` fields as in card M-HF-C2.
- `market-data` deps: research-core, rust_decimal, serde, toml (dev: rust_decimal_macros,
  serde_json). **No new deps** — file I/O via `std::fs`/`std::io` only (m-hf-track §2:
  memmap2/arrow would require their own recorded decision; they are NOT taken here).
- m-hf-track §2 contract (verbatim): "an `IntradaySource` trait (`fn slice(&self, Range<usize>)
  -> Vec<Bar>; fn len(&self)`) — cells slice only their window … Storage: fixed-record columnar
  binary (ts + scaled integer OHLCV, ~40 bytes/row) read via `std::fs` — zero new deps".

**Steps.**
1. `columnar.rs` — the format. Header (24 bytes): magic `b"MCHC"` (4) + version `u32` = 1 (4) +
   price/volume scale `u32` (4) + reserved `u32` = 0 (4) + row count `u64` (8). Row (48 bytes,
   little-endian): `ts: i64` + `open/high/low/close/volume` each an `i64` mantissa at the header
   scale. Conversion is EXACT or an error — never round:
   ```rust
   /// Scale `d` to an integer mantissa at `scale` decimal places; None if it doesn't fit exactly
   /// (lossy storage is a hygiene violation, not a warning).
   fn to_scaled_i64(d: Decimal, scale: u32) -> Option<i64> {
       let mut scaled = d;
       scaled.rescale(scale); // rust_decimal: adjusts exponent, may round
       if scaled != d {
           return None; // rescale rounded ⇒ d does not fit exactly
       }
       scaled.mantissa().try_into().ok()
   }
   fn from_scaled_i64(m: i64, scale: u32) -> Decimal {
       Decimal::new(m, scale)
   }
   ```
2. `ColumnarError` enum (Io(String), BadMagic, BadVersion(u32), ScaleOverflow { row: u64,
   field: &'static str }, RowCountMismatch { header: u64, actual: u64 }, SliceOutOfBounds {
   requested_end: usize, len: usize }) + Display + Error impls, same style as `binance_csv.rs`.
3. Writer: `pub struct ColumnarWriter` — `create(path, scale) -> Result<Self>`, `append_bars(&mut
   self, bars: &[Bar]) -> Result<()>` (streams rows through a `BufWriter`; errors on any inexact
   scale), `finish(mut self) -> Result<u64>` (seeks back, writes the final row count into the
   header, flushes; returns rows written). Chunked appends are the point — the caller never holds
   more than one segment in memory.
4. Reader + trait:
   ```rust
   pub trait IntradaySource {
       fn len(&self) -> usize;
       fn is_empty(&self) -> bool { self.len() == 0 }
       /// Materialize exactly the requested window — never the whole file.
       fn slice(&self, range: std::ops::Range<usize>) -> Result<Vec<Bar>, ColumnarError>;
   }
   pub struct ColumnarFile { /* path or BufReader-wrapped File + header fields */ }
   impl ColumnarFile { pub fn open(path: &Path) -> Result<Self, ColumnarError> { … } }
   impl IntradaySource for ColumnarFile { /* seek to 24 + 48*range.start, read 48*range.len() */ }
   ```
   Open validates magic/version and that `file_len == 24 + 48 * row_count`.
5. Unit tests (in-file, temp-dir paths via `std::env::temp_dir()` + process id, cleaned up):
   round-trip equality on a small C2-generated series (write → open → slice full range → assert
   `== bars_1s`, Decimals exactly equal); slice returns exactly the requested window (spot-check
   first/last bar of a middle window); inexact-scale value rejected at write (e.g. scale 2 with a
   3-dp price → `ScaleOverflow`); corrupt magic and truncated file rejected at open;
   `SliceOutOfBounds` on `len..len+1`.
6. Scale proof — `#[test] #[ignore = "scale proof: ~31.5M rows, run explicitly"] fn
   scale_proof_full_year_1s()`: write 365 daily segments of 86,400 bars each (chained
   `SyntheticSpec`s: segment i uses `seed: 7 + i`, `start_unix: 1_704_067_200 + i * 86_400`,
   steps 86_400 — content doesn't matter, scale does), `finish()` → assert row count
   31_536_000; then via `IntradaySource::slice`, read 1,000 windows of 43,200 bars at
   deterministic offsets and assert first/last timestamps arithmetic-correct per window. Print
   elapsed wall-clock (`std::time::Instant` is fine here — it never touches canonical run
   output) and the file size. Assert the slice path never allocates more than one window
   (structural: the returned Vec is the only allocation — assert `window.len() == 43_200`).
7. Operator/executor runs the proof once, capturing memory:
   `/usr/bin/time -l cargo test -p market-data --release scale_proof_full_year_1s -- --ignored --nocapture 2>&1 | tail -20`
   and records in the worklog: wall-clock (write + read phases), file size (expect ≈ 24 +
   48 × 31_536_000 ≈ 1.41 GiB), and "maximum resident set size". The fixture is written under
   the system temp dir and deleted at test end (`data/` is never touched).
8. `DECISIONS.md` — prepend D-0013 (newest first, house format): context (31.5M-row annual 1s
   series must not materialize per cell; m-hf-track §2 + adjudication must-address) → decision
   (fixed-record columnar binary, 24-byte header + 48-byte rows, exact scaled-integer mantissas,
   std::fs only; `IntradaySource` trait is the sweep-facing seam; memmap2/arrow REJECTED absent
   their own recorded decision, D-0002/D-0009 precedent) → consequences (measured numbers from
   step 7 pasted in; C8 wires the trait into the HF sweep; a format change after real data lands
   (C9) requires a migration note).
9. `cargo fmt --all`; full-workspace gate (the ignored test does not run in it).

**Gate.** Unit tests green; full-workspace gate green (expect **> 343** tests, 0 failed; the
`#[ignore]` proof excluded); the scale proof run ONCE with numbers recorded in worklog +
D-0013; demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a` and template sweep shasum
`7ad3df7de2e2c1139be427e9c953b57d4e289cb3` unchanged; `git status` shows only the card's 3 files
(+ queue/worklog) — no `Cargo.toml`, nothing under `data/`, no checked-in fixture.

**Guardrails (restated).** Exact-or-error scaling — never round on write; Decimal money
elsewhere; no new deps (std::fs/io only; no memmap2/arrow/rayon); the scale fixture lives in the
temp dir and is deleted — never under `data/`, never staged; `Instant` timing is print-only and
must not appear in any canonical-output path; never edit `schemas/*`, existing fixtures, the plan
pair, or `config/strategies/m5-frozen.toml`.

**Escalate-if.** `rescale`/mantissa behavior differs from the step-1 sketch (quote what you find —
don't improvise a rounding fix); the round-trip test fails with exact-looking inputs (format bug —
stop); the scale proof cannot finish in reasonable time (record how far it got + elapsed, stop —
the §7 fallback decision belongs to the planner at C8, not to you); any temptation to check the
fixture in, write under `data/`, or add a dependency.

---

### M-HF-C3 — `run_hf` / `LatencyPipeline`: latency-aware entry point (`simulator.rs` READ-ONLY) — `DONE` *(2026-07-13; 363 passed/0 failed/1 ignored; fixed_latency(1)==run green; thread counts {1,2,3,7,8} identical; demo/sweep shasums unchanged; simulator.rs untouched)*

**Goal.** Give the research engine a latency-aware HF entry point that generalizes the simulator
loop **without touching `run`** (m-hf-track §1 item 1, §3, §5 row C3): a signal decided from
completed bars `[0..=t]` lands `offset_bars ≥ 1` later and executes at the **landing bar's open**
— market state at landing, never at signal. Next-bar execution is the special case
`fixed_latency(1)`, proven equivalent by regression. Also introduces the deterministic
landing-draw primitive (splitmix64 hash stream keyed by `(cell_id, event_index)` — probability
without RNG), proven byte-identical across thread counts where it is introduced. Simulation only
— nothing here creates execution capability.

**Files.** Exactly these; `crates/portfolio/src/simulator.rs` is **READ-ONLY** (you will copy
from it, never write to it — not even a visibility change):
1. `crates/portfolio/src/latency.rs` — NEW (pipeline, draw primitive, `run_hf`, unit tests).
2. `crates/portfolio/src/lib.rs` — wiring only (one `pub mod` + one `pub use` + one doc line).
3. `crates/portfolio/tests/hf_regression.rs` — NEW (equivalence regression + thread-count test).

*(Sanctioned line-budget exception: latency.rs exceeds ~150 lines because `simulator.rs` is
read-only by design, so its private accounting helpers (~90 lines, simulator.rs:17-25, 57-62,
139-246) are duplicated **verbatim** into latency.rs. The `fixed_latency(1) == run` regression
pins the two copies together — if either drifts, the gate breaks. Near-complete code for all new
logic is below; the copied helpers are copy-from-named-lines, no judgment.)*

**Current state (verbatim, copied from source 2026-07-13 at `b0af75c` — if what you find
differs, escalate, don't adapt).**

- `crates/portfolio/src/simulator.rs:65-73` — the signature `run_hf` must mirror:
  ```rust
  pub fn run<F>(
      bars: &[Bar],
      initial_cash_usdc: Decimal,
      cost: &CostModel,
      mut target_fn: F,
  ) -> Result<RunOutput, SimError>
  where
      F: FnMut(&[Bar], Decimal) -> Decimal,
  ```
- `crates/portfolio/src/simulator.rs:89-94` — the hardcoded next-bar block being generalized
  (matches m-hf-track §1's citation exactly):
  ```rust
      for (t, bar) in bars.iter().enumerate() {
          // 1. Execute the target decided from completed bars [0..t] at this bar's OPEN price.
          if t >= 1 {
              let exec_price = bar.open;
              let current_weight = current_weight(&state, exec_price);
              let target = clamp01(target_fn(&bars[..t], current_weight));
  ```
- `crates/portfolio/src/simulator.rs:28` — `RunOutput` derives `#[derive(Debug, Clone,
  PartialEq, Eq)]` (fields :29-44, all pub: equity_curve, round_trips, final_state, n_trades,
  traded_notional_quote, fees_paid_quote, slippage_paid_quote, gas_paid_sol,
  priority_fees_paid_sol, bars_in_market) — so `assert_eq!` on whole outputs is exact.
- Private helpers you will COPY VERBATIM from `simulator.rs` (they build only on pub surface):
  `dust` (:17-20), `min_trade_notional` (:22-25), `struct OpenPosition` (:57-62), `rebalance`
  (:139-184), `record_round_trip` (:186-221, keep its `#[allow(clippy::too_many_arguments)]`),
  `clamp01` (:223-225), `traded_notional` (:227-237), `current_weight` (:239-246).
- `crates/portfolio/src/lib.rs:13-21` — current wiring:
  ```rust
  pub mod cost;
  pub mod equity;
  pub mod simulator;
  pub mod state;

  pub use cost::{BuyFill, CostModel, SellFill, Side};
  pub use equity::{EquityPoint, RoundTrip};
  pub use simulator::{run, RunOutput};
  pub use state::{PortfolioState, SimError, TradeOutcome};
  ```
- `crates/market-data/src/synthetic.rs:126-133` — the C2 primitive you duplicate (it is
  PRIVATE in market-data and stays there; portfolio must not depend on market-data):
  ```rust
  /// SplitMix64 — a deterministic integer hash sequence, NOT an entropy source.
  fn splitmix64(state: &mut u64) -> u64 {
      *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
      let mut z = *state;
      z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
      z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
      z ^ (z >> 31)
  }
  ```
  *(Drift note, on the record: m-hf-track §3 says "the same cell-identity primitive M4 §5.8
  uses for `run_id`" — in the real tree `run_id` is a tuple-formatted String (m4-sweep.md
  §5.8; results/src/lib.rs:17) and no u64 hash primitive exists. This card DEFINES the
  primitive; C8 wires `cell_id` derivation from the canonical cell tuple.)*
- `crates/portfolio/Cargo.toml` — deps: research-core, rust_decimal; dev-deps:
  rust_decimal_macros. **No Cargo.toml/Cargo.lock change is permitted by this card.**
- Pub surface available to latency.rs: `PortfolioState { quote_balance, base_balance }` +
  `new/equity/apply_buy/apply_sell` (state.rs:12-75), `TradeOutcome { side, exec_price,
  base_delta, quote_delta, gas_sol, dex_fee_quote, slippage_quote }` (state.rs:114-121),
  `SimError` (state.rs:126), `CostModel::{gas_sol, priority_sol}` (cost.rs:49,55), `Side`
  (cost.rs:15), `EquityPoint`/`RoundTrip` (equity.rs).

**Pinned semantics (planner decisions — do not re-decide):**
- `landing: Vec<LandingOutcome>` is indexed by **signal index** `t` (one entry per bar;
  `landing.len() == bars.len()`). Signal `t` = decision from completed bars `[0..=t]` (slice
  `&bars[..t + 1]`); it lands at bar `t + offset_bars` and executes at that bar's **open**.
- `min_latency ≥ 1` always. The same-slot-at-signal-price mode is NOT built here — m-hf-track
  §3 allows it only as an explicitly labeled upper-bound scenario, later card.
- `target_fn` is invoked **at landing time**, in landing order (ties on one bar: ascending
  signal index), with `(&bars[..t + 1], current_weight at landing bar's open)` — this exactly
  reproduces `run`'s argument stream under `fixed_latency(1)` (that call order is what makes
  stateful `FnMut` closures bar-for-bar equal). Overtaking (a later signal landing earlier) is
  legal and handled by the schedule.
- Un-landed (in-range, `landed == false`): no `target_fn` call, no balance change;
  `unlanded_orders += 1`. The attempted-cost **Decimal** term is C4's job (cost-model card) —
  C3 counts attempts only. Adversarial terms are COSTS to us only, never a benefit.
- Off-end (`t + offset_bars >= bars.len()`): dropped silently, NOT counted as un-landed —
  the generalization of `run`'s "final bar's signal is never executed" rule.
- Equity is marked at every bar's close, identical to `run` (including bar 0).
- `run_hf` returns `HfRunOutput { base: RunOutput, unlanded_orders: u32 }`; with
  `fixed_latency(1)`, `base` equals `run`'s output exactly and `unlanded_orders == 0`.

**Steps.**
1. **Step 0 (before touching any file):** run the full-workspace gate. Expect **349 passed,
   0 failed, 1 ignored**; fmt/clippy clean; demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a`;
   sweep shasum `7ad3df7de2e2c1139be427e9c953b57d4e289cb3`. If anything is already red, STOP and
   report the baseline failure in the worklog.
2. Create `crates/portfolio/src/latency.rs` — module doc + imports + types + validation:
   ```rust
   //! Latency-aware HF entry point (m-hf-track §3): `run_hf` generalizes the simulator loop
   //! WITHOUT touching `simulator::run` — next-bar execution is the special case
   //! `fixed_latency(1)`, proven equivalent by regression (tests/hf_regression.rs).
   //!
   //! Event ordering: the signal decided from completed bars `[0..=t]` lands at bar
   //! `t + offset_bars` (offset ≥ min_latency ≥ 1) and executes at that bar's OPEN — market
   //! state at landing, never at signal. Un-landed orders change no balances; their attempted
   //! count is recorded (the attempted-cost term is priced by the C4 cost fields). Latency is
   //! measured in BARS (slots at 1s resolution), never wall-clock. Landing outcomes are
   //! deterministic by construction: a splitmix64 hash stream keyed by `(cell_id,
   //! event_index)` — never RNG, thread id, evaluation order, or clock.

   use crate::cost::{CostModel, Side};
   use crate::equity::{EquityPoint, RoundTrip};
   use crate::simulator::RunOutput;
   use crate::state::{PortfolioState, SimError, TradeOutcome};
   use research_core::{Bar, Decimal, Timestamp};
   use std::fmt;

   /// Landing outcome for one signal index; `offset_bars` ≥ the pipeline's `min_latency`.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub struct LandingOutcome {
       pub offset_bars: usize,
       pub landed: bool,
   }

   /// Deterministic latency pipeline: one [`LandingOutcome`] per signal index (m-hf-track §3).
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct LatencyPipeline {
       min_latency: usize,
       landing: Vec<LandingOutcome>,
   }

   impl LatencyPipeline {
       /// `min_latency` ≥ 1 always: the same-slot-at-signal-price mode is NOT built here —
       /// it may only ever exist as an explicitly labeled upper-bound scenario (§3).
       pub fn new(min_latency: usize, landing: Vec<LandingOutcome>) -> Result<Self, HfError> {
           if min_latency == 0 {
               return Err(HfError::ZeroMinLatency);
           }
           if let Some(index) = landing.iter().position(|l| l.offset_bars < min_latency) {
               return Err(HfError::OffsetBelowMin { index });
           }
           Ok(Self { min_latency, landing })
       }

       #[must_use]
       pub fn min_latency(&self) -> usize {
           self.min_latency
       }

       #[must_use]
       pub fn landing(&self) -> &[LandingOutcome] {
           &self.landing
       }
   }

   /// Every signal lands, `offset_bars` later — `fixed_latency(1, n)` IS next-bar execution.
   #[must_use]
   pub fn fixed_latency(offset_bars: usize, n_signals: usize) -> LatencyPipeline {
       assert!(offset_bars >= 1, "same-slot execution is not modeled (m-hf-track §3)");
       LatencyPipeline {
           min_latency: offset_bars,
           landing: vec![LandingOutcome { offset_bars, landed: true }; n_signals],
       }
   }

   /// Errors from the HF entry point. `SimError` stays untouched (simulator.rs is read-only).
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum HfError {
       /// Underlying simulation/accounting error.
       Sim(SimError),
       /// One landing outcome per potential signal index: `landing.len() == bars.len()`.
       PipelineLength { landing: usize, bars: usize },
       /// `min_latency` must be ≥ 1 (no same-slot execution).
       ZeroMinLatency,
       /// `landing[index].offset_bars` < `min_latency`.
       OffsetBelowMin { index: usize },
       /// Landing probability must be an exact rational with `den ≥ 1` and `num ≤ den`.
       BadProbability { num: u64, den: u64 },
   }

   impl fmt::Display for HfError {
       fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
           match self {
               Self::Sim(e) => write!(f, "simulation error: {e}"),
               Self::PipelineLength { landing, bars } => {
                   write!(f, "pipeline has {landing} landing outcomes for {bars} bars")
               }
               Self::ZeroMinLatency => write!(f, "min_latency must be >= 1"),
               Self::OffsetBelowMin { index } => {
                   write!(f, "landing[{index}].offset_bars is below min_latency")
               }
               Self::BadProbability { num, den } => {
                   write!(f, "landing probability {num}/{den} is not a valid rational in [0,1]")
               }
           }
       }
   }

   impl std::error::Error for HfError {}

   impl From<SimError> for HfError {
       fn from(e: SimError) -> Self {
           Self::Sim(e)
       }
   }
   ```
3. The deterministic landing-draw primitive (append to latency.rs):
   ```rust
   /// SplitMix64 — a deterministic integer hash sequence, NOT an entropy source. Verbatim
   /// twin of the C2 generator's private fn (market-data/src/synthetic.rs:127-133),
   /// duplicated deliberately (pattern-level reuse, m-hf-track §1): a shared home would
   /// couple portfolio to market-data for seven lines of integer arithmetic.
   fn splitmix64(state: &mut u64) -> u64 {
       *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
       let mut z = *state;
       z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
       z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
       z ^ (z >> 31)
   }

   /// One landing draw — a pure function of `(cell_id, event_index)` and NOTHING else:
   /// provably independent of thread id, evaluation order, and wall clock (m-hf-track §3).
   fn landing_draw(cell_id: u64, event_index: u64) -> u64 {
       let mut state = cell_id;
       let mixed_cell = splitmix64(&mut state);
       let mut keyed = mixed_cell ^ event_index;
       splitmix64(&mut keyed)
   }

   /// Probability without RNG: `landed ⇔ draw % den < num`, `p = num/den` exact (§3).
   /// Fail-closed: an invalid rational is an error, never a default.
   ///
   /// `event_indices` are the stable, GLOBAL, fixture-relative indices, computed once before
   /// windowing — never window-local (else one physical slot's outcome could differ per
   /// cell). C8 is responsible for passing fixture-global ranges; the signature makes
   /// window-local indexing impossible to do silently.
   pub fn build_landing_table(
       cell_id: u64,
       event_indices: std::ops::Range<u64>,
       offset_bars: usize,
       p_land_num: u64,
       p_land_den: u64,
   ) -> Result<Vec<LandingOutcome>, HfError> {
       if p_land_den == 0 || p_land_num > p_land_den {
           return Err(HfError::BadProbability { num: p_land_num, den: p_land_den });
       }
       Ok(event_indices
           .map(|event_index| LandingOutcome {
               offset_bars,
               landed: (landing_draw(cell_id, event_index) % p_land_den) < p_land_num,
           })
           .collect())
   }
   ```
4. Copy the private helpers VERBATIM from `simulator.rs` into latency.rs under this header
   (ranges: :17-20, :22-25, :57-62, :139-184, :186-221, :223-225, :227-237, :239-246; keep
   every doc comment and the `#[allow(clippy::too_many_arguments)]`; change nothing):
   ```rust
   // ── Verbatim twins of simulator.rs's private accounting helpers ──────────────────────
   // simulator.rs is READ-ONLY by design (m-hf-track §1: "without touching run"), so its
   // private helpers are duplicated here rather than made pub(crate). The fixed_latency(1)
   // equality regression (tests/hf_regression.rs) pins the two copies together: if either
   // copy drifts, that gate breaks.
   ```
5. `run_hf` itself (append to latency.rs):
   ```rust
   /// Everything one HF run produces: the base accounting plus HF-only counters. With
   /// `fixed_latency(1)`, `base` equals `run`'s output exactly (the regression gate) and
   /// the HF counters are zero.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct HfRunOutput {
       pub base: RunOutput,
       /// In-range attempts that did not land (`landed == false`). Balances untouched; the
       /// attempted COST (a cost to us, never a benefit) is priced when C4's fields land.
       pub unlanded_orders: u32,
   }

   /// Run the deterministic latency-aware simulation (m-hf-track §3).
   ///
   /// The signal decided from completed bars `[0..=t]` executes at bar
   /// `t + landing[t].offset_bars`'s OPEN — market state at landing, never at signal.
   pub fn run_hf<F>(
       bars: &[Bar],
       initial_cash_usdc: Decimal,
       cost: &CostModel,
       pipeline: &LatencyPipeline,
       mut target_fn: F,
   ) -> Result<HfRunOutput, HfError>
   where
       F: FnMut(&[Bar], Decimal) -> Decimal,
   {
       if bars.is_empty() {
           return Err(HfError::Sim(SimError::NoBars));
       }
       if pipeline.landing.len() != bars.len() {
           return Err(HfError::PipelineLength {
               landing: pipeline.landing.len(),
               bars: bars.len(),
           });
       }

       // Landing schedule: land_at[l] = signal indices t with t + offset == l, ascending t
       // (ties execute in signal order; overtaking — a later signal landing earlier — is
       // legal). Off-end signals never land: the generalization of run's final-bar rule.
       let mut land_at: Vec<Vec<usize>> = vec![Vec::new(); bars.len()];
       let mut unlanded_orders = 0u32;
       for (t, outcome) in pipeline.landing.iter().enumerate() {
           match t.checked_add(outcome.offset_bars) {
               Some(l) if l < bars.len() => {
                   if outcome.landed {
                       land_at[l].push(t);
                   } else {
                       unlanded_orders += 1;
                   }
               }
               _ => {}
           }
       }

       let mut state = PortfolioState::new(initial_cash_usdc);
       let mut equity_curve = Vec::with_capacity(bars.len());
       let mut round_trips = Vec::new();
       let mut open: Option<OpenPosition> = None;
       let mut n_trades = 0u32;
       let mut traded_notional_quote = Decimal::ZERO;
       let mut fees_paid_quote = Decimal::ZERO;
       let mut slippage_paid_quote = Decimal::ZERO;
       let mut gas_paid_sol = Decimal::ZERO;
       let mut bars_in_market = 0u32;

       for (l, bar) in bars.iter().enumerate() {
           // 1. Execute every order landing at this bar, at ITS open — the target is decided
           //    from the signal-time history `bars[..=t]` but priced at landing.
           for &t in &land_at[l] {
               let exec_price = bar.open;
               let weight_now = current_weight(&state, exec_price);
               let target = clamp01(target_fn(&bars[..t + 1], weight_now));
               let was_flat = state.base_balance <= dust();
               let quote_before = state.quote_balance;
               if let Some(outcome) = rebalance(&mut state, target, exec_price, cost)? {
                   n_trades += 1;
                   traded_notional_quote += traded_notional(&outcome);
                   fees_paid_quote += outcome.dex_fee_quote;
                   slippage_paid_quote += outcome.slippage_quote;
                   gas_paid_sol += outcome.gas_sol;
                   record_round_trip(
                       &mut open,
                       &mut round_trips,
                       &state,
                       &outcome,
                       bar.ts,
                       was_flat,
                       quote_before,
                   );
               }
           }
           // 2. Mark equity at this bar's CLOSE (identical to `run`).
           equity_curve.push(EquityPoint {
               ts: bar.ts,
               equity_quote: state.equity(bar.close),
           });
           if state.base_balance > dust() {
               bars_in_market += 1;
           }
       }

       let priority_fees_paid_sol = cost.priority_sol() * Decimal::from(n_trades);
       Ok(HfRunOutput {
           base: RunOutput {
               equity_curve,
               round_trips,
               final_state: state,
               n_trades,
               traded_notional_quote,
               fees_paid_quote,
               slippage_paid_quote,
               gas_paid_sol,
               priority_fees_paid_sol,
               bars_in_market,
           },
           unlanded_orders,
       })
   }
   ```
   *(Note: `rebalance(...)? ` works because of `impl From<SimError> for HfError`.)*
6. Unit tests in latency.rs (`#[cfg(test)] mod tests`, `use rust_decimal_macros::dec;`).
   Series helper — 1s-spaced bars with **open ≠ close** so "executes at OPEN" is
   distinguishable (OHLC-valid: `open = p, close = p + 2, high = p + 3, low = p − 1`):
   ```rust
   fn series(prices: &[i64]) -> Vec<Bar> {
       prices
           .iter()
           .enumerate()
           .map(|(i, p)| {
               let open = Decimal::from(*p);
               Bar {
                   ts: Timestamp::from_unix(1_700_000_000 + i as i64),
                   open,
                   high: open + dec!(3),
                   low: open - dec!(1),
                   close: open + dec!(2),
                   volume: dec!(1),
               }
           })
           .collect()
   }
   ```
   Tests (names pinned):
   - `fixed_latency_one_is_all_landed_offset_one` — `fixed_latency(1, 3)`: len 3, every
     outcome `{ offset_bars: 1, landed: true }`, `min_latency() == 1`.
   - `zero_min_latency_rejected` — `LatencyPipeline::new(0, vec![])` → `Err(ZeroMinLatency)`.
   - `offset_below_min_rejected` — `new(2, vec![LandingOutcome { offset_bars: 1, landed:
     true }])` → `Err(OffsetBelowMin { index: 0 })`.
   - `pipeline_length_mismatch_rejected` — 4 bars, `fixed_latency(1, 3)` →
     `Err(PipelineLength { landing: 3, bars: 4 })`.
   - `empty_bars_rejected` — `run_hf(&[], …, &fixed_latency(1, 0), …)` →
     `Err(HfError::Sim(SimError::NoBars))`.
   - `bad_probability_rejected` — `build_landing_table(1, 0..10, 1, 1, 0)` and `(1, 0..10,
     1, 3, 2)` both → `Err(BadProbability { .. })`.
   - `landing_table_is_pure_and_keyed_by_cell` — same args twice → equal; `cell_id` 1 vs 2
     over `0..1000` with `p = 1/2` → tables differ.
   - `latency_two_executes_at_landing_open` — hand-computed, zero cost, series
     `[100, 200, 250, 400]`, cash 10 000, always-1 strategy, `fixed_latency(2, 4)`:
     signals t=0,1 land at bars 2,3; t=2,3 fall off-end. The t=0 order buys at bar 2's
     open **250** (NOT bar 1's 200 — the discriminating assertion) → exactly 40 SOL; the
     t=1 landing finds weight already 1 → no trade. Assert: `n_trades == 1`;
     `final_state.base_balance == dec!(40)`; `unlanded_orders == 0`;
     `equity_curve[1].equity_quote == dec!(10000)` (still flat at bar 1 close — under
     latency 1 it would be 10 100); `equity_curve[2].equity_quote == dec!(10080)`
     (40 × close 252); `equity_curve[3].equity_quote == dec!(16080)` (40 × close 402).
   - `unlanded_changes_no_balances_but_is_counted` — same series/strategy/cost,
     `LatencyPipeline::new(1, vec![{1,false},{1,true},{1,false},{1,true}])` (struct literals
     spelled out): t=0 and t=2 are in-range drops → counted; t=1 lands at bar 2 (buy 40 SOL
     at 250); t=3 is off-end (landed=true but dropped, NOT counted). Assert:
     `unlanded_orders == 2`; `n_trades == 1`; `final_state.base_balance == dec!(40)`;
     `equity_curve[1].equity_quote == dec!(10000)`.
   - `repeated_runs_are_deterministic` — same inputs twice → `assert_eq!` on `HfRunOutput`.
7. Wire `crates/portfolio/src/lib.rs`: add `pub mod latency;` to the module list (alphabetical:
   between `equity` and `simulator`), add
   `pub use latency::{build_landing_table, fixed_latency, run_hf, HfError, HfRunOutput, LandingOutcome, LatencyPipeline};`
   after the existing `pub use equity::…` line, and append one line to the module doc's layer
   list: `//! - [`latency::run_hf`]: the latency-aware HF entry point (m-hf-track §3); next-bar
   [`simulator::run`] is its `fixed_latency(1)` special case, proven by regression.`
8. Create `crates/portfolio/tests/hf_regression.rs`:
   ```rust
   //! M-HF-C3 gate: `run_hf(fixed_latency(1)) == run` bar-for-bar (orders, fills, balances,
   //! equity — exact Decimal equality), and the landing-table primitive is identical across
   //! explicit thread counts {1, 2, 3, 7, 8} (never `available_parallelism`; m4-sweep §5.6).

   use portfolio::{
       build_landing_table, fixed_latency, run, run_hf, CostModel, LandingOutcome,
   };
   use research_core::{Bar, Decimal, Timestamp};
   use rust_decimal_macros::dec;

   // (same `series` helper as latency.rs's tests — open = p, close = p + 2, 1s spacing)

   type Strat = Box<dyn FnMut(&[Bar], Decimal) -> Decimal>;

   fn constant_half() -> Strat {
       Box::new(|_h, _w| dec!(0.5))
   }
   fn stepper() -> Strat {
       let mut step = 0u32;
       Box::new(move |_h, _w| {
           step += 1;
           if step % 3 == 0 { dec!(0) } else { dec!(1) }
       })
   }
   fn banded() -> Strat {
       Box::new(|_h, w| if (w - dec!(0.5)).abs() > dec!(0.1) { dec!(0.5) } else { w })
   }

   fn shapes() -> Vec<Vec<Bar>> {
       vec![
           series(&[100, 110, 121, 133, 146, 160]), // rising
           series(&[160, 146, 133, 121, 110, 100]), // falling
           series(&[100, 140, 90, 150, 80, 160]),   // choppy
           series(&[100, 100, 100, 100, 100, 100]), // flat
       ]
   }
   fn costs() -> Vec<CostModel> {
       vec![
           CostModel::zero(),
           CostModel {
               dex_fee_bps: 5,
               slippage_bps: 20,
               base_fee_lamports: 5_000,
               priority_fee_lamports: 50_000,
           },
       ]
   }

   fn assert_fixed1_equals_run(mut mk: impl FnMut() -> Strat) {
       for bars in shapes() {
           for cost in costs() {
               let baseline = run(&bars, dec!(10_000), &cost, mk()).unwrap();
               let hf = run_hf(&bars, dec!(10_000), &cost, &fixed_latency(1, bars.len()), mk())
                   .unwrap();
               assert_eq!(hf.base, baseline, "run_hf(fixed_latency(1)) must equal run");
               assert_eq!(hf.unlanded_orders, 0);
           }
       }
   }

   #[test]
   fn fixed_latency_one_equals_run_constant_half() {
       assert_fixed1_equals_run(constant_half);
   }
   #[test]
   fn fixed_latency_one_equals_run_stateful_stepper() {
       assert_fixed1_equals_run(stepper);
   }
   #[test]
   fn fixed_latency_one_equals_run_weight_banded() {
       assert_fixed1_equals_run(banded);
   }

   #[test]
   fn landing_table_identical_across_thread_counts() {
       const N: u64 = 100_000;
       let cell_id = 0x00C0_FFEE_u64;
       let sequential = build_landing_table(cell_id, 0..N, 3, 7, 10).unwrap();
       // Non-degenerate: both outcomes occur, so the equality below is discriminating.
       assert!(sequential.iter().any(|l| l.landed));
       assert!(sequential.iter().any(|l| !l.landed));
       for &k in &[1usize, 2, 3, 7, 8] {
           let chunk = (N + k as u64 - 1) / k as u64;
           let mut parts: Vec<Vec<LandingOutcome>> = Vec::new();
           std::thread::scope(|s| {
               let handles: Vec<_> = (0..k as u64)
                   .map(|i| {
                       let start = i * chunk;
                       let end = ((i + 1) * chunk).min(N);
                       s.spawn(move || {
                           if start >= end {
                               Vec::new()
                           } else {
                               build_landing_table(cell_id, start..end, 3, 7, 10).unwrap()
                           }
                       })
                   })
                   .collect();
               for h in handles {
                   parts.push(h.join().unwrap());
               }
           });
           assert_eq!(parts.concat(), sequential, "thread count {k} diverged");
       }
   }
   ```
9. `cargo fmt --all`; run the full gate (below); flip this card to `DONE` + one worklog line.

**Gate.**
- `cargo test -p portfolio` — all new tests green AND every pre-existing test green,
  name-for-name (the M2 suite is untouched: `git diff --name-only` shows `latency.rs`,
  `lib.rs`, `tests/hf_regression.rs` and NO other portfolio file — `simulator.rs`, `state.rs`,
  `cost.rs`, `equity.rs` unchanged).
- The three `fixed_latency_one_equals_run_*` tests prove `run_hf(fixed_latency(1)) == run`
  bar-for-bar by full-struct `assert_eq!` (orders, fills, balances, equity curve — exact
  Decimal equality via `RunOutput: PartialEq + Eq`).
- `landing_table_identical_across_thread_counts` green with explicit counts {1,2,3,7,8} —
  never `available_parallelism`.
- Full workspace: `cargo fmt --all --check` clean; `cargo clippy --all-targets --all-features
  -- -D warnings` clean; `cargo test --workspace --all-features` → **0 failed, 1 ignored,
  > 349 passed** (record the exact new count in the worklog).
- `cargo run -p cli -- demo | shasum` → `ae064f79242f823ffd8f55bf9104e3e1b45d425a` unchanged;
  `cargo run -p cli -- sweep | shasum` → `7ad3df7de2e2c1139be427e9c953b57d4e289cb3` unchanged;
  `cargo run -p cli -- sweep-verify` → OK.
- `git status` shows exactly the card's 3 files (+ this queue/worklog flip) — no `Cargo.toml`,
  no `Cargo.lock`, nothing else.

**Guardrails (restated).** `Decimal` for all money — `u64`/`usize` only for indices and hash
values, never money; **no new deps, no Cargo.toml/Cargo.lock edits**; **no RNG and no clock**
anywhere in this card (`rand`, `Instant`, `SystemTime` must not appear — splitmix64 hash
streams only); latency in bars/slots, never wall-clock; adversarial terms are costs to us
only; `min_latency ≥ 1` (no same-slot execution); **`simulator.rs` READ-ONLY** — copy from
it, never write; never touch `schemas/`, existing fixtures, `config/strategies/m5-frozen.toml`,
the master-plan pair (`plans/master-plan.md`/`solana-crypto-trader-plan.md`), or the holdout
machinery (`sweep::partition` and everything around it); strategies remain intent-only;
NOTHING here creates execution capability (no keys/signing/submit/RPC/network — M8/M9 need
separate explicit human approval). Never `git commit`/`git push` — the operator commits.

**Escalate-if (STOP, record in plans/worklog.md, report — never improvise):** any quoted
signature/line above doesn't match the source; **any impulse or apparent need to edit
`simulator.rs`** (even one character, even visibility); any `fixed_latency_one_equals_run_*`
assertion fails (do NOT weaken the assertion, do NOT "fix" `run` — the pipeline semantics or
this card are wrong); any pre-existing (M2/simulator) test fails or needs modification;
`landing_table_identical_across_thread_counts` fails (the draw primitive is order-dependent —
a design-level break); the demo/sweep shasum moves; an unlisted file seems needed;
rust_decimal_macros is missing from portfolio's dev-deps.

---

### M-HF-C4 — HF cost model fields (depth-walk, regime priority, tip) — `DONE` *(2026-07-15; one escalation — a card transcription slip (0.05 vs 0.0005 base-fee floor) caught by the executor's hand-recompute, ruled and corrected by the planner; gate re-run green)*

**Goal.** Give the research engine the additive HF cost terms m-hf-track §3/§5 row C4 calls for —
depth-walk slippage (never a flat bps constant as the HF base case), congestion-regime-keyed
priority fee, and a tip — so a reference trade's total cost is hand-verifiable and **provably
greater than the Solana base-fee floor alone** (the classic fabricated-edge failure the base fee
alone would hide). Simulation only — nothing here creates execution capability, and nothing here
wires these costs into `run`/`run_hf` yet (that is a later card's job once cell/regime derivation
exists — C8 per m-hf-track §3's own note on `cell_id`).

**Planner decision, logged here (read before objecting or trying to "fix" it):** m-hf-track §3
says "`CostModel` gains `depth_curve`, `congestion_priority_table`, `tip_bps` — additive." Adding
non-`Option` fields directly to the existing `portfolio::CostModel` struct is **not actually
additive in practice** — grep shows ~30 `CostModel { .. }` struct-literal construction sites
across `portfolio`, `sweep`, `results`, and `cli` (including inside `simulator.rs`'s own
`#[cfg(test)]` module), every one of which would stop compiling and need a mechanical edit. That
would force touching `simulator.rs` — violating the reuse-first bet's standing rule ("`run_hf`
generalizes the loop WITHOUT touching `run`", m-hf-track §1/§3), which this track has held since
C3 and is not scoped to expire. **Decision: a new, separate `HfCostModel` wrapper struct
(`base: CostModel` plus the three new HF-only fields) carries the additive terms instead of
extending `CostModel` itself.** Zero existing files change; `CostModel` and every one of its ~30
call sites are untouched. This is an internal, reversible fork (AGENTS.md: agent decides and logs
these) — not a re-litigation of the spec's intent, which is preserved (the three fields exist,
are additive, and are scaled the same way `scale_cost_model` scales `CostModel`). Wiring
`HfCostModel` into `run_hf`, and the SweepSpec-level "HF-kind requires `depth_curve`" validation,
are explicitly deferred to later cards (C5/C8) — this card only defines the types and pure cost
function, proven by a hand-computed reference trade.

**Files.** Exactly these two; `portfolio::CostModel`/`cost.rs`, `simulator.rs`, and every existing
`CostModel` call site are untouched:
1. `crates/portfolio/src/hf_cost.rs` — NEW (depth curve, congestion table, `HfCostModel`,
   `hf_trade_cost`, unit tests).
2. `crates/portfolio/src/lib.rs` — wiring only (one `pub mod` + one `pub use` + one doc line).

**Current state (verbatim, copied from source 2026-07-14 — if what you find differs, escalate,
don't adapt).**
- `crates/portfolio/src/cost.rs:24-33` — `CostModel { dex_fee_bps: u32, slippage_bps: u32,
  base_fee_lamports: i64, priority_fee_lamports: i64 }`, all `pub`. **Untouched by this card.**
- `crates/research-core/src/money.rs:22,28,37` — `pub fn quantize_floor(value: Decimal, decimals:
  u32) -> Decimal`; `pub fn lamports_to_sol(lamports: i64) -> Decimal`; `pub fn apply_bps(value:
  Decimal, bps: u32) -> Decimal`; constants `USDC_DECIMALS: u32 = 6`, `SOL_DECIMALS: u32 = 9`,
  `LAMPORTS_PER_SOL: i64 = 1_000_000_000` (money.rs:11-14). All exact `Decimal`/integer, no
  floating point.
- `crates/portfolio/Cargo.toml` — deps: research-core, rust_decimal; dev-deps:
  rust_decimal_macros. **No Cargo.toml/Cargo.lock change is permitted by this card.**
- `crates/portfolio/src/lib.rs:13-24` — current wiring (post-C3):
  ```rust
  pub mod cost;
  pub mod equity;
  pub mod latency;
  pub mod simulator;
  pub mod state;

  pub use cost::{BuyFill, CostModel, SellFill, Side};
  pub use equity::{EquityPoint, RoundTrip};
  pub use latency::{
      build_landing_table, fixed_latency, run_hf, HfError, HfRunOutput, LandingOutcome,
      LatencyPipeline,
  };
  pub use simulator::{run, RunOutput};
  pub use state::{PortfolioState, SimError, TradeOutcome};
  ```
- m-hf-track §4 (for context, not built here): `scale_cost_model`/`ScenarioId` live in
  `crates/sweep/src/sensitivity.rs:60-115` — the ladder wiring for these new fields is C6's job,
  not this card's; this card only provides a scaling helper for `HfCostModel` for C6 to reuse.

**Pinned semantics (planner decisions — do not re-decide):**
- `DepthCurve` is a **piecewise-constant step function of trade notional** (quote/USDC terms),
  never a flat bps constant — that constant-bps mode still exists (it is `CostModel.slippage_bps`,
  untouched, and remains a ladder rung per m-hf-track §3, not built here). Bands are sorted
  ascending by `notional_upto`; the curve returns the first band whose `notional_upto >=
  notional`, or the **last** band's `impact_bps` if `notional` exceeds every band (walking off
  the end of a depth curve costs at least as much as its deepest quoted level — never less).
- `CongestionRegime` is `{ Calm, Busy, Hot }` — a plain enum here; deriving a regime from trailing
  volatility (m-hf-track §3's "non-lookahead function of trailing realized volatility/print
  density") is **not built in this card** — that is C8's job once intraday series exist end to
  end. `CongestionPriorityTable` is a fixed lookup (`Calm|Busy|Hot -> lamports`), not (yet) the
  checked-in TOML percentile table m-hf-track §3 describes for the landing/drop draw — this card
  supplies only the priority-fee-side lookup for cost pricing; the fee-conditional landing-percentile
  table wiring is deferred (C8, per C3's card's own drift note on `cell_id`).
- `tip_bps` prices a tip as basis points of trade notional (quote terms), same mechanism as
  `dex_fee_bps` — **not lamports**, despite m-hf-track §2.4's `tip(lamports)` phrasing: a
  fixed-lamport tip cannot scale with trade size the way a real priority-auction tip does, and
  bps-of-notional is the same unit convention `dex_fee_bps` already uses, so `hf_trade_cost` stays
  internally consistent. Recorded here as a deviation, not silently taken.
- `hf_trade_cost` is a **pure function of `(HfCostModel, notional_quote, price, regime)`** — no
  RNG, no clock, no side (buy/sell) branching (depth-walk and tip apply symmetrically; venue fee
  and gas already don't branch on side in the existing `CostModel` either, at the notional level).
  Its `total_quote` is the fee/slippage/tip terms (quote) plus the gas terms
  (`base_fee_lamports + regime priority`, converted to quote via `lamports_to_sol(..) * price`) —
  one common unit, so `total_quote > base_fee_floor_quote` is a well-formed comparison (the base
  fee alone, in the same quote terms).
- Errors are fail-closed: an empty or unsorted `DepthCurve` is a construction error, never a
  silent default.

**Steps.**
1. **Step 0 (before touching any file):** run the full-workspace gate. Expect **363 passed,
   0 failed, 1 ignored**; fmt/clippy clean; demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a`;
   sweep shasum `7ad3df7de2e2c1139be427e9c953b57d4e289cb3`. If anything is already red, STOP and
   report the baseline failure in the worklog.
2. Create `crates/portfolio/src/hf_cost.rs` — module doc + imports + depth curve:
   ```rust
   //! Additive HF cost-model fields (m-hf-track §3/§4/§5 row C4): depth-walk slippage,
   //! congestion-regime-keyed priority fee, and a tip. Carried on a separate [`HfCostModel`]
   //! wrapper rather than added to [`CostModel`] directly (planner decision, logged on the
   //! M-HF-C4 card: extending `CostModel`'s own fields would force edits to ~30 existing
   //! construction sites, including inside `simulator.rs`'s tests, breaking the reuse-first
   //! bet's standing rule that `simulator.rs` is never touched).
   //!
   //! Nothing here is wired into `run`/`run_hf` yet — this card defines the types and a pure
   //! cost function, proven by a hand-computed reference trade. Wiring into execution and the
   //! SweepSpec-level "HF-kind requires `depth_curve`" validation are later cards (C5/C8).

   use crate::cost::CostModel;
   use research_core::money::{apply_bps, lamports_to_sol};
   use research_core::Decimal;
   use std::fmt;

   /// One step of a piecewise-constant depth curve: trades up to `notional_upto` (quote terms)
   /// incur `impact_bps` of slippage. Bands are sorted ascending by `notional_upto`.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub struct DepthBand {
       pub notional_upto: Decimal,
       pub impact_bps: u32,
   }

   /// A depth-walk slippage curve (m-hf-track §3: "constant-bps slippage ... is structurally
   /// forbidden [as the HF base case]; it survives only as a ladder rung"). Never empty.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct DepthCurve {
       bands: Vec<DepthBand>,
   }

   impl DepthCurve {
       /// `bands` must be non-empty and sorted strictly ascending by `notional_upto`.
       pub fn new(bands: Vec<DepthBand>) -> Result<Self, HfCostError> {
           if bands.is_empty() {
               return Err(HfCostError::EmptyDepthCurve);
           }
           if bands.windows(2).any(|w| w[0].notional_upto >= w[1].notional_upto) {
               return Err(HfCostError::UnsortedDepthCurve);
           }
           Ok(Self { bands })
       }

       /// Impact in bps for a trade of `notional` (quote terms): the first band whose
       /// `notional_upto >= notional`, or the deepest (last) band if `notional` walks off the
       /// end of the curve — walking past the quoted depth never costs less than the deepest
       /// quoted level.
       #[must_use]
       pub fn impact_bps_for(&self, notional: Decimal) -> u32 {
           self.bands
               .iter()
               .find(|b| b.notional_upto >= notional)
               .unwrap_or_else(|| self.bands.last().expect("DepthCurve is never empty"))
               .impact_bps
       }
   }

   /// Congestion regime a trade lands in (m-hf-track §3). Deriving this from trailing
   /// volatility/print density is NOT built here — that is C8's job.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum CongestionRegime {
       Calm,
       Busy,
       Hot,
   }

   /// Regime-keyed priority-fee lookup, in lamports (m-hf-track §3's fee-conditional table,
   /// priority-fee side only — the landing-percentile side is deferred to C8).
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub struct CongestionPriorityTable {
       pub calm_lamports: i64,
       pub busy_lamports: i64,
       pub hot_lamports: i64,
   }

   impl CongestionPriorityTable {
       #[must_use]
       pub fn priority_lamports_for(&self, regime: CongestionRegime) -> i64 {
           match regime {
               CongestionRegime::Calm => self.calm_lamports,
               CongestionRegime::Busy => self.busy_lamports,
               CongestionRegime::Hot => self.hot_lamports,
           }
       }
   }

   /// Errors constructing HF cost types. Fail-closed: never a silent default.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum HfCostError {
       /// `DepthCurve` must have at least one band.
       EmptyDepthCurve,
       /// `DepthCurve` bands must be sorted strictly ascending by `notional_upto`.
       UnsortedDepthCurve,
   }

   impl fmt::Display for HfCostError {
       fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
           match self {
               Self::EmptyDepthCurve => write!(f, "depth curve must have at least one band"),
               Self::UnsortedDepthCurve => {
                   write!(f, "depth curve bands must be sorted ascending by notional_upto")
               }
           }
       }
   }

   impl std::error::Error for HfCostError {}
   ```
3. `HfCostModel` + `hf_trade_cost` (append to hf_cost.rs):
   ```rust
   /// The additive HF cost terms, carried alongside the existing [`CostModel`] rather than
   /// extending it (see module doc). `tip_bps` is bps of trade notional, not lamports (deviation
   /// from m-hf-track §2.4's literal wording — logged on the M-HF-C4 card).
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct HfCostModel {
       pub base: CostModel,
       pub depth_curve: DepthCurve,
       pub congestion_priority_table: CongestionPriorityTable,
       pub tip_bps: u32,
   }

   /// The priced breakdown of one trade under [`HfCostModel`]. All quote-terms fields are USDC;
   /// gas is converted to quote at `price` so `total_quote` and `base_fee_floor_quote` are
   /// comparable in one unit.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub struct HfTradeCost {
       pub venue_fee_quote: Decimal,
       pub depth_slippage_quote: Decimal,
       pub tip_quote: Decimal,
       pub gas_lamports: i64,
       pub gas_quote: Decimal,
       pub total_quote: Decimal,
       /// The Solana base fee alone, converted to quote at `price` — the fabricated-edge floor
       /// a real HF cost model must exceed (m-hf-track §5 row C4's gate).
       pub base_fee_floor_quote: Decimal,
   }

   /// Price one trade's HF cost terms. Pure: no RNG, no clock, no side branching (m-hf-track §3;
   /// depth-walk and tip apply symmetrically at the notional level, matching how `dex_fee_bps`
   /// and gas already don't branch on side in `CostModel`).
   #[must_use]
   pub fn hf_trade_cost(
       hf: &HfCostModel,
       notional_quote: Decimal,
       price: Decimal,
       regime: CongestionRegime,
   ) -> HfTradeCost {
       let venue_fee_quote = apply_bps(notional_quote, hf.base.dex_fee_bps);
       let depth_slippage_quote =
           apply_bps(notional_quote, hf.depth_curve.impact_bps_for(notional_quote));
       let tip_quote = apply_bps(notional_quote, hf.tip_bps);
       let gas_lamports = hf.base.base_fee_lamports
           + hf.congestion_priority_table.priority_lamports_for(regime);
       let gas_quote = lamports_to_sol(gas_lamports) * price;
       let base_fee_floor_quote = lamports_to_sol(hf.base.base_fee_lamports) * price;
       HfTradeCost {
           venue_fee_quote,
           depth_slippage_quote,
           tip_quote,
           gas_lamports,
           gas_quote,
           total_quote: venue_fee_quote + depth_slippage_quote + tip_quote + gas_quote,
           base_fee_floor_quote,
       }
   }

   /// Scale `base`'s HF-only fields by `num/den`, in integer/exact-Decimal space — the same
   /// pattern `sweep::sensitivity::scale_cost_model` uses for `CostModel` (C6 will fold this
   /// into the ladder; not wired here). `CostModel`'s own fields are scaled by the existing
   /// function, untouched.
   #[must_use]
   pub fn scale_hf_cost_model(hf: &HfCostModel, num: u32, den: u32) -> HfCostModel {
       let den_nonzero = den.max(1);
       let scale_bps = |bps: u32| ((u64::from(bps) * u64::from(num)) / u64::from(den_nonzero)) as u32;
       let scale_lamports =
           |l: i64| ((i128::from(l) * i128::from(num)) / i128::from(den_nonzero)) as i64;
       HfCostModel {
           base: hf.base.clone(),
           depth_curve: DepthCurve {
               bands: hf
                   .depth_curve
                   .bands
                   .iter()
                   .map(|b| DepthBand {
                       notional_upto: b.notional_upto,
                       impact_bps: scale_bps(b.impact_bps),
                   })
                   .collect(),
           },
           congestion_priority_table: CongestionPriorityTable {
               calm_lamports: scale_lamports(hf.congestion_priority_table.calm_lamports),
               busy_lamports: scale_lamports(hf.congestion_priority_table.busy_lamports),
               hot_lamports: scale_lamports(hf.congestion_priority_table.hot_lamports),
           },
           tip_bps: scale_bps(hf.tip_bps),
       }
   }
   ```
   *(Note: `DepthCurve`'s private `bands` field means `scale_hf_cost_model` must live in this
   same module — it does, by construction of step 3 being appended to `hf_cost.rs`.)*
4. Unit tests in `hf_cost.rs` (`#[cfg(test)] mod tests`, `use rust_decimal_macros::dec;`). Test
   names pinned:
   - `depth_curve_rejects_empty` — `DepthCurve::new(vec![])` → `Err(EmptyDepthCurve)`.
   - `depth_curve_rejects_unsorted` — two bands with equal or descending `notional_upto` →
     `Err(UnsortedDepthCurve)`.
   - `depth_curve_walks_bands_by_notional` — bands `[{100, 5}, {1_000, 20}, {10_000, 50}]`:
     `impact_bps_for(dec!(50)) == 5`; `impact_bps_for(dec!(100)) == 5` (boundary is inclusive);
     `impact_bps_for(dec!(500)) == 20`; `impact_bps_for(dec!(50_000)) == 50` (off the end →
     deepest band, never less).
   - `congestion_priority_table_looks_up_by_regime` — `{calm: 1_000, busy: 10_000, hot:
     100_000}`: `priority_lamports_for(Calm/Busy/Hot)` returns each field exactly.
   - `hf_trade_cost_matches_hand_computed_reference_trade` — fixed inputs: `base = CostModel {
     dex_fee_bps: 10, slippage_bps: 0, base_fee_lamports: 5_000, priority_fee_lamports: 0 }`
     (the existing flat `slippage_bps` is deliberately 0 and unused by `hf_trade_cost` — it only
     reads `dex_fee_bps`/`base_fee_lamports` off `base`); `depth_curve` = the 3-band curve above;
     `congestion_priority_table` = the one above; `tip_bps = 2`; `notional_quote = dec!(500)`;
     `price = dec!(100)`; `regime = Busy`. Hand-computed: `venue_fee_quote = 500 * 10/10_000 =
     dec!(0.5)`; `impact_bps_for(500) = 20` → `depth_slippage_quote = 500 * 20/10_000 =
     dec!(1)`; `tip_quote = 500 * 2/10_000 = dec!(0.1)`; `gas_lamports = 5_000 + 10_000 =
     15_000`; `gas_quote = lamports_to_sol(15_000) * 100 = dec!(0.000015) * 100 = dec!(0.0015)`;
     `total_quote = 0.5 + 1 + 0.1 + 0.0015 = dec!(1.6015)`; `base_fee_floor_quote =
     lamports_to_sol(5_000) * 100 = dec!(0.000005) * 100 = dec!(0.0005)` *(corrected 2026-07-15:
     the original draft dropped a zero here and asserted 0.05 — caught by the executor's
     escalate-if hand-recompute, ruled by the planner; see worklog)*. Assert every field
     `assert_eq!` against these exact `Decimal`s.
   - `total_cost_exceeds_base_fee_floor` — using the same reference trade, assert
     `cost.total_quote > cost.base_fee_floor_quote` (`dec!(1.6015) > dec!(0.0005)`) — the gate's
     "base fee alone provably ≠ total cost" assertion, named for what it proves.
   - `zero_hf_terms_still_exceeds_floor_when_gas_priority_is_nonzero` — `dex_fee_bps: 0`,
     `tip_bps: 0`, a depth curve with `impact_bps: 0` everywhere, but `busy_lamports > 0`:
     `total_quote` still strictly exceeds `base_fee_floor_quote` on the priority-fee term alone
     — proves the assertion is discriminating (not vacuously true from venue fee/tip/depth
     alone).
   - `scale_hf_cost_model_scales_hf_fields_exactly` — `scale_hf_cost_model(&hf, 2, 1)` doubles
     every `impact_bps`, every priority-table lamport field, and `tip_bps` exactly; `base` is
     unchanged (untouched by this function — `CostModel`'s own scaling stays
     `scale_cost_model`'s job).
   - `repeated_calls_are_deterministic` — `hf_trade_cost` called twice with identical inputs →
     `assert_eq!`.
5. Wire `crates/portfolio/src/lib.rs`: add `pub mod hf_cost;` to the module list (alphabetical:
   between `equity` and `latency`), add
   `pub use hf_cost::{CongestionPriorityTable, CongestionRegime, DepthBand, DepthCurve, HfCostError, HfCostModel, HfTradeCost, hf_trade_cost, scale_hf_cost_model};`
   after the existing `pub use equity::…` line, and append one line to the module doc's layer
   list: `//! - [`hf_cost::hf_trade_cost`]: additive HF cost terms (m-hf-track §3/§4) — depth-walk
   slippage, regime priority, tip; carried on [`hf_cost::HfCostModel`], not on [`cost::CostModel`]
   (see the module doc for why). Not yet wired into execution.`
6. `cargo fmt --all`; run the full gate (below); flip this card to `DONE` + one worklog line.

**Gate.**
- `cargo test -p portfolio` — all new tests green; `git diff --name-only` shows only
  `hf_cost.rs` and `lib.rs` — no `cost.rs`, no `simulator.rs`, no `state.rs`, no `equity.rs`, and
  no file outside `portfolio` (confirming the ~30 existing `CostModel` call sites needed zero
  edits — the wrapper-struct decision holds).
- `hf_trade_cost_matches_hand_computed_reference_trade` and
  `total_cost_exceeds_base_fee_floor` both green — the card's two named gate assertions.
- Full workspace: `cargo fmt --all --check` clean; `cargo clippy --all-targets --all-features
  -- -D warnings` clean; `cargo test --workspace --all-features` → **0 failed, 1 ignored,
  > 363 passed** (record the exact new count in the worklog).
- `cargo run -p cli -- demo | shasum` → `ae064f79242f823ffd8f55bf9104e3e1b45d425a` unchanged;
  `cargo run -p cli -- sweep | shasum` → `7ad3df7de2e2c1139be427e9c953b57d4e289cb3` unchanged
  (this card wires nothing into execution or the sweep/CLI paths, so both are structurally
  unaffected — the gate re-confirms it, not just assumes it); `cargo run -p cli -- sweep-verify`
  → OK.
- `git status` shows exactly the card's 2 files (+ this queue/worklog flip) — no `Cargo.toml`,
  no `Cargo.lock`, nothing else.

**Guardrails (restated).** `Decimal` for all money — `u32`/`i64`/`u64` only for bps/lamports/
indices, never money; **no new deps, no Cargo.toml/Cargo.lock edits**; no RNG and no clock
anywhere (`rand`, `Instant`, `SystemTime` must not appear); `CostModel`/`cost.rs` and
`simulator.rs` are **untouched** — this card must not edit them, not even a visibility change,
not even to add the new fields (that is the whole point of the wrapper-struct decision above);
never touch `schemas/`, existing fixtures, `config/strategies/m5-frozen.toml`, the master-plan
pair, or the holdout machinery; strategies remain intent-only; NOTHING here creates execution
capability (no keys/signing/submit/RPC/network — M8/M9 need separate explicit human approval).
Never `git commit`/`git push` — the operator commits.

**Escalate-if (STOP, record in plans/worklog.md, report — never improvise):** any quoted
signature/line above doesn't match the source; any impulse to add fields directly to
`CostModel` or to touch `simulator.rs` (even one character) to make a literal compile — that
means the wrapper-struct decision needs revisiting, which is a planning call, not an executor
one; `hf_trade_cost_matches_hand_computed_reference_trade` fails (recompute by hand before
concluding the card's numbers are wrong — do not adjust the assertion to match the code's
output); `total_cost_exceeds_base_fee_floor` fails; the demo/sweep shasum moves (it should be
structurally impossible — if it moves, something unintended got wired in); an unlisted file
seems needed; rust_decimal_macros is missing from portfolio's dev-deps; any temptation to start
wiring `hf_trade_cost` into `run_hf` (that's a later card — this one only defines and proves the
pricing function).

---

### M-HF-C5 — Adversarial execution terms: sandwich, pickoff, maker trade-through, base-rung expected adverse-selection — `DONE`

**Goal.** Give the research engine the adversarial (MEV / adverse-selection) execution terms
m-hf-track §3/§5 row C5 calls for — **sandwich** and **pickoff** losses priced as **costs to us
only, never a benefit**, at two rungs (a worst-case rung reproducing the full τ-loss on **every**
taker fill, and a **base rung** carrying the non-zero **expected** `p·τ` term), plus a
**trade-through-only maker fill** predicate where a mere touch fills nothing — all deterministic
by construction (pure Decimal arithmetic; no RNG, no clock). Serves m-hf-track §5 row C5's gate
("worst-case rung reproduces τ-loss every taker fill; touch-without-trade-through → no fill") and
§3's "adverse selection is priced at the BASE rung, not only worst-case." Simulation only —
nothing here creates execution capability, and nothing here wires these terms into `run`/`run_hf`
(that is C6/C8, once cell/regime derivation exists).

**Planner decisions, logged here (read before objecting or trying to "fix" them):**
- **Decision 1 — one new `adversarial.rs`, not `maker_fill.rs`.** All four terms (sandwich,
  pickoff, base-rung expected adverse-selection, and the maker trade-through predicate) live in a
  single new `crates/portfolio/src/adversarial.rs`, mirroring C4's single-cohesive-module shape
  (`hf_cost.rs`). **Drift note, on the record:** m-hf-track §3 names `maker_fill.rs` for the maker
  predicate. C5 places that predicate in `adversarial.rs` instead — it is one small pure function
  (~15 lines) sharing the module's single theme ("adversarial execution terms, priced/modeled as
  costs to us only"); a dedicated file for it would be premature fragmentation and extra wiring for
  a fresh executor. This is a reversible internal file-layout fork (AGENTS.md: agent decides and
  logs these); the spec's intent — a trade-through-only maker fill that refuses touch-only fills —
  is preserved exactly. Do **not** create `maker_fill.rs`.
- **Decision 2 — C5 stays PURE, exactly like C4; it does NOT wire into `run`/`run_hf`.** C5
  defines and proves pure pricing/fill functions. Wiring the adverse-selection cost into the
  execution loop, and adding the `AdversarialWorst`/`HotCongestion`/`Latency2x` `ScenarioId` ladder
  rungs, are **C6's** job (m-hf-track §4); the per-fill decision of *which* fills get targeted, and
  the fail-closed `(regime, percentile) → p` table, are **C8's** sweep-wiring job. This is the exact
  parallel to C4, which supplied only the pure `hf_trade_cost(regime)` and the priority-fee lookup
  and deferred regime-derivation + the landing-percentile table to C8. Do **not** let ladder/spec
  wiring creep into this card.
- **Decision 3 — pure-parameter route, no new hash primitive.** m-hf-track §3 allows determinism
  "hash streams (the C3 splitmix64/landing_draw pattern) **or pure parameters**." C5 takes the
  pure-parameter route: the base-rung term is the **closed-form expectation** `p·τ` (exact rational
  `p`), which is precisely what C8's per-fill hash realization (reusing C3's `landing_draw`
  primitive — the "one primitive reused for landing/sandwich/pickoff/auction" of §3) will average to
  over many fills. C5 introduces **no new stochastic primitive**, so there is nothing new to prove
  for determinism beyond Decimal purity, and C3's `landing_draw` is neither re-duplicated nor made
  `pub(crate)` (latency.rs is untouched). The two views — C5's closed-form `p·τ` and C8's hashed
  per-fill realization — are consistent by construction.
- **Decision 4 — no scale helper.** Unlike C4's `scale_hf_cost_model`, C5 provides **no** scaling
  function. The adversarial ladder rungs are a **mode switch** (base expected ↔ worst-case, i.e. `p`
  forced to 1), not a numeric `×n` scale like `Doubled`; C6 selects a rung by calling
  `adverse_selection_cost_worst` vs `adverse_selection_cost_expected`. Do **not** add a scaler.

**Files.** Exactly these two; `simulator.rs`, `cost.rs`, `latency.rs`, `hf_cost.rs`, and every
existing call site are untouched:
1. `crates/portfolio/src/adversarial.rs` — NEW (`AdversarialModel`, `AdversarialError`, the two
   adverse-selection cost functions, `MakerOrder`/`MakerFill`/`maker_trade_through_fill`, unit
   tests).
2. `crates/portfolio/src/lib.rs` — wiring only (one `pub mod` + one `pub use` block + one doc line).

**Current state (verbatim, copied from source 2026-07-15 at `a7b3235` — if what you find differs,
escalate, don't adapt).**
- `crates/portfolio/src/cost.rs:14-20` — `Side` (the maker predicate matches on it; **untouched**):
  ```rust
  /// Which direction a swap goes. Base = SOL, quote = USDC.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum Side {
      /// USDC → SOL.
      Buy,
      /// SOL → USDC.
      Sell,
  }
  ```
- `crates/research-core/src/money.rs:37-39` — `pub fn apply_bps(value: Decimal, bps: u32) ->
  Decimal` returns `value * Decimal::from(bps) / Decimal::from(10_000_u32)` — **exact**, proven by
  the already-green `bps_application_is_exact` test (money.rs:69: `apply_bps(1000, 20) == dec!(2)`,
  `apply_bps(100, 5) == dec!(0.05)`). All money is exact `Decimal`; no floating point.
- `crates/portfolio/src/lib.rs:18-36` — current wiring (post-C4), the block this card extends:
  ```rust
  pub mod cost;
  pub mod equity;
  pub mod hf_cost;
  pub mod latency;
  pub mod simulator;
  pub mod state;

  pub use cost::{BuyFill, CostModel, SellFill, Side};
  pub use equity::{EquityPoint, RoundTrip};
  pub use hf_cost::{
      hf_trade_cost, scale_hf_cost_model, CongestionPriorityTable, CongestionRegime, DepthBand,
      DepthCurve, HfCostError, HfCostModel, HfTradeCost,
  };
  pub use latency::{
      build_landing_table, fixed_latency, run_hf, HfError, HfRunOutput, LandingOutcome,
      LatencyPipeline,
  };
  pub use simulator::{run, RunOutput};
  pub use state::{PortfolioState, SimError, TradeOutcome};
  ```
- `crates/portfolio/Cargo.toml` — deps: research-core, rust_decimal; dev-deps: rust_decimal_macros.
  **No Cargo.toml/Cargo.lock change is permitted by this card.**

**Pinned semantics (planner decisions — do not re-decide):**
- **τ (tau) — the worst-case per-fill adverse-selection loss.** For a taker fill of `notional_quote`
  (quote/USDC terms), τ = the sandwich loss **plus** the pickoff loss, each `apply_bps(notional,
  bps)`, **both applied on every fill**. Summing both on the same fill is the intended pessimism —
  a cost to us, never a benefit; it never *under*-states our cost. τ is independent of the
  probability `p`.
- **Worst-case rung (`adverse_selection_cost_worst`)** = τ on **every** taker fill (p ≡ 1,
  deterministic). This is the m-hf-track §4 `AdversarialWorst` bound (the ladder-rung wiring itself
  is C6's; this card only provides the pure function it will call).
- **Base rung (`adverse_selection_cost_expected`)** = the **expected** term `p·τ`, where
  `p = p_adverse_num / p_adverse_den` is an **exact rational** (fail-closed: `den ≥ 1`, `num ≤ den`,
  else an error — mirrors C3's `build_landing_table` probability validation). Computed as
  `τ * Decimal::from(num) / Decimal::from(den)`. This is deterministic Decimal arithmetic (no
  RNG/clock) and **exact for the reference numbers below** (they are chosen to terminate); for a
  general rational it is deterministic and platform-independent (rust_decimal is pure-integer,
  not f64) though the `/den` step may round to Decimal's 28-digit precision — a determinism-safe,
  not a determinism-risky, operation. The base term is non-zero whenever `p > 0` and a bps is
  non-zero, and is always in `[0, τ]` (a cost, never a benefit; `p = 1` ⇒ equals the worst rung,
  `p = 0` ⇒ zero). The single pinned base rate here stands in for the full `(regime, percentile) →
  p` table, which is C8's job (Decision 2).
- **Maker trade-through fill (`maker_trade_through_fill`)** — a resting **bid** (`Side::Buy`) at
  `limit_price` fills **only** if the bar's low prints **strictly** through it (`bar_low <
  limit_price`); a resting **ask** (`Side::Sell`) only if the bar's high prints strictly through it
  (`bar_high > limit_price`). A mere **touch** (`bar_low == bid` / `bar_high == ask`) fills
  **nothing** (`filled_base == 0`, `traded_through == false`) — the strict inequality is the whole
  point: we refuse an optimistic fill we could not guarantee from queue position. Filled size is
  **capped by the printed `volume`** (base/SOL terms): `filled_base = min(base_size, volume)`
  (floored at 0). Pure: no RNG, no clock.
- Errors are fail-closed: a bad probability rational is a pricing error, never a silent default.

**Steps.**
1. **Step 0 (before touching any file):** run the full-workspace gate. Expect **373 passed,
   0 failed, 1 ignored**; fmt/clippy clean; demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a`;
   sweep shasum `7ad3df7de2e2c1139be427e9c953b57d4e289cb3`. If anything is already red, STOP and
   report the baseline failure in the worklog.
2. Create `crates/portfolio/src/adversarial.rs` — module doc + imports + types + the two
   adverse-selection cost functions:
   ```rust
   //! Adversarial (MEV / adverse-selection) execution terms, priced as COSTS TO US ONLY — never a
   //! benefit (m-hf-track §3/§5 row C5): a taker fill can be **sandwiched** (an adversary front-/
   //! back-runs it) and/or **picked off** (transacted against a stale price), each a loss of some
   //! bps of the fill notional; τ (tau) is that worst-case per-fill loss. A resting **maker** order
   //! fills only when the market provably trades THROUGH its price — a mere touch fills nothing.
   //!
   //! Two rungs, both deterministic by construction (no RNG, no clock — pure Decimal arithmetic):
   //! - **AdversarialWorst** (m-hf-track §4's ladder rung): the full τ on EVERY taker fill (p = 1).
   //! - **Base rung**: the EXPECTED adverse-selection term `p·τ`, `p` an exact rational — a
   //!   non-zero everyday MEV cost a candidate cannot exclude from its base economics (§3).
   //!
   //! Carried in its own module (like C4's [`crate::hf_cost`]), NOT wired into `run`/`run_hf` here:
   //! this card defines and proves the pure pricing/fill functions. Per-fill hash realization of
   //! WHICH fills get targeted (the C3 `(cell_id, event_index)` splitmix64 primitive) and the
   //! fail-closed `(regime, percentile) → p` table are C8's sweep-wiring job — exactly as C4
   //! supplied only the priority-fee lookup and deferred the landing-percentile table to C8.

   use crate::cost::Side;
   use research_core::money::apply_bps;
   use research_core::Decimal;
   use std::fmt;

   /// The adversarial execution terms for taker fills. Every loss is a COST TO US (non-negative).
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub struct AdversarialModel {
       /// Worst-case sandwich loss, bps of taker-fill notional (τ's sandwich component).
       pub sandwich_bps: u32,
       /// Worst-case pickoff / adverse-selection loss, bps of notional (τ's pickoff component).
       pub pickoff_bps: u32,
       /// Base-rung adverse-event probability as an EXACT rational `num/den` (`den ≥ 1`,
       /// `num ≤ den`) — the fail-closed table value (m-hf-track §3). The full
       /// `(regime, percentile) → p` table is C8's job. An invalid rational is a pricing error,
       /// never a silent default.
       pub p_adverse_num: u64,
       pub p_adverse_den: u64,
   }

   /// Errors pricing adversarial terms. Fail-closed: never a silent default.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum AdversarialError {
       /// The adverse-event probability is not a valid rational in [0,1] (`den ≥ 1`, `num ≤ den`).
       BadProbability { num: u64, den: u64 },
   }

   impl fmt::Display for AdversarialError {
       fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
           match self {
               Self::BadProbability { num, den } => write!(
                   f,
                   "adverse-selection probability {num}/{den} is not a valid rational in [0,1]"
               ),
           }
       }
   }

   impl std::error::Error for AdversarialError {}

   /// The full worst-case adverse-selection loss τ for a taker fill of `notional_quote` (USDC):
   /// sandwich + pickoff, BOTH on EVERY fill (m-hf-track §4's `AdversarialWorst` rung reproduces
   /// the τ-loss on every taker fill). Pessimistic by construction — a cost to us, never a benefit.
   /// Pure: no RNG, no clock, no side branching.
   #[must_use]
   pub fn adverse_selection_cost_worst(model: &AdversarialModel, notional_quote: Decimal) -> Decimal {
       apply_bps(notional_quote, model.sandwich_bps) + apply_bps(notional_quote, model.pickoff_bps)
   }

   /// The base-rung EXPECTED adverse-selection term `p·τ`, exact Decimal (m-hf-track §3 — the
   /// non-zero everyday MEV cost). `p = p_adverse_num / p_adverse_den`. Fail-closed on a bad
   /// rational. Always in `[0, τ]` (a cost to us, never a benefit; `p = 1` ⇒ the worst rung,
   /// `p = 0` ⇒ zero).
   pub fn adverse_selection_cost_expected(
       model: &AdversarialModel,
       notional_quote: Decimal,
   ) -> Result<Decimal, AdversarialError> {
       if model.p_adverse_den == 0 || model.p_adverse_num > model.p_adverse_den {
           return Err(AdversarialError::BadProbability {
               num: model.p_adverse_num,
               den: model.p_adverse_den,
           });
       }
       let tau = adverse_selection_cost_worst(model, notional_quote);
       Ok(tau * Decimal::from(model.p_adverse_num) / Decimal::from(model.p_adverse_den))
   }
   ```
3. The maker trade-through fill (append to adversarial.rs):
   ```rust
   /// One resting maker (limit) order awaiting a trade-through fill. `Side::Buy` is a resting bid,
   /// `Side::Sell` a resting ask.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub struct MakerOrder {
       pub side: Side,
       pub limit_price: Decimal,
       pub base_size: Decimal,
   }

   /// The outcome of a maker order over one bar. `filled_base == 0` means NO fill — either the
   /// market never reached the limit, or it only touched it without trading through.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub struct MakerFill {
       pub filled_base: Decimal,
       pub traded_through: bool,
   }

   /// Trade-through-only maker fill (m-hf-track §3/§5 row C5): a resting **bid** fills only if the
   /// bar's low prints STRICTLY through it (`bar_low < limit_price`); a resting **ask** only if the
   /// bar's high prints strictly through it (`bar_high > limit_price`). A mere touch
   /// (`bar_low == bid` / `bar_high == ask`) fills NOTHING — we refuse an optimistic fill we could
   /// not guarantee from queue position. Filled size is capped by the printed `volume` (base terms).
   /// Pure: no RNG, no clock.
   #[must_use]
   pub fn maker_trade_through_fill(
       order: &MakerOrder,
       bar_low: Decimal,
       bar_high: Decimal,
       volume: Decimal,
   ) -> MakerFill {
       let traded_through = match order.side {
           Side::Buy => bar_low < order.limit_price,
           Side::Sell => bar_high > order.limit_price,
       };
       let filled_base = if traded_through {
           order.base_size.min(volume).max(Decimal::ZERO)
       } else {
           Decimal::ZERO
       };
       MakerFill {
           filled_base,
           traded_through,
       }
   }
   ```
4. Unit tests in `adversarial.rs` (`#[cfg(test)] mod tests`, `use rust_decimal_macros::dec;`). Test
   names pinned. Every reference number below was verified empirically against the real `apply_bps`
   + rust_decimal during card-writing — if a test fails, recompute by hand and STOP (see escalate-if):
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use rust_decimal_macros::dec;

       fn reference_model() -> AdversarialModel {
           AdversarialModel {
               sandwich_bps: 30,
               pickoff_bps: 10,
               p_adverse_num: 5,
               p_adverse_den: 100,
           }
       }

       #[test]
       fn adverse_selection_worst_reproduces_full_tau_on_every_fill() {
           let m = reference_model();
           // τ = sandwich(30bps) + pickoff(10bps), on EVERY fill, independent of p.
           // @1000: 500*... → 1000*30/10_000 = 3, 1000*10/10_000 = 1 → 4.
           assert_eq!(adverse_selection_cost_worst(&m, dec!(1000)), dec!(4));
           // @2000: 6 + 2 = 8 — scales with notional, still ignores p.
           assert_eq!(adverse_selection_cost_worst(&m, dec!(2000)), dec!(8));
       }

       #[test]
       fn adverse_selection_expected_is_p_times_tau() {
           let m = reference_model();
           // p = 5/100, τ@1000 = 4 → expected = 4 * 5/100 = 0.2 (exact Decimal).
           assert_eq!(
               adverse_selection_cost_expected(&m, dec!(1000)).unwrap(),
               dec!(0.2)
           );
       }

       #[test]
       fn adverse_selection_expected_boundary_probabilities() {
           let expected_at = |num, den| {
               adverse_selection_cost_expected(
                   &AdversarialModel {
                       sandwich_bps: 30,
                       pickoff_bps: 10,
                       p_adverse_num: num,
                       p_adverse_den: den,
                   },
                   dec!(1000),
               )
               .unwrap()
           };
           assert_eq!(expected_at(0, 1), dec!(0)); // p = 0 → the base term vanishes
           assert_eq!(expected_at(1, 2), dec!(2)); // p = 1/2 → half of τ (=4)
           assert_eq!(expected_at(1, 1), dec!(4)); // p = 1 → equals the worst rung (τ)
       }

       #[test]
       fn adverse_selection_expected_rejects_bad_probability() {
           // den = 0 and num > den are both fail-closed errors (mirrors C3's build_landing_table).
           let bad_den = AdversarialModel {
               sandwich_bps: 30,
               pickoff_bps: 10,
               p_adverse_num: 1,
               p_adverse_den: 0,
           };
           assert!(matches!(
               adverse_selection_cost_expected(&bad_den, dec!(1000)),
               Err(AdversarialError::BadProbability { .. })
           ));
           let num_gt_den = AdversarialModel {
               sandwich_bps: 30,
               pickoff_bps: 10,
               p_adverse_num: 3,
               p_adverse_den: 2,
           };
           assert!(matches!(
               adverse_selection_cost_expected(&num_gt_den, dec!(1000)),
               Err(AdversarialError::BadProbability { .. })
           ));
       }

       #[test]
       fn adversarial_terms_are_costs_never_benefits() {
           let m = reference_model();
           let worst = adverse_selection_cost_worst(&m, dec!(1000));
           let expected = adverse_selection_cost_expected(&m, dec!(1000)).unwrap();
           assert!(worst >= dec!(0));
           assert!(expected >= dec!(0));
           assert!(
               expected <= worst,
               "expected {expected} must never exceed worst {worst}"
           );
       }

       #[test]
       fn maker_bid_fills_only_on_trade_through() {
           let bid = MakerOrder {
               side: Side::Buy,
               limit_price: dec!(100),
               base_size: dec!(5),
           };
           // Trade-through: low 99 < bid 100 → filled, capped at volume (min(5, 10) = 5).
           assert_eq!(
               maker_trade_through_fill(&bid, dec!(99), dec!(101), dec!(10)),
               MakerFill {
                   filled_base: dec!(5),
                   traded_through: true
               }
           );
           // Touch (low == bid) → NO fill (the discriminating assertion).
           assert_eq!(
               maker_trade_through_fill(&bid, dec!(100), dec!(101), dec!(10)),
               MakerFill {
                   filled_base: dec!(0),
                   traded_through: false
               }
           );
           // Never reached (low 101 > bid) → NO fill.
           assert_eq!(
               maker_trade_through_fill(&bid, dec!(101), dec!(103), dec!(10)),
               MakerFill {
                   filled_base: dec!(0),
                   traded_through: false
               }
           );
       }

       #[test]
       fn maker_ask_fills_only_on_trade_through() {
           let ask = MakerOrder {
               side: Side::Sell,
               limit_price: dec!(100),
               base_size: dec!(5),
           };
           // Trade-through: high 101 > ask 100 → filled (min(5, 10) = 5).
           assert_eq!(
               maker_trade_through_fill(&ask, dec!(99), dec!(101), dec!(10)),
               MakerFill {
                   filled_base: dec!(5),
                   traded_through: true
               }
           );
           // Touch (high == ask) → NO fill.
           assert_eq!(
               maker_trade_through_fill(&ask, dec!(99), dec!(100), dec!(10)),
               MakerFill {
                   filled_base: dec!(0),
                   traded_through: false
               }
           );
       }

       #[test]
       fn maker_fill_is_capped_by_printed_volume() {
           let bid = MakerOrder {
               side: Side::Buy,
               limit_price: dec!(100),
               base_size: dec!(50),
           };
           // Wants 50 SOL, only 8 printed through → filled 8 (the size cap).
           assert_eq!(
               maker_trade_through_fill(&bid, dec!(99), dec!(101), dec!(8)),
               MakerFill {
                   filled_base: dec!(8),
                   traded_through: true
               }
           );
       }

       #[test]
       fn repeated_calls_are_deterministic() {
           let m = reference_model();
           assert_eq!(
               adverse_selection_cost_worst(&m, dec!(1000)),
               adverse_selection_cost_worst(&m, dec!(1000))
           );
           assert_eq!(
               adverse_selection_cost_expected(&m, dec!(1000)),
               adverse_selection_cost_expected(&m, dec!(1000))
           );
           let bid = MakerOrder {
               side: Side::Buy,
               limit_price: dec!(100),
               base_size: dec!(5),
           };
           assert_eq!(
               maker_trade_through_fill(&bid, dec!(99), dec!(101), dec!(10)),
               maker_trade_through_fill(&bid, dec!(99), dec!(101), dec!(10))
           );
       }
   }
   ```
5. Wire `crates/portfolio/src/lib.rs`:
   - add `pub mod adversarial;` to the module list **alphabetically first** (before `pub mod
     cost;`);
   - add this re-export block **immediately before** the `pub use cost::…` line:
     ```rust
     pub use adversarial::{
         adverse_selection_cost_expected, adverse_selection_cost_worst, maker_trade_through_fill,
         AdversarialError, AdversarialModel, MakerFill, MakerOrder,
     };
     ```
   - append one bullet to the module doc's bottom-up layer list, **after** the existing
     `[`hf_cost::hf_trade_cost`]` bullet (lines 11-13) and before the `//!` blank line preceding the
     "Nothing here is a strategy" paragraph:
     ```rust
     //! - [`adversarial::adverse_selection_cost_worst`]: adversarial execution terms (m-hf-track
     //!   §3/§5 row C5) — sandwich/pickoff τ priced as costs to us (a worst rung + a base-rung
     //!   expected `p·τ`), and a trade-through-only maker fill. Pure; not yet wired into execution.
     ```
6. `cargo fmt --all`; run the full gate (below); flip this card to `DONE` + one worklog line.

**Gate.**
- `cargo test -p portfolio adversarial` — the 9 new tests green; `git diff --name-only` shows only
  `adversarial.rs` and `lib.rs` — no `cost.rs`, no `simulator.rs`, no `latency.rs`, no `hf_cost.rs`,
  no `state.rs`, and no file outside `portfolio`.
- `adverse_selection_worst_reproduces_full_tau_on_every_fill` and
  `maker_bid_fills_only_on_trade_through` both green — the card's two named gate properties
  (worst rung = full τ on every fill; touch-without-trade-through → no fill).
- Full workspace: `cargo fmt --all --check` clean; `cargo clippy --all-targets --all-features
  -- -D warnings` clean; `cargo test --workspace --all-features` → **0 failed, 1 ignored,
  382 passed** (373 + the 9 unit tests above; record the exact count in the worklog — if it is not
  382, a test failed to register, investigate before flipping).
- `cargo run -p cli -- demo | shasum` → `ae064f79242f823ffd8f55bf9104e3e1b45d425a` unchanged;
  `cargo run -p cli -- sweep | shasum` → `7ad3df7de2e2c1139be427e9c953b57d4e289cb3` unchanged (this
  card wires nothing into execution or the sweep/CLI paths, so both are structurally unaffected —
  the gate re-confirms it, not just assumes it); `cargo run -p cli -- sweep-verify` → OK.
- `git status` shows exactly the card's 2 files (+ this queue/worklog flip) — no `Cargo.toml`,
  no `Cargo.lock`, nothing else.

**Guardrails (restated).** `Decimal` for all money — `u32`/`u64` only for bps/probability-rationals,
never money; **no new deps, no Cargo.toml/Cargo.lock edits**; **no RNG and no clock** anywhere
(`rand`, `Instant`, `SystemTime`, `Math`-random must not appear — pure parameters only, per Decision
3); adversarial terms are **costs to us only, never a benefit** (worst ≥ expected ≥ 0 always);
**`simulator.rs`, `cost.rs`, `latency.rs`, `hf_cost.rs` are untouched** — this card must not edit
them, not even a visibility change, and must not re-duplicate or `pub(crate)`-expose C3's
`landing_draw` (Decision 3); never touch `schemas/`, existing fixtures,
`config/strategies/m5-frozen.toml`, the master-plan pair
(`plans/master-plan.md`/`solana-crypto-trader-plan.md`), or the holdout machinery; strategies remain
intent-only; NOTHING here creates execution capability (no keys/signing/submit/RPC/network — M8/M9
need separate explicit human approval). Never `git commit`/`git push` — the operator commits.

**Escalate-if (STOP, record in plans/worklog.md, report — never improvise):**
- any quoted signature/line/block above doesn't match the source at `a7b3235`;
- **any hand-computed reference number fails** — recompute by hand against the real `apply_bps`
  (`value*bps/10_000`, exact) and rust_decimal, twice, independently of the code; if the code
  disagrees with your hand recompute, the CODE or the CARD is wrong — **report; do NOT adjust the
  assertion to match the code's output** (this is the exact failure the C4 escalation caught);
- **any impulse to weaken a worst-case term** — the worst rung is τ on EVERY fill (p ignored), and a
  touch (`==`) is NEVER a fill (strict `<` / `>` only); do not loosen either;
- **any impulse or apparent need to touch `simulator.rs`** (even one character), or to wire these
  terms into `run`/`run_hf`, or to add a `ScenarioId` ladder rung, or to add a scale helper — those
  are C6/C8, not this card (Decisions 2 & 4);
- any impulse to create `maker_fill.rs` or to introduce a new hash/RNG primitive (Decisions 1 & 3);
- the demo/sweep shasum moves (structurally impossible here — if it moves, something unintended got
  wired in);
- an unlisted file seems needed; rust_decimal_macros is missing from portfolio's dev-deps; any
  pre-existing test fails for a reason unrelated to this card.

---

### M-HF-C5.1 — Fail-closed hardening of C4 cost-shaping inputs (depth-curve impact monotonicity + non-negative priority lamports) — `DONE` *(2026-07-15; both gaps closed at the type boundary in `hf_cost.rs` only — `DepthCurve::new` now rejects a strictly-decreasing `impact_bps`, `CongestionPriorityTable` fields are private behind a `new()` that rejects negative lamports; 3 new tests; full workspace 385 passed/0 failed/1 ignored; demo/sweep shasums unchanged; `lib.rs`/`cost.rs` untouched)*

**Why this card exists.** The mandatory C3–C5 adversarial checkpoint (m-hf-track §5, run 2026-07-15)
confirmed **two** fail-closed gaps in C4's `hf_cost.rs` — both **cost-understating**, both mirroring
the `DepthCurve::new` validation C4 *already* ships for a sibling field, neither reachable in any
wired path today (nothing constructs these types from external/untrusted input until C6 wires them
into the ladder and C8 derives them from an operator TOML). Land this **before C6**, so the invariant
is true at the moment of wiring. This is a small, contained, fail-closed hardening — **not** new
modeling and **not** any execution capability.

The two confirmed gaps (each reproduced by direct code trace during the checkpoint):
1. **`DepthCurve::new` accepts a non-monotonic `impact_bps` curve.** It validates only that
   `notional_upto` is strictly ascending (hf_cost.rs:38-43); `impact_bps` is unchecked. So
   `DepthCurve::new(vec![DepthBand{notional_upto: 100, impact_bps: 50}, DepthBand{notional_upto:
   10_000, impact_bps: 5}])` is accepted, and then `impact_bps_for(5_000) == 5` while
   `impact_bps_for(100) == 50` — a **larger** trade priced **cheaper** in bps than a smaller one,
   inverting the depth-walk premise and understating slippage at scale.
2. **`CongestionPriorityTable` accepts negative lamports → a fabricated benefit.** Its lamport fields
   are unvalidated `pub i64`. A negative priority fee (e.g. `calm_lamports: -10_000` with
   `base_fee_lamports: 5_000`) makes `gas_lamports = -5_000` (hf_cost.rs:158-159), `gas_quote =
   lamports_to_sol(-5_000)*price < 0` (:160), and `total_quote < 0` — a **negative cost**, strictly
   **below** `base_fee_floor_quote`, directly breaking C4's own `total_cost_exceeds_base_fee_floor`
   gate and the module's "costs to us only, never a benefit" invariant.

**Scope boundary (read before "fixing more").** `base_fee_lamports` lives on `CostModel` in
`cost.rs`, which is **read-only** for HF cards (M2-owned; a negative base fee would corrupt the daily
sim too, so if it is a concern it is a separate M2 matter). **Do NOT edit `cost.rs` or add a runtime
`assert!`/clamp inside `hf_trade_cost`** — a mid-run panic or silent repair is not fail-closed. Fix
at the **type boundary** with `Result`-returning smart constructors, exactly as `DepthCurve::new`
already does (validity-by-construction; `hf_trade_cost` stays a pure infallible function whose inputs
are valid by construction). This card closes the `CongestionPriorityTable` (ours) vector; the
`base_fee_lamports` vector is explicitly out of scope and named here so it is not silently dropped.

**Planner decision (logged).** Mirror `DepthCurve` exactly: make `CongestionPriorityTable`'s three
lamport fields **private** and add `CongestionPriorityTable::new(calm, busy, hot) -> Result<Self,
HfCostError>` rejecting any negative — `DepthCurve` already keeps its `bands` field private behind
`new()`, so this is the established shape, not new API philosophy. Keeping the fields `pub` with only
a validating constructor would leave the guard bypassable (not fail-closed). No external caller
constructs or field-reads these types (confirmed 2026-07-15: every `CongestionPriorityTable {`/field
read is inside `hf_cost.rs`; `lib.rs` only re-exports the type *names*), so the change ripples
nowhere outside this file.

**Files.** Exactly ONE: `crates/portfolio/src/hf_cost.rs` (production edits + its in-file
`#[cfg(test)] mod tests`). **`lib.rs` is UNTOUCHED** — `HfCostError`, `DepthCurve`,
`CongestionPriorityTable` are already re-exported (hf_cost.rs's types at lib.rs:35-38); new error
variants and the inherent `new()` ride the existing re-exports. **No `Cargo.toml`/`Cargo.lock`
change.**

**Current state (verbatim, verified 2026-07-15 at HEAD `6507193`; if what you find differs,
escalate — don't adapt).**
- `DepthCurve::new` (hf_cost.rs:34-45) — the *only* validation is non-empty + strictly-ascending
  `notional_upto`:
  ```rust
  pub fn new(bands: Vec<DepthBand>) -> Result<Self, HfCostError> {
      if bands.is_empty() {
          return Err(HfCostError::EmptyDepthCurve);
      }
      if bands
          .windows(2)
          .any(|w| w[0].notional_upto >= w[1].notional_upto)
      {
          return Err(HfCostError::UnsortedDepthCurve);
      }
      Ok(Self { bands })
  }
  ```
- `HfCostError` (hf_cost.rs:91-113) currently has exactly two variants: `EmptyDepthCurve`,
  `UnsortedDepthCurve`, with a `Display` `match` over both.
- `CongestionPriorityTable` (hf_cost.rs:72-88) — three `pub i64` fields + `priority_lamports_for`:
  ```rust
  pub struct CongestionPriorityTable {
      pub calm_lamports: i64,
      pub busy_lamports: i64,
      pub hot_lamports: i64,
  }
  impl CongestionPriorityTable {
      #[must_use]
      pub fn priority_lamports_for(&self, regime: CongestionRegime) -> i64 { /* match */ }
  }
  ```
- In-file construction / field-read sites that MUST be updated when the fields go private
  (all inside hf_cost.rs; confirmed there are no others in the workspace):
  - `scale_hf_cost_model` at ~:196-200 constructs `CongestionPriorityTable { calm_lamports:
    scale_lamports(hf.congestion_priority_table.calm_lamports), … }` — both the `{ … }` literal and
    the three `hf.congestion_priority_table.<field>` reads change.
  - test helper `reference_priority_table` at ~:228-234 (`CongestionPriorityTable { calm_lamports:
    1_000, busy_lamports: 10_000, hot_lamports: 100_000 }`).
  - test `zero_hf_terms_still_exceeds_floor_when_gas_priority_is_nonzero` at ~:345-349
    (`CongestionPriorityTable { calm_lamports: 0, busy_lamports: 10_000, hot_lamports: 0 }`).
  - test `scale_hf_cost_model_scales_hf_fields_exactly` at ~:375-377 reads
    `scaled.congestion_priority_table.{calm,busy,hot}_lamports`.
- The existing `reference_depth_curve` (impacts `5, 20, 50`) is already monotonic non-decreasing —
  no existing depth-curve test violates the new rule; the single-band curve in the zero-terms test
  has no `windows(2)` pair. So no existing assertion should change value.

**Steps.**
1. **Step 0 (before touching any file):** run the full-workspace gate. Expect **382 passed, 0
   failed, 1 ignored**; fmt/clippy clean; demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a`;
   sweep shasum `7ad3df7de2e2c1139be427e9c953b57d4e289cb3`. If already red, STOP and report.
2. `HfCostError`: add two variants and their `Display` arms — `NonMonotonicImpact` ("depth curve
   `impact_bps` must be non-decreasing across bands") and `NegativePriorityLamports { regime: &'static
   str, lamports: i64 }` *(or a field-free variant if simpler — planner leaves the exact shape to the
   executor, but it must name which value was bad in `Display`)* ("priority-fee lamports must be
   >= 0"). Keep `#[derive(Debug, Clone, PartialEq, Eq)]`.
3. `DepthCurve::new`: after the ascending-`notional_upto` check, add a **non-decreasing** check on
   `impact_bps` (equal is allowed — a flat region is legal; reject only a strict decrease):
   `if bands.windows(2).any(|w| w[0].impact_bps > w[1].impact_bps) { return
   Err(HfCostError::NonMonotonicImpact); }`.
4. `CongestionPriorityTable`: make the three lamport fields **private**; add
   `pub fn new(calm_lamports: i64, busy_lamports: i64, hot_lamports: i64) -> Result<Self,
   HfCostError>` that returns the appropriate negative-lamports error if ANY of the three is `< 0`,
   else `Ok(Self { … })`. `priority_lamports_for` is unchanged (it reads the now-private fields
   internally).
5. Update the in-file sites from the current-state list to go through the accessor/constructor:
   field reads → `priority_lamports_for(CongestionRegime::{Calm,Busy,Hot})`; literal constructions →
   `CongestionPriorityTable::new(…)` with `.expect("valid priority table")` in tests, and in
   `scale_hf_cost_model` `.expect("scaling non-negative lamports by num/den stays non-negative")`
   (the inputs are non-negative by construction and `num/den` are `u32`, so the scaled result is
   non-negative — the `expect` cannot fire).
6. Add tests (in the same `#[cfg(test)] mod tests`), names pinned:
   - `depth_curve_rejects_non_monotonic_impact` — `DepthCurve::new(vec![{100,50},{1_000,20}])` →
     `Err(HfCostError::NonMonotonicImpact)`; and `DepthCurve::new(vec![{100,5},{1_000,5}])` (equal
     impacts) → `Ok` (a flat step is legal).
   - `priority_table_rejects_negative_lamports` — `CongestionPriorityTable::new(-1, 0, 0)` →
     `Err(…negative…)`; `CongestionPriorityTable::new(0, 10_000, 100_000)` → `Ok`; and
     `.priority_lamports_for(CongestionRegime::Busy) == 10_000` on the Ok value.
   - `hf_cost_cannot_go_negative_via_priority_table` — assert the *former* fabricated-benefit input
     is now un-constructible: `CongestionPriorityTable::new(-10_000, 0, 0)` is `Err`, so the negative
     `total_quote` path from the checkpoint finding cannot be built (this is the finding's regression
     pin — it lives at the constructor, not at `hf_trade_cost`).
7. `cargo fmt --all`; run the full gate (below); flip this card to `DONE` + one worklog line.

**Gate.**
- `cargo test -p portfolio hf_cost` green, including the 3 new tests; the existing
  `hf_trade_cost_matches_hand_computed_reference_trade` / `total_cost_exceeds_base_fee_floor` /
  `scale_hf_cost_model_scales_hf_fields_exactly` still pass with **identical asserted numbers** (this
  card changes construction/validation only, never a priced value).
- Full workspace: `cargo fmt --all --check` clean; `cargo clippy --all-targets --all-features -- -D
  warnings` clean; `cargo test --workspace --all-features` → **0 failed, 1 ignored, 385 passed**
  (382 + the 3 new tests; if the count is not 385, a test failed to register — investigate before
  flipping).
- `cargo run -p cli -- demo | shasum` → `ae064f79242f823ffd8f55bf9104e3e1b45d425a` unchanged;
  `cargo run -p cli -- sweep | shasum` → `7ad3df7de2e2c1139be427e9c953b57d4e289cb3` unchanged
  (this card wires nothing into execution — the gate re-confirms it); `sweep-verify` → OK.
- `git status` shows exactly ONE code file (`crates/portfolio/src/hf_cost.rs`) + this queue/worklog
  flip — no `lib.rs`, no `cost.rs`, no `Cargo.toml`, no other crate.

**Guardrails (restated).** `Decimal` for money; lamports stay `i64` but are now guaranteed `>= 0` by
construction; **no new deps / no Cargo.toml edits**; **no RNG, no clock**; fail-closed at the type
boundary (no runtime `assert!`/clamp, no `Result` on `hf_trade_cost`); **`cost.rs`, `simulator.rs`,
`latency.rs`, `adversarial.rs` untouched**; never touch `schemas/`, existing fixtures,
`config/strategies/m5-frozen.toml`, the master-plan pair, or the holdout machinery; NOTHING here
creates execution capability. Never `git commit`/`git push` — the operator commits explicit paths.

**Escalate-if (STOP, record in plans/worklog.md, report — never improvise):**
- any quoted signature/line/block above doesn't match source at `6507193`;
- making the lamport fields private forces an edit **outside** `hf_cost.rs` (it should not — if it
  does, an unlisted consumer exists; report it rather than editing another file);
- the demo/sweep shasum moves (structurally impossible here — if it moves, something got wired in);
- any existing priced-value assertion (`…reference_trade`, `…exceeds_base_fee_floor`,
  `…scales_hf_fields_exactly`) has to change to stay green — that means a real behavioral change crept
  in; STOP;
- any impulse to also edit `cost.rs`/`CostModel` for `base_fee_lamports` (out of scope — see the
  scope boundary), or to wire these types into `run`/`run_hf`/the ladder (C6/C8), or to touch
  `adversarial.rs`.

---

### M-HF-C6 — Turnover-criterion replacement: additive HF advancement criteria (cost-drag / per-trade-edge) — `DONE` *(2026-07-15; byte-identical-to-M4 regression held; sweep shasum moved by exactly the `schema_version` line; scope extended to an 8th file, `runner.rs`, not in the card's list — see worklog)*

**Why this card exists (and why it is NARROW).** m-hf-track §4/§5 sketched C6 as "ladder rungs +
turnover replacement + additive schema." A 2026-07-15 surface-map audit (5 agents, grep-verified)
proved that sketch drifted from the code that has since landed — **three of its stated gates cite
surfaces that do not exist:**
1. **"Consumer-match audit (results, cli)" is vacuous.** `RejectionKind` is referenced in
   **neither** `crates/results` nor `crates/cli`; the CLI emits the report as opaque JSON
   (`println!("{json}")`), and `evaluate_candidate` builds reasons via independent `if`-checks, not a
   `match`. There is **no exhaustive `match RejectionKind` anywhere** to audit. (This card *creates* a
   compiler-forcing function instead — see step 2's `RejectionKind::label()`.)
2. **The "HF-kind spec-lint" has no field to hang on.** `SweepSpec`/`SweepSpecToml` has **no**
   HF-vs-standard discriminator; that discriminator is born in **C8** (§5 row C8: "SweepSpec HF-kind").
   So "HF-kind spec sets `turnover_budget` → parse error" **moves to C8** and is NOT in this card.
3. **New HF `ScenarioId` rungs don't reach the report for free.** `FeeSensitivity`/schema are
   hard-locked to 3 named slots (`before_costs`/`base`/`doubled`) — even today's
   `DoubledSlippage`/`DoubledPriority` never surface. Surfacing HF scenarios is **C8's** windowed-sweep
   job. So `ScenarioId` variants + `hf_cost_scenarios` + `data_provenance` **all move to C8**.

**Operator scope decision (2026-07-15): NARROW C6 = the turnover-criterion replacement ONLY** (the
one piece that is real, fully additive, and gated by a genuine byte-identical-to-M4 regression today).
Everything ladder/scenario/provenance/spec-lint moves to C8, where it is actually exercised. This
reorders the §5 card boundaries by operator decision; **a follow-up planner edit must correct
m-hf-track §5's C6 row + the §4/§5 gate language** (record it in the worklog when this card lands).
Nothing here creates execution capability; the new criteria are implemented + unit-tested but not
activated until C8 provides HF thresholds + evidence — exactly the C4/C5 pattern (pure logic first,
sweep wiring later).

**Goal.** Make the advancement battery able to reject an HF candidate on the two economics that
matter at high volume — **cost drag share** (costs eat too much of the gross edge) and **per-trade
edge** (thousands of near-zero-edge trades) — without disturbing any M4/LF verdict. Concretely:
`turnover_budget` becomes optional (an HF candidate is not rejected for high turnover — high turnover
is the design, its cost already priced), and two new **opt-in** criteria are added alongside it. The
**primary gate is a byte-identical-to-M4 regression**: M4-era thresholds (`turnover_budget = Some`,
the two new = `None`) + M4-era evidence must produce verdicts whose serialization is unchanged except
the schema-version string.

**Pinned semantics (planner decisions — do not re-decide).**
- **`turnover_budget: Decimal` → `Option<Decimal>`** on `AdvancementThresholds`. `Some(x)` ⇒ the
  existing `TurnoverImplausible` check applies unchanged (`ev.turnover > x`); `None` ⇒ the turnover
  criterion is inactive (the HF case). **The TOML stays required** — `AdvancementToml.turnover_budget`
  is NOT touched (so `config/strategies/m5-frozen.toml` and `strategy-lab.example.toml` keep parsing
  with **no edit**); `spec.rs`'s mapping wraps it in `Some(...)`. C8 (which owns HF-kind specs) is
  what lets an HF spec omit it.
- **Two new `Option<Decimal>` threshold fields:** `cost_drag_share_ceiling`, `per_trade_edge_floor`.
  Both `None` in every M4/LF path (added to each `AdvancementThresholds { .. }` site as `None`).
- **Two new `Option<Decimal>` evidence fields** on `CandidateEvidence:` `cost_drag_share`,
  `per_trade_edge`. Doc-define them precisely (below) but **C6 does not compute them** — they are
  `None` in every M4/LF path; **C8 computes them from fee-sensitivity sums** (no new collection pass).
  - `cost_drag_share` = the fraction of the candidate's **gross (before-costs) return** consumed by
    costs (e.g. `return_drag_costs / before_costs_return`, guarded for non-positive gross). Higher =
    worse. Criterion `CostDragExcessive` fires when `cost_drag_share > cost_drag_share_ceiling`.
  - `per_trade_edge` = the candidate's **net edge per trade** (net return contribution ÷ `n_trades`,
    the everyday-MEV/adverse-selection sink from C5 included once C8 wires it). Lower = worse.
    Criterion `PerTradeEdgeInsufficient` fires when `per_trade_edge < per_trade_edge_floor`.
- **Gating is `if let (Some(threshold), Some(observed))`** for each new criterion — a criterion is
  checked only when BOTH its threshold and its evidence are present. For M4 (both `None`) neither
  fires ⇒ byte-identical. **C8's responsibility (documented, not enforced here):** when it sets an HF
  threshold it must also populate the matching evidence; a C6 doc-comment states this boundary.
- **`RejectionKind` gains `CostDragExcessive`, `PerTradeEdgeInsufficient`, appended LAST** (after
  `InsufficientData`) so the derived `Ord`/discriminants of the existing 7 variants are unchanged.
  **New checks are appended AFTER the turnover check** in `evaluate_candidate`'s fixed canonical order
  (turnover → cost-drag → per-trade-edge), so an M4 candidate (new thresholds `None`) yields an
  identical `failed_criteria` vector.
- **The vacuous "consumer audit" is replaced by a real forcing function:** add
  `RejectionKind::label(self) -> &'static str` as an **exhaustive `match`** (mirroring
  `ScenarioId::label()` at sensitivity.rs:42-54) returning the snake_case string, and a test asserting
  every variant's `label()` equals its serde form AND is present in the schema's `kind` enum. Now the
  compiler forces any future variant to be handled, and the Rust↔schema enum can't silently drift.
- **Additive schema, minor bump `1.1.0` → `1.2.0`:** append the two new `kind` strings to
  `RejectionReason.kind.enum`; move `turnover_budget` OUT of `Thresholds.required` (it stays a
  property, now optional) and add `cost_drag_share_ceiling`/`per_trade_edge_floor` as optional
  `decimalString` properties. An old 1.1.0 report still validates under 1.2.0 (superset enum;
  nothing newly required). The report DTO omits `None` fields via
  `#[serde(skip_serializing_if = "Option::is_none")]`, so an M4 report serializes identically **except
  `schema_version`**.
- **The `sweep` shasum WILL move — by exactly the version string.** This is the first HF card to
  change sweep output. The default `sweep` uses M4 thresholds, so its report changes ONLY
  `"schema_version": "1.1.0"` → `"1.2.0"` (turnover still `Some` ⇒ present; new fields `None` ⇒
  omitted). The gate below requires the executor to **diff before/after and confirm the ONLY change is
  that one line**, then record the new shasum. `demo` shasum is unchanged (demo emits no sweep report).

**Files (7 — justified: one coherent concern threaded through the stack; most test edits are
mechanical `Some(..)`/`None` wrapping; the byte-identical regression is the safety net).**
1. `crates/sweep/src/advance.rs` — the core: `AdvancementThresholds` (turnover→Option + 2 fields),
   `CandidateEvidence` (+2 Option fields), `RejectionKind` (+2 variants + `label()`), `evaluate_candidate`
   (Option-gate turnover + 2 appended checks), in-file test helpers + new tests.
2. `crates/sweep/src/spec.rs` — mapping ONLY (lines 125-132): wrap `turnover_budget` in `Some(..)`,
   add the two new thresholds as `None`. **`AdvancementToml` struct + the config TOMLs are untouched.**
3. `crates/sweep/src/report.rs` — `ThresholdsDto` (turnover→`Option<String>` + 2 `Option<String>`,
   `skip_serializing_if`), `from_thresholds` mapping, `SWEEP_SCHEMA_VERSION` `1.1.0`→`1.2.0`, test helper.
4. `crates/sweep/tests/schema_validation.rs` — update the `thresholds()` helper; ADD a test that an
   M4-shape report (turnover present, new fields absent) validates under 1.2.0, and an HF-shape report
   (turnover absent, both new thresholds present) validates.
5. `crates/sweep/tests/sweep_runner.rs` — update the `AdvancementThresholds { .. }` construction (Some + None).
6. `crates/sweep/tests/hf_reuse_proof.rs` — update the `AdvancementThresholds { .. }` construction (Some + None).
7. `schemas/sweep-report.schema.json` — the additive edits above. **This card is explicitly
   authorized to edit this schema** (the general "never touch schemas/" guardrail is lifted for THIS
   file, for the additive-minor change described, and ONLY it — `run-result.schema.json` and every
   other schema stay untouched).

**Current state (verbatim, verified 2026-07-15 at HEAD `6507193` + the staged C5.1 diff; if what you
find differs, escalate — don't adapt).**
- `crates/sweep/src/advance.rs:18-32` `AdvancementThresholds` — `turnover_budget: Decimal` at :23 (the
  field this card wraps in `Option`); 6 fields total.
- `crates/sweep/src/advance.rs:36-60` `CandidateEvidence` — 9 fields; `turnover: Decimal` at :43.
- `crates/sweep/src/advance.rs:73-90` `RejectionKind` — `#[derive(Debug, Clone, Copy, PartialEq, Eq,
  PartialOrd, Ord, Serialize)] #[serde(rename_all = "snake_case")]`, 7 variants ending
  `InsufficientData` at :89.
- `crates/sweep/src/advance.rs:182-188` — the turnover check to Option-gate:
  ```rust
  if ev.turnover > th.turnover_budget {
      failed.push(RejectionReason::new(
          RejectionKind::TurnoverImplausible,
          ev.turnover,
          th.turnover_budget,
      ));
  }
  ```
  (`RejectionReason::new(kind, observed: Decimal, threshold: Decimal)` is at advance.rs:102 — the two
  new checks call it the same way with their `Some`-unwrapped values.)
- `crates/sweep/src/advance.rs:208-232` test helpers `thresholds()` (`turnover_budget: dec!(5)` at
  :211) and `passing()` evidence; `crates/sweep/src/advance.rs:368` in
  `threshold_boundaries_are_inclusive_except_doubled_costs` reads `ev.turnover = th.turnover_budget;`
  — this becomes `th.turnover_budget.unwrap()` (or restructured) once the field is `Option`.
- `crates/sweep/src/sensitivity.rs:42-54` `ScenarioId::label()` — the EXHAUSTIVE-match pattern to
  mirror for `RejectionKind::label()`:
  ```rust
  impl ScenarioId {
      #[must_use]
      pub fn label(self) -> &'static str {
          match self {
              Self::BeforeCosts => "before_costs",
              // …one arm per variant, no `_` arm…
          }
      }
  }
  ```
- `crates/sweep/src/report.rs:19` `pub const SWEEP_SCHEMA_VERSION: &str = "1.1.0";` (→ `"1.2.0"`).
- `crates/sweep/src/report.rs:26-46` `ThresholdsDto` (`pub turnover_budget: String` at :28) +
  `from_thresholds` (`turnover_budget: th.turnover_budget.to_string()` at :39).
- `crates/sweep/src/spec.rs:125-132` — the mapping this card edits:
  ```rust
  let thresholds = AdvancementThresholds {
      drawdown_budget: t.advancement.drawdown_budget,
      turnover_budget: t.advancement.turnover_budget,        // → Some(t.advancement.turnover_budget)
      baseline_margin: t.advancement.baseline_margin,
      dispersion_budget: t.advancement.dispersion_budget,
      neighbor_tolerance: t.advancement.neighbor_tolerance,
      min_windows: t.advancement.min_windows,
      // + cost_drag_share_ceiling: None, per_trade_edge_floor: None
  };
  ```
- `schemas/sweep-report.schema.json:38-45` `Thresholds.required` (contains `"turnover_budget"` at :40
  — remove it from `required`, keep it in `properties`); `:127-148` `RejectionReason` with the 7-value
  `kind.enum` at :135-143 (append the two new strings).
- Other `AdvancementThresholds { .. }` sites needing the Some+None update:
  `crates/sweep/src/report.rs:193`, `crates/sweep/tests/schema_validation.rs:35`,
  `crates/sweep/tests/sweep_runner.rs:48`, `crates/sweep/tests/hf_reuse_proof.rs:41`. (Confirmed:
  these six are the complete set workspace-wide.)

**Steps.**
1. **Step 0 (before touching any file):** run the full-workspace gate. Expect **385 passed, 0 failed,
   1 ignored** (post-C5.1); fmt/clippy clean; demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a`;
   sweep shasum `7ad3df7de2e2c1139be427e9c953b57d4e289cb3`. If already red, STOP and report. (This card
   assumes C5.1 is committed/landed; if the C5.1 diff is absent, the baseline is 382 not 385 — STOP
   and report the mismatch rather than proceeding.)
2. `advance.rs`: widen `turnover_budget` to `Option<Decimal>`; add `cost_drag_share_ceiling:
   Option<Decimal>`, `per_trade_edge_floor: Option<Decimal>` to `AdvancementThresholds`; add
   `cost_drag_share: Option<Decimal>`, `per_trade_edge: Option<Decimal>` to `CandidateEvidence` (with
   the doc-definitions from Pinned semantics, incl. the "C8 computes these" note); append
   `CostDragExcessive`, `PerTradeEdgeInsufficient` to `RejectionKind`; add the exhaustive
   `RejectionKind::label()`. In `evaluate_candidate`: change the turnover check to
   `if let Some(tb) = th.turnover_budget { if ev.turnover > tb { … } }`, then append (after it, in this
   order) the two new `if let (Some(t), Some(v)) = (…) { if v > t /* or v < t */ { … } }` checks.
3. `spec.rs`: edit the 125-132 mapping only (Some-wrap turnover; two new `None`).
4. `report.rs`: `ThresholdsDto` fields (turnover→`Option<String>` + 2, all with
   `#[serde(skip_serializing_if = "Option::is_none")]`); `from_thresholds` maps `Option<Decimal>` →
   `Option<String>` (`.map(|d| d.to_string())`); bump the version const; fix the test helper.
5. Update the remaining `AdvancementThresholds { .. }` sites (report.rs:193, and the three test files).
6. `schemas/sweep-report.schema.json`: append the two `kind` strings; drop `turnover_budget` from
   `Thresholds.required`; add the two new optional threshold properties.
7. Tests (pinned intent): in `advance.rs` — `turnover_inactive_when_budget_is_none` (turnover
   enormous, `turnover_budget = None` ⇒ NOT rejected); `cost_drag_excessive_fires_and_is_gated`
   (threshold `Some` + evidence `Some` over ceiling ⇒ `CostDragExcessive`; either `None` ⇒ inactive);
   `per_trade_edge_insufficient_fires_and_is_gated` (symmetric, `<` floor); `rejection_kind_label_matches_serde`
   (every variant's `label()` == its `serde_json` string); and the **byte-identical regression**:
   an M4-shape verdict (`turnover_budget = Some`, both new `None`, evidence new = `None`) has the
   same `failed_criteria`/serialization as the pre-C6 semantics for the same six-criterion inputs
   (reuse the existing `all_non_data_criteria_fail_in_canonical_order` expectation — it must still
   hold verbatim). In `schema_validation.rs` — the M4-shape-validates + HF-shape-validates tests.
8. `cargo fmt --all`; run the full gate (below); flip this card to `DONE` + one worklog line; **and
   note in the worklog that the m-hf-track §5 C6-row/gate language still needs the planner correction**
   described above (that is a separate small plan edit, not part of this executor card).

**Gate.**
- `cargo test -p sweep` green; every PRE-EXISTING `advance.rs`/`report.rs`/`sweep_runner`/`schema_validation`
  assertion still passes with only mechanical `Some(..)`/`None` wrapping — **no asserted verdict,
  ordering, observed, or threshold value changed** (if one has to change to stay green, a real
  behavioral regression crept in — STOP).
- Full workspace: `cargo fmt --all --check` clean; `cargo clippy --all-targets --all-features -- -D
  warnings` clean; `cargo test --workspace --all-features` → **0 failed, 1 ignored, N passed** where
  `N = 385 + (new tests added)`; record the exact N.
- **Schema/serialization diff is version-only for M4:** `cargo run -p cli -- sweep` — capture the
  report before (git-stash or the committed baseline) and after; **the ONLY textual diff must be
  `"schema_version": "1.1.0"` → `"1.2.0"`.** If anything else differs, a non-additive change leaked in
  — STOP and report. Then record the NEW sweep shasum in the worklog (it replaces
  `7ad3df7d…`); `cargo run -p cli -- sweep-verify` → OK (byte-identical across threads at 1.2.0).
- `cargo run -p cli -- demo | shasum` → `ae064f79242f823ffd8f55bf9104e3e1b45d425a` **unchanged** (demo
  emits no sweep report — if it moves, something unrelated got wired in; STOP).
- `git status` shows exactly the 7 card files (+ the queue/worklog flip) — no `run-result.schema.json`,
  no other schema, no `crates/results`/`crates/cli`, no `Cargo.toml`, no config TOML, no
  `AdvancementToml`/config-TOML edit.

**Guardrails (restated).** `Decimal` for all money/threshold math — the new fields are `Decimal`
budgets, never `f64` (advance.rs is f64-free, source-audited — keep it so); no new deps / no
Cargo.toml edits; determinism unchanged (fixed canonical order; total-ordered report; no RNG/clock);
the two new criteria are **costs to the candidate, never a benefit** (they can only ADD rejections,
never remove one — an M4 candidate's verdict set is a subset-preserving no-op); **do NOT** add a
`ScenarioId` variant, `hf_cost_scenarios`, `data_provenance`, an HF-kind spec discriminator, or a
spec-lint (all C8); **do NOT** touch `AdvancementToml`, the config TOMLs,
`config/strategies/m5-frozen.toml`, the holdout machinery, `run-result.schema.json`, the master-plan
pair, or `crates/results`/`crates/cli`; strategies stay intent-only; NOTHING here creates execution
capability. Never `git commit`/`git push` — the operator commits explicit paths.

**Escalate-if (STOP, record in plans/worklog.md, report — never improvise):**
- any quoted signature/line/block above doesn't match source at `6507193`+C5.1;
- the Step-0 baseline isn't 385/0/1 (e.g. C5.1 not landed) — report, don't proceed on a wrong baseline;
- **any PRE-C6 test assertion has to change to stay green** — the change is supposed to be byte-identical
  for M4; a forced assertion edit means a real regression — STOP;
- **the sweep report diff is anything other than the single `schema_version` line** — a non-additive
  change leaked in;
- the demo shasum moves;
- an old (1.1.0) report fails to validate against the 1.2.0 schema (the bump must be a superset);
- you find yourself needing a `ScenarioId` variant, an HF-kind spec field, a `_` wildcard added to a
  match, or an edit outside the 7 listed files — those are C8 or a scope breach; STOP.

---

## Deferred / blocked (unchanged)

| ID | Status | Milestone | File scope | Gate | Notes |
|----|--------|-----------|------------|------|-------|
| — | CLOSED | M5 | research decision | — | **Gate declared 2026-07-12: reject-all, holdout unread.** §M-HF wave 1 above is the active queue. |
| — | DEFERRED | M6 | `crates/route-model`, `crates/risk`, `crates/solana-execution` (no-sign) | — | Jupiter shadow quote collector. Re-verify plan §22 sources first. |
| — | DEFERRED | M7 | `crates/wallet-state` | — | read-only mainnet shadow. No signing key. |
| — | BLOCKED | M8/M9 | devnet/canary signing+submit | — | **Requires separate explicit human approval. Do not start.** |
