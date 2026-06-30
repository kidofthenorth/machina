---
description: Audit the CLAUDE.md context hierarchy for staleness and gaps
---

# /context-health

> Engine root: resolve `.claude/<X>` paths from `<repo>/.claude/` if present, else `~/.claude/` — see *Delivery modes* in `.claude/guide/context.md`.

Read-only audit of this repo's context layer. It does **not** edit anything — it
reports which `CLAUDE.md` files have fallen behind the code so a human can decide
what to refresh. The cadence of *updating* context only works if staleness is visible.

## What to do

1. **Find every context file.** List all `CLAUDE.md` files in the repo (the root one
   and every nested per-directory one). Use a **recursive glob** as the authority — it sees on-disk
   files whether or not they're tracked (init-cadence writes notes it doesn't stage, and some repos
   keep them gitignored on purpose); cross-check `git ls-files` only if you want. **Sanity-check the kit
   sits at the repo root:** if `.claude/.claude/` exists, the cadence was copied one level too deep —
   flag it (the commands/skills won't load until it's flattened up into `.claude/`).

2. **Read each one's markers.** Each per-module `CLAUDE.md` (see
   `.claude/templates/module-claude-md.md`) ends with:
   ```
   Describes: `<glob>`
   Last verified: `<YYYY-MM-DD or commit SHA>`
   ```
   - `Describes:` is the path glob the file is responsible for.
   - `Last verified:` is when a human last confirmed it against the code.

3. **Detect staleness.** For each file, check whether the code it `Describes:` changed
   after it was `Last verified:`:
   ```
   git log --oneline --since="<Last verified date> 00:00:00" -- <Describes glob>   # 00:00:00 pins midnight
   # or, if Last verified is a commit SHA:
   git log --oneline <SHA>..HEAD -- <Describes glob>
   ```
   Any commits returned ⇒ the code moved since the context was confirmed ⇒ **stale**.

4. **Detect missing context — unless the repo declares another strategy.** First check the root
   `AGENTS.md` (and root `CLAUDE.md`) for a `Module context:` line. If it routes context **elsewhere**
   (e.g. `Module context: docs — …`), the repo opted out of nested per-module notes by design: do
   **not** flag major directories as missing — emit the single **ℹ️** note below instead. Otherwise
   (no line, or `Module context: nested`), flag major directories (real modules with their own
   responsibility, not trivial leaf folders) that have **no** `CLAUDE.md`. (See *Routing module
   context elsewhere* in `.claude/guide/context.md`.)

5. **Detect broken context.** Among the *per-module* `CLAUDE.md` files (not the root one —
   it has no markers by design), flag any missing the `Describes:`/`Last verified:` markers,
   or whose `Describes:` glob matches no files, or whose `@import` deep-context paths don't
   resolve.

6. **Detect unverified drafts.** Grep every context file the cadence writes — each `CLAUDE.md`, the
   root `AGENTS.md`, and any `*.cadence-proposed.md` sidecar (not `.claude/`'s own prose) — for the
   `(confirm` marker family, tolerantly:
   case-insensitive, allowing markdown bold between the paren and the word — `grep -inE '\(\**confirm'`
   (why this pattern, not a plain `(confirm)`: see *Draft vs verified* in `.claude/guide/context.md`).
   A normal `/init-cadence` run leaves **none** — it verifies greppable facts and asks about the rest.
   A surviving `(confirm)` is the last-resort fallback (a non-interactive run that could neither verify
   nor ask), or one a human added by hand: a claim still flagged as not-yet-checked against the code.

## Output — a prioritized report

Group findings, most urgent first:

- **🔴 Stale** — code changed since `Last verified:`. List the file, its glob, and the
  commits that moved it. These need a human to re-read and re-stamp.
- **🟠 Missing** — major directory with no `CLAUDE.md` (when module context is *nested* — the
  default). Suggest creating one from `.claude/templates/module-claude-md.md`.
- **ℹ️ Module context routed elsewhere** — the root context declares `Module context: <…>` (e.g.
  docs-routed). Report it **once**, not per-directory, and don't flag those dirs as missing: their
  context lives outside this audit's view, so its freshness rides on *its own* governance (e.g. a
  `check-docs` script), not this tool. Informational, not a finding.
- **🟡 Broken** — missing/malformed markers or dangling `@import`/glob. Name the fix.
- **🟣 Unverified** — carries unretired `(confirm)` tags: a claim flagged as not-yet-checked against
  the code (a last-resort fallback, or one a human added — a normal init leaves none). List the file
  and the tagged lines. The fix is human, not structural: confirm each claim, delete the tag, then
  restamp `Last verified:`.
- **🟢 Healthy** — verified after its last code change. List briefly; don't belabor.

End with a one-line recommendation of which 1–3 files to update first.

## Notes
- **Structural only — this is rung one, not the whole ladder.** It catches *structural* drift:
  code changed since `Last verified:`, dead globs, missing or broken markers. It can't see
  *semantic* drift — a note that still looks current but describes a convention quietly
  superseded. For that, have a human (or a fresh agent) re-read the notes you marked 🟢 every so
  often; freshness markers track *when*, not *whether*, a note is still true. The one semantic
  signal it *can* catch is the explicit `(confirm)` tag (🟣) — because it's a literal string, not a
  judgment — but **clearing** it is still a human call, not something the audit can do. For the higher
  rungs this can't reach — semantic coherence, model consistency, the install story — run
  `/cadence-audit` (deep, multi-agent, adversarially verified).
- **Same-day commits — pin the boundary, then adjudicate.** `Last verified:` is day-granular
  (`YYYY-MM-DD`). Always use the `00:00:00` boundary from step 3: a bare `--since=<date>` can be
  parsed at the *current* time-of-day and silently drop a same-day commit — stale context reading
  green, depending on the minute you run the audit. With midnight pinned, a commit on the same day as
  `Last verified:` correctly surfaces as possibly-stale; then check whether the note already reflects
  it (a note written at 18:00 against an 18:21 commit is current, not stale). For exactness, stamp
  `Last verified:` with the commit SHA instead of the date (the marker format allows either) —
  `SHA..HEAD` resolves it with no day-boundary guesswork.
- Pure audit. Never edit a `CLAUDE.md` here — recommend, and let the human (or a fresh
  `/plan` for the update) do the writing.
- If the repo has no `git` history, skip the time-diff and report only **missing** / **broken** /
  **unverified** findings (the last is a `(confirm)`-tag grep, so it works without git), noting
  that staleness can't be computed without git.
