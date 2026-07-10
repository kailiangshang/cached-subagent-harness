# Agent Control Plane Design

Date: 2026-07-10

Status: approved for implementation planning

## Summary

Extend `cached-subagent-harness` from a completion-oriented lifecycle ledger
into a cross-host agent control plane with three connected capabilities:

1. truthful, live observability of delegated work;
2. quality-constrained model and reasoning routing;
3. capability discovery across agentic CLIs and desktop-agent hosts.

The design keeps facts independent from models. Host adapters, lifecycle
operations, and validated telemetry events populate a durable event store. A
local read-only Web dashboard renders that store directly. The controller and
an optional observer agent may summarize or interpret the facts, but neither is
the dashboard's sole data source.

The target first-class hosts are Codex, Claude, and OpenCode. They are not
labeled validated until their adapter contract and opt-in real-host smoke gates
pass. Unknown CLIs are discovered through passive scanning and optional active
probes. Desktop agents such as WorkBuddy or Wukong can integrate through a
generic JSONL, local HTTP, or MCP event bridge.

## PSOC

### Problem

The current harness records agent lifecycle state for machine audit, but it
does not provide a live, human-readable view of:

- which tasks exist;
- which agent owns each task;
- what each agent is currently doing;
- which work is queued, active, waiting, blocked, reported, or complete;
- which model was requested and actually used;
- how much token or credit budget has been consumed;
- whether a host can enforce the requested routing decision.

The existing statuses are also too coarse for long-running work. An agent may
remain `running` for a long time without a current step, heartbeat, blocker, or
evidence trail. A persistent observer agent would not solve the source-of-truth
problem: it consumes a concurrency slot and tokens, has no privileged access to
another agent's internal progress, can lag or hallucinate, and becomes another
agent that must itself be monitored.

The harness must work across more than Codex. Host-specific model selection,
status APIs, token telemetry, cancellation, and hooks cannot be assumed.

### Scenarios

#### Scenario 1: Fully observable native CLI

The controller decomposes a development task, evaluates each slice, selects a
capability profile, and starts agents through a host that exposes subagent
identity, per-agent model selection, status, cancellation, and token usage.
The dashboard shows the requested and actual model, live milestones, elapsed
time, token use, blockers, and completion evidence.

#### Scenario 2: Host cannot select a per-agent model

The router requests `light` for a bounded read-only task. The host inherits the
parent model and exposes no per-spawn override. The ledger records the requested
profile, the actual inherited model when knowable, and
`routing_status=unsupported`. The dashboard warns that routing was not applied
instead of claiming a saving.

#### Scenario 3: Agent becomes stale or blocked

An agent stops producing activity. The dashboard changes its display from
active to stale after the configured heartbeat threshold, without declaring it
failed. A later explicit blocker event records the cause and next action. The
controller may steer, interrupt, replace, or close the agent, and each action is
recorded.

#### Scenario 4: Unknown CLI or upgraded version

The harness finds an executable it has not validated, or notices that a known
host's path, version, or relevant configuration fingerprint changed. Passive
scanning produces tri-state capabilities with evidence and confidence. Unknown
capabilities remain unknown until the user explicitly permits an active probe.

#### Scenario 5: Desktop agent with no CLI

A desktop agent cannot be started or inspected through a command-line API, but
it can write JSONL, call a localhost HTTP endpoint, or invoke an MCP tool. It
publishes normalized events through the bridge. Unsupported lifecycle controls
and token fields remain unknown.

#### Scenario 6: A cheap route causes rework

A `light` agent returns `NEEDS_CONTEXT`, violates a required output contract, or
fails a quality gate. The router promotes the task once to the next eligible
profile. Total token accounting includes the initial attempt, escalation,
review, and fixer work, so a superficially cheap but wasteful route is not
reported as a saving.

### Options

#### Observability options

1. Main-agent-only reporting is simple but cannot see semantic progress while
   the controller is waiting and makes the main thread a bottleneck.
2. A persistent observer agent provides natural-language summaries but consumes
   tokens and concurrency without gaining authoritative state.
3. An event-driven store with direct Web rendering keeps facts deterministic;
   the controller and optional observer become presentation clients.

#### Host support options

1. Hard-coded adapters are reliable but cannot keep up with new hosts.
2. Pure automatic discovery has broad reach but cannot safely infer semantics
   from arbitrary help output or configuration.
3. First-class adapters plus passive discovery, active probes, and an open
   bridge balance reliability and extensibility.

#### Routing options

1. Fixed routing by role is predictable but too coarse.
2. Rule-based routing uses complexity, risk, role, uncertainty, and validation
   strength to choose the lowest eligible profile.
3. Learned routing can later calibrate decisions from observed outcomes, but it
   requires trustworthy cross-task telemetry first.

### Chosen Plan

Use an event-driven control plane, a local read-only Web dashboard, a hybrid
host compatibility layer, and rule-based capability routing. Keep the observer
agent optional and short-lived. Treat Codex, Claude, and OpenCode as first-class
adapters. Default to passive capability scanning and require explicit user
action for active probes. Defer learned routing until observed data is reliable.

## Goals

- Make delegated work understandable at a glance while it is running.
- Preserve a durable, auditable lifecycle history across compaction and resume.
- Keep the source of truth independent from any LLM.
- Select the lowest-capability model and reasoning profile that still satisfies
  development quality and risk constraints.
- Record requested behavior separately from actual host behavior.
- Support hosts with partial or no native observability without fabricating
  data.
- Preserve the existing prompt-layering, role-gate, write-scope, and completion
  guarantees.
- Make new host integrations additive rather than requiring changes to the
  router, event store, or dashboard.

## Non-goals

- Do not display speculative percentage completion.
- Do not require a persistent observer agent.
- Do not automate GUI-only desktop agents that expose no integration surface.
- Do not normalize incomparable provider credits or prices without explicit
  user-supplied mappings.
- Do not make an active probe part of ordinary startup.
- Do not ship a learned or self-modifying router in the first implementation.
- Do not replace host-native agent-thread inspection or controls.

## Design Principles

1. Facts before interpretation.
2. Quality is a constraint; token reduction is the optimization objective.
3. Requested state and actual state are different fields.
4. Unknown is a valid state and must never be converted to zero or false.
5. Milestones, evidence, and freshness replace fake percentages.
6. Passive discovery is safe by default; active discovery is explicit.
7. Host-specific behavior stays behind an adapter boundary.
8. The dashboard is read-only in its first release.
9. Observer output is advisory and never mutates authoritative facts.
10. A failed cheap route counts against, rather than in favor of, efficiency.

## Architecture

```text
                           +-----------------------+
                           | Controller / Router   |
                           | PSOC, task, route     |
                           +-----------+-----------+
                                       |
                         requested route and intent
                                       |
                                       v
+----------------+      +--------------+---------------+
| Capability     |----->| Host Adapter / Bridge        |
| Scanner        |      | spawn, inspect, cancel, usage|
+----------------+      +--------------+---------------+
                                       |
                    actual lifecycle and host telemetry
                                       |
             +-------------------------+--------------------+
             |                         |                    |
             v                         v                    v
      +-------------+          +---------------+    +---------------+
      | Controller  |          | Runtime hooks |    | Agent semantic|
      | lifecycle   |          | when available|    | checkpoints   |
      +------+------+          +-------+-------+    +-------+-------+
             |                         |                    |
             +-------------------------+--------------------+
                                       |
                                       v
                     +-----------------+----------------+
                     | SQLite control-plane store       |
                     | ledger + events + capabilities   |
                     +-----------------+----------------+
                                       |
                     +-----------------+----------------+
                     |                                  |
                     v                                  v
             +---------------+                  +---------------+
             | Local Web     |                  | status/watch  |
             | dashboard     |                  | CLI fallback  |
             +-------+-------+                  +---------------+
                     |
                     v
             optional on-demand observer
             reads and explains only
```

## Component Boundaries

### Control-plane core

The Rust `harnessctl` binary owns normalized schemas, validation, storage,
status projection, lifecycle audits, capability results, and routing records.
It must not contain provider model names as routing policy defaults.

### Capability scanner

The scanner discovers hosts and produces evidence-backed capability records. It
does not start agents during passive scanning.

Passive probes may inspect:

- executable path and file identity;
- version output;
- help output;
- known configuration locations and relevant non-secret keys;
- environment indicators that do not expose secret values;
- adapter manifests and bridge endpoints.

Active probes may start one minimal read-only task to verify model selection,
status, token telemetry, steering, cancellation, or closure. They require an
explicit command and must report their token and side-effect risk before
running.

### Host adapters

An adapter translates normalized operations into host behavior. The common
contract is:

```text
detect()
probe_passive()
probe_active()
prepare_spawn()
spawn()
inspect()
steer()
cancel()
close()
read_usage()
normalize_event()
```

An adapter may return `unsupported` for any operation. The core must not infer
success from the absence of an error.

Codex, Claude, and OpenCode receive first-class adapters and real compatibility
tests. Their supported capabilities are established by probes and tests rather
than assumptions in this design document.

### Generic desktop bridge

Desktop agents can publish the same event envelope through:

- append-only JSONL;
- authenticated localhost HTTP;
- an MCP event tool.

The bridge validates schema, task identity, agent identity, event type,
timestamp, and idempotency key. It cannot grant lifecycle controls that the
desktop host does not expose.

### Model router

The router reads task demand, host capability, user mappings, and quality
floors. It produces a routing decision; the adapter attempts to apply it and
records the result. The router never treats an unapplied request as an actual
model choice.

### Event store and projections

The SQLite store contains current-state projections plus append-only history.
It uses short transactions and WAL mode for concurrent readers and event
producers. Replaying events must reproduce the dashboard state.

### Dashboard service

`harnessctl dashboard` serves embedded static assets from the local binary,
binds to loopback by default, and renders a read-only projection. Server-sent
events provide live updates; bounded polling is the fallback.

The dashboard must not receive full prompts, source contents, secrets, or long
logs. It receives structured metadata and paths to local evidence.

### Optional observer

An observer is a short-lived, read-only analysis client. It may answer questions
such as why work is blocked or which task is the current bottleneck. It reads
the event store and exits after reporting. It never proxies events to the Web,
sets lifecycle state, or stays alive merely to animate the dashboard.

## Host Capability Contract

Each capability has a tri-state result:

```text
supported
unsupported
unknown
```

Each result also records:

```text
host_id
host_kind
binary_path
host_version
config_fingerprint
capability
status
evidence
confidence
probe_kind
scanned_at
expires_at
```

The initial capability vocabulary is:

```text
subagents
per_agent_model
reasoning_effort
live_status
semantic_heartbeat
token_usage
cache_usage
credit_usage
token_budget_limit
steering
cancellation
thread_close
lifecycle_hooks
event_bridge
```

Capability results are cached by host path, version, and relevant configuration
fingerprint. A changed binary, version, or fingerprint invalidates the cache.
Keep the reusable host-capability cache outside the repository and copy an
immutable scan snapshot into each task database so a resumed task remains
auditable even after later host upgrades.

### Host support levels

```text
Tier 1  Codex, Claude, OpenCode
        first-class adapter, contract fixtures, and real smoke verification

Tier 2  Unknown but scannable CLI
        passive discovery plus user-supplied command templates

Tier 3  Desktop or embedded agent
        JSONL, localhost HTTP, or MCP event bridge

Tier 4  Unobservable host
        explicit degraded mode with no fabricated status or usage
```

## Routing Model

### Stable capability profiles

The core uses provider-neutral profiles:

```text
light     bounded search, extraction, formatting, and low-risk read-only work
standard  scoped implementation, testing, and ordinary technical analysis
deep      ambiguous multi-step work, architecture, high-risk changes, and gates
```

Users and adapters map these profiles to host-specific models and reasoning
settings. Model names do not appear in the skill's universal policy.

### Task assessment

The controller records:

```text
complexity
risk
role
uncertainty
context_size
cross_module_scope
validation_strength
failure_cost
```

The route is the lowest profile satisfying all floors:

```text
required_profile = max(
  complexity_floor,
  risk_floor,
  role_floor,
  uncertainty_floor
)
```

Examples of mandatory floors:

- security-sensitive, destructive, or control-plane changes use `deep`;
- final architecture and high-risk reviews use `deep`;
- bounded read-only discovery may use `light`;
- scoped implementation with strong tests normally uses `standard`;
- manual user overrides may raise a profile at any time;
- lowering a mandatory risk floor requires an explicit policy change, not an ad
  hoc controller choice.

### Optimization objective

Minimize expected total effective token use subject to quality and risk gates:

```text
total_effective_tokens =
  initial_attempt
  + retries
  + escalations
  + review
  + fixer_rework
```

Raw provider telemetry remains separate:

```text
input_tokens
output_tokens
reasoning_tokens
cache_read_tokens
cache_write_tokens
credits
provider_cost
telemetry_source
```

Missing data is `unknown`, not zero. Token budgets record both the requested
limit and whether the host reports it as `enforced`, `advisory`, or
`unsupported`. Raw token units may differ across providers, so cross-provider
aggregation and cost comparison occur only when a user configures compatible
normalization, price, or credit mappings.

### Escalation

Promote one profile when a route returns `NEEDS_CONTEXT`, invalidates PSOC,
misses a required contract, or fails a covering quality gate for a capability-
related reason. Do not repeatedly retry the same weak profile. Preserve the
original attempt in total-token accounting and record `escalated_from`.

## Observability Data Model

### Agent ledger projection

Extend the current ledger with normalized fields:

```text
task_id
handle
parent_handle
role
task
host_id
requested_profile
requested_model
actual_model
requested_reasoning
actual_reasoning
routing_status
routing_reason
token_budget
budget_enforcement
status
current_step
blocker
next_action
spawned_at
last_activity_at
reported_at
closed_at
report_path
write_scope
telemetry_quality
input_tokens
output_tokens
reasoning_tokens
cache_read_tokens
cache_write_tokens
credits
provider_cost
telemetry_source
final_reason
```

No field represents speculative percent completion.

### Lifecycle state and operational facets

Preserve the existing canonical lifecycle states used by final audit:

```text
planned
spawned
running
reported
closed
failed
abandoned
externally-unknown
```

Do not add `queued`, `waiting`, `stale`, or `blocked` to that lifecycle enum.
They are independent operational facets derived from dependency readiness,
current-step events, heartbeat freshness, and blocker events. This separation
keeps lifecycle audit deterministic while allowing the dashboard to explain why
a `running` agent is currently waiting, stale, or blocked.

### Append-only events

Every state change produces an event:

```text
event_id
task_id
handle
sequence
event_type
source_kind
source_id
confidence
payload_json
occurred_at
ingested_at
idempotency_key
```

Initial event types include:

```text
task_planned
route_requested
route_applied
route_degraded
agent_spawned
agent_running
agent_heartbeat
agent_step_changed
agent_blocked
agent_unblocked
agent_reported
agent_failed
agent_interrupted
agent_closed
agent_superseded
usage_observed
quality_gate_passed
quality_gate_failed
```

### Evidence priority

When sources disagree, use this order and show the conflict:

1. host runtime or lifecycle hook;
2. successful harness lifecycle operation;
3. controller observation;
4. agent self-reported semantic checkpoint;
5. inferred or estimated state.

An observer agent has no special priority.

### Read-only roles and telemetry

`read-only` continues to prohibit application and control-plane file edits.
Where supported, an agent may append a schema-limited event for its own handle
through an authenticated harness endpoint. This is a telemetry capability, not
general file write access. The endpoint cannot change another agent, close a
thread, alter routing, or write arbitrary payloads.

When no safe telemetry channel exists, the controller and host adapter provide
coarse lifecycle state and semantic progress remains unknown.

## Lifecycle Data Flow

### Startup

1. Run passive host detection.
2. Load a fresh cached capability record when its identity still matches.
3. Mark unverified capabilities `unknown`.
4. Initialize the task report, ledger, event store, and agent budget.
5. Start the dashboard only when requested; ordinary harness operation does not
   depend on it.

### Dispatch

1. Record the task and PSOC context.
2. Assess demand and select a capability profile.
3. Record `route_requested` before spawn.
4. Ask the adapter to prepare and execute the spawn.
5. Record actual host, model, reasoning, handle, and routing status.
6. Start lifecycle observation using the strongest supported evidence source.

### Progress

1. Host hooks or inspection provide runtime milestones when available.
2. Agents publish bounded semantic checkpoints through the safe event channel
   when available.
3. The controller records steering, waits, reports, and reconciliation.
4. A missing heartbeat changes freshness to stale; it does not invent failure.
5. The Web and `status/watch` clients update from the same projection.

### Completion

1. Record report and quality-gate evidence.
2. Close or finalize every agent according to the existing lifecycle contract.
3. Audit the event projection against the ledger.
4. Include routing outcomes, total effective tokens, degraded capabilities, and
   unresolved telemetry gaps in the final report.

## User Experience

### Web dashboard

The dashboard contains:

- summary counts for queued, active, waiting, stale, blocked, reported, closed,
  and failed work;
- current concurrency and task agent budget;
- a dependency graph centered on the controller;
- agent cards with role, host, model profile, actual model, current step,
  freshness, blocker, elapsed time, and next action;
- per-agent swimlanes and an append-only event timeline;
- raw token, cache, credit, and telemetry-source badges;
- requested-versus-actual routing warnings;
- report, test, review, and completion evidence links;
- archived completed agents separated from active work.

Animation reflects facts: a node pulses only after recent activity, changes
color for waiting or blocked state, and fades into the archive after closure.

### CLI fallback

The same projection is available without a browser:

```text
harnessctl status --db <path> --format table|json|markdown
harnessctl watch --db <path>
harnessctl dashboard --db <path> [--open]
```

The controller also publishes compact main-thread summaries on meaningful state
changes. It does not repeat the full ledger on a timer.

## Capability Discovery Commands

```text
harnessctl doctor
harnessctl doctor --host <host>
harnessctl doctor --active
harnessctl doctor --format table|json|markdown
```

`doctor` is passive by default. Before an active probe, show the target host,
operation, expected side effects, possible token use, and cleanup behavior.

## Degraded Modes and Errors

- Unknown host: use generic discovery and require explicit configuration before
  executing commands.
- Per-agent routing unavailable: inherit host behavior, record
  `routing_status=unsupported`, and show the warning.
- Actual model unavailable: preserve the requested model and record actual as
  `unknown`.
- Token telemetry unavailable: show usage as `unknown`; do not estimate unless
  the estimate is labeled and its method is recorded.
- Live inspection unavailable: retain controller lifecycle events and mark
  semantic progress unknown.
- Heartbeat expired: display `stale`, not `failed` or `blocked`.
- Dashboard unavailable: continue through `status/watch` and record the Web
  degradation without blocking development.
- Event rejected: retain the rejection reason and do not partially mutate the
  projection.
- Duplicate event: ignore by idempotency key.
- Conflicting evidence: retain both events, project the higher-priority source,
  and display the conflict.
- Adapter crash: do not infer that the underlying agent stopped; reconcile with
  the host or mark externally unknown.
- Store corruption or failed migration: stop lifecycle mutation, preserve the
  original database, and require recovery before final audit.

## Security and Privacy

- Bind the Web and HTTP bridge to loopback by default.
- Use an unguessable per-task bridge token for event ingestion.
- Embed no API keys, auth tokens, full prompts, source files, diffs, or long logs
  in dashboard events.
- Redact secret-like environment values during passive scans.
- Treat help and version output as untrusted text.
- Validate all bridge payloads and bound their size.
- Permit semantic telemetry only for the sender's own task and handle.
- Keep dashboard endpoints read-only in the first release.
- Never run an unknown executable during passive scanning.

## Skill Packaging and Progressive Disclosure

Keep `SKILL.md` focused on the controller workflow and mandatory gates. Move
details into directly linked references:

```text
references/observability.md
references/model-routing.md
references/host-capabilities.md
```

The Rust binary remains the deterministic runtime. Dashboard assets are bundled
with the binary rather than requiring a separate Node service. Host-specific
fixtures and adapter details remain outside `SKILL.md` so adding a host does not
inflate every skill invocation.

## Testing Strategy

### Baseline behavior tests

Preserve the reported failure scenario: multiple agents start, but a user
cannot determine tasks, ownership, progress, blockers, or completion from one
view. Demonstrate the failure against the existing harness before implementing
the new behavior.

### Unit tests

- schema creation and migration;
- append-only event validation and idempotency;
- deterministic event replay and projection;
- stale-state calculation;
- evidence-priority conflicts;
- tri-state capability handling;
- cache invalidation by path, version, and configuration fingerprint;
- routing floors and manual overrides;
- escalation accounting;
- unknown telemetry never becoming zero;
- secret redaction and bridge payload limits.

### Contract tests

Every first-class adapter runs the same fixture suite for detection, capability
reporting, spawn preparation, model application reporting, lifecycle
normalization, cancellation, closure, and usage normalization. Unsupported
operations must pass by returning an explicit unsupported result.

### Integration tests

- fake host with complete capabilities;
- fake host with no per-agent model routing;
- fake host with delayed and conflicting events;
- JSONL, local HTTP, and MCP bridge fixtures;
- concurrent SQLite readers and bounded writers;
- dashboard SSE with polling fallback;
- controller restart and event replay;
- CLI status output matching Web projections.

### Real host smoke tests

Run opt-in smoke tests for Codex, Claude, and OpenCode in isolated temporary
repositories. Record host version, authentication mode without secrets,
requested and actual model data, observed usage, cleanup, and any unsupported
capabilities. Keep credentialed or token-consuming tests out of ordinary CI.

### Routing evaluation

Measure complete-development outcomes rather than only first-pass prompt size:

- quality gates passed;
- retries and escalations;
- reviewer and fixer work;
- total effective tokens;
- raw host telemetry and its source;
- elapsed time;
- unsupported or unknown routing outcomes.

No token-saving claim is valid unless quality gates are equivalent and retries,
escalations, reviews, and fixes are included.

## Acceptance Criteria

- One command displays every harness-created task and agent with ownership,
  lifecycle state, current step when known, freshness, blocker, and next action.
- Web and CLI status views are projections of the same durable facts.
- The dashboard remains useful with no observer agent.
- Requested and actual host/model behavior are never conflated.
- Unknown or unsupported capabilities are visible and auditable.
- Codex, Claude, and OpenCode adapters pass the shared contract suite and
  opt-in real smoke protocol before being labeled first-class.
- Passive scanning performs no agent spawn and consumes no model tokens.
- Active probes disclose cost and cleanup before running.
- The router respects risk and role floors, supports manual elevation, and
  escalates without same-tier retry loops.
- Token-efficiency reports include failed attempts, escalations, review, and
  fixer work.
- Completed and superseded agents are clearly separated from active work and
  satisfy the lifecycle audit.
- Missing telemetry is `unknown`, never zero or fabricated.
- Existing prompt-shape and lifecycle verification continue to pass.

## Delivery Boundaries

This is an umbrella architecture, not authorization for one monolithic change.
Implementation planning must create a separate, reviewable plan for each
independently testable increment. The first implementation plan covers only
increment 1; later increments start after the prior increment passes its gates.
Write a short delta design before a later increment only when new evidence
changes this approved architecture.

Deliver increments in this order:

1. event schema, projections, and CLI `status/watch`;
2. safe telemetry ingestion and lifecycle integration;
3. local read-only dashboard;
4. passive capability scanner and adapter contract;
5. Codex, Claude, and OpenCode first-class adapters;
6. rule-based routing and requested-versus-actual accounting;
7. desktop bridge and active probes;
8. routing evaluation and optional observer analysis.

Each increment must preserve existing release verification and may not claim
cross-host or token-efficiency success before its corresponding evidence exists.
