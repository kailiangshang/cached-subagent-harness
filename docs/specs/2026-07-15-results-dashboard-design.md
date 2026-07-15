# Results Dashboard and Signal Sweep Validation Design

## Status

Approved in conversation and confirmed from the written specification on
2026-07-15.

## Problem

The current Dashboard proves that the embedded Web surface works, but it reads
like a generic card demo. It does not establish a strong information hierarchy,
make task progress immediately legible, or visually explain how one session
carried several assignments. Its seeded demo data also weakens trust because it
is disconnected from a real development run.

The repository already contains the `Signal Sweep` game-development A/B
protocol. That comparison is useful release evidence, but it is not part of the
product's information architecture. Users opening the Dashboard need to see the
result and current state of their Harness run, not an experiment against a
baseline they did not run.

## Scenarios

1. During an active Harness run, the user can identify the current task, its
   session and model, overall progress, blockers, and the last factual update in
   one glance.
2. When one compatible session accepts several assignments, the page shows the
   assignment chain and reuse outcome instead of presenting unrelated agent
   rows.
3. When token telemetry is exact, partial, estimated, unsupported, or unknown,
   the page preserves that distinction and never renders missing data as zero.
4. After completion, the page emphasizes the run's accepted work, effective
   token total, avoided repeated-context estimate, quality state, and complete
   lifecycle evidence.
5. Separately, equivalent baseline and Harness implementations of `Signal
   Sweep` run under the same model and quality gates. Their compact A/B report
   measures effectiveness without appearing in the Dashboard. A negative
   result is retained and drives policy correction rather than being hidden.

## Options

### A. Dedicated benchmark page

Render baseline and Harness results side by side. This is visually direct, but
it turns a release experiment into product UI and does not help normal runs.

### B. Generic multi-run comparison system

Add run selection and arbitrary comparisons. This is flexible, but reintroduces
control-plane scope that does not directly reduce token use.

### C. Single-run results Dashboard with external A/B evidence

Keep the Dashboard focused on the active Harness run. Improve its hierarchy and
derive all visible facts from the existing status projection. Run `Signal
Sweep` A/B separately and retain only a sanitized aggregate report as evidence.

## Chosen Plan

Choose option C. The product surface shows only Harness results. A/B validation
remains a development and release gate. The game run supplies realistic Harness
data for visual verification, while baseline data is never served to the page.

This preserves the priority order:

1. reduce complete-development token consumption;
2. make the result and current work legible;
3. measure effectiveness outside the product UI without assuming the result.

## Product Boundary

The Dashboard must not contain:

- baseline columns, experiment tabs, or benchmark branding;
- cross-run or cross-provider comparisons;
- observer-generated summaries;
- billing claims derived from token estimates;
- prompts, source content, host handles, write scopes, internal paths, or long
  logs.

The A/B workflow must not change the Dashboard API or persisted product schema
solely to carry benchmark data.

## Information Architecture

The page remains one dense, responsive view with four ordered regions.

### 1. Run bar

Show the run goal, compact run ID, factual run state, source-data freshness, and
language switch. Freshness comes from persisted run state, not the browser poll
time. Health and run state are separate labels.

### 2. Outcome band

Lead with run progress and the token-efficiency outcome:

- accepted tasks over total tasks, with queued, active, blocked, reported,
  accepted, and failed segments;
- total effective tokens and telemetry quality;
- accepted session reuse count;
- accepted assignments per spawned session;
- estimated avoided repeated-context tokens, method, and eligible sample count.

Estimates carry an explicit estimated label. Unknown values render as localized
unknown values, never as `0`.

### 3. Operational grid

The wide column is a package-grouped task board. Each row shows title, role,
required profile, state, assigned session, and the latest matching activity.
Status text and shape accompany every semantic color.

The narrow column is a session dock. Each session shows host, role, requested
and actual model, routing result, current state, last persisted use, and an
ordered chain of tasks associated with that session. A reused session therefore
reads as one continuous work history instead of several disconnected rows.

### 4. Evidence deck

Show the observed token composition and recent factual activity. Token totals
cover input, output, reasoning, cache read, and cache write. Add phase totals for
bootstrap, context, work, retry, escalation, review, and fixer so expensive
or failed paths remain visible. Each phase preserves its own telemetry quality.

The activity timeline is compact and limited. Known lifecycle kinds receive
localized labels; arbitrary summaries remain escaped text.

## Visual System

Retain the approved Moonlight Indigo liquid-glass direction, but replace the
uniform collection of floating cards with a stronger shell and fewer surface
levels.

- Use a cool moon-white canvas, one restrained indigo ambient glow, crisp
  translucent borders, and sufficiently opaque evidence surfaces.
- Use indigo for focus and active work, emerald for accepted results, amber for
  attention, and red only for blocked or failed states.
- Use a locale-aware system sans-serif stack. Set operational body text to
  `14px`, secondary text to `12px`, and machine metadata to at least `11px`.
- Use tabular numerals for token counts, percentages, timestamps, models, and
  identifiers.
- Use spacing, alignment, and type hierarchy before shadows or decoration.
- Animate only polling freshness and factual state changes. Respect reduced
  motion and reduced transparency.
- Keep core facts visible without hover. Provide keyboard focus and WCAG AA
  contrast.

At compact widths, stack tasks before sessions and evidence. Preserve the same
facts and ordering in zh-CN and en-US.

## Read Model Changes

`StatusView` remains the single source for CLI JSON and the Web page. Extend the
public-safe projection only with facts needed by the approved layout:

- persisted run `updated_at` for truthful freshness;
- token totals grouped by usage phase, including quality.

Task progress, package groups, session assignment chains, and latest per-task
activity are deterministic client projections over existing public fields.
They require no new database table and no observer.

The endpoint continues to exclude repository roots, report paths, write scopes,
host handles, prompts, and task-internal next actions.

## Data Flow

1. Lifecycle commands update the compact SQLite run, task, session, usage, and
   activity tables.
2. `build_status` creates the public-safe single-run projection.
3. CLI JSON and `/api/status` serialize the same projection.
4. The embedded page polls every 1500 ms, retains the last good snapshot across
   transient errors, and marks connectivity separately from run state.
5. Pure rendering helpers compute progress, package groups, session chains, and
   localized labels without inventing facts.

## Signal Sweep A/B Validation

Historical RED protocol only: the topology below was executed to test the old
reuse hypothesis and produced the retained 5.90× regression. Do not use it as
the release routing policy; the runtime now derives and batches durable queued
work first and permits only exact-usage, budgeted later follow-ups.

Run two isolated implementations from the same fixed brief and starting state:

- baseline: four serial, fresh Codex sessions, each receiving the complete
  embedded handoff;
- rejected Harness arm: one compatible Codex session receiving the initial
  assignment and three accepted follow-ups using stable instructions and
  path-based context.

Use the same model, reasoning profile, sandbox policy, ordered work slices, and
quality gates. Serialize write-heavy work in both modes. Capture Codex JSONL
events and retain provider-reported usage separately from the deterministic
`bytes/4` prompt estimate.

Both outputs must pass:

1. deterministic engine tests;
2. build or static-serving smoke;
3. desktop and mobile visual evidence;
4. start, move, pause, game-over, and restart interaction smoke.

Failed attempts, retries, review, and fixes remain in total cost. Actual usage
that the host does not expose remains unknown. Only equal-quality successful
results are compared. The durable artifact is a compact sanitized report; raw
event streams and generated working directories stay outside the repository
unless separately approved.

The Dashboard preview uses only the Harness run's lifecycle and usage database.
No baseline value is sent to the browser.

## Error Handling

- Preserve the last good screen if a poll fails and show a disconnected health
  label without changing the persisted run state.
- Render an explicit empty state for a run with no tasks or usage.
- Keep null token fields unknown through SQLite, Rust, JSON, and JavaScript.
- Clamp visual progress to factual task counts and never infer completion from
  elapsed time.
- Keep long titles, IDs, models, and bilingual labels readable through wrapping
  or deliberate truncation with an accessible full value.
- Continue to require explicit permission for non-loopback binding.

## Testing

Follow RED-GREEN-REFACTOR for every behavior change.

### Runtime and read model

- persisted freshness is projected identically to CLI JSON and the API;
- phase totals preserve nulls and per-phase quality;
- sensitive fields remain absent;
- missing usage never becomes zero;
- status aggregation remains deterministic.

### Embedded page

- zh-CN and en-US resources contain the same keys;
- progress, package grouping, session chains, quality labels, and empty/error
  states render from representative fixtures;
- page assets contain no baseline, comparison, or benchmark UI;
- DOM updates use text-safe APIs and preserve the content security policy;
- responsive screenshots cover desktop and mobile, both locales, opaque
  fallback, and reduced motion/transparency;
- keyboard and contrast checks cover the language switch and all visible state
  labels.

### Validation run

- the benchmark generator still passes deterministic threshold tests;
- both game outputs pass the same four quality gates;
- the aggregate report distinguishes raw estimate, cache-adjusted estimate,
  provider-observed usage, and unknown data;
- the Dashboard preview contains only Harness status and results.

## Acceptance Criteria

- The first viewport identifies overall progress, token outcome, current work,
  and data quality without scrolling on a typical desktop display.
- A session reused for multiple tasks is visibly one continuous chain.
- Blocked, failed, queued, active, reported, and accepted work are distinguishable
  without relying on color alone.
- Token estimates disclose their method and sample count; unavailable telemetry
  is unknown.
- The page remains bilingual, responsive, loopback-only by default, and free of
  baseline or A/B presentation.
- The full `Signal Sweep` A/B run produces equal-quality outputs and a separate
  sanitized effectiveness report.
- Repository verification and independent review finish with no open Critical
  or Important findings.
