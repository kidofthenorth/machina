# Context files — the contract between code and agents

The session loop (plan → small diffs → fresh review) governs *one change*. Context files
govern *everything an agent knows before it starts*. They are the persistent, agent-readable
description of the code — the contract a fresh session reads to get up to speed without
re-deriving the whole repo. Stale or missing context is the quiet way a clean-window
workflow still produces wrong code: the window was clean, but what filled it was wrong.

## Delivery modes

The cadence kit can be installed in two ways:

- **Copied into the repo** (`.claude/`) — the default. Portable and team-friendly: everyone who
  checks out the repo gets the kit. Recommended for teams.
- **Installed globally** (`~/.claude/`) — solo convenience. Copy the kit's `.claude/` *contents*
  into `~/.claude/` once; commands and skills are then available in any repo without copying the
  folder in.

**Engine-root rule:** Any cadence file that references `.claude/<X>` (a template, checklist,
guide, or skill) resolves that path from the *engine install* — the repo's `.claude/` if present,
else `~/.claude/`. When both exist, the repo's `.claude/` wins (matching Claude Code's own
project-over-user precedence). Generated artifacts (`AGENTS.md`, `CLAUDE.md`, `plans/`) always
live in the repo, never in `~/.claude/`. In global mode, `~/.claude/VERSION` governs all repos
that rely on the global install.

## Mechanism: `AGENTS.md` (source of truth) + the native `CLAUDE.md` hierarchy

The shared baseline is a root **`AGENTS.md`** — the tool-agnostic source of truth (stack,
commands, map, conventions, hard rules) that Codex, Cursor, and other agents read directly.
Claude Code doesn't read `AGENTS.md` natively yet, so the repo also keeps a thin root
**`CLAUDE.md`** that imports it with `@AGENTS.md` and adds the Claude-only machinery (reset
discipline, skill/command pointers). Use `@AGENTS.md`, not a symlink — a symlink leaves no
room for the Claude-only lines and breaks on some Windows / CI checkouts.

Below the root, per-module context uses Claude Code's built-in hierarchy rather than a
parallel convention:

- **Nested `CLAUDE.md` files are auto-loaded by directory.** When the agent works in
  `src/auth/`, Claude Code pulls in `src/auth/CLAUDE.md` automatically. No wiring, no
  reminding the agent to read it. Keep each one short — it sits in the window the whole time
  the agent is in that directory.
- **`@import` is not lazy.** An `@imported` file loads at launch *every* session, so it helps
  organization, not context budget — reserve it for what you genuinely want in every window
  (like the `@AGENTS.md` baseline). For deep context you want only *sometimes* — rationale,
  architecture, history — keep it in a separate doc the agent pulls on demand (a nested
  `CLAUDE.md` deeper in the tree, or a plain doc found via glob/grep), not behind an `@import`.

**We deliberately do *not* invent a separate `CONTEXT.md` convention.** A parallel file the
tool doesn't auto-load would have to be manually wired into every plan, and agents would
silently skip it. Leaning on `AGENTS.md` + `CLAUDE.md` means context is read *by default* —
which is the whole point. Fight the tool and the context goes unread; lean on it and it's free.

**Portable vs Claude-only — be honest about the boundary.** Everything in `AGENTS.md` (the
project facts *and* the loop it describes in plain prose) is tool-agnostic: Codex, Cursor, and
humans read it and follow the loop by hand. The slash commands (`/plan`, `/review`,
`/context-health`), the skills, and the auto-loaded nested `CLAUDE.md` hierarchy are Claude Code
conveniences layered on top — not a separate system. Other tools lose the automation, not the
discipline. Don't write docs that imply non-Claude tools can run the slash commands or skills
natively; point them at the prose loop in `AGENTS.md` instead.

**Tracked vs gitignored — decide deliberately.** Nested `CLAUDE.md` files auto-load locally
whether or not git tracks them, so a `.gitignore` rule that swallows them breaks nothing *for
you* — but a bare `CLAUDE.md` line matches at *every* depth, and a teammate or non-Claude agent
who clones the repo then gets the portable `AGENTS.md` and **no** module context at all. Default
to tracking the hierarchy so the context travels with the code; gitignore it only when you
deliberately want context kept local (e.g. a public repo whose internal notes stay private).
`/init-cadence` checks this during its run: if a rule would swallow the context, it **asks you
inline** and applies a one-line fix only with your say-so — resolving the question within the pass,
rather than editing `.gitignore` behind your back or leaving it for later. (The kit's own folders use
collision-safe names — `guide/`, not `docs/` — so a host rule can't silently eat part of `.claude/`.)

Templates:
- Root baseline (any agent) → `.claude/templates/root-AGENTS.md`
- Root Claude shim (`@AGENTS.md` + Claude-only) → `.claude/templates/root-CLAUDE.md`
- Per-module, auto-loaded → `.claude/templates/module-claude-md.md`

## Granularity

One `CLAUDE.md` per **major module or directory** — a unit with its own responsibility.
Not one per file (noise, and impossible to keep fresh), not one for the whole repo (too
coarse to be useful past the root overview). When a directory's job is obvious from a
glance, it doesn't need its own context file; when an agent would have to read several
files to understand the contract, it does.

**Scale down for tiny repos.** A handful of files with one obvious job needs none of this
nesting: root `AGENTS.md` + root `CLAUDE.md` are the whole cadence. Skip per-module notes
entirely until the repo grows enough that someone has to read several files to understand a
part of it — add the first nested `CLAUDE.md` then, not before. Don't manufacture module
notes to look thorough; an empty hierarchy is worse than no hierarchy.

**Scale up for monorepos.** When several packages each have their own manifest, build, and tests, a
single root note can't describe them all. Give each package its own `CLAUDE.md` (its `Describes:` glob
is that package, and it records that package's commands), or treat each package as its own cadence if
they're truly independent. Don't force N toolchains into one root note.

## Routing module context elsewhere (the `Module context:` declaration)

Some mature repos already keep per-module context in **tracked docs** governed by a checker — e.g.
`docs/ENGINEERING.md` plus `docs/*/README.md`, kept honest by a `check-docs` script — instead of
nested `CLAUDE.md`. That's a legitimate choice: it trades Claude's per-directory **auto-load** for
context that is *tracked, tool-agnostic, and CI-governed*. Don't fight an existing system like that
by bolting a parallel nested surface beside it.

**Declare it instead,** with one line in the root `AGENTS.md`:
```
Module context: docs — see docs/ENGINEERING.md (+ docs/*/README.md), governed by check-docs.mjs.
```
`/context-health` reads that line and stops emitting 🟠 *missing* for every major directory; it
records the choice once (**ℹ️**) and reminds you it can't see into docs/, so that surface's freshness
rides on *its own* checker, not this audit. Omit the line (or write `Module context: nested`) to keep
the default — a nested `CLAUDE.md` per major module, flagged 🟠 when absent.

The tradeoff is real and worth naming: nested `CLAUDE.md` **auto-loads** the moment an agent works in
that directory; docs don't (the agent gets them via the root `@AGENTS.md` pointer, not per-directory).
So declaring `docs` is right when a governed docs system already exists — but for the few
highest-traffic or highest-risk directories, a short nested `CLAUDE.md` as a *complement* buys
auto-loaded context exactly where it pays off. Declaring `docs` doesn't forbid adding them.

## The cadence — create and update triggers

Context is only worth trusting if it tracks the code. Two triggers:

- **Create** when a new module lands, or a major refactor reshapes one enough that the
  old mental model no longer holds.
- **Update** when a module's **public interface, boundaries, or invariants** change —
  *as part of the same diff that changes them.* Updating the `CLAUDE.md` is not a
  follow-up task; it's part of "done." (The `context-loop` skill's Build phase enforces
  this.) When you re-confirm a file against the code, restamp its `Last verified:` marker.

## How agents use it

- A plan's **context check** (see `.claude/templates/plan.md`) reads the root baseline (`AGENTS.md`,
  pulled in via `CLAUDE.md`) plus the nested `CLAUDE.md` files for the directories the change
  touches — *before* proposing code.
- An **Explore** subagent grounds itself in that hierarchy first, then maps the code,
  so the main session never absorbs the exploration noise.

## Bootstrapping an existing repo

The code already exists; the context doesn't. **`/init-cadence` does this for you**; here's
what it (or you, by hand) does — don't write it cold:
1. Point an **Explore** subagent at each major module: "read this directory and write its
   `CLAUDE.md` from `.claude/templates/module-claude-md.md`, verifying every concrete claim —
   names, routes, paths — against the actual code."
2. **Resolve uncertainty as you go:** confirm what's greppable before writing it; for what the code
   can't settle (intent, an invariant), ask rather than guess. Don't leave it for a later pass.
3. Stamp `Last verified:` with today's date, so `/context-health` has a baseline.
4. Run the first real change through the full loop to validate the setup end to end.

## Draft vs verified — the `(confirm)` tag

A `(confirm)` tag marks a claim that hasn't been checked against the code — "verify this before
trusting it." In the normal flow you shouldn't see many: `/init-cadence` **verifies every greppable
fact before writing it and asks about anything it can't**, so it resolves uncertainty within its pass
rather than shipping a guess. A `(confirm)` is the *fallback* — left only when the agent can neither
verify nor ask (a non-interactive run), or added by hand to flag something you wrote but haven't
confirmed yet.

Keep the `(confirm` opening literal (write `**(confirm: route set)**`, not `(**confirm…**)`) so it
greps cleanly; `/context-health` matches the family case-insensitively and tolerates stray bold, but
a clean tag is easiest to find. A `Last verified:` stamp from the day a note was written means
*written today*, not *confirmed* — an unretired `(confirm)` is what marks that gap.

Retiring one is a deliberate step: read the tagged claim against the code, correct it if wrong,
**delete the `(confirm)` tag**, and restamp `Last verified:`. Until the tag is gone the note is
structurally fresh but semantically unverified — so `/context-health` reports any unretired
`(confirm)` tags (**unverified**), keeping the "stamped ≠ confirmed" gap visible.

## Keeping it fresh

Run [`/context-health`](../commands/context-health.md) to audit the hierarchy: it
flags context whose code has changed since `Last verified:` (**stale**), major directories
with no `CLAUDE.md` (**missing**), malformed markers or dangling `@import`s (**broken**), and
notes still carrying unretired `(confirm)` tags (**unverified**). It only reports — refresh
flagged files with a fresh, small `/plan`, the same as any other change.

## Improving imported docs (the inbound direction)

This repo is also the **yardstick**. When an existing repo's docs are brought here to be
upgraded, measure them against this guide and the templates: Is context
co-located and auto-loaded, or off in a file nothing reads? Is it scoped per-module, or
one giant doc? Does it carry freshness markers? Upgrade against the standard, then send it
back. No extra tooling — the master set *is* the standard.
