# Standalone Agent Control Plane Design

Date: 2026-07-10

Status: design approved; written specification pending user review

## Summary

Evolve `cached-subagent-harness` into a standalone, cross-host agent control
plane with five connected capabilities:

1. work-package planning under a compact built-in development method;
2. agent-session leases that reuse compatible context without weakening
   isolation;
3. truthful, live observability of work, assignments, sessions, and token use;
4. quality-constrained model and reasoning routing per lease;
5. capability discovery across agentic CLIs and desktop-agent hosts.

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

Superpowers is not a runtime or installation dependency. The standalone method
is the normal operating mode. An explicitly enabled methodology adapter may
integrate with Superpowers or another compatible workflow, but optional
integration cannot weaken or replace the invariants below.

## Non-negotiable Invariants

These invariants are the skill's constitution. New features, host adapters,
methodology adapters, routing policies, dashboards, and optimization work must
preserve them. When a lower-priority feature conflicts with an invariant, the
invariant wins.

### P0: Correctness and control-plane safety

1. **Harness first.** Every long task has a brief, durable report, budget,
   lifecycle state, gates, and final audit. Record a durable planned session and
   spawn authorization before invoking a host.
2. **PSOC first.** Define Problem, Scenarios, Options, and Chosen Plan before
   worker code. When evidence invalidates PSOC, return to the earliest invalid
   section before continuing. Resolve internal engineering corrections
   autonomously; ask the user when the loop changes product behavior or scope,
   contradicts the approved plan, or exposes a product decision the controller
   cannot resolve.
3. **Complete development.** Do not use `MVP` or token pressure to skip required
   behavior, tests, error handling, integration, documentation, review,
   verification, or cleanup.
4. **Explicit write scope.** Every writer has bounded allowed paths. Read-only
   roles remain read-only except for schema-limited telemetry about their own
   session or assignment.
5. **Protect the control plane.** Skill files and references, harness source and
   runtime, prompt templates, policies, ledgers, routing state, and
   agent-management rules are immutable to delegated agents unless an approved
   worker brief grants exact paths and validation duties.
6. **Independent gates.** A writer cannot approve its own high-risk work.
   Architecture boundaries, workflow or service contracts, shared data models,
   connectors or repositories, phase-end work, and whole-branch work always
   require independent review. Critical and Important findings are fixed or
   explicitly escalated.
7. **Evidence before completion.** Every reported writer or fixer assignment is
   waited and consumed, runs the project harness, records its report and commit
   checkpoint, and passes its deterministic dispatch gate before session reuse.
   Assignment acceptance additionally requires its configured independent
   review or quality gate. Relevant focused tests, review status, and lifecycle
   audit support any completion claim.

### P1: Lifecycle and concurrency discipline

8. **Durable state is authoritative.** Resume and compaction recover from the
   repository-backed report and machine ledger, not conversation memory. A
   planned row exists before spawn and is updated immediately after every host
   lifecycle result.
9. **Read-heavy parallel, write-heavy serial.** Independent read work may run in
   parallel. Only one assignment may actively write to overlapping scope at a
   time. Reusing one writer session across compatible assignments does not
   violate this rule.
10. **Close deliberately.** Close expired, failed, abandoned, cancelled, and
    superseded sessions promptly. Every temporary or replacement session has an
    explicit expiry predicate before spawn; when it fires, close that session
    before further dispatch. Keep a session open only while a valid lease makes
    known near-term reuse more valuable than closure. Final audit closes or
    explicitly finalizes every session.
11. **No uncontrolled fan-out.** Nested delegation remains disabled unless the
    user explicitly authorizes it and the budget records the reason.
12. **Budget every session.** Initial per-run defaults remain at most two open
    delegated sessions and four total spawned sessions. Idle reusable sessions
    count against the open limit. Raising either limit requires an
    evidence-backed budget.

### P2: Context and token discipline

13. **Information density first.** Prefer structured facts, paths, status,
    evidence, and decisions over repeated narrative. Optional methodology
    guidance loads only on entry to the relevant phase and only when its context
    cost is justified; never bulk-load it at startup.
14. **Stable prompt prefixes.** Stable role policy precedes the dynamic marker;
    task-specific values stay in the dynamic tail. Pass large artifacts by path.
    Reviewers receive brief, report, and review-package paths and do not
    rediscover context already present there. Agents write full file reports and
    return only compact status and report location to the controller.
15. **Subagents are investments.** Spawn only for real parallelism, context
    isolation, capability separation, or independent judgment. Batch or reuse
    related small assignments when that lowers complete-development cost.
16. **Quality-constrained optimization.** Select the lowest model and reasoning
    profile that satisfies role, risk, uncertainty, and quality floors. Count
    retries, escalation, review, and fixer work in total token use.

### P3: Portability, truth, and stable identity

17. **Requested is not actual.** Record requested and observed host, model,
    reasoning, budget, status, and usage separately.
18. **Unknown is honest.** Unsupported or unavailable telemetry remains
    `unknown`; never convert it to zero, success, or an inferred fact.
19. **Facts do not depend on an LLM.** Host adapters, lifecycle operations, and
    validated events produce dashboard facts. An observer may explain but never
    becomes the source of truth.
20. **Stable names, no version suffixes.** Keep skill, role, agent/session
    profile, and policy names stable. Unique session IDs and versions are data,
    not name suffixes.

Every implementation increment must map tests and acceptance evidence back to
these numbered invariants.

### Existing-contract disposition map

`Exact` retains the current operational meaning. `Evolved — approved` records a
change explicitly approved in this design; it is not an accidental weakening.
No existing normative rule is silently removed.

| Existing contract | Disposition | Target contract and evidence |
|---|---|---|
| Harness first | Exact | Invariants 1, 7-8 and final audit |
| PSOC first; loop to earliest invalid section | Exact | Invariant 2 and lifecycle planning |
| Ask only for unresolved product/scope/plan decisions | Exact | Invariant 2 |
| Information density first | Exact | Invariants 13-14 |
| Complete development; no shortcut MVP | Exact | Invariant 3 |
| Stable prompt prefix and dynamic tail | Exact | Invariant 14 and prompt-shape acceptance |
| File-backed full reports; compact controller returns | Exact | Invariants 7, 14 and assignment gate |
| Subagents are token investments | Exact | Invariants 12, 15-16 and churn benchmark |
| Stable skill, agent/session profile, and role names | Exact | Invariant 20 |
| Protect skill references, harness runtime, prompts, and policy | Exact | Invariants 4-5 |
| Planned ledger state exists before spawn | Exact | Invariants 1, 8 and `session_planned` event |
| Close superseded and temporary agents at explicit expiry | Exact | Invariant 10 and replacement lease fields |
| Default two concurrent/open and four total agents | Exact | Invariant 12 and budget tests |
| No recursive fan-out without user authority | Exact | Invariant 11 |
| Read-heavy parallel; overlapping writes serial | Evolved — approved | Invariant 9; serialization moves from thread lifetime to assignment scope |
| Fresh worker and closure before every next worker | Evolved — approved | Compatible assignments reuse one bounded writer lease; churn fixture proves reuse without concurrent overlap |
| Stable discussion, explorer, worker, reviewer, and fixer gates | Exact | Stable role contracts and role-gate tests |
| Project harness after every worker/fixer return | Exact | Invariant 7 and assignment gate |
| Mandatory independent-review trigger set | Exact | Invariant 6 and review-policy tests |
| Batch Critical/Important findings into one fixer pass | Exact | Package-review flow |
| Known sessions audit from ledger; external unknowns reconciled only when material | Exact | Completion reconciliation contract |
| Durable report, ledger, and final lifecycle audit | Exact | Invariants 7-8, 10 and completion flow |
| Superpowers guidance is phase-lazy and context-cost justified | Exact when integration is enabled | Invariant 13 and optional-adapter contract |
| Missing Superpowers is degraded mode | Evolved — approved | Standalone is authoritative; only unavailable required runtime capability is degraded |

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

The current package is also operationally heavier than its skill body implies.
Its installer automatically acquires Superpowers unless the user opts out, and
its documentation presents Superpowers as a broader development dependency.
Standalone operation exists only as a degraded fallback instead of the normal
contract.

Finally, the current one-row-per-agent lifecycle and close-before-next-worker
gate encourage a fresh thread for every plan item. When tasks are small and
sequential, the controller repeatedly pays prompt bootstrap, repository
rediscovery, ledger, report, review, and closure overhead. Cheap models do not
solve that churn and may increase total token use through retries and context
reconstruction.

### Scenarios

#### Scenario 1: Fully observable native CLI

The controller decomposes a development task, evaluates each slice, selects a
capability profile, and starts agents through a host that exposes subagent
identity, per-agent model selection, status, cancellation, and token usage.
The dashboard shows the requested and actual model, live milestones, elapsed
time, token use, blockers, and completion evidence.

#### Scenario 2: Host cannot select a per-agent model

The router requests `light` for a bounded read-only assignment. The host inherits
the parent model and exposes no per-spawn override. The ledger records the
requested profile, the actual inherited model when knowable, and
`routing_status=unsupported`. If the inherited capability is verified to meet
the floor, work may proceed with a warning and no false saving claim. If it is
unknown or below-floor, eligibility fails closed and the assignment is rerouted.

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
fails a quality gate. The router creates a new attempt for the assignment at the
next eligible profile. Total token accounting includes the initial attempt,
escalation, review, and fixer work, so a superficially cheap but wasteful route
is not reported as a saving.

#### Scenario 7: Related micro-assignments reuse one writer session

A work package contains six sequential assignments with the same role, model
profile, code area, write scope, and PSOC. The dispatcher creates one writer
session, issues a bounded lease, and sends follow-up assignments to that
session. One independent reviewer evaluates the package boundary. The dashboard
shows six assignments, one writer spawn, one reviewer spawn, and the measured
bootstrap-to-useful-work ratio.

#### Scenario 8: Reuse becomes unsafe

The next assignment needs a stronger model, a different role, broader write
scope, independent judgment, or unrelated context. The lease manager refuses
reuse, records the reason, expires or closes the old lease as appropriate, and
creates a new session. Reuse never overrides isolation or risk floors.

#### Scenario 9: Clean standalone installation

The user installs the harness into a clean runtime. The default installer does
not clone Superpowers, contact its repository, or copy external skills. The
built-in methodology can plan, test, review, verify, and audit the task. An
optional `--with-superpowers` path is explicit and separately tested.

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

#### Methodology options

1. Keep Superpowers as an automatic dependency, retaining broad workflows but
   also their installation weight and fresh-agent-per-task bias.
2. Copy a large subset of Superpowers into this skill, which replaces an
   external dependency with a maintenance and context dependency.
3. Make a compact standalone methodology kernel authoritative and expose
   explicit optional adapters for Superpowers or other methodologies.

#### Agent-granularity options

1. Fresh session per assignment maximizes isolation but pays fixed overhead for
   every micro-task.
2. A persistent role pool minimizes spawning but risks context pollution,
   permission drift, and accidental task coupling.
3. Work packages with bounded session leases reuse compatible context while
   forcing a new session when role, model, scope, risk, or independence changes.

### Chosen Plan

Use a compact standalone methodology kernel, work-package planning, bounded
agent-session leases, an event-driven control plane, a local read-only Web
dashboard, a hybrid host compatibility layer, and rule-based capability
routing. Make work packages and assignments the primary progress model; treat
sessions as reusable resources. Keep the observer optional and short-lived.
Treat Codex, Claude, and OpenCode as target first-class adapters. Default to
passive capability scanning and require explicit user action for active probes.
Make Superpowers an explicit optional adapter. Defer learned routing until
observed data is reliable.

## Goals

- Make delegated work understandable at a glance while it is running.
- Make standalone operation the normal, fully capable mode.
- Preserve every non-negotiable invariant while changing agent granularity.
- Represent delivery as work packages and assignments rather than equating a
  task with an agent thread.
- Reuse compatible sessions without weakening write isolation or review
  independence.
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
- Do not require or automatically install Superpowers.
- Do not keep sessions alive without a valid work-package lease, except for the
  bounded provisional interval between a recorded spawn authorization and route
  eligibility.
- Do not reuse a session across incompatible role, model, write scope, risk, or
  independence boundaries.
- Do not automate GUI-only desktop agents that expose no integration surface.
- Do not normalize incomparable provider credits or prices without explicit
  user-supplied mappings.
- Do not make an active probe part of ordinary startup.
- Do not ship a learned or self-modifying router in the first implementation.
- Do not replace host-native agent-thread inspection or controls.

## Derived Design Principles

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
11. Work progress and agent-thread lifecycle are separate dimensions.
12. Reuse is earned by lease compatibility, not preferred unconditionally.
13. One active writer does not imply one fresh writer session per assignment.
14. The Web leads with deliverables; agent topology is a secondary resource
    view.

## Architecture

```text
                  +--------------------------------+
                  | Standalone Methodology Kernel  |
                  | PSOC, work packages, gates     |
                  +---------------+----------------+
                                  |
                                  v
                  +---------------+----------------+
                  | Dispatcher and Lease Manager   |
                  | main vs reuse vs batch vs spawn|
                  +-------+----------------+-------+
                          |                |
                   route and lease    capability facts
                          |                |
                          v                v
                 +--------+-------+  +-----+----------+
                 | Model Router   |  | Host Scanner   |
                 +--------+-------+  +-----+----------+
                          |                |
                          +--------+-------+
                                   |
                                   v
                 +-----------------+----------------+
                 | Host Adapter / Desktop Bridge    |
                 | follow-up, spawn, inspect, close |
                 +-----------------+----------------+
                                   |
              +--------------------+--------------------+
              |                    |                    |
              v                    v                    v
      controller events     runtime or hooks     semantic checkpoints
              |                    |                    |
              +--------------------+--------------------+
                                   |
                                   v
                +------------------+------------------+
                | SQLite control-plane event store   |
                | runs, packages, assignments,       |
                | attempts, sessions, leases, routes |
                +------------------+------------------+
                                   |
                  +----------------+----------------+
                  |                                 |
                  v                                 v
          +---------------+                 +---------------+
          | Local Web     |                 | status/watch  |
          | work control  |                 | CLI fallback  |
          +-------+-------+                 +---------------+
                  |
                  v
          optional on-demand observer
          reads and explains only
```

## Component Boundaries

### Standalone methodology kernel

The built-in method owns PSOC, work-package boundaries, test-first behavior,
write serialization, independent review, verification, and final audit. It is
compact enough to operate without another plugin. Optional methodology adapters
may provide compatible artifacts or additional guidance, but the kernel remains
authoritative for the non-negotiable invariants. When explicitly enabled, an
adapter loads guidance only when its phase begins and the context cost is
justified; it never bulk-loads another methodology at startup.

### Stable role contracts

The standalone kernel keeps the existing stable roles while allowing a
compatible session to receive more than one assignment:

- `discussion` and `explorer` are read-only and return bounded product or code
  discovery evidence;
- `worker` is the normal writer, uses an explicit write scope, makes behavior
  changes test-first, runs focused tests, commits its checkpoint, and writes an
  assignment report;
- `reviewer` is read-only and independent of the writer session; it receives the
  brief, report, and review-package paths instead of rediscovering the project;
- `fixer` performs one bounded write pass for the complete Critical/Important
  findings set, updates tests and the existing report, and commits the fix.

A session's role never mutates. A different role, including writer-to-reviewer
or reviewer-to-fixer, requires a distinct session and lease. Full reports live
in files; agent returns contain only compact status, evidence summary, and the
report path.

### Work-package planner

The planner turns the approved PSOC into delivery-oriented work packages and
bounded assignments. It groups assignments when they share role, model floor,
   risk class, write scope, code area, base revision, dependency order, and
independence/review boundary. It splits work when any of those boundaries
diverge. Review and fixer work are explicit assignments with their own role,
model, risk, scope, and independence requirements.

### Dispatcher and lease manager

The dispatcher selects one of four actions for each ready assignment:

```text
execute_on_main
reuse_session
batch_into_package
spawn_session
```

It records the decision and evidence before execution. The lease manager binds
a session to one work package and a complete compatibility signature: role,
model profile, risk class, write scope, base revision, and independence/review
boundary. It validates the signature atomically before every follow-up and
rejects work that crosses any bound. Only one assignment may actively write to
overlapping scope, even when a writer session is reused.

After a validated assignment from the same writer session, the controller may
advance the lease's base revision to that controller-verified checkpoint. An
unrelated repository change, rejected report, or unverified head movement makes
the base stale and expires the lease; the session cannot silently bless its own
base.

A lease ends when its package finishes, the required role/model/risk/scope or
independence boundary changes, context becomes stale, the session fails, or its
run, package, or current assignment is cancelled. Keep an idle lease only when
the same package has a known compatible pending assignment and the open-session
budget is not under pressure. Otherwise expire it immediately. A temporary or
replacement lease records the session it replaces and an observable expiry
predicate such as `original_resumed` or `recovery_cancelled`; when the predicate
fires, revoke the lease and close the replacement before further dispatch.

### Control-plane core

The Rust `harnessctl` binary owns normalized schemas, validation, storage,
status projection, lifecycle audits, capability results, and routing records.
It also owns run, package, assignment, attempt, session, lease, and route
projections. It must not contain provider model names as routing policy defaults.

### Capability scanner

The scanner discovers hosts and produces evidence-backed capability records. It
does not start agents during passive scanning.

Passive probes may inspect without executing untrusted code:

- executable path and file identity;
- cached or file-backed version metadata;
- help or version output only for an allowlisted, identity-matched first-class
  adapter binary;
- known configuration locations and relevant non-secret keys;
- environment indicators that do not expose secret values;
- adapter manifests and bridge endpoints.

Executing an unknown or identity-changed binary, even with `--help` or
`--version`, is an active probe. Active probes may start one minimal read-only
assignment to verify model selection, status, token telemetry, steering,
cancellation, or closure. They require an explicit command and must report
their token and side-effect risk before running. A probe is still a budgeted,
planned assignment/session with ordinary cleanup and lifecycle audit; `probe`
does not bypass the constitution.

### Host adapters

An adapter translates normalized operations into host behavior. The common
contract is:

```text
detect()
probe_passive()
probe_active()
prepare_spawn()
spawn()
prepare_followup()
followup()
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

The bridge validates schema; the run, package, assignment, attempt, session, and
lease identity chain; event type; timestamp; and idempotency key. Ingestion uses
run-scoped authentication plus a session-bound credential or claim, so a sender
can report only its own current assignment attempt. The bridge cannot grant
lifecycle controls that the desktop host does not expose.

### Model router

The router reads assignment demand, host capability, user mappings, and quality
floors. It selects a profile for a session lease, not a disposable model choice
for every micro-assignment. The adapter attempts to apply the decision and
records the result. An assignment that needs a different profile is ineligible
for reuse. The router never treats an unapplied request as an actual model
choice. Before any assignment content is sent, a route-eligibility gate must
establish that the applied or inherited capability satisfies every mandatory
floor. Unknown or below-floor capability fails closed: close the provisional
session and reroute, use a demonstrably sufficient main execution path, or seek
an explicit policy decision. A warning alone never authorizes below-floor work.

### Event store and projections

The SQLite store contains current-state projections plus append-only history
for runs, work packages, assignments, attempts, sessions, leases, routes,
capabilities, and usage. It uses short transactions and WAL mode for concurrent
readers and event producers. Replaying events must reproduce the dashboard
state.

### Dashboard service

`harnessctl dashboard` serves embedded static assets from the local binary,
binds to loopback by default, and renders a read-only projection led by work
packages and assignments. Sessions, leases, model routes, and churn are
secondary resource views. Server-sent events provide live updates; bounded
polling is the fallback.

The dashboard must not receive full prompts, source contents, secrets, or long
logs. It receives structured metadata and paths to local evidence.

### Optional observer

An observer is a short-lived, read-only analysis client. It may answer questions
such as why work is blocked or which package is the current bottleneck. It reads
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
session_followup
session_resume
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
immutable scan snapshot into each run database so a resumed run remains
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
        explicit limited-capability mode with no fabricated status or usage
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

### Package and assignment assessment

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
work_package
code_area
write_scope
independence_requirement
```

The route is the lowest profile satisfying all floors for the session lease:

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

### Delegation and reuse decision

Before selecting a model, decide whether delegation itself is justified:

```text
if assignment is trivial and needs no isolation:
    execute_on_main
else if an open lease matches package, role, model, risk, scope,
        base revision, and independence boundary:
    reuse_session
else if related ready assignments can share those boundaries:
    batch_into_package
else if parallelism + isolation + capability + independent_judgment
        exceeds bootstrap + coordination + context_rebuild:
    spawn_session
else:
    execute_on_main
```

Do not rely on a universal file-count or token threshold. Record decision
features and calibrate defaults through the churn benchmark. Reuse is forbidden
across writer/reviewer roles, incompatible write scopes, unrelated packages,
different model or risk floors, stale base revision, different independence
boundary IDs, unresolved review boundaries, or an explicit independence
requirement.

### Optimization objective

Minimize expected total effective token use subject to quality and risk gates:

```text
total_effective_tokens =
  session_bootstrap
  + context_reload
  + useful_assignment_work
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

### Session economics

Calculate churn separately by role and host:

```text
assignments_per_spawn = accepted_delegated_assignments / spawned_sessions
churn_rate = spawned_sessions / completed_delegated_assignments
bootstrap_ratio = session_bootstrap_tokens / total_effective_tokens
reuse_count = accepted_followup_assignments
```

When a host cannot separate bootstrap, context reload, or useful work, keep
those values `unknown` and report only observed counts. Any counterfactual reuse
saving is an estimate with its method and confidence, never an observed fact.

### Escalation

Promote one profile when a route returns `NEEDS_CONTEXT`, invalidates PSOC,
misses a required contract, or fails a covering quality gate for a capability-
related reason. Do not repeatedly retry the same weak profile. Preserve the
original assignment attempt in total-token accounting and record
`escalated_from`. A profile change is incompatible with the existing lease even
when the host can mutate a live thread: revoke that lease and route a new,
correctly profiled session or demonstrably sufficient main execution path.

## Observability Data Model

### Entity projections

Do not equate a task with an agent row. Project the event stream into separate
entities.

#### Run

```text
run_id
goal
psoc_revision
status
session_budget
token_budget
report_path
ledger_path
started_at
completed_at
```

Run states are `planned`, `active`, `blocked`, `complete`, and `cancelled`.

#### Work package

```text
package_id
run_id
title
dependencies
role_floor
model_floor
risk_floor
write_scope
review_policy
independence_policy
status
blocker
next_action
```

Package states are `planned`, `ready`, `active`, `review`, `complete`,
`blocked`, and `cancelled`.

#### Assignment

```text
assignment_id
package_id
title
sequence
assignment_kind
required_role
model_floor
risk_class
write_scope
base_revision
independence_boundary_id
current_attempt_id
attempt_count
status
current_step
blocker
report_path
test_evidence
review_evidence
started_at
reported_at
validated_at
accepted_at
final_reason
```

Assignment states are `planned`, `queued`, `running`, `reported`, `validated`,
`accepted`, `failed`, and `cancelled`. `validated` means the report, focused
tests, commit checkpoint, and per-return harness passed, so a compatible writer
lease may be reused. `accepted` additionally requires the package's configured
independent-review or quality gate; an agent report alone is insufficient.
`assignment_kind` includes
`discussion`, `exploration`, `implementation`, `review`, and `fix`. A review
assignment is accepted when its own report and independence contract pass; the
package gate separately consumes the verdict it emitted, avoiding circular
self-approval. Blocked is a derived facet over the assignment state, not an
additional canonical state.

#### Assignment attempt

Retries, replacements, and escalation are projected through an explicit join
between logical work and the session that attempted it:

```text
attempt_id
assignment_id
session_id
lease_id
attempt_sequence
route_id
status
started_at
reported_at
validated_at
accepted_at
ended_at
outcome_reason
input_tokens
output_tokens
reasoning_tokens
cache_read_tokens
cache_write_tokens
credits
provider_cost
telemetry_source
```

Attempt states are `planned`, `running`, `reported`, `validated`, `accepted`,
`failed`, and `cancelled`. A failed attempt may requeue its assignment under
policy; an `assignment_failed` event is terminal only after retry/escalation
policy is exhausted. The assignment's `current_attempt_id` is only the current
projection; attempt history remains append-only and drives ownership and token
accounting.

#### Agent session

```text
session_id
handle
parent_handle
host_id
role
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
spawned_at
last_activity_at
last_reported_at
last_waited_at
outcome
close_disposition
close_requested_at
closed_at
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

Preserve the existing session lifecycle values during migration:

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

A reusable session may cycle from `reported` back to `running` only after the
controller waits for and consumes the prior assignment report, runs its gate,
records validation or a terminal outcome, and atomically revalidates the lease.
The normal transitions are:

```text
planned -> spawned -> running -> reported
reported -> running       # compatible validated follow-up
planned|spawned|running|reported -> closed
planned|spawned|running|reported -> failed|abandoned|externally-unknown
failed|abandoned|externally-unknown -> closed  # when later reconciliation succeeds
```

`outcome` retains success, failure, abandonment, or unknown cause when cleanup
later changes lifecycle status to `closed`. `close_disposition` is
`not-requested`, `requested`, `confirmed`, `unsupported`, or `unknown`; logical
closure remains auditable even when the host lacks `thread_close`. Final audit
accepts `closed`, or `failed`, `abandoned`, or `externally-unknown` only with an
explicit `final_reason` and next action. Session operational facets such as
`active`, `idle/reusable`, `waiting`, `stale`, and `blocked` are derived and do
not replace the canonical lifecycle value.

#### Session lease

```text
lease_id
session_id
package_id
role
model_profile
risk_class
write_scope
base_revision
independence_boundary_id
current_attempt_id
replaces_session_id
expiry_predicate
status
reuse_count
issued_at
last_used_at
expires_at
expiry_reason
```

Lease states are `planned`, `active`, `idle`, `expired`, `revoked`, and
`closed`. Only an `active` or compatible `idle` lease can receive a follow-up
assignment. `replaces_session_id` is null for ordinary work; temporary and
recovery leases require both that field and an observable `expiry_predicate`.

#### Routing decision

```text
route_id
attempt_id
required_profile
requested_model
requested_reasoning
actual_model
actual_reasoning
routing_status
eligibility_status
eligibility_evidence
escalated_from_route_id
decided_at
```

Eligibility is `eligible`, `rejected`, or `unknown`. Both `rejected` and
`unknown` fail closed before assignment delivery. A new route row, attempt, and
compatible execution path represent escalation; an existing lease's profile is
never mutated in place.

No entity contains speculative percent completion. The Web may show objective
counts such as `3 / 5 accepted assignments`, while also showing when the package
scope changed and altered the denominator.

### Append-only events

Every state change produces an event:

```text
event_id
run_id
package_id
assignment_id
attempt_id
session_id
lease_id
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

The control-plane core assigns `sequence` transactionally and monotonically
within a run after validating an event. Producer timestamps never determine
replay order. Producers supply an idempotency key; deterministic projection uses
the stored run sequence and retains `occurred_at` only as observed-time data.

Initial event types include:

```text
run_planned
run_started
run_blocked
run_unblocked
run_completed
run_cancelled
package_planned
package_ready
package_active
package_blocked
package_unblocked
package_review_started
package_review_completed
package_completed
package_cancelled
assignment_queued
assignment_started
assignment_step_changed
assignment_blocked
assignment_unblocked
assignment_reported
assignment_validated
assignment_accepted
assignment_requeued
assignment_failed
assignment_cancelled
attempt_planned
attempt_started
attempt_reported
attempt_validated
attempt_accepted
attempt_failed
attempt_cancelled
dispatch_main_selected
dispatch_reuse_selected
dispatch_batch_selected
dispatch_spawn_selected
route_requested
route_applied
route_degraded
route_rejected
session_planned
session_spawned
session_running
session_heartbeat
session_blocked
session_unblocked
session_reported
session_waited
session_failed
session_abandoned
session_externally_unknown
session_interrupted
session_close_requested
session_closed
session_superseded
lease_planned
lease_issued
lease_reused
lease_idle
lease_expired
lease_revoked
lease_closed
usage_observed
quality_gate_passed
quality_gate_failed
```

The event registry is validated against every canonical state transition. A new
state cannot ship without either a corresponding typed event or a schema-
validated generic transition event with equivalent replay and authorization
tests.

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
Where supported, an agent may append a schema-limited event for its own session
and current assignment attempt through an authenticated harness endpoint. This
is a telemetry capability, not general file write access. The endpoint cannot
change another agent, close a thread, accept an assignment, alter routing, or
write arbitrary payloads.

When no safe telemetry channel exists, the controller and host adapter provide
coarse lifecycle state and semantic progress remains unknown.

## Lifecycle Data Flow

### Startup

1. Run passive host detection.
2. Load a fresh cached capability record when its identity still matches.
3. Mark unverified capabilities `unknown`.
4. Initialize the run, repository report, machine ledger, event store, session
   budget, and token budget. The initial session budget is two open and four
   total spawned sessions unless the report contains an evidence-backed override.
5. Start the dashboard only when requested; ordinary harness operation does not
   depend on it.

### Plan work packages

1. Record the approved PSOC revision.
2. Define work packages, dependencies, assignment boundaries, write scopes, and
   review policies.
3. Group related assignments only when their role, model floor, risk class,
   scope, base revision, and independence boundary are compatible.
4. Mark a package `ready` only when its dependencies and product decisions are
   resolved.

### Dispatch

1. Assess the next ready assignment and record whether main execution, reuse,
   batching, or a new spawn is selected.
2. Persist `attempt_planned` and, for a spawn, `session_planned` plus the spawn
   authorization before invoking the host. These records contain the budget,
   role, complete lease signature, route intent, and any replacement expiry
   predicate so a crash cannot erase intent.
3. If main execution is selected, record a controller-owned attempt without
   creating a fake subagent session.
4. If reuse is selected, atomically revalidate the complete lease signature
   immediately before sending a follow-up. Increment `reuse_count` only after
   the host accepts it.
5. If spawn is selected, record `route_requested`, ask the adapter to spawn, and
   capture the actual handle, model, reasoning, and routing status. Do not issue
   an active lease or send assignment content until route eligibility proves the
   actual/inherited capability satisfies all floors. A rejected provisional
   session is finalized and rerouted.
6. Start lifecycle observation using the strongest supported evidence source.

### Progress

1. Host hooks or inspection provide runtime milestones when available.
2. Agents publish bounded semantic checkpoints for the current assignment
   through the safe event channel when available.
3. The controller records follow-ups, steering, waits, reports, lease changes,
   and reconciliation.
4. A missing heartbeat changes freshness to stale; it does not invent failure.
5. The Web and `status/watch` clients update from the same projection.

### Gate a reported assignment

1. The agent writes its full assignment report and returns only compact status,
   commit/test evidence, risks, and the report path.
2. The controller waits for the session, consumes the report, and records
   `session_waited` plus the attempt outcome.
3. After every writer or fixer report, the controller runs the project harness.
   A failure is classified and fixed before acceptance; a fixer return runs the
   harness again.
4. A successful writer or fixer assignment records its commit checkpoint,
   focused tests, harness result, and required review evidence.
5. Only then may the assignment become `validated`, the lease become
   `idle/reusable`, and its verified base revision advance. Low-risk work with a
   deterministic-only policy may also become `accepted`; work requiring
   independent review stays validated until that verdict passes. A failed or
   `LOOP_REQUIRED` report cannot be hidden by reusing the same session.

### Review a package

1. Deterministic acceptance without an independent reviewer is allowed only
   when the package is low risk and none of the mandatory review triggers apply.
2. Architecture boundaries, workflow or service contracts, shared data models,
   connectors or repositories, phase-end work, and whole-branch work always
   create an independent review assignment. Other packages receive independent
   package-boundary review by default.
3. Use per-assignment review for high-risk changes, public contracts, or an
   explicit independence boundary; otherwise one package review may cover the
   validated implementation assignments.
4. The review assignment records distinct writer and reviewer session IDs and
   the evidence it judged. The reviewer never shares a writer/fixer session.
5. Batch Critical and Important findings into one bounded fixer assignment,
   then reuse a compatible independent reviewer lease or create a correctly
   isolated one to re-review the affected evidence.

### Completion

1. Require every assignment to be accepted or explicitly failed/cancelled and
   every package to be complete or explicitly blocked/cancelled.
2. Record report and quality-gate evidence.
3. Expire leases and close or finalize every session according to the exact
   terminal-state contract. Any fired temporary/replacement expiry predicate is
   resolved before another dispatch.
4. For every known harness-created session, reconcile from the durable ledger;
   do not ask the user to reproduce host UI state. If an unknown UI-visible
   session materially affects budget, cleanup, or correctness and the host
   cannot list it, request at most one external reconciliation and record it as
   `externally-unknown` rather than inventing control.
5. Audit package, assignment, attempt, session, lease, and event projections.
6. Include routing outcomes, session churn, useful-versus-overhead tokens,
   unsupported capabilities, and unresolved telemetry gaps in the final report.

## User Experience

### Web dashboard

The dashboard has four coordinated views.

#### Work map

Lead with the work-package dependency graph rather than the agent topology.
Each package shows objective assignment counts, stage, blocker, active
assignment owners and sessions, quality gates, and next action. Scope changes
are visible when they change the assignment denominator.

#### Agent dock

Show sessions as resources with role, host, requested and actual model, lease,
current assignment, queued follow-ups, assignments completed, reuse count,
context freshness, write scope, elapsed time, and close reason. A compatible
idle session appears as `idle/reusable`; a session whose lease expired fades
into archive after lifecycle closure.

#### Token economy

Show main-agent work, useful assignment tokens, session bootstrap, context
reload, retries, escalation, review, and fixer costs separately. Include
assignments per spawn, reuse count, churn rate, requested-versus-actual routing,
and telemetry confidence. Warn when repeated sessions complete nearly the same
number of small assignments and bootstrap overhead is material.

#### Event timeline

Show package, assignment, attempt, session, lease, routing, usage, and quality
events in one filterable timeline. Per-session swimlanes remain available as a
secondary view.

Animation reflects facts: packages pulse only after accepted activity, sessions
pulse only after recent runtime or semantic events, blocked work changes color,
and closed sessions fade into archive. No animation implies invented progress.

### CLI fallback

The same projection is available without a browser:

```text
harnessctl status --db <path> --format table|json|markdown
harnessctl watch --db <path>
harnessctl dashboard --db <path> [--open]
```

The controller also publishes compact main-thread summaries on meaningful state
changes. It does not repeat the full ledger on a timer.

## Standalone Methodology and Installation

Standalone is the default and fully supported mode:

```text
scripts/install.sh
```

The default installer installs only this skill and its deterministic runtime.
It performs no Superpowers clone, fetch, checkout, or skill copy. Optional
integration is explicit:

```text
scripts/install.sh --with-superpowers
```

When enabled, the adapter may consume compatible planning, TDD, review, or
finishing artifacts. It cannot replace the invariant contract, force a fresh
session per assignment, or label absence of optional Superpowers integration as
degraded. Adapter guidance loads phase by phase, never as startup bulk context.
If the user explicitly requests `--with-superpowers` and setup is unavailable,
the optional setup fails visibly or records `adapter_unavailable`; it never
pretends the request succeeded, and the standalone kernel remains healthy.

The built-in methodology remains compact:

```text
PSOC
work-package planning
test-first behavior changes
bounded writes
package review
final verification
lifecycle audit
```

## Capability Discovery Commands

```text
harnessctl doctor
harnessctl doctor --host <host>
harnessctl doctor --active
harnessctl doctor --format table|json|markdown
```

`doctor` is passive by default. Before an active probe, show the target host,
operation, expected side effects, possible token use, and cleanup behavior.

## Fallbacks and Errors

- Unknown host: use generic discovery and require explicit configuration before
  executing commands.
- Per-agent routing unavailable: record `routing_status=unsupported`; use an
  inherited model only when verified capability still satisfies every floor.
  Otherwise fail route eligibility and select a safe path.
- Actual model or capability unavailable: preserve the request, record actual as
  `unknown`, and fail closed before sending assignment content when eligibility
  cannot be established.
- Token telemetry unavailable: show usage as `unknown`; do not estimate unless
  the estimate is labeled and its method is recorded.
- Live inspection unavailable: retain controller lifecycle events and mark
  semantic progress unknown.
- Session follow-up unavailable: batch compatible assignments before spawning
  and prefer one session per compatible package/role/signature batch; record
  reuse as unsupported.
- Lease mismatch: reject follow-up, record the failed reuse decision, and route
  the assignment through main execution, batching, or a correctly scoped spawn.
- Optional methodology not requested: continue in normal standalone mode with
  no degradation note. Explicitly requested integration unavailable: report or
  fail that adapter request visibly while keeping the standalone core healthy.
- Required harness/runtime capability unavailable: record actual degraded mode
  and use only a documented equivalent that preserves the invariant gates.
- Heartbeat expired: display `stale`, not `failed` or `blocked`.
- Dashboard unavailable: continue through `status/watch` and record the Web
  degradation without blocking development.
- Event rejected: retain the rejection reason and do not partially mutate the
  projection.
- Duplicate event: ignore by idempotency key.
- Conflicting evidence: retain both events, project the higher-priority source,
  and display the conflict.
- Adapter crash: do not infer that the underlying session stopped; reconcile
  with the host or mark it externally unknown.
- Store corruption or failed migration: stop lifecycle mutation, preserve the
  original database, and require recovery before final audit.

## Security and Privacy

- Bind the Web and HTTP bridge to loopback by default.
- Use an unguessable per-run bridge token plus session-bound claims for event
  ingestion.
- Embed no API keys, auth tokens, full prompts, source files, diffs, or long logs
  in dashboard events.
- Redact secret-like environment values during passive scans.
- Treat help and version output as untrusted text.
- Validate all bridge payloads and bound their size.
- Permit semantic telemetry only when the authenticated run/session identity
  chain owns the claimed current assignment attempt and lease.
- Keep dashboard endpoints read-only in the first release.
- Never run an unknown executable during passive scanning.

## Skill Packaging and Progressive Disclosure

Keep the numbered non-negotiable invariants, controller loop, and mandatory
gates in `SKILL.md`; never move the constitution out of the body agents actually
load. Move detailed mechanics into directly linked references:

```text
references/standalone-methodology.md
references/work-packages-and-leases.md
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

Also preserve the session-churn failure: six compatible sequential assignments
cause repeated fresh worker and reviewer sessions, repeated context loading, and
bootstrap overhead close to useful-work cost. Demonstrate that failure before
changing dispatch rules.

### Unit tests

- schema creation and migration;
- work-package, assignment, attempt, session, and lease projection boundaries;
- durable `session_planned` and spawn authorization before every host call;
- crash recovery between planned, spawned, and lease-issued events;
- initial two-open/four-total session budget and evidence-backed overrides;
- complete lease compatibility across role, model, risk, scope, base revision,
  and independence boundary;
- atomic lease revalidation, verified base advancement, and expiry reasons;
- replacement linkage and immediate expiry-predicate enforcement;
- dispatch decisions for main, reuse, batch, and spawn;
- one active writer across overlapping scopes;
- mandatory reviewer triggers and writer/reviewer identity separation;
- writer/fixer report, wait, commit, and per-return harness gates;
- profile escalation always creating a new compatible execution path;
- route eligibility rejecting unknown or below-floor actual capability;
- assignment-attempt retry, replacement, escalation, and token accounting;
- reusable `reported -> running` session transition guards and exact final-audit
  terminal states;
- append-only event validation and idempotency;
- event-registry coverage for every canonical transition;
- deterministic event replay and projection;
- transactional per-run sequence allocation under concurrent ingestion;
- stale-state calculation;
- evidence-priority conflicts;
- tri-state capability handling;
- cache invalidation by path, version, and configuration fingerprint;
- passive scanner trust boundary for unknown or changed executables;
- routing floors and manual overrides;
- escalation accounting;
- unknown telemetry never becoming zero;
- session-bound bridge authorization and identity-chain validation;
- secret redaction and bridge payload limits;
- phase-lazy optional methodology loading and visible explicit-adapter failure;
- invariant-to-test coverage remains complete.

### Contract tests

Every first-class adapter runs the same fixture suite for detection, capability
reporting, spawn preparation, model application reporting, lifecycle
normalization, follow-up/resume behavior, cancellation, closure, and usage
normalization. Unsupported operations must pass by returning an explicit
unsupported result.

### Integration tests

- fake host with complete capabilities;
- fake host with no per-agent model routing;
- fake hosts with unknown and below-floor actual models failing closed before
  assignment delivery;
- fake host with delayed and conflicting events;
- six compatible assignments using one writer lease and one independent package
  reviewer;
- incompatible role, model, risk, scope, base revision, and independence
  assignments each rejecting reuse independently;
- profile escalation replacing rather than mutating the current lease;
- temporary recovery session closing before dispatch when its expiry predicate
  fires;
- each mandatory review-trigger category producing an explicit independent
  review assignment;
- writer and fixer reports running the project harness before acceptance and
  reuse;
- a host without follow-up support batching assignments before spawn;
- clean standalone install that performs no Superpowers network or file action;
- phase-lazy explicit optional-methodology install path and visible requested
  integration failure;
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
- session bootstrap and context reload;
- assignments per spawn, reuse count, and churn rate;
- total effective tokens;
- raw host telemetry and its source;
- elapsed time;
- unsupported or unknown routing outcomes.

No token-saving claim is valid unless quality gates are equivalent and retries,
escalations, reviews, and fixes are included.

## Acceptance Criteria

- One command displays every run, work package, assignment, attempt, session,
  lease, and routing decision with applicable ownership, lifecycle state,
  current step, freshness, blocker, and next action; inapplicable fields are
  omitted rather than invented.
- Web and CLI status views are projections of the same durable facts.
- The Web leads with work-package progress and exposes session topology as a
  secondary resource view.
- The dashboard remains useful with no observer agent.
- Default installation performs no Superpowers clone, fetch, checkout, or skill
  copy, and standalone mode passes the complete development gates.
- Six compatible sequential assignments use at most one writer session and one
  independent package-review session in the churn fixture.
- Incompatible work fails lease validation and receives a correctly isolated
  execution path.
- Every host spawn has durable planned intent before the host call, and replay
  recovers safely from a crash at each dispatch boundary.
- Initial session limits remain two open and four total unless the durable report
  records an evidence-backed override.
- Exactly one assignment writes to overlapping scope at a time even when its
  session is reused.
- Every legacy mandatory-review trigger creates independent auditable review
  work, and no writer/fixer session is its own reviewer.
- Every writer/fixer report is waited, file-backed, committed, and followed by
  the project harness before acceptance or reuse.
- Temporary and replacement sessions carry explicit expiry predicates and close
  before further dispatch when those predicates fire.
- Requested and actual host/model behavior are never conflated.
- Unknown or unsupported capabilities are visible and auditable.
- Codex, Claude, and OpenCode adapters pass the shared contract suite and
  opt-in real smoke protocol before being labeled first-class.
- Passive scanning performs no agent spawn and consumes no model tokens.
- Active probes disclose cost and cleanup before running.
- The router respects risk and role floors, supports manual elevation, and
  escalates without same-tier retry loops. Unknown or below-floor actual
  capability fails closed before assignment delivery, and a profile change uses
  a new compatible execution path.
- Token-efficiency reports include failed attempts, escalations, review, and
  fixer work.
- Completed and superseded sessions are clearly separated from active work and
  satisfy the lifecycle audit.
- Missing telemetry is `unknown`, never zero or fabricated.
- Existing prompt-shape and lifecycle verification continue to pass.
- Every implementation increment maps acceptance evidence to all affected
  numbered invariants.

## Delivery Boundaries

This is an umbrella architecture, not authorization for one monolithic change.
Implementation planning must create a separate, reviewable plan for each
independently testable increment. The first implementation plan covers only
increment 1; later increments start after the prior increment passes its gates.
Write a short delta design before a later increment only when new evidence
changes this approved architecture.

Deliver increments in this order:

1. standalone methodology, invariant contract, and dependency-free installer;
2. run, package, assignment, attempt, session, lease, route, and event schema
   migration;
3. dispatch decisions, session reuse, lease enforcement, and churn benchmark;
4. CLI `status/watch` and safe telemetry ingestion;
5. local read-only work-package dashboard and token-economy views;
6. passive capability scanner and adapter contract;
7. Codex, Claude, and OpenCode first-class adapters;
8. rule-based lease routing and requested-versus-actual accounting;
9. desktop bridge and active probes;
10. routing evaluation and optional observer analysis.

Each increment must preserve existing release verification and may not claim
cross-host or token-efficiency success before its corresponding evidence exists.
