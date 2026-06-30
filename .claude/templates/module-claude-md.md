# Module context template — a per-directory `CLAUDE.md`

> Copy this into a module/directory as its `CLAUDE.md`. Claude Code **auto-loads** it
> whenever the agent works in that directory, so it is *always in the window* — keep it
> short. Put rationale and detail in a deep-context doc and `@import` it on demand.
>
> Delete this instruction block and the angle-bracket hints once filled in.

---

## What this module does
<One sentence. If you need a paragraph, the module is doing too much — or this should point at a deep-context doc.>

## Entry points / public interface
<The exports, routes, or functions callers depend on. What is the contract others rely on?>
- `<name>` — <what it's for>

## Boundaries & key dependencies
<What this module talks to, and what it must NOT reach into.>
- Depends on: `<module/path>` — <why>
- Must not touch: `<module/path>` — <why>

## Invariants & gotchas
<Must-not-change behavior and non-obvious traps an agent would otherwise break.>
- <invariant or gotcha>

## Deep context
<Optional. Point to richer architecture/rationale. Note: an `@import` loads every session,
so for detail you want only *sometimes*, link a plain doc the agent opens on demand (or a
nested `CLAUDE.md` deeper in the tree) — don't `@import` it here.>
- See `<path/to/architecture.md>`

---
<!-- Markers below are read by /context-health. Keep this exact format. -->
Describes: `<glob this CLAUDE.md covers, e.g. src/auth/**>`
Last verified: `<YYYY-MM-DD or commit SHA — update whenever you re-confirm this against the code>`
