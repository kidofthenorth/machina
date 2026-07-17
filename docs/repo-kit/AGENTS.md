# [PROJECT NAME] — Repo Agent Rules
<!-- Budget: 150 lines. Routing file, not an encyclopedia. Global rules live in ~/.claude/AGENTS.md (v1.3)
     and are NOT repeated here. Truth hierarchy: NORTH_STAR.md → active plan → context/ → this file → global. -->

## Project
[One-liner: what this is.] Stack: [languages, frameworks, key services].

## Commands
- Run: `[command]`
- Test: `[command]`
- Build: `[command]`
- **GATE** (machine definition of done — typecheck+tests+lint+build in one shot): `[command]`
  <!-- If no single gate command exists yet, creating one is the first milestone of the first plan. -->

## Routing (read on demand — grep, don't bulk-read)
- Current state, blockers, do-not-judge-yet → `context/state.md`
- Past decisions + forks not taken → `context/decisions.md`
- Style/naming/patterns → `context/conventions.md`
- Task ledger → `context/worklog.md`
- Active + archived plans → `plans/`

## Never do (project-specific)
- [Hard constraint 1 — e.g. never touch the legacy /v1 API]
- [Hard constraint 2]

## Architecture (one screen max)
[Layers/modules and one-sentence responsibilities. Where state lives. Key seams that need invariant tests.]
