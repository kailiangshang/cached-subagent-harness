use crate::domain::{
    ActivityInput, ActivityRecord, Profile, Role, RouteDemand, RunRecord, SessionInput,
    SessionRecord, SessionSignature, SessionStatus, StoreSnapshot, TaskInput, TaskRecord,
    TaskStatus, UsageInput, UsageRecord,
};
use crate::routing::required_profile;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use std::collections::BTreeSet;
use std::path::Path;

const SCHEMA_VERSION: i32 = 1;

const SCHEMA: &str = r#"
CREATE TABLE runs (
    run_id TEXT PRIMARY KEY CHECK(length(run_id) > 0),
    goal TEXT NOT NULL CHECK(length(goal) > 0),
    status TEXT NOT NULL CHECK(status IN ('active','complete','failed','cancelled')),
    repo_root TEXT NOT NULL CHECK(length(repo_root) > 0),
    report_path TEXT NOT NULL CHECK(length(report_path) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    ended_at TEXT
);

CREATE TABLE tasks (
    task_id TEXT PRIMARY KEY CHECK(length(task_id) > 0),
    run_id TEXT NOT NULL,
    package_key TEXT NOT NULL CHECK(length(package_key) > 0),
    title TEXT NOT NULL CHECK(length(title) > 0),
    sequence INTEGER NOT NULL CHECK(typeof(sequence)='integer' AND sequence >= 1),
    role TEXT NOT NULL CHECK(role IN ('discussion','explorer','worker','reviewer','fixer')),
    complexity TEXT NOT NULL CHECK(complexity IN ('light','standard','deep')),
    risk TEXT NOT NULL CHECK(risk IN ('low','medium','high','critical')),
    uncertainty TEXT NOT NULL CHECK(uncertainty IN ('light','standard','deep')),
    write_scope TEXT NOT NULL CHECK(json_valid(write_scope) AND json_type(write_scope)='array'),
    scope_hash TEXT NOT NULL CHECK(length(scope_hash) > 0),
    repo_revision TEXT NOT NULL CHECK(length(repo_revision) > 0),
    review_boundary TEXT,
    required_profile TEXT NOT NULL CHECK(required_profile IN ('light','standard','deep')),
    status TEXT NOT NULL CHECK(status IN ('queued','running','blocked','reported','accepted','failed','cancelled')),
    session_id TEXT,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(typeof(attempt_count)='integer' AND attempt_count >= 0),
    reuse_accepted INTEGER NOT NULL DEFAULT 0 CHECK(reuse_accepted IN (0,1)),
    next_action TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    ended_at TEXT,
    UNIQUE(run_id, sequence),
    FOREIGN KEY(run_id) REFERENCES runs(run_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(session_id) REFERENCES sessions(session_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE sessions (
    session_id TEXT PRIMARY KEY CHECK(length(session_id) > 0),
    run_id TEXT NOT NULL,
    host TEXT NOT NULL CHECK(length(host) > 0),
    handle TEXT,
    role TEXT NOT NULL CHECK(role IN ('discussion','explorer','worker','reviewer','fixer')),
    profile TEXT NOT NULL CHECK(profile IN ('light','standard','deep')),
    requested_model TEXT,
    actual_model TEXT,
    routing_status TEXT NOT NULL CHECK(routing_status IN ('requested','applied','unsupported','unknown')),
    package_key TEXT NOT NULL CHECK(length(package_key) > 0),
    scope_hash TEXT NOT NULL CHECK(length(scope_hash) > 0),
    repo_revision TEXT NOT NULL CHECK(length(repo_revision) > 0),
    review_boundary TEXT,
    status TEXT NOT NULL CHECK(status IN ('starting','busy','idle','closed','failed','unknown')),
    current_task_id TEXT,
    reuse_count INTEGER NOT NULL DEFAULT 0 CHECK(typeof(reuse_count)='integer' AND reuse_count >= 0),
    created_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL,
    ended_at TEXT,
    final_reason TEXT,
    FOREIGN KEY(run_id) REFERENCES runs(run_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(current_task_id) REFERENCES tasks(task_id) ON UPDATE RESTRICT ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE usage (
    usage_id TEXT PRIMARY KEY CHECK(length(usage_id) > 0),
    run_id TEXT NOT NULL,
    task_id TEXT,
    session_id TEXT,
    phase TEXT NOT NULL CHECK(phase IN ('bootstrap','context','work','retry','escalation','review','fixer')),
    input_tokens INTEGER CHECK(input_tokens IS NULL OR (typeof(input_tokens)='integer' AND input_tokens >= 0)),
    output_tokens INTEGER CHECK(output_tokens IS NULL OR (typeof(output_tokens)='integer' AND output_tokens >= 0)),
    reasoning_tokens INTEGER CHECK(reasoning_tokens IS NULL OR (typeof(reasoning_tokens)='integer' AND reasoning_tokens >= 0)),
    cache_read_tokens INTEGER CHECK(cache_read_tokens IS NULL OR (typeof(cache_read_tokens)='integer' AND cache_read_tokens >= 0)),
    cache_write_tokens INTEGER CHECK(cache_write_tokens IS NULL OR (typeof(cache_write_tokens)='integer' AND cache_write_tokens >= 0)),
    source TEXT NOT NULL CHECK(length(source) > 0),
    quality TEXT NOT NULL CHECK(quality IN ('exact','partial','estimated','unsupported','unknown')),
    observed_at TEXT NOT NULL,
    FOREIGN KEY(run_id) REFERENCES runs(run_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(task_id) REFERENCES tasks(task_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(session_id) REFERENCES sessions(session_id) ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE TABLE activity (
    activity_id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    task_id TEXT,
    session_id TEXT,
    kind TEXT NOT NULL CHECK(kind IN ('plan','batch','spawn','reuse','route','start','block','report','accept','fail','close')),
    summary TEXT NOT NULL CHECK(length(summary) > 0),
    occurred_at TEXT NOT NULL,
    FOREIGN KEY(run_id) REFERENCES runs(run_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(task_id) REFERENCES tasks(task_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    FOREIGN KEY(session_id) REFERENCES sessions(session_id) ON UPDATE RESTRICT ON DELETE RESTRICT
);

CREATE INDEX tasks_by_run_status ON tasks(run_id,status);
CREATE INDEX sessions_by_run_status ON sessions(run_id,status);
CREATE INDEX usage_by_run_phase ON usage(run_id,phase);
CREATE INDEX activity_by_run_id ON activity(run_id,activity_id);
PRAGMA user_version=1;
"#;

pub(crate) struct Store {
    conn: Connection,
}

impl Store {
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|error| error.to_string())?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|error| error.to_string())?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|error| error.to_string())?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|error| error.to_string())?;
        let version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| error.to_string())?;
        match version {
            0 => conn
                .execute_batch(SCHEMA)
                .map_err(|error| error.to_string())?,
            SCHEMA_VERSION => {}
            other => return Err(format!("unsupported lightweight schema version: {other}")),
        }
        let store = Self { conn };
        let expected = ["activity", "runs", "sessions", "tasks", "usage"]
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        if store.table_names()? != expected {
            return Err("lightweight schema table set does not match version 1".into());
        }
        Ok(store)
    }

    #[cfg(test)]
    pub(crate) fn schema_version(&self) -> Result<i32, String> {
        self.conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| error.to_string())
    }

    pub(crate) fn table_names(&self) -> Result<BTreeSet<String>, String> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .map_err(|error| error.to_string())?;
        statement
            .query_map([], |row| row.get(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|error| error.to_string())
    }

    #[cfg(test)]
    pub(crate) fn foreign_keys_enabled(&self) -> Result<bool, String> {
        self.conn
            .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i32>(0))
            .map(|value| value == 1)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn create_run(
        &mut self,
        run_id: &str,
        goal: &str,
        repo_root: &str,
        report_path: &str,
    ) -> Result<(), String> {
        for (name, value) in [
            ("run_id", run_id),
            ("goal", goal),
            ("repo_root", repo_root),
            ("report_path", report_path),
        ] {
            require_nonempty(name, value)?;
        }
        let transaction = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO runs(run_id,goal,status,repo_root,report_path,created_at,updated_at) VALUES(?1,?2,'active',?3,?4,?5,?5)",
                params![run_id, goal, repo_root, report_path, now(&transaction)?],
            )
            .map_err(|error| error.to_string())?;
        append_activity_tx(
            &transaction,
            &ActivityInput {
                run_id: run_id.into(),
                task_id: None,
                session_id: None,
                kind: "plan".into(),
                summary: "run created".into(),
            },
        )?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub(crate) fn add_task(&mut self, input: &TaskInput) -> Result<(), String> {
        validate_task_input(input)?;
        let scope = serde_json::to_string(&input.write_scope).map_err(|error| error.to_string())?;
        let transaction = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                r#"INSERT INTO tasks(
                    task_id,run_id,package_key,title,sequence,role,complexity,risk,uncertainty,
                    write_scope,scope_hash,repo_revision,review_boundary,required_profile,status,
                    created_at,updated_at
                ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,'queued',?15,?15)"#,
                params![
                    input.task_id,
                    input.run_id,
                    input.package_key,
                    input.title,
                    i64::try_from(input.sequence).map_err(|_| "task sequence is too large")?,
                    input.role.as_str(),
                    input.complexity.as_str(),
                    input.risk.as_str(),
                    input.uncertainty.as_str(),
                    scope,
                    input.scope_hash,
                    input.repo_revision,
                    input.review_boundary,
                    input.required_profile.as_str(),
                    now(&transaction)?,
                ],
            )
            .map_err(|error| error.to_string())?;
        append_activity_tx(
            &transaction,
            &ActivityInput {
                run_id: input.run_id.clone(),
                task_id: Some(input.task_id.clone()),
                session_id: None,
                kind: "plan".into(),
                summary: "task queued".into(),
            },
        )?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub(crate) fn update_task(
        &mut self,
        task_id: &str,
        target: TaskStatus,
        next_action: Option<&str>,
    ) -> Result<(), String> {
        require_nonempty("task_id", task_id)?;
        let transaction = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let (run_id, current): (String, String) = transaction
            .query_row(
                "SELECT run_id,status FROM tasks WHERE task_id=?1",
                [task_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| error.to_string())?;
        let current = current.parse::<TaskStatus>()?;
        if !task_transition_allowed(current, target) {
            return Err(format!(
                "illegal task transition {task_id}: {current} -> {target}"
            ));
        }
        let timestamp = now(&transaction)?;
        let ended_at = matches!(
            target,
            TaskStatus::Accepted | TaskStatus::Failed | TaskStatus::Cancelled
        )
        .then_some(timestamp.as_str());
        transaction
            .execute(
                "UPDATE tasks SET status=?2,next_action=?3,updated_at=?4,ended_at=?5 WHERE task_id=?1",
                params![task_id, target.as_str(), next_action, timestamp, ended_at],
            )
            .map_err(|error| error.to_string())?;
        let (kind, summary) = task_activity(target);
        append_activity_tx(
            &transaction,
            &ActivityInput {
                run_id,
                task_id: Some(task_id.into()),
                session_id: None,
                kind: kind.into(),
                summary: summary.into(),
            },
        )?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub(crate) fn add_session(&mut self, input: &SessionInput) -> Result<(), String> {
        validate_session_input(input)?;
        let transaction = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let timestamp = now(&transaction)?;
        transaction
            .execute(
                r#"INSERT INTO sessions(
                    session_id,run_id,host,handle,role,profile,requested_model,actual_model,
                    routing_status,package_key,scope_hash,repo_revision,review_boundary,status,
                    current_task_id,created_at,last_used_at
                ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?16)"#,
                params![
                    input.session_id,
                    input.run_id,
                    input.host,
                    input.handle,
                    input.role.as_str(),
                    input.profile.as_str(),
                    input.requested_model,
                    input.actual_model,
                    input.routing_status.as_str(),
                    input.package_key,
                    input.scope_hash,
                    input.repo_revision,
                    input.review_boundary,
                    input.status.as_str(),
                    input.current_task_id,
                    timestamp,
                ],
            )
            .map_err(|error| error.to_string())?;
        if let Some(task_id) = &input.current_task_id {
            let changed = transaction
                .execute(
                    "UPDATE tasks SET session_id=?2,updated_at=?3 WHERE task_id=?1 AND run_id=?4",
                    params![task_id, input.session_id, timestamp, input.run_id],
                )
                .map_err(|error| error.to_string())?;
            if changed != 1 {
                return Err("session current task does not belong to run".into());
            }
        }
        append_activity_tx(
            &transaction,
            &ActivityInput {
                run_id: input.run_id.clone(),
                task_id: input.current_task_id.clone(),
                session_id: Some(input.session_id.clone()),
                kind: "spawn".into(),
                summary: "session recorded".into(),
            },
        )?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub(crate) fn update_session(
        &mut self,
        session_id: &str,
        target: SessionStatus,
        current_task_id: Option<&str>,
        final_reason: Option<&str>,
    ) -> Result<(), String> {
        let transaction = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let (run_id, current): (String, String) = transaction
            .query_row(
                "SELECT run_id,status FROM sessions WHERE session_id=?1",
                [session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| error.to_string())?;
        let current = current.parse::<SessionStatus>()?;
        if !session_transition_allowed(current, target) {
            return Err(format!(
                "illegal session transition {session_id}: {current} -> {target}"
            ));
        }
        if matches!(target, SessionStatus::Failed | SessionStatus::Unknown)
            && final_reason.is_none_or(str::is_empty)
        {
            return Err(format!("{target} session requires final_reason"));
        }
        let timestamp = now(&transaction)?;
        let ended_at = matches!(
            target,
            SessionStatus::Closed | SessionStatus::Failed | SessionStatus::Unknown
        )
        .then_some(timestamp.as_str());
        transaction
            .execute(
                "UPDATE sessions SET status=?2,current_task_id=?3,last_used_at=?4,ended_at=?5,final_reason=?6 WHERE session_id=?1",
                params![session_id, target.as_str(), current_task_id, timestamp, ended_at, final_reason],
            )
            .map_err(|error| error.to_string())?;
        append_activity_tx(
            &transaction,
            &ActivityInput {
                run_id,
                task_id: current_task_id.map(str::to_string),
                session_id: Some(session_id.into()),
                kind: if target == SessionStatus::Failed {
                    "fail".into()
                } else {
                    "close".into()
                },
                summary: format!("session {target}"),
            },
        )?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub(crate) fn record_usage(&mut self, input: &UsageInput) -> Result<(), String> {
        validate_usage_input(input)?;
        self.conn
            .execute(
                r#"INSERT INTO usage(
                    usage_id,run_id,task_id,session_id,phase,input_tokens,output_tokens,
                    reasoning_tokens,cache_read_tokens,cache_write_tokens,source,quality,observed_at
                ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)"#,
                params![
                    input.usage_id,
                    input.run_id,
                    input.task_id,
                    input.session_id,
                    input.phase.as_str(),
                    to_i64(input.input_tokens)?,
                    to_i64(input.output_tokens)?,
                    to_i64(input.reasoning_tokens)?,
                    to_i64(input.cache_read_tokens)?,
                    to_i64(input.cache_write_tokens)?,
                    input.source,
                    input.quality.as_str(),
                    now(&self.conn)?,
                ],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub(crate) fn claim_idle_session(
        &mut self,
        run_id: &str,
        task_id: &str,
        signature: &SessionSignature,
    ) -> Result<Option<String>, String> {
        require_nonempty("run_id", run_id)?;
        require_nonempty("task_id", task_id)?;
        let transaction = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let authoritative = transaction
            .query_row(
                r#"SELECT run_id,role,required_profile,package_key,scope_hash,repo_revision,
                           review_boundary,status
                    FROM tasks WHERE task_id=?1"#,
                [task_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .map_err(|error| error.to_string())?;
        let task_role = authoritative.1.parse::<Role>()?;
        let task_profile = authoritative.2.parse::<Profile>()?;
        let compatible = authoritative.0 == run_id
            && authoritative.7 == TaskStatus::Queued.as_str()
            && task_role == signature.role
            && signature.profile >= task_profile
            && authoritative.3 == signature.package_key
            && authoritative.4 == signature.scope_hash
            && authoritative.5 == signature.repo_revision
            && authoritative.6 == signature.review_boundary;
        if !compatible {
            return Err("dispatch signature disagrees with authoritative task".into());
        }
        let session_id = transaction
            .query_row(
                r#"SELECT session_id FROM sessions
                    WHERE run_id=?1 AND host=?2 AND role=?3 AND profile=?4 AND package_key=?5
                      AND scope_hash=?6 AND repo_revision=?7 AND review_boundary IS ?8
                      AND status='idle'
                    ORDER BY last_used_at,session_id LIMIT 1"#,
                params![
                    run_id,
                    signature.host,
                    signature.role.as_str(),
                    signature.profile.as_str(),
                    signature.package_key,
                    signature.scope_hash,
                    signature.repo_revision,
                    signature.review_boundary,
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some(session_id) = session_id else {
            transaction.commit().map_err(|error| error.to_string())?;
            return Ok(None);
        };
        let timestamp = now(&transaction)?;
        let changed = transaction
            .execute(
                "UPDATE sessions SET status='busy',current_task_id=?2,last_used_at=?3 WHERE session_id=?1 AND status='idle'",
                params![session_id, task_id, timestamp],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("idle session claim lost".into());
        }
        let task_changed = transaction
            .execute(
                "UPDATE tasks SET session_id=?2,updated_at=?3 WHERE task_id=?1 AND run_id=?4 AND status='queued'",
                params![task_id, session_id, timestamp, run_id],
            )
            .map_err(|error| error.to_string())?;
        if task_changed != 1 {
            return Err("reused task is missing, active, or belongs to another run".into());
        }
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(Some(session_id))
    }

    pub(crate) fn accept_followup(
        &mut self,
        session_id: &str,
        task_id: &str,
    ) -> Result<(), String> {
        let transaction = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let run_id: String = transaction
            .query_row(
                "SELECT run_id FROM sessions WHERE session_id=?1 AND status='busy' AND current_task_id=?2",
                params![session_id, task_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let accepted = transaction
            .execute(
                "UPDATE tasks SET reuse_accepted=1 WHERE task_id=?1 AND session_id=?2 AND reuse_accepted=0",
                params![task_id, session_id],
            )
            .map_err(|error| error.to_string())?;
        if accepted == 0 {
            let already_accepted: bool = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM tasks WHERE task_id=?1 AND session_id=?2 AND reuse_accepted=1)",
                    params![task_id, session_id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if already_accepted {
                transaction.commit().map_err(|error| error.to_string())?;
                return Ok(());
            }
            return Err("follow-up task is not linked to this session".into());
        }
        transaction
            .execute(
                "UPDATE sessions SET reuse_count=reuse_count+1,last_used_at=?3 WHERE session_id=?1 AND current_task_id=?2",
                params![session_id, task_id, now(&transaction)?],
            )
            .map_err(|error| error.to_string())?;
        append_activity_tx(
            &transaction,
            &ActivityInput {
                run_id,
                task_id: Some(task_id.into()),
                session_id: Some(session_id.into()),
                kind: "reuse".into(),
                summary: "follow-up accepted".into(),
            },
        )?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub(crate) fn release_verified(
        &mut self,
        session_id: &str,
        task_id: &str,
        revision: &str,
    ) -> Result<(), String> {
        require_nonempty("revision", revision)?;
        let transaction = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let changed = transaction
            .execute(
                r#"UPDATE sessions SET status='idle',current_task_id=NULL,repo_revision=?3,
                       last_used_at=?4
                    WHERE session_id=?1 AND current_task_id=?2 AND status='busy'"#,
                params![session_id, task_id, revision, now(&transaction)?],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("session is not busy on the requested task".into());
        }
        transaction.commit().map_err(|error| error.to_string())
    }

    pub(crate) fn append_activity(&mut self, input: &ActivityInput) -> Result<(), String> {
        let transaction = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        append_activity_tx(&transaction, input)?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub(crate) fn snapshot(&self, run_id: &str) -> Result<StoreSnapshot, String> {
        require_nonempty("run_id", run_id)?;
        let run = self
            .conn
            .query_row(
                "SELECT run_id,goal,status,repo_root,report_path FROM runs WHERE run_id=?1",
                [run_id],
                |row| {
                    Ok(RunRecord {
                        run_id: row.get(0)?,
                        goal: row.get(1)?,
                        status: row.get(2)?,
                        repo_root: row.get(3)?,
                        report_path: row.get(4)?,
                    })
                },
            )
            .map_err(|error| error.to_string())?;

        let mut task_statement = self
            .conn
            .prepare(
                r#"SELECT task_id,run_id,package_key,title,sequence,role,complexity,risk,
                           uncertainty,write_scope,scope_hash,repo_revision,review_boundary,
                           required_profile,status,session_id,attempt_count,next_action
                    FROM tasks WHERE run_id=?1 ORDER BY sequence,task_id"#,
            )
            .map_err(|error| error.to_string())?;
        let tasks = task_statement
            .query_map([run_id], |row| {
                let write_scope_json: String = row.get(9)?;
                Ok(TaskRecord {
                    task_id: row.get(0)?,
                    run_id: row.get(1)?,
                    package_key: row.get(2)?,
                    title: row.get(3)?,
                    sequence: from_i64(row.get(4)?, 4)?,
                    role: parse_wire(row.get(5)?, 5)?,
                    complexity: parse_wire(row.get(6)?, 6)?,
                    risk: parse_wire(row.get(7)?, 7)?,
                    uncertainty: parse_wire(row.get(8)?, 8)?,
                    write_scope: serde_json::from_str(&write_scope_json)
                        .map_err(|error| invalid_column(9, error))?,
                    scope_hash: row.get(10)?,
                    repo_revision: row.get(11)?,
                    review_boundary: row.get(12)?,
                    required_profile: parse_wire(row.get(13)?, 13)?,
                    status: parse_wire(row.get(14)?, 14)?,
                    session_id: row.get(15)?,
                    attempt_count: from_i64(row.get(16)?, 16)?,
                    next_action: row.get(17)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        let mut session_statement = self
            .conn
            .prepare(
                r#"SELECT session_id,run_id,host,handle,role,profile,requested_model,actual_model,
                           routing_status,package_key,scope_hash,repo_revision,review_boundary,status,
                           current_task_id,reuse_count,last_used_at,final_reason
                    FROM sessions WHERE run_id=?1 ORDER BY session_id"#,
            )
            .map_err(|error| error.to_string())?;
        let sessions = session_statement
            .query_map([run_id], |row| {
                Ok(SessionRecord {
                    session_id: row.get(0)?,
                    run_id: row.get(1)?,
                    host: row.get(2)?,
                    handle: row.get(3)?,
                    role: parse_wire(row.get(4)?, 4)?,
                    profile: parse_wire(row.get(5)?, 5)?,
                    requested_model: row.get(6)?,
                    actual_model: row.get(7)?,
                    routing_status: parse_wire(row.get(8)?, 8)?,
                    package_key: row.get(9)?,
                    scope_hash: row.get(10)?,
                    repo_revision: row.get(11)?,
                    review_boundary: row.get(12)?,
                    status: parse_wire(row.get(13)?, 13)?,
                    current_task_id: row.get(14)?,
                    reuse_count: from_i64(row.get(15)?, 15)?,
                    last_used_at: row.get(16)?,
                    final_reason: row.get(17)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        let mut usage_statement = self
            .conn
            .prepare(
                r#"SELECT usage_id,run_id,task_id,session_id,phase,input_tokens,output_tokens,
                           reasoning_tokens,cache_read_tokens,cache_write_tokens,source,quality
                    FROM usage WHERE run_id=?1 ORDER BY observed_at,usage_id"#,
            )
            .map_err(|error| error.to_string())?;
        let usage = usage_statement
            .query_map([run_id], |row| {
                Ok(UsageRecord {
                    usage_id: row.get(0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    session_id: row.get(3)?,
                    phase: parse_wire(row.get(4)?, 4)?,
                    input_tokens: optional_u64(row.get(5)?, 5)?,
                    output_tokens: optional_u64(row.get(6)?, 6)?,
                    reasoning_tokens: optional_u64(row.get(7)?, 7)?,
                    cache_read_tokens: optional_u64(row.get(8)?, 8)?,
                    cache_write_tokens: optional_u64(row.get(9)?, 9)?,
                    source: row.get(10)?,
                    quality: parse_wire(row.get(11)?, 11)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        let mut activity_statement = self
            .conn
            .prepare(
                r#"SELECT activity_id,run_id,task_id,session_id,kind,summary,occurred_at
                    FROM activity WHERE run_id=?1 ORDER BY activity_id"#,
            )
            .map_err(|error| error.to_string())?;
        let activity = activity_statement
            .query_map([run_id], |row| {
                Ok(ActivityRecord {
                    activity_id: from_i64(row.get(0)?, 0)?,
                    run_id: row.get(1)?,
                    task_id: row.get(2)?,
                    session_id: row.get(3)?,
                    kind: row.get(4)?,
                    summary: row.get(5)?,
                    occurred_at: row.get(6)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        Ok(StoreSnapshot {
            run,
            tasks,
            sessions,
            usage,
            activity,
        })
    }

    pub(crate) fn task(&self, run_id: &str, task_id: &str) -> Result<TaskRecord, String> {
        self.snapshot(run_id)?
            .tasks
            .into_iter()
            .find(|task| task.task_id == task_id)
            .ok_or_else(|| format!("task not found in run: {task_id}"))
    }

    #[cfg(test)]
    pub(crate) fn clear_activity_for_test(&mut self) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM activity", [])
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub(crate) fn final_audit(&self, run_id: &str) -> Result<(), Vec<String>> {
        let snapshot = self.snapshot(run_id).map_err(|error| vec![error])?;
        let mut errors = Vec::new();
        for task in snapshot.tasks {
            if !matches!(
                task.status,
                TaskStatus::Accepted | TaskStatus::Failed | TaskStatus::Cancelled
            ) {
                errors.push(format!(
                    "task {} is not terminal: {}",
                    task.task_id, task.status
                ));
            }
        }
        for session in snapshot.sessions {
            if !matches!(
                session.status,
                SessionStatus::Closed | SessionStatus::Failed | SessionStatus::Unknown
            ) {
                errors.push(format!(
                    "session {} is not terminal: {}",
                    session.session_id, session.status
                ));
            }
            if matches!(
                session.status,
                SessionStatus::Failed | SessionStatus::Unknown
            ) && session.final_reason.as_deref().is_none_or(str::is_empty)
            {
                errors.push(format!(
                    "session {} requires final_reason",
                    session.session_id
                ));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn require_nonempty(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{name} must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_task_input(input: &TaskInput) -> Result<(), String> {
    for (name, value) in [
        ("task_id", input.task_id.as_str()),
        ("run_id", input.run_id.as_str()),
        ("package_key", input.package_key.as_str()),
        ("title", input.title.as_str()),
        ("scope_hash", input.scope_hash.as_str()),
        ("repo_revision", input.repo_revision.as_str()),
    ] {
        require_nonempty(name, value)?;
    }
    if input.sequence == 0 {
        return Err("task sequence must be at least 1".into());
    }
    if input.write_scope.is_empty() {
        return Err("write_scope must not be empty".into());
    }
    if input.write_scope.iter().any(|path| path.trim().is_empty()) {
        return Err("write_scope entries must not be empty".into());
    }
    let normalized = input.write_scope.iter().cloned().collect::<BTreeSet<_>>();
    if normalized.len() != input.write_scope.len() || normalized.iter().ne(input.write_scope.iter())
    {
        return Err("write_scope must be sorted and unique".into());
    }
    if input
        .review_boundary
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err("review_boundary must not be empty when provided".into());
    }
    let floor = required_profile(&RouteDemand {
        complexity: input.complexity,
        risk: input.risk,
        role: input.role,
        uncertainty: input.uncertainty,
    });
    if input.required_profile < floor {
        return Err(format!(
            "required_profile {} is below safety floor {floor}",
            input.required_profile
        ));
    }
    Ok(())
}

fn validate_session_input(input: &SessionInput) -> Result<(), String> {
    for (name, value) in [
        ("session_id", input.session_id.as_str()),
        ("run_id", input.run_id.as_str()),
        ("host", input.host.as_str()),
        ("package_key", input.package_key.as_str()),
        ("scope_hash", input.scope_hash.as_str()),
        ("repo_revision", input.repo_revision.as_str()),
    ] {
        require_nonempty(name, value)?;
    }
    for (name, value) in [
        ("handle", input.handle.as_deref()),
        ("requested_model", input.requested_model.as_deref()),
        ("actual_model", input.actual_model.as_deref()),
        ("review_boundary", input.review_boundary.as_deref()),
        ("current_task_id", input.current_task_id.as_deref()),
    ] {
        if value.is_some_and(|text| text.trim().is_empty()) {
            return Err(format!("{name} must not be empty when provided"));
        }
    }
    if matches!(input.status, SessionStatus::Failed | SessionStatus::Unknown) {
        return Err(format!(
            "{} session requires final_reason and must be created through a lifecycle transition",
            input.status
        ));
    }
    Ok(())
}

fn validate_usage_input(input: &UsageInput) -> Result<(), String> {
    for (name, value) in [
        ("usage_id", input.usage_id.as_str()),
        ("run_id", input.run_id.as_str()),
        ("source", input.source.as_str()),
    ] {
        require_nonempty(name, value)?;
    }
    for (name, value) in [
        ("task_id", input.task_id.as_deref()),
        ("session_id", input.session_id.as_deref()),
    ] {
        if value.is_some_and(|text| text.trim().is_empty()) {
            return Err(format!("{name} must not be empty when provided"));
        }
    }
    Ok(())
}

fn task_transition_allowed(current: TaskStatus, target: TaskStatus) -> bool {
    matches!(
        (current, target),
        (
            TaskStatus::Queued,
            TaskStatus::Running | TaskStatus::Cancelled
        ) | (
            TaskStatus::Running,
            TaskStatus::Blocked | TaskStatus::Reported | TaskStatus::Failed | TaskStatus::Cancelled
        ) | (
            TaskStatus::Blocked,
            TaskStatus::Running | TaskStatus::Failed | TaskStatus::Cancelled
        ) | (
            TaskStatus::Reported,
            TaskStatus::Running | TaskStatus::Accepted | TaskStatus::Failed | TaskStatus::Cancelled
        )
    )
}

fn session_transition_allowed(current: SessionStatus, target: SessionStatus) -> bool {
    matches!(
        (current, target),
        (
            SessionStatus::Starting,
            SessionStatus::Busy
                | SessionStatus::Idle
                | SessionStatus::Closed
                | SessionStatus::Failed
                | SessionStatus::Unknown
        ) | (
            SessionStatus::Busy,
            SessionStatus::Idle
                | SessionStatus::Closed
                | SessionStatus::Failed
                | SessionStatus::Unknown
        ) | (
            SessionStatus::Idle,
            SessionStatus::Busy
                | SessionStatus::Closed
                | SessionStatus::Failed
                | SessionStatus::Unknown
        )
    )
}

fn task_activity(target: TaskStatus) -> (&'static str, &'static str) {
    match target {
        TaskStatus::Queued => ("plan", "task queued"),
        TaskStatus::Running => ("start", "task running"),
        TaskStatus::Blocked => ("block", "task blocked"),
        TaskStatus::Reported => ("report", "task reported"),
        TaskStatus::Accepted => ("accept", "task accepted"),
        TaskStatus::Failed => ("fail", "task failed"),
        TaskStatus::Cancelled => ("close", "task cancelled"),
    }
}

fn append_activity_tx(transaction: &Transaction<'_>, input: &ActivityInput) -> Result<(), String> {
    for (name, value) in [
        ("run_id", input.run_id.as_str()),
        ("kind", input.kind.as_str()),
        ("summary", input.summary.as_str()),
    ] {
        require_nonempty(name, value)?;
    }
    if !matches!(
        input.kind.as_str(),
        "plan"
            | "batch"
            | "spawn"
            | "reuse"
            | "route"
            | "start"
            | "block"
            | "report"
            | "accept"
            | "fail"
            | "close"
    ) {
        return Err(format!("invalid activity kind: {}", input.kind));
    }
    transaction
        .execute(
            "INSERT INTO activity(run_id,task_id,session_id,kind,summary,occurred_at) VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                input.run_id,
                input.task_id,
                input.session_id,
                input.kind,
                input.summary,
                now(transaction)?,
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn now(connection: &Connection) -> Result<String, String> {
    connection
        .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |row| {
            row.get(0)
        })
        .map_err(|error| error.to_string())
}

fn to_i64(value: Option<u64>) -> Result<Option<i64>, String> {
    value
        .map(|number| i64::try_from(number).map_err(|_| "token count is too large".to_string()))
        .transpose()
}

fn invalid_column(
    index: usize,
    error: impl std::error::Error + Send + Sync + 'static,
) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(error))
}

fn parse_wire<T>(value: String, index: usize) -> rusqlite::Result<T>
where
    T: std::str::FromStr<Err = String>,
{
    value
        .parse()
        .map_err(|error| invalid_column(index, WireValueError(error)))
}

#[derive(Debug)]
struct WireValueError(String);

impl std::fmt::Display for WireValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for WireValueError {}

fn from_i64(value: i64, index: usize) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| invalid_column(index, error))
}

fn optional_u64(value: Option<i64>, index: usize) -> rusqlite::Result<Option<u64>> {
    value.map(|number| from_i64(number, index)).transpose()
}

#[cfg(test)]
mod tests {
    use super::Store;
    use crate::domain::{
        Profile, Risk, Role, RoutingStatus, SessionInput, SessionStatus, TaskInput, TaskStatus,
        UsageInput, UsagePhase, UsageQuality,
    };
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDb(PathBuf);

    impl TestDb {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "harnessctl-{label}-{}-{nonce}.db",
                std::process::id()
            )))
        }
    }

    impl Drop for TestDb {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
            let _ = fs::remove_file(self.0.with_extension("db-shm"));
            let _ = fs::remove_file(self.0.with_extension("db-wal"));
        }
    }

    #[test]
    fn fresh_store_has_only_compact_tables() {
        let db = TestDb::new("compact-schema");
        let store = Store::open(&db.0).unwrap();
        assert_eq!(store.schema_version().unwrap(), 1);
        assert_eq!(
            store.table_names().unwrap(),
            ["activity", "runs", "sessions", "tasks", "usage"]
                .into_iter()
                .map(str::to_string)
                .collect::<BTreeSet<_>>()
        );
        assert!(store.foreign_keys_enabled().unwrap());
    }

    fn seeded_store(label: &str) -> (TestDb, Store) {
        let db = TestDb::new(label);
        let mut store = Store::open(&db.0).unwrap();
        store
            .create_run("run-1", "reduce token churn", "/repo", "/repo/report.md")
            .unwrap();
        store
            .add_task(&TaskInput {
                task_id: "task-1".into(),
                run_id: "run-1".into(),
                package_key: "package-a".into(),
                title: "first task".into(),
                sequence: 1,
                role: Role::Worker,
                complexity: Profile::Standard,
                risk: Risk::Medium,
                uncertainty: Profile::Standard,
                write_scope: vec!["src".into()],
                scope_hash: "scope-a".into(),
                repo_revision: "abc123".into(),
                review_boundary: Some("review-a".into()),
                required_profile: Profile::Standard,
            })
            .unwrap();
        (db, store)
    }

    #[test]
    fn invalid_task_transition_is_atomic() {
        let (_db, mut store) = seeded_store("task-transition");
        store
            .update_task("task-1", TaskStatus::Running, Some("implement"))
            .unwrap();
        store
            .update_task("task-1", TaskStatus::Reported, Some("verify"))
            .unwrap();
        store
            .update_task("task-1", TaskStatus::Accepted, None)
            .unwrap();
        let before = store.snapshot("run-1").unwrap();
        let error = store
            .update_task("task-1", TaskStatus::Running, Some("reopen"))
            .unwrap_err();
        assert!(error.contains("illegal task transition"), "{error}");
        assert_eq!(store.snapshot("run-1").unwrap(), before);
    }

    #[test]
    fn unknown_usage_round_trips_as_none() {
        let (_db, mut store) = seeded_store("unknown-usage");
        store
            .add_session(&SessionInput {
                session_id: "session-1".into(),
                run_id: "run-1".into(),
                host: "codex".into(),
                handle: Some("agent-1".into()),
                role: Role::Worker,
                profile: Profile::Standard,
                requested_model: None,
                actual_model: None,
                routing_status: RoutingStatus::Unknown,
                package_key: "package-a".into(),
                scope_hash: "scope-a".into(),
                repo_revision: "abc123".into(),
                review_boundary: Some("review-a".into()),
                status: SessionStatus::Busy,
                current_task_id: Some("task-1".into()),
            })
            .unwrap();
        store
            .record_usage(&UsageInput {
                usage_id: "usage-1".into(),
                run_id: "run-1".into(),
                task_id: Some("task-1".into()),
                session_id: Some("session-1".into()),
                phase: UsagePhase::Work,
                input_tokens: None,
                output_tokens: None,
                reasoning_tokens: None,
                cache_read_tokens: None,
                cache_write_tokens: None,
                source: "host".into(),
                quality: UsageQuality::Unknown,
            })
            .unwrap();
        let snapshot = store.snapshot("run-1").unwrap();
        assert_eq!(snapshot.usage.len(), 1);
        assert_eq!(snapshot.usage[0].input_tokens, None);
        assert_eq!(snapshot.usage[0].output_tokens, None);
        assert_eq!(snapshot.usage[0].quality, UsageQuality::Unknown);
    }

    #[test]
    fn final_audit_requires_terminal_tasks_and_sessions() {
        let (_db, mut store) = seeded_store("final-audit");
        store
            .add_session(&SessionInput {
                session_id: "session-1".into(),
                run_id: "run-1".into(),
                host: "codex".into(),
                handle: Some("agent-1".into()),
                role: Role::Worker,
                profile: Profile::Standard,
                requested_model: None,
                actual_model: None,
                routing_status: RoutingStatus::Unknown,
                package_key: "package-a".into(),
                scope_hash: "scope-a".into(),
                repo_revision: "abc123".into(),
                review_boundary: Some("review-a".into()),
                status: SessionStatus::Busy,
                current_task_id: Some("task-1".into()),
            })
            .unwrap();
        let errors = store.final_audit("run-1").unwrap_err();
        assert!(errors.iter().any(|error| error.contains("task-1")));
        assert!(errors.iter().any(|error| error.contains("session-1")));

        store
            .update_task("task-1", TaskStatus::Running, Some("implement"))
            .unwrap();
        store
            .update_task("task-1", TaskStatus::Reported, Some("verify"))
            .unwrap();
        store
            .update_task("task-1", TaskStatus::Accepted, None)
            .unwrap();
        store
            .update_session(
                "session-1",
                SessionStatus::Closed,
                None,
                Some("package complete"),
            )
            .unwrap();
        store.final_audit("run-1").unwrap();
    }

    #[test]
    fn terminal_session_cannot_be_created_without_a_final_reason() {
        let (_db, mut store) = seeded_store("terminal-session-reason");
        let error = store
            .add_session(&SessionInput {
                session_id: "session-1".into(),
                run_id: "run-1".into(),
                host: "codex".into(),
                handle: None,
                role: Role::Worker,
                profile: Profile::Standard,
                requested_model: None,
                actual_model: None,
                routing_status: RoutingStatus::Unknown,
                package_key: "package-a".into(),
                scope_hash: "scope-a".into(),
                repo_revision: "abc123".into(),
                review_boundary: Some("review-a".into()),
                status: SessionStatus::Failed,
                current_task_id: None,
            })
            .unwrap_err();
        assert!(error.contains("final_reason"), "{error}");
        assert!(store.snapshot("run-1").unwrap().sessions.is_empty());
    }

    #[test]
    fn task_profile_cannot_be_lower_than_its_safety_floor() {
        let db = TestDb::new("task-profile-floor");
        let mut store = Store::open(&db.0).unwrap();
        store
            .create_run("run-1", "route", "/repo", "/report")
            .unwrap();
        let error = store
            .add_task(&TaskInput {
                task_id: "task-1".into(),
                run_id: "run-1".into(),
                package_key: "p".into(),
                title: "critical review".into(),
                sequence: 1,
                role: Role::Reviewer,
                complexity: Profile::Standard,
                risk: Risk::Critical,
                uncertainty: Profile::Standard,
                write_scope: vec!["none".into()],
                scope_hash: "scope".into(),
                repo_revision: "rev".into(),
                review_boundary: None,
                required_profile: Profile::Light,
            })
            .unwrap_err();
        assert!(error.contains("safety floor"), "{error}");
    }
}
