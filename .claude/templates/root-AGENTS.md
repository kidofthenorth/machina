# AGENTS.md template — the portable baseline

> Copy this to the **repo root** as `AGENTS.md`. This is the tool-agnostic source of truth
> every coding agent reads (Codex, Cursor, …). Claude Code reads it via the `@AGENTS.md`
> line in `CLAUDE.md` (see `.claude/templates/root-CLAUDE.md`).
>
> Keep it tight — it loads into every session. Per-module detail belongs in nested
> `CLAUDE.md` files (see `.claude/templates/module-claude-md.md`), not here.
>
> Delete this instruction block and the angle-bracket hints once filled in.

---

## Project
<One paragraph: what this repo is, who/what it serves, the one thing not to get wrong.>

## Commands
- Build: `<command>`
- Test: `<command>`
- Run / dev: `<command>`
- Lint / typecheck: `<command>`

## Map
<Where the major modules live; one line each.>
- `<dir>/` — <what it does>
<Optional — if this repo keeps module context in tracked docs instead of nested CLAUDE.md, declare it
so /context-health doesn't flag every module as missing: `Module context: docs — see <path>, governed by <checker>.`>

## How to work here (the loop)
Any agent — Claude, Codex, Cursor, or a human — works in the same rhythm:
- **Plan before code.** Write a short plan and check it against the real files before changing anything.
- **Read before you write.** Inspect the actual code the change touches; don't assume from names.
- **Small diffs.** One reviewable change at a time; the plan file carries the thread across resets.
- **Refresh context in the same diff** that changes behavior — updating the notes is part of "done."
- **Review with fresh eyes,** one lens at a time: correctness, then security, then tests.

In Claude Code this loop is wired up as `/plan`, `/review`, and `/context-health`. Other tools have
no slash commands — follow the same steps by hand. The discipline is portable; the commands aren't.

## Conventions
- <code style / naming / framework conventions agents should follow>

## Never do (without explicit approval)
<Be specific — migrations, generated files, vendored code, public API signatures, infra.>
- `<path or rule>`
