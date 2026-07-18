# Handoff — for a new chat continuing machina

You are picking up a build where **M5 is CLOSED (reject-all, holdout unread) and the M-HF
research track is ACTIVE**. Cards **C1–C2.6, C3/C4/C5, C5.1, and C6 are all DONE**; the C3–C5
adversarial checkpoint is closed. **The `sweep` shasum is now `85d06e5b…`** (C6 bumped the report
schema to 1.2.0); demo shasum still `ae064f79…`; workspace **391/0/1**. This is the single entry
point. **C7 is DONE (2026-07-16, committed at `f09761b`, independently re-verified):** NARROW scope
— `intraday_meanrev_v1` (survey §1.2) as a single-series intent-only `Strategy` + a ≥100k-bar
deterministic `run_hf` cadence proof. Workspace is now **402 passed / 0 failed / 1 ignored**;
demo shasum `ae064f79…` and sweep shasum `85d06e5b…` both UNCHANGED (the C7 byte-identity guard
held); sweep-verify OK. **C8 is C8.1-C8.7** (task-queue.md's "M-HF-C8 planner reconciliation"
section) **and C8.1-C8.4 + CARD-HYG-1 are now DONE + planner-verified, committed at
`3757d2a`+`ddd4512` (2026-07-17):** `ScenarioId` HF rungs; `data_provenance` rollup — sweep schema
**1.3.0**, sweep shasum moved once to `94e90c3c6060a11feddd8d55a19accf07a86f7d8`;
`ParamGrid::IntradayMeanRev`; `sweep::intraday_partition` (the S9-pattern intraday holdout seal);
gnhf e2e-test port. Workspace is **433 passed / 0 failed / 1 ignored**; demo shasum `ae064f79…`
unchanged; sweep-verify OK — gate freshly re-verified 2026-07-18 at `ddd4512`, clean tree.
**The next command is executing C8.5** (`HfSweepSpec` HF-kind TOML parsing + spec-lint,
parse-only). Its planner pre-verify ran 2026-07-18: all pinned shapes hold; three drifts pre-ruled
(see current-state.md "Next recommended command"). **C8.6** (HF cost/adversarial execution wiring)
and **C8.7** (the real row-C8 gate: windowed HF sweep) are scoped, not signature-pinned - each
needs its own planner reconciliation pass once its prerequisites land. **`statarb_pairs_v1` stays
scoped OUT of C8**, in the deferred mini-track `M-HF-C8-PAIR` (operator ruling HF-Q4; still
blocked on the unsupplied USDT/jitoSOL allowlist fields). Never start M6+ (network) or M8/M9
(signing/submit — separate explicit human approval). **Do not commit or push — the operator
commits.**

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
2. **[plans/task-queue.md](task-queue.md) — the M4 task queue, CLOSED**: cards M4-C1…M4-C10 are all
   DONE. Nothing in it is active; do not resume it. Design rationale lives in
   [plans/m4-sweep.md](m4-sweep.md) (S1–S11 DONE).
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
cargo test --workspace --all-features                    # 313 passed, 0 failed (2026-07-12, M5 gate declaration)
cargo run -p cli -- demo                                 # deterministic, schema-valid RunResult (shasum ae064f79242f823ffd8f55bf9104e3e1b45d425a, identical on repeat)
cargo run -p cli -- sweep                                # deterministic sweep report (shasum 7ad3df7de2e2c1139be427e9c953b57d4e289cb3, identical ×2 and at --threads 8)
cargo run -p cli -- sweep-verify                         # sweep-verify: OK — byte-identical across sequential, 2 and 8 threads, and repeat (18990 bytes); exit 0
# no-execution-deps scan (CI parity) — must print OK:
pattern='solana-sdk|solana-client|solana-program|solana-rpc|jupiter|jito|ed25519-dalek|keypair|bip39|tiny-bip39|secp256k1'
grep -REn --include='Cargo.toml' "$pattern" . | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#' || echo OK
```

Repo state (2026-07-12): **M5 gate declared (card M5-C6, Branch A reject-all).** The operator has
committed through M5-C5; the C6 declaration diff (plan files only) is staged pending operator
commit. Additional M5 evidence artifacts now checked in: `plans/m5-sweep-report.json`,
`plans/m5-data-validation.md`, `config/strategies/m5-frozen.toml` (immutable records). The real
snapshot lives in gitignored `data/raw/binance/SOLUSDC-1d` (916 bars; never staged). **Never**
`git add -A`; **never** stage `.claude/`. Stage explicit paths only. (Run `git status` to confirm
current HEAD and what's staged.)

## What's next — M5 is CLOSED (gate declared 2026-07-12); the HF track is the road ahead

**M5 outcome: reject-all (Branch A).** Decisive real-data sweep `plans/m5-sweep-report.json`
(sha256 `424e713f…fb40`, three-run byte-identical; 180 trials, 10 verdicts): every candidate
failed the frozen +0.02 baseline margin and edge-vanishes-under-doubled-costs (thresholds frozen
2026-07-11 in `config/strategies/m5-frozen.toml` BEFORE any real-data result — see
`plans/m5-data-validation.md`). `evaluate_on_holdout` was never called; the 2026-01-01..2026-06-30
holdout survives unseen for a future cycle. Step-0 review: PASS (6 agents).

**The active queue is task-queue.md §M-HF.** C1–C2.6, C3/C4/C5, C5.1, and **C6 (DONE + verified,
narrow: turnover-criterion replacement; sweep schema 1.2.0; new sweep shasum `85d06e5b…`)** are done;
the C3–C5 checkpoint is closed and m-hf-track §4/§5 was corrected 2026-07-15 to match the landed
scope. **C7 is DONE (2026-07-16 at `f09761b`, re-verified by a fresh session): `intraday_meanrev_v1`
ONLY** (survey §1.2, single-series intent-only mean-reversion; 9 unit + 2 cadence tests; ≥100k-bar
deterministic `run_hf` run; demo+sweep shasums unchanged — 402/0/1). **The C8 planner pass is DONE
(2026-07-16, plan files only): C8 is split into C8.1-C8.7 plus a deferred mini-track.** C8.1
(`ScenarioId` ladder enum), C8.2 (`data_provenance` rollup), and C8.3 (`ParamPoint`/`ParamGrid` gain
`IntradayMeanRev`) are **executor-ready now** - small, mechanical, additive, no operator input
needed. C8.4 (intraday holdout seal, S9-pattern) and C8.5 (HF-kind spec parsing) are drafted but
want a fresh re-verify right before executing. C8.6 (wire `HfCostModel`/`AdversarialModel` into a
new execution entry) and C8.7 (the real row-C8 gate: windowed HF sweep wiring across thread counts)
are scoped, not signature-pinned - each needs its own planner reconciliation pass once its
prerequisites land, mirroring the C6→C8/C7→C8 pattern. **`statarb_pairs_v1` (survey §1.6, SOL/LST
pair) is scoped OUT of C8 into a deferred mini-track, `M-HF-C8-PAIR`** (operator ruling HF-Q4,
2026-07-16: build the real two-leg engine, not a precomputed-spread approximation, because the
approximation risks the exact fabricated-edge failure mode the cost ladder exists to catch) - it
still can't run on the single-series engine, has no correlated-LST synthetic data, and needs the
HF-Q2 allowlist edit (jitoSOL/USDT approved 2026-07-12; the verbatim `[[token]]` block, with
operator-supplied mint/survivorship fields, still hasn't been supplied - it now lands with whichever
future PAIR card first needs it, not C8). HF-Q1 (deferred to wave 2) blocks C9/C10;
HF-Q3 blocks C10. Neither daily-bar family advances; M6 (shadow) has no candidate and does not start.

The Q7 HF amendment was made 2026-07-09 (D-0012): master-plan pair amended in lockstep
(cmp-verified); milestone plan at [m-hf-track.md](m-hf-track.md). M-HF execution still waits on
HF-Q1/HF-Q2 (questions.md).

Do NOT start M6+ (network), and never M8/M9 (signing/submission — separate explicit human approval).

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

## Seed prompt for the new chat — execute M-HF-C8.5

> You are the EXECUTOR for **machina** (`solana-crypto-trader`) — a paper-first, Solana-focused
> crypto trading-**research** platform in Rust built toward **genuine autonomous passive income**
> (truly autonomous, so truly passive), earned through milestone gates. The current phase is HF
> research on SYNTHETIC data only: no keys/signing/RPC/network anywhere; every HF strategy family
> is a hypothesis to test — no card claims a known-profitable algorithm, and rejecting them all
> cleanly is a successful outcome.
>
> **State (2026-07-18, HEAD `ddd4512`, clean tree):** M5 CLOSED (reject-all; holdout unread, seal
> intact). M-HF C1–C7 and **C8.1–C8.4** DONE + planner-verified. Workspace green at **433 passed,
> 0 failed, 1 ignored**; demo shasum `ae064f79242f823ffd8f55bf9104e3e1b45d425a`; sweep shasum
> `94e90c3c6060a11feddd8d55a19accf07a86f7d8` (schema 1.3.0 since C8.2); `sweep-verify: OK`.
>
> **Your card is M-HF-C8.5** (`HfSweepSpec`: HF-kind TOML parsing + spec-lint — PARSE-ONLY, not
> wired into `run_sweep`/`run_hf`/CLI; that is C8.7's job). Read the card in full in
> task-queue.md's "M-HF-C8 planner reconciliation (2026-07-16)" section before touching anything.
> Exactly 4 files + the queue/worklog flip: NEW `crates/sweep/src/hf_spec.rs`, additive `lib.rs`
> re-export, NEW `config/strategies/hf-strategy-lab.example.toml` (illustrative, NOT tuned), NEW
> `crates/sweep/tests/hf_spec_parsing.rs`. Do NOT touch `spec.rs`, `strategy-lab.example.toml`,
> `m5-frozen.toml`, or any schema.
>
> **The planner pre-verify already ran (2026-07-18) against `ddd4512` — all pinned shapes hold**
> (`HfCostModel{base,depth_curve,congestion_priority_table,tip_bps}`, `DepthCurve::new` /
> `CongestionPriorityTable::new` fail-closed constructors, `AdvancementThresholds`'s
> `turnover_budget: Option` + `cost_drag_share_ceiling`/`per_trade_edge_floor` pair,
> `ParamGrid::IntradayMeanRev`, `PartitionSpec` reuse; `toml`/`serde` are already sweep deps).
> **Three drifts are PRE-RULED — do not escalate on them:** (1) the card's "grep `hf_spec` returns
> nothing" is stale: `crates/sweep/tests/hf_reuse_proof.rs:36` has a LOCAL test helper
> `fn hf_spec()` — no module/type collision, leave that file alone and proceed; (2) baseline gate
> numbers are the 433/0/1 + `94e90c3c…` set above, not the card's older 402/`85d06e5b…`; (3) the
> card's quoted line numbers may drift by a few lines — shapes govern, not line numbers. For the
> template's `[hf_cost]` values, mirror `reference_depth_curve`/`reference_priority_table` in
> `crates/portfolio/src/hf_cost.rs`'s test module (≈:266/:284). Any OTHER mismatch vs the card's
> pinned shapes: STOP and report rather than adapt.
>
> Gate (from the card): `cargo test -p sweep hf_spec` green; full workspace fmt/clippy clean;
> 0 failed / 1 ignored, record exact N; demo AND sweep shasums **unchanged** (parse-only — if
> either moves, STOP); `git status` shows exactly the 4 files + queue/worklog flip. When the gate
> passes: flip the card, one worklog line, stop — the next card gets a fresh chat. C8.6/C8.7 need
> their own planner reconciliation pass first — never execute their scoped sketches.
>
> Non-negotiables (unchanged): `Decimal`/integer only for money — never f64; no new dependencies;
> determinism (no RNG/clock in canonical runs); the M4/M5 holdout machinery and
> `config/strategies/m5-frozen.toml` are immutable records; never stage anything under `data/`;
> never edit `fixtures/` (existing files), `plans/master-plan.md`, or `solana-crypto-trader-plan.md`;
> **do not edit any `schemas/*.json`** (C8.5 authorizes none); never `git commit`/`git push` (stage
> explicit paths only, never `.claude/`, never `git add -A`); never start M6+ (network) or M8/M9
> (signing/submit — separate explicit human approval).
