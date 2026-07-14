use crate::domain::{DispatchAction, DispatchDecision, DispatchRequest};
use crate::store::Store;

pub(crate) fn decide(
    store: &mut Store,
    request: &DispatchRequest,
) -> Result<DispatchDecision, String> {
    if request.trivial && !request.isolation_required {
        return Ok(DispatchDecision {
            action: DispatchAction::ExecuteOnMain,
            session_id: None,
            reason_codes: vec!["trivial_main_execution".into()],
        });
    }
    if request.host_supports_followup
        && let Some(session_id) =
            store.claim_idle_session(&request.run_id, &request.task_id, &request.signature)?
    {
        return Ok(DispatchDecision {
            action: DispatchAction::ReuseSession,
            session_id: Some(session_id),
            reason_codes: vec!["compatible_idle_session".into()],
        });
    }
    if request.related_ready_count > 1 {
        return Ok(DispatchDecision {
            action: DispatchAction::BatchThenSpawn,
            session_id: None,
            reason_codes: vec!["related_ready_tasks".into()],
        });
    }
    if request.isolation_required || request.delegation_value_exceeds_cost {
        Ok(DispatchDecision {
            action: DispatchAction::SpawnSession,
            session_id: None,
            reason_codes: vec!["delegation_value_exceeds_cost".into()],
        })
    } else {
        Ok(DispatchDecision {
            action: DispatchAction::ExecuteOnMain,
            session_id: None,
            reason_codes: vec!["spawn_cost_exceeds_value".into()],
        })
    }
}

pub(crate) fn accept_followup(
    store: &mut Store,
    session_id: &str,
    task_id: &str,
) -> Result<(), String> {
    store.accept_followup(session_id, task_id)
}

pub(crate) fn release_verified(
    store: &mut Store,
    session_id: &str,
    task_id: &str,
    revision: &str,
) -> Result<(), String> {
    store.release_verified(session_id, task_id, revision)
}

#[cfg(test)]
mod tests {
    use super::{accept_followup, decide, release_verified};
    use crate::domain::{
        DispatchAction, DispatchRequest, Profile, Risk, Role, RoutingStatus, SessionInput,
        SessionSignature, SessionStatus, TaskInput,
    };
    use crate::store::Store;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDb(PathBuf);
    impl TestDb {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "harnessctl-session-{}-{nonce}.db",
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

    fn task(task_id: &str, sequence: u64) -> TaskInput {
        TaskInput {
            task_id: task_id.into(),
            run_id: "run-1".into(),
            package_key: "package-a".into(),
            title: task_id.into(),
            sequence,
            role: Role::Worker,
            complexity: Profile::Standard,
            risk: Risk::Medium,
            uncertainty: Profile::Standard,
            write_scope: vec!["src".into()],
            scope_hash: "scope-a".into(),
            repo_revision: "abc123".into(),
            review_boundary: Some("review-a".into()),
            required_profile: Profile::Standard,
        }
    }

    fn signature() -> SessionSignature {
        SessionSignature {
            host: "codex".into(),
            role: Role::Worker,
            profile: Profile::Standard,
            package_key: "package-a".into(),
            scope_hash: "scope-a".into(),
            repo_revision: "abc123".into(),
            review_boundary: Some("review-a".into()),
        }
    }

    #[test]
    fn compatible_idle_session_is_claimed_once_and_counted_after_acceptance() {
        let db = TestDb::new();
        let mut first = Store::open(&db.0).unwrap();
        first
            .create_run("run-1", "reuse", "/repo", "/repo/report")
            .unwrap();
        first.add_task(&task("task-1", 1)).unwrap();
        first.add_task(&task("task-2", 2)).unwrap();
        first
            .add_session(&SessionInput {
                session_id: "session-1".into(),
                run_id: "run-1".into(),
                host: "codex".into(),
                handle: Some("agent-1".into()),
                role: Role::Worker,
                profile: Profile::Standard,
                requested_model: Some("standard".into()),
                actual_model: Some("standard".into()),
                routing_status: RoutingStatus::Applied,
                package_key: "package-a".into(),
                scope_hash: "scope-a".into(),
                repo_revision: "abc123".into(),
                review_boundary: Some("review-a".into()),
                status: SessionStatus::Idle,
                current_task_id: None,
            })
            .unwrap();
        let request = DispatchRequest {
            run_id: "run-1".into(),
            task_id: "task-1".into(),
            signature: signature(),
            trivial: false,
            isolation_required: false,
            related_ready_count: 1,
            delegation_value_exceeds_cost: true,
            host_supports_followup: true,
        };
        let decision = decide(&mut first, &request).unwrap();
        assert_eq!(decision.action, DispatchAction::ReuseSession);
        assert_eq!(decision.session_id.as_deref(), Some("session-1"));

        let mut second = Store::open(&db.0).unwrap();
        let mut second_request = request.clone();
        second_request.task_id = "task-2".into();
        assert_ne!(
            decide(&mut second, &second_request).unwrap().action,
            DispatchAction::ReuseSession
        );
        assert_eq!(first.snapshot("run-1").unwrap().sessions[0].reuse_count, 0);

        accept_followup(&mut first, "session-1", "task-1").unwrap();
        assert_eq!(first.snapshot("run-1").unwrap().sessions[0].reuse_count, 1);
        release_verified(&mut first, "session-1", "task-1", "def456").unwrap();
        let session = &first.snapshot("run-1").unwrap().sessions[0];
        assert_eq!(session.status, SessionStatus::Idle);
        assert_eq!(session.repo_revision, "def456");
    }

    #[test]
    fn trivial_work_stays_on_main_and_related_work_batches() {
        let db = TestDb::new();
        let mut store = Store::open(&db.0).unwrap();
        store
            .create_run("run-1", "decide", "/repo", "/report")
            .unwrap();
        store.add_task(&task("task-1", 1)).unwrap();
        let mut request = DispatchRequest {
            run_id: "run-1".into(),
            task_id: "task-1".into(),
            signature: signature(),
            trivial: true,
            isolation_required: false,
            related_ready_count: 1,
            delegation_value_exceeds_cost: false,
            host_supports_followup: true,
        };
        assert_eq!(
            decide(&mut store, &request).unwrap().action,
            DispatchAction::ExecuteOnMain
        );
        request.trivial = false;
        request.related_ready_count = 3;
        request.delegation_value_exceeds_cost = true;
        assert_eq!(
            decide(&mut store, &request).unwrap().action,
            DispatchAction::BatchThenSpawn
        );
    }
}
