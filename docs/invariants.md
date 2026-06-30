# System invariants (non-negotiable)

These hold at every milestone. A change that violates any of these is wrong by definition — not a
trade-off. CI and tests encode as many of them as is mechanically possible; the rest are review
gates. If you are an agent reading this: do not weaken these to make a task easier. Escalate instead
(`plans/questions.md`).

## 1. No signing before an approved phase
No private key loading, no transaction signing, and no transaction submission may exist anywhere in
the workspace before the gated milestones **M8 (devnet)** / **M9 (mainnet)** — and even then only
behind separate, explicit human approval. **Building the software never authorizes trading.**
- *Enforced now by:* there is no Solana SDK, Jupiter client, HTTP client, or wallet/keypair crate in
  any `Cargo.toml`; the CI job `no-execution-deps` fails the build if one is added.

## 2. No mainnet submit path
There is no code path that can submit a transaction to Solana mainnet. Setting a mainnet RPC URL
alone must never enable signing or submission. The future submit path (M8+) is feature-gated and
disabled by default, and mainnet canary requires separate approval (M9).
- *Enforced now by:* no RPC/submit code exists at all (strongest form).

## 3. Deterministic research
Same input data → identical orders, fills, balances, equity curve, and metrics, byte-for-byte.
Parallel sweeps (M4) must produce output identical to sequential runs; canonical ordering is
independent of thread scheduling.
- *Enforced by:* fixed-point money (`rust_decimal::Decimal`), ordered collections in canonical
  output, no wall-clock/RNG in canonical runs, and a repeated-run determinism test.

## 4. Explicit token allowlist
No strategy may trade a token absent from the active allowlist. The allowlist version is embedded in
every run result. The allowlist is a first-class, checked-in research artifact.
- *Enforced by:* allowlist loader + `allowlisted`/unknown-token checks; `allowlist_version` is a
  required field of `RunResult`.

## 5. No strategy uses future data
A strategy sees only completed bars `[0..=t]`; its target is executed at bar `t+1`'s open
(next-bar execution). The final bar's signal therefore cannot open a position.
- *Enforced by:* the simulator's execution loop and the "final-bar signal cannot open a position"
  test.

## 6. No strategy calls RPC / Jupiter directly
Strategies produce **portfolio intent (target weights) only**. They do not own keys, call RPC, fetch
quotes, or mutate execution state. Routing/quoting/execution live behind their own (future) crates.
- *Enforced by:* the `Strategy` trait takes market history and returns target weights; the
  `strategies` crate depends on no network/SDK crate.

## 7. Accounting by token balances and fixed-point amounts
Accounting uses realized on-chain balance deltas (in research, the simulator's exact balance deltas),
never just a quoted `outAmount`. All balances/prices/fees/equity are fixed-point decimals — never
floating point. Token outputs are quantized to token decimals.
- *Enforced by:* `Decimal` money types; the fill model returns explicit balance deltas; tests assert
  buy/sell debit/credit and that fees reduce equity.

## 8. Shadow mode before canary
The path to capital is strictly ordered: backtest → shadow (read-only, no signing) → devnet →
tiny-capital canary → larger, each behind objective gates and (from canary) explicit approval. No
execution work begins merely because the software exists.

## 9. Risk controls never block reducing risk
Risk gates may reject new or increasing exposure. They must never block reducing exposure, converting
back to USDC, or cancelling/abandoning stale unsigned transactions.

## 10. Secrets never enter the repo
No private keys, seed phrases, wallet files, or RPC credentials, in any form, ever. Only
`*.example.toml` templates are tracked; real config and all secrets are gitignored.
- *Enforced by:* `.gitignore` (keys/wallets/env/real-config) and operator-only, explicit-path commits.

## 11. No profitability claims
In-sample backtest results are never presented as evidence a strategy works. Advancement requires the
full validation battery (costs, doubled costs, walk-forward, holdout, baselines) — out of scope until
M4–M5.
