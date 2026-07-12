use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

const READ_ONLY_ROLES: &[&str] = &["discussion", "explorer", "reviewer"];
const WRITE_ROLES: &[&str] = &["worker", "fixer"];
const ALL_ROLES: &[&str] = &["discussion", "explorer", "worker", "reviewer", "fixer"];
const ALL_STATUSES: &[&str] = &[
    "planned",
    "spawned",
    "running",
    "reported",
    "closed",
    "failed",
    "abandoned",
    "externally-unknown",
];
const EXCEPTION_FINAL_STATUSES: &[&str] = &["failed", "abandoned", "externally-unknown"];

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

fn validate_role(role: &str) -> Result<(), String> {
    if ALL_ROLES.contains(&role) {
        Ok(())
    } else {
        Err(format!("unknown role: {role}"))
    }
}

fn validate_status(status: &str) -> Result<(), String> {
    if ALL_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(format!("unknown status: {status}"))
    }
}

fn validate_write_scope(role: &str, write_scope: &str) -> Result<(), String> {
    validate_role(role)?;
    if READ_ONLY_ROLES.contains(&role) && write_scope != "none" {
        return Err(format!("{role} must use write_scope=none"));
    }
    if WRITE_ROLES.contains(&role) && write_scope == "none" {
        return Err(format!("{role} requires explicit write_scope"));
    }
    Ok(())
}

pub(crate) fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "1" | "true" | "yes" => Ok(true),
        "0" | "false" | "no" => Ok(false),
        _ => Err(format!("invalid boolean: {value}")),
    }
}

pub(crate) fn set_meta(conn: &mut Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO harness_meta(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn get_meta_usize(
    conn: &Connection,
    key: &str,
    default_value: usize,
) -> Result<usize, String> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM harness_meta WHERE key=?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match value {
        Some(text) => text
            .parse::<usize>()
            .map_err(|error| format!("invalid {key} value {text}: {error}")),
        None => Ok(default_value),
    }
}

fn validate_input(input: &AgentInput) -> Result<(), String> {
    if input.handle.is_empty() {
        return Err("handle must be nonempty".to_string());
    }
    validate_status(&input.status)?;
    validate_write_scope(&input.role, &input.write_scope)
}

pub(crate) fn ledger_add(conn: &mut Connection, input: &AgentInput) -> Result<(), String> {
    validate_input(input)?;
    let transaction = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            r#"
            INSERT INTO agent_ledger(
                handle, role, task, status, report_path, spawned_at, waited, closed,
                write_scope, token_risk, final_reason, next_action, updated_at
            )
            VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now'))
            "#,
            params![
                input.handle,
                input.role,
                input.task,
                input.status,
                input.report_path,
                input.spawned_at,
                input.waited as i64,
                input.closed as i64,
                input.write_scope,
                input.token_risk,
                input.final_reason,
                input.next_action,
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            r#"
            INSERT INTO agent_sessions(
                session_id,run_id,handle,role,token_budget_mode,status,spawned_at,final_reason
            ) VALUES(?1,NULL,?1,?2,'unknown',?3,NULLIF(?4,''),NULLIF(?5,''))
            "#,
            params![
                input.handle,
                input.role,
                input.status,
                input.spawned_at,
                input.final_reason,
            ],
        )
        .map_err(|error| error.to_string())?;
    let imported_at = crate::schema::sqlite_now(&transaction).map_err(|error| error.to_string())?;
    transaction
        .execute(
            r#"
            INSERT INTO legacy_agent_ledger_import(
                legacy_row_key,session_id,import_status,import_reason,imported_at
            ) VALUES(?1,?1,'imported','ledger compatibility write has no truthful run ownership',?2)
            "#,
            params![input.handle, imported_at],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn ledger_update(
    conn: &mut Connection,
    handle: &str,
    patch: &AgentPatch,
) -> Result<(), String> {
    let current: Option<(String, String)> = conn
        .query_row(
            "SELECT role, write_scope FROM agent_ledger WHERE handle=?1",
            params![handle],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((role, current_write_scope)) = current else {
        return Err(format!("unknown handle: {handle}"));
    };
    if let Some(status) = &patch.status {
        validate_status(status)?;
    }
    let effective_scope = patch.write_scope.as_deref().unwrap_or(&current_write_scope);
    validate_write_scope(&role, effective_scope)?;

    let automatic_closed =
        matches!(patch.status.as_deref(), Some("closed")) && patch.closed.is_none();
    let closed = patch.closed.or(automatic_closed.then_some(true));
    let transaction = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let changed = transaction
        .execute(
            r#"
            UPDATE agent_ledger SET
                status=COALESCE(?2,status),
                report_path=COALESCE(?3,report_path),
                waited=COALESCE(?4,waited),
                closed=COALESCE(?5,closed),
                write_scope=COALESCE(?6,write_scope),
                token_risk=COALESCE(?7,token_risk),
                final_reason=COALESCE(?8,final_reason),
                next_action=COALESCE(?9,next_action),
                updated_at=datetime('now')
            WHERE handle=?1
            "#,
            params![
                handle,
                patch.status,
                patch.report_path,
                patch.waited.map(i64::from),
                closed.map(i64::from),
                patch.write_scope,
                patch.token_risk,
                patch.final_reason,
                patch.next_action,
            ],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("unknown handle: {handle}"));
    }
    if patch.status.is_some() || patch.final_reason.is_some() {
        let session_changed = transaction
            .execute(
                r#"
                UPDATE agent_sessions SET
                    status=COALESCE(?2,status),
                    final_reason=CASE WHEN ?3 THEN NULLIF(?4,'') ELSE final_reason END
                WHERE session_id=?1
                "#,
                params![
                    handle,
                    patch.status,
                    patch.final_reason.is_some(),
                    patch.final_reason,
                ],
            )
            .map_err(|error| error.to_string())?;
        if session_changed != 1 {
            return Err(format!(
                "missing compatibility session for handle: {handle}"
            ));
        }
    }
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn ledger_audit(
    conn: &Connection,
    mode: &str,
    max_concurrent: usize,
    max_total: usize,
) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();
    let total: usize = conn
        .query_row("SELECT COUNT(*) FROM agent_ledger", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let active: usize = conn
        .query_row(
            r#"
            SELECT COUNT(*) FROM agent_ledger
            WHERE status IN ('planned', 'spawned', 'running', 'reported') AND closed=0
            "#,
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if active > max_concurrent {
        errors.push(format!(
            "active agents {active} exceed max_concurrent {max_concurrent}"
        ));
    }
    if total > max_total {
        errors.push(format!("total agents {total} exceed max_total {max_total}"));
    }
    match mode {
        "budget" => {}
        "final" => {
            let mut statement = conn
                .prepare(
                    "SELECT handle, status, closed, final_reason FROM agent_ledger ORDER BY handle",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|error| error.to_string())?;
            for row in rows {
                let (handle, status, closed, final_reason) =
                    row.map_err(|error| error.to_string())?;
                if status == "closed" {
                    if closed != 1 {
                        errors.push(format!("agent {handle} is not final: {status}"));
                    }
                } else if EXCEPTION_FINAL_STATUSES.contains(&status.as_str()) {
                    if final_reason.trim().is_empty() {
                        errors.push(format!(
                            "agent {handle} is {status} but missing final reason"
                        ));
                    }
                } else {
                    errors.push(format!("agent {handle} is not final: {status}"));
                }
            }
        }
        _ => return Err(format!("unknown audit mode: {mode}")),
    }
    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::{AgentInput, AgentPatch, ledger_add, ledger_audit, ledger_update};
    use crate::schema::initialize_connection;
    use rusqlite::Connection;

    fn connection() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize_connection(&mut conn, false).unwrap();
        conn
    }

    fn input(handle: &str, status: &str) -> AgentInput {
        AgentInput {
            handle: handle.to_string(),
            role: "worker".to_string(),
            task: "任务/compat".to_string(),
            status: status.to_string(),
            report_path: "/tmp/报告.md".to_string(),
            spawned_at: "legacy-spawn-time".to_string(),
            waited: true,
            closed: false,
            write_scope: "src/a.rs,src/b.rs".to_string(),
            token_risk: "bounded".to_string(),
            final_reason: String::new(),
            next_action: "下一步".to_string(),
        }
    }

    fn physical_tuple(
        conn: &Connection,
        handle: &str,
    ) -> Option<(
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        String,
        String,
        String,
        String,
    )> {
        conn.query_row(
            r#"
            SELECT role,task,status,report_path,spawned_at,waited,closed,
                   write_scope,token_risk,final_reason,next_action
            FROM agent_ledger WHERE handle=?1
            "#,
            [handle],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            },
        )
        .ok()
    }

    fn session_tuple(
        conn: &Connection,
        handle: &str,
    ) -> Option<(
        Option<String>,
        Option<String>,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        Option<i64>,
    )> {
        conn.query_row(
            r#"
            SELECT run_id,handle,role,status,spawned_at,final_reason,
                   token_budget_mode,requested_model,input_tokens
            FROM agent_sessions WHERE session_id=?1
            "#,
            [handle],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .ok()
    }

    #[test]
    fn legacy_add_updates_compatibility_and_session_atomically() {
        let mut conn = connection();
        let input = input("agent-1", "running");
        ledger_add(&mut conn, &input).unwrap();
        assert_eq!(
            physical_tuple(&conn, "agent-1"),
            Some((
                "worker".into(),
                "任务/compat".into(),
                "running".into(),
                "/tmp/报告.md".into(),
                "legacy-spawn-time".into(),
                1,
                0,
                "src/a.rs,src/b.rs".into(),
                "bounded".into(),
                "".into(),
                "下一步".into(),
            ))
        );
        assert_eq!(
            session_tuple(&conn, "agent-1"),
            Some((
                None,
                Some("agent-1".into()),
                "worker".into(),
                "running".into(),
                Some("legacy-spawn-time".into()),
                None,
                "unknown".into(),
                None,
                None,
            ))
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM legacy_agent_ledger_import WHERE legacy_row_key='agent-1' AND session_id='agent-1'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn legacy_add_rolls_back_when_session_insert_fails() {
        let mut conn = connection();
        conn.execute(
            "INSERT INTO agent_sessions(session_id,role,token_budget_mode,status) VALUES('agent-1','worker','unknown','planned')",
            [],
        )
        .unwrap();
        let error = ledger_add(&mut conn, &input("agent-1", "running")).unwrap_err();
        assert!(!error.is_empty());
        assert_eq!(physical_tuple(&conn, "agent-1"), None);
        assert_eq!(
            conn.query_row(
                "SELECT status FROM agent_sessions WHERE session_id='agent-1'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            "planned"
        );
    }

    #[test]
    fn legacy_update_validates_before_dual_write() {
        let mut conn = connection();
        ledger_add(&mut conn, &input("agent-1", "running")).unwrap();
        let physical_before = physical_tuple(&conn, "agent-1");
        let session_before = session_tuple(&conn, "agent-1");
        let error = ledger_update(
            &mut conn,
            "agent-1",
            &AgentPatch {
                status: Some("invented".into()),
                report_path: Some("must-not-write".into()),
                ..AgentPatch::default()
            },
        )
        .unwrap_err();
        assert!(error.contains("unknown status"), "{error}");
        assert_eq!(physical_tuple(&conn, "agent-1"), physical_before);
        assert_eq!(session_tuple(&conn, "agent-1"), session_before);
    }

    #[test]
    fn legacy_update_rolls_back_when_session_update_fails() {
        let mut conn = connection();
        ledger_add(&mut conn, &input("agent-1", "running")).unwrap();
        conn.execute_batch(
            r#"
            CREATE TRIGGER force_session_update_failure
            BEFORE UPDATE ON agent_sessions
            BEGIN SELECT RAISE(ABORT, 'forced session failure'); END;
            "#,
        )
        .unwrap();
        let physical_before = physical_tuple(&conn, "agent-1");
        let error = ledger_update(
            &mut conn,
            "agent-1",
            &AgentPatch {
                status: Some("reported".into()),
                report_path: Some("/tmp/new.md".into()),
                ..AgentPatch::default()
            },
        )
        .unwrap_err();
        assert!(error.contains("forced session failure"), "{error}");
        assert_eq!(physical_tuple(&conn, "agent-1"), physical_before);
        assert_eq!(session_tuple(&conn, "agent-1").unwrap().3, "running");
    }

    #[test]
    fn legacy_update_preserves_unknown_handle_and_close_semantics() {
        let mut conn = connection();
        let error = ledger_update(
            &mut conn,
            "missing",
            &AgentPatch {
                status: Some("closed".into()),
                ..AgentPatch::default()
            },
        )
        .unwrap_err();
        assert_eq!(error, "unknown handle: missing");

        ledger_add(&mut conn, &input("agent-1", "reported")).unwrap();
        ledger_update(
            &mut conn,
            "agent-1",
            &AgentPatch {
                status: Some("closed".into()),
                final_reason: Some("完成".into()),
                ..AgentPatch::default()
            },
        )
        .unwrap();
        let physical = physical_tuple(&conn, "agent-1").unwrap();
        assert_eq!(physical.2, "closed");
        assert_eq!(physical.6, 1);
        assert_eq!(physical.9, "完成");
        let session = session_tuple(&conn, "agent-1").unwrap();
        assert_eq!(session.3, "closed");
        assert_eq!(session.5.as_deref(), Some("完成"));
    }

    #[test]
    fn legacy_audit_continues_to_count_physical_rows() {
        let conn = connection();
        conn.execute(
            "INSERT INTO agent_ledger(handle,role,status,write_scope) VALUES('physical-only','explorer','running','none')",
            [],
        )
        .unwrap();
        let errors = ledger_audit(&conn, "budget", 0, 0).unwrap();
        assert!(
            errors.iter().any(|error| error.contains("active agents 1")),
            "{errors:?}"
        );
        assert!(
            errors.iter().any(|error| error.contains("total agents 1")),
            "{errors:?}"
        );
    }

    #[test]
    fn final_audit_rejects_unclosed_reported_agent() {
        let mut conn = connection();
        let mut input = input("agent-1", "reported");
        input.role = "explorer".into();
        input.write_scope = "none".into();
        ledger_add(&mut conn, &input).unwrap();
        let errors = ledger_audit(&conn, "final", 2, 4).unwrap();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("agent agent-1 is not final")),
            "{errors:?}"
        );
    }

    #[test]
    fn final_audit_accepts_closed_agent() {
        let mut conn = connection();
        let mut input = input("agent-1", "closed");
        input.role = "explorer".into();
        input.write_scope = "none".into();
        input.closed = true;
        ledger_add(&mut conn, &input).unwrap();
        let errors = ledger_audit(&conn, "final", 2, 4).unwrap();
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn final_audit_rejects_failed_agent_without_reason() {
        let mut conn = connection();
        let mut input = input("agent-1", "failed");
        input.role = "explorer".into();
        input.write_scope = "none".into();
        ledger_add(&mut conn, &input).unwrap();
        let errors = ledger_audit(&conn, "final", 2, 4).unwrap();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("missing final reason")),
            "{errors:?}"
        );
    }
}
