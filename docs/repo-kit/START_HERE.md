# START_HERE — Repo Bootstrap Instructions (agent-parsed)

You are an agent starting work in this repository. This folder was just dropped in. Your job: activate it, then delete this file. Global operating rules are at `~/.claude/AGENTS.md` (v1.3) — read them first; nothing here overrides them.

## Mode detection
- Repo has meaningful existing code → **HARVEST mode** (all steps).
- Repo is empty/new → **NEW mode** (skip step 2; fill templates via interview in step 3).

## Steps

### 1. Placement check
Confirm this structure sits at repo root: `NORTH_STAR.md`, `AGENTS.md`, `context/` (state, decisions, conventions, worklog), `plans/` (TEMPLATE.plan.md, archive/, reports/). If a same-named file already existed in the repo, do NOT overwrite: merge its real content into the template structure and note the merge in your report.

### 2. Harvest (existing repos only)
Read the codebase and git history (last ~200 commits; use path filters if noisy) to fill, from evidence:
- `AGENTS.md`: project one-liner, stack, real run/test/build commands, architecture screen, hot seams.
- `context/conventions.md`: patterns actually in use (follow) and inconsistencies found (avoid).
- `context/decisions.md`: seed 3–8 entries for major architectural choices visible in history (mark `source: harvested`).
- `context/state.md`: honest works-now / known-issues from tests + code, not from README claims.
- Flag every stale doc you encounter (contradicts code) — list them; do not obey them.

### 3. North star
Draft `NORTH_STAR.md` from repo evidence (or interview the human in NEW mode). Keep `status: DRAFT — confirm`. You never edit this file again after the human confirms it — flag staleness instead.

### 4. GATE
Identify or define the single GATE command in `AGENTS.md`. If none exists, that's milestone M1 of the first plan — a repo without a gate has no machine definition of done.

### 5. First plan
Invoke the `plan-extraction` skill to create `plans/PLAN-001-*.md` for the current objective. New apps: walking skeleton first. Log it as the first `worklog.md` row.

### 6. Report + cleanup
Completion report per global T4 format (taxonomy, evidence, stale-docs list, exact next step). Then delete `START_HERE.md` — its job is done; `AGENTS.md` takes over.

## Standing rules while you work here (from global v1.3 — reminders, not overrides)
- Truth: NORTH_STAR.md → active plan → context/ → repo AGENTS.md → global rules.
- Run to completion. NEEDS_HUMAN is a report label, never a stop. Stops: credentials, destructive/irreversible external actions, exhausted failure, or plan-listed escalation points only.
- No commits unless the human asks this session. No AI attribution. No destructive git without in-the-moment approval.
- Write cadence T1–T5 is deterministic. Budgets enforced: compress in the same edit that breaches.
- Evidence or it didn't happen: gates pasted, file:line cites, worklog rows closed on evidence only.
