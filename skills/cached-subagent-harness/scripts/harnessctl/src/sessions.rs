use crate::domain::{DispatchAction, DispatchDecision, DispatchRequest};
use crate::store::Store;

pub(crate) const DEFAULT_MAX_SESSION_REUSES: u64 = 1;
pub(crate) const DEFAULT_MAX_SESSION_EFFECTIVE_TOKENS: u64 = 200_000;

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
    if request.related_ready_count > 1 {
        return Ok(DispatchDecision {
            action: DispatchAction::BatchThenSpawn,
            session_id: None,
            reason_codes: vec!["related_ready_tasks_batch_first".into()],
        });
    }
    let mut reason_codes = Vec::new();
    if request.host_supports_followup
        && let claim = store.claim_idle_session(
            &request.run_id,
            &request.task_id,
            &request.signature,
            request.reuse_budget,
        )?
    {
        if let Some(session_id) = claim.session_id {
            return Ok(DispatchDecision {
                action: DispatchAction::ReuseSession,
                session_id: Some(session_id),
                reason_codes: vec!["compatible_idle_session_within_budget".into()],
            });
        }
        reason_codes.extend(claim.reason_codes);
    } else if !request.host_supports_followup {
        reason_codes.push("followup_unsupported".into());
    }
    if request.isolation_required || request.delegation_value_exceeds_cost {
        reason_codes.push("delegation_value_exceeds_cost".into());
        Ok(DispatchDecision {
            action: DispatchAction::SpawnSession,
            session_id: None,
            reason_codes,
        })
    } else {
        reason_codes.push("spawn_cost_exceeds_value".into());
        Ok(DispatchDecision {
            action: DispatchAction::ExecuteOnMain,
            session_id: None,
            reason_codes,
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
        DispatchAction, DispatchRequest, Profile, ReuseBudget, Risk, Role, RoutingStatus,
        SessionInput, SessionSignature, SessionStatus, TaskInput, UsageInput, UsagePhase,
        UsageQuality,
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

    fn budget(max_accepted_followups: u64, max_effective_tokens: u64) -> ReuseBudget {
        ReuseBudget {
            max_accepted_followups,
            max_effective_tokens,
        }
    }

    fn record_exact_usage(store: &mut Store, session_id: &str, tokens: u64) {
        store
            .record_usage(&UsageInput {
                usage_id: format!("usage-{session_id}"),
                run_id: "run-1".into(),
                task_id: None,
                session_id: Some(session_id.into()),
                phase: UsagePhase::Work,
                input_tokens: Some(tokens),
                output_tokens: Some(0),
                reasoning_tokens: Some(0),
                cache_read_tokens: Some(0),
                cache_write_tokens: Some(0),
                source: "host".into(),
                quality: UsageQuality::Exact,
            })
            .unwrap();
    }

    #[test]
    fn related_ready_work_batches_before_claiming_an_idle_session() {
        let db = TestDb::new();
        let mut store = Store::open(&db.0).unwrap();
        store
            .create_run("run-1", "batch first", "/repo", "/report")
            .unwrap();
        store.add_task(&task("task-1", 1)).unwrap();
        store
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
                status: SessionStatus::Idle,
                current_task_id: None,
            })
            .unwrap();
        record_exact_usage(&mut store, "session-1", 50);

        let decision = decide(
            &mut store,
            &DispatchRequest {
                run_id: "run-1".into(),
                task_id: "task-1".into(),
                signature: signature(),
                trivial: false,
                isolation_required: false,
                related_ready_count: 3,
                delegation_value_exceeds_cost: true,
                host_supports_followup: true,
                reuse_budget: budget(1, 200),
            },
        )
        .unwrap();

        assert_eq!(decision.action, DispatchAction::BatchThenSpawn);
        assert!(
            decision
                .reason_codes
                .contains(&"related_ready_tasks_batch_first".into())
        );
        assert_eq!(
            store.snapshot("run-1").unwrap().sessions[0].status,
            SessionStatus::Idle
        );
    }

    #[test]
    fn reuse_requires_known_usage_and_respects_both_budgets() {
        let db = TestDb::new();
        let mut store = Store::open(&db.0).unwrap();
        store
            .create_run("run-1", "bounded reuse", "/repo", "/report")
            .unwrap();
        store.add_task(&task("task-1", 1)).unwrap();
        store.add_task(&task("task-2", 2)).unwrap();
        store
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
                status: SessionStatus::Idle,
                current_task_id: None,
            })
            .unwrap();

        let mut request = DispatchRequest {
            run_id: "run-1".into(),
            task_id: "task-1".into(),
            signature: signature(),
            trivial: false,
            isolation_required: false,
            related_ready_count: 1,
            delegation_value_exceeds_cost: true,
            host_supports_followup: true,
            reuse_budget: budget(1, 200),
        };
        let unknown = decide(&mut store, &request).unwrap();
        assert_eq!(unknown.action, DispatchAction::SpawnSession);
        assert!(
            unknown
                .reason_codes
                .contains(&"session_usage_unknown".into())
        );

        record_exact_usage(&mut store, "session-1", 50);
        let first = decide(&mut store, &request).unwrap();
        assert_eq!(first.action, DispatchAction::ReuseSession);
        accept_followup(&mut store, "session-1", "task-1").unwrap();
        release_verified(&mut store, "session-1", "task-1", "abc123").unwrap();

        request.task_id = "task-2".into();
        let exhausted = decide(&mut store, &request).unwrap();
        assert_eq!(exhausted.action, DispatchAction::SpawnSession);
        assert!(
            exhausted
                .reason_codes
                .contains(&"reuse_limit_reached".into())
        );

        request.reuse_budget = budget(2, 40);
        let over_tokens = decide(&mut store, &request).unwrap();
        assert_eq!(over_tokens.action, DispatchAction::SpawnSession);
        assert!(
            over_tokens
                .reason_codes
                .contains(&"session_token_budget_exhausted".into())
        );

        request.reuse_budget = budget(2, 50);
        let exactly_exhausted = decide(&mut store, &request).unwrap();
        assert_eq!(exactly_exhausted.action, DispatchAction::SpawnSession);
        assert!(
            exactly_exhausted
                .reason_codes
                .contains(&"session_token_budget_exhausted".into())
        );
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
        record_exact_usage(&mut first, "session-1", 50);
        let request = DispatchRequest {
            run_id: "run-1".into(),
            task_id: "task-1".into(),
            signature: signature(),
            trivial: false,
            isolation_required: false,
            related_ready_count: 1,
            delegation_value_exceeds_cost: true,
            host_supports_followup: true,
            reuse_budget: budget(1, 200),
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
            reuse_budget: budget(1, 200),
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

    #[test]
    fn caller_cannot_relabel_a_worker_task_for_an_incompatible_session() {
        let db = TestDb::new();
        let mut store = Store::open(&db.0).unwrap();
        store
            .create_run("run-1", "authority", "/repo", "/report")
            .unwrap();
        store.add_task(&task("task-1", 1)).unwrap();
        store
            .add_session(&SessionInput {
                session_id: "review-session".into(),
                run_id: "run-1".into(),
                host: "codex".into(),
                handle: None,
                role: Role::Reviewer,
                profile: Profile::Deep,
                requested_model: None,
                actual_model: None,
                routing_status: RoutingStatus::Unknown,
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
            signature: SessionSignature {
                role: Role::Reviewer,
                profile: Profile::Deep,
                ..signature()
            },
            trivial: false,
            isolation_required: false,
            related_ready_count: 1,
            delegation_value_exceeds_cost: true,
            host_supports_followup: true,
            reuse_budget: budget(1, 200),
        };
        let error = decide(&mut store, &request).unwrap_err();
        assert!(error.contains("authoritative task"), "{error}");
    }

    #[test]
    fn accepted_followup_is_idempotent() {
        let db = TestDb::new();
        let mut store = Store::open(&db.0).unwrap();
        store
            .create_run("run-1", "idempotent", "/repo", "/report")
            .unwrap();
        store.add_task(&task("task-1", 1)).unwrap();
        store
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
                status: SessionStatus::Idle,
                current_task_id: None,
            })
            .unwrap();
        record_exact_usage(&mut store, "session-1", 50);
        let request = DispatchRequest {
            run_id: "run-1".into(),
            task_id: "task-1".into(),
            signature: signature(),
            trivial: false,
            isolation_required: false,
            related_ready_count: 1,
            delegation_value_exceeds_cost: true,
            host_supports_followup: true,
            reuse_budget: budget(1, 200),
        };
        decide(&mut store, &request).unwrap();
        accept_followup(&mut store, "session-1", "task-1").unwrap();
        store.clear_activity_for_test().unwrap();
        accept_followup(&mut store, "session-1", "task-1").unwrap();
        assert_eq!(store.snapshot("run-1").unwrap().sessions[0].reuse_count, 1);
    }
}
