---
name: init-cadence
description: Bootstrap the context-loop cadence in a repo after installing the cadence engine (copied per-repo into .claude/ or globally into ~/.claude/) — scan the codebase and generate a filled-in root AGENTS.md (the tool-agnostic source of truth), a thin CLAUDE.md shim, per-module CLAUDE.md context, and a plans/ directory. Use after installing the cadence engine, or when the user says "init-cadence", "bootstrap the cadence", "set up the cadence", or "initialize context".
---

# Init Cadence

One-time setup that turns a freshly installed cadence engine into a working cadence for a specific
repo: it writes the repo's *own* context — root `AGENTS.md` + `CLAUDE.md`, plus per-module
`CLAUDE.md` — from the engine templates, grounded in what the code actually is. Run it once, right
after installing the cadence (per-repo in `.claude/` or globally in `~/.claude/`). It is the
automated version of "Bootstrapping an existing repo" in `.claude/guide/context.md`.

**One pass, no loose ends.** init-cadence completes within a single run. It resolves every fact one
of two ways — **verify** it against the source (anything greppable: symbols, routes, paths, services,
commands), or, for what the code can't settle (intent, the one rule not to break) and for any
blocking conflict (a `.gitignore` rule that would swallow the context), **ask** the user, batched,
and act on the answer. Don't write a guess, don't defer to a handoff, don't leave a routine
`(confirm)` tag. When the pass ends, `/context-health` should come up green with nothing to clean up.

## Before you start
- **Check for a nested kit (only if `<repo>/.claude/` exists).** If `.claude/.claude/VERSION`
  (or `.claude/.claude/templates/`) exists, the kit was copied one level too deep — the most
  common careless-copy outcome (`cp -R kit/.claude repo/.claude` when `repo/.claude` already
  existed). Flatten it before anything else: move `.claude/.claude/*` up into `.claude/`, then
  remove the empty inner dir. Don't mistake this for "not copied in fully." *(In global-only mode
  with no repo `.claude/`, there is nothing to check here — skip.)*
- Confirm the engine is present: check `<repo>/.claude/templates/` (copied mode) **or**
  `~/.claude/templates/` (global mode) for `root-AGENTS.md`, `root-CLAUDE.md`,
  `module-claude-md.md`, and the matching `guide/context.md`. If neither location has them, the
  kit isn't installed — stop and say so. (See *Delivery modes* in `.claude/guide/context.md`.)
- If a root `AGENTS.md` or `CLAUDE.md` already exists, do **not** overwrite blindly — show what
  you'd change and ask first. Re-running should refine, not clobber.

## Steps

> Engine root: resolve `.claude/<X>` paths from `<repo>/.claude/` if present, else `~/.claude/` — see *Delivery modes* in `.claude/guide/context.md`.

1. **Scan the repo (read-only).** Delegate to an **Explore** subagent so the main window stays
   clean. Have it report, from the actual code — not guesses:
   - stack / languages / frameworks;
   - build, test, run/dev, and lint/typecheck commands (from `package.json`, `Makefile`,
     `pyproject.toml`, CI config, etc.);
   - the top-level module layout (the major directories and what each is for);
   - whether per-module context already lives in **tracked docs** governed by a checker (a `docs/`
     tree + a `check-docs`/`lint-docs` script in `package.json`/CI) — if so, report the docs root and
     the checker command (this is the signal for step 4's docs-routing branch);
   - "never touch" candidates: migrations, generated/vendored code, lockfiles, infra, public
     API surfaces.
2. **Generate root `AGENTS.md`** from `.claude/templates/root-AGENTS.md`, filled from the scan.
   Delete the template's instruction block and every angle-bracket placeholder. Ask the user
   only for what the code can't tell you — the one-paragraph "what this is / who it serves" and
   the single thing not to get wrong — and ask it in **one batched message**, not a drip of
   questions. Propose the "Never do" rules you inferred (security invariants, generated/vendored
   paths, etc.) for the user to confirm rather than asserting them silently.
3. **Generate root `CLAUDE.md`** from `.claude/templates/root-CLAUDE.md`. It's mostly `@AGENTS.md`
   plus the Claude-only machinery; point the active-plan line at `plans/` and keep it short.
4. **Write per-module `CLAUDE.md`** for each *major* module the scan found (a directory with its
   own responsibility — not trivial leaf folders). Use `.claude/templates/module-claude-md.md` and
   stamp `Describes:` with that directory's glob, `Last verified:` with today's date. Keep each
   short — it's always in the window. **Verify before you write:** every concrete claim — a function
   or class name, a route, a file path, a service, a command — must be confirmed against the source
   first (grep the symbol, read the route table, list the services). Don't write what you haven't
   checked. The only things you may leave unsettled are genuine judgment calls the code can't decide
   (an intent, an invariant you can't prove) — collect those and ask them as batched questions (one
   message, per *One pass* above), never a guess left for later.
   - **Tiny repo?** If the scan finds no directory that rises to a *major* module — a handful of
     files with one obvious job — skip this step. Root `AGENTS.md` + `CLAUDE.md` are the whole
     cadence until the repo grows. Don't manufacture module notes to look thorough.
   - **Already routes module context elsewhere?** If the repo keeps per-module context in tracked
     docs governed by a checker (e.g. `docs/ENGINEERING.md` + `docs/*/README.md` with a `check-docs`
     script), don't bolt a competing nested `CLAUDE.md` surface beside it. Write one line in
     `AGENTS.md` — `Module context: docs — <where>, governed by <checker>` — so `/context-health`
     records the choice and stops flagging those dirs as missing (see *Routing module context
     elsewhere* in `.claude/guide/context.md`). Offer nested notes only as a complement for the few
     highest-traffic dirs, if the user wants auto-loaded context there.
   - **Monorepo / workspace?** If the scan finds several packages that each carry their own
     manifest/build/test (e.g. `packages/*`, each with a `package.json`), one root `## Commands`
     block can't represent them. Give each package its own per-package `CLAUDE.md` whose `Describes:`
     glob is that package and which records *that* package's commands — or, if the packages are truly
     independent projects, run the cadence per package. Don't cram N toolchains into the root or guess
     one; if the layout is ambiguous, ask which structure the user wants.
5. **Create `plans/`** (the home for the loop's plan files) if it doesn't exist.
6. **Resolve tracking in-pass, then report — nothing left to clean up.**
   - **No git?** If `git rev-parse --git-dir` fails, there's no `.gitignore` to swallow anything —
     note that and skip the tracking gate cleanly (don't run a command that fatals).
   - **Check tracking.** Otherwise run `git check-ignore` on the context files you wrote and on
     `.claude/`. If a host `.gitignore` rule would ignore the generated context (a bare `CLAUDE.md`
     matches at *every* depth, making the hierarchy local-only) or any kit file, don't defer it —
     **ask the user inline**: keep it local-only, or apply a one-line fix (show the exact change, e.g.
     `CLAUDE.md` → `/CLAUDE.md`, or add `!CLAUDE.md`). Apply it only with their say-so; otherwise leave
     it deliberately. (The kit's own folders use collision-safe names, so they shouldn't be swallowed —
     if one is, surface it.)
   - **Report — nothing left to clean up.** What you generated, what you verified, and how each
     question resolved. There should be **no drafts awaiting a human eye** — you verified as you wrote.
     Recommend the next steps: run `/context-health` to confirm the hierarchy comes up green, then `/plan <your first change>` to start the loop. If you
     skipped per-module notes (tiny repo), say so plainly — "no major modules; root notes are enough
     for now" — so it reads as *complete*, not half-finished.

## Guardrails
- Read-only exploration via the subagent; the only things you *write* are the repo's own
  context files and `plans/`. Never touch application code.
- Keep every file lean — short root, short module notes. Bloat defeats the point.
- Fill templates completely: delete the instruction block, every angle-bracket hint, and any
  optional section you aren't using (e.g. "Deep context") — never leave an empty heading.
- Don't invent commands or facts. If the scan finds no test command, say "none found" and ask,
  rather than guessing one.
- **Verify, or ask — don't guess** (the *One pass* doctrine above). The one addition: when you can
  *neither* verify nor ask (a non-interactive run), leave a `(confirm)` tag naming what to check —
  keep the literal opening `(confirm` so the audit catches it. It's a last-resort safety net
  `/context-health` flags, never a substitute for the work (see *Draft vs verified* in
  `.claude/guide/context.md`).
- **Non-interactive run? Default the blocking gates — don't improvise.** The `(confirm)` fallback
  covers *facts*, not *actions*. If you genuinely can't ask: (1) a pre-existing `AGENTS.md`/`CLAUDE.md`
  — **never clobber**; write your proposed version to a sidecar (`AGENTS.cadence-proposed.md`) and
  leave a `(confirm)` pointer to it. (2) a `.gitignore` that would swallow the context — **never
  silently edit it**; leave the context as-is and flag the swallow as a `(confirm)`-tagged blocker
  naming the one-line fix. Either way the run ends with no hidden clobber and no silent `.gitignore` edit.
