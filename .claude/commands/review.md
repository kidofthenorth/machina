---
description: Run one review lens (correctness | security | tests) in this fresh session
argument-hint: correctness | security | tests
---

# /review $ARGUMENTS

> Engine root: resolve `.claude/<X>` paths from `<repo>/.claude/` if present, else `~/.claude/` — see *Delivery modes* in `.claude/guide/context.md`.

Run **Phase 3** of the `context-loop` skill — a single review lens: **$ARGUMENTS**

This only works in a **fresh session** (or a dedicated subagent) — never the one that
wrote the code, and never blending lenses. One job, one pass.

1. Load the diff under review **and** the active plan file (`plans/<change-name>.md`).
2. Open `.claude/checklists/review.md` and run **only** the pass matching `$ARGUMENTS`:
   - `correctness` → Pass 1
   - `security` → Pass 2
   - `tests` → Pass 3
   - (Pass 4 — *Scope & diff hygiene* — has no argument; run it by hand from the checklist when a diff feels sprawling.)
   (If `$ARGUMENTS` is empty, ask which lens; don't run all three at once.)
3. Write findings tied to file/line. Then **stop** — don't fix, don't drift into another
   lens.
4. Feed findings back as a new, small `/plan` — not as sprawling edits to this session.

Run the lenses as separate passes, each in its own fresh session: correctness, then
security, then tests.
