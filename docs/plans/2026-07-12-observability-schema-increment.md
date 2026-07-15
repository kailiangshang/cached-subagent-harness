# Observability Schema Increment Implementation Plan

Status: **Historical / Superseded.** This event-store increment was implemented
as a development experiment and then deliberately removed before installation
when the product was simplified. Do not execute this plan against the current
tree. See [Current Product State](../current-state.md). Unchecked boxes retain
the original TDD record; they are not current backlog.

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `test-driven-development` task-by-task. The controller uses
> `cached-subagent-harness`, keeps one compatible writer session for all tasks,
> and runs one independent package review after the writer report and full
> harness evidence exist. Steps use checkbox (`- [ ]`) syntax for durable
> tracking.

**Goal:** Deliver the increment-2 normalized SQLite schema, truthful legacy
migration, atomic compatibility dual-write, append-only event ingestion, and
deterministic projection/replay foundations without implementing a later
runtime or UI increment.

**Architecture:** Keep `main.rs` as the CLI/prompt boundary, move connection and
versioned migration ownership into `schema.rs`, move the existing physical
ledger contract into `ledger.rs`, and place typed event append/reducer/replay
behavior in one `event_store.rs`. Use bundled SQLite, explicit transactions,
deferred restrictive foreign keys, JSON1 validation, and no new crate
dependency. The same writer performs the tasks serially and commits each
independently testable checkpoint.

**Tech Stack:** Rust 2024, `rusqlite 0.32` with bundled SQLite, Python standard
library release tests, Bash verification, SQLite WAL/foreign keys/JSON1.

## Global Constraints

- Read first:
  `docs/specs/2026-07-10-agent-control-plane-design.md`,
  `docs/specs/2026-07-12-observability-schema-delta.md`, and
  `increment-2-observability-schema.tmp`.
- Preserve all twenty non-negotiable invariants and map final evidence to the
  affected invariant numbers.
- Preserve the physical `agent_ledger`, all four existing ledger commands, all
  flags/defaults/boolean spellings/output/exit behavior, duplicate-handle
  behavior, automatic close flag, budget counts, and final-audit semantics.
- Legacy input may seed only `agent_sessions.session_id`, `handle`, `role`,
  `status`, nonempty `spawned_at`, and nonempty `final_reason`. It must not
  synthesize any other domain fact.
- `CURRENT_SCHEMA_VERSION` is `1`; `PRAGMA user_version` is authoritative and
  is set last inside one `BEGIN IMMEDIATE` migration transaction.
- Enable foreign keys and a five-second busy timeout before migration. Validate
  current structure read-only. Activate WAL only after logical migration
  succeeds.
- Support exactly empty version-zero databases, the historical ledger without
  `final_reason`, and the current legacy ledger. Reject partial, ambiguous,
  malformed, corrupt, current-but-structurally-wrong, and future layouts
  without repair or replacement.
- JSON uses bundled SQLite JSON1 and the exact version-1 envelopes in the delta
  spec. Do not add `serde`, an ORM, a migration framework, `chrono`, `tempfile`,
  a system SQLite dependency, or any other crate.
- New timestamps are exact UTC RFC3339 milliseconds. Legacy timestamp text is
  preserved verbatim and bypasses new-write canonicalization.
- All identifiers are nonempty application-supplied text. Unknown facts remain
  SQL null or an approved explicit `unknown`; numeric zero is an observed zero.
- All historical foreign keys are restrictive. Circular projection pointers
  are nullable and deferred. Foreign keys must never be disabled to make a
  write pass.
- Events are append-only, ordered by unique `(run_id, sequence)`, and
  idempotent at `(run_id, source_kind, source_id, idempotency_key)`.
- Attempt usage is attempt-scoped delta usage. Session usage is the latest
  lifetime cumulative observation. Never sum both layers.
- Monetary values use integer amount/scale/unit-or-currency triplets. No SQL
  `REAL`, Rust float, or cross-unit/currency aggregation.
- At most one lease per session is `active` or `idle`; multiple `planned`
  leases are allowed but cannot be used.
- Increment 2 adds no host call, dispatch policy, session-follow-up behavior,
  routing selection, status/watch command, Web asset, scanner, adapter, bridge,
  observer, token-savings claim, installation, or installed-skill update.
- Follow RED-GREEN-REFACTOR for every behavior. Record the exact failing test
  output before production code, then the focused passing command in
  `/tmp/increment-2-writer.md`.
- Allowed production/test paths are only:
  `skills/cached-subagent-harness/scripts/harnessctl/src/main.rs`,
  `skills/cached-subagent-harness/scripts/harnessctl/src/schema.rs`,
  `skills/cached-subagent-harness/scripts/harnessctl/src/ledger.rs`,
  `skills/cached-subagent-harness/scripts/harnessctl/src/event_store.rs`,
  `scripts/validate-release.py`, and
  `scripts/test_standalone_contract.py`.
- `Cargo.toml`, `Cargo.lock`, prompt text, skill/reference files, installer
  behavior, public CLI usage, and the approved specs are read-only for the
  writer.

## File Structure

| Path | Responsibility |
|---|---|
| `skills/cached-subagent-harness/scripts/harnessctl/src/main.rs` | Prompt/CLI parsing and output only; declares modules and converts ledger flags to typed inputs/patches. |
| `skills/cached-subagent-harness/scripts/harnessctl/src/schema.rs` | Connection configuration, exact layout detection, complete DDL, version migration, legacy import, structural validation, JSON/timestamp primitives, schema fixtures. |
| `skills/cached-subagent-harness/scripts/harnessctl/src/ledger.rs` | Existing metadata/add/update/audit contract, typed atomic patch, ledger/session compatibility dual-write, ledger unit tests. |
| `skills/cached-subagent-harness/scripts/harnessctl/src/event_store.rs` | Event registry, canonical input, idempotent append, sequence allocation, reducers, evidence provenance, usage rules, replay into an empty current database, event tests. |
| `scripts/validate-release.py` | Source-package manifest requiring all Rust modules. |
| `scripts/test_standalone_contract.py` | Negative package tests for missing Rust modules. |

The current schema contains these exact application tables:

```text
harness_meta
agent_ledger
runs
work_packages
work_package_dependencies
assignments
assignment_attempts
agent_sessions
session_leases
routing_decisions
control_plane_events
run_event_counters
legacy_agent_ledger_import
projection_field_sources
```

Use every logical field in the umbrella design plus every delta field. Physical
delta columns are exactly:

```text
runs: token_budget_mode, next_action, ended_at
work_packages: ended_at
assignments: next_action, ended_at
assignment_attempts: next_action
agent_sessions: run_id, token_budget_mode, next_action, ended_at,
  interrupted_at, interruption_reason, superseded_by_session_id, superseded_at
session_leases: next_action, ended_at
routing_decisions: next_action
```

`agent_sessions.run_id` is nullable only to preserve unattached legacy imports.
Every `session_planned` event supplies the event's run ID and the exact
version-1 spawn-authorization payload from D9, including budget reason and
explicit nested-delegation authority.

The Rust version-1 registries are exact:

```text
routing status: requested, applied, inherited, unsupported, degraded, rejected, unknown
risk: low, medium, high, critical
role floor: discussion, explorer, worker, reviewer, fixer
review policy: none, deterministic, independent
independence policy: none, different-session, different-role-and-session
telemetry quality: exact, partial, estimated, unsupported, unknown
session outcome: success, failure, abandonment, unknown
source kind: host-runtime, harness-operation, controller-observation, agent-report, inference
```

Physical usage/cost columns on attempts and sessions are exactly:

```text
input_tokens INTEGER NULL
output_tokens INTEGER NULL
reasoning_tokens INTEGER NULL
cache_read_tokens INTEGER NULL
cache_write_tokens INTEGER NULL
credits_amount INTEGER NULL
credits_scale INTEGER NULL
credits_unit TEXT NULL
cost_amount INTEGER NULL
cost_scale INTEGER NULL
cost_currency TEXT NULL
telemetry_source TEXT NULL
```

The three credit columns are all null or all nonnull; the three cost columns are
all null or all nonnull. Amounts and token counts are nonnegative, scales are in
`0..=12`, and a source is required when any usage field is present.

---

### Task 1: Versioned schema and atomic legacy migration

**Files:**

- Create: `skills/cached-subagent-harness/scripts/harnessctl/src/schema.rs`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/main.rs`
- Test: inline `schema::tests`

**Interfaces:**

```rust
// schema.rs
pub(crate) const CURRENT_SCHEMA_VERSION: i32 = 1;

#[derive(Debug)]
enum SchemaError {
    Sqlite(rusqlite::Error),
    UnsupportedVersion(i32),
    AmbiguousLayout(String),
    StructuralMismatch(String),
    Integrity(String),
    InvalidTimestamp(String),
    InvalidJson(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnversionedLayout {
    Empty,
    LegacyWithoutFinalReason,
    LegacyCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonTopLevel {
    Object,
    Array,
}

pub(crate) fn open_db(path: &str) -> Result<rusqlite::Connection, String>;
pub(crate) fn initialize_connection(
    conn: &mut rusqlite::Connection,
    file_backed: bool,
) -> Result<(), SchemaError>;
pub(crate) fn validate_canonical_timestamp(value: &str) -> Result<(), SchemaError>;
pub(crate) fn canonical_json(
    conn: &rusqlite::Connection,
    value: &str,
    top_level: JsonTopLevel,
) -> Result<String, SchemaError>;
```

`open_db` preserves parent-directory creation and never deletes, truncates,
renames over, vacuums, or replaces a failing database. `initialize_connection`
exists so tests can initialize in-memory connections without pretending WAL is
available there. Implement `Display`, `std::error::Error`, and
`From<rusqlite::Error>` for `SchemaError` by hand; do not add `thiserror`.

- [ ] **Step 1: Write the fresh-schema RED test**

Add `fresh_database_has_complete_versioned_schema`. Use a standard-library RAII
temporary path whose `Drop` removes `.db`, `-wal`, and `-shm`. Assert:

```rust
assert_eq!(pragma_i32(&conn, "user_version"), 1);
assert_eq!(table_names(&conn), EXPECTED_TABLES);
assert!(named_indexes(&conn).contains("ux_control_plane_events_run_sequence"));
assert!(named_indexes(&conn).contains("ux_session_leases_one_usable"));
assert!(foreign_key_check(&conn).is_empty());
assert_eq!(pragma_text(&conn, "journal_mode"), "wal");
assert_eq!(row_count(&conn, "control_plane_events"), 0);
```

Also execute one invalid child insert, one invalid state insert, one invalid
boolean insert, one incomplete cost triplet insert, and two active/idle leases
for one session. Each must return a constraint error. Verify two planned leases
for that session are accepted.

- [ ] **Step 2: Run the test and capture the expected RED**

Run:

```bash
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml schema::tests::fresh_database_has_complete_versioned_schema -- --nocapture
```

Expected: compilation/test failure because `schema` and `open_db` do not yet
exist. Record the output in the writer report before implementation.

- [ ] **Step 3: Implement connection configuration and the complete DDL**

Move only database-opening/schema code from `main.rs`. Declare:

```rust
mod schema;
use schema::open_db;
```

Use `Connection::busy_timeout(Duration::from_secs(5))`,
`PRAGMA foreign_keys=ON`, a JSON1 probe, and explicit current DDL without repair
`IF NOT EXISTS`. Define named indexes for every ownership/status lookup in the
spec, including:

```sql
CREATE UNIQUE INDEX ux_control_plane_events_run_sequence
ON control_plane_events(run_id, sequence);
CREATE UNIQUE INDEX ux_control_plane_events_idempotency
ON control_plane_events(run_id, source_kind, source_id, idempotency_key);
CREATE UNIQUE INDEX ux_session_leases_one_usable
ON session_leases(session_id) WHERE status IN ('active','idle');
```

All event-to-projection and circular pointer foreign keys are
`DEFERRABLE INITIALLY DEFERRED`; all deletes/updates are restrictive. Create
`control_plane_events` with update/delete abort triggers named
`trg_control_plane_events_no_update` and
`trg_control_plane_events_no_delete`.

`validate_canonical_timestamp` accepts exactly 24-byte UTC values such as
`2026-07-12T09:08:07.006Z`, validates calendar/time ranges including leap years,
and rejects offsets, missing milliseconds, leap seconds, and whitespace.
`canonical_json` uses JSON1 to reject invalid JSON, duplicate object keys,
wrong top-level type, and noncompact storage.

- [ ] **Step 4: Run the fresh-schema test to GREEN**

Run the Step 2 command. Expected: one passing test and zero warnings.

- [ ] **Step 5: Write layout/migration RED fixtures**

Add these independent tests using literal historical DDL copied from the
shipped commits rather than production constants:

```text
legacy_without_final_reason_migrates_atomically
legacy_current_migrates_without_losing_ledger_facts
current_database_reopen_is_idempotent
invalid_legacy_state_rolls_back_entire_migration
partial_unversioned_target_layout_is_rejected_unchanged
legacy_missing_non_migratable_column_is_rejected
future_schema_version_is_rejected_unchanged
corrupt_database_is_not_replaced
current_version_missing_required_index_is_rejected
concurrent_openers_observe_one_committed_migration
canonical_timestamp_rejects_noncanonical_values
canonical_json_rejects_duplicates_unknown_shape_and_whitespace
```

The two legacy fixtures include Unicode task/path/reason/next-action values,
empty and nonempty timestamps/reasons, every existing lifecycle state, and
nondefault booleans. Snapshot all thirteen physical ledger columns before
migration and compare byte-for-byte afterward. Assert only honest session
columns are populated, `run_id` is null, all other normalized entity/event
tables are empty, and import provenance exists exactly once.

- [ ] **Step 6: Run each new fixture and verify the intended RED**

Run:

```bash
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml schema::tests -- --nocapture
```

Expected: the fresh test stays green and each new behavior fails for its named
missing migration/validation reason, not a fixture syntax error.

- [ ] **Step 7: Implement exact layout classification and migration**

Algorithm:

```text
configure connection
read user_version
if version == 1: validate exact required columns/indexes/FKs/triggers; no write
if version > 1 or nonzero != 1: unsupported-version error; no write
if version == 0: BEGIN IMMEDIATE; re-read version; classify exact layout
empty: create all current objects
historical/current legacy: retain harness_meta and agent_ledger, add only the
  historical final_reason column if absent, create only the new target objects,
  import honest sessions and provenance
other: ambiguous-layout error before DDL
validate manifest + foreign_key_check
set user_version = 1 last
commit
file-backed only: set and verify journal_mode=WAL
```

The current-layout validator compares required table columns (name, affinity,
nullability, primary-key position), named index columns/partial SQL, foreign-key
targets/actions/deferred declarations, and append-only trigger presence. A
current mismatch returns a structural error; it never runs repair DDL.

- [ ] **Step 8: Run schema tests and refactor only while green**

Run:

```bash
cargo fmt --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml schema::tests -- --nocapture
```

Expected: every schema test passes. Keep all fixture helpers test-only.

- [ ] **Step 9: Commit Task 1**

```bash
git add skills/cached-subagent-harness/scripts/harnessctl/src/main.rs skills/cached-subagent-harness/scripts/harnessctl/src/schema.rs
git commit -m "feat: add versioned observability schema"
```

Record the commit and all RED/GREEN commands in `/tmp/increment-2-writer.md`.

### Task 2: Atomic legacy ledger compatibility

**Files:**

- Create: `skills/cached-subagent-harness/scripts/harnessctl/src/ledger.rs`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/main.rs`
- Test: inline `ledger::tests` and existing CLI unit tests

**Interfaces:**

```rust
pub(crate) struct AgentInput {
    pub(crate) handle: String,
    pub(crate) role: String,
    pub(crate) task: String,
    pub(crate) status: String,
    pub(crate) report_path: String,
    pub(crate) spawned_at: String,
    pub(crate) waited: bool,
    pub(crate) closed: bool,
    pub(crate) write_scope: String,
    pub(crate) token_risk: String,
    pub(crate) final_reason: String,
    pub(crate) next_action: String,
}

#[derive(Default)]
pub(crate) struct AgentPatch {
    pub(crate) status: Option<String>,
    pub(crate) report_path: Option<String>,
    pub(crate) waited: Option<bool>,
    pub(crate) closed: Option<bool>,
    pub(crate) write_scope: Option<String>,
    pub(crate) token_risk: Option<String>,
    pub(crate) final_reason: Option<String>,
    pub(crate) next_action: Option<String>,
}

pub(crate) fn set_meta(conn: &mut Connection, key: &str, value: &str) -> Result<(), String>;
pub(crate) fn get_meta_usize(conn: &Connection, key: &str, default_value: usize) -> Result<usize, String>;
pub(crate) fn ledger_add(conn: &mut Connection, input: &AgentInput) -> Result<(), String>;
pub(crate) fn ledger_update(conn: &mut Connection, handle: &str, patch: &AgentPatch) -> Result<(), String>;
pub(crate) fn ledger_audit(conn: &Connection, mode: &str, max_concurrent: usize, max_total: usize) -> Result<Vec<String>, String>;
```

- [ ] **Step 1: Write atomic dual-write RED tests**

Move the three existing audit tests without changing their assertions, then add:

```text
legacy_add_updates_compatibility_and_session_atomically
legacy_add_rolls_back_when_session_insert_fails
legacy_update_validates_before_dual_write
legacy_update_rolls_back_when_session_update_fails
legacy_update_preserves_unknown_handle_and_close_semantics
legacy_audit_continues_to_count_physical_rows
```

Assert the compatibility tuple remains exact; normalized mapping is only
handle/session ID, role, status, nonempty spawn time, and nonempty final reason.
Every requested/actual model/reasoning/budget/usage/outcome/time not present in
legacy input remains null or budget mode `unknown`. Unmapped ledger fields stay
only in `agent_ledger`.

- [ ] **Step 2: Run ledger tests and verify RED**

```bash
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml ledger::tests -- --nocapture
```

Expected: compile/test failures for missing module and atomic operations.

- [ ] **Step 3: Extract and implement typed transactional operations**

Move metadata, add, update, boolean parsing support needed by ledger code, and
audit into `ledger.rs`. Keep CLI flag parsing/output in `main.rs`.

`ledger_add` validates the entire input before opening a transaction, inserts
the complete physical row, inserts the honest session and one import-provenance
row, then commits. `ledger_update` loads the current role/scope, validates every
patch member and the automatic `status=closed => closed=true` rule before any
write, then updates both tables in one transaction. Use one SQL update per table
or a stable typed statement; do not retain the current partial multi-update
sequence.

The session role never mutates. The normalized final reason maps empty string to
null; spawned time maps empty string to null. `next_action`, task, report path,
scope, risk, waited, closed bit, and legacy update timestamp remain physical
compatibility facts only.

- [ ] **Step 4: Run focused and complete crate tests to GREEN**

```bash
cargo fmt --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml ledger::tests -- --nocapture
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
```

Expected: all old and new tests pass with unchanged CLI strings.

- [ ] **Step 5: Commit Task 2**

```bash
git add skills/cached-subagent-harness/scripts/harnessctl/src/main.rs skills/cached-subagent-harness/scripts/harnessctl/src/ledger.rs
git commit -m "refactor: make ledger compatibility atomic"
```

### Task 3: Typed append, idempotency, and run-local sequencing

**Files:**

- Create: `skills/cached-subagent-harness/scripts/harnessctl/src/event_store.rs`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/main.rs`
- Test: inline `event_store::tests`

**Interfaces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EventInput {
    pub(crate) event_id: String,
    pub(crate) run_id: String,
    pub(crate) package_id: Option<String>,
    pub(crate) assignment_id: Option<String>,
    pub(crate) attempt_id: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) lease_id: Option<String>,
    pub(crate) event_type: String,
    pub(crate) source_kind: String,
    pub(crate) source_id: String,
    pub(crate) confidence: Option<i64>,
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
    pub(crate) idempotency_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AppendDisposition {
    Appended,
    AlreadyIngested,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppendResult {
    pub(crate) event_id: String,
    pub(crate) sequence: i64,
    pub(crate) disposition: AppendDisposition,
}

pub(crate) fn append_event(
    conn: &mut Connection,
    event: &EventInput,
) -> Result<AppendResult, String>;
```

The module is crate-private and has no new CLI command in this increment.

- [ ] **Step 1: Write registry and common validation RED tests**

Add:

```text
event_registry_matches_approved_initial_set
event_registry_has_unique_names_and_assignment_planned
event_rejects_unknown_type_source_confidence_or_identity
event_rejects_noncanonical_payload_or_timestamp
event_identity_chain_rejects_cross_run_and_cross_owner_ids
```

The expected event set is every event in the delta transition table, exactly
once. The source set is exactly the five D5 values. Confidence accepts null,
zero, and 10000 and rejects -1 and 10001.

- [ ] **Step 2: Run registry tests and verify RED**

```bash
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml event_store::tests::event_registry -- --nocapture
```

Expected: missing module/registry failures.

- [ ] **Step 3: Implement the typed registry and common validators**

Declare `mod event_store;` in `main.rs`. Use a closed Rust enum or one constant
rule per event; `EventKind::parse` rejects unknown values. Each rule stores its
entity kind, legal source states, target/effect, required entity references,
allowed source minimum where applicable, and exact ordered payload keys.

Canonicalize only by accepting already canonical version-1 JSON; do not silently
rewrite a producer's ambiguous payload. Validate entity ownership before
sequence allocation.

- [ ] **Step 4: Write append/idempotency/concurrency RED tests**

Add:

```text
first_append_allocates_sequence_one_and_projects_run
identical_retry_returns_original_without_counter_advance
same_key_different_content_is_idempotency_conflict
duplicate_event_id_in_another_scope_is_conflict
failed_validation_does_not_advance_counter
concurrent_run_local_sequences_are_unique_and_gap_free
different_runs_allocate_independent_sequences
events_reject_update_and_delete
```

The concurrency test uses two file-backed connections and a barrier. For one
run, committed sequences must be exactly `1..=N` after sorting. A conflicting
retry must leave event count, counter, projections, and provenance snapshots
unchanged.

- [ ] **Step 5: Run append tests and verify RED**

```bash
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml event_store::tests -- --nocapture
```

Expected: registry tests stay green; append tests fail because storage and
allocation are missing.

- [ ] **Step 6: Implement one-transaction append**

Use `TransactionBehavior::Immediate`. Inside the transaction:

```text
validate input and canonical payload
look up the producer-scoped idempotency key
if present: compare semantic fields and return original or conflict
validate entity chain and transition
read/update run_event_counters.next_sequence
generate ingested_at with SQLite UTC milliseconds
insert event
apply reducer and provenance
run deferred ownership consistency checks
commit
```

The idempotency comparison excludes only the offered retry event ID, assigned
sequence, and ingested time. It includes occurred time and every semantic
reference/value. Allocate no sequence before the duplicate decision.

Task 3 must fully reduce `run_planned` so a run and its counter can be created by
the first event. Until Task 4 adds the remaining reducers, their rules return
`event_reducer_unavailable` with no mutation; the internal API cannot silently
store an unreduced state change.

- [ ] **Step 7: Run event tests to GREEN and commit**

```bash
cargo fmt --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml event_store::tests -- --nocapture
git add skills/cached-subagent-harness/scripts/harnessctl/src/main.rs skills/cached-subagent-harness/scripts/harnessctl/src/event_store.rs
git commit -m "feat: add idempotent event append"
```

### Task 4: Deterministic reducers, evidence conflicts, usage, and replay

**Files:**

- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/event_store.rs`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/schema.rs`
- Test: inline `event_store::tests` and `schema::tests`

**Interfaces:**

```rust
pub(crate) fn replay_run_into_empty(
    source: &Connection,
    target: &mut Connection,
    run_id: &str,
) -> Result<(), String>;
```

The target must be a current, empty application database. Replay copies stored
events in run sequence, preserves event ID/sequence/source/payload/timestamps,
applies the same reducer, reconstructs the next counter, and rejects gaps or an
already-present target run. It never mutates the source or deletes source
events.

- [ ] **Step 1: Write transition-table RED tests**

Create one table-driven case for every event rule. For each rule assert its legal
source/effect and at least one illegal source. Add focused chains:

```text
canonical_run_package_assignment_attempt_session_lease_route_chain
reported_is_not_validated_or_accepted
package_review_verdict_controls_resume_or_completion
assignment_requeue_requires_failed_current_attempt
interrupted_and_superseded_are_facets_not_states
lease_expired_revoked_closed_order_is_exact
one_usable_lease_is_enforced_during_reduce
circular_projection_pointers_preserve_ownership
package_dependencies_reject_cross_run_self_and_cycle
```

The canonical chain asserts all counters, current pointers, blockers,
next-actions, evidence, and terminal timestamps after every append.

- [ ] **Step 2: Run transition tests and verify RED**

```bash
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml event_store::tests -- --nocapture
```

Expected: Task-3 tests stay green; non-run reducers report unavailable.

- [ ] **Step 3: Implement all reducers from the delta table**

Use one match arm per typed event or small helpers shared only by identical
field mechanics. Payload validation names exact required/optional keys and
rejects extras. Creation reducers insert complete projection rows; transition
reducers update only fields named by the event and provenance rules.

Reducer ordering is fixed:

```text
insert event
create owner with nullable circular pointers when needed
create children/dependencies
validate run/package/assignment/attempt/session/lease/route ownership
update pointers and counters
apply winning field values and projection_field_sources
verify reducer-specific invariants
commit
```

An event with weaker conflicting evidence remains stored and increments
`conflict_count` but does not change the winning field. Same-priority later
sequence wins. Stronger correction requires `supersedes_event_id` and a valid
target value. Confidence never changes the comparator.

- [ ] **Step 4: Write evidence/usage RED tests**

Add:

```text
weaker_late_evidence_never_overwrites_stronger
same_priority_later_sequence_wins_and_records_conflict
stronger_correction_can_replace_weaker_projection
corroborating_equal_value_is_not_a_conflict
attempt_deltas_and_session_cumulative_usage_do_not_double_count
session_cumulative_counter_cannot_decrease_without_correction
exact_cost_rejects_partial_float_negative_or_cross_currency_sum
unknown_usage_remains_null_not_zero
```

Assert both projection values and the exact
`projection_field_sources` winner/conflict row.

- [ ] **Step 5: Implement evidence and usage projection rules**

Map source priority to integers `1..=5` only for comparison. Store source kind,
winner event, and sequence as canonical provenance. Populate amount/scale/unit
triplets atomically. Attempt `usage_observed` accepts delta/correction only;
session accepts cumulative/correction only. Never compute a combined total in
storage.

- [ ] **Step 6: Write replay RED tests**

Add:

```text
replay_reproduces_every_projection_and_provenance_row
replay_rejects_sequence_gap
replay_rejects_nonempty_target
replay_is_independent_of_occurred_timestamp_order
legacy_import_is_not_fabricated_into_replay_events
```

Build a run containing package blocking/unblocking, failed attempt/requeue,
route degradation, writer report/validation/acceptance, session reuse facet,
usage correction, review evidence, terminal lease/session/package/run events,
and at least one retained weaker conflict. Compare sorted, type-preserving rows
from every run-owned projection, event, counter, dependency, and provenance
table between source and replay target.

- [ ] **Step 7: Implement replay and run all Rust tests to GREEN**

```bash
cargo fmt --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml schema::tests -- --nocapture
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml ledger::tests -- --nocapture
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml event_store::tests -- --nocapture
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
cargo clippy --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml -- -D warnings
```

Expected: all commands exit zero with no warnings.

- [ ] **Step 8: Commit Task 4**

```bash
git add skills/cached-subagent-harness/scripts/harnessctl/src/schema.rs skills/cached-subagent-harness/scripts/harnessctl/src/event_store.rs
git commit -m "feat: add deterministic lifecycle projections"
```

### Task 5: Release packaging and complete regression gate

**Files:**

- Modify: `scripts/validate-release.py`
- Modify: `scripts/test_standalone_contract.py`

**Interfaces:** no runtime API change.

- [ ] **Step 1: Write missing-module RED test**

Add a helper that copies the release fixture, removes one source path, runs the
validator, and returns its process result. Add:

```python
def test_release_validator_requires_every_rust_module(self) -> None:
    modules = [
        "scripts/harnessctl/src/schema.rs",
        "scripts/harnessctl/src/ledger.rs",
        "scripts/harnessctl/src/event_store.rs",
    ]
    for module in modules:
        with self.subTest(module=module):
            result = self.run_validation_without_skill_file(module)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(f"missing skill file: {module}", result.stderr)
```

- [ ] **Step 2: Run the test and verify RED**

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.test_standalone_contract.StandaloneContractTests.test_release_validator_requires_every_rust_module
```

Expected: failure because the validator does not require the new modules.

- [ ] **Step 3: Add exact module paths to release validation**

Extend `required_files` in `validate_skill` with:

```python
"scripts/harnessctl/src/schema.rs",
"scripts/harnessctl/src/ledger.rs",
"scripts/harnessctl/src/event_store.rs",
```

Do not change plugin, installer, invariant, methodology, or Superpowers checks.

- [ ] **Step 4: Run focused release tests to GREEN**

```bash
python3 scripts/validate-release.py .
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts/test_standalone_contract.py
```

Expected: validator prints `release metadata validation passed`; all standalone
contract tests pass.

- [ ] **Step 5: Run the full project harness**

```bash
scripts/verify.sh
git status --short
```

Expected: `verification passed`. Status contains only deliberate source/test
changes and the ignored task report/ledger; it contains no target directory,
binary, fixture DB/WAL/SHM file, or Python cache.

- [ ] **Step 6: Self-review scope and invariant evidence**

In `/tmp/increment-2-writer.md`, record:

```text
Status
Problem / Scenarios / Options / Chosen Plan
Allowed Write Paths
Files Changed
Tests with RED and GREEN commands/results
Commits
Invariant evidence for 1-20 where affected
Legacy fact-preservation matrix
Known risks and explicit increment-3+ exclusions
Follow-up
```

Search the diff for host invocation, new CLI commands, Web assets, adapters,
scanners, routing policy, installer changes, Superpowers changes, dependency
changes, placeholder timestamps, zero-filled unknowns, `REAL`, and disabled
foreign keys. Any match must be removed or explained as a test assertion.

- [ ] **Step 7: Commit Task 5**

```bash
git add scripts/validate-release.py scripts/test_standalone_contract.py
git commit -m "test: validate observability runtime packaging"
```

## Controller Verification and Review Gate

The controller, not the writer, performs these after consuming the writer
report:

1. Compare the writer diff to both approved specs and every Global Constraint.
2. Re-run focused schema, ledger, event-store, Python contract, release, fmt,
   clippy, and complete `scripts/verify.sh` commands from a clean process.
3. Generate one review package from base commit `273c3fd` through writer HEAD.
4. Dispatch the planned independent reviewer with only the plan, writer report,
   and review-package paths. The reviewer returns separate Spec Verdict and
   Quality Verdict.
5. If any Critical or Important finding exists, reuse the writer for one batched
   fixer pass, require covering tests and a fix commit, then re-dispatch the same
   reviewer against an updated review package.
6. Run `ledger-audit --mode final`, close every increment-2 session, update
   `increment-2-observability-schema.tmp`, and only then mark increment 2
   complete and begin increment 3.
