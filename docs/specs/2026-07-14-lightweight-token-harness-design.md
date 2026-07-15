# Lightweight Token-Efficiency Harness Design

## Status

Implemented and current for the lightweight core. The 2026-07-15 results
Dashboard design refines the presentation section, and the real Signal Sweep
evidence replaces the original repeated-follow-up hypothesis with the
batch-first, budget-bounded contract already incorporated below.

The approved direction replaces the over-designed control-plane architecture
with a small standalone harness whose primary outcome is lower total effective
token use at unchanged quality. The Dashboard is mandatory, but it remains a
read-only view over the same compact state used by the CLI.

This design supersedes the implementation scope and delivery boundaries in
`2026-07-10-agent-control-plane-design.md`. The earlier document remains history,
not an implementation requirement. The standalone principles completed in
increment 1 remain binding.

Live evidence correction (2026-07-15): the original example of one Session
plus five sequential follow-ups is invalid. A real four-assignment Signal Sweep
run consumed 5.90× the equal-quality Baseline because resumed context grew on
every turn. The corrected contract below is batch-first and budget-bounded; the
negative run remains the RED evidence.

## Problem

Agentic development wastes tokens when controllers repeatedly create short-lived
agents, reload the same repository context, split compatible work into tiny
assignments, or route every task to the most capable model. Users also cannot
easily see which tasks are active, which sessions were reused, or whether a
claimed saving includes retries, review, and fixer work.

The previous design attempted to solve this with a full event-sourced control
plane: 67 event kinds, deterministic replay, field-level provenance, leases,
capability scanners, adapter frameworks, probes, bridges, and an observer. That
complexity does not directly improve token efficiency and delayed the first
usable status view.

## Product Priority

The product optimizes one objective:

> Minimize total effective token use while preserving the same development and
> review quality gates.

The following mechanisms are core and unchanged:

1. Combine compatible related tasks into one bounded work package.
2. Reuse a compatible agent session only for later work that was not available
   to the original batch and remains inside explicit follow-up and Token caps.
3. Route `light`, `standard`, and `deep` work to the lowest safe profile.
4. Count bootstrap, context reload, useful work, retry, escalation, review, and
   fixer tokens together.
5. Never report missing telemetry as zero or an unsupported model request as
   applied.
6. Keep independent review and complete-development gates; savings may not be
   manufactured by skipping required work.

Visualization is a required supporting feature. It explains and verifies the
core outcome but does not control execution.

## Preserved Standalone Principles

- Standalone is the default. Superpowers is optional and never required at
  startup or installation.
- PSOC precedes implementation.
- Behavior changes are test-first.
- Write-heavy work with overlapping scope is serial.
- Writers and fixers cannot review their own work.
- Critical and Important findings are fixed before acceptance.
- Stable prompts pass task-specific data by path.
- Known sessions have explicit lifecycle closure or a truthful terminal reason.
- Requested and actual host/model behavior remain separate facts.
- Unknown values remain unknown.

## Non-goals

The lightweight harness does not implement:

- a general event-sourcing platform;
- the previous 67-event registry;
- deterministic event replay;
- field-level provenance or conflict resolution;
- a separate lease entity or lease event machine;
- a capability-scanner subsystem;
- a provider adapter framework;
- active probes;
- a desktop bridge;
- an MCP event service;
- a permanent observer agent;
- cross-provider token or price comparison without explicit compatible data.

## Architecture

The system has three layers.

### 1. Skill policy

`SKILL.md` and its compact references teach the controller when to execute on
main, batch tasks, reuse a session, or spawn a session. They define quality
floors, model profiles, token accounting, report contracts, and lifecycle
cleanup. They contain no provider-specific model names.

### 2. Lightweight `harnessctl`

One small Rust binary provides six focused modules:

- `bundle`: groups compatible related tasks;
- `sessions`: atomically claims or creates compatible sessions;
- `routing`: selects `light`, `standard`, or `deep` from explicit floors;
- `accounting`: totals observed work and estimates avoided overhead honestly;
- `hosts`: renders fixed host command templates;
- `status` and `dashboard`: expose the same read-only projection.

SQLite remains because it provides atomic session claims and concurrent reads in
one dependency-free local file. Current-state tables are authoritative. A small
activity log exists for user visibility and debugging only; deleting it does not
change current state, and it is never replayed to reconstruct state.

### 3. Mandatory presentation

The binary provides terminal and Web views:

```text
harnessctl status [--json]
harnessctl watch [--interval-ms 1500]
harnessctl dashboard [--bind 127.0.0.1] [--port 0]
```

The Web assets are embedded in the binary. There is no Node service or frontend
framework. The page polls a JSON endpoint every 1500 ms. It is read-only and
binds to loopback by default.

## Compact Data Model

### `runs`

```text
run_id, goal, status, repo_root, report_path,
created_at, updated_at, ended_at
```

Statuses: `active`, `complete`, `failed`, `cancelled`.

### `tasks`

```text
task_id, run_id, package_key, title, role,
complexity, risk, uncertainty, write_scope,
required_profile, status, session_id,
attempt_count, next_action, created_at, updated_at, ended_at
```

Statuses: `queued`, `running`, `blocked`, `reported`, `accepted`, `failed`,
`cancelled`.

### `sessions`

```text
session_id, run_id, host, handle, role, profile,
requested_model, actual_model, routing_status,
package_key, scope_hash, repo_revision, review_boundary,
status, current_task_id, reuse_count,
created_at, last_used_at, ended_at, final_reason
```

Statuses: `starting`, `busy`, `idle`, `closed`, `failed`, `unknown`.

There is no lease table. The reusable-session signature is stored directly on
the session row:

```text
host + role + profile + package_key + scope_hash +
repo_revision + review_boundary
```

Reuse requires an exact signature match and an `idle` session. Claiming the
session changes it to `busy` inside one `BEGIN IMMEDIATE` transaction. A
successful verified checkpoint may update `repo_revision` before the session
returns to `idle`. An unverified report, changed scope, role/profile change,
review boundary, or unexpected revision closes the reusable path.

### `usage`

```text
usage_id, run_id, task_id, session_id, phase,
input_tokens, output_tokens, reasoning_tokens,
cache_read_tokens, cache_write_tokens,
source, quality, observed_at
```

Phases: `bootstrap`, `context`, `work`, `retry`, `escalation`, `review`,
`fixer`.

Every numeric field is nullable. `quality` is `exact`, `partial`, `estimated`,
`unsupported`, or `unknown`.

### `activity`

```text
activity_id, run_id, task_id, session_id,
kind, summary, occurred_at
```

Kinds are intentionally small: `plan`, `batch`, `spawn`, `reuse`, `route`,
`start`, `block`, `report`, `accept`, `fail`, `close`. This is an audit feed,
not a state machine.

## Task Bundling

Related ready tasks are combined only when all of these match:

- role;
- required profile;
- risk class;
- package or code area;
- write scope;
- repository revision;
- review boundary;
- dependency order.

Batching produces one bounded brief containing multiple ordered assignments. It
does not create an unrestricted permanent agent. A task with different risk,
scope, role, profile, revision, or independence requirement remains separate.

## Session Reuse

For each ready task, the controller chooses:

```text
trivial and no isolation needed -> execute_on_main
compatible related tasks exist  -> batch_then_spawn
compatible idle session within both budgets -> reuse_session
delegation benefit exceeds cost -> spawn_session
otherwise                        -> execute_on_main
```

The harness returns the decision and the exact reasons. It never calls a host by
itself without an explicit controller command. After a host accepts a follow-up,
the controller records the reuse; rejected follow-up does not increment
`reuse_count` but its consumed Tokens remain retry cost.

Known compatible ready work is always derived from durable queued state and
batched before follow-up reuse; a caller-supplied count is not authoritative.
Each reusable Session has an accepted-follow-up cap and an observed
total-effective Token cap. Runtime defaults are one follow-up and 200,000
Tokens. Flags may lower these release defaults but cannot raise them; a future
increase requires a versioned durable evidence policy. Missing, non-exact, or
non-normalizable usage, either exhausted cap, or a changed signature makes the
Session ineligible. Only complete exact usage linked to the current assignment
can release it to idle. Release also requires the task's durable accepted state
and usage strictly after the acceptance transaction's causal boundary; wall
clock equality is not evidence. Usage run/task/session ownership must agree. A
queued task may refresh a still-valid base revision only through a
compare-and-swap update while unassigned.

## Model Routing

The route is the maximum of four explicit floors:

```text
required_profile = max(complexity, risk, role, uncertainty)
```

Provider-neutral profiles are:

- `light`: bounded read-only search, extraction, and formatting;
- `standard`: scoped implementation, testing, and ordinary analysis;
- `deep`: architecture, ambiguous multi-step work, control-plane changes, and
  high-risk review.

Security-sensitive, destructive, control-plane, and final architecture review
work has a `deep` floor. A manual override may raise but not silently lower the
floor.

Host templates map profiles to command arguments. If the host cannot set a
model, the route records `unsupported` and actual model remains `unknown`.

## Host Command Templates

Host support is configuration, not a subsystem. Each template contains:

```text
name
spawn_command
followup_command
close_command
profile_arguments
```

Codex, Claude, and OpenCode ship with fixed templates covered by snapshot tests.
Other platforms that expose equivalent Skill and agent commands can supply the
same JSON template. No scanner, bridge, or adapter class is required.

Command rendering is data-only: arguments are returned as an array and are not
evaluated through a shell. Missing placeholders, unsupported follow-up, and
unknown profiles fail clearly.

## Token Accounting

`observed_total_tokens` sums every known token field across all phases. It never
coerces null to zero. A run is marked `partial` when any required observation is
missing.

Provider totals are normalized into non-overlapping categories before storage.
For Codex CLI JSONL, cached input is removed from `input_tokens` and stored as
`cache_read_tokens`; reasoning output is removed from `output_tokens` and stored
as `reasoning_tokens`. The CLI exposes no additional cache-write counter, so
that source contributes zero to the separate category rather than duplicating
input. Missing split fields remain unknown.

Primary efficiency metrics:

```text
assignments_per_spawn = accepted delegated tasks / spawned sessions
churn_rate = spawned sessions / completed delegated tasks
reuse_count = accepted follow-up assignments
total_effective_tokens = bootstrap + context + work + retry +
                         escalation + review + fixer
```

Estimated reuse savings are shown only after at least three exact bootstrap or
context observations for the same host/profile:

```text
estimated_saved_tokens = avoided_spawns * median_observed_overhead
```

The estimate displays its sample count and `estimated` label. Model-routing
cost savings require explicit compatible price mappings; otherwise the harness
reports route counts and raw tokens without a currency-saving claim.

A token-saving comparison is valid only when both paths satisfy the same
quality gates. Failed quality runs remain part of total cost and cannot be used
as the cheaper successful result.

## CLI Data Flow

1. `init` creates a run database.
2. `task add` records ready work and its floors.
3. `task refresh-revision` compare-and-swaps a still-valid queued task after a
   verified checkpoint when needed.
4. `decide` derives compatible queued work and returns main, batch, reuse, or
   spawn with reasons and lower-only Session budget flags.
5. `session record` stores the actual host handle and requested/actual model.
6. `task update` records progress, report, acceptance, or failure.
7. `usage add` atomically validates ownership and records exact normalized
   assignment usage after durable follow-up acceptance and before
   release/reuse.
8. `status`, `watch`, and `dashboard` read the same current-state queries.
9. `audit` rejects unfinished tasks, nonterminal Sessions, or terminal Sessions
   that retain a current assignment.

Existing prompt rendering and cache checks remain available.

## Dashboard Design

The established visual baseline remains:

- Moonlight Indigo palette;
- restrained liquid-glass surfaces;
- system sans-serif;
- zh-CN and en-US;
- 14 px operational body text, 12 px secondary text, and metadata no smaller
  than 11 px;
- dense Command Grid layout.

The first complete page has four panels:

1. **Tasks**: queued, running, blocked, reported, accepted, and failed tasks.
2. **Agents**: host, role, profile/model, current task, last activity, and reuse
   count.
3. **Token Economy**: total observed tokens, telemetry quality, assignments per
   spawn, churn, reuse, and estimated savings with method/sample size.
4. **Recent Actions**: the compact activity feed.

Unknown values render as `— / Unknown`, never `0`. The language toggle is local
to the page. The projection omits structured repository/report paths, write
scopes, Host handles, and task-internal next actions. Caller-provided goal,
title, and activity-summary text is not sanitized, so the controller must keep
prompts, source content, secrets, sensitive paths, and long logs out of those
display fields.

## Error Handling

- Invalid state transitions fail without partial writes.
- A busy or incompatible session cannot be reused.
- Known compatible ready work batches before an idle Session is considered.
- Non-exact/stale usage, an ownership mismatch, or an exhausted Session budget
  makes reuse ineligible.
- Busy Sessions have one current task; idle and terminal Sessions have none.
- Missing host template fields produce a configuration error before command
  rendering.
- Unsupported follow-up selects batch or spawn; it is not emulated by a
  permanent pool.
- Missing token telemetry marks accounting partial or unknown.
- Dashboard bind defaults to `127.0.0.1`; non-loopback requires explicit
  `--allow-remote true`. The embedded server provides no authentication or TLS,
  so remote exposure requires a trusted, access-controlled network or tunnel.
- A corrupted database is reported and never silently replaced.

## Testing Strategy

### Unit tests

- exact task-bundling compatibility dimensions;
- atomic session claim and concurrent double-claim rejection;
- batch-first ordering and accepted-follow-up/Token budget exhaustion;
- authoritative queued-set derivation, exact fresh usage, and cross-run
  ownership rejection;
- compare-and-swap queued revision refresh and terminal task-link cleanup;
- session reuse invalidation by every signature field;
- profile floors and manual elevation;
- token totals with null, partial, retry, review, and fixer phases;
- honest savings estimate threshold and median method;
- host template rendering without shell evaluation;
- state-transition validation and final audit;
- status JSON projection and bilingual labels.

### Integration tests

- six compatible ready tasks use one bounded batch rather than five follow-ups;
- later compatible work can use only the configured budgeted follow-ups;
- one incompatible dimension forces a new execution path;
- a host without follow-up batches compatible work before one spawn;
- light/standard/deep routes select the configured template arguments;
- missing actual model remains unknown;
- dashboard JSON equals CLI JSON;
- dashboard polling reflects a task and session update;
- full token accounting includes retry, review, and fixer work;
- an A/B fixture compares fresh-spawn and reuse paths with equal quality gates.

### Release verification

- clean standalone installation;
- no Superpowers access during installation or normal startup;
- Rust tests and Clippy with warnings denied;
- prompt cache contract;
- host template snapshots;
- dashboard bind and API smoke test;
- release metadata and full repository verification.

## Migration From the Over-designed Runtime

The existing increment-2 event runtime is development-only and has not been
installed. No production database migration is required.

Implementation will:

1. preserve reusable prompt and standalone-methodology behavior;
2. replace the schema with the compact tables in this design;
3. remove `event_store.rs` and its replay/transition tests;
4. replace the current ledger implementation with focused store/session/task
   operations;
5. update release validation so deleted event-runtime files are not required;
6. keep the prior architecture documents as superseded history;
7. add this design as the canonical implementation contract.

## Acceptance Criteria

- Six compatible ready tasks require one bounded batch and no preplanned
  follow-up chain.
- Later reuse stops on unknown usage, either exhausted budget, or a changed
  signature. Only exact current-assignment usage strictly after durable
  follow-up acceptance can release it; busy Sessions have one current task and
  idle/terminal Sessions have none.
- Runtime budget flags can lower but never raise release defaults.
- Changing any reuse-signature field prevents reuse.
- Light, standard, and deep routing is deterministic and provider-neutral.
- Requested and actual model data remain separate.
- Total tokens include retry, escalation, review, and fixer work.
- Unknown telemetry is never shown as zero.
- Savings estimates disclose method, sample count, and quality.
- Codex, Claude, and OpenCode command templates render exact argument arrays.
- A custom compatible host template works without code changes.
- `status`, `watch`, and the mandatory dashboard show the same state.
- The dashboard is bilingual, binds to loopback by default, and follows the
  approved Moonlight Indigo liquid-glass baseline. Remote binding is an
  explicit unauthenticated opt-in, not a public deployment mode.
- The installed Skill has no required Superpowers dependency.
- The obsolete event-sourced runtime is removed.
- Full verification and independent review pass with no open Critical or
  Important findings.
