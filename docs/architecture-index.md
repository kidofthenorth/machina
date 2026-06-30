# Architecture index

A small map for future agents. Read this before exploring. The authoritative roadmap is
[../plans/master-plan.md](../plans/master-plan.md); the rules are [invariants.md](invariants.md).

## Shape

A Cargo workspace of research-only crates. Data flows one direction; nothing below calls the network
or signs anything.

```
bars + tokens (market-data)
  → allowlist + validation
  → strategy target weights (strategies)        ← intent only, no RPC/keys
  → swap intents → deterministic fill model (portfolio)
  → portfolio state update → equity curve + round trips
  → metrics (metrics)
  → RunResult, validated against schemas/ (results)
  → wired end-to-end by the demo runner (cli)
```

## Crates that exist (M0–M3)

| Crate | Path | Responsibility | Depends on |
|-------|------|----------------|------------|
| `research-core` | `crates/research-core` | Domain foundation: `Amount`/`Price` (fixed-point money), `Symbol`/`MintAddress`/`TokenMeta`, `Bar`/OHLCV, `Timestamp` (UTC). No I/O. | — |
| `market-data` | `crates/market-data` | Bar-series validation (sorted/unique/OHLC/volume/gap), token metadata, allowlist load + checks. | research-core, serde, toml |
| `portfolio` | `crates/portfolio` | `PortfolioState`, swap fills, deterministic `CostModel` fill model, `simulator::run` (next-bar execution), `EquityPoint`, `RoundTrip`. | research-core, rust_decimal |
| `metrics` | `crates/metrics` | Performance metrics from an equity curve. Decimal for return/drawdown; f64 (display-only) for Sharpe/Sortino/vol. | research-core, rust_decimal |
| `strategies` | `crates/strategies` | `Strategy` trait (history → target weights, intent only), baselines (incl. `dca_sol`), `regime::classify` scaffold, `trend_alloc_v1` / `threshold_rebalance_v1` scaffolds. | research-core, rust_decimal (dev: portfolio) |
| `results` | `crates/results` | `RunResult`/`RunConfig` DTOs (money as exact decimal strings) + JSON-schema validation tests. | research-core, portfolio, metrics, serde, serde_json, (dev) jsonschema |
| `cli` | `crates/cli` | Deterministic demo runner: load fixtures → run a strategy → metrics → RunResult JSON. No network, no keys. | all of the above |

## Crates deferred (named in the plan, NOT yet created)

Created in milestone order as content arrives. Deliberately absent so the tree has no execution
surface (invariant 1/2).

| Crate | Milestone | Purpose |
|-------|-----------|---------|
| `sweep` | M4 | Parameter sweeps, parallel + canonical export, walk-forward. |
| `route-model` | M6 | Route/quote modeling from captured Jupiter data (no live calls in-crate). |
| `solana-execution` | M6/M8 | Quote/order adapter (no-sign/no-submit first); devnet path later, gated. |
| `wallet-state` | M7 | Read-only wallet snapshot + reconciliation. |
| `risk` | M6 | Risk gates (reject increasing exposure; never block de-risking). |

## Top-level layout

```
Cargo.toml                  workspace manifest (members + shared deps + lints)
rust-toolchain.toml         pinned stable toolchain
rustfmt.toml                formatting (CI gate)
.github/workflows/ci.yml    fmt + clippy + test + no-execution-deps scan
config/                     *.example.toml templates only (real config gitignored)
schemas/                    JSON Schema contracts (run-result, route-quote, execution-event, wallet-snapshot)
crates/                     the workspace members above
docs/                       this file + invariants.md
plans/                      master-plan, current-state, task-queue, questions, worklog, review-packet
```

## Conventions

- Fixed-point money everywhere (`rust_decimal::Decimal`); f64 only in metrics statistical functions.
- Strategy naming `<family>_v<n>`.
- UTC internally; bars sorted + unique; OHLC invariants enforced; volume ≥ 0.
- Determinism: ordered collections in canonical output; no RNG/wall-clock in canonical runs.
