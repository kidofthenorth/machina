## Project
**machina** is a paper-first, Solana-focused crypto **trading research platform** built toward one
goal: **genuine autonomous passive income** — low-touch, hands-off on-chain spot trading for a solo
operator, once a strategy has earned that trust. Getting there means first proving a strategy has a
robust edge after costs, latency, slippage, and operational failure; **no strategy is guaranteed to
clear that bar, and live trading starts only once its own milestone gate is explicitly approved.**

The one thing not to get wrong: **money-moving capability is gated by milestone and by explicit human
approval.** No key loading, transaction signing, or submission exists — or may be added — before the
research phase (M0–M5) is complete and separately approved. Building the software never authorizes
trading.

**Status: M0–M4 complete (M4 gate declared 2026-07-09); M5 research decision CLOSED 2026-07-12
(Branch A reject-all — holdout unread, seal intact); the M-HF research track is ACTIVE (cards
`M-HF-*` in `plans/task-queue.md`; next open card and seed prompt in `plans/handoff.md`).**
The authoritative roadmap is `plans/master-plan.md` (the full M0–M11 plan;
`solana-crypto-trader-plan.md` at the repo root is kept byte-identical to it — edit both or neither,
per Q6). Live build
state is `plans/current-state.md`. A Cargo workspace of 8 research crates builds and tests green;
there are **zero execution paths** (no keys/signing/submit/RPC). Next gated milestones (M4 gate →
M5 research decision → M6 Jupiter shadow) require their own approval; M8/M9 signing+submission
require **separate explicit human approval**.

## Commands
These run today (established in M0; CI runs the same from a clean checkout):
- Build: `cargo build`
- Test: `cargo test` *(deterministic and repeatable — 441 passed / 0 failed / 1 ignored as of
  2026-07-19, `scripts/gate.sh` run at `fcd75e4`)*
- Run / dev: `cargo run -p cli -- demo` *(deterministic research demo; no network, no keys)*
- Sweep: cargo run -p cli -- sweep [--threads N] [--out PATH]; verify determinism: cargo run -p cli -- sweep-verify *(research only; no network, no keys)*
- Lint / typecheck: `cargo clippy --all-targets --all-features -- -D warnings` + `cargo fmt --check`

## Cadence
`project-management: off` — this repo runs the **FOREMAN card-loop** (`docs/repo-kit/FOREMAN.md`),
which replaces the global Repo bootstrap cadence entirely.
- Entry point for any fresh session: `plans/handoff.md`. Cards: `plans/task-queue.md` (the only
  inbox — mid-session work becomes a card stub, never a scope expansion). Status truth:
  `plans/current-state.md`. Append-only log: `plans/worklog.md`.
- **GATE = `bash scripts/gate.sh`** — run unprompted before claiming green, flipping a card, or
  writing any handoff/worklog entry; paste its printed counts as evidence, never transcribe counts
  from memory. Handoff baselines are generated (`bash scripts/handoff-baselines.sh`), never typed.
- Every run that touches this repo ends with a dated worklog line (what changed, gate evidence,
  what's staged) — unprompted; a run without its worklog line is unfinished.

## Map
A Cargo workspace. **Existing** crates (`crates/`): `research-core`, `market-data`, `portfolio`,
`metrics`, `strategies`, `results`, `sweep`, `cli`. **Deferred** (named in the plan, not yet
created): `route-model` (M6), `solana-execution` (M6/M8), `wallet-state` (M7), `risk` (M6) —
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
- **When a piece of work is complete** (gates green, plan files updated): `git add` the explicit
  paths belonging to that work — never `git add -A`, never `.claude/`, never another session's
  in-flight files — and output a suggested commit message for the operator. Agents never run
  `git commit` or `git push` (the operator commits), and suggested messages carry no
  Co-Authored-By or AI-attribution trailers.

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
