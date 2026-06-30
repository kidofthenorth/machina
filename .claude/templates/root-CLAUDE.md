# CLAUDE.md template — the Claude Code shim

> Copy this to the **repo root** as `CLAUDE.md`, alongside `AGENTS.md` (from
> `.claude/templates/root-AGENTS.md`). It pulls the shared baseline in via `@AGENTS.md` and adds the
> Claude Code-specific machinery. Use `@AGENTS.md` — **not** a symlink (a symlink leaves no
> room for the Claude-only lines and breaks on some Windows / CI checkouts).
>
> Delete this instruction block once filled in.

---

@AGENTS.md

## The loop (how we work here)
Plan → small diffs → fresh-session review. See the `context-loop` skill. The active plan
file is the source of truth across resets.

- **Active plan:** `plans/<current-change>.md`  ← update this pointer when you start a change

## Agent guidance
- Search/exploration → an **Explore** subagent, so the main window stays clean.
- Review → a **fresh session** (or dedicated subagent), one lens at a time. A session never
  reviews its own work well.
- A task touching more than ~5 files → split it, or isolate parts in subagents.

## Reset discipline (keep this verbatim — it's what makes resets safe)
When compacting or before `/clear`, **always preserve**:
1. the active plan file path,
2. the list of modified files,
3. the test command (in `AGENTS.md`).

Reset early — around 30% context, before quality drops, not after. Keep the auto-compaction override
as a *backstop* set above that manual trigger, so it only fires if you blow past the reset:
```
CLAUDE_AUTOCOMPACT_PCT_OVERRIDE=40   # backstop — above the ~30% manual reset
```
A fresh session pointed back at the plan beats a long one carrying a pile of corrections.
