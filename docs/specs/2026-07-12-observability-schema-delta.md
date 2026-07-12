# Observability Schema Delta

Status: approved technical closure for delivery increment 2

Applies to the normalized SQLite schema, legacy migration, internal event
append API, and deterministic projection/replay foundations. This document
closes the storage ambiguities in the approved agent control-plane design. If
this delta and the umbrella design differ on a physical storage detail, this
delta governs increment 2; the twenty non-negotiable invariants continue to
govern all behavior.

## Scope

Increment 2 delivers storage contracts only. It does not dispatch work, reuse a
session, select a model, call a host, expose status/watch, serve a Dashboard,
scan capabilities, implement adapters or bridges, run an observer, or install
or update any skill.

The physical `agent_ledger` table and its current CLI/audit semantics remain
the compatibility surface. Migration may seed only factually equivalent
`agent_sessions` fields. It must not synthesize a run, package, assignment,
attempt, lease, route, event, model, usage value, timestamp, successful
outcome, or authorization.

## Closed Decisions

### D1: assignment creation is typed

Add `assignment_planned` to the initial event registry. It creates an
assignment in `planned` state. No package payload or generic transition may
silently create an assignment.

### D2: sessions have truthful run ownership

Add nullable `run_id` to `agent_sessions`, referencing `runs` with restrictive
update/delete behavior.

- Every session created through `session_planned` must have `run_id` equal to
  the event's run.
- A migrated legacy session has `run_id = NULL`; absence remains visible and is
  also recorded in `legacy_agent_ledger_import`.
- An unattached session cannot receive a normal lease, attempt, route, or new
  control-plane event until an evidence-backed reconciliation explicitly
  attaches it. Increment 2 does not implement that reconciliation.
- Ownership is never inferred from the first lease, attempt, or event.

### D3: dependencies are relational; other structures are versioned JSON

Package dependencies use
`work_package_dependencies(package_id, depends_on_package_id)` with a composite
primary key and restrictive foreign keys. Rust validation requires both
packages to belong to the same run, rejects self-dependency, and rejects a
cycle before commit.

All other structured values are compact JSON1 text produced and validated by
the store. Bundled SQLite JSON1 is a runtime requirement and is probed during
connection validation; it is not a system package dependency. Stored JSON
must have no duplicate object keys, use the field order below, contain no
unknown fields, and equal JSON1's compact `json(...)` result. Arrays whose
meaning is a set are sorted bytewise and duplicate-free.

Version 1 envelopes are:

```text
scope       {"v":1,"paths":[<nonempty string>...]}
budget      {"v":1,"max_open":<uint>,"max_total":<uint>,"override_reason":<string|null>}
policy      {"v":1,"kind":<registry value>}
evidence    {"v":1,"items":[{"kind":<nonempty string>,"ref":<nonempty string>,"result":<nonempty string>}]}
event       {"v":1,...event-specific fields in registry order...}
```

`max_total` must be at least `max_open`. `override_reason` is required when the
normal two-open/four-total budget is raised and otherwise may be null. Empty
read-only scope is `{"v":1,"paths":[]}`. Evidence items preserve occurrence
order because order may be meaningful; scope paths do not.

### D4: version-1 registries are explicit

Closed state, role, assignment-kind, model-profile, eligibility,
budget-enforcement, and close-disposition registries remain exactly as listed
in the umbrella design. The following version-1 registries are validated in
Rust rather than frozen into SQL `CHECK` clauses, so a later deliberate
registry expansion does not require destructive table repair:

| Registry | Version-1 values |
|---|---|
| routing status | `requested`, `applied`, `inherited`, `unsupported`, `degraded`, `rejected`, `unknown` |
| risk class/floor | `low`, `medium`, `high`, `critical` |
| role floor | `discussion`, `explorer`, `worker`, `reviewer`, `fixer` |
| review policy | `none`, `deterministic`, `independent` |
| independence policy | `none`, `different-session`, `different-role-and-session` |
| telemetry quality | `exact`, `partial`, `estimated`, `unsupported`, `unknown` |
| session outcome | `success`, `failure`, `abandonment`, `unknown` |

Telemetry provenance uses the event source-kind registry in D5. Unknown text is
rejected for new writes instead of being accepted as an accidental extension.
Legacy rows are copied only where their existing closed role/status values are
already valid.

### D5: source priority and confidence are separate facts

`source_kind` is one of the following ordered classes; lower priority number is
stronger evidence:

| Priority | `source_kind` |
|---:|---|
| 1 | `host-runtime` |
| 2 | `harness-operation` |
| 3 | `controller-observation` |
| 4 | `agent-report` |
| 5 | `inference` |

`source_id` is a nonempty stable producer identifier within the run.
`confidence` is nullable integer basis points in `0..=10000`. Null means no
confidence fact. Confidence is displayed and audited but never outranks source
priority or changes a transition's authorization.

### D6: conflicts retain both provenance and a deterministic winner

For each projected field, evidence comparison is:

1. stronger `source_kind` wins regardless of arrival time or confidence;
2. at the same priority, the later stored run sequence wins;
3. byte-equivalent canonical values are corroboration, not conflict.

Weaker later evidence is appended but cannot overwrite stronger projection
state. Stronger late evidence may correct a weaker projection even when the
weaker value would make the ordinary forward transition illegal. Such a
correction must name the prior winning event in `supersedes_event_id`, and the
target value must itself be valid for that entity.

Add the infrastructure projection
`projection_field_sources(run_id, entity_kind, entity_id, field_name,
winner_event_id, winner_source_kind, winner_sequence, conflict_count,
last_conflict_event_id)`. Its primary key is `(entity_kind, entity_id,
field_name)`; winner and conflict event IDs reference `control_plane_events`.
The event log retains every claim, while this table makes the current winner
and conflict count queryable without an LLM. Replay rebuilds this table from
sequence one and must reproduce it byte-for-byte.

### D7: idempotency distinguishes retry from disagreement

Idempotency scope is `(run_id, source_kind, source_id, idempotency_key)`.
Within one `BEGIN IMMEDIATE` append transaction:

- no prior key: validate, allocate the next run-local sequence, append, reduce,
  and commit;
- prior key with identical semantic content: return the original event ID and
  sequence as `already_ingested`, without advancing the counter or mutating a
  projection;
- prior key with different semantic content: return
  `idempotency_conflict`, without advancing the counter or mutating anything.

Semantic comparison includes entity references, event type, source fields,
confidence, canonical payload, and `occurred_at`. It excludes the retry's
offered `event_id`, the core-assigned sequence, and `ingested_at`. A globally
duplicate `event_id` under a different idempotency scope is always a conflict.

### D8: next action is a first-class nullable fact

Add nullable `next_action` to runs, assignments, assignment attempts,
agent sessions, session leases, and routing decisions. Work packages retain
their existing field. Null means inapplicable or not observed, never an empty
placeholder.

New exceptional session projections (`failed`, `abandoned`, or
`externally-unknown`) require both nonempty `final_reason` and `next_action`.
Migrated legacy rows remain exempt from this new-write guard because the
physical ledger is the only honest source for their historical next action.

### D9: `session_planned` is the durable spawn authorization

The event must be committed before a later increment may call a host. Its exact
version-1 shape is:

```text
{
  "v": 1,
  "authorization_ref": <nonempty-string>,
  "budget_reason": <nonempty-string>,
  "run_max_open": <uint>,
  "run_max_total": <uint>,
  "session_token_budget": {"mode": <bounded|unbounded>, "tokens": <uint|null>},
  "nested_delegation": {"allowed": <boolean>, "authority_ref": <string|null>},
  "requested_host": <string|null>,
  "requested_profile": <light|standard|deep>,
  "requested_model": <string|null>,
  "requested_reasoning": <string|null>,
  "parent_session_id": <string|null>
}
```

Angle-bracket strings above denote typed schema slots and are never stored
literally. The planner supplies every value; the schema supplies no budget
default. `authorization_ref` and `budget_reason` are always nonempty.
If nested delegation is allowed, `authority_ref` and `parent_session_id` are
nonempty and the durable run budget must record the user authority. If it is
not allowed, both are null. The payload cannot authorize itself and
`parent_handle` alone is never authority.

### D10: attempts are deltas; sessions are cumulative observations

Attempt usage columns store only usage attributable to that attempt. Session
usage columns store the latest accepted lifetime cumulative observation for the
host session. Reports sum attempt deltas for work cost or display session
cumulative totals, but never add the two layers together.

`usage_observed` has the exact common payload:

```text
{"v":1,"scope":"attempt|session","subject_id":<id>,"observation_kind":"delta|cumulative|correction","window_start":<UTC|null>,"window_end":<UTC>,"input_tokens":<uint|null>,"output_tokens":<uint|null>,"reasoning_tokens":<uint|null>,"cache_read_tokens":<uint|null>,"cache_write_tokens":<uint|null>,"credits":<exact-units|null>,"provider_cost":<exact-cost|null>,"telemetry_quality":<registry>,"supersedes_event_id":<id|null>}
```

Attempt scope accepts `delta` or `correction`; session scope accepts
`cumulative` or `correction`. A session cumulative update cannot decrease a
known counter unless `observation_kind=correction`, it names the superseded
event, and its evidence wins under D6. Null counters remain unknown and do not
erase an observed value unless an authorized correction explicitly supplies
the field as unknown.

### D11: budgets and monetary values are exact and typed

Runs and sessions store `token_budget_mode` as `bounded`, `unbounded`, or
`unknown`. `token_budget` is a nonnegative integer exactly when mode is
`bounded`, and null otherwise. New planned sessions must use `bounded` or
`unbounded`; migrated absence is `unknown`.

Logical credit and provider-cost values use integer units:

```text
credits       {"amount":<int64>,"scale":<0..=12>,"unit":<nonempty provider unit>}
provider_cost {"amount":<int64>,"scale":<0..=12>,"currency":<ISO-4217 uppercase code>}
```

`amount / 10^scale` is the exact value. All three members are present together
or the whole value is null. Negative values are rejected. Values with
different units or currencies are never aggregated. Physical columns may use
the corresponding amount/scale/unit or amount/scale/currency triplets; `REAL`
and floating-point arithmetic are forbidden.

### D12: canonical new timestamps and generic terminal time

New timestamps use UTC RFC3339 with millisecond precision exactly:
`YYYY-MM-DDTHH:MM:SS.sssZ`. Producer timestamps are validated; `ingested_at` is
generated inside the append transaction. Run-local sequence, not time, orders
replay.

Add `ended_at` to runs, work packages, assignments, agent sessions, and session
leases. Attempts retain their existing `ended_at`.

- `completed_at` applies only to successful completion/acceptance where such a
  field exists.
- `closed_at` records session closure, not failure or abandonment.
- `ended_at` records any terminal lifecycle result, including cancellation,
  failure, abandonment, expiry, revocation, or closure.
- Migrated legacy timestamp text is copied verbatim when factually mappable; it
  is not rewritten, supplemented, or treated as a newly canonical timestamp.

### D13: only one usable lease exists per session

At most one lease per session may be `active` or `idle`. Enforce this with a
partial unique index on `session_id WHERE status IN ('active','idle')`.
Multiple `planned` leases may coexist for crash recovery, but a planned lease
is never dispatchable and issuing one fails while another active/idle lease
exists. Increment 2 stores and tests the invariant; reuse policy remains in
increment 3.

## Event Transition Registry

Every event uses a version-1 payload, exact entity ownership, and a transition
from this table. “Facet” means canonical lifecycle state is unchanged. Creation
events reject an existing entity; terminal events reject an already terminal
entity except an identical idempotent retry.

| Event | Legal source → target or effect | Required payload beyond `v` |
|---|---|---|
| `run_planned` | absent → `planned` | complete run creation fields |
| `run_started` | `planned` → `active` | `psoc_revision`, `started_at`, `next_action` |
| `run_blocked` | `active` → `blocked` | `blocker`, `next_action` |
| `run_unblocked` | `blocked` → `active` | `resolution`, `next_action` |
| `run_completed` | `active` → `complete` | `completed_at`, `ended_at` |
| `run_cancelled` | `planned|active|blocked` → `cancelled` | `reason`, `ended_at`, `next_action` |
| `package_planned` | absent → `planned` | complete package creation fields |
| `package_ready` | `planned` → `ready` | `next_action` |
| `package_active` | `ready` → `active` | `next_action` |
| `package_blocked` | `ready|active|review` → `blocked` | `blocker`, `resume_status`, `next_action` |
| `package_unblocked` | `blocked` → saved `ready|active|review` | `resolution`, matching `resume_status`, `next_action` |
| `package_review_started` | `active` → `review` | `review_assignment_id`, `next_action` |
| `package_review_completed` | `review` → `review` for `accepted`; `review` → `active` for `changes_required` | `verdict`, `review_evidence`, `next_action` |
| `package_completed` | accepted `review` → `complete`; `active` → `complete` only for policy `none` | `ended_at` |
| `package_cancelled` | any nonterminal package → `cancelled` | `reason`, `ended_at`, `next_action` |
| `assignment_planned` | absent → `planned` | complete assignment creation fields |
| `assignment_queued` | `planned` → `queued` | `next_action` |
| `assignment_started` | `queued` → `running` | `attempt_id`, `started_at`, `current_step` |
| `assignment_step_changed` | `running` facet | `current_step` |
| `assignment_blocked` | `queued|running|reported|validated` facet | `blocker`, `next_action` |
| `assignment_unblocked` | same state with blocker → same state without blocker | `resolution`, `next_action` |
| `assignment_reported` | `running` → `reported` | `report_path`, `reported_at`, `next_action` |
| `assignment_validated` | `reported` → `validated` | `test_evidence`, `validated_at`, `next_action` |
| `assignment_accepted` | `validated` → `accepted` | `review_evidence`, `accepted_at`, `ended_at` |
| `assignment_requeued` | `queued|running|reported|validated` with failed current attempt → `queued` | `reason`, `next_action` |
| `assignment_failed` | `queued|running|reported|validated` → `failed` | `policy_exhausted=true`, `final_reason`, `ended_at`, `next_action` |
| `assignment_cancelled` | any nonterminal assignment → `cancelled` | `final_reason`, `ended_at`, `next_action` |
| `attempt_planned` | absent → `planned`; increment assignment attempt count and pointer atomically | complete attempt creation fields |
| `attempt_started` | `planned` → `running` | `started_at`, `next_action` |
| `attempt_reported` | `running` → `reported` | `reported_at`, `next_action` |
| `attempt_validated` | `reported` → `validated` | `validated_at`, `next_action` |
| `attempt_accepted` | `validated` → `accepted` | `accepted_at`, `ended_at` |
| `attempt_failed` | `planned|running|reported|validated` → `failed` | `outcome_reason`, `ended_at`, `next_action` |
| `attempt_cancelled` | any nonterminal attempt → `cancelled` | `outcome_reason`, `ended_at`, `next_action` |
| `dispatch_main_selected`, `dispatch_reuse_selected`, `dispatch_batch_selected`, `dispatch_spawn_selected` | planned attempt decision fact; state unchanged | strategy-specific identity, reason, and authorization reference |
| `route_requested` | absent → routing status `requested`, eligibility `unknown` | complete immutable route intent |
| `route_applied` | `requested|degraded` → `applied|inherited`, eligibility `eligible` | actual model/reasoning and eligibility evidence |
| `route_degraded` | `requested` → `degraded` | actual result, reason, explicit eligibility/evidence, next action |
| `route_rejected` | `requested|degraded` → `rejected`, eligibility `rejected` | reason, eligibility evidence, next action |
| `session_planned` | absent → `planned` | D9 authorization payload and complete session intent |
| `session_spawned` | `planned` → `spawned` | actual handle, host, `spawned_at`, next action |
| `session_running` | `spawned|reported` → `running` | `started_or_resumed_at`, lease/attempt IDs, next action; reported source additionally requires consumed prior report and passed gate evidence |
| `session_heartbeat` | `spawned|running|reported` facet | `last_activity_at` |
| `session_blocked` | `spawned|running|reported` facet | `blocker`, `next_action` |
| `session_unblocked` | same state with blocker → same state without blocker | `resolution`, `next_action` |
| `session_reported` | `running` → `reported` | `last_reported_at`, assignment/attempt IDs, next action |
| `session_waited` | `reported|failed|abandoned|externally-unknown` facet | `last_waited_at`, consumed report reference or terminal observation |
| `session_failed` | any nonterminal session → `failed` | `final_reason`, `ended_at`, `next_action` |
| `session_abandoned` | any nonterminal session → `abandoned` | `final_reason`, `ended_at`, `next_action` |
| `session_externally_unknown` | any nonterminal session → `externally-unknown` | `final_reason`, `ended_at`, `next_action` |
| `session_interrupted` | `spawned|running|reported` facet | `interruption_reason`, `interrupted_at`, `next_action`; outcome becomes `unknown` until stronger evidence resolves it |
| `session_close_requested` | any not-closed session facet | `close_requested_at`, `next_action`; disposition becomes `requested` |
| `session_closed` | any not-closed session → `closed` | `outcome`, `close_disposition`, `closed_at`, `ended_at` |
| `session_superseded` | any not-closed session facet | `superseded_by_session_id`, `superseded_at`, `reason`, `next_action`; must be followed by close/finalization |
| `lease_planned` | absent → `planned` | complete lease compatibility fields |
| `lease_issued` | `planned` → `active` | `issued_at`, `next_action` |
| `lease_reused` | `idle` → `active`; increment `reuse_count` | attempt ID, validated compatibility evidence, `last_used_at`, next action |
| `lease_idle` | `active` → `idle` | `last_used_at`, next action |
| `lease_expired` | `planned|active|idle` → `expired` | `expiry_reason`, `ended_at`, next action |
| `lease_revoked` | `planned|active|idle` → `revoked` | `expiry_reason`, `ended_at`, next action |
| `lease_closed` | `planned|active|idle|expired|revoked` → `closed` | `expiry_reason`, `ended_at` |
| `usage_observed` | usage projection only | exact D10 payload |
| `quality_gate_passed` | evidence projection only | subject entity, policy, evidence, observed time |
| `quality_gate_failed` | evidence projection only | subject entity, policy, findings reference, next action |

`session_interrupted` additionally projects nullable `interrupted_at` and
`interruption_reason`; `session_superseded` projects nullable
`superseded_by_session_id` and `superseded_at`. These are auditable facets, not
new lifecycle states.

Circular updates use one reducer order in a single transaction: create owning
projection with nullable pointers, create child rows, validate the complete
ownership chain, update pointers/counters, update field-source provenance, then
commit. Foreign keys remain enabled throughout.

## Migration and Version Contract

`PRAGMA user_version` is authoritative and `CURRENT_SCHEMA_VERSION = 1` because
all previously shipped databases are genuinely version zero.

On every connection, enable foreign keys and a bounded five-second busy timeout
before migration. For version zero, acquire `BEGIN IMMEDIATE`, re-read the
version, and classify exactly one layout:

1. empty application database;
2. historical `agent_ledger` without `final_reason`;
3. current legacy `agent_ledger` with `final_reason`.

Any partial target object, missing nonhistorical legacy column, ambiguous
layout, corrupt database, or future version fails closed without repair. Create
the current schema, add the one historical column if needed, copy only honest
session facts, validate structure and `foreign_key_check`, and set
`user_version` last in the same transaction. Activate and verify WAL only after
logical migration commits. A current-version structural mismatch is an error,
never an auto-repair opportunity.

Legacy import provenance uses the complete legacy handle as
`legacy_row_key` and deterministic `session_id`; `run_id` remains null. Existing
`ledger-add` and `ledger-update` validate all inputs first, then atomically
mutate the physical ledger row and only the honestly mappable session fields.
Audit and budget calculations continue to read physical `agent_ledger`.

Legacy migration creates no control-plane events. Deterministic replay applies
only to runs whose facts entered through the versioned event API.

## Increment-2 Acceptance Evidence

Increment 2 is complete only when tests prove:

- exact fresh/current/historical/partial/future/corrupt layout behavior;
- atomic migration rollback and concurrent opener serialization;
- byte-preservation of all legacy facts and zero fabricated work-graph facts;
- current CLI, boolean, budget, close, duplicate-handle, and final-audit
  compatibility;
- atomic compatibility dual-write of honestly mappable session fields;
- complete event/transition/payload registry coverage, including
  `assignment_planned`;
- run-local monotonic sequence under concurrency;
- identical retry and conflicting duplicate semantics;
- append-only event enforcement and deterministic replay;
- evidence-priority conflict retention and winner provenance;
- no double-counted usage, no floating-point cost, and unknown-not-zero;
- identity-chain, pointer, dependency-cycle, and one-usable-lease constraints;
- release packaging includes every new Rust module; and
- the complete existing `scripts/verify.sh` gate passes without installing or
  updating the skill.

An independent package reviewer must find no open Critical or Important issue.
The increment may claim replay/storage foundations only, not dispatch, reuse,
routing selection, status, Web UI, host portability, or token savings.
