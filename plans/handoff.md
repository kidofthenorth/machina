# Handoff — for a new chat continuing machina

You are picking up an in-progress build. This is the single entry point. Read the four pointer files
below, confirm the gates are green, then continue in milestone order. **Do not commit or push — the
operator commits.**

---

## 30-second orientation

**machina** (codename `solana-crypto-trader`) is a paper-first, Solana-focused crypto **trading
research platform** in Rust. It exists to discover whether a strategy has a robust edge after costs,
latency, slippage, and operational failure. It does **not** promise income, and **live trading is
never automatic**.

**The one rule that cannot be broken:** money-moving capability is gated by milestone **and** by
explicit human approval. No key loading, signing, or transaction submission exists — or may be added
— before the research phase (M0–M5) is complete and separately approved. Building the software never
authorizes trading. Full invariants: [docs/invariants.md](../docs/invariants.md).

## Read these first (source of truth)

1. [plans/current-state.md](current-state.md) — live per-milestone status, gates, blockers, next command.
2. [plans/master-plan.md](master-plan.md) — authoritative M0–M11 roadmap & architecture.
3. [plans/task-queue.md](task-queue.md) — actionable status table (TODO/DOING/DONE/DEFERRED/BLOCKED).
4. [docs/architecture-index.md](../docs/architecture-index.md) + [docs/invariants.md](../docs/invariants.md) — module map + hard invariants.

Audit + staged-diff record: [plans/review-packet.md](review-packet.md). Chronological log:
[plans/worklog.md](worklog.md). Open operator questions: [plans/questions.md](questions.md).

## What's accomplished (M0 → M2 complete; M3 scaffolds)

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
- **Quality.** A 6-lens adversarial verification workflow ran → **PASS, no blockers**; all gate-relevant
  findings fixed (gap detection, disallowed-token test, DCA baseline, regime scaffold, doc accuracy).

## Verified state (re-confirm on a fresh checkout)

```bash
cargo fmt --all --check                                  # clean
cargo clippy --all-targets --all-features -- -D warnings # clean
cargo test --workspace --all-features                    # 85 passed, 0 failed
cargo run -p cli -- demo                                 # deterministic, schema-valid RunResult
```

Repo state: **no commits yet** (BASE=EMPTY). 64 files are **staged but not committed** — the operator
commits. `.claude/` is intentionally not staged.

## What's next

- **NEXT milestone — M4 (sweep / walk-forward).** Create a `sweep` crate: parameter sweeps, parallel
  execution with **canonical output identical to sequential** (determinism is a hard requirement),
  walk-forward windows, turnover/fee-sensitivity reporting, rejection report. Start with `/plan`.
- **M5** research-decision gate; **M6** Jupiter shadow (re-verify plan §22 sources first — no signing).
- **BLOCKED — M8/M9** devnet/canary signing + submission: **requires separate explicit human approval.
  Do not start.**
- Real OHLCV ingestion is deferred pending an operator data-source decision (questions.md Q3); M1–M3
  use tiny synthetic checked-in fixtures only.

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

---

## Seed prompt for the new chat (paste this)

> You are the EXECUTOR continuing the **machina** (`solana-crypto-trader`) repo. Read
> `plans/handoff.md`, then `plans/current-state.md`, `plans/master-plan.md`, `plans/task-queue.md`,
> and `docs/invariants.md`. M0–M2 are complete and M3 scaffolds are in place; gates are green
> (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` = 85 passing). The repo has no
> commits yet and 64 files are staged — **do not commit or push; the operator commits.** Continue in
> milestone order: begin **M4 (sweep / walk-forward)** by writing a short plan and checking it against
> the real code before changing anything. Honor the operating contract in `plans/handoff.md`: no keys,
> no signing, no submission, no real-money path; determinism is a hard requirement; stage explicit
> paths only. Update `plans/current-state.md` and `plans/worklog.md` as part of "done."
