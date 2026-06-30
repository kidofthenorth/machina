---
description: Bootstrap the cadence in this repo — generate AGENTS.md, CLAUDE.md, module context, and plans/ from the cadence engine (copied per-repo in .claude/ or installed globally in ~/.claude/)
---

# /init-cadence

> Engine root: resolve `.claude/<X>` paths from `<repo>/.claude/` if present, else `~/.claude/` — see *Delivery modes* in `.claude/guide/context.md`.

Run the **`init-cadence`** skill: one-time setup after installing the cadence engine (copied per-repo into `.claude/`, or globally into `~/.claude/`).

It scans the codebase (via a read-only **Explore** subagent), then generates the repo's own
context from the engine templates:
1. root **`AGENTS.md`** — the tool-agnostic source of truth, filled from the scan
2. root **`CLAUDE.md`** — the thin `@AGENTS.md` shim + Claude-only machinery
3. per-module **`CLAUDE.md`** context for each major directory — facts verified against the code as it's written
4. a **`plans/`** directory for the loop

It verifies what it can against the code and asks you about anything it can't, so the pass finishes
with nothing to clean up. Then it points you at `/context-health` to confirm it's green, and your first `/plan`.

For a **tiny repo** with no major modules, it skips the per-module notes and says so — root
`AGENTS.md` + `CLAUDE.md` are the whole cadence until the repo grows.

If `AGENTS.md` / `CLAUDE.md` already exist, it asks before changing them.

> Run this **instead of** Claude Code's built-in `/init` — `/init-cadence` does everything `/init`
> does and more, in the cadence's shape. You don't need both.
