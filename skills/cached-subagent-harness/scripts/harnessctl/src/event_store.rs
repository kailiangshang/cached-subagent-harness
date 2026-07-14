use crate::schema::{
    JsonTopLevel, canonical_json, sqlite_now, validate_canonical_timestamp,
    validate_current_connection,
};
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
    source_floor: SourceFloor,
    effect: EventEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceFloor {
    ControllerObservation,
    AgentReport,
    Inference,
}

impl SourceFloor {
    const fn priority(self) -> i64 {
        match self {
            Self::ControllerObservation => 3,
            Self::AgentReport => 4,
            Self::Inference => 5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventEffect {
    Transition,
    Facet,
    Evidence,
    Usage,
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
        source_floor: SourceFloor::ControllerObservation,
        effect: EventEffect::Transition,
    }
}

const fn rule_with(
    name: &'static str,
    keys: &'static [&'static str],
    required_references: u8,
    allowed_references: u8,
    source_floor: SourceFloor,
    effect: EventEffect,
) -> EventRule {
    EventRule {
        name,
        keys,
        required_references,
        allowed_references,
        source_floor,
        effect,
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
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT,
    ),
    rule(
        "package_review_completed",
        &["v", "verdict", "review_evidence", "next_action"],
        PACKAGE | ASSIGNMENT,
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
    rule_with(
        "assignment_step_changed",
        &["v", "current_step"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
        SourceFloor::AgentReport,
        EventEffect::Facet,
    ),
    rule_with(
        "assignment_blocked",
        &["v", "blocker", "next_action"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
        SourceFloor::AgentReport,
        EventEffect::Facet,
    ),
    rule_with(
        "assignment_unblocked",
        &["v", "resolution", "next_action"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT,
        SourceFloor::ControllerObservation,
        EventEffect::Facet,
    ),
    rule_with(
        "assignment_reported",
        &["v", "report_path", "reported_at", "next_action"],
        PACKAGE | ASSIGNMENT,
        PACKAGE | ASSIGNMENT | ATTEMPT | SESSION,
        SourceFloor::AgentReport,
        EventEffect::Transition,
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
    rule_with(
        "attempt_reported",
        &["v", "reported_at", "next_action"],
        PACKAGE | ASSIGNMENT | ATTEMPT,
        ALL_REFERENCES,
        SourceFloor::AgentReport,
        EventEffect::Transition,
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
        ALL_REFERENCES,
        ALL_REFERENCES,
    ),
    rule_with(
        "session_heartbeat",
        &["v", "last_activity_at"],
        SESSION,
        ALL_REFERENCES,
        SourceFloor::AgentReport,
        EventEffect::Facet,
    ),
    rule_with(
        "session_blocked",
        &["v", "blocker", "next_action"],
        SESSION,
        ALL_REFERENCES,
        SourceFloor::AgentReport,
        EventEffect::Facet,
    ),
    rule_with(
        "session_unblocked",
        &["v", "resolution", "next_action"],
        SESSION,
        ALL_REFERENCES,
        SourceFloor::ControllerObservation,
        EventEffect::Facet,
    ),
    rule_with(
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
        SourceFloor::AgentReport,
        EventEffect::Transition,
    ),
    rule_with(
        "session_waited",
        &[
            "v",
            "last_waited_at",
            "consumed_report_ref",
            "terminal_observation",
        ],
        SESSION,
        ALL_REFERENCES,
        SourceFloor::ControllerObservation,
        EventEffect::Facet,
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
    rule_with(
        "session_interrupted",
        &["v", "interruption_reason", "interrupted_at", "next_action"],
        SESSION,
        ALL_REFERENCES,
        SourceFloor::ControllerObservation,
        EventEffect::Facet,
    ),
    rule_with(
        "session_close_requested",
        &["v", "close_requested_at", "next_action"],
        SESSION,
        ALL_REFERENCES,
        SourceFloor::ControllerObservation,
        EventEffect::Facet,
    ),
    rule(
        "session_closed",
        &["v", "outcome", "close_disposition", "closed_at", "ended_at"],
        SESSION,
        ALL_REFERENCES,
    ),
    rule_with(
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
        SourceFloor::ControllerObservation,
        EventEffect::Facet,
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
    rule_with(
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
        SourceFloor::Inference,
        EventEffect::Usage,
    ),
    rule_with(
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
        SourceFloor::ControllerObservation,
        EventEffect::Evidence,
    ),
    rule_with(
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
        SourceFloor::ControllerObservation,
        EventEffect::Evidence,
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
    (u8::from(event.package_id.is_some()) * PACKAGE)
        | (u8::from(event.assignment_id.is_some()) * ASSIGNMENT)
        | (u8::from(event.attempt_id.is_some()) * ATTEMPT)
        | (u8::from(event.session_id.is_some()) * SESSION)
        | (u8::from(event.lease_id.is_some()) * LEASE)
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
    if source_priority(&event.source_kind)? > rule.source_floor.priority() {
        return Err(format!(
            "unauthorized source {} for {} {:?} effect",
            event.source_kind, event.event_type, rule.effect
        ));
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
                && !(session.is_none()
                    && matches!(
                        event.event_type.as_str(),
                        "session_planned"
                            | "session_running"
                            | "dispatch_reuse_selected"
                            | "dispatch_spawn_selected"
                            | "lease_reused"
                    ))
            {
                return Err(format!(
                    "attempt {attempt_id} does not belong to session {event_session}"
                ));
            }
            if let Some(event_lease) = &event.lease_id
                && lease.as_deref() != Some(event_lease.as_str())
                && !(lease.is_none()
                    && matches!(
                        event.event_type.as_str(),
                        "session_running" | "dispatch_reuse_selected" | "lease_reused"
                    ))
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

    if rule.name == "run_planned" {
        let run_exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM runs WHERE run_id=?1)",
                [&event.run_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if run_exists {
            return Err(format!("run already exists: {}", event.run_id));
        }
    }

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

    reduce_event(&transaction, event, sequence)?;
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

fn payload_text(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
) -> Result<String, String> {
    required_json_text(transaction, &event.payload_json, key)
}

fn payload_optional_text(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
) -> Result<Option<String>, String> {
    json_text(transaction, &event.payload_json, key)
}

fn payload_integer(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
) -> Result<i64, String> {
    json_i64(transaction, &event.payload_json, key)?
        .ok_or_else(|| format!("{}.{key} must be an integer", event.event_type))
}

fn payload_boolean(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
) -> Result<bool, String> {
    let path = format!("$.{key}");
    let (value_type, value): (Option<String>, Option<i64>) = transaction
        .query_row(
            "SELECT json_type(?1,?2),json_extract(?1,?2)",
            params![event.payload_json, path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    if !matches!(value_type.as_deref(), Some("true" | "false")) {
        return Err(format!("{}.{key} must be boolean", event.event_type));
    }
    Ok(value == Some(1))
}

fn payload_json_value(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
) -> Result<Option<String>, String> {
    let path = format!("$.{key}");
    transaction
        .query_row(
            "SELECT CASE WHEN json_type(?1,?2) IN ('object','array') THEN json_extract(?1,?2) END",
            params![event.payload_json, path],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

fn required_reference<'a>(value: &'a Option<String>, name: &str) -> Result<&'a str, String> {
    value
        .as_deref()
        .ok_or_else(|| format!("missing required event reference: {name}"))
}

fn current_status(
    transaction: &Transaction<'_>,
    table: &str,
    id_column: &str,
    id: &str,
) -> Result<String, String> {
    transaction
        .query_row(
            &format!("SELECT status FROM {table} WHERE {id_column}=?1"),
            [id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

fn require_state(
    transaction: &Transaction<'_>,
    table: &str,
    id_column: &str,
    id: &str,
    event_type: &str,
    allowed: &[&str],
) -> Result<String, String> {
    let state = current_status(transaction, table, id_column, id)?;
    if allowed.contains(&state.as_str()) {
        Ok(state)
    } else {
        Err(format!(
            "illegal transition {event_type} from {table} state {state}"
        ))
    }
}

fn reduce_event(
    transaction: &Transaction<'_>,
    event: &EventInput,
    sequence: i64,
) -> Result<(), String> {
    match event.event_type.as_str() {
        "run_planned" => reduce_run_planned(transaction, event),
        "run_started" => reduce_run_started(transaction, event),
        "run_blocked" => reduce_run_blocked(transaction, event),
        "run_unblocked" => reduce_run_unblocked(transaction, event),
        "run_completed" => reduce_run_completed(transaction, event),
        "run_cancelled" => reduce_run_cancelled(transaction, event),
        name if name.starts_with("package_") => reduce_package(transaction, event),
        name if name.starts_with("assignment_") => reduce_assignment(transaction, event),
        name if name.starts_with("attempt_") => reduce_attempt(transaction, event),
        name if name.starts_with("dispatch_") => {
            let attempt_id = required_reference(&event.attempt_id, "attempt_id")?;
            require_state(
                transaction,
                "assignment_attempts",
                "attempt_id",
                attempt_id,
                name,
                &["planned"],
            )?;
            Ok(())
        }
        name if name.starts_with("route_") => reduce_route(transaction, event),
        name if name.starts_with("session_") => reduce_session(transaction, event, sequence),
        name if name.starts_with("lease_") => reduce_lease(transaction, event),
        "usage_observed" => reduce_usage(transaction, event, sequence),
        "quality_gate_passed" | "quality_gate_failed" => {
            reduce_quality_gate(transaction, event, sequence)
        }
        _ => Err(format!("event_reducer_unavailable: {}", event.event_type)),
    }
}

fn reduce_run_started(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    require_state(
        transaction,
        "runs",
        "run_id",
        &event.run_id,
        &event.event_type,
        &["planned"],
    )?;
    transaction
        .execute(
            "UPDATE runs SET status='active',psoc_revision=?2,started_at=?3,next_action=?4 WHERE run_id=?1",
            params![event.run_id, payload_text(transaction,event,"psoc_revision")?, payload_text(transaction,event,"started_at")?, payload_text(transaction,event,"next_action")?],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn reduce_run_blocked(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    require_state(
        transaction,
        "runs",
        "run_id",
        &event.run_id,
        &event.event_type,
        &["active"],
    )?;
    transaction
        .execute(
            "UPDATE runs SET status='blocked',next_action=?2 WHERE run_id=?1",
            params![
                event.run_id,
                payload_text(transaction, event, "next_action")?
            ],
        )
        .map_err(|error| error.to_string())?;
    payload_text(transaction, event, "blocker")?;
    Ok(())
}

fn reduce_run_unblocked(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    require_state(
        transaction,
        "runs",
        "run_id",
        &event.run_id,
        &event.event_type,
        &["blocked"],
    )?;
    payload_text(transaction, event, "resolution")?;
    transaction
        .execute(
            "UPDATE runs SET status='active',next_action=?2 WHERE run_id=?1",
            params![
                event.run_id,
                payload_text(transaction, event, "next_action")?
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn reduce_run_completed(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    require_state(
        transaction,
        "runs",
        "run_id",
        &event.run_id,
        &event.event_type,
        &["active"],
    )?;
    transaction
        .execute(
            "UPDATE runs SET status='complete',completed_at=?2,ended_at=?3,next_action=NULL WHERE run_id=?1",
            params![event.run_id, payload_text(transaction,event,"completed_at")?, payload_text(transaction,event,"ended_at")?],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn reduce_run_cancelled(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    require_state(
        transaction,
        "runs",
        "run_id",
        &event.run_id,
        &event.event_type,
        &["planned", "active", "blocked"],
    )?;
    payload_text(transaction, event, "reason")?;
    transaction
        .execute(
            "UPDATE runs SET status='cancelled',ended_at=?2,next_action=?3 WHERE run_id=?1",
            params![
                event.run_id,
                payload_text(transaction, event, "ended_at")?,
                payload_text(transaction, event, "next_action")?
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn dependency_would_cycle(
    conn: &Connection,
    package_id: &str,
    depends_on_package_id: &str,
) -> Result<bool, String> {
    conn.query_row(
        r#"
        WITH RECURSIVE reachable(package_id) AS (
            SELECT ?2
            UNION
            SELECT d.depends_on_package_id
            FROM work_package_dependencies d
            JOIN reachable r ON d.package_id=r.package_id
        )
        SELECT EXISTS(SELECT 1 FROM reachable WHERE package_id=?1)
        "#,
        params![package_id, depends_on_package_id],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn policy_envelope(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
    registry: &[&str],
) -> Result<String, String> {
    let policy = payload_json_value(transaction, event, key)?
        .ok_or_else(|| format!("{}.{key} must be a policy object", event.event_type))?;
    if object_keys(transaction, &policy)? != ["v", "kind"] {
        return Err(format!(
            "{}.{key} has an invalid policy envelope",
            event.event_type
        ));
    }
    let (version, kind): (i64, String) = transaction
        .query_row(
            "SELECT json_extract(?1,'$.v'),json_extract(?1,'$.kind')",
            [&policy],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    if version != 1 || !registry.contains(&kind.as_str()) {
        return Err(format!(
            "{}.{key} contains an unknown policy",
            event.event_type
        ));
    }
    Ok(policy)
}

fn scope_envelope(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
) -> Result<String, String> {
    let scope = payload_json_value(transaction, event, key)?
        .ok_or_else(|| format!("{}.{key} must be a scope object", event.event_type))?;
    if object_keys(transaction, &scope)? != ["v", "paths"] {
        return Err(format!(
            "{}.{key} has an invalid scope envelope",
            event.event_type
        ));
    }
    let version: i64 = transaction
        .query_row("SELECT json_extract(?1,'$.v')", [&scope], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if version != 1 {
        return Err(format!("{}.{key} requires v=1", event.event_type));
    }
    let paths_type: String = transaction
        .query_row("SELECT json_type(?1,'$.paths')", [&scope], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if paths_type != "array" {
        return Err(format!("{}.{key}.paths must be an array", event.event_type));
    }
    let mut statement = transaction
        .prepare("SELECT type,value FROM json_each(?1,'$.paths') ORDER BY key")
        .map_err(|error| error.to_string())?;
    let paths: Vec<(String, String)> = statement
        .query_map([&scope], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| error.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|error| error.to_string())?;
    if paths
        .iter()
        .any(|(value_type, value)| value_type != "text" || value.is_empty())
        || paths.windows(2).any(|pair| pair[0].1 >= pair[1].1)
    {
        return Err(format!(
            "{}.{key}.paths must be sorted, nonempty strings without duplicates",
            event.event_type
        ));
    }
    Ok(scope)
}

fn evidence_envelope(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
) -> Result<String, String> {
    let evidence = payload_json_value(transaction, event, key)?
        .ok_or_else(|| format!("{}.{key} must be an evidence object", event.event_type))?;
    if object_keys(transaction, &evidence)? != ["v", "items"] {
        return Err(format!(
            "{}.{key} has an invalid evidence envelope",
            event.event_type
        ));
    }
    let (version, items_type): (i64, String) = transaction
        .query_row(
            "SELECT json_extract(?1,'$.v'),json_type(?1,'$.items')",
            [&evidence],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    if version != 1 || items_type != "array" {
        return Err(format!("{}.{key} has invalid v/items", event.event_type));
    }
    let mut statement = transaction
        .prepare("SELECT value FROM json_each(?1,'$.items') ORDER BY key")
        .map_err(|error| error.to_string())?;
    let items: Vec<String> = statement
        .query_map([&evidence], |row| row.get(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|error| error.to_string())?;
    for item in items {
        if object_keys(transaction, &item)? != ["kind", "ref", "result"] {
            return Err(format!(
                "{}.{key} contains an invalid evidence item",
                event.event_type
            ));
        }
        let (kind, reference, result): (String, String, String) = transaction
            .query_row(
                "SELECT json_extract(?1,'$.kind'),json_extract(?1,'$.ref'),json_extract(?1,'$.result')",
                [&item],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| error.to_string())?;
        if kind.is_empty() || reference.is_empty() || result.is_empty() {
            return Err(format!(
                "{}.{key} evidence strings must be nonempty",
                event.event_type
            ));
        }
    }
    Ok(evidence)
}

fn reduce_package(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    let package_id = required_reference(&event.package_id, "package_id")?;
    match event.event_type.as_str() {
        "package_planned" => {
            let dependency_type: Option<String> = transaction
                .query_row(
                    "SELECT json_type(?1,'$.dependencies')",
                    [&event.payload_json],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if dependency_type.as_deref() != Some("array") {
                return Err("package_planned.dependencies must be an array".into());
            }
            let dependencies = payload_json_value(transaction, event, "dependencies")?
                .ok_or_else(|| "package_planned.dependencies must be an array".to_string())?;
            let mut dependency_statement = transaction
                .prepare("SELECT type,value FROM json_each(?1) ORDER BY key")
                .map_err(|error| error.to_string())?;
            let dependency_rows: Vec<(String, Option<String>)> = dependency_statement
                .query_map([&dependencies], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(|error| error.to_string())?
                .collect::<Result<_, _>>()
                .map_err(|error| error.to_string())?;
            let mut values = Vec::with_capacity(dependency_rows.len());
            for (value_type, value) in dependency_rows {
                let Some(value) = value.filter(|value| !value.is_empty()) else {
                    return Err("package dependencies must be nonempty strings".into());
                };
                if value_type != "text" {
                    return Err("package dependencies must be nonempty strings".into());
                }
                values.push(value);
            }
            if values.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err("package dependencies must be sorted and duplicate-free".into());
            }
            let write_scope = scope_envelope(transaction, event, "write_scope")?;
            transaction
                .execute(
                    r#"
                    INSERT INTO work_packages(
                        package_id,run_id,title,role_floor,model_floor,risk_floor,
                        write_scope,review_policy,independence_policy,status,next_action
                    ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'planned',?10)
                    "#,
                    params![
                        package_id,
                        event.run_id,
                        payload_text(transaction, event, "title")?,
                        payload_text(transaction, event, "role_floor")?,
                        payload_text(transaction, event, "model_floor")?,
                        payload_text(transaction, event, "risk_floor")?,
                        write_scope,
                        policy_envelope(
                            transaction,
                            event,
                            "review_policy",
                            &["none", "deterministic", "independent"]
                        )?,
                        policy_envelope(
                            transaction,
                            event,
                            "independence_policy",
                            &["none", "different-session", "different-role-and-session"]
                        )?,
                        payload_text(transaction, event, "next_action")?,
                    ],
                )
                .map_err(|error| error.to_string())?;
            for dependency in values {
                if dependency == package_id {
                    return Err("package cannot depend on itself".into());
                }
                let owner: Option<String> = transaction
                    .query_row(
                        "SELECT run_id FROM work_packages WHERE package_id=?1",
                        [&dependency],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?;
                if owner.as_deref() != Some(event.run_id.as_str()) {
                    return Err(format!(
                        "dependency {dependency} does not belong to run {}",
                        event.run_id
                    ));
                }
                if dependency_would_cycle(transaction, package_id, &dependency)? {
                    return Err(format!(
                        "package dependency {package_id}->{dependency} would create a cycle"
                    ));
                }
                transaction
                    .execute(
                        "INSERT INTO work_package_dependencies(package_id,depends_on_package_id) VALUES(?1,?2)",
                        params![package_id, dependency],
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
        "package_ready" => {
            require_state(
                transaction,
                "work_packages",
                "package_id",
                package_id,
                &event.event_type,
                &["planned"],
            )?;
            let incomplete: i64 = transaction
                .query_row(
                    r#"
                    SELECT COUNT(*) FROM work_package_dependencies d
                    JOIN work_packages p ON p.package_id=d.depends_on_package_id
                    WHERE d.package_id=?1 AND p.status<>'complete'
                    "#,
                    [package_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if incomplete != 0 {
                return Err("package dependencies are not complete".into());
            }
            transaction
                .execute(
                    "UPDATE work_packages SET status='ready',next_action=?2 WHERE package_id=?1",
                    params![package_id, payload_text(transaction, event, "next_action")?],
                )
                .map_err(|error| error.to_string())?;
        }
        "package_active" => {
            require_state(
                transaction,
                "work_packages",
                "package_id",
                package_id,
                &event.event_type,
                &["ready"],
            )?;
            transaction
                .execute(
                    "UPDATE work_packages SET status='active',next_action=?2 WHERE package_id=?1",
                    params![package_id, payload_text(transaction, event, "next_action")?],
                )
                .map_err(|error| error.to_string())?;
        }
        "package_blocked" => {
            let state = require_state(
                transaction,
                "work_packages",
                "package_id",
                package_id,
                &event.event_type,
                &["ready", "active", "review"],
            )?;
            let resume = payload_text(transaction, event, "resume_status")?;
            if resume != state {
                return Err(format!(
                    "package_blocked resume_status {resume} does not match {state}"
                ));
            }
            transaction.execute("UPDATE work_packages SET status='blocked',blocker=?2,next_action=?3 WHERE package_id=?1",params![package_id,payload_text(transaction,event,"blocker")?,payload_text(transaction,event,"next_action")?]).map_err(|error| error.to_string())?;
        }
        "package_unblocked" => {
            require_state(
                transaction,
                "work_packages",
                "package_id",
                package_id,
                &event.event_type,
                &["blocked"],
            )?;
            payload_text(transaction, event, "resolution")?;
            let resume = payload_text(transaction, event, "resume_status")?;
            if !["ready", "active", "review"].contains(&resume.as_str()) {
                return Err("invalid package resume_status".into());
            }
            let prior: Option<String> = transaction.query_row(
                "SELECT json_extract(payload_json,'$.resume_status') FROM control_plane_events WHERE package_id=?1 AND event_type='package_blocked' AND sequence < (SELECT sequence FROM control_plane_events WHERE event_id=?2) ORDER BY sequence DESC LIMIT 1",
                params![package_id,event.event_id], |row| row.get(0)).optional().map_err(|error| error.to_string())?;
            if prior.as_deref() != Some(resume.as_str()) {
                return Err("package_unblocked resume_status does not match blocker event".into());
            }
            transaction.execute("UPDATE work_packages SET status=?2,blocker=NULL,next_action=?3 WHERE package_id=?1",params![package_id,resume,payload_text(transaction,event,"next_action")?]).map_err(|error| error.to_string())?;
        }
        "package_review_started" => {
            require_state(
                transaction,
                "work_packages",
                "package_id",
                package_id,
                &event.event_type,
                &["active"],
            )?;
            let review_id = payload_text(transaction, event, "review_assignment_id")?;
            if event.assignment_id.as_deref() != Some(review_id.as_str()) {
                return Err("package_review_started assignment reference mismatch".into());
            }
            let owner: Option<String>=transaction.query_row("SELECT package_id FROM assignments WHERE assignment_id=?1 AND assignment_kind='review'",[&review_id],|row|row.get(0)).optional().map_err(|error| error.to_string())?;
            if owner.as_deref() != Some(package_id) {
                return Err("review assignment does not belong to package".into());
            }
            transaction
                .execute(
                    "UPDATE work_packages SET status='review',next_action=?2 WHERE package_id=?1",
                    params![package_id, payload_text(transaction, event, "next_action")?],
                )
                .map_err(|error| error.to_string())?;
        }
        "package_review_completed" => {
            require_state(
                transaction,
                "work_packages",
                "package_id",
                package_id,
                &event.event_type,
                &["review"],
            )?;
            let review_id = required_reference(&event.assignment_id, "assignment_id")?;
            validate_completed_review_assignment(transaction, event, package_id, review_id)?;
            let verdict = payload_text(transaction, event, "verdict")?;
            let evidence = evidence_envelope(transaction, event, "review_evidence")?;
            let report_path: String = transaction
                .query_row(
                    "SELECT report_path FROM assignments WHERE assignment_id=?1",
                    [review_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            let evidence_has_report: i64 = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM json_each(?1,'$.items') WHERE json_extract(value,'$.ref')=?2)",
                    params![evidence, report_path],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if evidence_has_report != 1 {
                return Err(
                    "package review evidence does not reference the accepted review report".into(),
                );
            }
            let target = match verdict.as_str() {
                "accepted" => "review",
                "changes_required" => "active",
                _ => return Err("invalid package review verdict".into()),
            };
            transaction
                .execute(
                    "UPDATE work_packages SET status=?2,next_action=?3 WHERE package_id=?1",
                    params![
                        package_id,
                        target,
                        payload_text(transaction, event, "next_action")?
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        "package_completed" => {
            let state = current_status(transaction, "work_packages", "package_id", package_id)?;
            if state == "active" {
                let policy:String=transaction.query_row("SELECT json_extract(review_policy,'$.kind') FROM work_packages WHERE package_id=?1",[package_id],|row|row.get(0)).map_err(|error|error.to_string())?;
                if policy != "none" {
                    return Err("package requires review before completion".into());
                }
            } else if state == "review" {
                let verdict:Option<String>=transaction.query_row("SELECT json_extract(payload_json,'$.verdict') FROM control_plane_events WHERE package_id=?1 AND event_type='package_review_completed' ORDER BY sequence DESC LIMIT 1",[package_id],|row|row.get(0)).optional().map_err(|error|error.to_string())?;
                if verdict.as_deref() != Some("accepted") {
                    return Err("package review is not accepted".into());
                }
            } else {
                return Err(format!(
                    "illegal transition package_completed from work_packages state {state}"
                ));
            }
            transaction.execute("UPDATE work_packages SET status='complete',ended_at=?2,next_action=NULL,blocker=NULL WHERE package_id=?1",params![package_id,payload_text(transaction,event,"ended_at")?]).map_err(|error|error.to_string())?;
        }
        "package_cancelled" => {
            require_state(
                transaction,
                "work_packages",
                "package_id",
                package_id,
                &event.event_type,
                &["planned", "ready", "active", "review", "blocked"],
            )?;
            payload_text(transaction, event, "reason")?;
            transaction.execute("UPDATE work_packages SET status='cancelled',ended_at=?2,next_action=?3 WHERE package_id=?1",params![package_id,payload_text(transaction,event,"ended_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        _ => return Err(format!("event_reducer_unavailable: {}", event.event_type)),
    }
    Ok(())
}

struct ReviewAssignmentState {
    owner: String,
    kind: String,
    role: String,
    status: String,
    report_path: Option<String>,
    test_evidence: Option<String>,
    review_evidence: Option<String>,
    attempt_id: Option<String>,
}

fn validate_completed_review_assignment(
    transaction: &Transaction<'_>,
    event: &EventInput,
    package_id: &str,
    review_id: &str,
) -> Result<(), String> {
    let started: Option<(String, Option<String>)> = transaction
        .query_row(
            r#"
            SELECT json_extract(payload_json,'$.review_assignment_id'), assignment_id
            FROM control_plane_events
            WHERE package_id=?1 AND event_type='package_review_started'
              AND sequence < (SELECT sequence FROM control_plane_events WHERE event_id=?2)
            ORDER BY sequence DESC LIMIT 1
            "#,
            params![package_id, event.event_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if started
        .as_ref()
        .map(|(payload_id, event_id)| (payload_id.as_str(), event_id.as_deref()))
        != Some((review_id, Some(review_id)))
    {
        return Err("package review completion does not match the active review assignment".into());
    }

    let review = transaction
        .query_row(
            "SELECT package_id,assignment_kind,required_role,status,report_path,test_evidence,review_evidence,current_attempt_id FROM assignments WHERE assignment_id=?1",
            [review_id],
            |row| Ok(ReviewAssignmentState {
                owner: row.get(0)?,
                kind: row.get(1)?,
                role: row.get(2)?,
                status: row.get(3)?,
                report_path: row.get(4)?,
                test_evidence: row.get(5)?,
                review_evidence: row.get(6)?,
                attempt_id: row.get(7)?,
            }),
        )
        .map_err(|error| error.to_string())?;
    if review.owner != package_id
        || review.kind != "review"
        || review.role != "reviewer"
        || review.status != "accepted"
    {
        return Err(
            "package review assignment is not an accepted owned reviewer assignment".into(),
        );
    }
    if review.report_path.as_deref().is_none_or(str::is_empty)
        || review.test_evidence.is_none()
        || review.review_evidence.is_none()
    {
        return Err("accepted review assignment is missing report or gate evidence".into());
    }
    let attempt_id = review
        .attempt_id
        .ok_or_else(|| "accepted review assignment has no attempt".to_string())?;
    let (attempt_status, review_session): (String, Option<String>) = transaction
        .query_row(
            "SELECT status,session_id FROM assignment_attempts WHERE attempt_id=?1 AND assignment_id=?2",
            params![attempt_id, review_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    if attempt_status != "accepted" {
        return Err("review assignment current attempt is not accepted".into());
    }
    let independence: String = transaction
        .query_row(
            "SELECT json_extract(independence_policy,'$.kind') FROM work_packages WHERE package_id=?1",
            [package_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if independence != "none" {
        let review_session = review_session
            .ok_or_else(|| "independent review requires a reviewer session".to_string())?;
        let session_role: String = transaction
            .query_row(
                "SELECT role FROM agent_sessions WHERE session_id=?1",
                [&review_session],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if session_role != "reviewer" {
            return Err("independent review session must use reviewer role".into());
        }
        let overlapping_writer: i64 = transaction
            .query_row(
                r#"
                SELECT COUNT(*) FROM assignment_attempts aa
                JOIN assignments a ON a.assignment_id=aa.assignment_id
                WHERE a.package_id=?1 AND a.assignment_id<>?2
                  AND a.assignment_kind IN ('implementation','fix')
                  AND aa.session_id=?3
                "#,
                params![package_id, review_id, review_session],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if overlapping_writer != 0 {
            return Err("independent review cannot reuse a writer or fixer session".into());
        }
    }
    Ok(())
}

fn reduce_assignment(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    let assignment_id = required_reference(&event.assignment_id, "assignment_id")?;
    let package_id = required_reference(&event.package_id, "package_id")?;
    match event.event_type.as_str() {
        "assignment_planned" => {
            let write_scope = scope_envelope(transaction, event, "write_scope")?;
            let sequence = payload_integer(transaction, event, "sequence")?;
            if sequence < 1 {
                return Err("assignment sequence must be positive".into());
            }
            let kind = payload_text(transaction, event, "assignment_kind")?;
            let role = payload_text(transaction, event, "required_role")?;
            let compatible = matches!(
                (kind.as_str(), role.as_str()),
                ("discussion", "discussion")
                    | ("exploration", "explorer")
                    | ("implementation", "worker")
                    | ("review", "reviewer")
                    | ("fix", "fixer")
            );
            if !compatible {
                return Err(format!(
                    "assignment kind {kind} is incompatible with role {role}"
                ));
            }
            transaction
                .execute(
                    r#"
                    INSERT INTO assignments(
                        assignment_id,package_id,title,sequence,assignment_kind,required_role,
                        model_floor,risk_class,write_scope,base_revision,
                        independence_boundary_id,current_attempt_id,attempt_count,status,next_action
                    ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,NULL,0,'planned',?12)
                    "#,
                    params![
                        assignment_id,
                        package_id,
                        payload_text(transaction, event, "title")?,
                        sequence,
                        kind,
                        role,
                        payload_text(transaction, event, "model_floor")?,
                        payload_text(transaction, event, "risk_class")?,
                        write_scope,
                        payload_optional_text(transaction, event, "base_revision")?,
                        payload_optional_text(transaction, event, "independence_boundary_id")?,
                        payload_text(transaction, event, "next_action")?,
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        "assignment_queued" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["planned"],
            )?;
            transaction
                .execute(
                    "UPDATE assignments SET status='queued',next_action=?2 WHERE assignment_id=?1",
                    params![
                        assignment_id,
                        payload_text(transaction, event, "next_action")?
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        "assignment_started" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["queued"],
            )?;
            let attempt = payload_text(transaction, event, "attempt_id")?;
            if event.attempt_id.as_deref() != Some(attempt.as_str()) {
                return Err("assignment_started attempt_id does not match event".into());
            }
            let current: Option<String> = transaction
                .query_row(
                    "SELECT current_attempt_id FROM assignments WHERE assignment_id=?1",
                    [assignment_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if current.as_deref() != Some(attempt.as_str()) {
                return Err("assignment_started attempt is not current".into());
            }
            require_state(
                transaction,
                "assignment_attempts",
                "attempt_id",
                &attempt,
                &event.event_type,
                &["running"],
            )?;
            transaction.execute("UPDATE assignments SET status='running',started_at=?2,current_step=?3,next_action=NULL WHERE assignment_id=?1",params![assignment_id,payload_text(transaction,event,"started_at")?,payload_text(transaction,event,"current_step")?]).map_err(|error|error.to_string())?;
        }
        "assignment_step_changed" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["running"],
            )?;
            transaction
                .execute(
                    "UPDATE assignments SET current_step=?2 WHERE assignment_id=?1",
                    params![
                        assignment_id,
                        payload_text(transaction, event, "current_step")?
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        "assignment_blocked" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["queued", "running", "reported", "validated"],
            )?;
            transaction
                .execute(
                    "UPDATE assignments SET blocker=?2,next_action=?3 WHERE assignment_id=?1",
                    params![
                        assignment_id,
                        payload_text(transaction, event, "blocker")?,
                        payload_text(transaction, event, "next_action")?
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        "assignment_unblocked" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["queued", "running", "reported", "validated"],
            )?;
            let blocker: Option<String> = transaction
                .query_row(
                    "SELECT blocker FROM assignments WHERE assignment_id=?1",
                    [assignment_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if blocker.is_none() {
                return Err("assignment is not blocked".into());
            }
            payload_text(transaction, event, "resolution")?;
            transaction
                .execute(
                    "UPDATE assignments SET blocker=NULL,next_action=?2 WHERE assignment_id=?1",
                    params![
                        assignment_id,
                        payload_text(transaction, event, "next_action")?
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        "assignment_reported" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["running"],
            )?;
            transaction.execute("UPDATE assignments SET status='reported',report_path=?2,reported_at=?3,next_action=?4 WHERE assignment_id=?1",params![assignment_id,payload_text(transaction,event,"report_path")?,payload_text(transaction,event,"reported_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "assignment_validated" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["reported"],
            )?;
            let evidence = evidence_envelope(transaction, event, "test_evidence")?;
            transaction.execute("UPDATE assignments SET status='validated',test_evidence=?2,validated_at=?3,next_action=?4 WHERE assignment_id=?1",params![assignment_id,evidence,payload_text(transaction,event,"validated_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "assignment_accepted" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["validated"],
            )?;
            let evidence = evidence_envelope(transaction, event, "review_evidence")?;
            let (policy, stored_test_evidence, stored_review_evidence): (
                String,
                Option<String>,
                Option<String>,
            ) = transaction
                .query_row(
                    r#"
                    SELECT json_extract(p.review_policy,'$.kind'),a.test_evidence,a.review_evidence
                    FROM assignments a JOIN work_packages p ON p.package_id=a.package_id
                    WHERE a.assignment_id=?1
                    "#,
                    [assignment_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(|error| error.to_string())?;
            match policy.as_str() {
                "none" => {}
                "deterministic" if stored_test_evidence.is_some() => {}
                "independent" if stored_review_evidence.as_deref() == Some(evidence.as_str()) => {}
                "deterministic" => {
                    return Err(
                        "deterministic assignment acceptance requires test gate evidence".into(),
                    );
                }
                "independent" => {
                    return Err(
                        "independent assignment acceptance requires prior matching gate evidence"
                            .into(),
                    );
                }
                _ => return Err("unknown assignment review policy".into()),
            }
            transaction.execute("UPDATE assignments SET status='accepted',review_evidence=?2,accepted_at=?3,ended_at=?4,next_action=NULL,blocker=NULL WHERE assignment_id=?1",params![assignment_id,evidence,payload_text(transaction,event,"accepted_at")?,payload_text(transaction,event,"ended_at")?]).map_err(|error|error.to_string())?;
        }
        "assignment_requeued" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["queued", "running", "reported", "validated"],
            )?;
            let current: Option<String> = transaction
                .query_row(
                    "SELECT current_attempt_id FROM assignments WHERE assignment_id=?1",
                    [assignment_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            let current = current.ok_or_else(|| "assignment has no current attempt".to_string())?;
            require_state(
                transaction,
                "assignment_attempts",
                "attempt_id",
                &current,
                &event.event_type,
                &["failed"],
            )?;
            payload_text(transaction, event, "reason")?;
            transaction.execute("UPDATE assignments SET status='queued',blocker=NULL,next_action=?2 WHERE assignment_id=?1",params![assignment_id,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "assignment_failed" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["queued", "running", "reported", "validated"],
            )?;
            if !payload_boolean(transaction, event, "policy_exhausted")? {
                return Err("assignment_failed requires policy_exhausted=true".into());
            }
            transaction.execute("UPDATE assignments SET status='failed',final_reason=?2,ended_at=?3,next_action=?4 WHERE assignment_id=?1",params![assignment_id,payload_text(transaction,event,"final_reason")?,payload_text(transaction,event,"ended_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "assignment_cancelled" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["planned", "queued", "running", "reported", "validated"],
            )?;
            transaction.execute("UPDATE assignments SET status='cancelled',final_reason=?2,ended_at=?3,next_action=?4 WHERE assignment_id=?1",params![assignment_id,payload_text(transaction,event,"final_reason")?,payload_text(transaction,event,"ended_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        _ => return Err(format!("event_reducer_unavailable: {}", event.event_type)),
    }
    Ok(())
}

fn reduce_attempt(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    let attempt_id = required_reference(&event.attempt_id, "attempt_id")?;
    let assignment_id = required_reference(&event.assignment_id, "assignment_id")?;
    match event.event_type.as_str() {
        "attempt_planned" => {
            require_state(
                transaction,
                "assignments",
                "assignment_id",
                assignment_id,
                &event.event_type,
                &["planned", "queued"],
            )?;
            let attempt_sequence = payload_integer(transaction, event, "attempt_sequence")?;
            let count: i64 = transaction
                .query_row(
                    "SELECT attempt_count FROM assignments WHERE assignment_id=?1",
                    [assignment_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if attempt_sequence != count + 1 {
                return Err(format!(
                    "attempt sequence must be {}, got {attempt_sequence}",
                    count + 1
                ));
            }
            transaction.execute(
                "INSERT INTO assignment_attempts(attempt_id,assignment_id,session_id,lease_id,attempt_sequence,status,next_action) VALUES(?1,?2,?3,?4,?5,'planned',?6)",
                params![attempt_id,assignment_id,event.session_id,event.lease_id,attempt_sequence,payload_text(transaction,event,"next_action")?]
            ).map_err(|error|error.to_string())?;
            transaction.execute("UPDATE assignments SET current_attempt_id=?2,attempt_count=attempt_count+1 WHERE assignment_id=?1",params![assignment_id,attempt_id]).map_err(|error|error.to_string())?;
        }
        "attempt_started" => {
            require_state(
                transaction,
                "assignment_attempts",
                "attempt_id",
                attempt_id,
                &event.event_type,
                &["planned"],
            )?;
            transaction.execute("UPDATE assignment_attempts SET status='running',started_at=?2,next_action=?3 WHERE attempt_id=?1",params![attempt_id,payload_text(transaction,event,"started_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "attempt_reported" => {
            require_state(
                transaction,
                "assignment_attempts",
                "attempt_id",
                attempt_id,
                &event.event_type,
                &["running"],
            )?;
            transaction.execute("UPDATE assignment_attempts SET status='reported',reported_at=?2,next_action=?3 WHERE attempt_id=?1",params![attempt_id,payload_text(transaction,event,"reported_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "attempt_validated" => {
            require_state(
                transaction,
                "assignment_attempts",
                "attempt_id",
                attempt_id,
                &event.event_type,
                &["reported"],
            )?;
            transaction.execute("UPDATE assignment_attempts SET status='validated',validated_at=?2,next_action=?3 WHERE attempt_id=?1",params![attempt_id,payload_text(transaction,event,"validated_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "attempt_accepted" => {
            require_state(
                transaction,
                "assignment_attempts",
                "attempt_id",
                attempt_id,
                &event.event_type,
                &["validated"],
            )?;
            transaction.execute("UPDATE assignment_attempts SET status='accepted',accepted_at=?2,ended_at=?3,next_action=NULL WHERE attempt_id=?1",params![attempt_id,payload_text(transaction,event,"accepted_at")?,payload_text(transaction,event,"ended_at")?]).map_err(|error|error.to_string())?;
        }
        "attempt_failed" => {
            require_state(
                transaction,
                "assignment_attempts",
                "attempt_id",
                attempt_id,
                &event.event_type,
                &["planned", "running", "reported", "validated"],
            )?;
            transaction.execute("UPDATE assignment_attempts SET status='failed',outcome_reason=?2,ended_at=?3,next_action=?4 WHERE attempt_id=?1",params![attempt_id,payload_text(transaction,event,"outcome_reason")?,payload_text(transaction,event,"ended_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "attempt_cancelled" => {
            require_state(
                transaction,
                "assignment_attempts",
                "attempt_id",
                attempt_id,
                &event.event_type,
                &["planned", "running", "reported", "validated"],
            )?;
            transaction.execute("UPDATE assignment_attempts SET status='cancelled',outcome_reason=?2,ended_at=?3,next_action=?4 WHERE attempt_id=?1",params![attempt_id,payload_text(transaction,event,"outcome_reason")?,payload_text(transaction,event,"ended_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        _ => return Err(format!("event_reducer_unavailable: {}", event.event_type)),
    }
    Ok(())
}

fn current_route_id(transaction: &Transaction<'_>, attempt_id: &str) -> Result<String, String> {
    transaction
        .query_row(
            "SELECT route_id FROM assignment_attempts WHERE attempt_id=?1",
            [attempt_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("attempt {attempt_id} has no current route"))
}

fn require_route_status(
    transaction: &Transaction<'_>,
    route_id: &str,
    event_type: &str,
    allowed: &[&str],
) -> Result<String, String> {
    let status: String = transaction
        .query_row(
            "SELECT routing_status FROM routing_decisions WHERE route_id=?1",
            [route_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if allowed.contains(&status.as_str()) {
        Ok(status)
    } else {
        Err(format!(
            "illegal transition {event_type} from routing state {status}"
        ))
    }
}

fn reduce_route(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    let attempt_id = required_reference(&event.attempt_id, "attempt_id")?;
    match event.event_type.as_str() {
        "route_requested" => {
            require_state(
                transaction,
                "assignment_attempts",
                "attempt_id",
                attempt_id,
                &event.event_type,
                &["planned"],
            )?;
            let route_id = payload_text(transaction, event, "route_id")?;
            let escalated = payload_optional_text(transaction, event, "escalated_from_route_id")?;
            if let Some(previous) = &escalated {
                let owner: Option<String> = transaction
                    .query_row(
                        "SELECT attempt_id FROM routing_decisions WHERE route_id=?1",
                        [previous],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?;
                if owner.as_deref() != Some(attempt_id) {
                    return Err("escalated route does not belong to attempt".into());
                }
            }
            transaction
                .execute(
                    r#"
                INSERT INTO routing_decisions(
                    route_id,attempt_id,required_profile,requested_model,requested_reasoning,
                    routing_status,eligibility_status,escalated_from_route_id,next_action,decided_at
                ) VALUES(?1,?2,?3,?4,?5,'requested','unknown',?6,?7,?8)
                "#,
                    params![
                        route_id,
                        attempt_id,
                        payload_text(transaction, event, "required_profile")?,
                        payload_optional_text(transaction, event, "requested_model")?,
                        payload_optional_text(transaction, event, "requested_reasoning")?,
                        escalated,
                        payload_text(transaction, event, "next_action")?,
                        payload_text(transaction, event, "decided_at")?
                    ],
                )
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "UPDATE assignment_attempts SET route_id=?2 WHERE attempt_id=?1",
                    params![attempt_id, route_id],
                )
                .map_err(|error| error.to_string())?;
        }
        "route_applied" => {
            let route_id = current_route_id(transaction, attempt_id)?;
            require_route_status(
                transaction,
                &route_id,
                &event.event_type,
                &["requested", "degraded"],
            )?;
            let status = payload_text(transaction, event, "routing_status")?;
            if !["applied", "inherited"].contains(&status.as_str()) {
                return Err("route_applied requires applied or inherited status".into());
            }
            let evidence = evidence_envelope(transaction, event, "eligibility_evidence")?;
            let actual_model = payload_optional_text(transaction, event, "actual_model")?;
            let actual_reasoning = payload_optional_text(transaction, event, "actual_reasoning")?;
            transaction.execute("UPDATE routing_decisions SET routing_status=?2,eligibility_status='eligible',actual_model=?3,actual_reasoning=?4,eligibility_evidence=?5,next_action=NULL,decided_at=?6 WHERE route_id=?1",params![route_id,status,actual_model,actual_reasoning,evidence,payload_text(transaction,event,"decided_at")?]).map_err(|error|error.to_string())?;
            if let Some(session_id) = &event.session_id {
                transaction.execute("UPDATE agent_sessions SET routing_status=?2,actual_model=?3,actual_reasoning=?4 WHERE session_id=?1",params![session_id,status,actual_model,actual_reasoning]).map_err(|error|error.to_string())?;
            }
        }
        "route_degraded" => {
            let route_id = current_route_id(transaction, attempt_id)?;
            require_route_status(transaction, &route_id, &event.event_type, &["requested"])?;
            let eligibility = payload_text(transaction, event, "eligibility_status")?;
            if !["eligible", "rejected", "unknown"].contains(&eligibility.as_str()) {
                return Err("invalid route eligibility".into());
            }
            let evidence = evidence_envelope(transaction, event, "eligibility_evidence")?;
            payload_text(transaction, event, "reason")?;
            transaction.execute("UPDATE routing_decisions SET routing_status='degraded',eligibility_status=?2,actual_model=?3,actual_reasoning=?4,eligibility_evidence=?5,next_action=?6,decided_at=?7 WHERE route_id=?1",params![route_id,eligibility,payload_optional_text(transaction,event,"actual_model")?,payload_optional_text(transaction,event,"actual_reasoning")?,evidence,payload_text(transaction,event,"next_action")?,payload_text(transaction,event,"decided_at")?]).map_err(|error|error.to_string())?;
        }
        "route_rejected" => {
            let route_id = current_route_id(transaction, attempt_id)?;
            require_route_status(
                transaction,
                &route_id,
                &event.event_type,
                &["requested", "degraded"],
            )?;
            payload_text(transaction, event, "reason")?;
            let evidence = evidence_envelope(transaction, event, "eligibility_evidence")?;
            transaction.execute("UPDATE routing_decisions SET routing_status='rejected',eligibility_status='rejected',eligibility_evidence=?2,next_action=?3,decided_at=?4 WHERE route_id=?1",params![route_id,evidence,payload_text(transaction,event,"next_action")?,payload_text(transaction,event,"decided_at")?]).map_err(|error|error.to_string())?;
        }
        _ => return Err(format!("event_reducer_unavailable: {}", event.event_type)),
    }
    Ok(())
}

fn nested_object(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
) -> Result<String, String> {
    payload_json_value(transaction, event, key)?
        .ok_or_else(|| format!("{}.{key} must be an object", event.event_type))
}

fn object_keys(transaction: &Transaction<'_>, json: &str) -> Result<Vec<String>, String> {
    ordered_json_keys(transaction, json)
}

fn reduce_session(
    transaction: &Transaction<'_>,
    event: &EventInput,
    sequence: i64,
) -> Result<(), String> {
    let session_id = required_reference(&event.session_id, "session_id")?;
    match event.event_type.as_str() {
        "session_planned" => {
            let assignment_id = required_reference(&event.assignment_id, "assignment_id")?;
            let role: String = transaction
                .query_row(
                    "SELECT required_role FROM assignments WHERE assignment_id=?1",
                    [assignment_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            let auth = payload_text(transaction, event, "authorization_ref")?;
            if auth == session_id {
                return Err("session authorization cannot authorize itself".into());
            }
            payload_text(transaction, event, "budget_reason")?;
            let max_open = payload_integer(transaction, event, "run_max_open")?;
            let max_total = payload_integer(transaction, event, "run_max_total")?;
            let (stored_open,stored_total):(i64,i64)=transaction.query_row("SELECT json_extract(session_budget,'$.max_open'),json_extract(session_budget,'$.max_total') FROM runs WHERE run_id=?1",[&event.run_id],|row|Ok((row.get(0)?,row.get(1)?))).map_err(|error|error.to_string())?;
            if (max_open, max_total) != (stored_open, stored_total) {
                return Err("session authorization budget does not match run budget".into());
            }
            let token = nested_object(transaction, event, "session_token_budget")?;
            if object_keys(transaction, &token)? != ["mode", "tokens"] {
                return Err("invalid session_token_budget envelope".into());
            }
            let mode: String = transaction
                .query_row("SELECT json_extract(?1,'$.mode')", [&token], |row| {
                    row.get(0)
                })
                .map_err(|error| error.to_string())?;
            let tokens:Option<i64>=transaction.query_row("SELECT CASE WHEN json_type(?1,'$.tokens')='integer' THEN json_extract(?1,'$.tokens') END",[&token],|row|row.get(0)).map_err(|error|error.to_string())?;
            let valid_token_budget = match (mode.as_str(), tokens) {
                ("bounded", Some(value)) => value >= 0,
                ("unbounded", None) => true,
                _ => false,
            };
            if !valid_token_budget {
                return Err("invalid session token budget".into());
            }
            let delegation = nested_object(transaction, event, "nested_delegation")?;
            if object_keys(transaction, &delegation)? != ["allowed", "authority_ref"] {
                return Err("invalid nested_delegation envelope".into());
            }
            let (allowed_type, allowed_value): (String, i64) = transaction
                .query_row(
                    "SELECT json_type(?1,'$.allowed'),json_extract(?1,'$.allowed')",
                    [&delegation],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|error| error.to_string())?;
            if !matches!(allowed_type.as_str(), "true" | "false") {
                return Err("nested_delegation.allowed must be boolean".into());
            }
            let allowed = allowed_value == 1;
            let authority: Option<String> = transaction
                .query_row(
                    "SELECT json_extract(?1,'$.authority_ref')",
                    [&delegation],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            let parent = payload_optional_text(transaction, event, "parent_session_id")?;
            if allowed {
                if authority.as_deref().is_none_or(str::is_empty)
                    || parent.as_deref().is_none_or(str::is_empty)
                {
                    return Err(
                        "nested delegation requires authority_ref and parent_session_id".into(),
                    );
                }
                let parent_id = parent
                    .as_deref()
                    .ok_or_else(|| "nested delegation requires parent_session_id".to_string())?;
                let parent_run: Option<String> = transaction
                    .query_row(
                        "SELECT run_id FROM agent_sessions WHERE session_id=?1",
                        [parent_id],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?
                    .flatten();
                if parent_run.as_deref() != Some(event.run_id.as_str()) {
                    return Err("parent session does not belong to run".into());
                }
                let run_authority:Option<String>=transaction.query_row("SELECT json_extract(session_budget,'$.override_reason') FROM runs WHERE run_id=?1",[&event.run_id],|row|row.get(0)).map_err(|error|error.to_string())?;
                if run_authority.as_deref().is_none_or(str::is_empty) {
                    return Err(
                        "nested delegation authority is not recorded in the run budget".into(),
                    );
                }
            } else if authority.is_some() || parent.is_some() {
                return Err("disabled nested delegation requires null authority and parent".into());
            }
            let profile = payload_text(transaction, event, "requested_profile")?;
            transaction.execute(
                "INSERT INTO agent_sessions(session_id,run_id,role,host_id,requested_profile,requested_model,requested_reasoning,token_budget,token_budget_mode,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'planned')",
                params![session_id,event.run_id,role,payload_optional_text(transaction,event,"requested_host")?,profile,payload_optional_text(transaction,event,"requested_model")?,payload_optional_text(transaction,event,"requested_reasoning")?,tokens,mode]
            ).map_err(|error|error.to_string())?;
        }
        "session_spawned" => {
            require_state(
                transaction,
                "agent_sessions",
                "session_id",
                session_id,
                &event.event_type,
                &["planned"],
            )?;
            transaction.execute("UPDATE agent_sessions SET status='spawned',handle=?2,host_id=?3,spawned_at=?4,next_action=?5 WHERE session_id=?1",params![session_id,payload_text(transaction,event,"handle")?,payload_text(transaction,event,"host_id")?,payload_text(transaction,event,"spawned_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "session_running" => {
            let prior = require_state(
                transaction,
                "agent_sessions",
                "session_id",
                session_id,
                &event.event_type,
                &["spawned", "reported"],
            )?;
            let attempt = payload_text(transaction, event, "attempt_id")?;
            let lease = payload_text(transaction, event, "lease_id")?;
            let package_ref = required_reference(&event.package_id, "package_id")?;
            let assignment_ref = required_reference(&event.assignment_id, "assignment_id")?;
            if event.attempt_id.as_deref() != Some(attempt.as_str())
                || event.lease_id.as_deref() != Some(lease.as_str())
            {
                return Err("session_running payload identity mismatch".into());
            }
            if prior == "reported"
                && (payload_optional_text(transaction, event, "prior_report_ref")?.is_none()
                    || evidence_envelope(transaction, event, "gate_evidence").is_err())
            {
                return Err(
                    "reported session reuse requires consumed report and gate evidence".into(),
                );
            }
            let lease_owner: (String, String, String, Option<String>) = transaction
                .query_row(
                    "SELECT session_id,package_id,status,current_attempt_id FROM session_leases WHERE lease_id=?1",
                    [&lease],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .map_err(|error| error.to_string())?;
            if lease_owner.0 != session_id {
                return Err("lease does not belong to session".into());
            }
            if lease_owner.1 != package_ref
                || lease_owner.2 != "active"
                || lease_owner
                    .3
                    .as_deref()
                    .is_some_and(|current| current != attempt)
            {
                return Err("session_running requires an active compatible lease".into());
            }
            let (attempt_owner, attempt_status, bound_session, bound_lease): (
                String,
                String,
                Option<String>,
                Option<String>,
            ) = transaction
                .query_row(
                    "SELECT assignment_id,status,session_id,lease_id FROM assignment_attempts WHERE attempt_id=?1",
                    [&attempt],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .map_err(|error| error.to_string())?;
            if attempt_owner != assignment_ref
                || attempt_status != "planned"
                || bound_session
                    .as_deref()
                    .is_some_and(|bound| bound != session_id)
                || bound_lease.as_deref().is_some_and(|bound| bound != lease)
            {
                return Err("session_running requires a usable planned attempt".into());
            }
            let changed = transaction.execute("UPDATE assignment_attempts SET session_id=?2,lease_id=?3 WHERE attempt_id=?1 AND status='planned' AND (session_id IS NULL OR session_id=?2) AND (lease_id IS NULL OR lease_id=?3)",params![attempt,session_id,lease]).map_err(|error|error.to_string())?;
            if changed != 1 {
                return Err("session_running attempt binding failed".into());
            }
            transaction
                .execute(
                    "UPDATE session_leases SET current_attempt_id=?2 WHERE lease_id=?1",
                    params![lease, attempt],
                )
                .map_err(|error| error.to_string())?;
            transaction.execute("UPDATE agent_sessions SET status='running',last_activity_at=?2,next_action=?3 WHERE session_id=?1",params![session_id,payload_text(transaction,event,"started_or_resumed_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "session_heartbeat" => {
            require_state(
                transaction,
                "agent_sessions",
                "session_id",
                session_id,
                &event.event_type,
                &["spawned", "running", "reported"],
            )?;
            let incoming = payload_text(transaction, event, "last_activity_at")?;
            let current: Option<String> = transaction
                .query_row(
                    "SELECT last_activity_at FROM agent_sessions WHERE session_id=?1",
                    [session_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            let current = current.unwrap_or_else(|| "null".to_string());
            if projection_wins(
                transaction,
                event,
                sequence,
                ProjectionClaim {
                    entity_kind: "session",
                    entity_id: session_id,
                    field_name: "last_activity_at",
                    current_value: &current,
                    incoming_value: &incoming,
                    illegal_correction: false,
                    supersedes: None,
                },
            )? {
                transaction
                    .execute(
                        "UPDATE agent_sessions SET last_activity_at=?2 WHERE session_id=?1",
                        params![session_id, incoming],
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
        "session_blocked" => {
            require_state(
                transaction,
                "agent_sessions",
                "session_id",
                session_id,
                &event.event_type,
                &["spawned", "running", "reported"],
            )?;
            if current_session_blocker_event(transaction, event)?.as_deref()
                == Some("session_blocked")
            {
                return Err("session is already blocked".into());
            }
            payload_text(transaction, event, "blocker")?;
            transaction
                .execute(
                    "UPDATE agent_sessions SET next_action=?2 WHERE session_id=?1",
                    params![session_id, payload_text(transaction, event, "next_action")?],
                )
                .map_err(|error| error.to_string())?;
        }
        "session_unblocked" => {
            require_state(
                transaction,
                "agent_sessions",
                "session_id",
                session_id,
                &event.event_type,
                &["spawned", "running", "reported"],
            )?;
            if current_session_blocker_event(transaction, event)?.as_deref()
                != Some("session_blocked")
            {
                return Err("session is not blocked".into());
            }
            payload_text(transaction, event, "resolution")?;
            transaction
                .execute(
                    "UPDATE agent_sessions SET next_action=?2 WHERE session_id=?1",
                    params![session_id, payload_text(transaction, event, "next_action")?],
                )
                .map_err(|error| error.to_string())?;
        }
        "session_reported" => {
            require_state(
                transaction,
                "agent_sessions",
                "session_id",
                session_id,
                &event.event_type,
                &["running"],
            )?;
            let assignment = payload_text(transaction, event, "assignment_id")?;
            let attempt = payload_text(transaction, event, "attempt_id")?;
            if event.assignment_id.as_deref() != Some(assignment.as_str())
                || event.attempt_id.as_deref() != Some(attempt.as_str())
            {
                return Err("session_reported identity mismatch".into());
            }
            transaction.execute("UPDATE agent_sessions SET status='reported',last_reported_at=?2,next_action=?3 WHERE session_id=?1",params![session_id,payload_text(transaction,event,"last_reported_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "session_waited" => {
            require_state(
                transaction,
                "agent_sessions",
                "session_id",
                session_id,
                &event.event_type,
                &["reported", "failed", "abandoned", "externally-unknown"],
            )?;
            let report = payload_optional_text(transaction, event, "consumed_report_ref")?;
            let terminal = payload_optional_text(transaction, event, "terminal_observation")?;
            if report.is_none() == terminal.is_none() {
                return Err(
                    "session_waited requires exactly one report or terminal observation".into(),
                );
            }
            transaction
                .execute(
                    "UPDATE agent_sessions SET last_waited_at=?2 WHERE session_id=?1",
                    params![
                        session_id,
                        payload_text(transaction, event, "last_waited_at")?
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        "session_failed" | "session_abandoned" | "session_externally_unknown" => {
            require_state(
                transaction,
                "agent_sessions",
                "session_id",
                session_id,
                &event.event_type,
                &["planned", "spawned", "running", "reported"],
            )?;
            let (status, outcome) = match event.event_type.as_str() {
                "session_failed" => ("failed", "failure"),
                "session_abandoned" => ("abandoned", "abandonment"),
                _ => ("externally-unknown", "unknown"),
            };
            transaction.execute("UPDATE agent_sessions SET status=?2,outcome=?3,final_reason=?4,ended_at=?5,next_action=?6 WHERE session_id=?1",params![session_id,status,outcome,payload_text(transaction,event,"final_reason")?,payload_text(transaction,event,"ended_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "session_interrupted" => {
            require_state(
                transaction,
                "agent_sessions",
                "session_id",
                session_id,
                &event.event_type,
                &["spawned", "running", "reported"],
            )?;
            transaction.execute("UPDATE agent_sessions SET interrupted_at=?2,interruption_reason=?3,outcome='unknown',next_action=?4 WHERE session_id=?1",params![session_id,payload_text(transaction,event,"interrupted_at")?,payload_text(transaction,event,"interruption_reason")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "session_close_requested" => {
            let state = current_status(transaction, "agent_sessions", "session_id", session_id)?;
            if state == "closed" {
                return Err("session already closed".into());
            }
            transaction.execute("UPDATE agent_sessions SET close_disposition='requested',close_requested_at=?2,next_action=?3 WHERE session_id=?1",params![session_id,payload_text(transaction,event,"close_requested_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "session_closed" => {
            let state = current_status(transaction, "agent_sessions", "session_id", session_id)?;
            if state == "closed" {
                return Err("session already closed".into());
            }
            transaction.execute("UPDATE agent_sessions SET status='closed',outcome=?2,close_disposition=?3,closed_at=?4,ended_at=?5,next_action=NULL WHERE session_id=?1",params![session_id,payload_text(transaction,event,"outcome")?,payload_text(transaction,event,"close_disposition")?,payload_text(transaction,event,"closed_at")?,payload_text(transaction,event,"ended_at")?]).map_err(|error|error.to_string())?;
        }
        "session_superseded" => {
            let state = current_status(transaction, "agent_sessions", "session_id", session_id)?;
            if state == "closed" {
                return Err("session already closed".into());
            }
            let replacement = payload_text(transaction, event, "superseded_by_session_id")?;
            if replacement == session_id {
                return Err("session cannot supersede itself".into());
            }
            let owner: Option<String> = transaction
                .query_row(
                    "SELECT run_id FROM agent_sessions WHERE session_id=?1",
                    [&replacement],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .flatten();
            if owner.as_deref() != Some(event.run_id.as_str()) {
                return Err("replacement session does not belong to run".into());
            }
            payload_text(transaction, event, "reason")?;
            transaction.execute("UPDATE agent_sessions SET superseded_by_session_id=?2,superseded_at=?3,next_action=?4 WHERE session_id=?1",params![session_id,replacement,payload_text(transaction,event,"superseded_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        _ => return Err(format!("event_reducer_unavailable: {}", event.event_type)),
    }
    Ok(())
}

fn current_session_blocker_event(
    transaction: &Transaction<'_>,
    event: &EventInput,
) -> Result<Option<String>, String> {
    transaction
        .query_row(
            r#"
            SELECT event_type FROM control_plane_events
            WHERE session_id=?1 AND event_type IN ('session_blocked','session_unblocked')
              AND sequence < (SELECT sequence FROM control_plane_events WHERE event_id=?2)
            ORDER BY sequence DESC LIMIT 1
            "#,
            params![event.session_id, event.event_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn reduce_lease(transaction: &Transaction<'_>, event: &EventInput) -> Result<(), String> {
    let lease_id = required_reference(&event.lease_id, "lease_id")?;
    let session_id = required_reference(&event.session_id, "session_id")?;
    let package_id = required_reference(&event.package_id, "package_id")?;
    match event.event_type.as_str() {
        "lease_planned" => {
            let write_scope = scope_envelope(transaction, event, "write_scope")?;
            let replaces = payload_optional_text(transaction, event, "replaces_session_id")?;
            let predicate = payload_optional_text(transaction, event, "expiry_predicate")?;
            if replaces.is_some() != predicate.is_some() {
                return Err(
                    "replacement lease requires both replaces_session_id and expiry_predicate"
                        .into(),
                );
            }
            if let Some(replaced) = &replaces {
                let owner: Option<String> = transaction
                    .query_row(
                        "SELECT run_id FROM agent_sessions WHERE session_id=?1",
                        [replaced],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?
                    .flatten();
                if owner.as_deref() != Some(event.run_id.as_str()) {
                    return Err("replaced session does not belong to run".into());
                }
            }
            transaction.execute(
                "INSERT INTO session_leases(lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,independence_boundary_id,replaces_session_id,expiry_predicate,status,reuse_count,next_action,expires_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'planned',0,?12,?13)",
                params![lease_id,session_id,package_id,payload_text(transaction,event,"role")?,payload_text(transaction,event,"model_profile")?,payload_text(transaction,event,"risk_class")?,write_scope,payload_text(transaction,event,"base_revision")?,payload_optional_text(transaction,event,"independence_boundary_id")?,replaces,predicate,payload_text(transaction,event,"next_action")?,payload_optional_text(transaction,event,"expires_at")?]
            ).map_err(|error|error.to_string())?;
        }
        "lease_issued" => {
            require_state(
                transaction,
                "session_leases",
                "lease_id",
                lease_id,
                &event.event_type,
                &["planned"],
            )?;
            transaction.execute("UPDATE session_leases SET status='active',issued_at=?2,next_action=?3 WHERE lease_id=?1",params![lease_id,payload_text(transaction,event,"issued_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "lease_reused" => {
            require_state(
                transaction,
                "session_leases",
                "lease_id",
                lease_id,
                &event.event_type,
                &["idle"],
            )?;
            let attempt = payload_text(transaction, event, "attempt_id")?;
            if event.attempt_id.as_deref() != Some(attempt.as_str()) {
                return Err("lease_reused attempt mismatch".into());
            }
            evidence_envelope(transaction, event, "compatibility_evidence")?;
            transaction.execute("UPDATE session_leases SET status='active',reuse_count=reuse_count+1,current_attempt_id=?2,last_used_at=?3,next_action=?4 WHERE lease_id=?1",params![lease_id,attempt,payload_text(transaction,event,"last_used_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "lease_idle" => {
            require_state(
                transaction,
                "session_leases",
                "lease_id",
                lease_id,
                &event.event_type,
                &["active"],
            )?;
            transaction.execute("UPDATE session_leases SET status='idle',last_used_at=?2,next_action=?3 WHERE lease_id=?1",params![lease_id,payload_text(transaction,event,"last_used_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "lease_expired" | "lease_revoked" => {
            require_state(
                transaction,
                "session_leases",
                "lease_id",
                lease_id,
                &event.event_type,
                &["planned", "active", "idle"],
            )?;
            let status = if event.event_type == "lease_expired" {
                "expired"
            } else {
                "revoked"
            };
            transaction.execute("UPDATE session_leases SET status=?2,expiry_reason=?3,ended_at=?4,next_action=?5 WHERE lease_id=?1",params![lease_id,status,payload_text(transaction,event,"expiry_reason")?,payload_text(transaction,event,"ended_at")?,payload_text(transaction,event,"next_action")?]).map_err(|error|error.to_string())?;
        }
        "lease_closed" => {
            require_state(
                transaction,
                "session_leases",
                "lease_id",
                lease_id,
                &event.event_type,
                &["planned", "active", "idle", "expired", "revoked"],
            )?;
            transaction.execute("UPDATE session_leases SET status='closed',expiry_reason=?2,ended_at=?3,next_action=NULL WHERE lease_id=?1",params![lease_id,payload_text(transaction,event,"expiry_reason")?,payload_text(transaction,event,"ended_at")?]).map_err(|error|error.to_string())?;
        }
        _ => return Err(format!("event_reducer_unavailable: {}", event.event_type)),
    }
    Ok(())
}

fn source_priority(source: &str) -> Result<i64, String> {
    match source {
        "host-runtime" => Ok(1),
        "harness-operation" => Ok(2),
        "controller-observation" => Ok(3),
        "agent-report" => Ok(4),
        "inference" => Ok(5),
        _ => Err(format!("unknown source kind: {source}")),
    }
}

struct ProjectionClaim<'a> {
    entity_kind: &'a str,
    entity_id: &'a str,
    field_name: &'a str,
    current_value: &'a str,
    incoming_value: &'a str,
    illegal_correction: bool,
    supersedes: Option<&'a str>,
}

fn projection_wins(
    transaction: &Transaction<'_>,
    event: &EventInput,
    sequence: i64,
    claim: ProjectionClaim<'_>,
) -> Result<bool, String> {
    let ProjectionClaim {
        entity_kind,
        entity_id,
        field_name,
        current_value,
        incoming_value,
        illegal_correction,
        supersedes,
    } = claim;
    let prior: Option<(String, String, i64, i64)> = transaction
        .query_row(
            r#"
            SELECT winner_event_id,winner_source_kind,winner_sequence,conflict_count
            FROM projection_field_sources
            WHERE entity_kind=?1 AND entity_id=?2 AND field_name=?3
            "#,
            params![entity_kind, entity_id, field_name],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((winner_event, winner_source, winner_sequence, conflict_count)) = prior else {
        transaction
            .execute(
                r#"
                INSERT INTO projection_field_sources(
                    run_id,entity_kind,entity_id,field_name,winner_event_id,
                    winner_source_kind,winner_sequence,conflict_count,last_conflict_event_id
                ) VALUES(?1,?2,?3,?4,?5,?6,?7,0,NULL)
                "#,
                params![
                    event.run_id,
                    entity_kind,
                    entity_id,
                    field_name,
                    event.event_id,
                    event.source_kind,
                    sequence
                ],
            )
            .map_err(|error| error.to_string())?;
        return Ok(true);
    };
    let incoming_priority = source_priority(&event.source_kind)?;
    let winner_priority = source_priority(&winner_source)?;
    let wins = incoming_priority < winner_priority
        || (incoming_priority == winner_priority && sequence > winner_sequence);
    if current_value == incoming_value {
        if wins {
            transaction
                .execute(
                    r#"
                    UPDATE projection_field_sources SET
                        winner_event_id=?4,winner_source_kind=?5,winner_sequence=?6,run_id=?7
                    WHERE entity_kind=?1 AND entity_id=?2 AND field_name=?3
                    "#,
                    params![
                        entity_kind,
                        entity_id,
                        field_name,
                        event.event_id,
                        event.source_kind,
                        sequence,
                        event.run_id
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        return Ok(false);
    }
    if wins && illegal_correction && supersedes != Some(winner_event.as_str()) {
        return Err(format!(
            "correction for {entity_kind}/{entity_id}/{field_name} must supersede {winner_event}"
        ));
    }
    if wins {
        transaction
            .execute(
                r#"
                UPDATE projection_field_sources SET
                    winner_event_id=?4,winner_source_kind=?5,winner_sequence=?6,
                    conflict_count=?7,last_conflict_event_id=?4,run_id=?8
                WHERE entity_kind=?1 AND entity_id=?2 AND field_name=?3
                "#,
                params![
                    entity_kind,
                    entity_id,
                    field_name,
                    event.event_id,
                    event.source_kind,
                    sequence,
                    conflict_count + 1,
                    event.run_id
                ],
            )
            .map_err(|error| error.to_string())?;
    } else {
        transaction
            .execute(
                r#"
                UPDATE projection_field_sources SET
                    conflict_count=?4,last_conflict_event_id=?5
                WHERE entity_kind=?1 AND entity_id=?2 AND field_name=?3
                "#,
                params![
                    entity_kind,
                    entity_id,
                    field_name,
                    conflict_count + 1,
                    event.event_id
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(wins)
}

fn nullable_nonnegative_integer(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
) -> Result<Option<i64>, String> {
    let path = format!("$.{key}");
    let value_type: String = transaction
        .query_row(
            "SELECT json_type(?1,?2)",
            params![event.payload_json, path],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    match value_type.as_str() {
        "null" => Ok(None),
        "integer" => {
            let value: i64 = transaction
                .query_row(
                    "SELECT json_extract(?1,?2)",
                    params![event.payload_json, path],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if value < 0 {
                Err(format!("usage {key} must be nonnegative"))
            } else {
                Ok(Some(value))
            }
        }
        other => Err(format!("usage {key} must be integer or null, got {other}")),
    }
}

struct UsageProjection<'a> {
    table: &'a str,
    id_column: &'a str,
    entity_kind: &'a str,
    entity_id: &'a str,
    field_name: &'a str,
}

fn apply_usage_integer(
    transaction: &Transaction<'_>,
    event: &EventInput,
    sequence: i64,
    projection: UsageProjection<'_>,
    incoming: Option<i64>,
    correction: bool,
    supersedes: Option<&str>,
) -> Result<(), String> {
    let UsageProjection {
        table,
        id_column,
        entity_kind,
        entity_id,
        field_name,
    } = projection;
    if incoming.is_none() && !correction {
        return Ok(());
    }
    let current: Option<i64> = transaction
        .query_row(
            &format!("SELECT {field_name} FROM {table} WHERE {id_column}=?1"),
            [entity_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let incoming_repr = incoming.map_or_else(|| "null".to_string(), |value| value.to_string());
    let current_repr = current.map_or_else(|| "null".to_string(), |value| value.to_string());
    let illegal = correction && (incoming.is_none() || incoming < current);
    if projection_wins(
        transaction,
        event,
        sequence,
        ProjectionClaim {
            entity_kind,
            entity_id,
            field_name,
            current_value: &current_repr,
            incoming_value: &incoming_repr,
            illegal_correction: illegal,
            supersedes,
        },
    )? {
        transaction
            .execute(
                &format!(
                    "UPDATE {table} SET {field_name}=?2,telemetry_source=?3 WHERE {id_column}=?1"
                ),
                params![entity_id, incoming, event.source_kind],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[derive(Clone)]
struct ExactTriplet {
    amount: i64,
    scale: i64,
    unit: String,
    canonical: String,
}

fn exact_triplet(
    transaction: &Transaction<'_>,
    event: &EventInput,
    key: &str,
    unit_key: &str,
) -> Result<Option<ExactTriplet>, String> {
    let path = format!("$.{key}");
    let value_type: String = transaction
        .query_row(
            "SELECT json_type(?1,?2)",
            params![event.payload_json, path],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if value_type == "null" {
        return Ok(None);
    }
    if value_type != "object" {
        return Err(format!("{key} must be an exact object or null"));
    }
    let canonical: String = transaction
        .query_row(
            "SELECT json_extract(?1,?2)",
            params![event.payload_json, path],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if object_keys(transaction, &canonical)? != ["amount", "scale", unit_key] {
        return Err(format!("invalid {key} exact-value envelope"));
    }
    let (amount_type, scale_type): (String, String) = transaction
        .query_row(
            "SELECT json_type(?1,'$.amount'),json_type(?1,'$.scale')",
            [&canonical],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    if amount_type != "integer" || scale_type != "integer" {
        return Err(format!("{key} amount and scale must be integers"));
    }
    let (amount, scale, unit): (i64, i64, String) = transaction
        .query_row(
            &format!(
                "SELECT json_extract(?1,'$.amount'),json_extract(?1,'$.scale'),json_extract(?1,'$.{unit_key}')"
            ),
            [&canonical],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| error.to_string())?;
    if amount < 0 || !(0..=12).contains(&scale) || unit.is_empty() {
        return Err(format!("invalid {key} exact value"));
    }
    if unit_key == "currency"
        && (unit.len() != 3 || !unit.bytes().all(|byte| byte.is_ascii_uppercase()))
    {
        return Err("provider_cost currency must be uppercase ISO-4217 text".into());
    }
    Ok(Some(ExactTriplet {
        amount,
        scale,
        unit,
        canonical,
    }))
}

struct TripletProjection<'a> {
    table: &'a str,
    id_column: &'a str,
    entity_kind: &'a str,
    entity_id: &'a str,
    field_name: &'a str,
    amount_column: &'a str,
    scale_column: &'a str,
    unit_column: &'a str,
}

fn apply_usage_triplet(
    transaction: &Transaction<'_>,
    event: &EventInput,
    sequence: i64,
    projection: TripletProjection<'_>,
    incoming: Option<ExactTriplet>,
    correction: bool,
    supersedes: Option<&str>,
) -> Result<(), String> {
    if incoming.is_none() && !correction {
        return Ok(());
    }
    let unit_key = if projection.field_name == "provider_cost" {
        "currency"
    } else {
        "unit"
    };
    let current: Option<String> = transaction
        .query_row(
            &format!(
                "SELECT json_object('amount',{0},'scale',{1},'{5}',{2}) FROM {3} WHERE {4}=?1 AND {0} IS NOT NULL",
                projection.amount_column,
                projection.scale_column,
                projection.unit_column,
                projection.table,
                projection.id_column,
                unit_key,
            ),
            [projection.entity_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let current_repr = current.unwrap_or_else(|| "null".to_string());
    let incoming_repr = incoming
        .as_ref()
        .map_or_else(|| "null".to_string(), |value| value.canonical.clone());
    if projection_wins(
        transaction,
        event,
        sequence,
        ProjectionClaim {
            entity_kind: projection.entity_kind,
            entity_id: projection.entity_id,
            field_name: projection.field_name,
            current_value: &current_repr,
            incoming_value: &incoming_repr,
            illegal_correction: correction && incoming.is_none(),
            supersedes,
        },
    )? {
        let (amount, scale, unit) = incoming
            .map(|value| (Some(value.amount), Some(value.scale), Some(value.unit)))
            .unwrap_or((None, None, None));
        transaction
            .execute(
                &format!(
                    "UPDATE {0} SET {1}=?2,{2}=?3,{3}=?4,telemetry_source=?5 WHERE {4}=?1",
                    projection.table,
                    projection.amount_column,
                    projection.scale_column,
                    projection.unit_column,
                    projection.id_column,
                ),
                params![projection.entity_id, amount, scale, unit, event.source_kind],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn reduce_usage(
    transaction: &Transaction<'_>,
    event: &EventInput,
    sequence: i64,
) -> Result<(), String> {
    let scope = payload_text(transaction, event, "scope")?;
    let subject = payload_text(transaction, event, "subject_id")?;
    let observation = payload_text(transaction, event, "observation_kind")?;
    let (table, id_column, entity_kind) = match scope.as_str() {
        "attempt" => {
            if event.attempt_id.as_deref() != Some(subject.as_str()) {
                return Err("attempt usage subject_id does not match event attempt_id".into());
            }
            if !["delta", "correction"].contains(&observation.as_str()) {
                return Err("attempt usage accepts only delta or correction".into());
            }
            ("assignment_attempts", "attempt_id", "attempt")
        }
        "session" => {
            if event.session_id.as_deref() != Some(subject.as_str()) {
                return Err("session usage subject_id does not match event session_id".into());
            }
            if !["cumulative", "correction"].contains(&observation.as_str()) {
                return Err("session usage accepts only cumulative or correction".into());
            }
            ("agent_sessions", "session_id", "session")
        }
        _ => return Err("usage scope must be attempt or session".into()),
    };
    let correction = observation == "correction";
    let supersedes = payload_optional_text(transaction, event, "supersedes_event_id")?;
    if correction && supersedes.is_none() {
        return Err("usage correction requires supersedes_event_id".into());
    }
    if let Some(superseded) = &supersedes {
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM control_plane_events WHERE event_id=?1 AND run_id=?2)",
                params![superseded, event.run_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if !exists {
            return Err(format!("unknown superseded event: {superseded}"));
        }
    }
    let telemetry_quality = payload_text(transaction, event, "telemetry_quality")?;
    if !["exact", "partial", "estimated", "unsupported", "unknown"]
        .contains(&telemetry_quality.as_str())
    {
        return Err("invalid telemetry_quality".into());
    }
    let token_fields = [
        "input_tokens",
        "output_tokens",
        "reasoning_tokens",
        "cache_read_tokens",
        "cache_write_tokens",
    ];
    for field in token_fields {
        let value = nullable_nonnegative_integer(transaction, event, field)?;
        if scope == "session" && !correction {
            let current: Option<i64> = transaction
                .query_row(
                    &format!("SELECT {field} FROM {table} WHERE {id_column}=?1"),
                    [&subject],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if matches!((current, value), (Some(old), Some(new)) if new < old) {
                return Err(format!(
                    "session cumulative {field} cannot decrease without correction"
                ));
            }
        }
        apply_usage_integer(
            transaction,
            event,
            sequence,
            UsageProjection {
                table,
                id_column,
                entity_kind,
                entity_id: &subject,
                field_name: field,
            },
            value,
            correction,
            supersedes.as_deref(),
        )?;
    }
    let credits = exact_triplet(transaction, event, "credits", "unit")?;
    apply_usage_triplet(
        transaction,
        event,
        sequence,
        TripletProjection {
            table,
            id_column,
            entity_kind,
            entity_id: &subject,
            field_name: "credits",
            amount_column: "credits_amount",
            scale_column: "credits_scale",
            unit_column: "credits_unit",
        },
        credits,
        correction,
        supersedes.as_deref(),
    )?;
    let cost = exact_triplet(transaction, event, "provider_cost", "currency")?;
    apply_usage_triplet(
        transaction,
        event,
        sequence,
        TripletProjection {
            table,
            id_column,
            entity_kind,
            entity_id: &subject,
            field_name: "provider_cost",
            amount_column: "cost_amount",
            scale_column: "cost_scale",
            unit_column: "cost_currency",
        },
        cost,
        correction,
        supersedes.as_deref(),
    )?;
    if entity_kind == "session" {
        transaction
            .execute(
                "UPDATE agent_sessions SET telemetry_quality=?2 WHERE session_id=?1",
                params![subject, telemetry_quality],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn reduce_quality_gate(
    transaction: &Transaction<'_>,
    event: &EventInput,
    sequence: i64,
) -> Result<(), String> {
    let subject_kind = payload_text(transaction, event, "subject_kind")?;
    let subject_id = payload_text(transaction, event, "subject_id")?;
    let policy_json = policy_envelope(
        transaction,
        event,
        "policy",
        &["none", "deterministic", "independent"],
    )?;
    let policy: String = transaction
        .query_row("SELECT json_extract(?1,'$.kind')", [&policy_json], |row| {
            row.get(0)
        })
        .map_err(|error| error.to_string())?;
    let reference_matches = match subject_kind.as_str() {
        "run" => subject_id == event.run_id,
        "package" => event.package_id.as_deref() == Some(subject_id.as_str()),
        "assignment" => event.assignment_id.as_deref() == Some(subject_id.as_str()),
        "attempt" => event.attempt_id.as_deref() == Some(subject_id.as_str()),
        "session" => event.session_id.as_deref() == Some(subject_id.as_str()),
        "lease" => event.lease_id.as_deref() == Some(subject_id.as_str()),
        "route" => {
            event
                .attempt_id
                .as_deref()
                .and_then(|attempt_id| current_route_id(transaction, attempt_id).ok())
                .as_deref()
                == Some(subject_id.as_str())
        }
        _ => false,
    };
    if !reference_matches {
        return Err("quality gate subject does not match event identity".into());
    }
    if event.event_type == "quality_gate_passed" {
        let evidence = evidence_envelope(transaction, event, "evidence")?;
        let (table, id_column, field_name) = match subject_kind.as_str() {
            "assignment" => {
                let column = if policy == "independent" {
                    "review_evidence"
                } else {
                    "test_evidence"
                };
                ("assignments", "assignment_id", column.to_string())
            }
            "run" => ("runs", "run_id", format!("quality_gate:{policy}")),
            "package" => (
                "work_packages",
                "package_id",
                format!("quality_gate:{policy}"),
            ),
            "attempt" => (
                "assignment_attempts",
                "attempt_id",
                format!("quality_gate:{policy}"),
            ),
            "session" => (
                "agent_sessions",
                "session_id",
                format!("quality_gate:{policy}"),
            ),
            "lease" => (
                "session_leases",
                "lease_id",
                format!("quality_gate:{policy}"),
            ),
            "route" => (
                "routing_decisions",
                "route_id",
                format!("quality_gate:{policy}"),
            ),
            _ => return Err("invalid quality gate subject_kind".into()),
        };
        let exists: bool = transaction
            .query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {id_column}=?1)"),
                [&subject_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if !exists {
            return Err(format!("unknown quality gate subject: {subject_id}"));
        }
        let current = if subject_kind == "assignment" {
            transaction
                .query_row(
                    &format!("SELECT {field_name} FROM {table} WHERE {id_column}=?1"),
                    [&subject_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .map_err(|error| error.to_string())?
        } else {
            transaction
                .query_row(
                    r#"
                    SELECT json_extract(events.payload_json,'$.evidence')
                    FROM projection_field_sources AS sources
                    JOIN control_plane_events AS events
                      ON events.event_id=sources.winner_event_id
                    WHERE sources.entity_kind=?1 AND sources.entity_id=?2
                      AND sources.field_name=?3
                    "#,
                    params![subject_kind, subject_id, field_name],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|error| error.to_string())?
        };
        let current = current.unwrap_or_else(|| "null".to_string());
        if projection_wins(
            transaction,
            event,
            sequence,
            ProjectionClaim {
                entity_kind: &subject_kind,
                entity_id: &subject_id,
                field_name: &field_name,
                current_value: &current,
                incoming_value: &evidence,
                illegal_correction: false,
                supersedes: None,
            },
        )? && subject_kind == "assignment"
        {
            let changed = transaction
                .execute(
                    &format!("UPDATE assignments SET {field_name}=?2 WHERE assignment_id=?1"),
                    params![subject_id, evidence],
                )
                .map_err(|error| error.to_string())?;
            if changed != 1 {
                return Err(format!("unknown quality gate subject: {subject_id}"));
            }
        }
    } else {
        payload_text(transaction, event, "findings_ref")?;
        let next_action = payload_text(transaction, event, "next_action")?;
        let (table, id_column) = match subject_kind.as_str() {
            "run" => ("runs", "run_id"),
            "package" => ("work_packages", "package_id"),
            "assignment" => ("assignments", "assignment_id"),
            "attempt" => ("assignment_attempts", "attempt_id"),
            "session" => ("agent_sessions", "session_id"),
            "lease" => ("session_leases", "lease_id"),
            "route" => ("routing_decisions", "route_id"),
            _ => return Err("invalid quality gate subject_kind".into()),
        };
        let current: Option<String> = transaction
            .query_row(
                &format!("SELECT next_action FROM {table} WHERE {id_column}=?1"),
                [&subject_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let current = current.unwrap_or_else(|| "null".to_string());
        if projection_wins(
            transaction,
            event,
            sequence,
            ProjectionClaim {
                entity_kind: &subject_kind,
                entity_id: &subject_id,
                field_name: "next_action",
                current_value: &current,
                incoming_value: &next_action,
                illegal_correction: false,
                supersedes: None,
            },
        )? {
            let changed = transaction
                .execute(
                    &format!("UPDATE {table} SET next_action=?2 WHERE {id_column}=?1"),
                    params![subject_id, next_action],
                )
                .map_err(|error| error.to_string())?;
            if changed != 1 {
                return Err(format!("unknown quality gate subject: {subject_id}"));
            }
        }
    }
    Ok(())
}

struct ReplayStoredEvent {
    input: EventInput,
    sequence: i64,
    ingested_at: String,
}

pub(crate) fn replay_run_into_empty(
    source: &Connection,
    target: &mut Connection,
    run_id: &str,
) -> Result<(), String> {
    if run_id.is_empty() {
        return Err("replay run_id must be nonempty".into());
    }
    validate_current_connection(target).map_err(|error| error.to_string())?;
    let occupied: i64 = target
        .query_row(
            r#"
            SELECT
              (SELECT COUNT(*) FROM agent_ledger) +
              (SELECT COUNT(*) FROM runs) +
              (SELECT COUNT(*) FROM work_packages) +
              (SELECT COUNT(*) FROM assignments) +
              (SELECT COUNT(*) FROM assignment_attempts) +
              (SELECT COUNT(*) FROM agent_sessions) +
              (SELECT COUNT(*) FROM session_leases) +
              (SELECT COUNT(*) FROM routing_decisions) +
              (SELECT COUNT(*) FROM control_plane_events) +
              (SELECT COUNT(*) FROM run_event_counters) +
              (SELECT COUNT(*) FROM legacy_agent_ledger_import) +
              (SELECT COUNT(*) FROM projection_field_sources)
            "#,
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if occupied != 0 {
        return Err("replay target must be empty".into());
    }

    let events = {
        let mut statement = source
            .prepare(
                r#"
                SELECT event_id,run_id,package_id,assignment_id,attempt_id,session_id,
                       lease_id,event_type,source_kind,source_id,confidence,payload_json,
                       occurred_at,idempotency_key,sequence,ingested_at
                FROM control_plane_events WHERE run_id=?1 ORDER BY sequence
                "#,
            )
            .map_err(|error| error.to_string())?;
        statement
            .query_map([run_id], |row| {
                Ok(ReplayStoredEvent {
                    input: EventInput {
                        event_id: row.get(0)?,
                        run_id: row.get(1)?,
                        package_id: row.get(2)?,
                        assignment_id: row.get(3)?,
                        attempt_id: row.get(4)?,
                        session_id: row.get(5)?,
                        lease_id: row.get(6)?,
                        event_type: row.get(7)?,
                        source_kind: row.get(8)?,
                        source_id: row.get(9)?,
                        confidence: row.get(10)?,
                        payload_json: row.get(11)?,
                        occurred_at: row.get(12)?,
                        idempotency_key: row.get(13)?,
                    },
                    sequence: row.get(14)?,
                    ingested_at: row.get(15)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    if events.is_empty() {
        return Err(format!("replay source has no events for run {run_id}"));
    }
    for (index, event) in events.iter().enumerate() {
        let expected = index as i64 + 1;
        if event.sequence != expected {
            return Err(format!(
                "replay sequence gap: expected {expected}, got {}",
                event.sequence
            ));
        }
    }
    if events[0].input.event_type != "run_planned" {
        return Err("replay sequence one must be run_planned".into());
    }

    let transaction = target
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    for stored in &events {
        validate_common(&transaction, &stored.input)?;
        validate_identity_chain(&transaction, &stored.input)?;
        if stored.sequence > 1 {
            let next: i64 = transaction
                .query_row(
                    "SELECT next_sequence FROM run_event_counters WHERE run_id=?1",
                    [run_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if next != stored.sequence {
                return Err(format!(
                    "replay counter mismatch: expected {}, got {next}",
                    stored.sequence
                ));
            }
            transaction
                .execute(
                    "UPDATE run_event_counters SET next_sequence=?2 WHERE run_id=?1",
                    params![run_id, stored.sequence + 1],
                )
                .map_err(|error| error.to_string())?;
        }
        let event = &stored.input;
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
                    stored.sequence,
                    event.event_type,
                    event.source_kind,
                    event.source_id,
                    event.confidence,
                    event.payload_json,
                    event.occurred_at,
                    stored.ingested_at,
                    event.idempotency_key
                ],
            )
            .map_err(|error| error.to_string())?;
        reduce_event(&transaction, event, stored.sequence)?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{AppendDisposition, EventInput, append_event, event_names, replay_run_into_empty};
    use crate::schema::{initialize_connection, open_db};
    use rusqlite::Connection;
    use std::collections::BTreeSet;
    use std::fs;
    use std::sync::{Arc, Barrier};
    use std::time::{SystemTime, UNIX_EPOCH};

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
    type EventMutation = Box<dyn Fn(&mut EventInput)>;

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

    #[derive(Default)]
    struct Refs<'a> {
        package: Option<&'a str>,
        assignment: Option<&'a str>,
        attempt: Option<&'a str>,
        session: Option<&'a str>,
        lease: Option<&'a str>,
    }

    fn append_named(
        conn: &mut Connection,
        run_id: &str,
        event_id: &str,
        event_type: &str,
        refs: Refs<'_>,
        payload: &str,
    ) {
        append_event(
            conn,
            &EventInput {
                event_id: event_id.to_string(),
                run_id: run_id.to_string(),
                package_id: refs.package.map(str::to_string),
                assignment_id: refs.assignment.map(str::to_string),
                attempt_id: refs.attempt.map(str::to_string),
                session_id: refs.session.map(str::to_string),
                lease_id: refs.lease.map(str::to_string),
                event_type: event_type.to_string(),
                source_kind: "controller-observation".to_string(),
                source_id: "controller-1".to_string(),
                confidence: Some(10_000),
                payload_json: payload.to_string(),
                occurred_at: "2026-07-12T09:08:07.006Z".to_string(),
                idempotency_key: event_id.to_string(),
            },
        )
        .unwrap_or_else(|error| panic!("{event_id}/{event_type}: {error}"));
    }

    fn setup_planned_attempt(conn: &mut Connection) {
        append_run(conn, "e01", "run-1", "e01");
        append_named(
            conn,
            "run-1",
            "e02",
            "run_started",
            Refs::default(),
            "{\"v\":1,\"psoc_revision\":\"psoc-2\",\"started_at\":\"2026-07-12T09:08:08.006Z\",\"next_action\":\"plan package\"}",
        );
        append_named(
            conn,
            "run-1",
            "e03",
            "package_planned",
            Refs {
                package: Some("package-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"title\":\"package\",\"dependencies\":[],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[\"src\"]},\"review_policy\":{\"v\":1,\"kind\":\"none\"},\"independence_policy\":{\"v\":1,\"kind\":\"none\"},\"next_action\":\"queue\"}",
        );
        append_named(
            conn,
            "run-1",
            "e04",
            "package_ready",
            Refs {
                package: Some("package-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"next_action\":\"activate\"}",
        );
        append_named(
            conn,
            "run-1",
            "e05",
            "package_active",
            Refs {
                package: Some("package-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"next_action\":\"plan assignment\"}",
        );
        append_named(
            conn,
            "run-1",
            "e06",
            "assignment_planned",
            Refs {
                package: Some("package-1"),
                assignment: Some("assignment-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"title\":\"implement\",\"sequence\":1,\"assignment_kind\":\"implementation\",\"required_role\":\"worker\",\"model_floor\":\"standard\",\"risk_class\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[\"src\"]},\"base_revision\":\"base\",\"independence_boundary_id\":null,\"next_action\":\"queue\"}",
        );
        append_named(
            conn,
            "run-1",
            "e07",
            "assignment_queued",
            Refs {
                package: Some("package-1"),
                assignment: Some("assignment-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"next_action\":\"plan attempt\"}",
        );
        append_named(
            conn,
            "run-1",
            "e08",
            "attempt_planned",
            Refs {
                package: Some("package-1"),
                assignment: Some("assignment-1"),
                attempt: Some("attempt-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"attempt_sequence\":1,\"next_action\":\"route\"}",
        );
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

    fn transition_contract(name: &str) -> (&'static [&'static str], &'static str) {
        match name {
            "run_planned" | "package_planned" | "assignment_planned" => (&["absent"], "planned"),
            "attempt_planned" => (&["assignment-planned", "assignment-queued"], "planned"),
            "route_requested" | "session_planned" => (&["attempt-planned"], "planned"),
            "lease_planned" => (&["session-owned"], "planned"),
            "run_started" => (&["planned"], "active"),
            "run_blocked" => (&["active"], "blocked"),
            "run_unblocked" => (&["blocked"], "active"),
            "run_completed" => (&["active"], "complete"),
            "run_cancelled" => (&["planned", "active", "blocked"], "cancelled"),
            "package_ready" => (&["planned"], "ready"),
            "package_active" => (&["ready"], "active"),
            "package_blocked" => (&["ready", "active", "review"], "blocked"),
            "package_unblocked" => (&["blocked"], "ready-or-active-or-review"),
            "package_review_started" => (&["active"], "review"),
            "package_review_completed" => (&["review"], "review-or-active"),
            "package_completed" => (&["active", "review"], "complete"),
            "package_cancelled" => (
                &["planned", "ready", "active", "review", "blocked"],
                "cancelled",
            ),
            "assignment_queued" => (&["planned"], "queued"),
            "assignment_started" => (&["queued"], "running"),
            "assignment_step_changed" => (&["running"], "facet"),
            "assignment_blocked" | "assignment_unblocked" => {
                (&["queued", "running", "reported", "validated"], "facet")
            }
            "assignment_reported" => (&["running"], "reported"),
            "assignment_validated" => (&["reported"], "validated"),
            "assignment_accepted" => (&["validated"], "accepted"),
            "assignment_requeued" => (&["queued", "running", "reported", "validated"], "queued"),
            "assignment_failed" => (&["queued", "running", "reported", "validated"], "failed"),
            "assignment_cancelled" => (
                &["planned", "queued", "running", "reported", "validated"],
                "cancelled",
            ),
            "attempt_started" => (&["planned"], "running"),
            "attempt_reported" => (&["running"], "reported"),
            "attempt_validated" => (&["reported"], "validated"),
            "attempt_accepted" => (&["validated"], "accepted"),
            "attempt_failed" | "attempt_cancelled" => {
                (&["planned", "running", "reported", "validated"], "terminal")
            }
            "dispatch_main_selected"
            | "dispatch_reuse_selected"
            | "dispatch_batch_selected"
            | "dispatch_spawn_selected" => (&["planned"], "facet"),
            "route_applied" => (&["requested", "degraded"], "applied-or-inherited"),
            "route_degraded" => (&["requested"], "degraded"),
            "route_rejected" => (&["requested", "degraded"], "rejected"),
            "session_spawned" => (&["planned"], "spawned"),
            "session_running" => (&["spawned", "reported"], "running"),
            "session_heartbeat"
            | "session_blocked"
            | "session_unblocked"
            | "session_interrupted" => (&["spawned", "running", "reported"], "facet"),
            "session_reported" => (&["running"], "reported"),
            "session_waited" => (
                &["reported", "failed", "abandoned", "externally-unknown"],
                "facet",
            ),
            "session_failed" | "session_abandoned" | "session_externally_unknown" => (
                &["planned", "spawned", "running", "reported"],
                "exception-terminal",
            ),
            "session_close_requested" | "session_superseded" => (
                &[
                    "planned",
                    "spawned",
                    "running",
                    "reported",
                    "failed",
                    "abandoned",
                    "externally-unknown",
                ],
                "facet",
            ),
            "session_closed" => (
                &[
                    "planned",
                    "spawned",
                    "running",
                    "reported",
                    "failed",
                    "abandoned",
                    "externally-unknown",
                ],
                "closed",
            ),
            "lease_issued" => (&["planned"], "active"),
            "lease_reused" => (&["idle"], "active"),
            "lease_idle" => (&["active"], "idle"),
            "lease_expired" | "lease_revoked" => (&["planned", "active", "idle"], "terminal"),
            "lease_closed" => (
                &["planned", "active", "idle", "expired", "revoked"],
                "closed",
            ),
            "usage_observed" | "quality_gate_passed" | "quality_gate_failed" => {
                (&["owned-subject"], "facet")
            }
            _ => panic!("missing transition contract for {name}"),
        }
    }

    fn event_count_and_counter(conn: &Connection, run_id: &str) -> (i64, i64) {
        conn.query_row(
            "SELECT (SELECT COUNT(*) FROM control_plane_events WHERE run_id=?1),COALESCE((SELECT next_sequence FROM run_event_counters WHERE run_id=?1),1)",
            [run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
    }

    fn illegal_source_for(name: &str) -> &'static str {
        match super::event_rule(name).unwrap().source_floor {
            super::SourceFloor::ControllerObservation => "agent-report",
            super::SourceFloor::AgentReport => "inference",
            super::SourceFloor::Inference => "rumor",
        }
    }

    fn transition_payload(name: &str, source_state: &str) -> String {
        let timestamp = "2026-07-12T09:30:00.000Z";
        match name {
            "run_planned" => run_payload("matrix"),
            "run_started" => format!(
                "{{\"v\":1,\"psoc_revision\":\"psoc-2\",\"started_at\":\"{timestamp}\",\"next_action\":\"work\"}}"
            ),
            "run_blocked" => "{\"v\":1,\"blocker\":\"blocked\",\"next_action\":\"resolve\"}".into(),
            "run_unblocked" => "{\"v\":1,\"resolution\":\"resolved\",\"next_action\":\"work\"}".into(),
            "run_completed" => format!("{{\"v\":1,\"completed_at\":\"{timestamp}\",\"ended_at\":\"{timestamp}\"}}"),
            "run_cancelled" => format!("{{\"v\":1,\"reason\":\"cancel\",\"ended_at\":\"{timestamp}\",\"next_action\":\"audit\"}}"),
            "package_planned" => "{\"v\":1,\"title\":\"matrix\",\"dependencies\":[],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"review_policy\":{\"v\":1,\"kind\":\"none\"},\"independence_policy\":{\"v\":1,\"kind\":\"none\"},\"next_action\":\"ready\"}".into(),
            "package_ready" | "package_active" => "{\"v\":1,\"next_action\":\"continue\"}".into(),
            "package_blocked" => format!("{{\"v\":1,\"blocker\":\"blocked\",\"resume_status\":\"{source_state}\",\"next_action\":\"resolve\"}}"),
            "package_unblocked" => "{\"v\":1,\"resolution\":\"resolved\",\"resume_status\":\"active\",\"next_action\":\"continue\"}".into(),
            "package_review_started" => "{\"v\":1,\"review_assignment_id\":\"matrix-review\",\"next_action\":\"review\"}".into(),
            "package_review_completed" => "{\"v\":1,\"verdict\":\"accepted\",\"review_evidence\":{\"v\":1,\"items\":[{\"kind\":\"review\",\"ref\":\"/tmp/review.md\",\"result\":\"accepted\"}]},\"next_action\":\"complete\"}".into(),
            "package_completed" => format!("{{\"v\":1,\"ended_at\":\"{timestamp}\"}}"),
            "package_cancelled" => format!("{{\"v\":1,\"reason\":\"cancel\",\"ended_at\":\"{timestamp}\",\"next_action\":\"audit\"}}"),
            "assignment_planned" => "{\"v\":1,\"title\":\"matrix\",\"sequence\":3,\"assignment_kind\":\"implementation\",\"required_role\":\"worker\",\"model_floor\":\"standard\",\"risk_class\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"base_revision\":\"base\",\"independence_boundary_id\":null,\"next_action\":\"queue\"}".into(),
            "assignment_queued" => "{\"v\":1,\"next_action\":\"run\"}".into(),
            "assignment_started" => format!("{{\"v\":1,\"attempt_id\":\"matrix-attempt\",\"started_at\":\"{timestamp}\",\"current_step\":\"edit\"}}"),
            "assignment_step_changed" => "{\"v\":1,\"current_step\":\"test\"}".into(),
            "assignment_blocked" => "{\"v\":1,\"blocker\":\"blocked\",\"next_action\":\"resolve\"}".into(),
            "assignment_unblocked" => "{\"v\":1,\"resolution\":\"resolved\",\"next_action\":\"continue\"}".into(),
            "assignment_reported" => format!("{{\"v\":1,\"report_path\":\"/tmp/matrix.md\",\"reported_at\":\"{timestamp}\",\"next_action\":\"validate\"}}"),
            "assignment_validated" => format!("{{\"v\":1,\"test_evidence\":{{\"v\":1,\"items\":[{{\"kind\":\"test\",\"ref\":\"suite\",\"result\":\"passed\"}}]}},\"validated_at\":\"{timestamp}\",\"next_action\":\"accept\"}}"),
            "assignment_accepted" => format!("{{\"v\":1,\"review_evidence\":{{\"v\":1,\"items\":[]}},\"accepted_at\":\"{timestamp}\",\"ended_at\":\"{timestamp}\"}}"),
            "assignment_requeued" => "{\"v\":1,\"reason\":\"retry\",\"next_action\":\"queue\"}".into(),
            "assignment_failed" => format!("{{\"v\":1,\"policy_exhausted\":true,\"final_reason\":\"failed\",\"ended_at\":\"{timestamp}\",\"next_action\":\"audit\"}}"),
            "assignment_cancelled" => format!("{{\"v\":1,\"final_reason\":\"cancelled\",\"ended_at\":\"{timestamp}\",\"next_action\":\"audit\"}}"),
            "attempt_planned" => "{\"v\":1,\"attempt_sequence\":2,\"next_action\":\"route\"}".into(),
            "attempt_started" => format!("{{\"v\":1,\"started_at\":\"{timestamp}\",\"next_action\":\"work\"}}"),
            "attempt_reported" => format!("{{\"v\":1,\"reported_at\":\"{timestamp}\",\"next_action\":\"validate\"}}"),
            "attempt_validated" => format!("{{\"v\":1,\"validated_at\":\"{timestamp}\",\"next_action\":\"accept\"}}"),
            "attempt_accepted" => format!("{{\"v\":1,\"accepted_at\":\"{timestamp}\",\"ended_at\":\"{timestamp}\"}}"),
            "attempt_failed" | "attempt_cancelled" => format!("{{\"v\":1,\"outcome_reason\":\"terminal\",\"ended_at\":\"{timestamp}\",\"next_action\":\"audit\"}}"),
            "dispatch_main_selected" | "dispatch_batch_selected" => "{\"v\":1,\"reason\":\"selected\",\"authorization_ref\":\"auth\"}".into(),
            "dispatch_reuse_selected" => "{\"v\":1,\"session_id\":\"matrix-session\",\"lease_id\":\"matrix-lease\",\"reason\":\"compatible\",\"authorization_ref\":\"auth\"}".into(),
            "dispatch_spawn_selected" => "{\"v\":1,\"session_id\":\"matrix-session\",\"reason\":\"isolate\",\"authorization_ref\":\"auth\"}".into(),
            "route_requested" => format!("{{\"v\":1,\"route_id\":\"matrix-new-route\",\"required_profile\":\"standard\",\"requested_model\":null,\"requested_reasoning\":null,\"escalated_from_route_id\":null,\"decided_at\":\"{timestamp}\",\"next_action\":\"apply\"}}"),
            "route_applied" => format!("{{\"v\":1,\"routing_status\":\"applied\",\"actual_model\":\"model\",\"actual_reasoning\":null,\"eligibility_evidence\":{{\"v\":1,\"items\":[{{\"kind\":\"probe\",\"ref\":\"matrix-route\",\"result\":\"eligible\"}}]}},\"decided_at\":\"{timestamp}\"}}"),
            "route_degraded" => format!("{{\"v\":1,\"actual_model\":null,\"actual_reasoning\":null,\"reason\":\"unsupported\",\"eligibility_status\":\"unknown\",\"eligibility_evidence\":{{\"v\":1,\"items\":[]}},\"next_action\":\"fallback\",\"decided_at\":\"{timestamp}\"}}"),
            "route_rejected" => format!("{{\"v\":1,\"reason\":\"ineligible\",\"eligibility_evidence\":{{\"v\":1,\"items\":[]}},\"next_action\":\"reroute\",\"decided_at\":\"{timestamp}\"}}"),
            "session_planned" => "{\"v\":1,\"authorization_ref\":\"auth\",\"budget_reason\":\"matrix\",\"run_max_open\":2,\"run_max_total\":4,\"session_token_budget\":{\"mode\":\"unbounded\",\"tokens\":null},\"nested_delegation\":{\"allowed\":false,\"authority_ref\":null},\"requested_host\":null,\"requested_profile\":\"standard\",\"requested_model\":null,\"requested_reasoning\":null,\"parent_session_id\":null}".into(),
            "session_spawned" => format!("{{\"v\":1,\"handle\":\"matrix-handle\",\"host_id\":\"host\",\"spawned_at\":\"{timestamp}\",\"next_action\":\"run\"}}"),
            "session_running" => {
                let (report, evidence) = if source_state == "reported" {
                    ("\"/tmp/matrix.md\"", "{\"v\":1,\"items\":[]}")
                } else {
                    ("null", "null")
                };
                format!("{{\"v\":1,\"started_or_resumed_at\":\"{timestamp}\",\"lease_id\":\"matrix-lease\",\"attempt_id\":\"matrix-attempt\",\"prior_report_ref\":{report},\"gate_evidence\":{evidence},\"next_action\":\"work\"}}")
            }
            "session_heartbeat" => format!("{{\"v\":1,\"last_activity_at\":\"{timestamp}\"}}"),
            "session_blocked" => "{\"v\":1,\"blocker\":\"blocked\",\"next_action\":\"resolve\"}".into(),
            "session_unblocked" => "{\"v\":1,\"resolution\":\"resolved\",\"next_action\":\"continue\"}".into(),
            "session_reported" => format!("{{\"v\":1,\"last_reported_at\":\"{timestamp}\",\"assignment_id\":\"matrix-assignment\",\"attempt_id\":\"matrix-attempt\",\"next_action\":\"wait\"}}"),
            "session_waited" => {
                let (report, terminal) = if source_state == "reported" {
                    ("\"/tmp/matrix.md\"", "null")
                } else {
                    ("null", "\"observed terminal\"")
                };
                format!("{{\"v\":1,\"last_waited_at\":\"{timestamp}\",\"consumed_report_ref\":{report},\"terminal_observation\":{terminal}}}")
            }
            "session_failed" | "session_abandoned" | "session_externally_unknown" => format!("{{\"v\":1,\"final_reason\":\"terminal\",\"ended_at\":\"{timestamp}\",\"next_action\":\"audit\"}}"),
            "session_interrupted" => format!("{{\"v\":1,\"interruption_reason\":\"host\",\"interrupted_at\":\"{timestamp}\",\"next_action\":\"recover\"}}"),
            "session_close_requested" => format!("{{\"v\":1,\"close_requested_at\":\"{timestamp}\",\"next_action\":\"close\"}}"),
            "session_closed" => format!("{{\"v\":1,\"outcome\":\"unknown\",\"close_disposition\":\"confirmed\",\"closed_at\":\"{timestamp}\",\"ended_at\":\"{timestamp}\"}}"),
            "session_superseded" => format!("{{\"v\":1,\"superseded_by_session_id\":\"matrix-replacement\",\"superseded_at\":\"{timestamp}\",\"reason\":\"replace\",\"next_action\":\"close\"}}"),
            "lease_planned" => "{\"v\":1,\"role\":\"worker\",\"model_profile\":\"standard\",\"risk_class\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"base_revision\":\"base\",\"independence_boundary_id\":null,\"replaces_session_id\":null,\"expiry_predicate\":null,\"expires_at\":null,\"next_action\":\"issue\"}".into(),
            "lease_issued" => format!("{{\"v\":1,\"issued_at\":\"{timestamp}\",\"next_action\":\"run\"}}"),
            "lease_reused" => format!("{{\"v\":1,\"attempt_id\":\"matrix-attempt\",\"compatibility_evidence\":{{\"v\":1,\"items\":[{{\"kind\":\"scope\",\"ref\":\"matrix-lease\",\"result\":\"compatible\"}}]}},\"last_used_at\":\"{timestamp}\",\"next_action\":\"run\"}}"),
            "lease_idle" => format!("{{\"v\":1,\"last_used_at\":\"{timestamp}\",\"next_action\":\"reuse\"}}"),
            "lease_expired" | "lease_revoked" => format!("{{\"v\":1,\"expiry_reason\":\"terminal\",\"ended_at\":\"{timestamp}\",\"next_action\":\"close\"}}"),
            "lease_closed" => format!("{{\"v\":1,\"expiry_reason\":\"cleanup\",\"ended_at\":\"{timestamp}\"}}"),
            "usage_observed" => format!("{{\"v\":1,\"scope\":\"session\",\"subject_id\":\"matrix-session\",\"observation_kind\":\"cumulative\",\"window_start\":null,\"window_end\":\"{timestamp}\",\"input_tokens\":1,\"output_tokens\":null,\"reasoning_tokens\":null,\"cache_read_tokens\":null,\"cache_write_tokens\":null,\"credits\":null,\"provider_cost\":null,\"telemetry_quality\":\"exact\",\"supersedes_event_id\":null}}"),
            "quality_gate_passed" => format!("{{\"v\":1,\"subject_kind\":\"assignment\",\"subject_id\":\"matrix-assignment\",\"policy\":{{\"v\":1,\"kind\":\"deterministic\"}},\"evidence\":{{\"v\":1,\"items\":[{{\"kind\":\"test\",\"ref\":\"suite\",\"result\":\"passed\"}}]}},\"observed_at\":\"{timestamp}\"}}"),
            "quality_gate_failed" => format!("{{\"v\":1,\"subject_kind\":\"assignment\",\"subject_id\":\"matrix-assignment\",\"policy\":{{\"v\":1,\"kind\":\"deterministic\"}},\"findings_ref\":\"/tmp/findings.md\",\"next_action\":\"fix\",\"observed_at\":\"{timestamp}\"}}"),
            _ => panic!("missing executable payload for {name}"),
        }
    }

    fn insert_transition_graph(conn: &Connection) {
        conn.execute("INSERT INTO work_packages(package_id,run_id,title,role_floor,model_floor,risk_floor,write_scope,review_policy,independence_policy,status,next_action) VALUES('matrix-package','matrix-run','matrix','worker','standard','medium','{\"v\":1,\"paths\":[]}','{\"v\":1,\"kind\":\"none\"}','{\"v\":1,\"kind\":\"none\"}','active','work')",[]).unwrap();
        conn.execute("INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,base_revision,current_step,attempt_count,status,next_action) VALUES('matrix-assignment','matrix-package','matrix',1,'implementation','worker','standard','medium','{\"v\":1,\"paths\":[]}','base','edit',1,'running','work')",[]).unwrap();
        conn.execute("INSERT INTO agent_sessions(session_id,run_id,handle,host_id,role,requested_profile,token_budget_mode,status,next_action) VALUES('matrix-session','matrix-run','matrix-handle','host','worker','standard','unbounded','running','work')",[]).unwrap();
        conn.execute("INSERT INTO assignment_attempts(attempt_id,assignment_id,session_id,attempt_sequence,status,next_action) VALUES('matrix-attempt','matrix-assignment','matrix-session',1,'planned','work')",[]).unwrap();
        conn.execute("INSERT INTO session_leases(lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,current_attempt_id,status,reuse_count,next_action) VALUES('matrix-lease','matrix-session','matrix-package','worker','standard','medium','{\"v\":1,\"paths\":[]}','base','matrix-attempt','active',0,'work')",[]).unwrap();
        conn.execute("UPDATE assignment_attempts SET lease_id='matrix-lease' WHERE attempt_id='matrix-attempt'",[]).unwrap();
        conn.execute("INSERT INTO routing_decisions(route_id,attempt_id,required_profile,routing_status,eligibility_status,next_action,decided_at) VALUES('matrix-route','matrix-attempt','standard','requested','unknown','apply','2026-07-12T09:29:00.000Z')",[]).unwrap();
        conn.execute("UPDATE assignment_attempts SET route_id='matrix-route' WHERE attempt_id='matrix-attempt'",[]).unwrap();
        conn.execute("UPDATE assignments SET current_attempt_id='matrix-attempt' WHERE assignment_id='matrix-assignment'",[]).unwrap();
        conn.execute("INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('matrix-replacement','matrix-run','worker','unbounded','planned')",[]).unwrap();
        conn.execute("INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('matrix-review-session','matrix-run','reviewer','unbounded','reported')",[]).unwrap();
        conn.execute("INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,attempt_count,status,report_path,test_evidence,review_evidence) VALUES('matrix-review','matrix-package','review',2,'review','reviewer','standard','medium','{\"v\":1,\"paths\":[]}',1,'accepted','/tmp/review.md','{\"v\":1,\"items\":[{\"kind\":\"test\",\"ref\":\"suite\",\"result\":\"passed\"}]}','{\"v\":1,\"items\":[{\"kind\":\"review\",\"ref\":\"matrix-review\",\"result\":\"accepted\"}]}')",[]).unwrap();
        conn.execute("INSERT INTO assignment_attempts(attempt_id,assignment_id,session_id,attempt_sequence,status) VALUES('matrix-review-attempt','matrix-review','matrix-review-session',1,'accepted')",[]).unwrap();
        conn.execute("UPDATE assignments SET current_attempt_id='matrix-review-attempt' WHERE assignment_id='matrix-review'",[]).unwrap();
    }

    fn source_for_event(name: &str) -> &'static str {
        match super::event_rule(name).unwrap().source_floor {
            super::SourceFloor::ControllerObservation => "controller-observation",
            super::SourceFloor::AgentReport => "agent-report",
            super::SourceFloor::Inference => "inference",
        }
    }

    fn executable_transition_fixture(
        name: &str,
        source_state: &str,
        legal: bool,
    ) -> (Connection, EventInput) {
        let mut conn = connection();
        if name == "run_planned" {
            if !legal {
                append_run(
                    &mut conn,
                    "matrix-existing",
                    "matrix-run",
                    "matrix-existing",
                );
            }
        } else {
            append_run(&mut conn, "matrix-seed", "matrix-run", "matrix-seed");
            conn.execute(
                "UPDATE runs SET status='active' WHERE run_id='matrix-run'",
                [],
            )
            .unwrap();
            insert_transition_graph(&conn);
        }

        let target_state = if legal { source_state } else { "illegal" };
        if name.starts_with("run_") && name != "run_planned" {
            let status = if legal { source_state } else { "complete" };
            conn.execute(
                "UPDATE runs SET status=?1 WHERE run_id='matrix-run'",
                [status],
            )
            .unwrap();
        } else if name.starts_with("package_") {
            if name != "package_planned" {
                let status = if legal { source_state } else { "complete" };
                if name == "package_completed" && source_state == "review" && legal {
                    conn.execute("UPDATE work_packages SET review_policy='{\"v\":1,\"kind\":\"independent\"}',independence_policy='{\"v\":1,\"kind\":\"different-role-and-session\"}',status='active' WHERE package_id='matrix-package'",[]).unwrap();
                    append_named(
                        &mut conn,
                        "matrix-run",
                        "matrix-review-start",
                        "package_review_started",
                        Refs {
                            package: Some("matrix-package"),
                            assignment: Some("matrix-review"),
                            ..Refs::default()
                        },
                        "{\"v\":1,\"review_assignment_id\":\"matrix-review\",\"next_action\":\"review\"}",
                    );
                    append_named(
                        &mut conn,
                        "matrix-run",
                        "matrix-review-pass",
                        "package_review_completed",
                        Refs {
                            package: Some("matrix-package"),
                            assignment: Some("matrix-review"),
                            ..Refs::default()
                        },
                        "{\"v\":1,\"verdict\":\"accepted\",\"review_evidence\":{\"v\":1,\"items\":[{\"kind\":\"review\",\"ref\":\"/tmp/review.md\",\"result\":\"accepted\"}]},\"next_action\":\"complete\"}",
                    );
                } else if name == "package_review_completed" && legal {
                    conn.execute("UPDATE work_packages SET review_policy='{\"v\":1,\"kind\":\"independent\"}',independence_policy='{\"v\":1,\"kind\":\"different-role-and-session\"}',status='active' WHERE package_id='matrix-package'",[]).unwrap();
                    append_named(
                        &mut conn,
                        "matrix-run",
                        "matrix-review-start",
                        "package_review_started",
                        Refs {
                            package: Some("matrix-package"),
                            assignment: Some("matrix-review"),
                            ..Refs::default()
                        },
                        "{\"v\":1,\"review_assignment_id\":\"matrix-review\",\"next_action\":\"review\"}",
                    );
                } else if name == "package_unblocked" && legal {
                    conn.execute("UPDATE work_packages SET status='active' WHERE package_id='matrix-package'",[]).unwrap();
                    append_named(
                        &mut conn,
                        "matrix-run",
                        "matrix-package-block",
                        "package_blocked",
                        Refs {
                            package: Some("matrix-package"),
                            ..Refs::default()
                        },
                        "{\"v\":1,\"blocker\":\"blocked\",\"resume_status\":\"active\",\"next_action\":\"resolve\"}",
                    );
                } else {
                    conn.execute(
                        "UPDATE work_packages SET status=?1,blocker=CASE WHEN ?1='blocked' THEN 'blocked' ELSE NULL END WHERE package_id='matrix-package'",
                        [status],
                    )
                    .unwrap();
                }
            }
        } else if name.starts_with("assignment_") {
            if name != "assignment_planned" {
                let status = if legal { source_state } else { "accepted" };
                conn.execute(
                    "UPDATE assignments SET status=?1 WHERE assignment_id='matrix-assignment'",
                    [status],
                )
                .unwrap();
                if name == "assignment_started" && legal {
                    conn.execute(
                        "UPDATE assignment_attempts SET status='running' WHERE attempt_id='matrix-attempt'",
                        [],
                    )
                    .unwrap();
                }
                if name == "assignment_requeued" && legal {
                    conn.execute("UPDATE assignment_attempts SET status='failed' WHERE attempt_id='matrix-attempt'",[]).unwrap();
                }
                if name == "assignment_unblocked" && legal {
                    conn.execute("UPDATE assignments SET status=?1,blocker=NULL WHERE assignment_id='matrix-assignment'",[source_state]).unwrap();
                    append_named(
                        &mut conn,
                        "matrix-run",
                        "matrix-assignment-block",
                        "assignment_blocked",
                        Refs {
                            package: Some("matrix-package"),
                            assignment: Some("matrix-assignment"),
                            ..Refs::default()
                        },
                        "{\"v\":1,\"blocker\":\"blocked\",\"next_action\":\"resolve\"}",
                    );
                }
            }
        } else if name.starts_with("attempt_") {
            if name == "attempt_planned" {
                conn.execute(
                    "UPDATE assignments SET status=?1 WHERE assignment_id='matrix-assignment'",
                    [if legal && source_state == "assignment-queued" {
                        "queued"
                    } else if legal {
                        "planned"
                    } else {
                        "running"
                    }],
                )
                .unwrap();
            } else {
                conn.execute(
                    "UPDATE assignment_attempts SET status=?1 WHERE attempt_id='matrix-attempt'",
                    [if legal { source_state } else { "accepted" }],
                )
                .unwrap();
            }
        } else if name.starts_with("dispatch_") {
            conn.execute(
                "UPDATE assignment_attempts SET status=?1 WHERE attempt_id='matrix-attempt'",
                [if legal { "planned" } else { "running" }],
            )
            .unwrap();
        } else if name.starts_with("route_") {
            if name == "route_requested" {
                conn.execute(
                    "UPDATE assignment_attempts SET status=?1 WHERE attempt_id='matrix-attempt'",
                    [if legal { "planned" } else { "running" }],
                )
                .unwrap();
            } else {
                conn.execute(
                    "UPDATE routing_decisions SET routing_status=?1 WHERE route_id='matrix-route'",
                    [if legal { source_state } else { "rejected" }],
                )
                .unwrap();
            }
        } else if name.starts_with("session_") {
            if name == "session_planned" {
                conn.execute("UPDATE assignment_attempts SET status='planned',session_id=NULL,lease_id=NULL WHERE attempt_id='matrix-attempt'",[]).unwrap();
            } else {
                conn.execute(
                    "UPDATE agent_sessions SET status=?1 WHERE session_id='matrix-session'",
                    [if legal { source_state } else { "closed" }],
                )
                .unwrap();
                if name == "session_running" && legal {
                    conn.execute("UPDATE assignment_attempts SET status='planned',session_id=NULL,lease_id=NULL WHERE attempt_id='matrix-attempt'",[]).unwrap();
                    conn.execute("UPDATE session_leases SET status='active',current_attempt_id=NULL WHERE lease_id='matrix-lease'",[]).unwrap();
                }
                if name == "session_unblocked" && legal {
                    append_named(
                        &mut conn,
                        "matrix-run",
                        "matrix-session-block",
                        "session_blocked",
                        Refs {
                            session: Some("matrix-session"),
                            ..Refs::default()
                        },
                        "{\"v\":1,\"blocker\":\"blocked\",\"next_action\":\"resolve\"}",
                    );
                }
            }
        } else if name.starts_with("lease_") && name != "lease_planned" {
            conn.execute(
                "UPDATE session_leases SET status=?1 WHERE lease_id='matrix-lease'",
                [if legal { source_state } else { "closed" }],
            )
            .unwrap();
        }

        let mut refs = Refs::default();
        match name {
            n if n.starts_with("package_") => {
                refs.package = Some(if n == "package_planned" && legal {
                    "matrix-new-package"
                } else {
                    "matrix-package"
                });
            }
            n if n.starts_with("assignment_") => {
                refs.package = Some("matrix-package");
                refs.assignment = Some(if n == "assignment_planned" && legal {
                    "matrix-new-assignment"
                } else {
                    "matrix-assignment"
                });
                if n == "assignment_started" {
                    refs.attempt = Some("matrix-attempt");
                }
            }
            n if n.starts_with("attempt_") => {
                refs.package = Some("matrix-package");
                refs.assignment = Some("matrix-assignment");
                refs.attempt = Some(if n == "attempt_planned" && legal {
                    "matrix-new-attempt"
                } else {
                    "matrix-attempt"
                });
            }
            n if n.starts_with("dispatch_") || n.starts_with("route_") => {
                refs.package = Some("matrix-package");
                refs.assignment = Some("matrix-assignment");
                refs.attempt = Some("matrix-attempt");
                if n == "dispatch_reuse_selected" {
                    refs.session = Some("matrix-session");
                    refs.lease = Some("matrix-lease");
                } else if n == "dispatch_spawn_selected" {
                    refs.session = Some("matrix-session");
                }
            }
            "session_planned" => {
                refs.package = Some("matrix-package");
                refs.assignment = Some("matrix-assignment");
                refs.attempt = Some("matrix-attempt");
                refs.session = Some(if legal {
                    "matrix-new-session"
                } else {
                    "matrix-session"
                });
            }
            "session_running" => {
                refs.package = Some("matrix-package");
                refs.assignment = Some("matrix-assignment");
                refs.attempt = Some("matrix-attempt");
                refs.session = Some("matrix-session");
                refs.lease = Some("matrix-lease");
            }
            "session_reported" => {
                refs.package = Some("matrix-package");
                refs.assignment = Some("matrix-assignment");
                refs.attempt = Some("matrix-attempt");
                refs.session = Some("matrix-session");
            }
            n if n.starts_with("session_") => refs.session = Some("matrix-session"),
            n if n.starts_with("lease_") => {
                refs.package = Some("matrix-package");
                refs.session = Some("matrix-session");
                refs.lease = Some(if n == "lease_planned" && legal {
                    "matrix-new-lease"
                } else {
                    "matrix-lease"
                });
                if n == "lease_reused" {
                    refs.assignment = Some("matrix-assignment");
                    refs.attempt = Some("matrix-attempt");
                }
            }
            "usage_observed" => refs.session = Some("matrix-session"),
            "quality_gate_passed" | "quality_gate_failed" => {
                refs.package = Some("matrix-package");
                refs.assignment = Some("matrix-assignment");
            }
            _ => {}
        }
        if matches!(name, "package_review_started" | "package_review_completed") {
            refs.assignment = Some("matrix-review");
        }
        if !legal
            && matches!(
                name,
                "usage_observed" | "quality_gate_passed" | "quality_gate_failed"
            )
        {
            if name == "usage_observed" {
                refs.session = Some("missing-session");
            } else {
                refs.assignment = Some("missing-assignment");
            }
        }
        let payload = if !legal && name == "usage_observed" {
            transition_payload(name, target_state).replace("matrix-session", "missing-session")
        } else if !legal && matches!(name, "quality_gate_passed" | "quality_gate_failed") {
            transition_payload(name, target_state)
                .replace("matrix-assignment", "missing-assignment")
        } else {
            transition_payload(name, target_state)
        };
        let event = EventInput {
            event_id: format!(
                "matrix-{name}-{source_state}-{}",
                if legal { "legal" } else { "illegal" }
            ),
            run_id: "matrix-run".into(),
            package_id: refs.package.map(str::to_string),
            assignment_id: refs.assignment.map(str::to_string),
            attempt_id: refs.attempt.map(str::to_string),
            session_id: refs.session.map(str::to_string),
            lease_id: refs.lease.map(str::to_string),
            event_type: name.into(),
            source_kind: source_for_event(name).into(),
            source_id: format!("matrix-source-{name}"),
            confidence: Some(9_000),
            payload_json: payload,
            occurred_at: "2026-07-12T09:30:00.000Z".into(),
            idempotency_key: format!(
                "matrix-{name}-{source_state}-{}",
                if legal { "legal" } else { "illegal" }
            ),
        };
        (conn, event)
    }

    #[test]
    fn transition_table_has_legal_effect_and_illegal_source_for_every_event() {
        for name in event_names() {
            let (legal, effect) = transition_contract(name);
            for source_state in legal {
                let (mut conn, event) = executable_transition_fixture(name, source_state, true);
                let before = event_count_and_counter(&conn, &event.run_id);
                append_event(&mut conn, &event)
                    .unwrap_or_else(|error| panic!("legal {name} from {source_state}: {error}"));
                let after = event_count_and_counter(&conn, &event.run_id);
                assert_eq!(after.0, before.0 + 1, "{name}/{source_state}/{effect}");
                assert_eq!(after.1, before.1 + 1, "{name}/{source_state}/{effect}");
            }

            let (mut conn, event) = executable_transition_fixture(name, "illegal", false);
            let before = event_count_and_counter(&conn, &event.run_id);
            let error = append_event(&mut conn, &event).unwrap_err();
            assert!(
                error.contains("illegal")
                    || error.contains("already")
                    || error.contains("unknown")
                    || error.contains("does not match")
                    || error.contains("requires"),
                "illegal {name}: {error}"
            );
            assert_eq!(event_count_and_counter(&conn, &event.run_id), before);

            let (mut conn, mut event) = executable_transition_fixture(name, legal[0], true);
            event.source_kind = illegal_source_for(name).into();
            event.source_id = format!("illegal-source-{name}");
            event.idempotency_key = format!("illegal-source-{name}");
            let before = event_count_and_counter(&conn, &event.run_id);
            let error = append_event(&mut conn, &event).unwrap_err();
            assert!(
                error.contains("unauthorized source") || error.contains("unknown source kind"),
                "illegal source {name}: {error}"
            );
            assert_eq!(event_count_and_counter(&conn, &event.run_id), before);
        }
        assert_eq!(
            transition_contract("assignment_reported"),
            (&["running"][..], "reported")
        );
        assert_eq!(
            transition_contract("session_interrupted"),
            (&["spawned", "running", "reported"][..], "facet")
        );
    }

    #[test]
    fn event_rejects_unknown_type_source_confidence_or_identity() {
        let cases: Vec<(&str, EventMutation)> = vec![
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
            "INSERT INTO work_packages(package_id,run_id,title,role_floor,model_floor,risk_floor,write_scope,review_policy,independence_policy,status) VALUES('package-a','run-a','package','worker','standard','medium','{\"v\":1,\"paths\":[]}','{\"v\":1,\"kind\":\"deterministic\"}','{\"v\":1,\"kind\":\"different-session\"}','planned')",
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
    fn canonical_run_package_assignment_attempt_session_lease_route_chain() {
        let mut conn = connection();
        setup_planned_attempt(&mut conn);
        let attempt_refs = || Refs {
            package: Some("package-1"),
            assignment: Some("assignment-1"),
            attempt: Some("attempt-1"),
            ..Refs::default()
        };
        append_named(
            &mut conn,
            "run-1",
            "e09",
            "route_requested",
            attempt_refs(),
            "{\"v\":1,\"route_id\":\"route-1\",\"required_profile\":\"standard\",\"requested_model\":null,\"requested_reasoning\":null,\"escalated_from_route_id\":null,\"decided_at\":\"2026-07-12T09:08:09.006Z\",\"next_action\":\"spawn\"}",
        );
        append_named(
            &mut conn,
            "run-1",
            "e10",
            "session_planned",
            Refs {
                session: Some("session-1"),
                ..attempt_refs()
            },
            "{\"v\":1,\"authorization_ref\":\"auth-1\",\"budget_reason\":\"package writer\",\"run_max_open\":2,\"run_max_total\":4,\"session_token_budget\":{\"mode\":\"unbounded\",\"tokens\":null},\"nested_delegation\":{\"allowed\":false,\"authority_ref\":null},\"requested_host\":null,\"requested_profile\":\"standard\",\"requested_model\":null,\"requested_reasoning\":null,\"parent_session_id\":null}",
        );
        let full_refs = || Refs {
            package: Some("package-1"),
            assignment: Some("assignment-1"),
            attempt: Some("attempt-1"),
            session: Some("session-1"),
            lease: Some("lease-1"),
        };
        let session_refs = || Refs {
            package: Some("package-1"),
            assignment: Some("assignment-1"),
            session: Some("session-1"),
            ..Refs::default()
        };
        let lease_refs = || Refs {
            package: Some("package-1"),
            session: Some("session-1"),
            lease: Some("lease-1"),
            ..Refs::default()
        };
        append_named(
            &mut conn,
            "run-1",
            "e11",
            "lease_planned",
            lease_refs(),
            "{\"v\":1,\"role\":\"worker\",\"model_profile\":\"standard\",\"risk_class\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[\"src\"]},\"base_revision\":\"base\",\"independence_boundary_id\":null,\"replaces_session_id\":null,\"expiry_predicate\":null,\"expires_at\":null,\"next_action\":\"spawn\"}",
        );
        append_named(
            &mut conn,
            "run-1",
            "e12",
            "session_spawned",
            session_refs(),
            "{\"v\":1,\"handle\":\"host-handle\",\"host_id\":\"host-1\",\"spawned_at\":\"2026-07-12T09:08:10.006Z\",\"next_action\":\"apply route\"}",
        );
        append_named(
            &mut conn,
            "run-1",
            "e13",
            "route_applied",
            attempt_refs(),
            "{\"v\":1,\"routing_status\":\"applied\",\"actual_model\":\"observed-model\",\"actual_reasoning\":null,\"eligibility_evidence\":{\"v\":1,\"items\":[{\"kind\":\"probe\",\"ref\":\"route-1\",\"result\":\"eligible\"}]},\"decided_at\":\"2026-07-12T09:08:11.006Z\"}",
        );
        append_named(
            &mut conn,
            "run-1",
            "e14",
            "lease_issued",
            lease_refs(),
            "{\"v\":1,\"issued_at\":\"2026-07-12T09:08:12.006Z\",\"next_action\":\"run\"}",
        );
        append_named(
            &mut conn,
            "run-1",
            "e15",
            "session_running",
            full_refs(),
            "{\"v\":1,\"started_or_resumed_at\":\"2026-07-12T09:08:13.006Z\",\"lease_id\":\"lease-1\",\"attempt_id\":\"attempt-1\",\"prior_report_ref\":null,\"gate_evidence\":null,\"next_action\":\"work\"}",
        );
        append_named(
            &mut conn,
            "run-1",
            "e16",
            "attempt_started",
            full_refs(),
            "{\"v\":1,\"started_at\":\"2026-07-12T09:08:14.006Z\",\"next_action\":\"work\"}",
        );
        append_named(
            &mut conn,
            "run-1",
            "e17",
            "assignment_started",
            full_refs(),
            "{\"v\":1,\"attempt_id\":\"attempt-1\",\"started_at\":\"2026-07-12T09:08:14.006Z\",\"current_step\":\"edit\"}",
        );
        assert_eq!(
            conn.query_row("SELECT status FROM runs WHERE run_id='run-1'", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap(),
            "active"
        );
        assert_eq!(
            conn.query_row(
                "SELECT status FROM work_packages WHERE package_id='package-1'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "active"
        );
        assert_eq!(
            conn.query_row(
                "SELECT status,current_attempt_id,attempt_count FROM assignments WHERE assignment_id='assignment-1'",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, i64>(2)?))
            )
            .unwrap(),
            ("running".into(), Some("attempt-1".into()), 1)
        );
        assert_eq!(
            conn.query_row(
                "SELECT status,route_id,session_id,lease_id FROM assignment_attempts WHERE attempt_id='attempt-1'",
                [],
                |row| Ok((row.get::<_,String>(0)?,row.get::<_,Option<String>>(1)?,row.get::<_,Option<String>>(2)?,row.get::<_,Option<String>>(3)?))
            )
            .unwrap(),
            ("running".into(),Some("route-1".into()),Some("session-1".into()),Some("lease-1".into()))
        );
        assert_eq!(
            conn.query_row(
                "SELECT status FROM session_leases WHERE lease_id='lease-1'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "active"
        );
        assert_eq!(conn.query_row("SELECT routing_status,actual_model FROM routing_decisions WHERE route_id='route-1'",[],|row|Ok((row.get::<_,String>(0)?,row.get::<_,Option<String>>(1)?))).unwrap(),("applied".into(),Some("observed-model".into())));
    }

    #[test]
    fn nonbinding_events_reject_unattached_and_cross_session_attempts() {
        let mut conn = connection();
        setup_planned_attempt(&mut conn);
        for session in ["session-a", "session-b"] {
            conn.execute(
                "INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES(?1,'run-1','worker','unbounded','running')",
                [session],
            )
            .unwrap();
        }
        let heartbeat = |event_id: &str, session_id: &str| EventInput {
            event_id: event_id.into(),
            run_id: "run-1".into(),
            package_id: Some("package-1".into()),
            assignment_id: Some("assignment-1".into()),
            attempt_id: Some("attempt-1".into()),
            session_id: Some(session_id.into()),
            lease_id: None,
            event_type: "session_heartbeat".into(),
            source_kind: "host-runtime".into(),
            source_id: "host".into(),
            confidence: None,
            payload_json: "{\"v\":1,\"last_activity_at\":\"2026-07-12T09:08:20.006Z\"}".into(),
            occurred_at: "2026-07-12T09:08:20.006Z".into(),
            idempotency_key: event_id.into(),
        };
        let error = append_event(&mut conn, &heartbeat("bad-unattached", "session-a")).unwrap_err();
        assert!(error.contains("does not belong to session"), "{error}");
        conn.execute(
            "UPDATE assignment_attempts SET session_id='session-a' WHERE attempt_id='attempt-1'",
            [],
        )
        .unwrap();
        let error = append_event(&mut conn, &heartbeat("bad-cross", "session-b")).unwrap_err();
        assert!(error.contains("does not belong to session"), "{error}");
    }

    fn setup_usage_subjects(conn: &mut Connection) {
        append_run(conn, "usage-run", "usage-run", "usage-run");
        conn.execute(
            "INSERT INTO work_packages(package_id,run_id,title,role_floor,model_floor,risk_floor,write_scope,review_policy,independence_policy,status) VALUES('usage-package','usage-run','usage','worker','standard','medium','{\"v\":1,\"paths\":[]}','{\"v\":1,\"kind\":\"none\"}','{\"v\":1,\"kind\":\"none\"}','active')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,attempt_count,status) VALUES('usage-assignment','usage-package','usage',1,'implementation','worker','standard','medium','{\"v\":1,\"paths\":[]}',1,'running')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('usage-session','usage-run','worker','unbounded','running')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO assignment_attempts(attempt_id,assignment_id,session_id,attempt_sequence,status) VALUES('usage-attempt','usage-assignment','usage-session',1,'running')",
            [],
        ).unwrap();
        conn.execute(
            "UPDATE assignments SET current_attempt_id='usage-attempt' WHERE assignment_id='usage-assignment'",
            [],
        ).unwrap();
    }

    struct Usage<'a> {
        event_id: &'a str,
        scope: &'a str,
        observation: &'a str,
        input_tokens: Option<i64>,
        source_kind: &'a str,
        supersedes: Option<&'a str>,
        credits: &'a str,
        cost: &'a str,
    }

    fn usage_event(input: Usage<'_>) -> EventInput {
        let (subject, package, assignment, attempt, session) = if input.scope == "attempt" {
            (
                "usage-attempt",
                Some("usage-package".into()),
                Some("usage-assignment".into()),
                Some("usage-attempt".into()),
                None,
            )
        } else {
            (
                "usage-session",
                None,
                None,
                None,
                Some("usage-session".into()),
            )
        };
        let tokens = input
            .input_tokens
            .map_or_else(|| "null".to_string(), |value| value.to_string());
        let supersedes = input
            .supersedes
            .map_or_else(|| "null".to_string(), |value| format!("\"{value}\""));
        EventInput {
            event_id: input.event_id.into(),
            run_id: "usage-run".into(),
            package_id: package,
            assignment_id: assignment,
            attempt_id: attempt,
            session_id: session,
            lease_id: None,
            event_type: "usage_observed".into(),
            source_kind: input.source_kind.into(),
            source_id: format!("source-{}", input.event_id),
            confidence: Some(5000),
            payload_json: format!(
                "{{\"v\":1,\"scope\":\"{}\",\"subject_id\":\"{}\",\"observation_kind\":\"{}\",\"window_start\":null,\"window_end\":\"2026-07-12T09:09:00.006Z\",\"input_tokens\":{},\"output_tokens\":null,\"reasoning_tokens\":null,\"cache_read_tokens\":null,\"cache_write_tokens\":null,\"credits\":{},\"provider_cost\":{},\"telemetry_quality\":\"exact\",\"supersedes_event_id\":{}}}",
                input.scope,
                subject,
                input.observation,
                tokens,
                input.credits,
                input.cost,
                supersedes,
            ),
            occurred_at: "2026-07-12T09:09:00.006Z".into(),
            idempotency_key: input.event_id.into(),
        }
    }

    #[test]
    fn weaker_late_evidence_never_overwrites_stronger() {
        let mut conn = connection();
        setup_usage_subjects(&mut conn);
        for usage in [
            Usage {
                event_id: "usage-strong",
                scope: "session",
                observation: "cumulative",
                input_tokens: Some(10),
                source_kind: "host-runtime",
                supersedes: None,
                credits: "null",
                cost: "null",
            },
            Usage {
                event_id: "usage-weak",
                scope: "session",
                observation: "cumulative",
                input_tokens: Some(20),
                source_kind: "inference",
                supersedes: None,
                credits: "null",
                cost: "null",
            },
        ] {
            append_event(&mut conn, &usage_event(usage)).unwrap();
        }
        assert_eq!(
            conn.query_row(
                "SELECT input_tokens FROM agent_sessions WHERE session_id='usage-session'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            10
        );
        assert_eq!(conn.query_row("SELECT winner_event_id,winner_source_kind,conflict_count,last_conflict_event_id FROM projection_field_sources WHERE entity_kind='session' AND entity_id='usage-session' AND field_name='input_tokens'",[],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)?,row.get::<_,Option<String>>(3)?))).unwrap(),("usage-strong".into(),"host-runtime".into(),1,Some("usage-weak".into())));
    }

    #[test]
    fn same_priority_later_sequence_wins_and_records_conflict() {
        let mut conn = connection();
        setup_usage_subjects(&mut conn);
        for usage in [
            Usage {
                event_id: "usage-one",
                scope: "session",
                observation: "cumulative",
                input_tokens: Some(10),
                source_kind: "controller-observation",
                supersedes: None,
                credits: "null",
                cost: "null",
            },
            Usage {
                event_id: "usage-two",
                scope: "session",
                observation: "cumulative",
                input_tokens: Some(20),
                source_kind: "controller-observation",
                supersedes: None,
                credits: "null",
                cost: "null",
            },
        ] {
            append_event(&mut conn, &usage_event(usage)).unwrap();
        }
        assert_eq!(
            conn.query_row(
                "SELECT input_tokens FROM agent_sessions WHERE session_id='usage-session'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            20
        );
        assert_eq!(conn.query_row("SELECT winner_event_id,conflict_count FROM projection_field_sources WHERE entity_kind='session' AND entity_id='usage-session' AND field_name='input_tokens'",[],|row|Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?))).unwrap(),("usage-two".into(),1));
    }

    #[test]
    fn stronger_correction_can_replace_weaker_projection() {
        let mut conn = connection();
        setup_usage_subjects(&mut conn);
        append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "usage-weak",
                scope: "session",
                observation: "cumulative",
                input_tokens: Some(20),
                source_kind: "inference",
                supersedes: None,
                credits: "null",
                cost: "null",
            }),
        )
        .unwrap();
        append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "usage-correction",
                scope: "session",
                observation: "correction",
                input_tokens: Some(5),
                source_kind: "host-runtime",
                supersedes: Some("usage-weak"),
                credits: "null",
                cost: "null",
            }),
        )
        .unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT input_tokens FROM agent_sessions WHERE session_id='usage-session'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            5
        );
        assert_eq!(conn.query_row("SELECT winner_event_id,winner_source_kind,conflict_count FROM projection_field_sources WHERE entity_kind='session' AND entity_id='usage-session' AND field_name='input_tokens'",[],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)?))).unwrap(),("usage-correction".into(),"host-runtime".into(),1));
    }

    #[test]
    fn corroborating_equal_value_is_not_a_conflict() {
        let mut conn = connection();
        setup_usage_subjects(&mut conn);
        append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "usage-strong",
                scope: "session",
                observation: "cumulative",
                input_tokens: Some(10),
                source_kind: "host-runtime",
                supersedes: None,
                credits: "null",
                cost: "null",
            }),
        )
        .unwrap();
        append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "usage-equal",
                scope: "session",
                observation: "cumulative",
                input_tokens: Some(10),
                source_kind: "inference",
                supersedes: None,
                credits: "null",
                cost: "null",
            }),
        )
        .unwrap();
        assert_eq!(conn.query_row("SELECT winner_event_id,conflict_count,last_conflict_event_id FROM projection_field_sources WHERE entity_kind='session' AND entity_id='usage-session' AND field_name='input_tokens'",[],|row|Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?,row.get::<_,Option<String>>(2)?))).unwrap(),("usage-strong".into(),0,None));
    }

    #[test]
    fn attempt_deltas_and_session_cumulative_usage_do_not_double_count() {
        let mut conn = connection();
        setup_usage_subjects(&mut conn);
        append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "attempt-delta",
                scope: "attempt",
                observation: "delta",
                input_tokens: Some(3),
                source_kind: "host-runtime",
                supersedes: None,
                credits: "{\"amount\":2,\"scale\":0,\"unit\":\"credit\"}",
                cost: "null",
            }),
        )
        .unwrap();
        append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "session-total",
                scope: "session",
                observation: "cumulative",
                input_tokens: Some(10),
                source_kind: "host-runtime",
                supersedes: None,
                credits: "{\"amount\":9,\"scale\":0,\"unit\":\"credit\"}",
                cost: "null",
            }),
        )
        .unwrap();
        assert_eq!(conn.query_row("SELECT input_tokens,credits_amount FROM assignment_attempts WHERE attempt_id='usage-attempt'",[],|row|Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?))).unwrap(),(3,2));
        assert_eq!(conn.query_row("SELECT input_tokens,credits_amount FROM agent_sessions WHERE session_id='usage-session'",[],|row|Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?))).unwrap(),(10,9));
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM control_plane_events WHERE event_type='usage_observed'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
    }

    #[test]
    fn session_cumulative_counter_cannot_decrease_without_correction() {
        let mut conn = connection();
        setup_usage_subjects(&mut conn);
        append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "session-ten",
                scope: "session",
                observation: "cumulative",
                input_tokens: Some(10),
                source_kind: "host-runtime",
                supersedes: None,
                credits: "null",
                cost: "null",
            }),
        )
        .unwrap();
        let next_before = conn
            .query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='usage-run'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        let error = append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "session-five",
                scope: "session",
                observation: "cumulative",
                input_tokens: Some(5),
                source_kind: "host-runtime",
                supersedes: None,
                credits: "null",
                cost: "null",
            }),
        )
        .unwrap_err();
        assert!(error.contains("cannot decrease"), "{error}");
        assert_eq!(
            conn.query_row(
                "SELECT input_tokens FROM agent_sessions WHERE session_id='usage-session'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            10
        );
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='usage-run'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            next_before
        );
    }

    #[test]
    fn exact_cost_rejects_partial_float_negative_or_cross_currency_sum() {
        let invalid = [
            "{\"amount\":1,\"scale\":2}",
            "{\"amount\":1.5,\"scale\":2,\"currency\":\"USD\"}",
            "{\"amount\":-1,\"scale\":2,\"currency\":\"USD\"}",
            "{\"amount\":1,\"scale\":2,\"currency\":\"usd\"}",
        ];
        for (index, cost) in invalid.into_iter().enumerate() {
            let mut conn = connection();
            setup_usage_subjects(&mut conn);
            let error = append_event(
                &mut conn,
                &usage_event(Usage {
                    event_id: &format!("invalid-{index}"),
                    scope: "attempt",
                    observation: "delta",
                    input_tokens: None,
                    source_kind: "host-runtime",
                    supersedes: None,
                    credits: "null",
                    cost,
                }),
            )
            .unwrap_err();
            assert!(!error.is_empty());
            assert_eq!(
                conn.query_row(
                    "SELECT cost_amount FROM assignment_attempts WHERE attempt_id='usage-attempt'",
                    [],
                    |row| row.get::<_, Option<i64>>(0)
                )
                .unwrap(),
                None
            );
        }
        let mut conn = connection();
        setup_usage_subjects(&mut conn);
        append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "usd",
                scope: "attempt",
                observation: "delta",
                input_tokens: None,
                source_kind: "controller-observation",
                supersedes: None,
                credits: "null",
                cost: "{\"amount\":100,\"scale\":2,\"currency\":\"USD\"}",
            }),
        )
        .unwrap();
        append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "eur",
                scope: "attempt",
                observation: "delta",
                input_tokens: None,
                source_kind: "controller-observation",
                supersedes: None,
                credits: "null",
                cost: "{\"amount\":80,\"scale\":2,\"currency\":\"EUR\"}",
            }),
        )
        .unwrap();
        assert_eq!(conn.query_row("SELECT cost_amount,cost_scale,cost_currency FROM assignment_attempts WHERE attempt_id='usage-attempt'",[],|row|Ok((row.get::<_,i64>(0)?,row.get::<_,i64>(1)?,row.get::<_,String>(2)?))).unwrap(),(80,2,"EUR".into()));
        assert_eq!(conn.query_row("SELECT conflict_count FROM projection_field_sources WHERE entity_kind='attempt' AND entity_id='usage-attempt' AND field_name='provider_cost'",[],|row|row.get::<_,i64>(0)).unwrap(),1);
    }

    #[test]
    fn unknown_usage_remains_null_not_zero() {
        let mut conn = connection();
        setup_usage_subjects(&mut conn);
        append_event(
            &mut conn,
            &usage_event(Usage {
                event_id: "unknown",
                scope: "session",
                observation: "cumulative",
                input_tokens: None,
                source_kind: "host-runtime",
                supersedes: None,
                credits: "null",
                cost: "null",
            }),
        )
        .unwrap();
        let values:(Option<i64>,Option<i64>,Option<i64>,Option<i64>)=conn.query_row("SELECT input_tokens,output_tokens,credits_amount,cost_amount FROM agent_sessions WHERE session_id='usage-session'",[],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap();
        assert_eq!(values, (None, None, None, None));
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM projection_field_sources WHERE entity_kind='session' AND entity_id='usage-session'",[],|row|row.get::<_,i64>(0)).unwrap(),0);
    }

    fn setup_replay_source(conn: &mut Connection) {
        let at = "2026-07-12T09:10:00.006Z";
        let evidence =
            "{\"v\":1,\"items\":[{\"kind\":\"test\",\"ref\":\"suite\",\"result\":\"passed\"}]}";
        let review_evidence = "{\"v\":1,\"items\":[{\"kind\":\"review\",\"ref\":\"/tmp/review.md\",\"result\":\"accepted\"}]}";
        append_run(conn, "rich-run", "run-1", "rich-run");
        append_named(
            conn,
            "run-1",
            "rich-run-start",
            "run_started",
            Refs::default(),
            &format!(
                "{{\"v\":1,\"psoc_revision\":\"psoc-rich\",\"started_at\":\"{at}\",\"next_action\":\"packages\"}}"
            ),
        );

        append_named(
            conn,
            "run-1",
            "foundation-plan",
            "package_planned",
            Refs {
                package: Some("foundation"),
                ..Refs::default()
            },
            "{\"v\":1,\"title\":\"foundation\",\"dependencies\":[],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"review_policy\":{\"v\":1,\"kind\":\"none\"},\"independence_policy\":{\"v\":1,\"kind\":\"none\"},\"next_action\":\"ready\"}",
        );
        append_named(
            conn,
            "run-1",
            "foundation-ready",
            "package_ready",
            Refs {
                package: Some("foundation"),
                ..Refs::default()
            },
            "{\"v\":1,\"next_action\":\"active\"}",
        );
        append_named(
            conn,
            "run-1",
            "foundation-active",
            "package_active",
            Refs {
                package: Some("foundation"),
                ..Refs::default()
            },
            "{\"v\":1,\"next_action\":\"complete\"}",
        );
        append_named(
            conn,
            "run-1",
            "foundation-complete",
            "package_completed",
            Refs {
                package: Some("foundation"),
                ..Refs::default()
            },
            &format!("{{\"v\":1,\"ended_at\":\"{at}\"}}"),
        );

        append_named(
            conn,
            "run-1",
            "package-plan",
            "package_planned",
            Refs {
                package: Some("package-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"title\":\"delivery\",\"dependencies\":[\"foundation\"],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"high\",\"write_scope\":{\"v\":1,\"paths\":[\"src\"]},\"review_policy\":{\"v\":1,\"kind\":\"independent\"},\"independence_policy\":{\"v\":1,\"kind\":\"different-role-and-session\"},\"next_action\":\"ready\"}",
        );
        append_named(
            conn,
            "run-1",
            "package-ready",
            "package_ready",
            Refs {
                package: Some("package-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"next_action\":\"active\"}",
        );
        append_named(
            conn,
            "run-1",
            "package-active",
            "package_active",
            Refs {
                package: Some("package-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"next_action\":\"writer\"}",
        );
        append_named(
            conn,
            "run-1",
            "package-block",
            "package_blocked",
            Refs {
                package: Some("package-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"blocker\":\"capacity\",\"resume_status\":\"active\",\"next_action\":\"unblock\"}",
        );
        append_named(
            conn,
            "run-1",
            "package-unblock",
            "package_unblocked",
            Refs {
                package: Some("package-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"resolution\":\"capacity restored\",\"resume_status\":\"active\",\"next_action\":\"writer\"}",
        );

        append_named(
            conn,
            "run-1",
            "writer-plan",
            "assignment_planned",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                ..Refs::default()
            },
            "{\"v\":1,\"title\":\"implement\",\"sequence\":1,\"assignment_kind\":\"implementation\",\"required_role\":\"worker\",\"model_floor\":\"standard\",\"risk_class\":\"high\",\"write_scope\":{\"v\":1,\"paths\":[\"src\"]},\"base_revision\":\"base\",\"independence_boundary_id\":\"boundary\",\"next_action\":\"queue\"}",
        );
        append_named(
            conn,
            "run-1",
            "writer-queue",
            "assignment_queued",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                ..Refs::default()
            },
            "{\"v\":1,\"next_action\":\"attempt\"}",
        );
        append_named(
            conn,
            "run-1",
            "writer-attempt-1-plan",
            "attempt_planned",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"attempt_sequence\":1,\"next_action\":\"route\"}",
        );
        append_named(
            conn,
            "run-1",
            "writer-route-1",
            "route_requested",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"route_id\":\"writer-route-1\",\"required_profile\":\"standard\",\"requested_model\":null,\"requested_reasoning\":null,\"escalated_from_route_id\":null,\"decided_at\":\"{at}\",\"next_action\":\"spawn\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-session-plan",
            "session_planned",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                session: Some("writer-session"),
                ..Refs::default()
            },
            "{\"v\":1,\"authorization_ref\":\"writer-auth\",\"budget_reason\":\"writer\",\"run_max_open\":2,\"run_max_total\":4,\"session_token_budget\":{\"mode\":\"unbounded\",\"tokens\":null},\"nested_delegation\":{\"allowed\":false,\"authority_ref\":null},\"requested_host\":null,\"requested_profile\":\"standard\",\"requested_model\":null,\"requested_reasoning\":null,\"parent_session_id\":null}",
        );
        append_named(
            conn,
            "run-1",
            "writer-lease-plan",
            "lease_planned",
            Refs {
                package: Some("package-1"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
                ..Refs::default()
            },
            "{\"v\":1,\"role\":\"worker\",\"model_profile\":\"standard\",\"risk_class\":\"high\",\"write_scope\":{\"v\":1,\"paths\":[\"src\"]},\"base_revision\":\"base\",\"independence_boundary_id\":\"boundary\",\"replaces_session_id\":null,\"expiry_predicate\":null,\"expires_at\":null,\"next_action\":\"issue\"}",
        );
        append_named(
            conn,
            "run-1",
            "writer-spawn",
            "session_spawned",
            Refs {
                session: Some("writer-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"handle\":\"writer-handle\",\"host_id\":\"host\",\"spawned_at\":\"{at}\",\"next_action\":\"route\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-route-degraded",
            "route_degraded",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"actual_model\":null,\"actual_reasoning\":null,\"reason\":\"selector unsupported\",\"eligibility_status\":\"unknown\",\"eligibility_evidence\":{{\"v\":1,\"items\":[]}},\"next_action\":\"inherit\",\"decided_at\":\"{at}\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-route-applied",
            "route_applied",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"routing_status\":\"inherited\",\"actual_model\":null,\"actual_reasoning\":null,\"eligibility_evidence\":{{\"v\":1,\"items\":[{{\"kind\":\"host\",\"ref\":\"host\",\"result\":\"inherited\"}}]}},\"decided_at\":\"{at}\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-lease-issue",
            "lease_issued",
            Refs {
                package: Some("package-1"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
                ..Refs::default()
            },
            &format!("{{\"v\":1,\"issued_at\":\"{at}\",\"next_action\":\"run\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "writer-session-run-1",
            "session_running",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!(
                "{{\"v\":1,\"started_or_resumed_at\":\"{at}\",\"lease_id\":\"writer-lease\",\"attempt_id\":\"writer-attempt-1\",\"prior_report_ref\":null,\"gate_evidence\":null,\"next_action\":\"work\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-attempt-1-start",
            "attempt_started",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!("{{\"v\":1,\"started_at\":\"{at}\",\"next_action\":\"work\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "writer-start-1",
            "assignment_started",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!(
                "{{\"v\":1,\"attempt_id\":\"writer-attempt-1\",\"started_at\":\"{at}\",\"current_step\":\"edit\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-block",
            "assignment_blocked",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"blocker\":\"test failure\",\"next_action\":\"fix\"}",
        );
        append_named(
            conn,
            "run-1",
            "writer-unblock",
            "assignment_unblocked",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"resolution\":\"fixed\",\"next_action\":\"report\"}",
        );
        append_named(
            conn,
            "run-1",
            "writer-report-1",
            "assignment_reported",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                session: Some("writer-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"report_path\":\"/tmp/writer-1.md\",\"reported_at\":\"{at}\",\"next_action\":\"retry\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-session-report-1",
            "session_reported",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                session: Some("writer-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"last_reported_at\":\"{at}\",\"assignment_id\":\"writer\",\"attempt_id\":\"writer-attempt-1\",\"next_action\":\"wait\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-attempt-1-fail",
            "attempt_failed",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!(
                "{{\"v\":1,\"outcome_reason\":\"retryable\",\"ended_at\":\"{at}\",\"next_action\":\"requeue\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-requeue",
            "assignment_requeued",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"reason\":\"retry\",\"next_action\":\"attempt\"}",
        );
        append_named(
            conn,
            "run-1",
            "writer-lease-idle",
            "lease_idle",
            Refs {
                package: Some("package-1"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
                ..Refs::default()
            },
            &format!("{{\"v\":1,\"last_used_at\":\"{at}\",\"next_action\":\"reuse\"}}"),
        );

        append_named(
            conn,
            "run-1",
            "writer-attempt-2-plan",
            "attempt_planned",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                ..Refs::default()
            },
            "{\"v\":1,\"attempt_sequence\":2,\"next_action\":\"route\"}",
        );
        append_named(
            conn,
            "run-1",
            "writer-route-2",
            "route_requested",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"route_id\":\"writer-route-2\",\"required_profile\":\"standard\",\"requested_model\":null,\"requested_reasoning\":null,\"escalated_from_route_id\":null,\"decided_at\":\"{at}\",\"next_action\":\"reuse\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-dispatch-reuse",
            "dispatch_reuse_selected",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            "{\"v\":1,\"session_id\":\"writer-session\",\"lease_id\":\"writer-lease\",\"reason\":\"compatible\",\"authorization_ref\":\"reuse-auth\"}",
        );
        append_named(
            conn,
            "run-1",
            "writer-lease-reuse",
            "lease_reused",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!(
                "{{\"v\":1,\"attempt_id\":\"writer-attempt-2\",\"compatibility_evidence\":{{\"v\":1,\"items\":[{{\"kind\":\"scope\",\"ref\":\"writer-lease\",\"result\":\"compatible\"}}]}},\"last_used_at\":\"{at}\",\"next_action\":\"run\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-session-run-2",
            "session_running",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!(
                "{{\"v\":1,\"started_or_resumed_at\":\"{at}\",\"lease_id\":\"writer-lease\",\"attempt_id\":\"writer-attempt-2\",\"prior_report_ref\":\"/tmp/writer-1.md\",\"gate_evidence\":{evidence},\"next_action\":\"work\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-route-2-applied",
            "route_applied",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"routing_status\":\"applied\",\"actual_model\":\"observed-model\",\"actual_reasoning\":null,\"eligibility_evidence\":{evidence},\"decided_at\":\"{at}\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-attempt-2-start",
            "attempt_started",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!("{{\"v\":1,\"started_at\":\"{at}\",\"next_action\":\"work\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "writer-start-2",
            "assignment_started",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!(
                "{{\"v\":1,\"attempt_id\":\"writer-attempt-2\",\"started_at\":\"{at}\",\"current_step\":\"verify\"}}"
            ),
        );

        for (event_id, source, observation, tokens, supersedes) in [
            ("usage-weak", "inference", "cumulative", 20, "null"),
            (
                "usage-correction",
                "host-runtime",
                "correction",
                10,
                "\"usage-weak\"",
            ),
        ] {
            append_event(conn,&EventInput{event_id:event_id.into(),run_id:"run-1".into(),package_id:None,assignment_id:None,attempt_id:None,session_id:Some("writer-session".into()),lease_id:None,event_type:"usage_observed".into(),source_kind:source.into(),source_id:event_id.into(),confidence:Some(8_000),payload_json:format!("{{\"v\":1,\"scope\":\"session\",\"subject_id\":\"writer-session\",\"observation_kind\":\"{observation}\",\"window_start\":null,\"window_end\":\"{at}\",\"input_tokens\":{tokens},\"output_tokens\":null,\"reasoning_tokens\":null,\"cache_read_tokens\":null,\"cache_write_tokens\":null,\"credits\":null,\"provider_cost\":null,\"telemetry_quality\":\"exact\",\"supersedes_event_id\":{supersedes}}}"),occurred_at:at.into(),idempotency_key:event_id.into()}).unwrap();
        }
        append_named(
            conn,
            "run-1",
            "writer-report-2",
            "assignment_reported",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"report_path\":\"/tmp/writer-2.md\",\"reported_at\":\"{at}\",\"next_action\":\"validate\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-attempt-2-report",
            "attempt_reported",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!("{{\"v\":1,\"reported_at\":\"{at}\",\"next_action\":\"validate\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "writer-session-report-2",
            "session_reported",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"last_reported_at\":\"{at}\",\"assignment_id\":\"writer\",\"attempt_id\":\"writer-attempt-2\",\"next_action\":\"review\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-validate",
            "assignment_validated",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"test_evidence\":{evidence},\"validated_at\":\"{at}\",\"next_action\":\"review\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-attempt-2-validate",
            "attempt_validated",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!("{{\"v\":1,\"validated_at\":\"{at}\",\"next_action\":\"review\"}}"),
        );

        append_named(
            conn,
            "run-1",
            "review-plan",
            "assignment_planned",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                ..Refs::default()
            },
            "{\"v\":1,\"title\":\"review\",\"sequence\":2,\"assignment_kind\":\"review\",\"required_role\":\"reviewer\",\"model_floor\":\"deep\",\"risk_class\":\"high\",\"write_scope\":{\"v\":1,\"paths\":[]},\"base_revision\":\"head\",\"independence_boundary_id\":\"boundary\",\"next_action\":\"queue\"}",
        );
        append_named(
            conn,
            "run-1",
            "review-queue",
            "assignment_queued",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                ..Refs::default()
            },
            "{\"v\":1,\"next_action\":\"attempt\"}",
        );
        append_named(
            conn,
            "run-1",
            "review-attempt-plan",
            "attempt_planned",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                ..Refs::default()
            },
            "{\"v\":1,\"attempt_sequence\":1,\"next_action\":\"route\"}",
        );
        append_named(
            conn,
            "run-1",
            "review-route",
            "route_requested",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"route_id\":\"review-route\",\"required_profile\":\"deep\",\"requested_model\":null,\"requested_reasoning\":null,\"escalated_from_route_id\":null,\"decided_at\":\"{at}\",\"next_action\":\"spawn\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-session-plan",
            "session_planned",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                session: Some("review-session"),
                ..Refs::default()
            },
            "{\"v\":1,\"authorization_ref\":\"review-auth\",\"budget_reason\":\"review\",\"run_max_open\":2,\"run_max_total\":4,\"session_token_budget\":{\"mode\":\"unbounded\",\"tokens\":null},\"nested_delegation\":{\"allowed\":false,\"authority_ref\":null},\"requested_host\":null,\"requested_profile\":\"deep\",\"requested_model\":null,\"requested_reasoning\":null,\"parent_session_id\":null}",
        );
        append_named(
            conn,
            "run-1",
            "review-lease-plan",
            "lease_planned",
            Refs {
                package: Some("package-1"),
                session: Some("review-session"),
                lease: Some("review-lease"),
                ..Refs::default()
            },
            "{\"v\":1,\"role\":\"reviewer\",\"model_profile\":\"deep\",\"risk_class\":\"high\",\"write_scope\":{\"v\":1,\"paths\":[]},\"base_revision\":\"head\",\"independence_boundary_id\":\"boundary\",\"replaces_session_id\":null,\"expiry_predicate\":null,\"expires_at\":null,\"next_action\":\"issue\"}",
        );
        append_named(
            conn,
            "run-1",
            "review-spawn",
            "session_spawned",
            Refs {
                session: Some("review-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"handle\":\"review-handle\",\"host_id\":\"host\",\"spawned_at\":\"{at}\",\"next_action\":\"route\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-route-applied",
            "route_applied",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"routing_status\":\"applied\",\"actual_model\":\"review-model\",\"actual_reasoning\":null,\"eligibility_evidence\":{evidence},\"decided_at\":\"{at}\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-lease-issue",
            "lease_issued",
            Refs {
                package: Some("package-1"),
                session: Some("review-session"),
                lease: Some("review-lease"),
                ..Refs::default()
            },
            &format!("{{\"v\":1,\"issued_at\":\"{at}\",\"next_action\":\"run\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "review-session-run",
            "session_running",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                session: Some("review-session"),
                lease: Some("review-lease"),
            },
            &format!(
                "{{\"v\":1,\"started_or_resumed_at\":\"{at}\",\"lease_id\":\"review-lease\",\"attempt_id\":\"review-attempt\",\"prior_report_ref\":null,\"gate_evidence\":null,\"next_action\":\"review\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-attempt-start",
            "attempt_started",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                session: Some("review-session"),
                lease: Some("review-lease"),
            },
            &format!("{{\"v\":1,\"started_at\":\"{at}\",\"next_action\":\"review\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "review-start",
            "assignment_started",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                session: Some("review-session"),
                lease: Some("review-lease"),
            },
            &format!(
                "{{\"v\":1,\"attempt_id\":\"review-attempt\",\"started_at\":\"{at}\",\"current_step\":\"inspect\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-report",
            "assignment_reported",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                session: Some("review-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"report_path\":\"/tmp/review.md\",\"reported_at\":\"{at}\",\"next_action\":\"validate\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-attempt-report",
            "attempt_reported",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                session: Some("review-session"),
                lease: Some("review-lease"),
            },
            &format!("{{\"v\":1,\"reported_at\":\"{at}\",\"next_action\":\"validate\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "review-session-report",
            "session_reported",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                session: Some("review-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"last_reported_at\":\"{at}\",\"assignment_id\":\"review\",\"attempt_id\":\"review-attempt\",\"next_action\":\"gate\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-validate",
            "assignment_validated",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"test_evidence\":{evidence},\"validated_at\":\"{at}\",\"next_action\":\"gate\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-attempt-validate",
            "attempt_validated",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                session: Some("review-session"),
                lease: Some("review-lease"),
            },
            &format!("{{\"v\":1,\"validated_at\":\"{at}\",\"next_action\":\"gate\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "review-gate",
            "quality_gate_passed",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"subject_kind\":\"assignment\",\"subject_id\":\"review\",\"policy\":{{\"v\":1,\"kind\":\"independent\"}},\"evidence\":{review_evidence},\"observed_at\":\"{at}\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-accept",
            "assignment_accepted",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"review_evidence\":{review_evidence},\"accepted_at\":\"{at}\",\"ended_at\":\"{at}\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-attempt-accept",
            "attempt_accepted",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                attempt: Some("review-attempt"),
                session: Some("review-session"),
                lease: Some("review-lease"),
            },
            &format!("{{\"v\":1,\"accepted_at\":\"{at}\",\"ended_at\":\"{at}\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "package-review-start",
            "package_review_started",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                ..Refs::default()
            },
            "{\"v\":1,\"review_assignment_id\":\"review\",\"next_action\":\"review\"}",
        );
        append_named(
            conn,
            "run-1",
            "package-review-complete",
            "package_review_completed",
            Refs {
                package: Some("package-1"),
                assignment: Some("review"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"verdict\":\"accepted\",\"review_evidence\":{review_evidence},\"next_action\":\"complete\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-gate",
            "quality_gate_passed",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"subject_kind\":\"assignment\",\"subject_id\":\"writer\",\"policy\":{{\"v\":1,\"kind\":\"independent\"}},\"evidence\":{review_evidence},\"observed_at\":\"{at}\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-accept",
            "assignment_accepted",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"review_evidence\":{review_evidence},\"accepted_at\":\"{at}\",\"ended_at\":\"{at}\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "writer-attempt-2-accept",
            "attempt_accepted",
            Refs {
                package: Some("package-1"),
                assignment: Some("writer"),
                attempt: Some("writer-attempt-2"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
            },
            &format!("{{\"v\":1,\"accepted_at\":\"{at}\",\"ended_at\":\"{at}\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "writer-lease-close",
            "lease_closed",
            Refs {
                package: Some("package-1"),
                session: Some("writer-session"),
                lease: Some("writer-lease"),
                ..Refs::default()
            },
            &format!("{{\"v\":1,\"expiry_reason\":\"complete\",\"ended_at\":\"{at}\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "review-lease-close",
            "lease_closed",
            Refs {
                package: Some("package-1"),
                session: Some("review-session"),
                lease: Some("review-lease"),
                ..Refs::default()
            },
            &format!("{{\"v\":1,\"expiry_reason\":\"complete\",\"ended_at\":\"{at}\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "writer-session-close",
            "session_closed",
            Refs {
                session: Some("writer-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"outcome\":\"success\",\"close_disposition\":\"confirmed\",\"closed_at\":\"{at}\",\"ended_at\":\"{at}\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "review-session-close",
            "session_closed",
            Refs {
                session: Some("review-session"),
                ..Refs::default()
            },
            &format!(
                "{{\"v\":1,\"outcome\":\"success\",\"close_disposition\":\"confirmed\",\"closed_at\":\"{at}\",\"ended_at\":\"{at}\"}}"
            ),
        );
        append_named(
            conn,
            "run-1",
            "package-complete",
            "package_completed",
            Refs {
                package: Some("package-1"),
                ..Refs::default()
            },
            &format!("{{\"v\":1,\"ended_at\":\"{at}\"}}"),
        );
        append_named(
            conn,
            "run-1",
            "run-complete",
            "run_completed",
            Refs::default(),
            &format!("{{\"v\":1,\"completed_at\":\"{at}\",\"ended_at\":\"{at}\"}}"),
        );
    }

    fn snapshot(conn: &Connection, table: &str, columns: &str, order: &str) -> Vec<String> {
        let sql = format!("SELECT json_array({columns}) FROM {table} ORDER BY {order}");
        let mut statement = conn.prepare(&sql).unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn replay_reproduces_every_projection_and_provenance_row() {
        let mut source = connection();
        setup_replay_source(&mut source);
        for table in [
            "work_packages",
            "work_package_dependencies",
            "assignments",
            "assignment_attempts",
            "agent_sessions",
            "session_leases",
            "routing_decisions",
            "projection_field_sources",
        ] {
            assert!(
                source
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                        .get::<_, i64>(0))
                    .unwrap()
                    > 0,
                "rich replay fixture left {table} empty"
            );
        }
        for event_type in [
            "route_degraded",
            "attempt_failed",
            "assignment_requeued",
            "lease_reused",
            "quality_gate_passed",
            "package_review_completed",
            "usage_observed",
            "session_closed",
            "package_completed",
            "run_completed",
        ] {
            assert!(
                source
                    .query_row(
                        "SELECT COUNT(*) FROM control_plane_events WHERE event_type=?1",
                        [event_type],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap()
                    > 0,
                "rich replay fixture omitted {event_type}"
            );
        }
        let mut target = connection();
        replay_run_into_empty(&source, &mut target, "run-1").unwrap();
        let tables = [
            (
                "runs",
                "run_id,goal,psoc_revision,status,session_budget,token_budget,token_budget_mode,report_path,ledger_path,next_action,started_at,completed_at,ended_at",
                "run_id",
            ),
            (
                "work_packages",
                "package_id,run_id,title,role_floor,model_floor,risk_floor,write_scope,review_policy,independence_policy,status,blocker,next_action,ended_at",
                "package_id",
            ),
            (
                "work_package_dependencies",
                "package_id,depends_on_package_id",
                "package_id,depends_on_package_id",
            ),
            (
                "assignments",
                "assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,base_revision,independence_boundary_id,current_attempt_id,attempt_count,status,current_step,blocker,next_action,report_path,test_evidence,review_evidence,started_at,reported_at,validated_at,accepted_at,ended_at,final_reason",
                "assignment_id",
            ),
            (
                "assignment_attempts",
                "attempt_id,assignment_id,session_id,lease_id,attempt_sequence,route_id,status,next_action,started_at,reported_at,validated_at,accepted_at,ended_at,outcome_reason,input_tokens,output_tokens,reasoning_tokens,cache_read_tokens,cache_write_tokens,credits_amount,credits_scale,credits_unit,cost_amount,cost_scale,cost_currency,telemetry_source",
                "attempt_id",
            ),
            (
                "agent_sessions",
                "session_id,run_id,handle,parent_handle,host_id,role,requested_profile,requested_model,actual_model,requested_reasoning,actual_reasoning,routing_status,routing_reason,token_budget,token_budget_mode,budget_enforcement,status,next_action,spawned_at,last_activity_at,last_reported_at,last_waited_at,outcome,close_disposition,close_requested_at,closed_at,ended_at,interrupted_at,interruption_reason,superseded_by_session_id,superseded_at,telemetry_quality,input_tokens,output_tokens,reasoning_tokens,cache_read_tokens,cache_write_tokens,credits_amount,credits_scale,credits_unit,cost_amount,cost_scale,cost_currency,telemetry_source,final_reason",
                "session_id",
            ),
            (
                "session_leases",
                "lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,independence_boundary_id,current_attempt_id,replaces_session_id,expiry_predicate,status,reuse_count,next_action,issued_at,last_used_at,expires_at,expiry_reason,ended_at",
                "lease_id",
            ),
            (
                "routing_decisions",
                "route_id,attempt_id,required_profile,requested_model,requested_reasoning,actual_model,actual_reasoning,routing_status,eligibility_status,eligibility_evidence,escalated_from_route_id,next_action,decided_at",
                "route_id",
            ),
            (
                "control_plane_events",
                "event_id,run_id,package_id,assignment_id,attempt_id,session_id,lease_id,sequence,event_type,source_kind,source_id,confidence,payload_json,occurred_at,ingested_at,idempotency_key",
                "sequence",
            ),
            ("run_event_counters", "run_id,next_sequence", "run_id"),
            (
                "projection_field_sources",
                "run_id,entity_kind,entity_id,field_name,winner_event_id,winner_source_kind,winner_sequence,conflict_count,last_conflict_event_id",
                "entity_kind,entity_id,field_name",
            ),
        ];
        for (table, columns, order) in tables {
            assert_eq!(
                snapshot(&target, table, columns, order),
                snapshot(&source, table, columns, order),
                "replay mismatch in {table}"
            );
        }
    }

    #[test]
    fn replay_rejects_sequence_gap() {
        let mut source = connection();
        setup_replay_source(&mut source);
        source
            .execute("DROP TRIGGER trg_control_plane_events_no_delete", [])
            .unwrap();
        source
            .execute(
                "DELETE FROM control_plane_events WHERE run_id='run-1' AND sequence=2",
                [],
            )
            .unwrap();
        let mut target = connection();
        let error = replay_run_into_empty(&source, &mut target, "run-1").unwrap_err();
        assert!(error.contains("sequence gap"), "{error}");
        assert_eq!(
            target
                .query_row("SELECT COUNT(*) FROM runs", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn replay_rejects_nonempty_target() {
        let mut source = connection();
        setup_replay_source(&mut source);
        let mut target = connection();
        append_run(&mut target, "other-event", "other-run", "other");
        let error = replay_run_into_empty(&source, &mut target, "run-1").unwrap_err();
        assert!(error.contains("target must be empty"), "{error}");
    }

    #[test]
    fn replay_rejects_structurally_wrong_current_target() {
        let mut source = connection();
        setup_replay_source(&mut source);
        let mut target = connection();
        target
            .execute("DROP TRIGGER trg_control_plane_events_no_delete", [])
            .unwrap();
        let error = replay_run_into_empty(&source, &mut target, "run-1").unwrap_err();
        assert!(
            error.contains("structural mismatch")
                || error.contains("trg_control_plane_events_no_delete"),
            "{error}"
        );
        assert_eq!(
            target
                .query_row("SELECT COUNT(*) FROM control_plane_events", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            0
        );
    }

    #[test]
    fn replay_is_independent_of_occurred_timestamp_order() {
        let mut source = connection();
        append_run(&mut source, "time-run", "time-run", "time-run");
        let mut started = run_event("time-start", "time-run", "time-start");
        started.event_type = "run_started".into();
        started.payload_json="{\"v\":1,\"psoc_revision\":\"psoc-2\",\"started_at\":\"2026-07-12T09:08:08.006Z\",\"next_action\":\"work\"}".into();
        started.occurred_at = "2020-01-01T00:00:00.000Z".into();
        append_event(&mut source, &started).unwrap();
        let mut target = connection();
        replay_run_into_empty(&source, &mut target, "time-run").unwrap();
        assert_eq!(
            target
                .query_row(
                    "SELECT status FROM runs WHERE run_id='time-run'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "active"
        );
    }

    #[test]
    fn legacy_import_is_not_fabricated_into_replay_events() {
        let mut source = connection();
        append_run(&mut source, "event-run", "run-1", "event-run");
        source.execute("INSERT INTO agent_sessions(session_id,handle,role,token_budget_mode,status) VALUES('legacy','legacy','explorer','unknown','closed')",[]).unwrap();
        source.execute("INSERT INTO legacy_agent_ledger_import(legacy_row_key,session_id,import_status,import_reason,imported_at) VALUES('legacy','legacy','imported','fixture','2026-07-12T09:00:00.000Z')",[]).unwrap();
        let mut target = connection();
        replay_run_into_empty(&source, &mut target, "run-1").unwrap();
        assert_eq!(
            target
                .query_row(
                    "SELECT COUNT(*) FROM legacy_agent_ledger_import",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            target
                .query_row(
                    "SELECT COUNT(*) FROM agent_sessions WHERE session_id='legacy'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }

    fn direct_active_package(conn: &Connection, package_id: &str, review_policy: &str) {
        conn.execute("UPDATE runs SET status='active' WHERE run_id='run-1'", [])
            .unwrap();
        let review_policy = format!("{{\"v\":1,\"kind\":\"{review_policy}\"}}");
        conn.execute("INSERT INTO work_packages(package_id,run_id,title,role_floor,model_floor,risk_floor,write_scope,review_policy,independence_policy,status) VALUES(?1,'run-1','package','worker','standard','medium','{\"v\":1,\"paths\":[]}',?2,'{\"v\":1,\"kind\":\"none\"}','active')",[package_id,&review_policy]).unwrap();
    }

    #[test]
    fn reported_is_not_validated_or_accepted() {
        let mut conn = connection();
        append_run(&mut conn, "state-run", "run-1", "state-run");
        direct_active_package(&conn, "package-1", "none");
        conn.execute("INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,attempt_count,status) VALUES('assignment-1','package-1','work',1,'implementation','worker','standard','medium','{\"v\":1,\"paths\":[]}',0,'running')",[]).unwrap();
        append_named(
            &mut conn,
            "run-1",
            "reported",
            "assignment_reported",
            Refs {
                package: Some("package-1"),
                assignment: Some("assignment-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"report_path\":\"/tmp/report.md\",\"reported_at\":\"2026-07-12T09:11:00.000Z\",\"next_action\":\"validate\"}",
        );
        let row:(String,Option<String>,Option<String>)=conn.query_row("SELECT status,test_evidence,review_evidence FROM assignments WHERE assignment_id='assignment-1'",[],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        assert_eq!(row, ("reported".into(), None, None));
        let before = conn
            .query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        let event=EventInput { event_id:"premature-accept".into(),run_id:"run-1".into(),package_id:Some("package-1".into()),assignment_id:Some("assignment-1".into()),attempt_id:None,session_id:None,lease_id:None,event_type:"assignment_accepted".into(),source_kind:"controller-observation".into(),source_id:"controller".into(),confidence:None,payload_json:"{\"v\":1,\"review_evidence\":{\"v\":1,\"items\":[]},\"accepted_at\":\"2026-07-12T09:11:01.000Z\",\"ended_at\":\"2026-07-12T09:11:01.000Z\"}".into(),occurred_at:"2026-07-12T09:11:01.000Z".into(),idempotency_key:"premature".into() };
        assert!(
            append_event(&mut conn, &event)
                .unwrap_err()
                .contains("illegal transition")
        );
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            before
        );
    }

    #[test]
    fn independent_assignment_acceptance_requires_prior_gate_evidence() {
        let mut conn = connection();
        append_run(&mut conn, "independent-run", "run-1", "independent-run");
        direct_active_package(&conn, "package-1", "independent");
        conn.execute(
            "INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,attempt_count,status,test_evidence) VALUES('assignment-1','package-1','work',1,'implementation','worker','standard','medium','{\"v\":1,\"paths\":[]}',0,'validated','{\"v\":1,\"items\":[]}')",
            [],
        )
        .unwrap();
        let accepted = |event_id: &str| {
            EventInput {
            event_id: event_id.into(),
            run_id: "run-1".into(),
            package_id: Some("package-1".into()),
            assignment_id: Some("assignment-1".into()),
            attempt_id: None,
            session_id: None,
            lease_id: None,
            event_type: "assignment_accepted".into(),
            source_kind: "controller-observation".into(),
            source_id: "controller".into(),
            confidence: None,
            payload_json: "{\"v\":1,\"review_evidence\":{\"v\":1,\"items\":[{\"kind\":\"review\",\"ref\":\"review-report\",\"result\":\"pass\"}]},\"accepted_at\":\"2026-07-12T09:20:00.000Z\",\"ended_at\":\"2026-07-12T09:20:00.000Z\"}".into(),
            occurred_at: "2026-07-12T09:20:00.000Z".into(),
            idempotency_key: event_id.into(),
        }
        };
        let before = conn
            .query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        assert!(append_event(&mut conn, &accepted("accept-without-gate")).is_err());
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            before
        );

        append_named(
            &mut conn,
            "run-1",
            "independent-gate",
            "quality_gate_passed",
            Refs {
                package: Some("package-1"),
                assignment: Some("assignment-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"subject_kind\":\"assignment\",\"subject_id\":\"assignment-1\",\"policy\":{\"v\":1,\"kind\":\"independent\"},\"evidence\":{\"v\":1,\"items\":[{\"kind\":\"review\",\"ref\":\"review-report\",\"result\":\"pass\"}]},\"observed_at\":\"2026-07-12T09:20:01.000Z\"}",
        );
        append_event(&mut conn, &accepted("accept-after-gate")).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT status FROM assignments WHERE assignment_id='assignment-1'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "accepted"
        );
    }

    #[test]
    fn package_review_completion_requires_accepted_owned_review_assignment() {
        let mut conn = connection();
        append_run(&mut conn, "review-guard-run", "run-1", "review-guard-run");
        direct_active_package(&conn, "package-1", "independent");
        conn.execute("INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,attempt_count,status) VALUES('review-1','package-1','review',1,'review','reviewer','deep','high','{\"v\":1,\"paths\":[]}',0,'planned')",[]).unwrap();
        append_named(
            &mut conn,
            "run-1",
            "review-guard-start",
            "package_review_started",
            Refs {
                package: Some("package-1"),
                assignment: Some("review-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"review_assignment_id\":\"review-1\",\"next_action\":\"review\"}",
        );
        let completion = EventInput {
            event_id: "review-guard-complete".into(),
            run_id: "run-1".into(),
            package_id: Some("package-1".into()),
            assignment_id: Some("review-1".into()),
            attempt_id: None,
            session_id: None,
            lease_id: None,
            event_type: "package_review_completed".into(),
            source_kind: "controller-observation".into(),
            source_id: "controller".into(),
            confidence: None,
            payload_json: "{\"v\":1,\"verdict\":\"accepted\",\"review_evidence\":{\"v\":1,\"items\":[{\"kind\":\"review\",\"ref\":\"review-report\",\"result\":\"pass\"}]},\"next_action\":\"complete\"}".into(),
            occurred_at: "2026-07-12T09:20:00.000Z".into(),
            idempotency_key: "review-guard-complete".into(),
        };
        for status in ["planned", "reported", "validated"] {
            conn.execute(
                "UPDATE assignments SET status=?2 WHERE assignment_id='review-1'",
                ["review-1", status],
            )
            .unwrap();
            assert!(
                append_event(&mut conn, &completion).is_err(),
                "status={status}"
            );
        }
    }

    #[test]
    fn session_running_requires_active_lease_and_nonterminal_attempt() {
        for lease_status in ["planned", "idle", "expired", "revoked", "closed"] {
            let mut conn = connection();
            setup_planned_attempt(&mut conn);
            conn.execute("INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('session-1','run-1','worker','unbounded','spawned')",[]).unwrap();
            conn.execute("INSERT INTO session_leases(lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,status,reuse_count) VALUES('lease-1','session-1','package-1','worker','standard','medium','{\"v\":1,\"paths\":[]}','base',?1,0)",[lease_status]).unwrap();
            let event = session_running_event("run-invalid-lease");
            let before = conn
                .query_row(
                    "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap();
            assert!(
                append_event(&mut conn, &event).is_err(),
                "lease_status={lease_status}"
            );
            assert_eq!(
                conn.query_row(
                    "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
                before
            );
        }

        for attempt_status in ["accepted", "failed", "cancelled"] {
            let mut conn = connection();
            setup_planned_attempt(&mut conn);
            conn.execute(
                "UPDATE assignment_attempts SET status=?1 WHERE attempt_id='attempt-1'",
                [attempt_status],
            )
            .unwrap();
            conn.execute("INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('session-1','run-1','worker','unbounded','spawned')",[]).unwrap();
            conn.execute("INSERT INTO session_leases(lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,status,reuse_count) VALUES('lease-1','session-1','package-1','worker','standard','medium','{\"v\":1,\"paths\":[]}','base','active',0)",[]).unwrap();
            assert!(
                append_event(&mut conn, &session_running_event("run-terminal-attempt")).is_err(),
                "attempt_status={attempt_status}"
            );
        }
    }

    fn session_running_event(event_id: &str) -> EventInput {
        EventInput {
            event_id: event_id.into(),
            run_id: "run-1".into(),
            package_id: Some("package-1".into()),
            assignment_id: Some("assignment-1".into()),
            attempt_id: Some("attempt-1".into()),
            session_id: Some("session-1".into()),
            lease_id: Some("lease-1".into()),
            event_type: "session_running".into(),
            source_kind: "controller-observation".into(),
            source_id: "controller".into(),
            confidence: None,
            payload_json: "{\"v\":1,\"started_or_resumed_at\":\"2026-07-12T09:21:00.000Z\",\"lease_id\":\"lease-1\",\"attempt_id\":\"attempt-1\",\"prior_report_ref\":null,\"gate_evidence\":null,\"next_action\":\"run\"}".into(),
            occurred_at: "2026-07-12T09:21:00.000Z".into(),
            idempotency_key: event_id.into(),
        }
    }

    #[test]
    fn session_blocked_facet_requires_balanced_transitions() {
        let mut conn = connection();
        append_run(&mut conn, "block-run", "run-1", "block-run");
        conn.execute("INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('session-1','run-1','worker','unbounded','running')",[]).unwrap();
        let facet = |event_id: &str, event_type: &str, payload: &str| EventInput {
            event_id: event_id.into(),
            run_id: "run-1".into(),
            package_id: None,
            assignment_id: None,
            attempt_id: None,
            session_id: Some("session-1".into()),
            lease_id: None,
            event_type: event_type.into(),
            source_kind: "controller-observation".into(),
            source_id: "controller".into(),
            confidence: None,
            payload_json: payload.into(),
            occurred_at: "2026-07-12T09:22:00.000Z".into(),
            idempotency_key: event_id.into(),
        };
        let unblock_payload = "{\"v\":1,\"resolution\":\"clear\",\"next_action\":\"continue\"}";
        let block_payload = "{\"v\":1,\"blocker\":\"waiting\",\"next_action\":\"wait\"}";
        assert!(
            append_event(
                &mut conn,
                &facet("unblock-before-block", "session_unblocked", unblock_payload)
            )
            .is_err()
        );
        append_event(
            &mut conn,
            &facet("block-1", "session_blocked", block_payload),
        )
        .unwrap();
        assert!(
            append_event(
                &mut conn,
                &facet("block-2", "session_blocked", block_payload)
            )
            .is_err()
        );
        append_event(
            &mut conn,
            &facet("unblock-1", "session_unblocked", unblock_payload),
        )
        .unwrap();
        assert!(
            append_event(
                &mut conn,
                &facet("unblock-2", "session_unblocked", unblock_payload)
            )
            .is_err()
        );
    }

    #[test]
    fn package_review_verdict_controls_resume_or_completion() {
        let mut conn = connection();
        append_run(&mut conn, "review-run", "run-1", "review-run");
        direct_active_package(&conn, "package-1", "independent");
        conn.execute("INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,attempt_count,status) VALUES('review-1','package-1','review',1,'review','reviewer','deep','high','{\"v\":1,\"paths\":[]}',0,'planned')",[]).unwrap();
        conn.execute("INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('review-session','run-1','reviewer','unbounded','closed')",[]).unwrap();
        conn.execute("INSERT INTO assignment_attempts(attempt_id,assignment_id,session_id,attempt_sequence,status) VALUES('review-attempt','review-1','review-session',1,'accepted')",[]).unwrap();
        conn.execute("UPDATE assignments SET status='accepted',report_path='/tmp/review.md',test_evidence='{\"v\":1,\"items\":[]}',review_evidence='{\"v\":1,\"items\":[{\"kind\":\"independence\",\"ref\":\"review-session\",\"result\":\"pass\"}]}',current_attempt_id='review-attempt' WHERE assignment_id='review-1'",[]).unwrap();
        let start = |conn: &mut Connection, id: &str| {
            append_named(
                conn,
                "run-1",
                id,
                "package_review_started",
                Refs {
                    package: Some("package-1"),
                    assignment: Some("review-1"),
                    ..Refs::default()
                },
                "{\"v\":1,\"review_assignment_id\":\"review-1\",\"next_action\":\"review\"}",
            )
        };
        start(&mut conn, "review-start-1");
        append_named(
            &mut conn,
            "run-1",
            "changes",
            "package_review_completed",
            Refs {
                package: Some("package-1"),
                assignment: Some("review-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"verdict\":\"changes_required\",\"review_evidence\":{\"v\":1,\"items\":[{\"kind\":\"report\",\"ref\":\"/tmp/review.md\",\"result\":\"changes-required\"}]},\"next_action\":\"fix\"}",
        );
        assert_eq!(
            conn.query_row(
                "SELECT status FROM work_packages WHERE package_id='package-1'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "active"
        );
        start(&mut conn, "review-start-2");
        append_named(
            &mut conn,
            "run-1",
            "accepted-review",
            "package_review_completed",
            Refs {
                package: Some("package-1"),
                assignment: Some("review-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"verdict\":\"accepted\",\"review_evidence\":{\"v\":1,\"items\":[{\"kind\":\"report\",\"ref\":\"/tmp/review.md\",\"result\":\"accepted\"}]},\"next_action\":\"complete\"}",
        );
        append_named(
            &mut conn,
            "run-1",
            "package-done",
            "package_completed",
            Refs {
                package: Some("package-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"ended_at\":\"2026-07-12T09:12:00.000Z\"}",
        );
        assert_eq!(
            conn.query_row(
                "SELECT status FROM work_packages WHERE package_id='package-1'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "complete"
        );
    }

    #[test]
    fn assignment_requeue_requires_failed_current_attempt() {
        let mut conn = connection();
        append_run(&mut conn, "requeue-run", "run-1", "requeue-run");
        direct_active_package(&conn, "package-1", "none");
        conn.execute("INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,current_attempt_id,attempt_count,status) VALUES('assignment-1','package-1','work',1,'implementation','worker','standard','medium','{\"v\":1,\"paths\":[]}','attempt-1',1,'running')",[]).unwrap_err();
        conn.execute("INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,attempt_count,status) VALUES('assignment-1','package-1','work',1,'implementation','worker','standard','medium','{\"v\":1,\"paths\":[]}',1,'running')",[]).unwrap();
        conn.execute("INSERT INTO assignment_attempts(attempt_id,assignment_id,attempt_sequence,status) VALUES('attempt-1','assignment-1',1,'running')",[]).unwrap();
        conn.execute("UPDATE assignments SET current_attempt_id='attempt-1' WHERE assignment_id='assignment-1'",[]).unwrap();
        let mut event = EventInput {
            event_id: "requeue-bad".into(),
            run_id: "run-1".into(),
            package_id: Some("package-1".into()),
            assignment_id: Some("assignment-1".into()),
            attempt_id: Some("attempt-1".into()),
            session_id: None,
            lease_id: None,
            event_type: "assignment_requeued".into(),
            source_kind: "controller-observation".into(),
            source_id: "controller".into(),
            confidence: None,
            payload_json: "{\"v\":1,\"reason\":\"retry\",\"next_action\":\"queue\"}".into(),
            occurred_at: "2026-07-12T09:13:00.000Z".into(),
            idempotency_key: "requeue-bad".into(),
        };
        assert!(
            append_event(&mut conn, &event)
                .unwrap_err()
                .contains("state running")
        );
        conn.execute(
            "UPDATE assignment_attempts SET status='failed' WHERE attempt_id='attempt-1'",
            [],
        )
        .unwrap();
        event.event_id = "requeue-good".into();
        event.idempotency_key = "requeue-good".into();
        append_event(&mut conn, &event).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT status FROM assignments WHERE assignment_id='assignment-1'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "queued"
        );
    }

    #[test]
    fn interrupted_and_superseded_are_facets_not_states() {
        let mut conn = connection();
        append_run(&mut conn, "facet-run", "run-1", "facet-run");
        conn.execute("UPDATE runs SET status='active' WHERE run_id='run-1'", [])
            .unwrap();
        for (id, status) in [("session-1", "running"), ("session-2", "planned")] {
            conn.execute("INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES(?1,'run-1','worker','unbounded',?2)",[id,status]).unwrap();
        }
        append_named(
            &mut conn,
            "run-1",
            "interrupted",
            "session_interrupted",
            Refs {
                session: Some("session-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"interruption_reason\":\"recover\",\"interrupted_at\":\"2026-07-12T09:14:00.000Z\",\"next_action\":\"replace\"}",
        );
        append_named(
            &mut conn,
            "run-1",
            "superseded",
            "session_superseded",
            Refs {
                session: Some("session-1"),
                ..Refs::default()
            },
            "{\"v\":1,\"superseded_by_session_id\":\"session-2\",\"superseded_at\":\"2026-07-12T09:14:01.000Z\",\"reason\":\"replacement\",\"next_action\":\"close\"}",
        );
        assert_eq!(conn.query_row("SELECT status,interruption_reason,superseded_by_session_id FROM agent_sessions WHERE session_id='session-1'",[],|row|Ok((row.get::<_,String>(0)?,row.get::<_,Option<String>>(1)?,row.get::<_,Option<String>>(2)?))).unwrap(),("running".into(),Some("recover".into()),Some("session-2".into())));
    }

    #[test]
    fn lease_expired_revoked_closed_order_is_exact() {
        let mut conn = connection();
        append_run(&mut conn, "lease-run", "run-1", "lease-run");
        direct_active_package(&conn, "package-1", "none");
        conn.execute("INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('session-1','run-1','worker','unbounded','running')",[]).unwrap();
        conn.execute("INSERT INTO session_leases(lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,status,reuse_count) VALUES('lease-1','session-1','package-1','worker','standard','medium','{\"v\":1,\"paths\":[]}','base','active',0)",[]).unwrap();
        conn.execute("INSERT INTO session_leases(lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,status,reuse_count) VALUES('lease-2','session-1','package-1','worker','standard','medium','{\"v\":1,\"paths\":[]}','base','planned',0)",[]).unwrap();
        let refs = || Refs {
            package: Some("package-1"),
            session: Some("session-1"),
            lease: Some("lease-1"),
            ..Refs::default()
        };
        append_named(
            &mut conn,
            "run-1",
            "expired",
            "lease_expired",
            refs(),
            "{\"v\":1,\"expiry_reason\":\"timeout\",\"ended_at\":\"2026-07-12T09:15:00.000Z\",\"next_action\":\"close\"}",
        );
        let bad=EventInput { event_id:"revoked-after-expired".into(),run_id:"run-1".into(),package_id:Some("package-1".into()),assignment_id:None,attempt_id:None,session_id:Some("session-1".into()),lease_id:Some("lease-1".into()),event_type:"lease_revoked".into(),source_kind:"controller-observation".into(),source_id:"controller".into(),confidence:None,payload_json:"{\"v\":1,\"expiry_reason\":\"cancel\",\"ended_at\":\"2026-07-12T09:15:01.000Z\",\"next_action\":\"close\"}".into(),occurred_at:"2026-07-12T09:15:01.000Z".into(),idempotency_key:"bad-revoke".into()};
        assert!(append_event(&mut conn, &bad).is_err());
        append_named(
            &mut conn,
            "run-1",
            "closed",
            "lease_closed",
            refs(),
            "{\"v\":1,\"expiry_reason\":\"cleanup\",\"ended_at\":\"2026-07-12T09:15:02.000Z\"}",
        );
        assert_eq!(
            conn.query_row(
                "SELECT status FROM session_leases WHERE lease_id='lease-1'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "closed"
        );
    }

    #[test]
    fn one_usable_lease_is_enforced_during_reduce() {
        let mut conn = connection();
        append_run(&mut conn, "usable-run", "run-1", "usable-run");
        direct_active_package(&conn, "package-1", "none");
        conn.execute("INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('session-1','run-1','worker','unbounded','running')",[]).unwrap();
        conn.execute("INSERT INTO session_leases(lease_id,session_id,package_id,role,model_profile,risk_class,write_scope,base_revision,status,reuse_count) VALUES('lease-active','session-1','package-1','worker','standard','medium','{\"v\":1,\"paths\":[]}','base','active',0)",[]).unwrap();
        append_named(
            &mut conn,
            "run-1",
            "lease-plan",
            "lease_planned",
            Refs {
                package: Some("package-1"),
                session: Some("session-1"),
                lease: Some("lease-new"),
                ..Refs::default()
            },
            "{\"v\":1,\"role\":\"worker\",\"model_profile\":\"standard\",\"risk_class\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"base_revision\":\"base\",\"independence_boundary_id\":null,\"replaces_session_id\":null,\"expiry_predicate\":null,\"expires_at\":null,\"next_action\":\"issue\"}",
        );
        let before = conn
            .query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        let issue = EventInput {
            event_id: "lease-issue".into(),
            run_id: "run-1".into(),
            package_id: Some("package-1".into()),
            assignment_id: None,
            attempt_id: None,
            session_id: Some("session-1".into()),
            lease_id: Some("lease-new".into()),
            event_type: "lease_issued".into(),
            source_kind: "controller-observation".into(),
            source_id: "controller".into(),
            confidence: None,
            payload_json:
                "{\"v\":1,\"issued_at\":\"2026-07-12T09:16:00.000Z\",\"next_action\":\"run\"}"
                    .into(),
            occurred_at: "2026-07-12T09:16:00.000Z".into(),
            idempotency_key: "lease-issue".into(),
        };
        assert!(append_event(&mut conn, &issue).is_err());
        assert_eq!(
            conn.query_row(
                "SELECT status FROM session_leases WHERE lease_id='lease-new'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "planned"
        );
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            before
        );
    }

    #[test]
    fn circular_projection_pointers_preserve_ownership() {
        let (mut conn, event) = executable_transition_fixture("session_running", "spawned", true);
        append_event(&mut conn, &event).unwrap();
        assert_eq!(conn.query_row(
            "SELECT a.current_attempt_id,aa.assignment_id,aa.session_id,aa.lease_id,aa.route_id,l.session_id,l.package_id,l.current_attempt_id,r.attempt_id FROM assignments a JOIN assignment_attempts aa ON aa.attempt_id=a.current_attempt_id JOIN session_leases l ON l.lease_id=aa.lease_id JOIN routing_decisions r ON r.route_id=aa.route_id WHERE a.assignment_id='matrix-assignment'",
            [],
            |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?,row.get::<_,String>(6)?,row.get::<_,String>(7)?,row.get::<_,String>(8)?)),
        ).unwrap(),(
            "matrix-attempt".into(),"matrix-assignment".into(),"matrix-session".into(),"matrix-lease".into(),"matrix-route".into(),"matrix-session".into(),"matrix-package".into(),"matrix-attempt".into(),"matrix-attempt".into()
        ));
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );

        let (mut conn, mut cross_owner) =
            executable_transition_fixture("session_running", "spawned", true);
        cross_owner.event_id = "cross-owner-pointer".into();
        cross_owner.session_id = Some("matrix-replacement".into());
        cross_owner.idempotency_key = "cross-owner-pointer".into();
        let before = event_count_and_counter(&conn, "matrix-run");
        let error = append_event(&mut conn, &cross_owner).unwrap_err();
        assert!(
            error.contains("lease") && error.contains("ownership"),
            "{error}"
        );
        assert_eq!(event_count_and_counter(&conn, "matrix-run"), before);
        assert_eq!(
            conn.query_row("SELECT session_id,lease_id FROM assignment_attempts WHERE attempt_id='matrix-attempt'",[],|row|Ok((row.get::<_,Option<String>>(0)?,row.get::<_,Option<String>>(1)?))).unwrap(),
            (None, None)
        );
    }

    #[test]
    fn package_dependencies_reject_cross_run_self_and_cycle() {
        let mut conn = connection();
        append_run(&mut conn, "deps-run", "run-1", "deps-run");
        conn.execute("UPDATE runs SET status='active' WHERE run_id='run-1'", [])
            .unwrap();
        append_named(
            &mut conn,
            "run-1",
            "dep-a",
            "package_planned",
            Refs {
                package: Some("dep-a"),
                ..Refs::default()
            },
            "{\"v\":1,\"title\":\"a\",\"dependencies\":[],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"review_policy\":{\"v\":1,\"kind\":\"none\"},\"independence_policy\":{\"v\":1,\"kind\":\"none\"},\"next_action\":\"done\"}",
        );
        conn.execute(
            "UPDATE work_packages SET status='complete' WHERE package_id='dep-a'",
            [],
        )
        .unwrap();
        append_named(
            &mut conn,
            "run-1",
            "dep-b",
            "package_planned",
            Refs {
                package: Some("dep-b"),
                ..Refs::default()
            },
            "{\"v\":1,\"title\":\"b\",\"dependencies\":[\"dep-a\"],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"review_policy\":{\"v\":1,\"kind\":\"none\"},\"independence_policy\":{\"v\":1,\"kind\":\"none\"},\"next_action\":\"ready\"}",
        );
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM work_package_dependencies WHERE package_id='dep-b' AND depends_on_package_id='dep-a'",[],|row|row.get::<_,i64>(0)).unwrap(),1);
        let self_event=EventInput { event_id:"self".into(),run_id:"run-1".into(),package_id:Some("self".into()),assignment_id:None,attempt_id:None,session_id:None,lease_id:None,event_type:"package_planned".into(),source_kind:"controller-observation".into(),source_id:"controller".into(),confidence:None,payload_json:"{\"v\":1,\"title\":\"self\",\"dependencies\":[\"self\"],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"review_policy\":{\"v\":1,\"kind\":\"none\"},\"independence_policy\":{\"v\":1,\"kind\":\"none\"},\"next_action\":\"bad\"}".into(),occurred_at:"2026-07-12T09:17:00.000Z".into(),idempotency_key:"self".into()};
        assert!(
            append_event(&mut conn, &self_event)
                .unwrap_err()
                .contains("itself")
        );
        append_run(&mut conn, "deps-other-run", "run-2", "deps-other-run");
        append_named(
            &mut conn,
            "run-2",
            "other-package",
            "package_planned",
            Refs {
                package: Some("other-package"),
                ..Refs::default()
            },
            "{\"v\":1,\"title\":\"other\",\"dependencies\":[],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"review_policy\":{\"v\":1,\"kind\":\"none\"},\"independence_policy\":{\"v\":1,\"kind\":\"none\"},\"next_action\":\"done\"}",
        );
        let cross_run = EventInput {
            event_id: "cross-run-dependency".into(),
            run_id: "run-1".into(),
            package_id: Some("cross-run-package".into()),
            assignment_id: None,
            attempt_id: None,
            session_id: None,
            lease_id: None,
            event_type: "package_planned".into(),
            source_kind: "controller-observation".into(),
            source_id: "controller".into(),
            confidence: None,
            payload_json: "{\"v\":1,\"title\":\"cross\",\"dependencies\":[\"other-package\"],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"review_policy\":{\"v\":1,\"kind\":\"none\"},\"independence_policy\":{\"v\":1,\"kind\":\"none\"},\"next_action\":\"reject\"}".into(),
            occurred_at: "2026-07-12T09:17:00.000Z".into(),
            idempotency_key: "cross-run-dependency".into(),
        };
        let before = event_count_and_counter(&conn, "run-1");
        let error = append_event(&mut conn, &cross_run).unwrap_err();
        assert!(error.contains("does not belong to run"), "{error}");
        assert_eq!(event_count_and_counter(&conn, "run-1"), before);

        let mut cycle = connection();
        append_run(&mut cycle, "cycle-run", "cycle-run", "cycle-run");
        cycle.pragma_update(None, "foreign_keys", "OFF").unwrap();
        cycle.execute("INSERT INTO work_packages(package_id,run_id,title,role_floor,model_floor,risk_floor,write_scope,review_policy,independence_policy,status) VALUES('cycle-x','cycle-run','x','worker','standard','medium','{\"v\":1,\"paths\":[]}','{\"v\":1,\"kind\":\"none\"}','{\"v\":1,\"kind\":\"none\"}','planned')",[]).unwrap();
        cycle.execute("INSERT INTO work_package_dependencies(package_id,depends_on_package_id) VALUES('cycle-x','cycle-y')",[]).unwrap();
        cycle.pragma_update(None, "foreign_keys", "ON").unwrap();
        assert_eq!(
            cycle
                .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let cycle_event = EventInput {
            event_id: "cycle-y".into(),
            run_id: "cycle-run".into(),
            package_id: Some("cycle-y".into()),
            assignment_id: None,
            attempt_id: None,
            session_id: None,
            lease_id: None,
            event_type: "package_planned".into(),
            source_kind: "controller-observation".into(),
            source_id: "controller".into(),
            confidence: None,
            payload_json: "{\"v\":1,\"title\":\"y\",\"dependencies\":[\"cycle-x\"],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{\"v\":1,\"paths\":[]},\"review_policy\":{\"v\":1,\"kind\":\"none\"},\"independence_policy\":{\"v\":1,\"kind\":\"none\"},\"next_action\":\"reject\"}".into(),
            occurred_at: "2026-07-12T09:17:00.000Z".into(),
            idempotency_key: "cycle-y".into(),
        };
        let before = event_count_and_counter(&cycle, "cycle-run");
        let error = append_event(&mut cycle, &cycle_event).unwrap_err();
        assert!(error.contains("would create a cycle"), "{error}");
        assert_eq!(event_count_and_counter(&cycle, "cycle-run"), before);
        assert_eq!(
            cycle
                .query_row(
                    "SELECT COUNT(*) FROM work_packages WHERE package_id='cycle-y'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn package_dependencies_require_a_sorted_array() {
        let mut conn = connection();
        append_run(&mut conn, "shape-run", "shape-run", "shape-run");
        for package in ["dep-a", "dep-z"] {
            append_named(
                &mut conn,
                "shape-run",
                package,
                "package_planned",
                Refs {
                    package: Some(package),
                    ..Refs::default()
                },
                &format!(
                    "{{\"v\":1,\"title\":\"{package}\",\"dependencies\":[],\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{{\"v\":1,\"paths\":[]}},\"review_policy\":{{\"v\":1,\"kind\":\"none\"}},\"independence_policy\":{{\"v\":1,\"kind\":\"none\"}},\"next_action\":\"done\"}}"
                ),
            );
            conn.execute(
                "UPDATE work_packages SET status='complete' WHERE package_id=?1",
                [package],
            )
            .unwrap();
        }
        for (event_id, dependencies) in [
            ("deps-unsorted", "[\"dep-z\",\"dep-a\"]"),
            ("deps-object", "{\"first\":\"dep-a\"}"),
        ] {
            let event = EventInput {
                event_id: event_id.into(),
                run_id: "shape-run".into(),
                package_id: Some(event_id.into()),
                assignment_id: None,
                attempt_id: None,
                session_id: None,
                lease_id: None,
                event_type: "package_planned".into(),
                source_kind: "controller-observation".into(),
                source_id: "controller".into(),
                confidence: None,
                payload_json: format!(
                    "{{\"v\":1,\"title\":\"bad\",\"dependencies\":{dependencies},\"role_floor\":\"worker\",\"model_floor\":\"standard\",\"risk_floor\":\"medium\",\"write_scope\":{{\"v\":1,\"paths\":[]}},\"review_policy\":{{\"v\":1,\"kind\":\"none\"}},\"independence_policy\":{{\"v\":1,\"kind\":\"none\"}},\"next_action\":\"reject\"}}"
                ),
                occurred_at: "2026-07-12T09:18:00.000Z".into(),
                idempotency_key: event_id.into(),
            };
            let before = conn
                .query_row(
                    "SELECT next_sequence FROM run_event_counters WHERE run_id='shape-run'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap();
            assert!(append_event(&mut conn, &event).is_err());
            assert_eq!(
                conn.query_row(
                    "SELECT next_sequence FROM run_event_counters WHERE run_id='shape-run'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
                before
            );
        }
    }

    #[test]
    fn nested_delegation_allowed_requires_json_boolean() {
        let mut conn = connection();
        setup_planned_attempt(&mut conn);
        let event = EventInput {
            event_id: "numeric-delegation".into(),
            run_id: "run-1".into(),
            package_id: Some("package-1".into()),
            assignment_id: Some("assignment-1".into()),
            attempt_id: Some("attempt-1".into()),
            session_id: Some("numeric-session".into()),
            lease_id: None,
            event_type: "session_planned".into(),
            source_kind: "controller-observation".into(),
            source_id: "controller".into(),
            confidence: None,
            payload_json: "{\"v\":1,\"authorization_ref\":\"auth\",\"budget_reason\":\"reason\",\"run_max_open\":2,\"run_max_total\":4,\"session_token_budget\":{\"mode\":\"unbounded\",\"tokens\":null},\"nested_delegation\":{\"allowed\":0,\"authority_ref\":null},\"requested_host\":null,\"requested_profile\":\"standard\",\"requested_model\":null,\"requested_reasoning\":null,\"parent_session_id\":null}".into(),
            occurred_at: "2026-07-12T09:18:00.000Z".into(),
            idempotency_key: "numeric-delegation".into(),
        };
        let before = conn
            .query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        let error = append_event(&mut conn, &event).unwrap_err();
        assert!(error.contains("boolean"), "{error}");
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            before
        );
    }

    fn setup_provenance_subjects(conn: &mut Connection) {
        append_run(
            conn,
            "provenance-run-event",
            "provenance-run",
            "provenance-run",
        );
        conn.execute(
            "INSERT INTO work_packages(package_id,run_id,title,role_floor,model_floor,risk_floor,write_scope,review_policy,independence_policy,status) VALUES('provenance-package','provenance-run','provenance','worker','standard','medium','{\"v\":1,\"paths\":[]}','{\"v\":1,\"kind\":\"none\"}','{\"v\":1,\"kind\":\"none\"}','active')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO assignments(assignment_id,package_id,title,sequence,assignment_kind,required_role,model_floor,risk_class,write_scope,attempt_count,status) VALUES('provenance-assignment','provenance-package','provenance',1,'implementation','worker','standard','medium','{\"v\":1,\"paths\":[]}',0,'validated')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('provenance-session','provenance-run','worker','unbounded','running')",
            [],
        )
        .unwrap();
    }

    fn heartbeat_claim(event_id: &str, source_kind: &str, last_activity_at: &str) -> EventInput {
        EventInput {
            event_id: event_id.into(),
            run_id: "provenance-run".into(),
            package_id: None,
            assignment_id: None,
            attempt_id: None,
            session_id: Some("provenance-session".into()),
            lease_id: None,
            event_type: "session_heartbeat".into(),
            source_kind: source_kind.into(),
            source_id: format!("source-{event_id}"),
            confidence: Some(8_000),
            payload_json: format!("{{\"v\":1,\"last_activity_at\":\"{last_activity_at}\"}}"),
            occurred_at: "2026-07-12T09:20:00.000Z".into(),
            idempotency_key: event_id.into(),
        }
    }

    fn quality_gate_claim(event_id: &str, source_kind: &str, result: &str) -> EventInput {
        EventInput {
            event_id: event_id.into(),
            run_id: "provenance-run".into(),
            package_id: Some("provenance-package".into()),
            assignment_id: Some("provenance-assignment".into()),
            attempt_id: None,
            session_id: None,
            lease_id: None,
            event_type: "quality_gate_passed".into(),
            source_kind: source_kind.into(),
            source_id: format!("source-{event_id}"),
            confidence: Some(8_000),
            payload_json: format!(
                "{{\"v\":1,\"subject_kind\":\"assignment\",\"subject_id\":\"provenance-assignment\",\"policy\":{{\"v\":1,\"kind\":\"deterministic\"}},\"evidence\":{{\"v\":1,\"items\":[{{\"kind\":\"test\",\"ref\":\"suite\",\"result\":\"{result}\"}}]}},\"observed_at\":\"2026-07-12T09:20:00.000Z\"}}"
            ),
            occurred_at: "2026-07-12T09:20:00.000Z".into(),
            idempotency_key: event_id.into(),
        }
    }

    #[test]
    fn authoritative_events_reject_agent_report_and_inference_sources() {
        let mut conn = connection();
        setup_provenance_subjects(&mut conn);
        let before = conn
            .query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='provenance-run'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();

        let mut accepted = EventInput {
            event_id: "unauthorized-accept".into(),
            run_id: "provenance-run".into(),
            package_id: Some("provenance-package".into()),
            assignment_id: Some("provenance-assignment".into()),
            attempt_id: None,
            session_id: None,
            lease_id: None,
            event_type: "assignment_accepted".into(),
            source_kind: "agent-report".into(),
            source_id: "writer".into(),
            confidence: Some(10_000),
            payload_json: "{\"v\":1,\"review_evidence\":{\"v\":1,\"items\":[]},\"accepted_at\":\"2026-07-12T09:20:00.000Z\",\"ended_at\":\"2026-07-12T09:20:00.000Z\"}".into(),
            occurred_at: "2026-07-12T09:20:00.000Z".into(),
            idempotency_key: "unauthorized-accept".into(),
        };
        let error = append_event(&mut conn, &accepted).unwrap_err();
        assert!(error.contains("unauthorized source"), "{error}");

        accepted.event_id = "unauthorized-close".into();
        accepted.package_id = None;
        accepted.assignment_id = None;
        accepted.session_id = Some("provenance-session".into());
        accepted.event_type = "session_closed".into();
        accepted.source_kind = "inference".into();
        accepted.source_id = "observer".into();
        accepted.payload_json = "{\"v\":1,\"outcome\":\"unknown\",\"close_disposition\":\"observed\",\"closed_at\":\"2026-07-12T09:20:00.000Z\",\"ended_at\":\"2026-07-12T09:20:00.000Z\"}".into();
        accepted.idempotency_key = "unauthorized-close".into();
        let error = append_event(&mut conn, &accepted).unwrap_err();
        assert!(error.contains("unauthorized source"), "{error}");

        let error = append_event(
            &mut conn,
            &quality_gate_claim("unauthorized-gate", "agent-report", "passed"),
        )
        .unwrap_err();
        assert!(error.contains("unauthorized source"), "{error}");
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='provenance-run'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            before
        );
        assert_eq!(
            conn.query_row(
                "SELECT status FROM assignments WHERE assignment_id='provenance-assignment'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "validated"
        );
        assert_eq!(
            conn.query_row(
                "SELECT status FROM agent_sessions WHERE session_id='provenance-session'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "running"
        );
    }

    #[test]
    fn lifecycle_field_provenance_uses_deterministic_winner_and_conflicts() {
        let mut conn = connection();
        setup_provenance_subjects(&mut conn);
        for claim in [
            heartbeat_claim(
                "heartbeat-agent",
                "agent-report",
                "2026-07-12T09:20:30.000Z",
            ),
            heartbeat_claim(
                "heartbeat-host-correction",
                "host-runtime",
                "2026-07-12T09:20:10.000Z",
            ),
            heartbeat_claim(
                "heartbeat-corroboration",
                "host-runtime",
                "2026-07-12T09:20:10.000Z",
            ),
            heartbeat_claim(
                "heartbeat-host-later",
                "host-runtime",
                "2026-07-12T09:20:20.000Z",
            ),
            heartbeat_claim(
                "heartbeat-agent-weaker",
                "agent-report",
                "2026-07-12T09:20:40.000Z",
            ),
        ] {
            append_event(&mut conn, &claim).unwrap();
        }
        assert_eq!(
            conn.query_row(
                "SELECT last_activity_at FROM agent_sessions WHERE session_id='provenance-session'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "2026-07-12T09:20:20.000Z"
        );
        assert_eq!(conn.query_row("SELECT winner_event_id,winner_source_kind,conflict_count,last_conflict_event_id FROM projection_field_sources WHERE entity_kind='session' AND entity_id='provenance-session' AND field_name='last_activity_at'",[],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)?,row.get::<_,Option<String>>(3)?))).unwrap(),("heartbeat-host-later".into(),"host-runtime".into(),3,Some("heartbeat-agent-weaker".into())));

        let error = append_event(
            &mut conn,
            &heartbeat_claim(
                "heartbeat-inference",
                "inference",
                "2026-07-12T09:20:50.000Z",
            ),
        )
        .unwrap_err();
        assert!(error.contains("unauthorized source"), "{error}");
    }

    #[test]
    fn quality_gate_provenance_uses_deterministic_winner_and_conflicts() {
        let mut conn = connection();
        setup_provenance_subjects(&mut conn);
        for claim in [
            quality_gate_claim("gate-controller", "controller-observation", "controller"),
            quality_gate_claim("gate-host", "host-runtime", "host"),
            quality_gate_claim("gate-corroboration", "host-runtime", "host"),
            quality_gate_claim("gate-controller-weaker", "controller-observation", "weaker"),
            quality_gate_claim("gate-host-later", "host-runtime", "latest"),
        ] {
            append_event(&mut conn, &claim).unwrap();
        }
        assert_eq!(
            conn.query_row(
                "SELECT test_evidence FROM assignments WHERE assignment_id='provenance-assignment'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "{\"v\":1,\"items\":[{\"kind\":\"test\",\"ref\":\"suite\",\"result\":\"latest\"}]}"
        );
        assert_eq!(conn.query_row("SELECT winner_event_id,winner_source_kind,conflict_count,last_conflict_event_id FROM projection_field_sources WHERE entity_kind='assignment' AND entity_id='provenance-assignment' AND field_name='test_evidence'",[],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)?,row.get::<_,Option<String>>(3)?))).unwrap(),("gate-host-later".into(),"host-runtime".into(),3,Some("gate-host-later".into())));
    }

    #[test]
    fn concurrent_run_local_sequences_are_unique_and_gap_free() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "harnessctl-event-concurrency-{}-{nonce}.db",
            std::process::id()
        ));
        let path_text = path.to_str().unwrap().to_string();
        let mut setup = open_db(&path_text).unwrap();
        append_run(&mut setup, "event-run", "run-1", "event-run");
        setup
            .execute(
                "INSERT INTO agent_sessions(session_id,run_id,role,token_budget_mode,status) VALUES('session-1','run-1','worker','unbounded','running')",
                [],
            )
            .unwrap();
        drop(setup);

        let count = 8;
        let barrier = Arc::new(Barrier::new(count + 1));
        let mut threads = Vec::new();
        for index in 0..count {
            let barrier = barrier.clone();
            let path = path_text.clone();
            threads.push(std::thread::spawn(move || {
                let mut conn = open_db(&path).unwrap();
                let event = EventInput {
                    event_id: format!("heartbeat-{index}"),
                    run_id: "run-1".into(),
                    package_id: None,
                    assignment_id: None,
                    attempt_id: None,
                    session_id: Some("session-1".into()),
                    lease_id: None,
                    event_type: "session_heartbeat".into(),
                    source_kind: "host-runtime".into(),
                    source_id: format!("host-{index}"),
                    confidence: None,
                    payload_json: format!(
                        "{{\"v\":1,\"last_activity_at\":\"2026-07-12T09:08:{:02}.006Z\"}}",
                        20 + index
                    ),
                    occurred_at: "2026-07-12T09:08:19.006Z".into(),
                    idempotency_key: format!("heartbeat-{index}"),
                };
                barrier.wait();
                append_event(&mut conn, &event).map(|result| result.sequence)
            }));
        }
        barrier.wait();
        let mut allocated = threads
            .into_iter()
            .map(|thread| thread.join().unwrap().unwrap())
            .collect::<Vec<_>>();
        allocated.sort_unstable();
        assert_eq!(allocated, (2..=count as i64 + 1).collect::<Vec<_>>());
        let conn = open_db(&path_text).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT next_sequence FROM run_event_counters WHERE run_id='run-1'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            count as i64 + 2
        );
        drop(conn);
        for suffix in ["", "-wal", "-shm"] {
            let _ = fs::remove_file(format!("{path_text}{suffix}"));
        }
    }
}
