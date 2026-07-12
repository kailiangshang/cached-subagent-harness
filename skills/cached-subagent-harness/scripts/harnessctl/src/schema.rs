use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::Duration;

pub(crate) const CURRENT_SCHEMA_VERSION: i32 = 1;

const LEGACY_TABLES_DDL: &str = r#"
CREATE TABLE harness_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE agent_ledger (
    handle TEXT PRIMARY KEY CHECK(length(handle) > 0),
    role TEXT NOT NULL CHECK(role IN ('discussion','explorer','worker','reviewer','fixer')),
    task TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK(status IN ('planned','spawned','running','reported','closed','failed','abandoned','externally-unknown')),
    report_path TEXT NOT NULL DEFAULT '',
    spawned_at TEXT NOT NULL DEFAULT '',
    waited INTEGER NOT NULL DEFAULT 0 CHECK(typeof(waited)='integer' AND waited IN (0,1)),
    closed INTEGER NOT NULL DEFAULT 0 CHECK(typeof(closed)='integer' AND closed IN (0,1)),
    write_scope TEXT NOT NULL DEFAULT 'none',
    token_risk TEXT NOT NULL DEFAULT '',
    final_reason TEXT NOT NULL DEFAULT '',
    next_action TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

const TARGET_TABLES_DDL: &str = r#"
CREATE TABLE runs (
    run_id TEXT PRIMARY KEY CHECK(length(run_id) > 0),
    goal TEXT NOT NULL CHECK(length(goal) > 0),
    psoc_revision TEXT,
    status TEXT NOT NULL CHECK(status IN ('planned','active','blocked','complete','cancelled')),
    session_budget TEXT NOT NULL CHECK(json_valid(session_budget) AND json_type(session_budget)='object' AND json(session_budget)=session_budget),
    token_budget INTEGER CHECK(token_budget IS NULL OR (typeof(token_budget)='integer' AND token_budget >= 0)),
    token_budget_mode TEXT NOT NULL CHECK(token_budget_mode IN ('bounded','unbounded','unknown')),
    report_path TEXT NOT NULL CHECK(length(report_path) > 0),
    ledger_path TEXT NOT NULL CHECK(length(ledger_path) > 0),
    next_action TEXT,
    started_at TEXT,
    completed_at TEXT,
    ended_at TEXT,
    CHECK((token_budget_mode='bounded' AND token_budget IS NOT NULL) OR (token_budget_mode IN ('unbounded','unknown') AND token_budget IS NULL))
);

CREATE TABLE work_packages (
    package_id TEXT PRIMARY KEY CHECK(length(package_id) > 0),
    run_id TEXT NOT NULL,
    title TEXT NOT NULL CHECK(length(title) > 0),
    role_floor TEXT NOT NULL CHECK(role_floor IN ('discussion','explorer','worker','reviewer','fixer')),
    model_floor TEXT NOT NULL CHECK(model_floor IN ('light','standard','deep')),
    risk_floor TEXT NOT NULL CHECK(risk_floor IN ('low','medium','high','critical')),
    write_scope TEXT NOT NULL CHECK(json_valid(write_scope) AND json_type(write_scope)='object' AND json(write_scope)=write_scope),
    review_policy TEXT NOT NULL CHECK(json_valid(review_policy) AND json_type(review_policy)='object' AND json(review_policy)=review_policy),
    independence_policy TEXT NOT NULL CHECK(json_valid(independence_policy) AND json_type(independence_policy)='object' AND json(independence_policy)=independence_policy),
    status TEXT NOT NULL CHECK(status IN ('planned','ready','active','review','complete','blocked','cancelled')),
    blocker TEXT,
    next_action TEXT,
    ended_at TEXT,
    UNIQUE(package_id, run_id),
    FOREIGN KEY(run_id) REFERENCES runs(run_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    CHECK(status <> 'blocked' OR (blocker IS NOT NULL AND length(blocker) > 0))
);

CREATE TABLE work_package_dependencies (
    package_id TEXT NOT NULL,
    depends_on_package_id TEXT NOT NULL,
    PRIMARY KEY(package_id, depends_on_package_id),
    FOREIGN KEY(package_id) REFERENCES work_packages(package_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(depends_on_package_id) REFERENCES work_packages(package_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    CHECK(package_id <> depends_on_package_id)
);

CREATE TABLE assignments (
    assignment_id TEXT PRIMARY KEY CHECK(length(assignment_id) > 0),
    package_id TEXT NOT NULL,
    title TEXT NOT NULL CHECK(length(title) > 0),
    sequence INTEGER NOT NULL CHECK(typeof(sequence)='integer' AND sequence >= 1),
    assignment_kind TEXT NOT NULL CHECK(assignment_kind IN ('discussion','exploration','implementation','review','fix')),
    required_role TEXT NOT NULL CHECK(required_role IN ('discussion','explorer','worker','reviewer','fixer')),
    model_floor TEXT NOT NULL CHECK(model_floor IN ('light','standard','deep')),
    risk_class TEXT NOT NULL CHECK(risk_class IN ('low','medium','high','critical')),
    write_scope TEXT NOT NULL CHECK(json_valid(write_scope) AND json_type(write_scope)='object' AND json(write_scope)=write_scope),
    base_revision TEXT,
    independence_boundary_id TEXT,
    current_attempt_id TEXT,
    attempt_count INTEGER NOT NULL CHECK(typeof(attempt_count)='integer' AND attempt_count >= 0),
    status TEXT NOT NULL CHECK(status IN ('planned','queued','running','reported','validated','accepted','failed','cancelled')),
    current_step TEXT,
    blocker TEXT,
    next_action TEXT,
    report_path TEXT,
    test_evidence TEXT CHECK(test_evidence IS NULL OR (json_valid(test_evidence) AND json_type(test_evidence)='object' AND json(test_evidence)=test_evidence)),
    review_evidence TEXT CHECK(review_evidence IS NULL OR (json_valid(review_evidence) AND json_type(review_evidence)='object' AND json(review_evidence)=review_evidence)),
    started_at TEXT,
    reported_at TEXT,
    validated_at TEXT,
    accepted_at TEXT,
    ended_at TEXT,
    final_reason TEXT,
    UNIQUE(assignment_id, package_id),
    UNIQUE(package_id, sequence),
    FOREIGN KEY(package_id) REFERENCES work_packages(package_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(current_attempt_id, assignment_id) REFERENCES assignment_attempts(attempt_id, assignment_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE agent_sessions (
    session_id TEXT PRIMARY KEY CHECK(length(session_id) > 0),
    run_id TEXT,
    handle TEXT,
    parent_handle TEXT,
    host_id TEXT,
    role TEXT NOT NULL CHECK(role IN ('discussion','explorer','worker','reviewer','fixer')),
    requested_profile TEXT CHECK(requested_profile IS NULL OR requested_profile IN ('light','standard','deep')),
    requested_model TEXT,
    actual_model TEXT,
    requested_reasoning TEXT,
    actual_reasoning TEXT,
    routing_status TEXT CHECK(routing_status IS NULL OR routing_status IN ('requested','applied','inherited','unsupported','degraded','rejected','unknown')),
    routing_reason TEXT,
    token_budget INTEGER CHECK(token_budget IS NULL OR (typeof(token_budget)='integer' AND token_budget >= 0)),
    token_budget_mode TEXT NOT NULL CHECK(token_budget_mode IN ('bounded','unbounded','unknown')),
    budget_enforcement TEXT CHECK(budget_enforcement IS NULL OR budget_enforcement IN ('enforced','advisory','unsupported','unknown')),
    status TEXT NOT NULL CHECK(status IN ('planned','spawned','running','reported','closed','failed','abandoned','externally-unknown')),
    next_action TEXT,
    spawned_at TEXT,
    last_activity_at TEXT,
    last_reported_at TEXT,
    last_waited_at TEXT,
    outcome TEXT CHECK(outcome IS NULL OR outcome IN ('success','failure','abandonment','unknown')),
    close_disposition TEXT CHECK(close_disposition IS NULL OR close_disposition IN ('not-requested','requested','confirmed','unsupported','unknown')),
    close_requested_at TEXT,
    closed_at TEXT,
    ended_at TEXT,
    interrupted_at TEXT,
    interruption_reason TEXT,
    superseded_by_session_id TEXT,
    superseded_at TEXT,
    telemetry_quality TEXT CHECK(telemetry_quality IS NULL OR telemetry_quality IN ('exact','partial','estimated','unsupported','unknown')),
    input_tokens INTEGER CHECK(input_tokens IS NULL OR (typeof(input_tokens)='integer' AND input_tokens >= 0)),
    output_tokens INTEGER CHECK(output_tokens IS NULL OR (typeof(output_tokens)='integer' AND output_tokens >= 0)),
    reasoning_tokens INTEGER CHECK(reasoning_tokens IS NULL OR (typeof(reasoning_tokens)='integer' AND reasoning_tokens >= 0)),
    cache_read_tokens INTEGER CHECK(cache_read_tokens IS NULL OR (typeof(cache_read_tokens)='integer' AND cache_read_tokens >= 0)),
    cache_write_tokens INTEGER CHECK(cache_write_tokens IS NULL OR (typeof(cache_write_tokens)='integer' AND cache_write_tokens >= 0)),
    credits_amount INTEGER,
    credits_scale INTEGER,
    credits_unit TEXT,
    cost_amount INTEGER,
    cost_scale INTEGER,
    cost_currency TEXT,
    telemetry_source TEXT,
    final_reason TEXT,
    UNIQUE(session_id, run_id),
    FOREIGN KEY(run_id) REFERENCES runs(run_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(superseded_by_session_id) REFERENCES agent_sessions(session_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    CHECK((token_budget_mode='bounded' AND token_budget IS NOT NULL) OR (token_budget_mode IN ('unbounded','unknown') AND token_budget IS NULL)),
    CHECK((credits_amount IS NULL AND credits_scale IS NULL AND credits_unit IS NULL) OR (typeof(credits_amount)='integer' AND credits_amount >= 0 AND typeof(credits_scale)='integer' AND credits_scale BETWEEN 0 AND 12 AND credits_unit IS NOT NULL AND length(credits_unit) > 0)),
    CHECK((cost_amount IS NULL AND cost_scale IS NULL AND cost_currency IS NULL) OR (typeof(cost_amount)='integer' AND cost_amount >= 0 AND typeof(cost_scale)='integer' AND cost_scale BETWEEN 0 AND 12 AND cost_currency GLOB '[A-Z][A-Z][A-Z]')),
    CHECK((input_tokens IS NULL AND output_tokens IS NULL AND reasoning_tokens IS NULL AND cache_read_tokens IS NULL AND cache_write_tokens IS NULL AND credits_amount IS NULL AND cost_amount IS NULL) OR (telemetry_source IS NOT NULL AND length(telemetry_source) > 0))
);

CREATE TABLE session_leases (
    lease_id TEXT PRIMARY KEY CHECK(length(lease_id) > 0),
    session_id TEXT NOT NULL,
    package_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('discussion','explorer','worker','reviewer','fixer')),
    model_profile TEXT NOT NULL CHECK(model_profile IN ('light','standard','deep')),
    risk_class TEXT NOT NULL CHECK(risk_class IN ('low','medium','high','critical')),
    write_scope TEXT NOT NULL CHECK(json_valid(write_scope) AND json_type(write_scope)='object' AND json(write_scope)=write_scope),
    base_revision TEXT NOT NULL CHECK(length(base_revision) > 0),
    independence_boundary_id TEXT,
    current_attempt_id TEXT,
    replaces_session_id TEXT,
    expiry_predicate TEXT,
    status TEXT NOT NULL CHECK(status IN ('planned','active','idle','expired','revoked','closed')),
    reuse_count INTEGER NOT NULL CHECK(typeof(reuse_count)='integer' AND reuse_count >= 0),
    next_action TEXT,
    issued_at TEXT,
    last_used_at TEXT,
    expires_at TEXT,
    expiry_reason TEXT,
    ended_at TEXT,
    UNIQUE(lease_id, session_id),
    FOREIGN KEY(session_id) REFERENCES agent_sessions(session_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(package_id) REFERENCES work_packages(package_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(current_attempt_id) REFERENCES assignment_attempts(attempt_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(replaces_session_id) REFERENCES agent_sessions(session_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    CHECK((replaces_session_id IS NULL AND expiry_predicate IS NULL) OR (replaces_session_id IS NOT NULL AND length(replaces_session_id) > 0 AND expiry_predicate IS NOT NULL AND length(expiry_predicate) > 0))
);

CREATE TABLE assignment_attempts (
    attempt_id TEXT PRIMARY KEY CHECK(length(attempt_id) > 0),
    assignment_id TEXT NOT NULL,
    session_id TEXT,
    lease_id TEXT,
    attempt_sequence INTEGER NOT NULL CHECK(typeof(attempt_sequence)='integer' AND attempt_sequence >= 1),
    route_id TEXT,
    status TEXT NOT NULL CHECK(status IN ('planned','running','reported','validated','accepted','failed','cancelled')),
    next_action TEXT,
    started_at TEXT,
    reported_at TEXT,
    validated_at TEXT,
    accepted_at TEXT,
    ended_at TEXT,
    outcome_reason TEXT,
    input_tokens INTEGER CHECK(input_tokens IS NULL OR (typeof(input_tokens)='integer' AND input_tokens >= 0)),
    output_tokens INTEGER CHECK(output_tokens IS NULL OR (typeof(output_tokens)='integer' AND output_tokens >= 0)),
    reasoning_tokens INTEGER CHECK(reasoning_tokens IS NULL OR (typeof(reasoning_tokens)='integer' AND reasoning_tokens >= 0)),
    cache_read_tokens INTEGER CHECK(cache_read_tokens IS NULL OR (typeof(cache_read_tokens)='integer' AND cache_read_tokens >= 0)),
    cache_write_tokens INTEGER CHECK(cache_write_tokens IS NULL OR (typeof(cache_write_tokens)='integer' AND cache_write_tokens >= 0)),
    credits_amount INTEGER,
    credits_scale INTEGER,
    credits_unit TEXT,
    cost_amount INTEGER,
    cost_scale INTEGER,
    cost_currency TEXT,
    telemetry_source TEXT,
    UNIQUE(attempt_id, assignment_id),
    UNIQUE(assignment_id, attempt_sequence),
    FOREIGN KEY(assignment_id) REFERENCES assignments(assignment_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(session_id) REFERENCES agent_sessions(session_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(lease_id, session_id) REFERENCES session_leases(lease_id, session_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(route_id, attempt_id) REFERENCES routing_decisions(route_id, attempt_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    CHECK((lease_id IS NULL) = (session_id IS NULL) OR lease_id IS NULL),
    CHECK((credits_amount IS NULL AND credits_scale IS NULL AND credits_unit IS NULL) OR (typeof(credits_amount)='integer' AND credits_amount >= 0 AND typeof(credits_scale)='integer' AND credits_scale BETWEEN 0 AND 12 AND credits_unit IS NOT NULL AND length(credits_unit) > 0)),
    CHECK((cost_amount IS NULL AND cost_scale IS NULL AND cost_currency IS NULL) OR (typeof(cost_amount)='integer' AND cost_amount >= 0 AND typeof(cost_scale)='integer' AND cost_scale BETWEEN 0 AND 12 AND cost_currency GLOB '[A-Z][A-Z][A-Z]')),
    CHECK((input_tokens IS NULL AND output_tokens IS NULL AND reasoning_tokens IS NULL AND cache_read_tokens IS NULL AND cache_write_tokens IS NULL AND credits_amount IS NULL AND cost_amount IS NULL) OR (telemetry_source IS NOT NULL AND length(telemetry_source) > 0))
);

CREATE TABLE routing_decisions (
    route_id TEXT PRIMARY KEY CHECK(length(route_id) > 0),
    attempt_id TEXT NOT NULL,
    required_profile TEXT NOT NULL CHECK(required_profile IN ('light','standard','deep')),
    requested_model TEXT,
    requested_reasoning TEXT,
    actual_model TEXT,
    actual_reasoning TEXT,
    routing_status TEXT NOT NULL CHECK(routing_status IN ('requested','applied','inherited','unsupported','degraded','rejected','unknown')),
    eligibility_status TEXT NOT NULL CHECK(eligibility_status IN ('eligible','rejected','unknown')),
    eligibility_evidence TEXT CHECK(eligibility_evidence IS NULL OR (json_valid(eligibility_evidence) AND json_type(eligibility_evidence)='object' AND json(eligibility_evidence)=eligibility_evidence)),
    escalated_from_route_id TEXT,
    next_action TEXT,
    decided_at TEXT NOT NULL CHECK(length(decided_at) > 0),
    UNIQUE(route_id, attempt_id),
    FOREIGN KEY(attempt_id) REFERENCES assignment_attempts(attempt_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(escalated_from_route_id) REFERENCES routing_decisions(route_id) ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE TABLE control_plane_events (
    event_id TEXT PRIMARY KEY CHECK(length(event_id) > 0),
    run_id TEXT NOT NULL,
    package_id TEXT,
    assignment_id TEXT,
    attempt_id TEXT,
    session_id TEXT,
    lease_id TEXT,
    sequence INTEGER NOT NULL CHECK(typeof(sequence)='integer' AND sequence >= 1),
    event_type TEXT NOT NULL CHECK(length(event_type) > 0),
    source_kind TEXT NOT NULL CHECK(source_kind IN ('host-runtime','harness-operation','controller-observation','agent-report','inference')),
    source_id TEXT NOT NULL CHECK(length(source_id) > 0),
    confidence INTEGER CHECK(confidence IS NULL OR (typeof(confidence)='integer' AND confidence BETWEEN 0 AND 10000)),
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json) AND json_type(payload_json)='object' AND json(payload_json)=payload_json),
    occurred_at TEXT NOT NULL CHECK(length(occurred_at) > 0),
    ingested_at TEXT NOT NULL CHECK(length(ingested_at) > 0),
    idempotency_key TEXT NOT NULL CHECK(length(idempotency_key) > 0),
    FOREIGN KEY(run_id) REFERENCES runs(run_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(package_id) REFERENCES work_packages(package_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(assignment_id) REFERENCES assignments(assignment_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(attempt_id) REFERENCES assignment_attempts(attempt_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(session_id) REFERENCES agent_sessions(session_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(lease_id) REFERENCES session_leases(lease_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE run_event_counters (
    run_id TEXT PRIMARY KEY,
    next_sequence INTEGER NOT NULL CHECK(typeof(next_sequence)='integer' AND next_sequence >= 1),
    FOREIGN KEY(run_id) REFERENCES runs(run_id) ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE TABLE legacy_agent_ledger_import (
    legacy_row_key TEXT PRIMARY KEY CHECK(length(legacy_row_key) > 0),
    session_id TEXT NOT NULL UNIQUE,
    import_status TEXT NOT NULL CHECK(import_status='imported'),
    import_reason TEXT,
    imported_at TEXT NOT NULL CHECK(length(imported_at) > 0),
    FOREIGN KEY(session_id) REFERENCES agent_sessions(session_id) ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE TABLE projection_field_sources (
    run_id TEXT NOT NULL,
    entity_kind TEXT NOT NULL CHECK(length(entity_kind) > 0),
    entity_id TEXT NOT NULL CHECK(length(entity_id) > 0),
    field_name TEXT NOT NULL CHECK(length(field_name) > 0),
    winner_event_id TEXT NOT NULL,
    winner_source_kind TEXT NOT NULL CHECK(winner_source_kind IN ('host-runtime','harness-operation','controller-observation','agent-report','inference')),
    winner_sequence INTEGER NOT NULL CHECK(typeof(winner_sequence)='integer' AND winner_sequence >= 1),
    conflict_count INTEGER NOT NULL CHECK(typeof(conflict_count)='integer' AND conflict_count >= 0),
    last_conflict_event_id TEXT,
    PRIMARY KEY(entity_kind, entity_id, field_name),
    FOREIGN KEY(run_id) REFERENCES runs(run_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(winner_event_id) REFERENCES control_plane_events(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(last_conflict_event_id) REFERENCES control_plane_events(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);
"#;

const INDEXES_AND_TRIGGERS_DDL: &str = r#"
CREATE INDEX ix_runs_status_started ON runs(status, started_at);
CREATE INDEX ix_work_packages_run_status ON work_packages(run_id, status);
CREATE INDEX ix_work_package_dependencies_depends ON work_package_dependencies(depends_on_package_id);
CREATE INDEX ix_assignments_package_status_sequence ON assignments(package_id, status, sequence);
CREATE INDEX ix_assignments_current_attempt ON assignments(current_attempt_id);
CREATE INDEX ix_assignment_attempts_assignment_status ON assignment_attempts(assignment_id, status);
CREATE INDEX ix_assignment_attempts_session_status ON assignment_attempts(session_id, status);
CREATE INDEX ix_assignment_attempts_lease ON assignment_attempts(lease_id);
CREATE INDEX ix_assignment_attempts_route ON assignment_attempts(route_id);
CREATE INDEX ix_agent_sessions_run_status ON agent_sessions(run_id, status);
CREATE INDEX ix_agent_sessions_status_spawned ON agent_sessions(status, spawned_at);
CREATE INDEX ix_session_leases_package_status ON session_leases(package_id, status);
CREATE INDEX ix_session_leases_current_attempt ON session_leases(current_attempt_id);
CREATE UNIQUE INDEX ux_session_leases_one_usable ON session_leases(session_id) WHERE status IN ('active','idle');
CREATE INDEX ix_routing_decisions_attempt ON routing_decisions(attempt_id);
CREATE UNIQUE INDEX ux_control_plane_events_run_sequence ON control_plane_events(run_id, sequence);
CREATE UNIQUE INDEX ux_control_plane_events_idempotency ON control_plane_events(run_id, source_kind, source_id, idempotency_key);
CREATE INDEX ix_control_plane_events_package_sequence ON control_plane_events(package_id, sequence);
CREATE INDEX ix_control_plane_events_assignment_sequence ON control_plane_events(assignment_id, sequence);
CREATE INDEX ix_control_plane_events_attempt_sequence ON control_plane_events(attempt_id, sequence);
CREATE INDEX ix_control_plane_events_session_sequence ON control_plane_events(session_id, sequence);
CREATE INDEX ix_control_plane_events_lease_sequence ON control_plane_events(lease_id, sequence);
CREATE INDEX ix_projection_field_sources_winner ON projection_field_sources(winner_event_id);
CREATE TRIGGER trg_control_plane_events_no_update
BEFORE UPDATE ON control_plane_events
BEGIN
    SELECT RAISE(ABORT, 'control_plane_events is append-only');
END;
CREATE TRIGGER trg_control_plane_events_no_delete
BEFORE DELETE ON control_plane_events
BEGIN
    SELECT RAISE(ABORT, 'control_plane_events is append-only');
END;
"#;

const FRESH_DDL: &str = r#"
CREATE TABLE harness_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE agent_ledger (
    handle TEXT PRIMARY KEY CHECK(length(handle) > 0),
    role TEXT NOT NULL CHECK(role IN ('discussion','explorer','worker','reviewer','fixer')),
    task TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK(status IN ('planned','spawned','running','reported','closed','failed','abandoned','externally-unknown')),
    report_path TEXT NOT NULL DEFAULT '',
    spawned_at TEXT NOT NULL DEFAULT '',
    waited INTEGER NOT NULL DEFAULT 0 CHECK(typeof(waited)='integer' AND waited IN (0,1)),
    closed INTEGER NOT NULL DEFAULT 0 CHECK(typeof(closed)='integer' AND closed IN (0,1)),
    write_scope TEXT NOT NULL DEFAULT 'none',
    token_risk TEXT NOT NULL DEFAULT '',
    final_reason TEXT NOT NULL DEFAULT '',
    next_action TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

#[derive(Debug)]
pub(crate) enum SchemaError {
    Sqlite(rusqlite::Error),
    UnsupportedVersion(i32),
    AmbiguousLayout(String),
    StructuralMismatch(String),
    Integrity(String),
    InvalidTimestamp(String),
    InvalidJson(String),
}

impl fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "sqlite error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported schema version: {version}")
            }
            Self::AmbiguousLayout(reason) => {
                write!(formatter, "ambiguous version-zero layout: {reason}")
            }
            Self::StructuralMismatch(reason) => {
                write!(formatter, "schema structural mismatch: {reason}")
            }
            Self::Integrity(reason) => write!(formatter, "database integrity failure: {reason}"),
            Self::InvalidTimestamp(reason) => {
                write!(formatter, "invalid canonical timestamp: {reason}")
            }
            Self::InvalidJson(reason) => write!(formatter, "invalid canonical JSON: {reason}"),
        }
    }
}

impl std::error::Error for SchemaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlite(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for SchemaError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
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
    #[allow(dead_code)]
    Array,
}

pub(crate) fn open_db(path: &str) -> Result<Connection, String> {
    let db_path = Path::new(path);
    if let Some(parent) = db_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut conn = Connection::open(db_path).map_err(|error| error.to_string())?;
    initialize_connection(&mut conn, true).map_err(|error| error.to_string())?;
    Ok(conn)
}

pub(crate) fn initialize_connection(
    conn: &mut Connection,
    file_backed: bool,
) -> Result<(), SchemaError> {
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let foreign_keys: i32 = conn.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
    if foreign_keys != 1 {
        return Err(SchemaError::Integrity(
            "foreign keys could not be enabled".into(),
        ));
    }
    probe_json1(conn)?;
    check_integrity(conn)?;

    let version = user_version(conn)?;
    match version {
        CURRENT_SCHEMA_VERSION => validate_current_structure(conn)?,
        0 => migrate_version_zero(conn)?,
        other => return Err(SchemaError::UnsupportedVersion(other)),
    }

    if file_backed {
        let mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(SchemaError::Integrity(format!(
                "WAL activation returned {mode}"
            )));
        }
    }
    Ok(())
}

fn probe_json1(conn: &Connection) -> Result<(), SchemaError> {
    let value: String = conn.query_row("SELECT json('{\"probe\":1}')", [], |row| row.get(0))?;
    if value != "{\"probe\":1}" {
        return Err(SchemaError::Integrity(
            "bundled SQLite JSON1 probe failed".into(),
        ));
    }
    Ok(())
}

fn check_integrity(conn: &Connection) -> Result<(), SchemaError> {
    let result: String = conn.query_row("PRAGMA quick_check(1)", [], |row| row.get(0))?;
    if result == "ok" {
        Ok(())
    } else {
        Err(SchemaError::Integrity(result))
    }
}

fn user_version(conn: &Connection) -> Result<i32, SchemaError> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

fn migrate_version_zero(conn: &mut Connection) -> Result<(), SchemaError> {
    let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    match user_version(&transaction)? {
        CURRENT_SCHEMA_VERSION => {
            validate_current_structure(&transaction)?;
            transaction.commit()?;
            return Ok(());
        }
        0 => {}
        other => return Err(SchemaError::UnsupportedVersion(other)),
    }

    let layout = classify_unversioned_layout(&transaction)?;
    match layout {
        UnversionedLayout::Empty => transaction.execute_batch(&format!(
            "{LEGACY_TABLES_DDL}\n{TARGET_TABLES_DDL}\n{INDEXES_AND_TRIGGERS_DDL}"
        ))?,
        UnversionedLayout::LegacyWithoutFinalReason => {
            validate_legacy_rows(&transaction, false)?;
            transaction.execute(
                "ALTER TABLE agent_ledger ADD COLUMN final_reason TEXT NOT NULL DEFAULT ''",
                [],
            )?;
            transaction
                .execute_batch(&format!("{TARGET_TABLES_DDL}\n{INDEXES_AND_TRIGGERS_DDL}"))?;
            import_legacy_rows(&transaction)?;
        }
        UnversionedLayout::LegacyCurrent => {
            validate_legacy_rows(&transaction, true)?;
            transaction
                .execute_batch(&format!("{TARGET_TABLES_DDL}\n{INDEXES_AND_TRIGGERS_DDL}"))?;
            import_legacy_rows(&transaction)?;
        }
    }
    validate_current_structure(&transaction)?;
    ensure_foreign_key_check_empty(&transaction)?;
    transaction.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    transaction.commit()?;
    Ok(())
}

fn table_names(conn: &Connection) -> Result<BTreeSet<String>, SchemaError> {
    let mut statement = conn.prepare(
        "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    Ok(statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?)
}

fn classify_unversioned_layout(conn: &Connection) -> Result<UnversionedLayout, SchemaError> {
    let tables = table_names(conn)?;
    if tables.is_empty() {
        return Ok(UnversionedLayout::Empty);
    }
    let legacy: BTreeSet<String> = ["agent_ledger", "harness_meta"]
        .into_iter()
        .map(str::to_string)
        .collect();
    if tables != legacy {
        return Err(SchemaError::AmbiguousLayout(format!(
            "unexpected application tables: {tables:?}"
        )));
    }
    validate_legacy_meta_shape(conn)?;
    let columns = column_manifest(conn, "agent_ledger")?;
    let historical = legacy_agent_columns(false);
    let current = legacy_agent_columns(true);
    if columns == historical {
        Ok(UnversionedLayout::LegacyWithoutFinalReason)
    } else if columns == current {
        Ok(UnversionedLayout::LegacyCurrent)
    } else {
        Err(SchemaError::AmbiguousLayout(format!(
            "agent_ledger columns do not match either shipped layout: {columns:?}"
        )))
    }
}

type ColumnManifest = Vec<(String, String, i32, Option<String>, i32)>;

fn column_manifest(conn: &Connection, table: &str) -> Result<ColumnManifest, SchemaError> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    Ok(statement
        .query_map([], |row| {
            Ok((
                row.get(1)?,
                row.get::<_, String>(2)?.to_ascii_uppercase(),
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?)
}

fn validate_legacy_meta_shape(conn: &Connection) -> Result<(), SchemaError> {
    let expected = vec![
        ("key".into(), "TEXT".into(), 0, None, 1),
        ("value".into(), "TEXT".into(), 1, None, 0),
    ];
    let actual = column_manifest(conn, "harness_meta")?;
    if actual == expected {
        Ok(())
    } else {
        Err(SchemaError::AmbiguousLayout(format!(
            "harness_meta columns differ: {actual:?}"
        )))
    }
}

fn legacy_agent_columns(with_final_reason: bool) -> ColumnManifest {
    let mut columns = vec![
        ("handle".into(), "TEXT".into(), 0, None, 1),
        ("role".into(), "TEXT".into(), 1, None, 0),
        ("task".into(), "TEXT".into(), 1, Some("''".into()), 0),
        ("status".into(), "TEXT".into(), 1, None, 0),
        ("report_path".into(), "TEXT".into(), 1, Some("''".into()), 0),
        ("spawned_at".into(), "TEXT".into(), 1, Some("''".into()), 0),
        ("waited".into(), "INTEGER".into(), 1, Some("0".into()), 0),
        ("closed".into(), "INTEGER".into(), 1, Some("0".into()), 0),
        (
            "write_scope".into(),
            "TEXT".into(),
            1,
            Some("'none'".into()),
            0,
        ),
        ("token_risk".into(), "TEXT".into(), 1, Some("''".into()), 0),
    ];
    if with_final_reason {
        columns.push((
            "final_reason".into(),
            "TEXT".into(),
            1,
            Some("''".into()),
            0,
        ));
    }
    columns.extend([
        ("next_action".into(), "TEXT".into(), 1, Some("''".into()), 0),
        (
            "updated_at".into(),
            "TEXT".into(),
            1,
            Some("datetime('now')".into()),
            0,
        ),
    ]);
    columns
}

fn validate_legacy_rows(conn: &Connection, has_final_reason: bool) -> Result<(), SchemaError> {
    let final_expression = if has_final_reason {
        "final_reason"
    } else {
        "'' AS final_reason"
    };
    let sql = format!(
        "SELECT handle, role, status, {final_expression} FROM agent_ledger ORDER BY handle"
    );
    let mut statement = conn.prepare(&sql)?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let handle: Option<String> = row.get(0)?;
        let Some(handle) = handle.filter(|value| !value.is_empty()) else {
            return Err(SchemaError::Integrity(
                "legacy agent row has null or empty handle".into(),
            ));
        };
        let role: String = row.get(1)?;
        if !["discussion", "explorer", "worker", "reviewer", "fixer"].contains(&role.as_str()) {
            return Err(SchemaError::Integrity(format!(
                "legacy agent {handle} has invalid role {role}"
            )));
        }
        let status: String = row.get(2)?;
        if ![
            "planned",
            "spawned",
            "running",
            "reported",
            "closed",
            "failed",
            "abandoned",
            "externally-unknown",
        ]
        .contains(&status.as_str())
        {
            return Err(SchemaError::Integrity(format!(
                "legacy agent {handle} has invalid status {status}"
            )));
        }
        let _: String = row.get(3)?;
    }
    Ok(())
}

fn import_legacy_rows(conn: &Connection) -> Result<(), SchemaError> {
    let imported_at = sqlite_now(conn)?;
    conn.execute(
        r#"
        INSERT INTO agent_sessions(
            session_id, run_id, handle, role, token_budget_mode, status,
            spawned_at, final_reason
        )
        SELECT handle, NULL, handle, role, 'unknown', status,
               NULLIF(spawned_at, ''), NULLIF(final_reason, '')
        FROM agent_ledger
        "#,
        [],
    )?;
    conn.execute(
        r#"
        INSERT INTO legacy_agent_ledger_import(
            legacy_row_key, session_id, import_status, import_reason, imported_at
        )
        SELECT handle, handle, 'imported',
               'legacy agent ledger has no truthful run ownership', ?1
        FROM agent_ledger
        "#,
        [imported_at],
    )?;
    Ok(())
}

fn normalize_sql(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn object_sql(
    conn: &Connection,
    object_type: &str,
) -> Result<BTreeMap<String, String>, SchemaError> {
    let mut statement = conn.prepare(
        "SELECT name, sql FROM sqlite_schema WHERE type=?1 AND sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    Ok(statement
        .query_map([object_type], |row| {
            Ok((row.get(0)?, normalize_sql(&row.get::<_, String>(1)?)))
        })?
        .collect::<Result<_, _>>()?)
}

fn expected_connection() -> Result<Connection, SchemaError> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.execute_batch(&format!(
        "{FRESH_DDL}\n{TARGET_TABLES_DDL}\n{INDEXES_AND_TRIGGERS_DDL}"
    ))?;
    Ok(conn)
}

fn validate_current_structure(conn: &Connection) -> Result<(), SchemaError> {
    let expected = expected_connection()?;
    let expected_tables = table_names(&expected)?;
    let actual_tables = table_names(conn)?;
    if actual_tables != expected_tables {
        return Err(SchemaError::StructuralMismatch(format!(
            "table manifest expected {expected_tables:?}, got {actual_tables:?}"
        )));
    }

    for table in ["harness_meta", "agent_ledger"] {
        let expected_columns = column_manifest(&expected, table)?;
        let actual_columns = column_manifest(conn, table)?;
        if table == "agent_ledger" {
            let expected_by_name: BTreeMap<_, _> = expected_columns
                .into_iter()
                .map(|item| (item.0.clone(), item))
                .collect();
            let actual_by_name: BTreeMap<_, _> = actual_columns
                .into_iter()
                .map(|item| (item.0.clone(), item))
                .collect();
            if actual_by_name != expected_by_name {
                return Err(SchemaError::StructuralMismatch(
                    "agent_ledger column contract differs".into(),
                ));
            }
        } else if actual_columns != expected_columns {
            return Err(SchemaError::StructuralMismatch(format!(
                "{table} column contract differs"
            )));
        }
    }

    let expected_table_sql = object_sql(&expected, "table")?;
    let actual_table_sql = object_sql(conn, "table")?;
    for (name, sql) in expected_table_sql {
        if matches!(name.as_str(), "harness_meta" | "agent_ledger") {
            continue;
        }
        if actual_table_sql.get(&name) != Some(&sql) {
            return Err(SchemaError::StructuralMismatch(format!(
                "table definition differs: {name}"
            )));
        }
    }

    for object_type in ["index", "trigger"] {
        let expected_objects = object_sql(&expected, object_type)?;
        let actual_objects = object_sql(conn, object_type)?;
        if actual_objects != expected_objects {
            return Err(SchemaError::StructuralMismatch(format!(
                "{object_type} manifest differs; expected {:?}, got {:?}",
                expected_objects.keys().collect::<Vec<_>>(),
                actual_objects.keys().collect::<Vec<_>>()
            )));
        }
    }
    ensure_foreign_key_check_empty(conn)?;
    Ok(())
}

fn ensure_foreign_key_check_empty(conn: &Connection) -> Result<(), SchemaError> {
    let violation: Option<(String, i64, String)> = conn
        .query_row("PRAGMA foreign_key_check", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .optional()?;
    if let Some((table, rowid, parent)) = violation {
        Err(SchemaError::Integrity(format!(
            "foreign key violation in {table} row {rowid} against {parent}"
        )))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_canonical_timestamp(value: &str) -> Result<(), SchemaError> {
    let bytes = value.as_bytes();
    if bytes.len() != 24
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'.'
        || bytes[23] != b'Z'
        || bytes.iter().enumerate().any(|(index, byte)| {
            ![4, 7, 10, 13, 16, 19, 23].contains(&index) && !byte.is_ascii_digit()
        })
    {
        return Err(SchemaError::InvalidTimestamp(value.into()));
    }
    let parse = |range: std::ops::Range<usize>| -> Result<u32, SchemaError> {
        value[range]
            .parse()
            .map_err(|_| SchemaError::InvalidTimestamp(value.into()))
    };
    let year = parse(0..4)?;
    let month = parse(5..7)?;
    let day = parse(8..10)?;
    let hour = parse(11..13)?;
    let minute = parse(14..16)?;
    let second = parse(17..19)?;
    let _millisecond = parse(20..23)?;
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > max_day || hour > 23 || minute > 59 || second > 59 {
        return Err(SchemaError::InvalidTimestamp(value.into()));
    }
    Ok(())
}

pub(crate) fn canonical_json(
    conn: &Connection,
    value: &str,
    top_level: JsonTopLevel,
) -> Result<String, SchemaError> {
    let valid: i32 = conn.query_row("SELECT json_valid(?1)", [value], |row| row.get(0))?;
    if valid != 1 {
        return Err(SchemaError::InvalidJson("JSON1 rejected the value".into()));
    }
    let canonical: String = conn.query_row("SELECT json(?1)", [value], |row| row.get(0))?;
    if canonical != value {
        return Err(SchemaError::InvalidJson(
            "value is not compact canonical JSON".into(),
        ));
    }
    let actual_type: String = conn.query_row("SELECT json_type(?1)", [value], |row| row.get(0))?;
    let expected_type = match top_level {
        JsonTopLevel::Object => "object",
        JsonTopLevel::Array => "array",
    };
    if actual_type != expected_type {
        return Err(SchemaError::InvalidJson(format!(
            "expected top-level {expected_type}, got {actual_type}"
        )));
    }
    let duplicate: Option<String> = conn
        .query_row(
            r#"
            SELECT CAST(key AS TEXT)
            FROM json_tree(?1)
            WHERE key IS NOT NULL AND typeof(key)='text'
            GROUP BY parent, key
            HAVING COUNT(*) > 1
            LIMIT 1
            "#,
            [value],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(key) = duplicate {
        return Err(SchemaError::InvalidJson(format!(
            "duplicate object key: {key}"
        )));
    }
    Ok(canonical)
}

pub(crate) fn sqlite_now(conn: &Connection) -> Result<String, SchemaError> {
    let value: String =
        conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |row| {
            row.get(0)
        })?;
    validate_canonical_timestamp(&value)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{JsonTopLevel, canonical_json, open_db, validate_canonical_timestamp};
    use rusqlite::{Connection, params};
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);
    type HonestSessionTuple = (
        Option<String>,
        Option<String>,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        Option<i64>,
        Option<String>,
    );

    struct TempDb {
        path: PathBuf,
    }

    impl TempDb {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "harnessctl-{label}-{}-{nonce}-{id}.db",
                std::process::id()
            ));
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            for suffix in ["", "-wal", "-shm"] {
                let _ = fs::remove_file(format!("{}{}", self.path.display(), suffix));
            }
        }
    }

    fn pragma_i32(conn: &Connection, name: &str) -> i32 {
        conn.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
            .unwrap()
    }

    fn pragma_text(conn: &Connection, name: &str) -> String {
        conn.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
            .unwrap()
    }

    fn table_names(conn: &Connection) -> BTreeSet<String> {
        let mut statement = conn
            .prepare(
                "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn named_indexes(conn: &Connection) -> BTreeSet<String> {
        let mut statement = conn
            .prepare(
                "SELECT name FROM sqlite_schema WHERE type='index' AND name NOT LIKE 'sqlite_%'",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn foreign_key_check(conn: &Connection) -> Vec<String> {
        let mut statement = conn.prepare("PRAGMA foreign_key_check").unwrap();
        statement
            .query_map([], |row| {
                Ok(format!(
                    "{}:{}:{}",
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn row_count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    const HISTORICAL_LEDGER_DDL: &str = r#"
        CREATE TABLE harness_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE agent_ledger (
            handle TEXT PRIMARY KEY,
            role TEXT NOT NULL,
            task TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL,
            report_path TEXT NOT NULL DEFAULT '',
            spawned_at TEXT NOT NULL DEFAULT '',
            waited INTEGER NOT NULL DEFAULT 0,
            closed INTEGER NOT NULL DEFAULT 0,
            write_scope TEXT NOT NULL DEFAULT 'none',
            token_risk TEXT NOT NULL DEFAULT '',
            next_action TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
    "#;

    const CURRENT_LEDGER_DDL: &str = r#"
        CREATE TABLE harness_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE agent_ledger (
            handle TEXT PRIMARY KEY,
            role TEXT NOT NULL,
            task TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL,
            report_path TEXT NOT NULL DEFAULT '',
            spawned_at TEXT NOT NULL DEFAULT '',
            waited INTEGER NOT NULL DEFAULT 0,
            closed INTEGER NOT NULL DEFAULT 0,
            write_scope TEXT NOT NULL DEFAULT 'none',
            token_risk TEXT NOT NULL DEFAULT '',
            final_reason TEXT NOT NULL DEFAULT '',
            next_action TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
    "#;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct LegacyRow {
        handle: String,
        role: String,
        task: String,
        status: String,
        report_path: String,
        spawned_at: String,
        waited: i64,
        closed: i64,
        write_scope: String,
        token_risk: String,
        final_reason: String,
        next_action: String,
        updated_at: String,
    }

    fn seed_legacy(conn: &Connection, current: bool) -> Vec<LegacyRow> {
        let statuses = [
            ("planned", "discussion"),
            ("spawned", "explorer"),
            ("running", "worker"),
            ("reported", "reviewer"),
            ("closed", "fixer"),
            ("failed", "worker"),
            ("abandoned", "explorer"),
            ("externally-unknown", "reviewer"),
        ];
        let mut expected = Vec::new();
        for (index, (status, role)) in statuses.into_iter().enumerate() {
            let row = LegacyRow {
                handle: format!("会话-{index}-{status}"),
                role: role.to_string(),
                task: format!("任务-🙂-{index}"),
                status: status.to_string(),
                report_path: format!("/tmp/报告 {index}.md"),
                spawned_at: if index % 2 == 0 {
                    format!("legacy-time-{index}")
                } else {
                    String::new()
                },
                waited: i64::from(index % 2 == 0),
                closed: i64::from(index % 3 == 0),
                write_scope: if role == "worker" || role == "fixer" {
                    format!("路径/范围-{index}")
                } else {
                    "none".to_string()
                },
                token_risk: format!("风险-{index}"),
                final_reason: if current
                    && matches!(status, "failed" | "abandoned" | "externally-unknown")
                {
                    format!("终止原因-{index}")
                } else {
                    String::new()
                },
                next_action: format!("下一步-{index}"),
                updated_at: format!("legacy-updated-{index}"),
            };
            if current {
                conn.execute(
                    r#"
                    INSERT INTO agent_ledger(
                        handle,role,task,status,report_path,spawned_at,waited,closed,
                        write_scope,token_risk,final_reason,next_action,updated_at
                    ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
                    "#,
                    params![
                        row.handle,
                        row.role,
                        row.task,
                        row.status,
                        row.report_path,
                        row.spawned_at,
                        row.waited,
                        row.closed,
                        row.write_scope,
                        row.token_risk,
                        row.final_reason,
                        row.next_action,
                        row.updated_at,
                    ],
                )
                .unwrap();
            } else {
                conn.execute(
                    r#"
                    INSERT INTO agent_ledger(
                        handle,role,task,status,report_path,spawned_at,waited,closed,
                        write_scope,token_risk,next_action,updated_at
                    ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
                    "#,
                    params![
                        row.handle,
                        row.role,
                        row.task,
                        row.status,
                        row.report_path,
                        row.spawned_at,
                        row.waited,
                        row.closed,
                        row.write_scope,
                        row.token_risk,
                        row.next_action,
                        row.updated_at,
                    ],
                )
                .unwrap();
            }
            expected.push(row);
        }
        expected
    }

    fn snapshot_legacy(conn: &Connection) -> Vec<LegacyRow> {
        let mut statement = conn
            .prepare(
                r#"
                SELECT handle,role,task,status,report_path,spawned_at,waited,closed,
                       write_scope,token_risk,final_reason,next_action,updated_at
                FROM agent_ledger ORDER BY handle
                "#,
            )
            .unwrap();
        statement
            .query_map([], |row| {
                Ok(LegacyRow {
                    handle: row.get(0)?,
                    role: row.get(1)?,
                    task: row.get(2)?,
                    status: row.get(3)?,
                    report_path: row.get(4)?,
                    spawned_at: row.get(5)?,
                    waited: row.get(6)?,
                    closed: row.get(7)?,
                    write_scope: row.get(8)?,
                    token_risk: row.get(9)?,
                    final_reason: row.get(10)?,
                    next_action: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn assert_honest_legacy_import(conn: &Connection, expected: &[LegacyRow]) {
        assert_eq!(row_count(conn, "agent_sessions"), expected.len() as i64);
        assert_eq!(
            row_count(conn, "legacy_agent_ledger_import"),
            expected.len() as i64
        );
        for row in expected {
            let actual: HonestSessionTuple = conn
                .query_row(
                    r#"
                    SELECT run_id,handle,role,status,spawned_at,final_reason,
                           token_budget_mode,input_tokens,requested_model
                    FROM agent_sessions WHERE session_id=?1
                    "#,
                    [&row.handle],
                    |record| {
                        Ok((
                            record.get(0)?,
                            record.get(1)?,
                            record.get(2)?,
                            record.get(3)?,
                            record.get(4)?,
                            record.get(5)?,
                            record.get(6)?,
                            record.get(7)?,
                            record.get(8)?,
                        ))
                    },
                )
                .unwrap();
            assert_eq!(actual.0, None);
            assert_eq!(actual.1.as_deref(), Some(row.handle.as_str()));
            assert_eq!(actual.2, row.role);
            assert_eq!(actual.3, row.status);
            assert_eq!(
                actual.4,
                (!row.spawned_at.is_empty()).then(|| row.spawned_at.clone())
            );
            assert_eq!(
                actual.5,
                (!row.final_reason.is_empty()).then(|| row.final_reason.clone())
            );
            assert_eq!(actual.6, "unknown");
            assert_eq!(actual.7, None);
            assert_eq!(actual.8, None);
        }
        for table in [
            "runs",
            "work_packages",
            "assignments",
            "assignment_attempts",
            "session_leases",
            "routing_decisions",
            "control_plane_events",
            "run_event_counters",
            "projection_field_sources",
        ] {
            assert_eq!(row_count(conn, table), 0, "fabricated rows in {table}");
        }
    }

    #[test]
    fn fresh_database_has_complete_versioned_schema() {
        let temp = TempDb::new("fresh-schema");
        let conn = open_db(temp.path().to_str().unwrap()).unwrap();

        assert_eq!(pragma_i32(&conn, "user_version"), 1);
        assert_eq!(
            table_names(&conn),
            [
                "agent_ledger",
                "agent_sessions",
                "assignment_attempts",
                "assignments",
                "control_plane_events",
                "harness_meta",
                "legacy_agent_ledger_import",
                "projection_field_sources",
                "routing_decisions",
                "run_event_counters",
                "runs",
                "session_leases",
                "work_package_dependencies",
                "work_packages",
            ]
            .into_iter()
            .map(str::to_string)
            .collect()
        );
        assert!(named_indexes(&conn).contains("ux_control_plane_events_run_sequence"));
        assert!(named_indexes(&conn).contains("ux_session_leases_one_usable"));
        assert!(foreign_key_check(&conn).is_empty());
        assert_eq!(pragma_text(&conn, "journal_mode"), "wal");
        assert_eq!(row_count(&conn, "control_plane_events"), 0);

        let bad_child = conn.execute(
            "INSERT INTO work_packages(package_id,run_id,title,role_floor,model_floor,risk_floor,write_scope,review_policy,independence_policy,status) VALUES('p-bad','missing','bad','worker','standard','medium','{\"v\":1,\"paths\":[]}','{\"v\":1,\"kind\":\"deterministic\"}','{\"v\":1,\"kind\":\"different-session\"}','planned')",
            [],
        );
        assert!(bad_child.is_err());
        let bad_state = conn.execute(
            "INSERT INTO runs(run_id,goal,status,session_budget,token_budget_mode,report_path,ledger_path) VALUES('r-bad','goal','bogus','{\"v\":1,\"max_open\":2,\"max_total\":4,\"override_reason\":null}','unbounded','/tmp/report','/tmp/ledger')",
            [],
        );
        assert!(bad_state.is_err());
        let bad_boolean = conn.execute(
            "INSERT INTO agent_ledger(handle,role,status,waited,closed) VALUES('bad-bool','worker','planned',2,0)",
            [],
        );
        assert!(bad_boolean.is_err());

        conn.execute(
            "INSERT INTO runs(run_id,goal,status,session_budget,token_budget_mode,report_path,ledger_path) VALUES(?1,'goal','planned','{\"v\":1,\"max_open\":2,\"max_total\":4,\"override_reason\":null}','unbounded','/tmp/report','/tmp/ledger')",
            ["run-1"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO work_packages(package_id,run_id,title,role_floor,model_floor,risk_floor,write_scope,review_policy,independence_policy,status) VALUES('package-1','run-1','package','worker','standard','medium','{\"v\":1,\"paths\":[]}','{\"v\":1,\"kind\":\"deterministic\"}','{\"v\":1,\"kind\":\"different-session\"}','planned')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,attempt_count,status) VALUES('assignment-1','package-1','assignment',1,'implementation','worker','standard','medium','{\"v\":1,\"paths\":[]}',0,'planned')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('session-1','run-1','worker','unbounded','planned')",
            [],
        )
        .unwrap();

        let incomplete_cost = conn.execute(
            "INSERT INTO assignment_attempts(attempt_id,assignment_id,session_id,attempt_sequence,status,cost_amount) VALUES('attempt-bad','assignment-1','session-1',1,'planned',1)",
            [],
        );
        assert!(incomplete_cost.is_err());

        conn.execute(
            "INSERT INTO session_leases(lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,independence_boundary_id,status,reuse_count) VALUES('lease-active','session-1','package-1','worker','standard','medium','{\"v\":1,\"paths\":[]}','base','boundary','active',0)",
            [],
        )
        .unwrap();
        let second_usable = conn.execute(
            "INSERT INTO session_leases(lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,independence_boundary_id,status,reuse_count) VALUES('lease-idle','session-1','package-1','worker','standard','medium','{\"v\":1,\"paths\":[]}','base','boundary','idle',0)",
            [],
        );
        assert!(second_usable.is_err());
        for lease_id in ["lease-planned-1", "lease-planned-2"] {
            conn.execute(
                "INSERT INTO session_leases(lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,independence_boundary_id,status,reuse_count) VALUES(?1,'session-1','package-1','worker','standard','medium','{\"v\":1,\"paths\":[]}','base','boundary','planned',0)",
                params![lease_id],
            )
            .unwrap();
        }
    }

    #[test]
    fn legacy_without_final_reason_migrates_atomically() {
        let temp = TempDb::new("historical-legacy");
        let raw = Connection::open(temp.path()).unwrap();
        raw.execute_batch(HISTORICAL_LEDGER_DDL).unwrap();
        let expected = seed_legacy(&raw, false);
        drop(raw);

        let conn = open_db(temp.path().to_str().unwrap()).unwrap();
        assert_eq!(snapshot_legacy(&conn), {
            let mut rows = expected.clone();
            rows.sort_by(|left, right| left.handle.cmp(&right.handle));
            rows
        });
        assert_honest_legacy_import(&conn, &expected);
        assert_eq!(pragma_i32(&conn, "user_version"), 1);
    }

    #[test]
    fn legacy_current_migrates_without_losing_ledger_facts() {
        let temp = TempDb::new("current-legacy");
        let raw = Connection::open(temp.path()).unwrap();
        raw.execute_batch(CURRENT_LEDGER_DDL).unwrap();
        let mut expected = seed_legacy(&raw, true);
        expected.sort_by(|left, right| left.handle.cmp(&right.handle));
        drop(raw);

        let conn = open_db(temp.path().to_str().unwrap()).unwrap();
        assert_eq!(snapshot_legacy(&conn), expected);
        assert_honest_legacy_import(&conn, &expected);
    }

    #[test]
    fn current_database_reopen_is_idempotent() {
        let temp = TempDb::new("reopen");
        let first = open_db(temp.path().to_str().unwrap()).unwrap();
        let schema_before: Vec<(String, String)> = first
            .prepare("SELECT name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY name")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        drop(first);
        let second = open_db(temp.path().to_str().unwrap()).unwrap();
        let schema_after: Vec<(String, String)> = second
            .prepare("SELECT name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY name")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(schema_after, schema_before);
    }

    #[test]
    fn invalid_legacy_state_rolls_back_entire_migration() {
        let temp = TempDb::new("invalid-legacy-state");
        let raw = Connection::open(temp.path()).unwrap();
        raw.execute_batch(HISTORICAL_LEDGER_DDL).unwrap();
        raw.execute(
            "INSERT INTO agent_ledger(handle,role,status) VALUES('bad','worker','invented')",
            [],
        )
        .unwrap();
        drop(raw);
        assert!(open_db(temp.path().to_str().unwrap()).is_err());
        let unchanged = Connection::open(temp.path()).unwrap();
        assert_eq!(pragma_i32(&unchanged, "user_version"), 0);
        assert_eq!(
            table_names(&unchanged),
            ["agent_ledger", "harness_meta"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
        assert!(
            !column_manifest_for_test(&unchanged, "agent_ledger")
                .contains(&"final_reason".to_string())
        );
    }

    fn column_manifest_for_test(conn: &Connection, table: &str) -> Vec<String> {
        let mut statement = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        statement
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn partial_unversioned_target_layout_is_rejected_unchanged() {
        let temp = TempDb::new("partial-layout");
        let raw = Connection::open(temp.path()).unwrap();
        raw.execute("CREATE TABLE runs(run_id TEXT PRIMARY KEY)", [])
            .unwrap();
        let before: String = raw
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE name='runs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(raw);
        assert!(open_db(temp.path().to_str().unwrap()).is_err());
        let raw = Connection::open(temp.path()).unwrap();
        let after: String = raw
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE name='runs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(after, before);
        assert_eq!(
            table_names(&raw),
            ["runs"].into_iter().map(str::to_string).collect()
        );
    }

    #[test]
    fn legacy_missing_non_migratable_column_is_rejected() {
        let temp = TempDb::new("missing-column");
        let raw = Connection::open(temp.path()).unwrap();
        raw.execute_batch(&CURRENT_LEDGER_DDL.replace("token_risk TEXT NOT NULL DEFAULT '',", ""))
            .unwrap();
        drop(raw);
        assert!(open_db(temp.path().to_str().unwrap()).is_err());
        let raw = Connection::open(temp.path()).unwrap();
        assert_eq!(pragma_i32(&raw, "user_version"), 0);
        assert!(
            !column_manifest_for_test(&raw, "agent_ledger").contains(&"token_risk".to_string())
        );
    }

    #[test]
    fn future_schema_version_is_rejected_unchanged() {
        let temp = TempDb::new("future-version");
        drop(open_db(temp.path().to_str().unwrap()).unwrap());
        let raw = Connection::open(temp.path()).unwrap();
        raw.pragma_update(None, "user_version", 2).unwrap();
        let count_before = table_names(&raw);
        drop(raw);
        let error = open_db(temp.path().to_str().unwrap()).unwrap_err();
        assert!(error.contains("unsupported schema version: 2"), "{error}");
        let raw = Connection::open(temp.path()).unwrap();
        assert_eq!(pragma_i32(&raw, "user_version"), 2);
        assert_eq!(table_names(&raw), count_before);
    }

    #[test]
    fn corrupt_database_is_not_replaced() {
        let temp = TempDb::new("corrupt");
        let bytes = b"not a sqlite database\0with evidence";
        fs::write(temp.path(), bytes).unwrap();
        assert!(open_db(temp.path().to_str().unwrap()).is_err());
        assert_eq!(fs::read(temp.path()).unwrap(), bytes);
    }

    #[test]
    fn current_version_missing_required_index_is_rejected() {
        let temp = TempDb::new("missing-index");
        drop(open_db(temp.path().to_str().unwrap()).unwrap());
        let raw = Connection::open(temp.path()).unwrap();
        raw.execute("DROP INDEX ux_control_plane_events_run_sequence", [])
            .unwrap();
        drop(raw);
        assert!(open_db(temp.path().to_str().unwrap()).is_err());
        let raw = Connection::open(temp.path()).unwrap();
        assert_eq!(pragma_i32(&raw, "user_version"), 1);
        assert!(!named_indexes(&raw).contains("ux_control_plane_events_run_sequence"));
    }

    #[test]
    fn concurrent_openers_observe_one_committed_migration() {
        let temp = TempDb::new("concurrent-open");
        let path = temp.path().to_path_buf();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let path = path.clone();
            let barrier = barrier.clone();
            threads.push(std::thread::spawn(move || {
                barrier.wait();
                open_db(path.to_str().unwrap()).map(|_| ())
            }));
        }
        barrier.wait();
        for thread in threads {
            thread.join().unwrap().unwrap();
        }
        let conn = Connection::open(path).unwrap();
        assert_eq!(pragma_i32(&conn, "user_version"), 1);
        assert_eq!(table_names(&conn).len(), 14);
    }

    #[test]
    fn canonical_timestamp_rejects_noncanonical_values() {
        validate_canonical_timestamp("2026-07-12T09:08:07.006Z").unwrap();
        validate_canonical_timestamp("2024-02-29T23:59:59.999Z").unwrap();
        for value in [
            "2026-07-12T09:08:07Z",
            "2026-07-12T09:08:07.006+00:00",
            "2026-07-12T09:08:60.006Z",
            "2026-02-29T09:08:07.006Z",
            "2026-13-12T09:08:07.006Z",
            "2026-07-32T09:08:07.006Z",
            "2026-07-12T24:08:07.006Z",
            " 2026-07-12T09:08:07.006Z",
        ] {
            assert!(
                validate_canonical_timestamp(value).is_err(),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn canonical_json_rejects_duplicates_unknown_shape_and_whitespace() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(
            canonical_json(&conn, "{\"v\":1,\"paths\":[]}", JsonTopLevel::Object).unwrap(),
            "{\"v\":1,\"paths\":[]}"
        );
        assert_eq!(
            canonical_json(&conn, "[]", JsonTopLevel::Array).unwrap(),
            "[]"
        );
        for value in [
            "{\"v\":1,\"v\":1}",
            "{\"outer\":{\"x\":1,\"x\":2}}",
            "{ \"v\": 1 }",
            "not-json",
        ] {
            assert!(
                canonical_json(&conn, value, JsonTopLevel::Object).is_err(),
                "accepted {value}"
            );
        }
        assert!(canonical_json(&conn, "[]", JsonTopLevel::Object).is_err());
        assert!(canonical_json(&conn, "{}", JsonTopLevel::Array).is_err());
    }
}
