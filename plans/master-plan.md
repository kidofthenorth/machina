# Solana Crypto Trader: Fresh Repository Build Plan

## 1. Mission

Build a paper-first, Solana-focused crypto trading platform that can research, simulate, and later
paper-shadow on-chain spot strategies without pretending that low fees alone create an edge.

The system is optimized for:

- Deterministic research over crypto bars, quotes, liquidity snapshots, and executed-route data.
- Solana spot trading with explicit modeling of network fees, priority fees, slippage, failed
  transactions, and route quality.
- Strict token allowlists and wallet/key isolation.
- Low-touch operation after a strategy is proven, not constant discretionary intervention.
- A path from backtest -> shadow mode -> tiny-capital canary -> larger deployment only after
  objective gates pass.

This plan does not promise passive income. It creates the machine for discovering whether a
strategy has a robust edge after costs, latency, slippage, and operational failures.

## 2. Strategic Position

### Why Solana is worth considering

Solana meaningfully improves two crypto-bot constraints:

- Low base transaction fees. Solana documentation lists a base fee of 5,000 lamports per
  signature, plus an optional prioritization fee.
- High activity and deep DeFi routing. Jupiter aggregates Solana swap routes and exposes an API
  that can return assembled transactions, managed execution, and route competition across routers.

Those advantages make Solana a reasonable first crypto lane, especially compared with retail CEX
fee schedules where frequent trading can be eaten by maker/taker fees.

### What Solana does not solve

Solana does not automatically solve:

- Strategy edge.
- Slippage.
- MEV and adverse selection.
- Bad token liquidity.
- Rug pulls and malicious token behavior.
- Failed transactions.
- RPC rate limits and provider outages.
- Priority-fee selection.
- Wallet/key security.
- Data quality and survivorship bias.

The plan therefore treats Solana as an execution venue with attractive mechanics, not as the source
of profit.

### Product stance

Start with liquid, allowlisted Solana spot pairs and conservative time horizons.

Initial focus:

- SOL/USDC
- BTC or wrapped BTC exposure only after SOL/USDC research and shadow execution are stable
- ETH or wrapped ETH exposure only after SOL/USDC research and shadow execution are stable
- Major Solana liquid-staking tokens only after peg/liquidity risks are modeled

### Income thesis

The first thesis is not "a crypto bot wins every day." It is:

> A low-turnover SOL/USDC allocation engine can capture enough SOL upside while avoiding enough
> severe downtrends to produce drawdown-controlled compounding after realistic Jupiter execution
> costs.

The second thesis, after the first simulator and shadow logger are trustworthy, is:

> SOL/USDC threshold rebalancing can harvest volatility in range-bound regimes without taking the
> hidden tail risk of an unmanaged grid system.

The third thesis, later, is:

> Concentrated-liquidity range management can produce fee income only if impermanent loss,
> out-of-range inventory, and rebalancing costs are explicitly modeled and monitored.

The project must report return targets as a function of capital. For example, $100/day requires
about $36,500/year before taxes and infrastructure costs. That is 36.5%/year on $100,000,
73%/year on $50,000, and 146%/year on $25,000. Smaller accounts therefore require either a very
strong edge or much higher risk.

Explicitly avoid at first:

- Memecoin sniping
- Newly launched tokens
- Rug-pull hunting
- Copy-trading wallets
- Sandwiching
- Liquidation racing
- Cross-DEX latency arbitrage
- Leveraged perpetuals
- Borrow/lending loops
- Thin pools
- Any token without an allowlist entry and liquidity history

## 3. Recommended Platform Direction

Use a Rust-first monorepo with a small optional TypeScript boundary only if a Solana SDK or Jupiter
client is materially safer/faster there.

Default:

- Rust for deterministic research, portfolio simulation, sweeps, walk-forward, metrics, route
  modeling, and the core execution state machine.
- Rust for Solana RPC and transaction submission if the SDK surface is stable enough.
- TypeScript only as an adapter process if it substantially reduces Solana/Jupiter integration
  risk.
- React dashboard later, after the core API and event model are stable.

Why:

- Rust is appropriate for high-volume historical simulation and strict accounting.
- Rust reduces accidental floating-point/accounting errors when paired with fixed-point types.
- Solana execution is latency-sensitive enough that a typed, compiled runtime is useful.
- A narrow adapter boundary prevents SDK churn from infecting strategy or research code.

## 4. Repository Layout

```text
solana-crypto-trader/
  AGENTS.md
  README.md
  DECISIONS.md
  Cargo.toml
  plans/
    master-plan.md
  config/
    tokens/
      allowlist.example.toml
    strategies/
    costs/
    rpc/
      providers.example.toml
  schemas/
    run-result.schema.json
    target-portfolio.schema.json
    execution-event.schema.json
    route-quote.schema.json
    wallet-snapshot.schema.json
  crates/
    market-data/
    research-core/
    strategies/
    portfolio/
    metrics/
    sweep/
    route-model/
    solana-execution/
    wallet-state/
    risk/
    results/
    cli/
  execution/
    optional-ts-adapter/
  api/
  frontend/
  data/
    raw/
    normalized/
  results/
  scripts/
```

Generated data, private keys, seed phrases, wallet files, RPC credentials, databases, and exports
must be gitignored.

## 5. Core Architecture

### Research pipeline

```text
market data
  -> token and pool validation
  -> deterministic bar/quote/liquidity series
  -> strategy signal
  -> target portfolio
  -> route and fill simulation
  -> portfolio state update
  -> equity curve and round trips
  -> metrics
  -> canonical result export
```

### Shadow/paper execution pipeline

```text
wallet snapshot
  -> token account reconciliation
  -> latest bars, quotes, and liquidity
  -> strategy signal
  -> target portfolio
  -> risk validation
  -> quote route
  -> simulated execution or unsigned transaction build
  -> decision log
  -> post-decision monitoring
```

### Live canary pipeline, later only

```text
wallet snapshot
  -> reconcile
  -> risk gate
  -> quote route
  -> simulate transaction
  -> sign with hot-wallet spending cap
  -> submit with priority-fee policy
  -> confirm or expire
  -> reconcile exact token balances
  -> alert on drift
```

The strategy produces portfolio intent. It must not own signing keys, call RPC directly, or mutate
execution state.

## 6. Solana-Specific Execution Model

### Required core types

- `TokenId`
- `MintAddress`
- `TokenAccount`
- `WalletSnapshot`
- `Bar`
- `QuoteSnapshot`
- `LiquiditySnapshot`
- `PoolSnapshot`
- `RouteQuote`
- `SwapIntent`
- `UnsignedTransaction`
- `SubmittedTransaction`
- `ConfirmationState`
- `Fill`
- `PortfolioState`
- `Position`
- `CashOrStableBalance`
- `EquityPoint`
- `EquityCurve`
- `RoundTrip`
- `Metrics`
- `RunConfig`
- `RunResult`

### Research event ordering

For each decision timestamp:

1. Load completed market data only.
2. Mark current token balances using a deterministic pricing source.
3. Present exact portfolio state to the strategy.
4. Produce target weights.
5. Convert target weights into swap intents.
6. Simulate route quote, slippage, DEX fees, network fee, and priority fee.
7. Apply accepted fills in deterministic order.
8. Record token balances, stable balance, equity, costs, and route metadata.

### Live/shadow event ordering

For each decision cycle:

1. Fetch wallet token balances from RPC.
2. Fetch open or recently submitted transaction signatures.
3. Reconcile prior submitted transactions.
4. Fetch market data and route quotes.
5. Run strategy and risk.
6. Build proposed swap intents.
7. In shadow mode, record the quote and simulated fill without signing.
8. In canary mode, simulate the transaction before signing.
9. Submit only if the quote, simulation, slippage, priority fee, and risk gates all pass.
10. Wait for confirmation or expiration.
11. Reconcile exact post-trade token balances.

No live code path exists in M0-M6. The first live-capable path must be separately approved.

## 7. Venue and API Strategy

### Jupiter-first routing

Use Jupiter as the first DEX routing integration because it is the dominant Solana routing surface
and exposes a single swap API for route quotes and assembled transactions.

Default integration:

- Jupiter Swap V2 `/swap/v2/order` for routes and transaction preparation.
- Jupiter Swap V2 `/swap/v2/execute` only after shadow mode proves route quality.
- Route metadata capture.
- Slippage threshold.
- Price-impact threshold.
- Minimum received amount.
- Platform/referral fees disabled unless explicitly configured.

Do not build the new bot around older Ultra or Legacy Metis assumptions. Ultra and legacy-style
paths may remain useful references, but the plan's implementation target is Swap V2 unless current
docs change before implementation.

Advanced custom transaction building is deferred until the default route path is understood.
If custom composition becomes necessary, use the build/submit path as a separate milestone with its
own tests for compute units, priority fees, tips, blockhash expiry, duplicate-send prevention, and
balance reconciliation.

Every quote/order/execute attempt must persist:

- Router/source.
- Input mint and amount.
- Output mint and expected amount.
- Minimum received amount.
- Price impact.
- Slippage mode and threshold.
- Platform fee, DEX fee, base fee, and priority fee estimate.
- Request ID or equivalent correlation ID.
- Slot and observed timestamp.
- Latency timestamps for quote, build/order, sign, submit, confirm.
- Transaction signature, when submitted.
- Final status and error code.
- Pre/post token balances for realized accounting.

### RPC provider strategy

Do not rely on public RPC endpoints for production-like operation. Solana documentation warns that
public endpoints are shared infrastructure and may return rate-limit or block responses.

Use provider abstraction:

- `PublicRpcProvider` for local development only.
- `DedicatedRpcProvider` for shadow/canary.
- Optional backup provider for reads.
- Explicit policy for provider mismatch and stale slots.

Every RPC response used in a decision must include:

- Slot.
- Commitment.
- Provider ID.
- Timestamp observed by the bot.

### CEX fallback

A centralized exchange adapter is optional and later.

It can be useful for:

- Better historical OHLCV availability.
- Comparing SOL spot execution costs against DEX routing.
- Avoiding on-chain custody at the earliest paper stage.

But a CEX adapter must not replace the deterministic research core or dilute the Solana-specific
execution model.

## 8. Strategy Scope

### MVP strategy family

Start with one simple, explainable SOL/USDC allocation strategy:

- SOL/USDC trend/risk-off filter.
- Daily or 4-hour bars.
- Long SOL or hold USDC.
- Optional discrete target buckets, for example 0%, 25%, 50%, or 75% SOL.
- Threshold rebalancing only when target-vs-current drift exceeds a configured band.
- No leverage.
- No shorting.
- No memecoins.
- No multi-hop exotic tokens unless the router naturally uses them and route risk allows it.

Candidate signals:

- Price above/below long moving average.
- Fast/slow moving-average crossover.
- Trailing return with volatility filter.
- Breakout with volatility-adjusted position size.

Choose one trend/risk-off signal for MVP and pair it with threshold rebalancing. Treat the other
signals as benchmarks or later variants.

Required baselines:

- Hold USDC.
- Buy and hold SOL.
- Static 50/50 SOL/USDC, periodically rebalanced.
- DCA into SOL.
- Same strategy before execution costs.

The MVP advances only if it improves drawdown-adjusted results against relevant baselines after
conservative slippage, platform fee, base fee, priority fee, and failed-transaction assumptions.

### Portfolio expansion

After MVP:

- Add BTC/USDC and ETH/USDC if liquidity assumptions are reliable.
- Add SOL liquid-staking tokens only as a separate strategy class.
- Add stablecoin yield routing only after smart-contract and depeg risks are modeled.

### Explicitly deferred strategies

- Market making.
- Arbitrage.
- Sandwiching or backrunning.
- New-token sniping.
- Social sentiment trading.
- Wallet copy-trading.
- Perpetual futures.
- Unmanaged grid systems.
- Concentrated-liquidity range management.
- Cross-chain bridge strategies.
- NFT trading.

Concentrated-liquidity range management is deferred because it is the first "real income" lane but
also the first lane where impermanent/divergence loss, out-of-range inventory, and active
rebalancing can dominate visible fee income. It becomes a separate milestone only after the spot
allocation simulator and Jupiter shadow logger are reliable.

## 9. Portfolio and Risk Model

### MVP constraints

- Spot only.
- Long-only.
- USDC as the quote/cash asset.
- No borrow.
- No leverage.
- One risk asset in MVP.
- Maximum SOL exposure default: 50%.
- Maximum per-trade notional default: 10% of portfolio.
- Minimum wallet SOL reserve for network fees.
- No trade if estimated route impact exceeds configured threshold.
- No trade if expected received amount is below the backtest-modeled tolerance.

### Later portfolio constraints

- Per-token max weight.
- Category max weight.
- Stablecoin reserve.
- Turnover threshold.
- Volatility targeting.
- Drawdown halt.
- Daily loss halt.
- Cooldown after failed transaction cluster.
- Cooldown after RPC/provider disagreement.

### Risk invariant

Risk controls may reject new or increasing exposure. They must never block reducing risk, converting
back to USDC, or canceling/abandoning stale unsigned transactions.

## 10. Deterministic Research Engine

The research simulator must not depend on live RPC, live quotes, or asynchronous channels.

It must model:

- Token balances.
- USDC cash balance.
- DEX fee.
- Aggregator fee if configured.
- Solana base fee.
- Priority fee estimate.
- Slippage.
- Price impact.
- Failed route probability only as a scenario stressor, not as random behavior in canonical runs.
- Quote staleness.
- Final-bar liquidation policy.

Determinism requirements:

- Same input data yields identical orders, fills, balances, equity curve, and metrics.
- Parallel sweeps produce identical canonical output to sequential sweeps.
- Decimal or integer fixed-point arithmetic for token balances and quote values.
- Canonical result ordering independent of thread scheduling.

## 11. Data Engineering

### Required data sources

MVP:

- SOL/USDC OHLCV bars.
- USDC price fixed at 1.0 with explicit depeg scenario tests.
- Historical SOL liquidity proxy.
- Transaction cost model.
- Forward-captured Jupiter SOL/USDC quote snapshots for shadow validation.

Later:

- Pool liquidity snapshots.
- Executed transaction metadata.
- Priority-fee history.
- RPC slot/latency history.
- Token supply and mint metadata.
- Allowlist and delist history.

### Recommended data stack

Use different sources for different truths:

- Jupiter Swap V2 for live route quotes and execution planning.
- Birdeye or CoinGecko/GeckoTerminal for early OHLCV research.
- Helius for wallet, transaction, RPC, and confirmation monitoring.
- Pyth as an independent SOL/USD sanity-check oracle.
- DexScreener as a cheap liquidity and pair-discovery cross-check.
- Bitquery later for serious historical DEX trade and route reconstruction.

Jupiter is not a historical backtest data source by itself. It is the execution and forward quote
surface. Honest historical backtests need archived candles, trade/pool data, or a conservative
execution-cost model. The best replay data will be the bot's own forward-captured Jupiter quotes
once shadow mode has run long enough.

### Backtest quality tiers

- Tier A, quick research: SOL/USDC OHLCV plus conservative fee/slippage/priority-fee assumptions.
- Tier B, better research: historical DEX trades or liquidity snapshots plus route/liquidity
  filters.
- Tier C, best replay: forward-captured Jupiter quotes replayed exactly as the bot observed them.

No strategy can graduate to canary from Tier A evidence alone.

### Data validation

- UTC timestamps internally.
- Bars sorted and unique.
- No silent forward-fill.
- OHLC invariants enforced.
- Volume nonnegative.
- Token decimals verified from mint metadata.
- Token allowlist version embedded in every run.
- Missing data blocks a run unless explicitly configured as a scenario.

### Survivorship and token safety

The allowlist is a first-class research artifact.

Every token entry must include:

- Mint address.
- Symbol.
- Decimals.
- First allowed date.
- Last allowed date, if removed.
- Allowed venues/routes.
- Minimum liquidity.
- Minimum age.
- Custody notes.
- Risk category.

No strategy can trade a token absent from the allowlist.

## 12. Execution Safety

### Key management

Never store private keys, seed phrases, or wallet files in the repository.

Execution phases:

1. Research only: no keys.
2. Shadow mode: read-only wallet address, no signing key.
3. Devnet mode: devnet key only.
4. Canary mode: dedicated hot wallet with tiny capped funds.
5. Larger live mode: separate explicit approval.

Required controls:

- Hot-wallet balance cap.
- Daily spend cap.
- Per-trade cap.
- Token allowlist.
- Destination account validation.
- Quote expiry.
- Slippage limit.
- Transaction simulation before signing.
- Confirmation timeout.
- Post-trade balance reconciliation.
- Kill switch.

### Paper/shadow enforcement

Before live canary exists, the system must enforce:

- No mainnet private key loading.
- No transaction signing.
- No transaction submission.
- Shadow decisions only.

Tests must prove that setting a mainnet RPC URL alone cannot enable signing or submission.

## 13. Transaction Lifecycle

### States

- `Planned`
- `Quoted`
- `RejectedByRisk`
- `Built`
- `Simulated`
- `Signed`
- `Submitted`
- `Confirmed`
- `Expired`
- `Failed`
- `Reconciled`

### Required recorded fields

- Strategy version.
- Decision timestamp.
- Wallet address.
- Input mint.
- Output mint.
- Input amount.
- Expected output amount.
- Minimum output amount.
- Route source.
- Price impact.
- Slippage setting.
- Base fee estimate.
- Priority fee estimate.
- RPC provider.
- Slot.
- Transaction signature, if submitted.
- Confirmation status.
- Actual pre/post balances.

## 14. Validation and Selection

### Data partitions

Define before tuning:

- Development period.
- Validation/walk-forward period.
- Final untouched holdout.

Crypto regime changes are severe. Do not trust one bull-market backtest.

### Required evaluation

- Bull, bear, chop, and crash regimes.
- Cost sensitivity.
- Slippage sensitivity.
- Priority-fee sensitivity.
- Delayed execution scenarios.
- Quote staleness scenarios.
- Stablecoin depeg scenario.
- Liquidity drought scenario.
- RPC outage scenario.
- Walk-forward windows.
- Neighboring-parameter stability.
- Baseline comparison.

### Baselines

Every experiment compares against:

- Hold USDC.
- Buy and hold SOL.
- Simple periodic DCA into SOL.
- Same strategy before costs.
- Same strategy with doubled costs/slippage.

### Candidate advancement

A strategy advances only if:

- It beats relevant baselines after realistic costs.
- It survives doubled cost/slippage assumptions.
- It does not depend on one short period.
- Neighboring parameters remain acceptable.
- Drawdown fits a predefined risk budget.
- Turnover is economically plausible.
- Walk-forward results remain acceptable.
- Trial count is recorded.

## 15. Metrics

Required metrics:

- Total return.
- CAGR or annualized return where meaningful.
- Annualized volatility.
- Sharpe.
- Sortino.
- Maximum drawdown.
- Calmar.
- Ulcer index.
- Win rate.
- Profit factor.
- Average trade.
- Worst trade.
- Turnover.
- Time in market.
- Fees paid.
- Slippage paid.
- Priority fees paid.
- Failed transaction count in scenario tests.
- Quote-to-fill drift.
- Worst rolling 1-, 7-, 30-, and 90-day returns.
- Exposure by token.
- Stablecoin reserve percentage.

Crypto-specific metrics:

- Route price impact distribution.
- Priority fee distribution.
- Confirmation latency distribution.
- RPC error rate.
- Requote rate.
- Rejected-by-risk count.
- Token allowlist violations blocked.

## 16. Testing Strategy

### Research tests

- Buy debits USDC and credits token.
- Sell credits USDC and debits token.
- Oversell impossible.
- Unaffordable buy rejected.
- Token decimals handled exactly.
- Fees included in balances and equity.
- Final-bar signal cannot open a new position.
- Deterministic repeated runs.
- Parallel and sequential sweeps match.
- Maximum drawdown in [0,1].
- Long-only total return cannot go below -100% without fees exceeding capital by bug.

### Route/fill tests

- Minimum-output slippage guard works.
- Price-impact guard works.
- Quote expiry blocks stale execution.
- Priority fee included in cost.
- Failed route does not mutate balances.
- Partial or missing confirmation forces reconciliation before next exposure.
- Jupiter response edge cases are handled: empty transaction, null transaction, explicit error code,
  expired RFQ/route, failed execution, and invalid signed transaction.
- Duplicate-submit prevention holds across blockhash expiry and process restart.
- Accounting uses realized token balance deltas, not only quoted `outAmount`.

### Wallet/execution tests

- No private key required in shadow mode.
- Mainnet signing impossible until live canary feature is explicitly enabled.
- Hot-wallet cap enforced.
- Daily spend cap enforced.
- Unknown token blocked.
- Wrong mint blocked.
- Wrong token account blocked.
- RPC provider disagreement blocks new exposure.
- Kill switch blocks new exposure and allows risk reduction.

### End-to-end tests

- Fixture bars -> target portfolio -> simulated route -> balances -> metrics.
- Shadow decision writes audit event without signing.
- Devnet transaction can be simulated in a non-production test lane.
- Restart after submitted transaction reconciles before making a new decision.

## 17. Performance Requirements

Measure before optimizing.

Initial targets:

- Single SOL/USDC daily-bar run over several years in under one second.
- 10,000-run parameter sweep in minutes on desktop hardware.
- Parallel output identical to sequential output.
- Route simulation and accounting bounded by per-run state.

Do not optimize for sub-second live execution until a strategy actually requires it and has proven
edge after realistic latency assumptions.

## 18. Observability and Operations

Required dashboard/API signals:

- Current mode: research, shadow, devnet, canary, live-disabled.
- Wallet address.
- Wallet balances.
- Last successful decision cycle.
- Last market data timestamp.
- Last RPC slot.
- RPC provider status.
- Current target portfolio.
- Pending transaction state.
- Last quote and route.
- Last rejected risk reason.
- Equity and drawdown.
- Fee and slippage summary.
- Kill switch status.

Alerts:

- RPC stale or unavailable.
- Quote stale.
- Transaction simulation failure.
- Transaction confirmation timeout.
- Balance reconciliation mismatch.
- Unknown token appears in wallet.
- Hot-wallet cap exceeded.
- Daily spend cap hit.
- Drawdown halt.
- Failed transaction cluster.
- Route price impact too high.

## 19. Delivery Milestones

### M0: Repository bootstrap

Deliver:

- Repository structure.
- `AGENTS.md`, `README.md`, `DECISIONS.md`, and master plan.
- Rust formatting, linting, tests, and CI.
- Secret-safe config templates.
- JSON schemas for run result, route quote, execution event, and wallet snapshot.

Gate:

- Clean CI from an empty checkout.
- No key, RPC credential, or live-submit path exists.

### M1: Data foundation

Deliver:

- Bar types.
- Token metadata and allowlist.
- Data validation.
- Small checked-in fixtures.

Gate:

- Corrupt, duplicate, unsorted, missing, bad-decimal, and disallowed-token tests pass.

### M2: Deterministic spot simulator

Deliver:

- Portfolio state.
- Swap intent.
- Fill/route model.
- Fees/slippage/priority-fee accounting.
- Equity curve and round trips.

Gate:

- Accounting and deterministic-regression tests pass.

### M3: Strategy Lab MVP

Deliver:

- `trend_alloc_v1`.
- `threshold_rebalance_v1`.
- Regime classifier scaffold.
- Strategy comparison report.
- Baselines.
- Cost and slippage scenarios.

Gate:

- Both MVP strategy families produce explainable orders, fills, balances, equity, and metrics.
- Results are compared against hold USDC, buy-and-hold SOL, static 50/50 SOL/USDC, and DCA.
- No strategy advances unless drawdown-adjusted performance survives doubled
  fee/slippage/priority-fee assumptions.

### M4: Sweep and walk-forward

Deliver:

- Parameter sweeps for the MVP strategy families.
- Parallel execution.
- Canonical result export.
- Walk-forward windows.
- Strategy-family comparison.
- Turnover and fee-sensitivity reporting.
- Rejection report for failed candidate families.

Gate:

- Parallel and sequential results match.
- Repeated runs are identical.
- Untouched holdout remains untouched.

### M5: Research decision

Deliver:

- Trial count.
- Stability surfaces.
- Walk-forward report.
- Baseline comparison.
- Cost/slippage/latency sensitivity.
- Strategy-family ranking.
- Explicit advance or reject decision.

Gate:

- One candidate is selected for mainnet shadow because it satisfies predefined robustness and
  drawdown criteria, or all candidates are rejected and the project returns to research.
- No execution work starts merely because the software exists.

### M6: Jupiter shadow quote collector

Deliver:

- RPC provider abstraction.
- Jupiter Swap V2 quote/order adapter in no-sign/no-submit mode.
- Quote snapshot persistence.
- Route quality, price impact, platform fee, priority fee, and latency capture.
- Shadow decision log that records whether the strategy would have traded.
- Risk controls.

Gate:

- Shadow mode can run without signing keys and cannot submit transactions.
- The system can replay captured quotes into the simulator.
- Backtest-modeled execution and live observed quote quality are compared.

### M7: Mainnet shadow strategy

Deliver:

- Read-only wallet snapshot.
- Strategy runs on live completed bars.
- Jupiter quotes captured for every would-trade decision.
- No signing key.
- No transaction submission.
- Incident log.

Gate:

- Shadow decisions remain reconciled.
- Quote assumptions are within modeled tolerances over a meaningful sample.
- No canary starts until shadow quote drift, route failure rate, and priority-fee assumptions are
  understood.

### M8: Devnet/canary transaction path

Deliver:

- Transaction simulation.
- Devnet signing/submission.
- Confirmation tracking.
- Post-transaction reconciliation.
- Hot-wallet cap.

Gate:

- Devnet tests pass.
- Mainnet canary remains feature-disabled unless separately approved.

### M9: Tiny-capital canary, optional

Only after separate approval:

Deliver:

- Dedicated hot wallet.
- Tiny balance cap.
- Strict per-trade and daily caps.
- Canary strategy version frozen.
- Manual kill switch verified.
- Balance-delta accounting after every transaction.
- Automatic disable on failed transaction cluster, quote drift, excessive priority fee, or
  provider/Jupiter degradation.

Gate:

- No unexplained balance drift.
- Realized fees/slippage within modeled limits.
- No failed transaction cluster.

### M10: Concentrated liquidity research, optional

Only after the SOL/USDC allocation path is stable:

Deliver:

- Orca/Raydium/Meteora research adapters.
- Concentrated-liquidity position simulator.
- Impermanent/divergence-loss accounting.
- Out-of-range inventory accounting.
- Rebalance cost model.
- Fee APR versus inventory loss decomposition.

Gate:

- Fee income remains positive after inventory loss, rebalancing costs, and adverse price movement
  in walk-forward tests.

### M11: Expansion

Only after M9 or M10:

- Add more allowlisted tokens.
- Add alternative execution venues.
- Add CEX comparison adapter.
- Add more strategy families.

## 20. Stop Conditions

Stop or redesign if:

- Strategy fails after realistic fees/slippage/priority fees.
- Performance depends on one short period.
- Walk-forward collapses.
- Cost sensitivity erases the edge.
- Drawdown exceeds budget.
- Quote-to-fill drift is too large.
- RPC/provider reliability is insufficient.
- Shadow mode cannot stay reconciled.
- Wallet/key handling is not airtight.
- Operator workload is not actually passive.

Do not compensate by adding leverage, memecoin exposure, faster trading, or more parameters.

## 21. Reuse Policy

### Reuse from existing projects

From `systematic-trader` / stock plan:

- Deterministic research discipline.
- Portfolio accounting and metrics concepts.
- JSON schema boundary.
- Paper/shadow-first workflow.
- Review cadence and explicit gates.

From `ibkr-bot`:

- Risk gate patterns.
- Structured logging.
- API/WebSocket/dashboard ideas.
- Audit database discipline.
- Paper-only enforcement mindset.

From `quant-sweep`:

- Deterministic engine lessons.
- Parallel sweep/canonical export discipline.
- Metrics and walk-forward ideas after verification.

### Use public crypto repos only for reference

Public crypto bot repositories may be mined for:

- API examples.
- Edge-case tests.
- Solana transaction-building examples.
- Jupiter integration examples.

They must not be adopted wholesale as:

- Strategy logic.
- Accounting logic.
- Risk logic.
- Wallet security model.
- Backtesting engine.

## 22. Source Notes for Current Assumptions

Current planning assumptions are based on:

- Solana fees documentation: base fee plus optional prioritization fee; base fee listed as 5,000
  lamports per signature.
- Solana RPC documentation: public endpoints are shared and not intended for production traffic,
  with possible rate-limit/block responses.
- Jupiter Swap V2 documentation: `/swap/v2/order` and `/swap/v2/execute` are the default route;
  older Ultra and legacy Metis assumptions should not be the foundation unless docs change.
- Jupiter transaction docs: quote/order/build/sign/submit/land/confirm are separate failure points,
  and transaction landing cannot be treated as guaranteed.
- Framework survey: Freqtrade, Hummingbot, Jesse, and NautilusTrader are useful for architecture
  lessons, dry-run discipline, connector boundaries, event lifecycles, and backtest/live parity, but
  should not be adopted wholesale as the strategy/accounting/risk foundation.
- Solana ecosystem data survey: Jupiter for live quotes/routes, Birdeye or GeckoTerminal/CoinGecko
  for early OHLCV, Helius for wallet/RPC/transaction monitoring, Pyth for independent price sanity,
  DexScreener for liquidity cross-checks, and Bitquery later for deeper historical DEX replay.
- Concentrated-liquidity docs from Orca/Raydium/Meteora: LP fee income requires explicit modeling
  of range exits, one-sided inventory, impermanent/divergence loss, and rebalance costs.
- Empirical literature on Solana failed transactions and bot/congestion behavior: low fees and high
  throughput do not eliminate failed transactions or congestion.

These sources should be rechecked before implementing M6 or later.

## 23. Executor Operating Contract

- Never commit or push. The operator commits.
- Stage explicit paths only, never `git add -A`.
- Do not edit secrets or local data.
- Do not create or load private keys in M0-M6.
- Do not add mainnet transaction submission before separate approval.
- Work in vertical milestones.
- Every packet includes full staged diff and gate outputs.
- Do not claim profitability from in-sample results.
- Do not use final holdout during ordinary development.

## 24. First Executor Task

Create the new repository and implement M0 only.

Required first packet:

```text
REPO=solana-crypto-trader | STEP=M0-BOOTSTRAP | LENS=architecture | BASE=<initial sha or EMPTY>
```

M0 must include:

- Repository layout.
- Rust workspace manifest.
- Formatting, linting, tests, and CI.
- `AGENTS.md`, `README.md`, `DECISIONS.md`, and `plans/master-plan.md`.
- Secret-safe config templates.
- Initial JSON schemas:
  - `run-result.schema.json`
  - `route-quote.schema.json`
  - `execution-event.schema.json`
  - `wallet-snapshot.schema.json`
- Minimal schema-validation tests.
- No private keys.
- No broker/exchange SDK dependency unless needed only for schema-free compilation tests.
- No strategy implementation.
- No mainnet submit path.

Stop for review after staging explicit M0 files.

## 25. Research Result: Public Strategy Folklore and Shared Bot Systems

### Purpose

This section records a separate strategy-research pass focused on human discussion and
user-submitted bot systems rather than API documentation. The goal was to identify what retail and
open-source crypto bot builders repeatedly try, what they say works, and what failure modes appear
often enough to shape the first build.

These findings are not proof of profitability. Reddit/forum posts are anecdotes, and public GitHub
strategies are usually educational, overfit, stale, or incomplete. Treat them as hypothesis
generators and regression-test inspiration, not as strategies to copy.

### Verification status

The following claims were cross-checked against primary pages or direct source files:

- Freqtrade's public strategy repository exists, is widely used, and explicitly says its strategies
  are educational, should be backtested, and should be dry-run before risking money.
- Jesse's research docs include a concrete EMA50/EMA100 plus ADX trend-filter example and emphasize
  rule-significance testing, random-signal comparison, and overfitting awareness.
- Hummingbot V2 controllers include concrete public patterns: dynamic PMM using MACD and NATR,
  Bollinger/DCA-style directional management, and MACD+Bollinger confirmation.
- NostalgiaForInfinity is a real public Freqtrade strategy family with complex multi-timeframe
  logic, broad pair recommendations, position-adjustment behavior, and extremely loose stoploss
  configuration. This is useful as a cautionary example, not a starting strategy.
- Reddit grid-bot discussions consistently frame grid bots as automated DCA / volatility harvesting
  rather than money printers, and repeatedly warn about trend exits, accumulated inventory, and
  underperformance versus holding during strong bull moves.
- Solana/SOL-USDC LP discussions consistently emphasize impermanent/divergence loss, range width,
  one-sided inventory, and the tradeoff between narrow high-fee ranges and wider safer ranges.

### Evidence quality scale

- High: direct source code or official project docs.
- Medium: mature project examples or repeated forum themes across communities.
- Low: single user anecdote, claimed performance, or unverified strategy result.

No claimed return percentage from public discussions is accepted as evidence. Only the structure of
the strategy and the repeated failure modes are imported.

### Reusable strategy patterns

#### 1. Low-frequency trend/risk-off

Evidence quality: high for existence, medium for usefulness.

Observed patterns:

- EMA cross with ADX trend filter.
- MACD trend confirmation.
- Supertrend-like trend filters.
- Breakout entries with volatility filters.
- Higher timeframes such as 4h or daily to survive fees and slippage.

Public-research implication:

This remains the first SOL/USDC strategy family. Trend/risk-off is less exciting than a bot that
prints many small wins, but it is the clearest way to reduce catastrophic SOL drawdowns while
keeping upside exposure.

Implementation notes:

- `trend_alloc_v1`: target allocation buckets of 0%, 25%, 50%, 75%, or 100% SOL.
- Candidate signals: EMA50/EMA100 plus ADX, MACD above signal plus long moving-average filter, or
  trailing-return breakout with volatility filter.
- Required bars: 4h and daily.
- Required validation: walk-forward, regime split, fee survival, parameter-neighborhood stability,
  and comparison to buy-and-hold SOL.

#### 2. Threshold rebalancing / volatility harvesting

Evidence quality: medium.

Observed patterns:

- Grid users often describe the useful part of grid trading as averaging in when price falls and
  averaging out when price rises.
- The more defensible version is not a dense grid. It is a target SOL/USDC allocation with
  threshold bands and explicit trend/risk-off gating.

Public-research implication:

This becomes the second core strategy family, paired with trend/risk-off rather than competing with
it.

Implementation notes:

- `threshold_rebalance_v1`: maintain a target SOL allocation and trade only when drift exceeds a
  configured band.
- Bands should widen during high volatility, bad Jupiter route quality, elevated priority fees, or
  poor transaction landing conditions.
- Test against static 50/50 SOL/USDC, DCA SOL, and buy-and-hold SOL.

#### 3. Squeeze / breakout confirmation

Evidence quality: medium.

Observed patterns:

- Public TradingView-style strategies often combine Bollinger/Keltner squeeze, ATR breakout,
  volume confirmation, RSI filters, and cooldowns.
- Hummingbot public controllers show combinations of Bollinger position plus MACD direction filters
  to reduce pure mean-reversion false positives.

Public-research implication:

This should be a research backlog item, not the MVP. It is a reasonable third strategy family once
the deterministic simulator can compare it fairly.

Implementation notes:

- `squeeze_breakout_v1`: Bollinger Band width or Keltner squeeze, ATR breakout threshold, volume
  confirmation, RSI overextension guard, daily trend filter, cooldown after exit.
- Do not allow same-bar signal/fill.
- Require lower turnover than scalping strategies.

#### 4. Bounded grid experiment

Evidence quality: medium for risks, low for profitability.

Observed patterns:

- Grid bots are popular and emotionally satisfying because they close frequent small winners.
- Repeated warnings: grids work in sideways markets, underperform in strong uptrends, accumulate
  falling inventory in downtrends, and can hide large tail risk.
- Suggested mitigations from discussions include ATR-sized grid spacing, constant spacing within a
  session, ADX or trend filters, max number of grid levels crossed, and reduced capital when trend
  strength rises.

Public-research implication:

Do not build a standalone grid bot as the first product. A bounded grid may become an experiment
after trend/risk-off and threshold rebalancing exist.

Implementation notes:

- `bounded_grid_experiment`: only active when regime classifier says "range/chop."
- Grid spacing based on ATR.
- Max inventory, max levels crossed, max daily loss, and forced de-risking to USDC.
- Automatically disable when ADX/trend strength exceeds threshold.
- Must beat threshold rebalancing after fees, slippage, and priority fees to remain in the backlog.

#### 5. DCA / safety-order systems

Evidence quality: high for existence, low for suitability.

Observed patterns:

- Freqtrade and public strategies often implement rebuys/safety orders when a position is down and
  a fresh signal appears.
- Complex strategies such as NostalgiaForInfinity can include broad pair universes, position
  adjustment, grinding, and very loose stoploss settings.

Public-research implication:

Do not import DCA/safety-order logic into the MVP. The pattern can improve average entry price, but
it also hides exposure growth and can turn a bad signal into a larger bad signal.

Implementation notes:

- If ever tested, it must be a separate experiment with max rebuy count, max total exposure, and
  explicit comparison to simply holding the target allocation.
- No martingale sizing.
- No "average down until it works" behavior.

#### 6. Concentrated-liquidity range management

Evidence quality: high for mechanism, medium for operational lessons, low for profitability claims.

Observed patterns:

- LP/range users describe the core tradeoff clearly: narrow ranges can earn more fees but leave the
  range more often; wider ranges are safer but earn less.
- SOL/USDC LP exposure naturally sells SOL into strength and buys SOL into weakness, eventually
  becoming one-sided if price moves far enough.
- Impermanent/divergence loss is repeatedly identified as the core risk.

Public-research implication:

LP range management is still the first "income-like" expansion lane, but it must wait until the
spot simulator and shadow quote logger are mature.

Implementation notes:

- `cl_lp_research_v1`: simulate Orca/Raydium/Meteora style ranges, fees, inventory conversion,
  out-of-range time, and rebalancing costs.
- Track fee income separately from inventory P&L.
- Compare against static SOL/USDC holding and threshold rebalancing.
- Never report fee APR without net P&L after divergence loss.

#### 7. Market making / XEMM / arbitrage / MEV

Evidence quality: high for existence, low for suitability to this project.

Observed patterns:

- Hummingbot and related discussions show real patterns: pure market making, dynamic spreads,
  cross-exchange market making, and hedged maker/taker flows.
- Hummingbot public controllers include PMM Simple, PMM Dynamic, inventory skew, Bollinger/DCA
  controllers, grid executors, XEMM executors, and arbitrage executors.
- PMM Dynamic-style logic shifts the reference price with MACD and widens/tightens spreads with
  volatility such as NATR.
- Solana/Jupiter arbitrage repos exist and often scan routes or react to large trades.
- MEV/backrun/searcher systems are infrastructure competitions involving latency, priority fees,
  bundles, route access, and capital.

Public-research implication:

Do not start here for the Solana/Jupiter spot bot. Hummingbot PMM patterns are useful references,
but pure market making assumes the ability to place and manage resting maker orders, inventory, and
often multiple levels on a venue. Jupiter Swap is primarily a route/swap execution surface, not a
native maker-order venue. PMM ideas may later inform CEX adapters, OpenBook-style venues, or LP/range
models, but they do not replace the SOL/USDC allocation MVP.

Implementation notes:

- Keep as a long-term research category only.
- Any future XEMM or arbitrage work must have exchange inventory reconciliation, hedge-fail
  handling, latency measurement, and fee/priority-fee budgets before strategy code.
- If a PMM lane is ever tested, start with `pmm_simple_inventory_skew` as a baseline, then
  `pmm_dynamic_vol_spread` with hard caps on spread, reference-price shift, inventory, stale data,
  and candle warmup/NaN handling.
- Treat Hummingbot DCA/Grid controllers as idea references only; they need quote-budget caps,
  protective exits, max concurrent exposure, and deterministic accounting before use here.

### Strategy backlog from this research

The strategy backlog is now:

1. `trend_alloc_v1`
   - SOL/USDC, 4h/daily.
   - Regime/trend filter.
   - Target allocation buckets.
   - First production research target.

2. `threshold_rebalance_v1`
   - SOL/USDC allocation drift bands.
   - Volatility- and execution-aware band widening.
   - Second production research target.

3. `squeeze_breakout_v1`
   - Bollinger/Keltner squeeze, ATR breakout, volume confirmation, RSI guard, cooldown.
   - Research after the first two families.

4. `bounded_grid_experiment`
   - ATR grid spacing, ADX/range gate, max levels, max inventory, forced de-risk.
   - Experimental only.

5. `cl_lp_research_v1`
   - Concentrated liquidity simulator.
   - Fee APR decomposed into net P&L after divergence loss.
   - Separate post-MVP lane.

6. `arb_xemm_research_later`
   - Not part of the first repo build.
   - Keep as a notes-only category until infrastructure, capital, and operational requirements are
     justified.

7. `pmm_research_later`
   - Not part of the first Solana/Jupiter MVP.
   - Study Hummingbot PMM Simple, inventory skew, PMM Dynamic, grid executor, and DCA executor as
     later references.
   - Requires a venue capable of maker-style quoting or a CLMM/LP translation layer.

### Required validation gates imported from public-strategy research

Any public-inspired strategy must pass:

- Backtest with realistic Jupiter execution costs, not zero-cost fills.
- Next-bar or quote-time execution, never signal-price execution.
- Fee/slippage/priority-fee doubling scenario.
- Out-of-sample period.
- Walk-forward period.
- Parameter-neighborhood stability.
- Trial count recorded.
- Baseline comparison against hold USDC, buy-and-hold SOL, static 50/50 SOL/USDC, and DCA SOL.
- Trade frequency cap or turnover budget.
- Shadow-mode comparison against live Jupiter quotes before canary.

The strategy is rejected if its edge disappears after costs, depends on one period, depends on one
parameter point, or requires increasing frequency/leverage to hit the desired return target.

### Plan changes caused by this research

- M3 must include both `trend_alloc_v1` and `threshold_rebalance_v1`, not only one trend strategy.
- M4 sweep/walk-forward must report turnover and fee sensitivity as first-class outputs.
- M6 Jupiter shadow collector is not optional; it is the bridge between backtest assumptions and
  live execution reality.
- `bounded_grid_experiment` is explicitly deferred until trend/range classification exists.
- `cl_lp_research_v1` remains M10 and cannot be promoted until the engine can model net LP P&L, not
  just fee APR.
- Public repos may provide test fixtures and strategy sketches, but no public strategy is accepted
  without being rewritten into this system's deterministic accounting model.

## 26. R1 Strategy Mining Deliverable

### Goal

R1 extends the public-strategy folklore pass into an implementation-oriented shortlist. The goal is
not to find a public strategy to copy. The goal is to define a small set of SOL/USDC hypotheses
that can be tested honestly in this repo's deterministic simulator and later checked against live
Jupiter quote behavior.

The core conclusion is regime matching:

- Trend systems are for directional SOL regimes.
- Rebalancing/grid-like systems are for range or chop regimes.
- Mean-reversion systems need trend filters.
- Breakout systems need volatility/volume confirmation.
- LP systems need pool-state accounting, not just candle data.

No single indicator family should become the product. The product is the ability to classify market
state, choose a constrained strategy family, and prove results after costs.

### R1 ranked shortlist

#### 1. `trend_alloc_v1`

Status: first implementation candidate.

Purpose:

- Keep SOL exposure when the market is favorable.
- Hold USDC or reduced SOL allocation during unfavorable regimes.
- Reduce catastrophic drawdowns before trying more income-like systems.

Public inspirations:

- Jesse Donchian: long-only breakout above prior Donchian upper band, filtered by long moving
  average, exit on lower band or trend failure.
- Jesse TurtleRules: Donchian-style breakout with ATR sizing and exits.
- Freqtrade Supertrend: multiple ATR-derived supertrend conditions agreeing on direction.
- Freqtrade TrendRider-style logic: EMA regime, RSI/ADX/MACD/OBV/volume, higher-timeframe context.

Initial version:

- Timeframes: 4h and daily.
- Allocation buckets: 0%, 25%, 50%, 75%, 100% SOL.
- Candidate filters:
  - close above long moving average;
  - ADX above threshold for trend confirmation;
  - Donchian breakout or trailing-return confirmation;
  - optional BTC or broad-crypto context later, not in MVP.
- Exits:
  - trend filter failure;
  - volatility stop;
  - max drawdown / risk-off halt.

Why it might work:

- SOL has large directional regimes.
- Avoiding severe downtrends can matter more than catching every rally.
- Low turnover keeps Jupiter execution costs less important.

Why it might fail:

- Lag after sharp reversals.
- Whipsaw during chop.
- Underperformance versus buy-and-hold during strong bull markets.
- Overfit parameter thresholds.

Acceptance gates:

- Beats hold USDC on return.
- Improves drawdown-adjusted result versus buy-and-hold SOL.
- Survives doubled execution cost assumptions.
- Shows parameter-neighborhood stability.
- Does not depend on one bull-market segment.

#### 2. `threshold_rebalance_v1`

Status: first implementation candidate, paired with `trend_alloc_v1`.

Purpose:

- Convert grid-bot intuition into a safer allocation rule.
- Buy SOL only after meaningful allocation drift.
- Sell SOL only after meaningful allocation drift.
- Avoid dense grids and martingale behavior.

Public inspirations:

- Crypto grid-bot discussions where the useful part is buying lower and selling higher.
- Crypto portfolio rebalancers with drift thresholds.
- Public index/rebalancer repos that rebalance only when allocation drift exceeds a threshold.

Initial version:

- Target allocation from `trend_alloc_v1`, for example 25%, 50%, or 75% SOL.
- Rebalance only when current allocation drifts by more than a configured band.
- Band width increases when:
  - volatility is high;
  - Jupiter price impact is high;
  - priority fees are elevated;
  - quote/route quality is poor;
  - transaction landing conditions are poor.

Why it might work:

- SOL volatility creates repeated allocation drift.
- Thresholding avoids overtrading.
- It is easier to reason about than a dense grid.

Why it might fail:

- Persistent SOL trends can make rebalancing lag buy-and-hold.
- A bear trend can keep buying declining SOL unless risk-off gating works.
- Too-tight bands turn it into a fee machine.

Acceptance gates:

- Beats static 50/50 SOL/USDC after costs.
- Does not materially increase max drawdown versus static 50/50.
- Has bounded turnover.
- Stops adding SOL during risk-off regimes.

#### 3. `breakout_confirm_v1`

Status: second-wave research candidate after the allocation engine exists.

Purpose:

- Catch high-conviction SOL expansion moves after compression.
- Avoid constant low-quality trading.

Public inspirations:

- Jesse Dual Thrust: range breakout using recent high/low range and ATR stop.
- Jesse SimpleBollinger: upper-band continuation filtered by a trend condition.
- Public TradingView/Pine patterns using Bollinger/Keltner squeeze, ATR breakout, volume spike,
  RSI guard, and cooldown.
- Donchian breakout with ADX and volume filters.

Initial version:

- Timeframes: 1h and 4h.
- Compression detector: Bollinger Band width, Keltner squeeze, or rolling volatility percentile.
- Trigger: close beyond ATR-adjusted breakout level.
- Filters:
  - volume expansion;
  - daily trend agreement;
  - RSI overextension guard;
  - cooldown after failed breakout.

Why it might work:

- SOL has violent expansion moves.
- Waiting for compression plus confirmation may reduce churn.

Why it might fail:

- Fakeouts.
- Late entries.
- Higher turnover and worse execution sensitivity than daily trend allocation.

Acceptance gates:

- Must beat `trend_alloc_v1` in at least one clearly defined regime without worse full-period
  drawdown.
- Must keep trade frequency below a configured cap.
- Must remain profitable after doubled slippage and priority fees.

#### 4. `pullback_meanrev_v1`

Status: research candidate, not MVP.

Purpose:

- Test whether SOL exhaustion dips inside an uptrend can be bought safely.

Public inspirations:

- Freqtrade Strategy003: RSI/Fisher/MFI/SMA/Stochastic/SAR style exhaustion and exit logic.
- Freqtrade Bandtastic: Bollinger lower-band entries, RSI/MFI/EMA guards, upper-band/trailing exits.
- Public BB/RSI/ADX systems where mean reversion is only allowed in non-trending or favorable
  regimes.

Initial version:

- Only active when the higher-timeframe trend is neutral-to-positive.
- Entry on deep Bollinger or RSI/Fisher exhaustion.
- Exit on mean reversion to mid/upper band or trend failure.
- No averaging down unless the separate DCA experiment is enabled.

Why it might work:

- SOL often has fast liquidation/panic wicks followed by reflexive rebounds.

Why it might fail:

- Falling-knife entries during regime breaks.
- Fee/slippage sensitivity on lower timeframes.
- Overfit oscillator thresholds.

Acceptance gates:

- Must not increase max drawdown versus `trend_alloc_v1`.
- Must outperform simply waiting for the trend allocator to re-enter.
- Must pass bear-market stress tests.

#### 5. `bounded_grid_experiment`

Status: experimental only.

Purpose:

- Test whether grid-like behavior adds anything beyond threshold rebalancing.

Public inspirations:

- Spot grid repos with fixed price levels.
- Human discussion around ATR-sized grids, ADX/range filters, max levels, and regime exits.

Initial version:

- Only active when market state is classified as range/chop.
- Grid spacing based on ATR or realized volatility.
- Fixed capital allocation per grid session.
- Max levels crossed.
- Max inventory.
- Hard risk-off exit.
- No martingale.

Why it might work:

- SOL can spend long periods oscillating inside broad ranges.

Why it might fail:

- Trend breaks turn the grid into inventory accumulation.
- Strong bull trends can leave the bot underexposed.
- Too many small fills may underperform simple threshold rebalancing.

Acceptance gates:

- Must beat `threshold_rebalance_v1` after costs.
- Must not require hidden leverage or unbounded inventory.
- Must disable cleanly in trend regimes.

#### 6. `dca_safety_order_experiment`

Status: high-risk experiment, not MVP.

Purpose:

- Test whether capped safety orders improve pullback systems without hiding tail risk.

Public inspirations:

- Freqtrade position-adjustment / safety-order callbacks.
- NFI-style grind/DCA behavior.

Initial version:

- Max rebuy count.
- Max total SOL exposure.
- Max total capital committed.
- No martingale size escalation.
- Only allowed when higher-timeframe risk-off filter is false.

Why it might work:

- It can improve average entry during normal pullbacks.

Why it might fail:

- It can transform one bad signal into a larger bad signal.
- It can lock capital during a bear trend.
- It can look stable until a deep drawdown event.

Acceptance gates:

- Must report exposure growth separately from strategy P&L.
- Must beat the same system without DCA.
- Must survive deep 2021-2022 and 2025-style crypto stress scenarios when data is available.

#### 7. `cl_lp_research_v1`

Status: later read-only research, not live automation.

Purpose:

- Evaluate whether SOL/USDC concentrated liquidity can provide net income after divergence loss,
  out-of-range time, and rebalance costs.

Public/protocol inspirations:

- Orca Whirlpools: custom ranges, ticks, fixed/adaptive fees, clear CLMM model.
- Raydium CLMM: Uniswap-v3-style positions, fee tiers, tick spacing, rewards, dynamic fees.
- Meteora DLMM: bin-based liquidity, dynamic fees, spot/curve/bid-ask allocation modes.
- Kamino vaults: automated liquidity management benchmark, not something to blindly trust.

Initial research strategies:

- Wide centered range, for example +/-20% to +/-40% around spot.
- Layered ranges, for example a tight active range plus a wider reserve range.
- Volatility-width model: widen ranges as realized volatility rises.
- Directional skew: bias range placement according to the trend allocator.
- Kamino vault benchmark: compare DIY simulation against vault share performance.

Why it might work:

- LP fees are a real income source.
- SOL/USDC volume can be substantial.
- Solana costs allow more active management than high-fee chains.

Why it might fail:

- Impermanent/divergence loss can exceed fee income.
- Out-of-range positions earn no fees.
- Rebalance costs and timing can eat apparent APR.
- Pool security and smart-contract risk are non-trivial.

Acceptance gates:

- Must decompose fee income, inventory P&L, divergence loss, and rebalance costs.
- Must compare against hold SOL, hold USDC, static 50/50, and threshold rebalancing.
- Must not report APR without net P&L.
- No wallet signer or LP transaction path until read-only research is proven.

### Rejected or notes-only directions

#### Arbitrage / XEMM / MEV

Reason:

- Real but infrastructure-heavy.
- Requires latency, capital, inventory on multiple venues, priority-fee/Jito strategy, and
  adversarial execution assumptions.
- Not aligned with low-maintenance passive income for a solo operator.

Disposition:

- Notes-only until the main SOL/USDC allocation system is proven.

#### Copy trading / wallet following / sniper bots

Reason:

- Adverse selection and edge decay.
- Public wallets often become crowded after edge is visible.
- Private-key and execution-risk patterns in public repos are often unsafe.
- Not aligned with deterministic research-first development.

Disposition:

- Reject for this project.

#### Public strategy clone

Reason:

- Public repos are useful as idea mines, but profitability claims are unreliable.
- Many examples are educational, stale, exchange-specific, or overfit.

Disposition:

- Rewrite ideas into the deterministic simulator; never import a public strategy as production
  logic without independent validation.

### R1 implementation impact

M3 should be renamed from "Strategy MVP" to "Strategy Lab MVP" and should include:

- `trend_alloc_v1`
- `threshold_rebalance_v1`
- baseline runner
- regime classifier scaffold
- strategy comparison report

M4 should include:

- parameter sweeps for the two MVP strategies;
- walk-forward windows;
- strategy-family comparison;
- turnover and fee-sensitivity reporting;
- rejection report for failed candidate families.

M5 should be a strategy-selection gate:

- choose one candidate for mainnet shadow;
- or explicitly reject all candidates and return to research;
- no execution work should start merely because the software exists.

M6 should stay focused on Jupiter shadow quotes:

- collect real route/price-impact/fee/latency data;
- compare against assumptions from backtests;
- update execution-cost model before canary.

LP work should remain M10 read-only research unless the operator explicitly chooses to make LP the
primary project instead of SOL/USDC spot allocation.
