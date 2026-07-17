# FOREMAN — the card loop

A planner / executor / reviewer cadence for running a repo with AI agents. Portable — drop this
file into any repo (gitignore it; it's your playbook, not a project artifact). Distilled from a
real milestone: 13 cards, 3 adversarial reviews, 2 correct mid-flight stops, gate declared on the
third attempt with every finding either fixed or descoped **on the record**.

The core idea: **separate thinking from typing, and never let the same context grade its own
homework.** A strong model plans and verifies; cheap fresh sessions execute; disposable agent swarms
attack the work before anything is declared done.

---

## 0. Quick start — casting the Foreman in a new repo

The doc describes the process; you still have to cast the role. Open a chat with your **strongest
model** in the repo and paste this kickoff prompt (fill the goal):

> Read `FOREMAN.md` — you are the **Planner** (the foreman: you run the crew, check the work, and
> never swing the hammer yourself). The goal is: **\<what you want built>**.
>
> Start at step 0 of the loop: audit what the plan/context files claim against the real code (use
> read-only subagents so your window stays clean), record a drift table in the worklog, fix every
> stale pointer, then write the card queue per §4. **If the plan-file scaffolding (§2) doesn't
> exist yet, create it first** — task-queue, worklog, current-state, handoff, DECISIONS — seeded
> from what the code actually is, not what anyone remembers.
>
> From then on, each time I paste an executor's output back to you: verify its claims yourself
> (re-run the full gate, check file scope with `git status`), stage explicit paths only, draft a
> commit message (no co-author or AI-attribution trailers — I commit, you never do), and give me
> the next fresh-chat handoff prompt per §5. Run adversarial review workflows (cheap-model
> subagents, e.g. `model: 'sonnet'`) at the risk points §3 names — after the audit, after the
> riskiest card, before any gate declaration — with §7's skeptic pass and pre-committed decision
> rule. When an executor or review escalates, rule per §8: fix the cause or descope on the record —
> never weaken the assertion that caught it.
>
> Never execute a card yourself; never declare a gate on claims you haven't re-run; never let a
> deliverable be satisfied by a sentence instead of an artifact.

**What the environment must provide:**

1. **A strong model in the planner seat, cheap fresh sessions for executors.** You are the bus
   between them — paste handoffs out, paste reports back. The executor side needs nothing special:
   any repo, any capable cheap model, one card per fresh chat.
2. **Plan-file scaffolding** (§2). In a repo that has it, the planner audits it; in a bare repo,
   the kickoff prompt tells the planner to create it during step 0.
3. **A harness with subagent/workflow support** (e.g. Claude Code) for the review swarms and the
   audit fan-outs. In a plain chat without subagents the planner can still do everything else —
   it should then run the review lenses **sequentially, one lens per fresh pass** (weaker than a
   parallel swarm with skeptics, but honest about it).

---

## 1. Roles

| Role | Who | Does | Never does |
|---|---|---|---|
| **Operator** | the human | runs `git commit`/`push`; makes scope calls when asked; ferries messages between sessions | writes code under deadline pressure |
| **Planner** | strong model, long-lived session | audits reality, writes cards, verifies executor claims, stages files, drafts commit messages, writes handoffs, rules on escalations | executes cards; trusts reports it hasn't re-run |
| **Executor** | cheap model, **one fresh session per card** | types in exactly one card, runs its gate, flips status + one worklog line | opens files the card doesn't name; improvises; commits |
| **Review swarm** | cheap-model subagents in a workflow | attacks finished work through independent lenses; skeptics attack the findings | fixes anything (report-only, narrow test-fix allowance at most) |

## 2. The artifacts

- **Task queue** (`plans/task-queue.md`) — the cards, in execution order, each with a live status.
  The single source of "what next."
- **Worklog** (`plans/worklog.md`) — append-only, dated, terse. Gate results inline. Drift tables,
  review verdicts, and escalation rulings all land here. History is never rewritten.
- **Current state** (`plans/current-state.md`) — where are we, one screen. Updated in the same diff
  as the change it describes.
- **Handoff** (`plans/handoff.md`) — the fresh-session entry point + the seed prompt to paste.
- **Decision log** (`DECISIONS.md`) — numbered entries, newest first: context → decision →
  consequences. Every scope change, contract change, and descope gets one. Silence is the enemy.
- **Master plan** — the authoritative roadmap. The planner quotes it; nobody rewords it casually.

## 3. The loop

```
                     ┌─────────────────────────────────────────────┐
                     ▼                                             │
 [0] RECONCILE ── audit plan claims vs code (agents fan out,       │
     drift table → worklog; fix every stale pointer)               │
                     │                                             │
 [1] CARD-IFY ─── planner reads the REAL code, writes cards        │
     (signatures copied verbatim in-session, never from memory)    │
                     │                                             │
 [2] EXECUTE ──── operator pastes handoff → fresh executor does    │
     ONE card → reports → stops                                    │
                     │                                             │
 [3] VERIFY ───── planner re-runs the gate itself, checks file     │
     scope, stages explicit paths, drafts commit msg, writes       │
     the next handoff → operator commits                           │
                     │                                             │
 [4] REVIEW ───── at risk points only: multi-agent adversarial     │
     review; confirmed findings → new cards or recorded descopes ──┘
                     │
 [5] GATE ─────── declaration card: fresh evidence battery against
     the authoritative plan's own words; any failure = do not declare
```

**Risk points that earn a review (step 4):** after the initial reconciliation audit; after the
single riskiest card; **before any milestone/gate declaration.** Not after every card — reviews are
expensive attention; spend them where a wrong answer compounds.

## 4. Card anatomy

Every card must pass this test: *"a fresh, cheaper model could complete it without asking a question
or opening a file not named on the card."* If it can't, the card isn't done being written.

```markdown
### <ID> — <title> — `TODO`
**Goal.** One sentence + which milestone/gate criterion it serves.
**Files.** Exact paths. ≤3 files / ~150 changed lines (split if not; sanctioned
exceptions are labeled as such, with the reason).
**Current state (verbatim).** The signatures/blocks the card touches, COPIED FROM
SOURCE during card-writing with file:line — never paraphrased from memory. If the
executor finds a mismatch, the card is wrong, not the code.
**Steps.** Numbered. No step may require a judgment call or a search. For anything
subtle, near-complete code the executor types in, not prose describing code.
**Gate.** Runnable commands with expected results (exact test names, exit codes,
output hashes). Plus the full-workspace gate.
**Guardrails.** The project's non-negotiables RESTATED on every card (money types,
determinism, forbidden files, no new deps, …). Repetition is the point.
**Escalate-if.** Conditions where the executor STOPS and reports instead of
improvising: a signature doesn't match; an unrelated test fails; an unlisted file
seems needed; anything ambiguous touches the invariants.
```

Standing rules at the top of the queue (stated once, restated per card in compressed form):
one card per fresh session; flip status + one worklog line on completion (doesn't count against the
file budget); `fmt` after pasting snippets; the full-workspace gate command; the escalation
protocol.

## 5. The handoff prompt

The operator pastes one paragraph-block into each fresh executor session. Template:

> You are the EXECUTOR for **<repo>** — <one-line mission, stated plainly>. <Current phase +
> hard boundary, e.g. "research only: no network anywhere">.
>
> **State:** <cards done>; workspace green at **<N> tests, 0 failed**; <key output hashes>.
> Your task is **<card ID> in <queue file>**. Read the queue's common rules, then the card. It is
> self-contained — do not open files it doesn't name; do not do more than it says.
>
> <Card-specific cautions: the 2–3 ways THIS card most likely goes wrong, and what invariant to
> protect. E.g. "the hash must not change — if it moves, your refactor changed behavior: stop.">
>
> When the gate passes: flip the card, one worklog line, stop — the next card gets a fresh chat.
> If any escalate-if triggers: STOP, record the mismatch in the worklog, report. Never
> `git commit`/`push`; never add co-author trailers; <repo's never-do list, compressed>.

The state line always carries **numbers the executor can check** (test count, hashes) — a fresh
session's first act is confirming the world matches the handoff.

## 6. Planner verification (every executor report)

Never relay an executor's claims forward unverified. On each report:

1. `git status --porcelain` — file scope matches the card exactly; nothing stray.
2. Re-run the full gate **yourself** (fmt, lint, full test count, determinism hashes ×2).
3. Spot-check the card's specific claims (grep for the forbidden call, check the status flips,
   confirm the decision-log entry landed where claimed).
4. Stage **explicit paths only** — never `-A`, never another session's in-flight files, never a
   red tree.
5. Draft the commit message (what + why + gate results + "Next: <card>"). No AI-attribution
   trailers. The operator commits.
6. Write the next handoff.

## 7. The adversarial review recipe

Run as a multi-agent workflow (cheap model, e.g. `model: 'sonnet'` on every agent):

1. **Lenses** — 4–6 parallel reviewers, each with ONE angle (correctness, determinism,
   invariant-scope, spec-conformance, test-quality/mutation, …). Each is told: findings need
   file:line evidence; your output feeds a program.
2. **Skeptics** — 2 independent agents per finding, prompted to *refute* it with evidence. A
   finding survives only if not refuted. This kills the plausible-but-wrong majority.
3. **Decision rule, fixed in advance:** confirmed blocker/major → **STOP, record, do not
   proceed** (the planner rules on scope); confirmed minor → record; fix only if ≤10 lines of
   *test* code; zero confirmed → note "review: PASS (N agents)" in the worklog and proceed.
4. One lens should always be **plan-conformance**: does every deliverable the authoritative plan
   names map to a *real artifact* — not to a sentence in a checklist that quietly reads the
   deliverable down?

## 8. Escalation rulings (the planner's two moves)

When an executor or review stops the line, the planner has exactly two honest options:

- **Fix the thing.** If the finding is a real defect (in machina: a zero-cost scenario reporting
  nonzero slippage), write a new card that fixes the cause — **never weaken the assertion** that
  caught it. The fix card names the off-limits file explicitly and bounds the change to the
  character.
- **Descope on the record.** If the artifact honestly satisfies the authoritative plan and only a
  derived document over-promised, amend the derived document AND write a decision-log entry saying
  what was descoped, where it moved, and why (in machina: per-family aggregation rows → the next
  milestone's ranking deliverable, D-0011). A descope without a record is the failure mode; the
  record is what makes it legitimate.

Both moves end the same way: the interrupted card re-runs its gate fresh. Declarations rest on
freshly-run commands, never on prior claims.

## 9. Git discipline

- Agents **never** commit or push. The operator commits, between cards.
- Stage explicit paths per unit of finished work; suggested commit messages carry no
  `Co-Authored-By`/AI trailers.
- Never stage a tree with a failing test; blocked work stays uncommitted until its unblock card
  turns it green.
- Escalated/blocked states are visible in the queue (`BLOCKED` + pointer to the worklog entry),
  so a fresh session inherits the truth.

## 10. Model economics

- **Planner:** strongest model you have, max effort. It's one long session doing audit, design,
  verification, and rulings — the leverage point.
- **Executors:** a strong-but-cheaper model, default/high effort, one fresh context per card. The
  cards were designed so brilliance is unnecessary; freshness and literalism are what you're buying.
- **Review swarms:** same cheaper model, many agents. Breadth beats depth here; the skeptic pass
  compensates for individual shallowness.
- Do **not** give executors multi-agent orchestration — it invites improvisation, which is exactly
  what cards exist to prevent. Save the swarms for review.

## 11. What this prevents (all observed, not hypothetical)

- **Plan drift** — status files claiming "not started" for shipped code, stale test counts,
  "uncommitted" work that was committed days ago. (Fixed by step 0's drift table.)
- **Underspecified tasks** — "wire up the CLI" hiding three missing subsystems. (Card-writing
  forces the planner to read the real code first; the gaps become their own cards.)
- **Deliverables satisfied on a technicality** — a re-exported type with zero callers standing in
  for "reporting". (Caught by the plan-conformance lens; fixed, not excused.)
- **Latent defects surfaced by new visibility** — a ±1-ulp money residue invisible until a card
  exported the field. (Escalated per protocol; the assert stayed, the accounting got fixed.)
- **Non-discriminating tests** — a regression test whose literals pass even against the bug.
  (Caught by a skeptic; red→green verified on the replacement.)
- **Self-grading** — the session that wrote the code declaring its own milestone. (The declaration
  card runs a fresh battery and is itself preceded by an independent review with authority to stop
  it. It stopped it twice. Both stops were right.)
