use crate::schema::{JsonTopLevel, canonical_json, sqlite_now, validate_canonical_timestamp};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

const PACKAGE: u8 = 1;
const ASSIGNMENT: u8 = 2;
const ATTEMPT: u8 = 4;
const SESSION: u8 = 8;
const LEASE: u8 = 16;
const ALL_REFERENCES: u8 = PACKAGE | ASSIGNMENT | ATTEMPT | SESSION | LEASE;

#[derive(Clone, Copy)]
struct EventRule {
    name: &'static str,
    keys: &'static [&'static str],
    required_references: u8,
    allowed_references: u8,
}

const fn rule(
    name: &'static str,
    keys: &'static [&'static str],
    required_references: u8,
    allowed_references: u8,
) -> EventRule {
    EventRule {
        name,
        keys,
        required_references,
        allowed_references,
    }
}

const EVENT_RULES: &[EventRule] = &[
    rule(
        "run_planned",
        &[
            "v",
            "goal",
            "psoc_revision",
            "session_budget",
            "token_budget_mode",
            "token_budget",
            "report_path",
            "ledger_path",
            "next_action",
        ],
        0,
        0,
    ),
    rule(
        "run_started",
        &["v", "psoc_revision", "started_at", "next_action"],
        0,
        0,
    ),
    rule("run_blocked", &["v", "blocker", "next_action"], 0, 0),
    rule("run_unblocked", &["v", "resolution", "next_action"], 0, 0),
    rule("run_completed", &["v", "completed_at", "ended_at"], 0, 0),
    rule(
        "run_cancelled",
        &["v", "reason", "ended_at", "next_action"],
        0,
        0,
    ),
    rule(
        "package_planned",
        &[
            "v",
            "title",
            "dependencies",
            "role_floor",
            "model_floor",
            "risk_floor",
            "write_scope",
            "review_policy",
            "independence_policy",
            "next_action",
        ],
        PACKAGE,
        PACKAGE,
    ),
    rule("package_ready", &["v", "next_action"], PACKAGE, PACKAGE),
    rule("package_active", &["v", "next_action"], PACKAGE, PACKAGE),
    rule(
        "package_blocked",
        &["v", "blocker", "resume_status", "next_action"],
        PACKAGE,
        PACKAGE,
    ),
    rule(
        "package_unblocked",
        &["v", "resolution", "resume_status", "next_action"],
        PACKAGE,
        PACKAGE,
    ),
    rule(
        "package_review_started",
        &["v", "review_assignment_id", "next_action"],
        PACKAGE,
        PACKAGE | ASSIGNMENT,
    ),
    rule(
        "package_review_completed",
        &["v", "verdict", "review_evidence", "next_action"],
        PACKAGE,
        PACKAGE | ASSIGNMENT,
    ),
    rule("package_completed", &["v", "ended_at"], PACKAGE, PACKAGE),
    rule(
        "package_cancelled",
        &["v", "reason", "ended_at", "next_action"],
        PACKAGE,
        PACKAGE,
    ),
    rule(
        "assignment_planned",
        &[
            "v",
            "title",
            "sequence",
            "assignment_kind",
            "required_role",
            "model_floor",
            "risk_class",
            "write_scope",
            "base_revision",
            "independence_boundary_id",
            "next_action",
        ],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT,
    ),
    rule(
        "assignment_queued",
        &["v", "next_action"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT,
    ),
    rule(
        "assignment_started",
        &["v", "attempt_id", "started_at", "current_step"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        PACKAGE | ASSIGNMENT | ATTEMPT | SESSION | LEASE,
    ),
    rule(
        "assignment_step_changed",
        &["v", "current_step"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
    ),
    rule(
        "assignment_blocked",
        &["v", "blocker", "next_action"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
    ),
    rule(
        "assignment_unblocked",
        &["v", "resolution", "next_action"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
    ),
    rule(
        "assignment_reported",
        &["v", "report_path", "reported_at", "next_action"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT | SESSION,
    ),
    rule(
        "assignment_validated",
        &["v", "test_evidence", "validated_at", "next_action"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
    ),
    rule(
        "assignment_accepted",
        &["v", "review_evidence", "accepted_at", "ended_at"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
    ),
    rule(
        "assignment_requeued",
        &["v", "reason", "next_action"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
    ),
    rule(
        "assignment_failed",
        &[
            "v",
            "policy_exhausted",
            "final_reason",
            "ended_at",
            "next_action",
        ],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
    ),
    rule(
        "assignment_cancelled",
        &["v", "final_reason", "ended_at", "next_action"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
    ),
    rule(
        "attempt_planned",
        &["v", "attempt_sequence", "next_action"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "attempt_started",
        &["v", "started_at", "next_action"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "attempt_reported",
        &["v", "reported_at", "next_action"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "attempt_validated",
        &["v", "validated_at", "next_action"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "attempt_accepted",
        &["v", "accepted_at", "ended_at"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "attempt_failed",
        &["v", "outcome_reason", "ended_at", "next_action"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "attempt_cancelled",
        &["v", "outcome_reason", "ended_at", "next_action"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "dispatch_main_selected",
        &["v", "reason", "authorization_ref"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
    ),
    rule(
        "dispatch_reuse_selected",
        &["v", "session_id", "lease_id", "reason", "authorization_ref"],
        PACKAGE | ASSIGNMENT | ATTEMPT | SESSION | LEASE,
        ALL_REFERENCES,
    ),
    rule(
        "dispatch_batch_selected",
        &["v", "reason", "authorization_ref"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
    ),
    rule(
        "dispatch_spawn_selected",
        &["v", "session_id", "reason", "authorization_ref"],
        PACKAGE | ASSIGNMENT | ATTEMPT | SESSION,
        PACKAGE | ASSIGNMENT | ATTEMPT | SESSION,
    ),
    rule(
        "route_requested",
        &[
            "v",
            "route_id",
            "required_profile",
            "requested_model",
            "requested_reasoning",
            "escalated_from_route_id",
            "decided_at",
            "next_action",
        ],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "route_applied",
        &[
            "v",
            "routing_status",
            "actual_model",
            "actual_reasoning",
            "eligibility_evidence",
            "decided_at",
        ],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "route_degraded",
        &[
            "v",
            "actual_model",
            "actual_reasoning",
            "reason",
            "eligibility_status",
            "eligibility_evidence",
            "next_action",
            "decided_at",
        ],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "route_rejected",
        &[
            "v",
            "reason",
            "eligibility_evidence",
            "next_action",
            "decided_at",
        ],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
    ),
    rule(
        "session_planned",
        &[
            "v",
            "authorization_ref",
            "budget_reason",
            "run_max_open",
            "run_max_total",
            "session_token_budget",
            "nested_delegation",
            "requested_host",
            "requested_profile",
            "requested_model",
            "requested_reasoning",
            "parent_session_id",
        ],
        PACKAGE | ASSIGNMENT | ATTEMPT | SESSION,
        PACKAGE | ASSIGNMENT | ATTEMPT | SESSION,
    ),
    rule(
        "session_spawned",
        &["v", "handle", "host_id", "spawned_at", "next_action"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_running",
        &[
            "v",
            "started_or_resumed_at",
            "lease_id",
            "attempt_id",
            "prior_report_ref",
            "gate_evidence",
            "next_action",
        ],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_heartbeat",
        &["v", "last_activity_at"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_blocked",
        &["v", "blocker", "next_action"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_unblocked",
        &["v", "resolution", "next_action"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_reported",
        &[
            "v",
            "last_reported_at",
            "assignment_id",
            "attempt_id",
            "next_action",
        ],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_waited",
        &[
            "v",
            "last_waited_at",
            "consumed_report_ref",
            "terminal_observation",
        ],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_failed",
        &["v", "final_reason", "ended_at", "next_action"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_abandoned",
        &["v", "final_reason", "ended_at", "next_action"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_externally_unknown",
        &["v", "final_reason", "ended_at", "next_action"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_interrupted",
        &["v", "interruption_reason", "interrupted_at", "next_action"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_close_requested",
        &["v", "close_requested_at", "next_action"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_closed",
        &["v", "outcome", "close_disposition", "closed_at", "ended_at"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "session_superseded",
        &[
            "v",
            "superseded_by_session_id",
            "superseded_at",
            "reason",
            "next_action",
        ],
        SESSION,
        ALL_REFERENCES,
    ),
    rule(
        "lease_planned",
        &[
            "v",
            "role",
            "model_profile",
            "risk_class",
            "write_scope",
            "base_revision",
            "independence_boundary_id",
            "replaces_session_id",
            "expiry_predicate",
            "expires_at",
            "next_action",
        ],
        PACKAGE | SESSION | LEASE,
        ALL_REFERENCES,
    ),
    rule(
        "lease_issued",
        &["v", "issued_at", "next_action"],
        PACKAGE | SESSION | LEASE,
        ALL_REFERENCES,
    ),
    rule(
        "lease_reused",
        &[
            "v",
            "attempt_id",
            "compatibility_evidence",
            "last_used_at",
            "next_action",
        ],
        PACKAGE | ATTEMPT | SESSION | LEASE,
        ALL_REFERENCES,
    ),
    rule(
        "lease_idle",
        &["v", "last_used_at", "next_action"],
        PACKAGE | SESSION | LEASE,
        ALL_REFERENCES,
    ),
    rule(
        "lease_expired",
        &["v", "expiry_reason", "ended_at", "next_action"],
        PACKAGE | SESSION | LEASE,
        ALL_REFERENCES,
    ),
    rule(
        "lease_revoked",
        &["v", "expiry_reason", "ended_at", "next_action"],
        PACKAGE | SESSION | LEASE,
        ALL_REFERENCES,
    ),
    rule(
        "lease_closed",
        &["v", "expiry_reason", "ended_at"],
        PACKAGE | SESSION | LEASE,
        ALL_REFERENCES,
    ),
    rule(
        "usage_observed",
        &[
            "v",
            "scope",
            "subject_id",
            "observation_kind",
            "window_start",
            "window_end",
            "input_tokens",
            "output_tokens",
            "reasoning_tokens",
            "cache_read_tokens",
            "cache_write_tokens",
            "credits",
            "provider_cost",
            "telemetry_quality",
            "supersedes_event_id",
        ],
        0,
        ALL_REFERENCES,
    ),
    rule(
        "quality_gate_passed",
        &[
            "v",
            "subject_kind",
            "subject_id",
            "policy",
            "evidence",
            "observed_at",
        ],
        0,
        ALL_REFERENCES,
    ),
    rule(
        "quality_gate_failed",
        &[
            "v",
            "subject_kind",
            "subject_id",
            "policy",
            "findings_ref",
            "next_action",
            "observed_at",
        ],
        0,
        ALL_REFERENCES,
    ),
];

pub(crate) fn event_names() -> Vec<&'static str> {
    EVENT_RULES.iter().map(|rule| rule.name).collect()
}

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

fn event_rule(name: &str) -> Result<&'static EventRule, String> {
    EVENT_RULES
        .iter()
        .find(|rule| rule.name == name)
        .ok_or_else(|| format!("unknown event type: {name}"))
}

fn reference_mask(event: &EventInput) -> u8 {
    u8::from(event.package_id.is_some()) * PACKAGE
        | u8::from(event.assignment_id.is_some()) * ASSIGNMENT
        | u8::from(event.attempt_id.is_some()) * ATTEMPT
        | u8::from(event.session_id.is_some()) * SESSION
        | u8::from(event.lease_id.is_some()) * LEASE
}

fn validate_common(conn: &Connection, event: &EventInput) -> Result<&'static EventRule, String> {
    for (name, value) in [
        ("event_id", event.event_id.as_str()),
        ("run_id", event.run_id.as_str()),
        ("source_id", event.source_id.as_str()),
        ("idempotency_key", event.idempotency_key.as_str()),
    ] {
        if value.is_empty() {
            return Err(format!("{name} must be nonempty"));
        }
    }
    for (name, value) in [
        ("package_id", event.package_id.as_deref()),
        ("assignment_id", event.assignment_id.as_deref()),
        ("attempt_id", event.attempt_id.as_deref()),
        ("session_id", event.session_id.as_deref()),
        ("lease_id", event.lease_id.as_deref()),
    ] {
        if matches!(value, Some("")) {
            return Err(format!("{name} must be nonempty when present"));
        }
    }
    let rule = event_rule(&event.event_type)?;
    if ![
        "host-runtime",
        "harness-operation",
        "controller-observation",
        "agent-report",
        "inference",
    ]
    .contains(&event.source_kind.as_str())
    {
        return Err(format!("unknown source kind: {}", event.source_kind));
    }
    if let Some(confidence) = event.confidence
        && !(0..=10_000).contains(&confidence)
    {
        return Err(format!("confidence out of range: {confidence}"));
    }
    validate_canonical_timestamp(&event.occurred_at).map_err(|error| error.to_string())?;
    canonical_json(conn, &event.payload_json, JsonTopLevel::Object)
        .map_err(|error| error.to_string())?;
    validate_payload(conn, rule, &event.payload_json)?;

    let actual = reference_mask(event);
    let missing = rule.required_references & !actual;
    if missing != 0 {
        let name = if missing & PACKAGE != 0 {
            "package_id"
        } else if missing & ASSIGNMENT != 0 {
            "assignment_id"
        } else if missing & ATTEMPT != 0 {
            "attempt_id"
        } else if missing & SESSION != 0 {
            "session_id"
        } else {
            "lease_id"
        };
        return Err(format!("{} requires {name}", event.event_type));
    }
    if actual & !rule.allowed_references != 0 {
        return Err(format!(
            "{} contains an inapplicable entity reference",
            event.event_type
        ));
    }
    Ok(rule)
}

fn ordered_json_keys(conn: &Connection, json: &str) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare("SELECT key FROM json_each(?1) ORDER BY id")
        .map_err(|error| error.to_string())?;
    statement
        .query_map([json], |row| row.get(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|error| error.to_string())
}

fn validate_payload(conn: &Connection, rule: &EventRule, payload: &str) -> Result<(), String> {
    let keys = ordered_json_keys(conn, payload)?;
    let expected: Vec<String> = rule.keys.iter().map(|key| (*key).to_string()).collect();
    if keys != expected {
        return Err(format!(
            "{} payload keys must be {:?} in order, got {:?}",
            rule.name, rule.keys, keys
        ));
    }
    let version: Option<i64> = conn
        .query_row("SELECT json_extract(?1,'$.v')", [payload], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if version != Some(1) {
        return Err(format!("{} payload requires integer v=1", rule.name));
    }

    for key in rule.keys.iter().copied().filter(|key| *key != "v") {
        let path = format!("$.{key}");
        let value_type: Option<String> = conn
            .query_row("SELECT json_type(?1,?2)", params![payload, path], |row| {
                row.get(0)
            })
            .map_err(|error| error.to_string())?;
        if value_type.as_deref() == Some("text") {
            let value: String = conn
                .query_row(
                    "SELECT json_extract(?1,?2)",
                    params![payload, path],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if value.is_empty() {
                return Err(format!(
                    "{rule_name}.{key} must be nonempty",
                    rule_name = rule.name
                ));
            }
            if key.ends_with("_at")
                || matches!(key, "started_or_resumed_at" | "window_start" | "window_end")
            {
                validate_canonical_timestamp(&value).map_err(|error| error.to_string())?;
            }
        }
    }
    if rule.name == "run_planned" {
        validate_run_planned_payload(conn, payload)?;
    }
    Ok(())
}

fn json_text(conn: &Connection, payload: &str, key: &str) -> Result<Option<String>, String> {
    let path = format!("$.{key}");
    conn.query_row(
        "SELECT CASE WHEN json_type(?1,?2)='text' THEN json_extract(?1,?2) END",
        params![payload, path],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn required_json_text(conn: &Connection, payload: &str, key: &str) -> Result<String, String> {
    json_text(conn, payload, key)?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("run_planned.{key} must be a nonempty string"))
}

fn json_i64(conn: &Connection, payload: &str, key: &str) -> Result<Option<i64>, String> {
    let path = format!("$.{key}");
    conn.query_row(
        "SELECT CASE WHEN json_type(?1,?2)='integer' THEN json_extract(?1,?2) END",
        params![payload, path],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn validate_run_planned_payload(conn: &Connection, payload: &str) -> Result<(), String> {
    required_json_text(conn, payload, "goal")?;
    if json_text(conn, payload, "psoc_revision")?.is_none() {
        return Err("run_planned.psoc_revision must be a nonempty string".into());
    }
    required_json_text(conn, payload, "report_path")?;
    required_json_text(conn, payload, "ledger_path")?;
    required_json_text(conn, payload, "next_action")?;
    let mode = required_json_text(conn, payload, "token_budget_mode")?;
    let budget = json_i64(conn, payload, "token_budget")?;
    match (mode.as_str(), budget) {
        ("bounded", Some(value)) if value >= 0 => {}
        ("unbounded", None) => {}
        _ => {
            return Err(
                "run_planned token budget must be bounded+nonnegative or unbounded+null".into(),
            );
        }
    }

    let budget_json: String = conn
        .query_row(
            "SELECT json_extract(?1,'$.session_budget')",
            [payload],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let budget_keys = ordered_json_keys(conn, &budget_json)?;
    if budget_keys != ["v", "max_open", "max_total", "override_reason"] {
        return Err("run_planned.session_budget has an invalid envelope".into());
    }
    let (version, max_open, max_total, reason): (i64, i64, i64, Option<String>) = conn
        .query_row(
            "SELECT json_extract(?1,'$.v'),json_extract(?1,'$.max_open'),json_extract(?1,'$.max_total'),json_extract(?1,'$.override_reason')",
            [&budget_json],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|error| error.to_string())?;
    if version != 1 || max_open < 0 || max_total < max_open {
        return Err("run_planned.session_budget values are invalid".into());
    }
    if (max_open > 2 || max_total > 4) && reason.as_deref().is_none_or(str::is_empty) {
        return Err("raised session budget requires override_reason".into());
    }
    Ok(())
}

fn validate_identity_chain(
    transaction: &Transaction<'_>,
    event: &EventInput,
) -> Result<(), String> {
    if event.event_type != "run_planned" {
        let run_exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM runs WHERE run_id=?1)",
                [&event.run_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if !run_exists {
            return Err(format!("unknown run: {}", event.run_id));
        }
    }
    if let Some(package_id) = &event.package_id {
        let owner: Option<String> = transaction
            .query_row(
                "SELECT run_id FROM work_packages WHERE package_id=?1",
                [package_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if event.event_type != "package_planned" {
            let owner = owner.ok_or_else(|| format!("unknown package: {package_id}"))?;
            if owner != event.run_id {
                return Err(format!(
                    "package {package_id} belongs to run {owner}, not {}",
                    event.run_id
                ));
            }
        } else if owner.is_some() {
            return Err(format!("package already exists: {package_id}"));
        }
    }
    if let Some(assignment_id) = &event.assignment_id {
        let owner: Option<String> = transaction
            .query_row(
                "SELECT package_id FROM assignments WHERE assignment_id=?1",
                [assignment_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if event.event_type != "assignment_planned" {
            let owner = owner.ok_or_else(|| format!("unknown assignment: {assignment_id}"))?;
            if event.package_id.as_deref() != Some(owner.as_str()) {
                return Err(format!(
                    "assignment {assignment_id} belongs to package {owner}"
                ));
            }
        } else if owner.is_some() {
            return Err(format!("assignment already exists: {assignment_id}"));
        }
    }
    if let Some(attempt_id) = &event.attempt_id {
        let owner: Option<(String, Option<String>, Option<String>)> = transaction
            .query_row(
                "SELECT assignment_id,session_id,lease_id FROM assignment_attempts WHERE attempt_id=?1",
                [attempt_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if event.event_type != "attempt_planned" {
            let (assignment, session, lease) =
                owner.ok_or_else(|| format!("unknown attempt: {attempt_id}"))?;
            if event.assignment_id.as_deref() != Some(assignment.as_str()) {
                return Err(format!(
                    "attempt {attempt_id} belongs to assignment {assignment}"
                ));
            }
            if let Some(event_session) = &event.session_id
                && session.as_deref() != Some(event_session.as_str())
            {
                return Err(format!(
                    "attempt {attempt_id} does not belong to session {event_session}"
                ));
            }
            if let Some(event_lease) = &event.lease_id
                && lease.as_deref() != Some(event_lease.as_str())
            {
                return Err(format!(
                    "attempt {attempt_id} does not belong to lease {event_lease}"
                ));
            }
        } else if owner.is_some() {
            return Err(format!("attempt already exists: {attempt_id}"));
        }
    }
    if let Some(session_id) = &event.session_id {
        let owner: Option<Option<String>> = transaction
            .query_row(
                "SELECT run_id FROM agent_sessions WHERE session_id=?1",
                [session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if event.event_type != "session_planned" {
            let owner = owner.ok_or_else(|| format!("unknown session: {session_id}"))?;
            match owner {
                Some(owner) if owner == event.run_id => {}
                Some(owner) => return Err(format!("session {session_id} belongs to run {owner}")),
                None => {
                    return Err(format!(
                        "session {session_id} is an unattached legacy import"
                    ));
                }
            }
        } else if owner.is_some() {
            return Err(format!("session already exists: {session_id}"));
        }
    }
    if let Some(lease_id) = &event.lease_id {
        let owner: Option<(String, String)> = transaction
            .query_row(
                "SELECT session_id,package_id FROM session_leases WHERE lease_id=?1",
                [lease_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if event.event_type != "lease_planned" {
            let (session, package) = owner.ok_or_else(|| format!("unknown lease: {lease_id}"))?;
            if event.session_id.as_deref() != Some(session.as_str())
                || event.package_id.as_deref() != Some(package.as_str())
            {
                return Err(format!(
                    "lease {lease_id} ownership does not match event chain"
                ));
            }
        } else if owner.is_some() {
            return Err(format!("lease already exists: {lease_id}"));
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct StoredSemanticEvent {
    event_id: String,
    sequence: i64,
    package_id: Option<String>,
    assignment_id: Option<String>,
    attempt_id: Option<String>,
    session_id: Option<String>,
    lease_id: Option<String>,
    event_type: String,
    confidence: Option<i64>,
    payload_json: String,
    occurred_at: String,
}

fn lookup_idempotency(
    transaction: &Transaction<'_>,
    event: &EventInput,
) -> Result<Option<StoredSemanticEvent>, String> {
    transaction
        .query_row(
            r#"
            SELECT event_id,sequence,package_id,assignment_id,attempt_id,session_id,
                   lease_id,event_type,confidence,payload_json,occurred_at
            FROM control_plane_events
            WHERE run_id=?1 AND source_kind=?2 AND source_id=?3 AND idempotency_key=?4
            "#,
            params![
                event.run_id,
                event.source_kind,
                event.source_id,
                event.idempotency_key
            ],
            |row| {
                Ok(StoredSemanticEvent {
                    event_id: row.get(0)?,
                    sequence: row.get(1)?,
                    package_id: row.get(2)?,
                    assignment_id: row.get(3)?,
                    attempt_id: row.get(4)?,
                    session_id: row.get(5)?,
                    lease_id: row.get(6)?,
                    event_type: row.get(7)?,
                    confidence: row.get(8)?,
                    payload_json: row.get(9)?,
                    occurred_at: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn semantic_matches(stored: &StoredSemanticEvent, event: &EventInput) -> bool {
    stored.package_id == event.package_id
        && stored.assignment_id == event.assignment_id
        && stored.attempt_id == event.attempt_id
        && stored.session_id == event.session_id
        && stored.lease_id == event.lease_id
        && stored.event_type == event.event_type
        && stored.confidence == event.confidence
        && stored.payload_json == event.payload_json
        && stored.occurred_at == event.occurred_at
}

pub(crate) fn append_event(
    conn: &mut Connection,
    event: &EventInput,
) -> Result<AppendResult, String> {
    let rule = validate_common(conn, event)?;
    let transaction = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    if let Some(stored) = lookup_idempotency(&transaction, event)? {
        if semantic_matches(&stored, event) {
            return Ok(AppendResult {
                event_id: stored.event_id,
                sequence: stored.sequence,
                disposition: AppendDisposition::AlreadyIngested,
            });
        }
        return Err(format!(
            "idempotency_conflict for run/source/key: {}/{}/{}",
            event.run_id, event.source_id, event.idempotency_key
        ));
    }
    let duplicate_event_id: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM control_plane_events WHERE event_id=?1)",
            [&event.event_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if duplicate_event_id {
        return Err(format!("duplicate event_id: {}", event.event_id));
    }
    validate_identity_chain(&transaction, event)?;

    let sequence = if rule.name == "run_planned" {
        1
    } else {
        let next: i64 = transaction
            .query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id=?1",
                [&event.run_id],
                |row| row.get(0),
            )
            .map_err(|error| format!("missing run event counter: {error}"))?;
        transaction
            .execute(
                "UPDATE run_event_counters SET next_sequence=?2 WHERE run_id=?1",
                params![event.run_id, next + 1],
            )
            .map_err(|error| error.to_string())?;
        next
    };
    let ingested_at = sqlite_now(&transaction).map_err(|error| error.to_string())?;
    transaction
        .execute(
            r#"
            INSERT INTO control_plane_events(
                event_id,run_id,package_id,assignment_id,attempt_id,session_id,lease_id,
                sequence,event_type,source_kind,source_id,confidence,payload_json,
                occurred_at,ingested_at,idempotency_key
            ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
            "#,
            params![
                event.event_id,
                event.run_id,
                event.package_id,
                event.assignment_id,
                event.attempt_id,
                event.session_id,
                event.lease_id,
                sequence,
                event.event_type,
                event.source_kind,
                event.source_id,
                event.confidence,
                event.payload_json,
                event.occurred_at,
                ingested_at,
                event.idempotency_key,
            ],
        )
        .map_err(|error| error.to_string())?;

    if rule.name == "run_planned" {
        reduce_run_planned(&transaction, event)?;
    } else {
        return Err(format!("event_reducer_unavailable: {}", rule.name));
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(AppendResult {
        event_id: event.event_id.clone(),
        sequence,
        disposition: AppendDisposition::Appended,
    })
}

fn reduce_run_planned(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    let exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM runs WHERE run_id=?1)",
            [&event.run_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if exists {
        return Err(format!("run already exists: {}", event.run_id));
    }
    let payload = &event.payload_json;
    let goal = required_json_text(transaction, payload, "goal")?;
    let psoc_revision = json_text(transaction, payload, "psoc_revision")?;
    let session_budget: String = transaction
        .query_row(
            "SELECT json_extract(?1,'$.session_budget')",
            [payload],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let token_budget_mode = required_json_text(transaction, payload, "token_budget_mode")?;
    let token_budget = json_i64(transaction, payload, "token_budget")?;
    let report_path = required_json_text(transaction, payload, "report_path")?;
    let ledger_path = required_json_text(transaction, payload, "ledger_path")?;
    let next_action = required_json_text(transaction, payload, "next_action")?;
    transaction
        .execute(
            r#"
            INSERT INTO runs(
                run_id,goal,psoc_revision,status,session_budget,token_budget,
                token_budget_mode,report_path,ledger_path,next_action
            ) VALUES(?1,?2,?3,'planned',?4,?5,?6,?7,?8,?9)
            "#,
            params![
                event.run_id,
                goal,
                psoc_revision,
                session_budget,
                token_budget,
                token_budget_mode,
                report_path,
                ledger_path,
                next_action,
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO run_event_counters(run_id,next_sequence) VALUES(?1,2)",
            [&event.run_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AppendDisposition, EventInput, append_event, event_names};
    use crate::schema::initialize_connection;
    use rusqlite::Connection;
    use std::collections::BTreeSet;

    const EXPECTED_EVENTS: &[&str] = &[
        "run_planned",
        "run_started",
        "run_blocked",
        "run_unblocked",
        "run_completed",
        "run_cancelled",
        "package_planned",
        "package_ready",
        "package_active",
        "package_blocked",
        "package_unblocked",
        "package_review_started",
        "package_review_completed",
        "package_completed",
        "package_cancelled",
        "assignment_planned",
        "assignment_queued",
        "assignment_started",
        "assignment_step_changed",
        "assignment_blocked",
        "assignment_unblocked",
        "assignment_reported",
        "assignment_validated",
        "assignment_accepted",
        "assignment_requeued",
        "assignment_failed",
        "assignment_cancelled",
        "attempt_planned",
        "attempt_started",
        "attempt_reported",
        "attempt_validated",
        "attempt_accepted",
        "attempt_failed",
        "attempt_cancelled",
        "dispatch_main_selected",
        "dispatch_reuse_selected",
        "dispatch_batch_selected",
        "dispatch_spawn_selected",
        "route_requested",
        "route_applied",
        "route_degraded",
        "route_rejected",
        "session_planned",
        "session_spawned",
        "session_running",
        "session_heartbeat",
        "session_blocked",
        "session_unblocked",
        "session_reported",
        "session_waited",
        "session_failed",
        "session_abandoned",
        "session_externally_unknown",
        "session_interrupted",
        "session_close_requested",
        "session_closed",
        "session_superseded",
        "lease_planned",
        "lease_issued",
        "lease_reused",
        "lease_idle",
        "lease_expired",
        "lease_revoked",
        "lease_closed",
        "usage_observed",
        "quality_gate_passed",
        "quality_gate_failed",
    ];

    fn connection() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize_connection(&mut conn, false).unwrap();
        conn
    }

    fn run_payload(goal: &str) -> String {
        format!(
            "{{\"v\":1,\"goal\":\"{goal}\",\"psoc_revision\":\"psoc-1\",\"session_budget\":{{\"v\":1,\"max_open\":2,\"max_total\":4,\"override_reason\":null}},\"token_budget_mode\":\"unbounded\",\"token_budget\":null,\"report_path\":\"/tmp/report.md\",\"ledger_path\":\"/tmp/ledger.db\",\"next_action\":\"start\"}}"
        )
    }

    fn run_event(event_id: &str, run_id: &str, key: &str) -> EventInput {
        EventInput {
            event_id: event_id.to_string(),
            run_id: run_id.to_string(),
            package_id: None,
            assignment_id: None,
            attempt_id: None,
            session_id: None,
            lease_id: None,
            event_type: "run_planned".to_string(),
            source_kind: "controller-observation".to_string(),
            source_id: "controller-1".to_string(),
            confidence: Some(10_000),
            payload_json: run_payload("goal"),
            occurred_at: "2026-07-12T09:08:07.006Z".to_string(),
            idempotency_key: key.to_string(),
        }
    }

    fn append_run(conn: &mut Connection, event_id: &str, run_id: &str, key: &str) {
        append_event(conn, &run_event(event_id, run_id, key)).unwrap();
    }

    #[test]
    fn event_registry_matches_approved_initial_set() {
        assert_eq!(event_names(), EXPECTED_EVENTS);
    }

    #[test]
    fn event_registry_has_unique_names_and_assignment_planned() {
        let names = event_names();
        let unique: BTreeSet<_> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len());
        assert!(unique.contains("assignment_planned"));
        assert_eq!(names.len(), 67);
    }

    #[test]
    fn event_rejects_unknown_type_source_confidence_or_identity() {
        let cases: Vec<(&str, Box<dyn Fn(&mut EventInput)>)> = vec![
            (
                "unknown event type",
                Box::new(|event| event.event_type = "invented".into()),
            ),
            (
                "unknown source kind",
                Box::new(|event| event.source_kind = "rumor".into()),
            ),
            ("confidence", Box::new(|event| event.confidence = Some(-1))),
            (
                "confidence",
                Box::new(|event| event.confidence = Some(10_001)),
            ),
            ("event_id", Box::new(|event| event.event_id.clear())),
            ("run_id", Box::new(|event| event.run_id.clear())),
            ("source_id", Box::new(|event| event.source_id.clear())),
            (
                "idempotency_key",
                Box::new(|event| event.idempotency_key.clear()),
            ),
        ];
        for (index, (expected, mutate)) in cases.into_iter().enumerate() {
            let mut conn = connection();
            let mut event = run_event(&format!("event-{index}"), &format!("run-{index}"), "key");
            mutate(&mut event);
            let error = append_event(&mut conn, &event).unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, got {error:?}"
            );
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM control_plane_events", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
                0
            );
        }
        let mut conn = connection();
        let mut event = run_event("event-ok", "run-ok", "key");
        event.confidence = None;
        append_event(&mut conn, &event).unwrap();
    }

    #[test]
    fn event_rejects_noncanonical_payload_or_timestamp() {
        let mut conn = connection();
        let cases = vec![
            (
                "{ \"v\": 1 }".to_string(),
                "2026-07-12T09:08:07.006Z".to_string(),
            ),
            (
                "{\"v\":1,\"v\":1}".to_string(),
                "2026-07-12T09:08:07.006Z".to_string(),
            ),
            ("[]".to_string(), "2026-07-12T09:08:07.006Z".to_string()),
            (run_payload("goal"), "2026-07-12T09:08:07Z".to_string()),
        ];
        for (payload, timestamp) in cases {
            let mut event = run_event("event-bad", "run-bad", "key");
            event.payload_json = payload;
            event.occurred_at = timestamp;
            assert!(append_event(&mut conn, &event).is_err());
        }
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM control_plane_events", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
    }

    #[test]
    fn event_identity_chain_rejects_cross_run_and_cross_owner_ids() {
        let mut conn = connection();
        append_run(&mut conn, "event-run-a", "run-a", "run-a");
        append_run(&mut conn, "event-run-b", "run-b", "run-b");
        conn.execute(
            "INSERT INTO work_packages(package_id,run_id,title,role_floor,model_floor,risk_floor,write_scope,review_policy,independence_policy,status) VALUES('package-a','run-a','package','worker','standard','medium','{\"v\":1,\"paths\":[]}','deterministic','different-session','planned')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,attempt_count,status) VALUES('assignment-a','package-a','assignment',1,'implementation','worker','standard','medium','{\"v\":1,\"paths\":[]}',0,'planned')",
            [],
        )
        .unwrap();

        let mut cross_run = run_event("event-cross-run", "run-b", "cross-run");
        cross_run.event_type = "package_ready".into();
        cross_run.package_id = Some("package-a".into());
        cross_run.payload_json = "{\"v\":1,\"next_action\":\"activate\"}".into();
        let error = append_event(&mut conn, &cross_run).unwrap_err();
        assert!(
            error.contains("package package-a belongs to run run-a"),
            "{error}"
        );

        let mut cross_owner = cross_run;
        cross_owner.event_id = "event-cross-owner".into();
        cross_owner.run_id = "run-a".into();
        cross_owner.event_type = "assignment_queued".into();
        cross_owner.assignment_id = Some("assignment-a".into());
        cross_owner.package_id = None;
        cross_owner.idempotency_key = "cross-owner".into();
        cross_owner.payload_json = "{\"v\":1,\"next_action\":\"run\"}".into();
        let error = append_event(&mut conn, &cross_owner).unwrap_err();
        assert!(error.contains("requires package_id"), "{error}");
    }

    #[test]
    fn first_append_allocates_sequence_one_and_projects_run() {
        let mut conn = connection();
        let result = append_event(&mut conn, &run_event("event-1", "run-1", "key-1")).unwrap();
        assert_eq!(result.event_id, "event-1");
        assert_eq!(result.sequence, 1);
        assert_eq!(result.disposition, AppendDisposition::Appended);
        let run: (String, String, String, String, Option<i64>, String) = conn
            .query_row(
                "SELECT goal,psoc_revision,status,token_budget_mode,token_budget,next_action FROM runs WHERE run_id='run-1'",
                [],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?)),
            )
            .unwrap();
        assert_eq!(
            run,
            (
                "goal".into(),
                "psoc-1".into(),
                "planned".into(),
                "unbounded".into(),
                None,
                "start".into()
            )
        );
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
    }

    #[test]
    fn identical_retry_returns_original_without_counter_advance() {
        let mut conn = connection();
        let original = run_event("event-original", "run-1", "key-1");
        append_event(&mut conn, &original).unwrap();
        let mut retry = original.clone();
        retry.event_id = "event-offered-on-retry".into();
        let result = append_event(&mut conn, &retry).unwrap();
        assert_eq!(result.event_id, "event-original");
        assert_eq!(result.sequence, 1);
        assert_eq!(result.disposition, AppendDisposition::AlreadyIngested);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM control_plane_events", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
    }

    #[test]
    fn same_key_different_content_is_idempotency_conflict() {
        let mut conn = connection();
        append_run(&mut conn, "event-original", "run-1", "key-1");
        let mut conflict = run_event("event-conflict", "run-1", "key-1");
        conflict.payload_json = run_payload("different");
        let error = append_event(&mut conn, &conflict).unwrap_err();
        assert!(error.contains("idempotency_conflict"), "{error}");
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM control_plane_events", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row("SELECT goal FROM runs WHERE run_id='run-1'", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap(),
            "goal"
        );
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
    }

    #[test]
    fn duplicate_event_id_in_another_scope_is_conflict() {
        let mut conn = connection();
        append_run(&mut conn, "event-1", "run-1", "key-1");
        let conflict = run_event("event-1", "run-2", "key-2");
        let error = append_event(&mut conn, &conflict).unwrap_err();
        assert!(error.contains("duplicate event_id"), "{error}");
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM runs", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn failed_validation_does_not_advance_counter() {
        let mut conn = connection();
        append_run(&mut conn, "event-1", "run-1", "key-1");
        let mut invalid = run_event("event-2", "run-1", "key-2");
        invalid.event_type = "run_started".into();
        invalid.payload_json = "{\"v\":1,\"psoc_revision\":\"psoc-2\",\"started_at\":\"bad\",\"next_action\":\"work\"}".into();
        assert!(append_event(&mut conn, &invalid).is_err());
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM control_plane_events WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn different_runs_allocate_independent_sequences() {
        let mut conn = connection();
        let left = append_event(&mut conn, &run_event("event-a", "run-a", "key-a")).unwrap();
        let right = append_event(&mut conn, &run_event("event-b", "run-b", "key-b")).unwrap();
        assert_eq!((left.sequence, right.sequence), (1, 1));
    }

    #[test]
    fn events_reject_update_and_delete() {
        let mut conn = connection();
        append_run(&mut conn, "event-1", "run-1", "key-1");
        assert!(
            conn.execute(
                "UPDATE control_plane_events SET source_id='changed' WHERE event_id='event-1'",
                []
            )
            .is_err()
        );
        assert!(
            conn.execute(
                "DELETE FROM control_plane_events WHERE event_id='event-1'",
                []
            )
            .is_err()
        );
        assert_eq!(
            conn.query_row(
                "SELECT source_id FROM control_plane_events WHERE event_id='event-1'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "controller-1"
        );
    }

    #[test]
    #[ignore = "Task 4 installs truthful non-creation reducers; plan-order correction is recorded in the writer report"]
    fn concurrent_run_local_sequences_are_unique_and_gap_free() {
        panic!("RED retained until Task 4 reducers make multiple same-run commits truthful");
    }
}
