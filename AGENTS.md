## Project
**machina** is a paper-first, Solana-focused crypto **trading research platform** — a Rust system to
research, simulate, and (much later, behind explicit gates) shadow- and live-trade on-chain spot
strategies. It serves a solo operator who wants low-touch operation *after* a strategy is proven. It
is a machine for discovering whether a strategy has a robust edge after costs, latency, slippage, and
operational failure — **it does not promise passive income, and live trading is never automatic.**

The one thing not to get wrong: **money-moving capability is gated by milestone and by explicit human
approval.** No key loading, transaction signing, or submission exists — or may be added — before the
research phase (M0–M5) is complete and separately approved. Building the software never authorizes
trading.

**Status: M0 done; M1–M2 complete with M3 strategy scaffolds.** The authoritative roadmap is
`plans/master-plan.md` (the full M0–M11 plan; `solana-crypto-trader-plan.md` at the repo root is the
original, byte-identical source). Live build state is `plans/current-state.md`. A Cargo workspace of
7 research crates builds and tests green; there are **zero execution paths** (no keys/signing/submit
/RPC). Next gated milestones (M4 sweep → M5 research decision → M6 Jupiter shadow) require their own
approval; M8/M9 signing+submission require **separate explicit human approval**.

## Commands
These run today (established in M0; CI runs the same from a clean checkout):
- Build: `cargo build`
- Test: `cargo test` *(deterministic and repeatable — 85 tests)*
- Run / dev: `cargo run -p cli -- demo` *(deterministic research demo; no network, no keys)*
- Lint / typecheck: `cargo clippy --all-targets --all-features -- -D warnings` + `cargo fmt --check`

## Map
A Cargo workspace. **Existing** crates (`crates/`): `research-core`, `market-data`, `portfolio`,
`metrics`, `strategies`, `results`, `cli`. **Deferred** (named in the plan, not yet created):
`sweep` (M4), `route-model` (M6), `solana-execution` (M6/M8), `wallet-state` (M7), `risk` (M6) —
built in **milestone order**, not all at once.
- `docs/architecture-index.md` — the module map (read first); `docs/invariants.md` — hard invariants.
- `plans/master-plan.md` — the authoritative roadmap (M0–M11); `plans/current-state.md` — live state.
- `config/` (`*.example.toml` templates only), `schemas/` (JSON Schema contracts), `crates/`.

Per-module `CLAUDE.md` context is added as each crate grows non-obvious; the small crates are
self-documenting via their `lib.rs` docs for now.

## How to work here (the loop)
Any agent — Claude, Codex, Cursor, or a human — works in the same rhythm:
- **Plan before code.** Write a short plan and check it against the real files before changing anything.
- **Read before you write.** Inspect the actual code the change touches; don't assume from names.
- **Small diffs.** One reviewable change at a time; the plan file carries the thread across resets.
- **Refresh context in the same diff** that changes behavior — updating the notes is part of "done."
- **Review with fresh eyes,** one lens at a time: correctness, then security, then tests.
- **Build in milestone order.** A milestone advances only when its gate criteria are met; don't start
  execution work just because code exists.

In Claude Code this loop is wired up as `/plan`, `/review`, and `/context-health`. Other tools have
no slash commands — follow the same steps by hand. The discipline is portable; the commands aren't.

## Conventions
- **Determinism is a hard requirement.** Same input data → identical orders, fills, balances, equity
  curve, and metrics. Parallel sweeps must produce canonical output identical to sequential runs
  (ordering independent of thread scheduling).
- **Fixed-point arithmetic** (decimal/integer) for all token balances and quote values — never
  floating point for money.
- **Strategies produce intent only** (target weights); they must not own keys or call RPC.
- **Accounting uses realized on-chain balance deltas**, not just a quoted `outAmount`.
- Data hygiene: UTC internally; bars sorted and unique; OHLC invariants enforced; volume ≥ 0; token
  decimals verified from mint metadata; missing data blocks a run unless explicitly configured as a
  scenario (no silent forward-fill).
- Strategy naming: `<family>_v<n>` (e.g. `trend_alloc_v1`, `threshold_rebalance_v1`).
- Secrets only via `*.example.toml` templates; all real keys, RPC credentials, and generated
  data/exports are gitignored.

## Never do (without explicit approval)
- **Never commit private keys, seed phrases, or wallet files** — in any form, ever.
- **No key loading, transaction signing, or transaction submission before the gated milestones**
  (M8 devnet / M9 mainnet). Setting a mainnet RPC URL alone must never enable signing or submission.
- **No live mainnet trading** without separate, explicit human approval (M9).
- **Never trade a token absent from the allowlist.**
- **Risk controls must never block *reducing* risk** — converting back to USDC, cancelling/abandoning
  stale unsigned transactions, or lowering exposure. They only gate new or increasing exposure.
- Don't skip milestone gates or weaken determinism/accounting guarantees.
- Don't edit schemas-as-contracts, generated/exported data, or vendored code without cause.
