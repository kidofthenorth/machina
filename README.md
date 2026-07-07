# machina — solana-crypto-trader

A **paper-first, Solana-focused crypto trading research platform** built toward one goal: **genuine
autonomous passive income** — low-touch, hands-off on-chain spot trading for a solo operator, once a
strategy has earned that trust. Getting there means first proving a strategy has a *robust edge after
costs, latency, slippage, and operational failure*; **no strategy is guaranteed to clear that bar,
and live trading starts only once its own milestone gate is explicitly approved.**

> ⚠️ **This software is research infrastructure.** Nothing here is investment advice or a profit
> claim. In-sample backtest results are not evidence of future returns.

## The one rule that cannot be broken

**Money-moving capability is gated by milestone *and* by explicit human approval.** No key loading,
transaction signing, or transaction submission exists — or may be added — before the research phase
(M0–M5) is complete and separately approved. **Building the software never authorizes trading.**
See [docs/invariants.md](docs/invariants.md).

## Status

**Milestone M4 (sweep / walk-forward) in progress: the deterministic sweep engine (S1–S11) is built
and green; only the CLI wiring + gate declaration remain.** This is research-only code: there is
**no** key handling, **no** RPC client, **no** Jupiter client, **no** signing path, and **no**
transaction-submission path anywhere in the workspace. The roadmap (M0–M11) lives in
[plans/master-plan.md](plans/master-plan.md).

Current build state for agents/operators: [plans/current-state.md](plans/current-state.md).

## What exists today

| Crate | Purpose | Milestone |
|-------|---------|-----------|
| [`research-core`](crates/research-core) | Shared domain types: fixed-point money (`Amount`/`Price`), tokens, bars, UTC time | M0/M1 |
| [`market-data`](crates/market-data) | Bar-series validation, token metadata, allowlist loader | M1 |
| [`portfolio`](crates/portfolio) | Portfolio state, swap intents, deterministic fill model, simulator, equity curve, round trips | M2 |
| [`metrics`](crates/metrics) | Performance metrics from an equity curve (return, drawdown, Sharpe, Sortino, …) | M2/M3 |
| [`strategies`](crates/strategies) | Strategy trait + baselines + `trend_alloc_v1` / `threshold_rebalance_v1` scaffolds | M3 |
| [`results`](crates/results) | `RunConfig` / `RunResult` models + JSON-schema validation | M0/M2 |
| [`sweep`](crates/sweep) | Deterministic parameter sweeps: parallel execution (`std::thread::scope`), walk-forward windows, sealed holdout, cost/fee sensitivity, advancement/rejection report | M4 |
| [`cli`](crates/cli) | Deterministic demo runner that wires the above end-to-end | M2/M3 |

Deferred crates (named in the plan, **not yet created**): `route-model` (M6),
`solana-execution` (M6/M8), `wallet-state` (M7), `risk` (M6). See
[docs/architecture-index.md](docs/architecture-index.md).

## Determinism contract

Same input data → **identical** orders, fills, balances, equity curve, and metrics, byte-for-byte.

- **Fixed-point money everywhere** (`rust_decimal::Decimal`) — never floating point for balances,
  prices, fees, or equity. Statistical reporting metrics (Sharpe/Sortino/volatility) use `f64` and
  are display-only; they are never fed back into accounting.
- **Next-bar execution.** A strategy sees bars `[0..=t]` and its target is executed at bar `t+1`'s
  open. The final bar's signal therefore cannot open a position (no lookahead).
- **No nondeterministic iteration** in canonical output (ordered collections only).

## Build & test

```bash
cargo build              # build the workspace
cargo test               # run all tests (must be deterministic and repeatable)
cargo fmt --check        # formatting gate
cargo clippy --all-targets --all-features -- -D warnings   # lint gate
cargo run -p cli -- demo # run the deterministic demo (research only — no network, no keys)
```

CI runs the same gates from a clean checkout: [.github/workflows/ci.yml](.github/workflows/ci.yml).

## Configuration

Only **templates** are tracked, named `*.example.toml`. Copy a template, drop the `.example`, and
fill in real values locally — real config is gitignored. There are **no secrets** in any tracked
config; RPC provider entries are URLs/labels only, never API keys.

- [`config/tokens/allowlist.example.toml`](config/tokens/allowlist.example.toml) — the token
  allowlist (a first-class research artifact). **No strategy may trade a token absent from it.**
- [`config/rpc/providers.example.toml`](config/rpc/providers.example.toml)
- [`config/costs/solana-mainnet.example.toml`](config/costs/solana-mainnet.example.toml)
- [`config/strategies/strategy-lab.example.toml`](config/strategies/strategy-lab.example.toml)

## How work happens here

Plan → small diffs → fresh-session review, in milestone order. The active plan and live state are in
[`plans/`](plans/); architecture and non-negotiable invariants are in [`docs/`](docs/). See
[AGENTS.md](AGENTS.md) and [CLAUDE.md](CLAUDE.md).

## License

Unlicensed / private research project. Do not redistribute strategy logic or accounting code as
production-ready.
