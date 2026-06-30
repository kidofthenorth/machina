---
name: context-loop
description: Run a disciplined plan -> small-diff -> fresh-review loop for any non-trivial code change, so the agent works against a written plan and a clean context window instead of a cluttered one. Use when starting a feature, fix, or refactor that touches real code — especially anything spanning more than a file or two, or any task where the agent has previously drifted, forgotten plan details, or gone off-script. Trigger on "start a change", "implement this with the loop", "plan this change", "run the loop", or when a task is big enough that context will fill before it is done.
---

# Context Loop

The constraint is context, not capability. A clean window with a written plan beats a long, cluttered session every time. Run every non-trivial change through three phases. Do not collapse them.

> Engine root: resolve `.claude/<X>` paths from `<repo>/.claude/` if present, else `~/.claude/` — see *Delivery modes* in `.claude/guide/context.md`.

## Phase 1 — Plan (before touching code)
1. Copy `.claude/templates/plan.md` to `plans/<change-name>.md` and fill it in with the user. Ask only for what you cannot infer.
2. Enter plan mode. Delegate codebase exploration to a read-only subagent so the main window stays clean. As part of the context check, **read the relevant `CLAUDE.md` hierarchy first** — the root `CLAUDE.md` plus the nested per-directory ones for the dirs the change touches — then confirm the files/functions named in the plan exist, find existing utilities to reuse, and surface any assumption that is wrong. (See `.claude/guide/context.md`.)
3. Report the context check back to the user. Correct the plan before writing any code. Do not start implementing until the plan survives contact with the real codebase.

## Phase 2 — Build (small diffs, reset early)
1. Implement the plan **exactly as written**. Add nothing that is not in the plan. If something necessary is missing, stop and update the plan first.
2. One step from the plan = one small, reviewable diff. Finish it, verify it, then move on.
3. Watch context. Around 30% full — well below the 40% auto-compaction backstop — checkpoint progress into the plan file's Progress section (done, next, modified files) and recommend the user `/clear` and resume in a fresh session pointed back at the plan. A fresh session with a precise prompt beats a long one carrying corrections.
4. Keep durable constraints in CLAUDE.md (modified-files list, test command, "don't touch" rules) so a reset never loses the thread.
5. If the change alters a module's public interface, boundaries, or invariants, **update that directory's `CLAUDE.md` in the same diff** — refreshing context is part of "done," not a follow-up. Restamp its `Last verified:` marker. (See `.claude/guide/context.md`.)

## Phase 3 — Review (fresh session, one lens at a time)
1. Run review in a clean session or dedicated subagent — never the session that wrote the code. It will agree with itself.
2. Use `.claude/checklists/review.md`. Run one lens per pass: correctness, then security, then tests & edge cases. Load the diff + plan, run only that lens, report, stop.
3. Feed findings back as a new, small plan — not as sprawling edits to a tired session.

## Guardrails
- If the plan has been corrected twice and the agent is still off, the context is polluted: `/clear` and restart from a sharper plan rather than pushing on.
- If a task touches more than ~5 files, split it or isolate parts in subagents.
- The plan file is the source of truth. Resets are safe because the plan persists — protect that invariant.
- Context files are the other half of the contract: agents read them before touching code, and update them when behavior changes. Run `/context-health` to catch context that has drifted from the code.
