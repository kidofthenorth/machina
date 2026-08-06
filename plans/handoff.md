# Handoff — machina fresh-chat entry point (FOREMAN card loop) · AWC-ready

**Role/scope.** Seeds either seat. **PLANNER** (foreman, long-lived): audits reality, writes
cards, verifies every executor report, stages, commits verified slices (authority below), orders
reviews — never executes cards. **EXECUTOR** (one fresh chat per card): exactly one card, then
stop — no commit messages, no next-session prompts (Role rule, task-queue.md). Queue:
`plans/task-queue.md` · status: `plans/current-state.md` · log: `plans/worklog.md` (append-only)
· decisions: `DECISIONS.md` · roadmap: `plans/master-plan.md` (lockstep pair — never edit).

## Pre-close baseline (2026-08-06, planner-run gate + baselines — committed after writing, so HEAD has moved past this SHA; step 0 is `git log -1`, trust that)
- HEAD `1cc49e2` pre-handoff, tree clean (device-local `machina-gnhf-worktrees/` now gitignored).
- Gate exit 0: **489 passed / 0 failed / 1 ignored** · demo shasum
  `ae064f79242f823ffd8f55bf9104e3e1b45d425a` (×2 identical) · sweep shasum
  `f6e1ab5132754d69c3a2be23fc549df36c07909f` · sweep-verify OK (18990 bytes, byte-identical
  across 1/2/8 threads + repeat) · no-exec-deps OK.

## Mission state + exact next step
- M0–M5 DONE (M5 reject-all 2026-07-12; holdout unread, seal intact). **M-HF track ACTIVE.**
  C1–C7, C8.1–C8.6b, **C8.7a–f DONE** (C8.7f `run_hf_sweep` capstone landed at `12babe1`;
  REVIEW-C8.4-SEAL ruled PASS 2026-07-21). Gate count 489 matches the post-C8.7f worklog claim.
- **Exact next step: card M-HF-C8.7g** (task-queue.md §"M-HF-C8.7 planner reconciliation
  (2026-07-20)") — the row-C8 gate battery: NEW test-only file
  `crates/sweep/tests/hf_sweep_determinism.rs`, no `src/` edits; gate must land **0 failed /
  2 ignored** (ignored 1→2 is sanctioned there), BOTH shasums unchanged. Read the section's
  common rules + decisions D-a…D-h before casting.
- Then: **post-C8.7 adversarial review** — the operator's AWC cast (2026-08-06) IS the
  go-ahead the queue requires — then the row-C8 gate declaration, then the C8.8/C9/C10 wave.
- **Stale-doc note:** AGENTS.md's banner ("M-HF PARKED at C8.7b") and any older handoff/state
  claims predate C8.7c–f landing — this file + task-queue.md + worklog are current truth.

## AWC — 24h autonomous workflow cadence (operator-cast 2026-08-06; device B)
- **Cast:** one PLANNER (strongest model, high effort), 24h budget from cast.
- **Step 0:** `git pull --ff-only` (fresh clone: toolchain per `rust-toolchain.toml` first) ·
  `git log -1` + `git status --porcelain` · `bash scripts/gate.sh` — needs `shasum`; the gate
  fails loud if it's missing (Windows: run under WSL/Git-Bash with perl). Reconcile against
  this file — trust nothing you didn't re-run. `docs/repo-kit/` is gitignored — absent on a
  fresh clone, this brief IS the loop.
- **Loop:** reconcile → ONE card → fresh executor chat per card → planner re-runs gate +
  shasums + file scope → flip card + dated worklog line → commit the verified slice → push →
  next card. Review swarm (cheap fresh sessions, skeptic pass) before any gate/row declaration.
- **Authority (operator ruling 2026-08-06 — supersedes "the operator commits" lines in
  AGENTS.md for AWC runs):** PLANNER commits verified green slices under the operator's
  configured global git identity, zero co-author/AI trailers, one commit per card, and pushes
  `origin/main`. EXECUTORS still never commit. Never commit or push a red tree.
- **Stops (hard):** NO key loading, signing, submission, RPC, or network — ever, in this phase
  (M8/M9 need separate explicit human approval) · determinism or fixed-point money weakened ·
  either shasum moves without a card sanctioning it · lockstep plan pair edited · anything
  externally visible beyond `git push`. BLOCKED → worklog line, switch cards; never guess.
- **End of run:** rewrite this handoff (≤60 lines) from re-verified state, refresh
  current-state.md, final worklog line, everything committed + pushed.

## Hard rules that bite (full lists: AGENTS.md, docs/invariants.md)
- Determinism + `Decimal` money non-negotiable; synthetic HF data only. Never stage `.claude/`
  or `data/`; never `git add -A`. Never edit `plans/master-plan.md` /
  `solana-crypto-trader-plan.md` except byte-identical lockstep (Q6). Gate counts pasted from a
  run THIS session, never transcribed; baselines only via `bash scripts/handoff-baselines.sh`.
