# Review checklist — one lens at a time

**Run each pass in a FRESH session (or a dedicated subagent), reviewing one lens only.**
A session that wrote the code — or that just reviewed it for something else — will agree with itself. A clean session with a single job is the whole point.

For each pass: load the diff + the plan file, run only that lens, write findings tied to file/line, then stop.

---

## Pass 1 — Correctness
- [ ] Does the change actually satisfy the plan's Definition of Done?
- [ ] Does the logic do what it claims? Walk the happy path end to end.
- [ ] Boundary conditions: empty, zero, null, max, off-by-one.
- [ ] Error paths handled — not just the success case?
- [ ] Any behavior changed that the plan said to preserve?

## Pass 2 — Security
- [ ] Untrusted input validated and sanitized before use?
- [ ] Injection surfaces (SQL, shell, path, template, HTML) safe?
- [ ] Secrets and tokens not logged, hardcoded, or committed?
- [ ] AuthZ/AuthN checks present where they need to be?
- [ ] New dependencies trustworthy and pinned?

## Pass 3 — Tests & edge cases
- [ ] New behavior covered by tests?
- [ ] Failure modes tested, not just the happy path?
- [ ] Would the tests actually fail if the code regressed?
- [ ] Edge cases from Pass 1 represented in tests?

## Pass 4 (optional) — Scope & diff hygiene
- [ ] Diff stayed inside the plan's "In scope" list?
- [ ] No drive-by refactors, formatting churn, or unrelated changes?
- [ ] Diff small enough to review in one sitting? If not, that's the finding — split it.
