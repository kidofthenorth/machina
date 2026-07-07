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
| M4·S1–S11 | DONE | M4 | `crates/sweep` (+ portfolio/results/research-core additive), `schemas/sweep-report.schema.json` | parallel==sequential byte-identical {1,2,3,7,8}; repeated identical; holdout sealed | Plan: [m4-sweep.md](m4-sweep.md). Deterministic sweep core (S1–S7), walk-forward (S8), holdout gate (S9), cost/fee sensitivity (S10), advancement report + schema (S11) — all adversarially reviewed. **Verified against source 2026-07-06 (audit in worklog): all claims confirmed; one S9 nuance → card M4-C1.** Workspace green at `df18267`: **272 tests, 0 failed**. |

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

### M4-C7 — CLI `machina sweep-verify` — `TODO`

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

### M4-C8 — Record DECISIONS D-0009 + refresh docs for the sweep CLI — `TODO`

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

### M4-C9 — M4 gate declaration (evidence checklist against master-plan.md) — `TODO`

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
2. Evidence checklist — confirm each maps to a real artifact (all should exist if C1–C8 are DONE):
   | Master-plan item | Evidence |
   |---|---|
   | Parameter sweeps, MVP families | `sweep::param` grids (both families) + `SweepSpec.grids` + CLI sweep over the template grids |
   | Parallel execution | `sweep::parallel` (`std::thread::scope`) + `--threads N` + determinism.rs {1,2,3,7,8} |
   | Canonical result export | `SweepReport::to_json` + `schemas/sweep-report.schema.json` validation tests (schema_validation.rs, sweep_runner.rs) |
   | Walk-forward windows | `sweep::window` + `DevValidation::walk_forward_windows` + walk_forward.rs |
   | Strategy-family comparison | one report covering both families' candidates, scored via the shared `eval_strategy` core against the same 4 cost-matched baselines |
   | Turnover & fee-sensitivity reporting | `RunOutput.traded_notional_quote` → `CellResult.turnover` (+ budget criterion); `sensitivity` ladder + doubled-costs criterion with recorded observed/threshold |
   | Rejection report | `advance` (7 criteria) + `report` verdicts, schema-validated |
   | Gate: parallel==sequential | determinism.rs + sweep_runner.rs test (a) + step 1e/1f above |
   | Gate: repeated identical | same three, plus step 1d |
   | Gate: holdout untouched | holdout_sealing.rs (counter+digest) + `compile_fail` doctests + CLI `assert_eq!(read_count, 0)` |
3. In `plans/m4-sweep.md`: set the S12 row Status to `✅ DONE (as cards M4-C1…C10)`; add one line to
   the §1 goal section: `**GATE DECLARED <today's date>** — evidence in worklog + task-queue M4 cards.`
4. In `plans/current-state.md`: Milestone section — move M4 to **DONE** (one line: engine S1–S11 +
   cards C1–C8, gate declared, N tests green); set "DOING" to `— (between milestones; M5 requires
   operator decisions — see handoff)`; update the "Gates run" date/counts with step-1 results.
5. In this file: flip this card to DONE; update the `M4·S1–S11` row Notes with "GATE DECLARED".
6. Append the worklog line with the step-1 results.

**Gate.** All step-1 commands pass; the three plan files agree M4 is complete; no code was changed.

**Guardrails.** Plan files only — no code/schema/config edits; do NOT reword anything in
master-plan.md; determinism evidence must come from freshly run commands (step 1), not from this
card's text; no new dependencies; holdout stays sealed.

**Escalate-if.** ANY step-1 command fails (M4 is NOT complete — stop, report, do not declare); any
checklist row lacks its artifact; cards C1–C8 are not all DONE in this file.

---

### M4-C10 — Point current-state/handoff at the M5 decision and STOP — `TODO`

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

## Deferred / blocked (unchanged)

| ID | Status | Milestone | File scope | Gate | Notes |
|----|--------|-----------|------------|------|-------|
| — | DEFERRED | M5 | research decision | — | Trial count, stability, advance/reject gate. **Blocked on operator: Q3 (real data) + Q5 (frozen thresholds) + explicit go.** No implementation queued (see M4-C10). |
| — | DEFERRED | M6 | `crates/route-model`, `crates/risk`, `crates/solana-execution` (no-sign) | — | Jupiter shadow quote collector. Re-verify plan §22 sources first. |
| — | DEFERRED | M7 | `crates/wallet-state` | — | read-only mainnet shadow. No signing key. |
| — | BLOCKED | M8/M9 | devnet/canary signing+submit | — | **Requires separate explicit human approval. Do not start.** |
