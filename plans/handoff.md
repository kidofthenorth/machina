# Handoff — for a new chat continuing machina

You are picking up an in-progress build. This is the single entry point. Read the five pointer files
below, confirm the gates are green, then continue in milestone order. **Do not commit or push — the
operator commits.**

---

## 30-second orientation

**machina** (codename `solana-crypto-trader`) is a paper-first, Solana-focused crypto **trading
research platform** in Rust, built toward one goal: **genuine autonomous passive income** —
low-touch, hands-off on-chain spot trading for a solo operator, once a strategy has earned that
trust. Getting there means first proving a strategy has a robust edge after costs, latency,
slippage, and operational failure; no strategy is guaranteed to clear that bar, and **live trading
starts only once its own milestone gate is explicitly approved**.

**The one rule that cannot be broken:** money-moving capability is gated by milestone **and** by
explicit human approval. No key loading, signing, or transaction submission exists — or may be added
— before the research phase (M0–M5) is complete and separately approved. Building the software never
authorizes trading. Full invariants: [docs/invariants.md](../docs/invariants.md).

## Read these first (source of truth)

1. [plans/current-state.md](current-state.md) — live per-milestone status, gates, blockers, next command.
2. **[plans/task-queue.md](task-queue.md) — the ACTIVE work queue: task cards M4-C1…M4-C10** (the former
   S12, expanded by the 2026-07-06 audit). Execute in order, one card per fresh session; each card is
   self-contained. Design rationale lives in [plans/m4-sweep.md](m4-sweep.md) (S1–S11 DONE).
3. [plans/master-plan.md](master-plan.md) — authoritative M0–M11 roadmap & architecture.
4. [plans/m4-sweep.md](m4-sweep.md) — the M4 design record (S1–S11 rationale the cards build on).
5. [docs/architecture-index.md](../docs/architecture-index.md) + [docs/invariants.md](../docs/invariants.md) — module map + hard invariants.

Audit + staged-diff record: [plans/review-packet.md](review-packet.md). Chronological log:
[plans/worklog.md](worklog.md). Open operator questions: [plans/questions.md](questions.md).

## What's accomplished (M0–M2 complete; M3 scaffolds; **M4 in progress — S1–S11 done**)

- **M0 Bootstrap.** 7-crate Cargo workspace; pinned toolchain; rustfmt; CI (`fmt + clippy -D warnings
  + test + no-execution-deps` scan); README/DECISIONS/AGENTS/CLAUDE; 4 secret-safe `*.example.toml`
  templates; 4 JSON Schemas (run-result/route-quote/execution-event/wallet-snapshot) + validation tests.
- **M1 Data foundation.** `research-core` (fixed-point money via `rust_decimal`, token/mint, OHLCV bar,
  UTC time) + `market-data` (allowlist loader; series validation incl. empty/OHLC/duplicate/unsorted/
  **gap-missing**; token-decimal + disallowed-token checks; checked-in fixtures).
- **M2 Deterministic spot simulator.** `portfolio` (deterministic `CostModel` fill model;
  `PortfolioState` accounting with oversell/unaffordable/gas guards; next-bar `simulator::run`; equity
  curve; round trips) + `metrics` (return/drawdown in `Decimal`; Sharpe/Sortino/vol in display-only f64).
- **M3 Strategy Lab (scaffolds).** `Strategy` trait (intent only); baselines `hold_usdc` /
  `buy_and_hold_sol` / `static_50_50` / `dca_sol`; `regime::classify` scaffold; `trend_alloc_v1`;
  `threshold_rebalance_v1`; `results` RunResult model; `cli` deterministic demo.
- **M4 — sweep core + walk-forward + holdout seal + cost sensitivity + advancement report (S1–S11 of 12 done).** Designed via a 3-architecture + adversarial
  workflow (see [m4-sweep.md](m4-sweep.md)). Shipped, each a small green diff:
  - **S1** additive `portfolio::RunOutput.traded_notional_quote` (the correct turnover base — the
    adversary proved round-trip turnover reads ~0 for zero-round-trip rebalancers).
  - **S2** `research-core::time::{parse_ymd, civil_date_to_unix}` (Hinnant inverse; pure-int; zero deps).
  - **S3** `results` turnover plumbed into `MetricsReport.turnover` (unchanged run-result schema).
  - **S4** new `crates/sweep` crate, wired into the workspace (no-exec-deps scan still clean).
  - **S5** `sweep::param` — `ParamPoint`/`ParamGrid` fixed-order Cartesian product + intent-only
    `build_strategy` bridge + scale-canonical `param_id`.
  - **S6** `sweep::{cell, turnover}` — pure `eval_cell` (wraps `run`+`Metrics`), `CellResult`
    (Decimal keys, non-finite f64→None), exact `turnover_ratio`.
  - **S7 — DETERMINISM GATE PROVEN.** `sweep::parallel` — `run_in_parallel` over `std::thread::scope`
    (contiguous index chunks, disjoint writes, read back in index order), `Parallelism`, `SweepCell`,
    `run_cells`. Gate test: a 48-cell sweep is byte-identical across thread counts {1,2,3,7,8},
    parallel==sequential, repeated-run identical, + anti-vacuous guard. **Zero new deps** (rayon
    rejected; record as D-0009 in S12).
  - **S8 — Walk-forward windows.** `sweep::window` — `Window { train, test: Range<usize> }`,
    `WalkForward { kind, train_len, test_len, step, embargo }`, `WindowKind::{Rolling (default),
    Anchored}`, `WindowError`. `new()` rejects bad params (zero lengths, `step<test_len` overlap);
    `windows(n_bars)` is pure + total + defensive (saturating math; struct-literal bypass → empty,
    never an infinite loop). Invariants structural: `train.end + embargo == test.start`, ordered +
    non-overlapping test ranges, in-bounds. **Zero new deps.** Adversarially verified (5-lens +
    adjudicator): impl correct; two test-coverage gaps (embargo magnitude, Anchored `step` coupling)
    found and closed.
  - **S9 — HOLDOUT GATE (invariant 11's enforcement point).** `sweep::{config, partition}`:
    `PartitionSpec::{ByIndex, ByDate}` (ByDate via the S2 `parse_ymd`); `PartitionedBars::from_spec`
    validates the series via canonical `market_data::validate_series` (sorted **and unique**) then
    **physically moves** the holdout into a SEPARATE `Vec<Bar>` (`split_off`, not an aliasing borrow);
    rejects overlap/empty/out-of-bounds. `seal_holdout(self) -> (DevValidation, Sealed)` — `DevValidation`
    has NO holdout accessor and walk-forward runs over dev/val length only, so it is structurally
    incapable of reaching the holdout. `evaluate_on_holdout(sealed: Sealed, …)` consumes the seal **by
    value** (call-once; M5-only — the M4 CLI never calls it); the single read passes a `Cell<u32>`
    counter gateway. Gate green: read-counter==0 + digest unchanged after the full sweep+selection
    pipeline; 3 `compile_fail` doctests (no selection→holdout path, no seal fabrication, call-once) —
    2 of 3 carry `no_run` canaries; the `Sealed` one is queued as card M4-C1 (2026-07-06 audit);
    overlap rejected. **Zero new deps.** Adversarially reviewed (6-lens +
    skeptics, 12 agents): **seal sound, no blockers**; 3 minor/nit fixes applied (delegate to
    `validate_series` → duplicate-ts rejection; real post-split structural assert; within-process-only
    digest doc).
  - **S10 — COST/FEE SENSITIVITY LADDER + cost-matched baselines.** `sweep::{sensitivity, baseline}`
    (+ `cell.rs` extracted a shared `pub(crate) eval_strategy` core, so candidates and baselines score
    through byte-identical logic). `cost_scenarios` = {before_costs=`zero()`, base, doubled,
    doubled_slippage, doubled_priority} via exact integer `scale_cost_model` (u32 bps / i64 lamports,
    widened, no f64); `ScenarioMetrics` / `FeeSensitivity` (Decimal `return_drag_*`, baseline-grounded
    `survives_doubled`, not a bare `>0`); the 4 required baselines re-run cost-matched, `label()` ==
    strategy `name()`, `best_baseline_return` floor. **Zero new deps.** Adversarially reviewed (3-lens +
    skeptics, 9 agents): **no code defects**; key finding — **cost monotonicity is conditional** (higher
    costs can suppress a marginal trade → fewer fees / higher return / negative drag), now documented +
    pinned by a regression test; 2 doc nits fixed; 2 findings refuted (overflow/negative-lamport both
    unreachable — `CostModel` has no `Deserialize`).
  - **S11 — ADVANCEMENT/REJECTION REPORT + new schema (D-0001).** `sweep::{advance, report}` +
    `schemas/sweep-report.schema.json`. `advance.rs`: Decimal-only `evaluate_candidate` — 7
    `RejectionKind` criteria, InsufficientData hard-stop, fixed canonical order, `RejectionReason`
    records observed/threshold as exact decimal strings. `report.rs`: `SweepReport` with a
    robustness-only `note` (invariant 11), `trial_count`, and a **total-order** verdict sort
    (byte-identical even with duplicate labels). New Draft-2020-12 schema (additive contract);
    **run-result schema UNCHANGED**; `tests/schema_validation.rs` mirrors `results` (real report
    validates; rejects numeric budget / unknown enums / extra props; source-level f64-audit).
    **Zero new deps.** Adversarially reviewed (3-lens + skeptics, 7 agents): **schema fidelity clean**;
    3 nit fixes (single-source edge-vanishes; total-order sort; coupling-code-enforced note); 1 refuted.
- **Quality.** A 6-lens adversarial verification workflow ran on M0–M3 → **PASS, no blockers**; a
  5-lens + adjudicator workflow verified S8 → impl correct, found & fixed two test gaps; a 6-lens +
  per-finding-skeptic workflow verified the S9 seal → **sound**, 3 minor/nit hardening fixes applied; a
  3-lens + skeptic workflow verified S10 → **no code defects**, surfaced conditional cost monotonicity
  (documented + regression-tested), 2 doc nits fixed; a 3-lens + skeptic workflow verified S11 → **schema
  fidelity clean**, 3 nit fixes applied.

## Verified state (re-confirm on a fresh checkout)

```bash
cargo fmt --all --check                                  # clean
cargo clippy --all-targets --all-features -- -D warnings # clean
cargo test --workspace --all-features                    # 272 passed, 0 failed (2026-07-06; 169 at S11 + 103 test-hardening)
cargo run -p cli -- demo                                 # deterministic, schema-valid RunResult (shasum-identical on repeat)
# no-execution-deps scan (CI parity) — must print OK:
pattern='solana-sdk|solana-client|solana-program|solana-rpc|jupiter|jito|ed25519-dalek|keypair|bip39|tiny-bip39|secp256k1'
grep -REn --include='Cargo.toml' "$pattern" . | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#' || echo OK
```

Repo state (2026-07-06): the operator has committed **everything through S11 plus 9 test-hardening
commits — HEAD = `df18267`, working tree clean**. **Never** `git add -A`; **never** stage `.claude/`.
Stage explicit paths only. (Run `git status` to confirm — the operator may have committed more since
this was written.)

## What's next — execute the M4 task cards (see [task-queue.md](task-queue.md))

S1–S11 are done. The former S12 was **expanded on 2026-07-06 into ten self-contained task cards
M4-C1…M4-C10** after an audit found it underspecified (no `SweepSpec`/`run_sweep` orchestrator or TOML
`Deserialize` existed in `crates/sweep`, and `strategy-lab.example.toml` lacked the
`[walk_forward]`/`[advancement]`/grid blocks the CLI design assumed). Execute the cards **in order,
one per fresh session**; each carries its own files, verbatim signatures, steps, gate, guardrails, and
escalate-ifs — a session should never need to open a file its card doesn't name.

- **M4-C1** — add the missing `no_run` canary to the `Sealed` no-forgery doctest (S9 nuance).
- **M4-C2** — `sweep::spec`: `SweepSpec::from_toml_str` + the template's `[walk_forward]`/`[advancement]`/grid blocks.
- **M4-C3…C5** — `sweep::runner`: canonical enumeration → evidence aggregation → `run_sweep` + report-level determinism test.
- **M4-C6…C7** — CLI `machina sweep [--threads N] [--out PATH]` and `machina sweep-verify` (no clap, D-0002; holdout never reachable).
- **M4-C8** — DECISIONS **D-0009** + docs/AGENTS refresh.
- **M4-C9** — **M4 gate declaration**: run the full battery, check the evidence table against master-plan.md:873-889, declare M4 complete.
- **M4-C10** — point current-state/handoff at the **M5 operator decision** and STOP.

Beyond M4: **M5** research-decision gate (needs the operator to FREEZE walk-forward sizing + rejection
thresholds first — questions.md Q5); **M6** Jupiter shadow (re-verify plan §22 sources; no signing).
**BLOCKED — M8/M9** devnet/canary signing + submission: **separate explicit human approval. Do not start.**
Real OHLCV ingestion is deferred pending an operator data-source decision (questions.md Q3); M1–M4 use
tiny synthetic checked-in fixtures only.

## Operating contract (executor rules — keep these)

- Never `git commit`/`git push`; stage **explicit paths only**, never `git add -A`; never stage `.claude/`.
- Never create/load/store private keys, seed phrases, wallet files, API secrets, or paid credentials.
- No signing, no mainnet (or any) submit path, no real-money execution path before the gated +
  approved milestones. Setting an RPC URL alone must never enable signing/submission.
- Determinism is non-negotiable: same input → identical orders/fills/balances/equity/metrics;
  fixed-point money (never f64 for money); ordered collections; no RNG/clock in canonical runs.
- Strategies emit intent (target weights) only — no keys, no RPC. Never trade a non-allowlisted token.
- Work in milestone order; small diffs; refresh `plans/current-state.md` + `worklog.md` in the same diff.
- No profitability claims from in-sample results.

**M4-specific (don't regress what S1–S11 established):**
- Parallelism is `std::thread::scope` only — **no new dependency** (rayon is rejected, D-0009).
  Any sweep output must stay byte-identical parallel==sequential==repeated; the gate lives in
  `crates/sweep/tests/determinism.rs` and must use **explicit** thread counts, never `available_parallelism`.
- Turnover comes from `RunOutput.traded_notional_quote`, **never** from `round_trips` (rebalancers
  close zero round trips while churning — round-trip turnover undercounts them to ~0).
- All sort/threshold/selection keys are `Decimal`; `f64` is display-only and non-finite values are
  normalized to `None` at `CellResult` construction (so structural equality can't flake on `NaN`).
- The holdout (S9, **done**) is sealed and must stay that way: parameter selection only ever sees
  `&DevValidation` (development/validation); `evaluate_on_holdout` consumes its seal by value (M5-only)
  and **must not be wired into any M4 CLI path** (S12). Don't add a holdout accessor to `DevValidation`.
- Walk-forward windows (S8) are index ranges over the dev/val array only: keep `train.end + embargo
  == test.start`, test ranges ordered + non-overlapping, and never index past `n_bars`. S9 runs
  `WalkForward::windows` over the dev/val partition length — the holdout is a *separate* allocation,
  so windows are structurally incapable of reaching it.
- Cost scenarios (S10) scale by **exact integer** arithmetic only (u32/i64 fields; `scale_cost_model`).
  Cost **monotonicity is NOT a law** — higher costs can suppress a marginal trade, so doubled can be
  cheaper / higher-return and `return_drag_doubled` can go negative (see m4-sweep §9). Do **not**
  re-impose monotonicity as a runtime invariant; `fee_sensitivity` records the signed drag honestly.
  Baselines run through the shared `cell::eval_strategy` core (same pipeline as candidates — no drift).
- The advancement report (S11) is **robustness-only** (invariant 11): `Advanceable` = "eligible for M5
  review", never "profitable". All threshold/selection keys are `Decimal` (`advance.rs`/`report.rs` are
  f64-free, source-audited). `SweepReport` verdicts sort by a **total** order so output is byte-identical;
  `schemas/sweep-report.schema.json` is an additive contract (D-0001) — **`run-result.schema.json` stays
  UNCHANGED**. Schema checks shape; code enforces semantics (status⇔`failed_criteria`).

---

## Seed prompt for the new chat — execute ONE M4 task card (paste this, filling in the card ID)

> You are the EXECUTOR continuing the **machina** (`solana-crypto-trader`) repo — a paper-first,
> Solana-focused crypto trading-**research** platform in Rust built toward **genuine autonomous
> passive income** (truly autonomous, so truly passive), earned through milestone gates. The current
> phase is research: no keys/signing/RPC/network anywhere.
>
> Your entire task this session is **one card: M4-C\<N\>** in `plans/task-queue.md` (the first card
> whose Status is TODO, taken in order). Open `plans/task-queue.md`, read the card queue's common
> rules ("How to execute a card", "Guardrails", "Escalate and STOP"), then your card. **The card is
> self-contained** — its files, verbatim current signatures, numbered steps, and runnable gate are all
> inline. Do not open files the card doesn't name; do not do more than the card says.
>
> **State:** M0–M3 done; M4 engine S1–S11 built and green (272 tests at `df18267`, 2026-07-06); the
> cards are the remainder of M4. **Do not commit or push; the operator commits** (stage explicit paths
> only, never `.claude/`, never `git add -A`).
>
> When the card's gate passes: flip the card's Status to DONE in `plans/task-queue.md` and append one
> line to `plans/worklog.md`. If ANY escalate-if condition triggers — a signature doesn't match the
> card, an unrelated test fails, you need an unlisted file, or anything ambiguous touches money,
> determinism, the holdout, or schemas — **STOP, record the mismatch in `plans/worklog.md`, and
> report** instead of improvising.
>
> Non-negotiables (restated on every card): `Decimal`/integer only for money — never f64; parallel
> output byte-identical to sequential; no new dependencies; no execution/signing/RPC code; the holdout
> stays sealed — **never call `evaluate_on_holdout`**; never edit `schemas/*.json` or
> `plans/master-plan.md`. After card M4-C10, **STOP — M5 is an operator decision** (freeze
> questions.md Q5 thresholds; choose the Q3 data source; explicit go). **Never start M8/M9**
> (signing/submit) — separate explicit human approval required.
