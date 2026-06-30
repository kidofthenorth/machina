# Plan: <change name>

> One change per plan. One plan per session. Keep the diff small enough to review in one sitting.

## Goal
<One sentence. What does "done" look like from the outside? If you can't say it in a sentence, the change is too big — split it.>

## Context check (agent does this FIRST, before writing any code)
Before implementing, verify this plan against the real codebase and report back:
- [ ] Do the files/functions named below actually exist and look as assumed?
- [ ] Are there existing utilities or patterns I should reuse instead of writing new ones?
- [ ] Does anything here conflict with current behavior, types, or tests?
- [ ] List any assumption in this plan that turned out to be wrong.

Do not start implementing until these are confirmed or the plan is corrected.

## In scope (files this change is allowed to touch)
- `path/to/file`
- ...

## Out of scope (do NOT touch)
<Be explicit. Migrations? Config? Unrelated refactors? Name them so they don't drift in.>
- ...

## Steps (each step = one small, reviewable diff)
1. ...
2. ...
3. ...

## Definition of done
- [ ] <Testable criterion 1>
- [ ] <Testable criterion 2>
- [ ] Tests pass: `<command>`
- [ ] No changes outside "In scope"

## Constraints
- Implement this plan exactly as written. Do not add anything not listed above. If something necessary is missing, stop and update this plan first.
- Preserve: <public APIs / signatures / behavior that must not change>

## Progress (checkpoint here before any context reset)
- Done so far: ...
- Next step: ...
- Files modified: ...

## Review lenses (run after implementation, each in a fresh session)
- [ ] Correctness
- [ ] Security
- [ ] Tests & edge cases
