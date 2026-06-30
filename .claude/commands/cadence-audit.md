---
description: Deep multi-agent audit of the cadence + context system — the adversarially-verified companion to /context-health
---

# /cadence-audit

The **deep** counterpart to `/context-health`. Where context-health is rung one (structural: stale
markers, dead globs, missing notes), `/cadence-audit` runs the higher rungs a single structural pass
can't see — and refutes every finding before it counts.

**Run the `cadence-audit` workflow:** call the Workflow tool with `{ name: "cadence-audit" }`.

It fans out seven independent review lenses over the dropped-in `.claude/` kit and this repo's own
context, has an adversarial skeptic try to **refute** each finding (default to "not a defect"), then
synthesizes a prioritized, de-duplicated verdict — `bulletproof` / `minor-gaps` / `real-gaps` — with
one fix per item. The lenses: cross-reference integrity · convention consistency (the marker never-do;
conventions defined once) · model coherence (one consistent story) · init-cadence robustness (green
in one pass across repo shapes) · context-health robustness (no false positives/negatives) ·
install/distribution story · leanness.

**Report-only** — like `/context-health`, it edits nothing; it hands you the list to act on.

**Heavy by design:** it spawns ~20–40 subagents and runs for a few minutes. Use it after a batch of
cadence changes, before fanning the kit out to more repos, or when `/context-health` is green but you
want semantic assurance. It needs multi-agent orchestration enabled — running this command is your opt-in.
