# DECISIONS

Architecture decision log. Newest first. Each entry: context → decision → consequences. Keep terse.
These are *implementation* decisions made by the executor within the bounds of the master plan; they
do not authorize any new capability (no signing, no submission — see
[docs/invariants.md](docs/invariants.md)).

---

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
