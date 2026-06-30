// cadence-audit — the deep, adversarially-verified companion to /context-health.
// Fans out independent review lenses over the dropped-in `.claude/` kit + this repo's context,
// refutes every finding with a skeptic before it counts, and synthesizes a prioritized verdict.
// Report-only: it edits nothing. Runs in any repo that has the cadence (relative paths).
// Optional args: { root: "<repo root>" } — defaults to "." (the current repo).

export const meta = {
  name: 'cadence-audit',
  description: 'Deep multi-agent audit of the cadence + context system: 7 lenses, each finding adversarially verified, synthesized into a prioritized verdict. Report-only.',
  whenToUse: 'After a batch of cadence/context changes, before fanning the kit out to more repos, or when /context-health is green but you want semantic assurance.',
  phases: [
    { title: 'Review', detail: '7 independent lenses over the kit + context' },
    { title: 'Verify', detail: 'adversarially refute each finding' },
    { title: 'Synthesize', detail: 'dedup + prioritize + verdict' },
  ],
}

const root = (args && args.root) || '.'

const FILES = `${root}/.claude/VERSION, ${root}/.claude/skills/*/SKILL.md, ${root}/.claude/commands/*.md, ${root}/.claude/templates/*.md, ${root}/.claude/guide/context.md, ${root}/.claude/checklists/review.md, ${root}/.claude/workflows/*.js, and the repo's own context: ${root}/README.md, ${root}/AGENTS.md, ${root}/CLAUDE.md (plus any nested CLAUDE.md)`

const DIMENSIONS = [
  { key: 'xref', lens: 'Cross-reference & path integrity', detail: 'Every path / file / section-heading / command referenced across the kit and the repo\'s own context must resolve on disk. No dangling @imports, no broken internal markdown links, no command or skill pointing at a missing or renamed file/heading.' },
  { key: 'conventions', lens: 'Convention consistency & the marker never-do', detail: '(1) The Describes:/Last verified: marker format must be byte-identical between .claude/templates/module-claude-md.md and .claude/commands/context-health.md (a hard never-do). (2) Any cross-file convention — e.g. the (confirm) tag and the "Module context:" declaration — must be DEFINED ONCE (in guide/context.md) and merely REFERENCED elsewhere; flag a second definition or divergent wording. (3) Enumerated lists that appear in more than one file (e.g. the context-health finding categories) must agree.' },
  { key: 'coherence', lens: 'Model coherence — one consistent story', detail: 'No two files may give contradictory guidance. Hunt for conflicts such as "drafts a human fixes later / not gospel" vs "verify-then-write, no loose ends", "never edit .gitignore" vs "ask then apply a one-line fix", or (confirm) framed as normal output vs last-resort fallback. The kit must read as one coherent system.' },
  { key: 'init', lens: 'init-cadence robustness — green in one pass on ANY repo', detail: 'Walk the init-cadence skill against repo shapes and find where it would NOT come up green in one pass or would do the wrong thing: tiny repo; repo with pre-existing mature AGENTS.md/CLAUDE.md (must refine-not-clobber); .gitignore that swallows CLAUDE.md or kit files; docs-routed repo (does it write the Module context: line?); no-git repo; monorepo/multi-package; non-interactive run (can it neither verify nor ask?). Are all branches specified, or left to improvisation?' },
  { key: 'health', lens: 'context-health robustness — no false positives/negatives', detail: 'Does the audit correctly handle every case: stale (incl. the same-day ambiguity rule), missing (vs a declared "Module context:" strategy), broken markers/globs/@imports, unverified (the (confirm family, bold-tolerant grep), no-git repos, and a root CLAUDE.md that is NOT a cadence @AGENTS.md shim (a repo with its own hand-written CLAUDE.md)? Find concrete false-positives or false-negatives it would emit.' },
  { key: 'install', lens: 'Install & distribution story', detail: 'Does "copy the .claude folder in" work when the TARGET REPO ALREADY HAS a .claude/ (settings, custom commands, worktrees)? Is merge-not-replace / no-nesting guidance present anywhere, or is it a gap a user falls into (ending up with .claude/.claude, or a file-manager "Replace" deleting their settings)? Assess the hidden-dotfolder distribution friction and the re-copy/update path. Missing guidance is a real defect.' },
  { key: 'lean', lens: 'Leanness & altitude', detail: 'The ethos is lean — "add paragraphs not a framework; refine, don\'t gut; keep it short, it\'s always in the window." Flag GENUINE bloat, redundancy, or over-specification: two files saying the same thing divergently, a section grown long enough to defeat its purpose. Only real altitude problems, not stylistic nitpicks.' },
]

const FINDINGS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        properties: {
          title: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor', 'nit'] },
          files: { type: 'array', items: { type: 'string' } },
          evidence: { type: 'string', description: 'exact quote / file:line proving it' },
          why: { type: 'string' },
          fix: { type: 'string' },
        },
        required: ['title', 'severity', 'files', 'evidence', 'why', 'fix'],
      },
    },
    dimension_verdict: { type: 'string' },
  },
  required: ['findings', 'dimension_verdict'],
}

const VERDICT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    real: { type: 'boolean' },
    confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
    corrected_severity: { type: 'string', enum: ['blocker', 'major', 'minor', 'nit', 'invalid'] },
    reasoning: { type: 'string' },
  },
  required: ['real', 'confidence', 'corrected_severity', 'reasoning'],
}

const REPORT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    verdict: { type: 'string', enum: ['bulletproof', 'minor-gaps', 'real-gaps'] },
    summary: { type: 'string' },
    prioritized: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        properties: {
          title: { type: 'string' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor', 'nit'] },
          files: { type: 'array', items: { type: 'string' } },
          fix: { type: 'string' },
        },
        required: ['title', 'severity', 'files', 'fix'],
      },
    },
  },
  required: ['verdict', 'summary', 'prioritized'],
}

const reviewPrompt = (d) => `You are a meticulous, skeptical reviewer auditing the "context-loop" cadence — a drop-in \`.claude/\` folder plus root docs (AGENTS.md / CLAUDE.md) that a repo uses to keep agent context honest. Repo root: ${root}

Your SINGLE lens: ${d.lens}.
Scrutinize specifically: ${d.detail}

Read the actual files (Read/Grep/Bash under ${root}). Relevant files: ${FILES}.

Report ONLY real, defensible defects within your lens — things that would actually bite a user or break the goal "bulletproof: works on ANY repo via copy-paste + /init-cadence, green in one /init-cadence pass + one /context-health pass." Cite exact file + line/quote as evidence and propose a concrete fix. If your lens is clean, return findings: [] and say so in dimension_verdict. Do NOT inflate nitpicks into majors or invent issues to look thorough.`

const verifyPrompt = (d, f) => `Adversarially verify ONE review finding about the context-loop cadence at ${root}. Your job is to REFUTE it if you can.

Finding (lens: ${d.lens}):
- Title: ${f.title}
- Claimed severity: ${f.severity}
- Files: ${(f.files || []).join(', ')}
- Evidence: ${f.evidence}
- Why it matters: ${f.why}
- Proposed fix: ${f.fix}

Open the actual files and check: (a) is the evidence ACCURATE — quotes/line refs real, not misremembered? (b) is it genuinely a defect, or does another part of the kit already handle it / is it intended-by-design? (c) is the claimed severity right? Default to real=false if you cannot substantiate it against the files. Set corrected_severity='invalid' if it is not a real issue. Cite what you actually saw.`

const synthPrompt = (confirmed) => `Synthesize the confirmed findings of a multi-agent fresh-eyes audit of the context-loop cadence. Each finding below already survived adversarial verification. Produce a prioritized, DE-DUPLICATED report and an overall verdict on whether the cadence meets its bar: "bulletproof — works on ANY repo via copy-paste + /init-cadence, green in one /init-cadence pass + one /context-health pass."

Confirmed findings (JSON):
${JSON.stringify(confirmed, null, 2)}

Merge overlapping findings, order by real-world impact (blocker → nit), and give each a crisp one-line fix. verdict = 'bulletproof' if no real gaps, 'minor-gaps' if only minor/nit, 'real-gaps' if any major/blocker survives.`

phase('Review')
const reviewed = await pipeline(
  DIMENSIONS,
  (d) => agent(reviewPrompt(d), { label: `review:${d.key}`, phase: 'Review', schema: FINDINGS_SCHEMA, effort: 'high' }),
  (review, d) => parallel(((review && review.findings) || []).map((f) => () =>
    agent(verifyPrompt(d, f), { label: `verify:${d.key}`, phase: 'Verify', schema: VERDICT_SCHEMA, effort: 'high' })
      .then((v) => ({ ...f, dimension: d.key, verdict: v }))
      .catch(() => null)
  ))
)

const all = reviewed.flat().filter(Boolean)
const confirmed = all
  .filter((f) => f.verdict && f.verdict.real && f.verdict.corrected_severity !== 'invalid')
  .map((f) => ({ title: f.title, severity: f.verdict.corrected_severity, dimension: f.dimension, files: f.files, evidence: f.evidence, why: f.why, fix: f.fix, confidence: f.verdict.confidence }))

log(`${all.length} raw findings → ${confirmed.length} survived adversarial verification`)

phase('Synthesize')
const report = await agent(synthPrompt(confirmed), { label: 'synthesize', phase: 'Synthesize', schema: REPORT_SCHEMA, effort: 'high' })

return { rawFindings: all.length, confirmedCount: confirmed.length, confirmed, report }
