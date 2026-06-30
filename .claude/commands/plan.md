---
description: Start a change — write its plan and check it against the real codebase
argument-hint: <what you want to change>
---

# /plan $ARGUMENTS

> Engine root: resolve `.claude/<X>` paths from `<repo>/.claude/` if present, else `~/.claude/` — see *Delivery modes* in `.claude/guide/context.md`.

Run **Phase 1** of the `context-loop` skill for the change: **$ARGUMENTS**

1. Copy `.claude/templates/plan.md` to `plans/<change-name>.md` and fill it in — Goal, In scope,
   Out of scope, Steps, Definition of done. Ask the user only for what you cannot infer.
2. Enter plan mode. Delegate exploration to a read-only **Explore** subagent so this
   window stays clean. As the context check, have it **read the relevant `CLAUDE.md`
   hierarchy first** (root + the nested ones for the directories this change touches),
   then confirm the files/functions named in the plan exist, find existing utilities to
   reuse, and surface any assumption that's wrong.
3. Report the context check back. Correct the plan before writing any code. Do **not**
   start implementing until the plan survives contact with the real codebase.

Then hand back to the user to approve or correct the plan. Once approved, implement it
**exactly as written** (Phase 2), keeping diffs small and resetting early — around 30% context, with 40% auto-compaction as the backstop.
