use crate::accounting::efficiency_report;
use crate::bundle::DEFAULT_MAX_TASKS_PER_BUNDLE;
use crate::domain::{
    ActivityStatusView, DispatchPolicyStatusView, Language, RunStatusView, SessionStatusView,
    StatusView, TaskStatusView,
};
use crate::sessions::{DEFAULT_MAX_SESSION_EFFECTIVE_TOKENS, DEFAULT_MAX_SESSION_REUSES};
use crate::store::Store;

pub(crate) fn build_status(store: &Store, run_id: &str) -> Result<StatusView, String> {
    let snapshot = store.snapshot(run_id)?;
    let efficiency = efficiency_report(&snapshot);
    let mut recent_activity = snapshot
        .activity
        .iter()
        .map(|activity| ActivityStatusView {
            activity_id: activity.activity_id,
            task_id: activity.task_id.clone(),
            session_id: activity.session_id.clone(),
            kind: activity.kind.clone(),
            summary: activity.summary.clone(),
            occurred_at: activity.occurred_at.clone(),
        })
        .collect::<Vec<_>>();
    recent_activity.reverse();
    recent_activity.truncate(20);
    Ok(StatusView {
        run: RunStatusView {
            run_id: snapshot.run.run_id,
            goal: snapshot.run.goal,
            status: snapshot.run.status,
            updated_at: snapshot.run.updated_at,
        },
        dispatch_policy: DispatchPolicyStatusView {
            max_tasks_per_bundle: DEFAULT_MAX_TASKS_PER_BUNDLE,
            max_accepted_followups: DEFAULT_MAX_SESSION_REUSES,
            max_effective_tokens: DEFAULT_MAX_SESSION_EFFECTIVE_TOKENS,
            increases_require_evidence: true,
        },
        tasks: snapshot
            .tasks
            .into_iter()
            .map(|task| TaskStatusView {
                task_id: task.task_id,
                package_key: task.package_key,
                title: task.title,
                role: task.role,
                required_profile: task.required_profile,
                status: task.status,
                session_id: task.session_id,
            })
            .collect(),
        sessions: snapshot
            .sessions
            .into_iter()
            .map(|session| SessionStatusView {
                session_id: session.session_id,
                host: session.host,
                role: session.role,
                profile: session.profile,
                requested_model: session.requested_model,
                actual_model: session.actual_model,
                routing_status: session.routing_status,
                status: session.status,
                current_task_id: session.current_task_id,
                reuse_count: session.reuse_count,
                last_used_at: session.last_used_at,
            })
            .collect(),
        efficiency,
        recent_activity,
    })
}

pub(crate) fn render_json(view: &StatusView) -> Result<String, String> {
    serde_json::to_string_pretty(view).map_err(|error| error.to_string())
}

pub(crate) fn render_text(view: &StatusView, language: Language) -> String {
    let (goal, tasks, agents, tokens, actions, unknown) = match language {
        Language::ZhCn => ("目标", "任务", "智能体", "Token", "最近动态", "未知"),
        Language::EnUs => (
            "Goal",
            "Tasks",
            "Agents",
            "Tokens",
            "Recent activity",
            "unknown",
        ),
    };
    let total = view
        .efficiency
        .totals
        .total_effective
        .map(|value| value.to_string())
        .unwrap_or_else(|| unknown.into());
    let mut lines = vec![
        format!("{goal}: {}", view.run.goal),
        format!("{tasks}: {}", view.tasks.len()),
    ];
    for task in &view.tasks {
        lines.push(format!(
            "  {}  {}  {}",
            task.task_id, task.status, task.title
        ));
    }
    lines.push(format!("{agents}: {}", view.sessions.len()));
    for session in &view.sessions {
        lines.push(format!(
            "  {}  {}  {}  reuse={}",
            session.session_id, session.status, session.profile, session.reuse_count
        ));
    }
    lines.push(format!("{tokens}: {total}"));
    lines.push(format!("{actions}: {}", view.recent_activity.len()));
    for activity in &view.recent_activity {
        lines.push(format!("  {}  {}", activity.kind, activity.summary));
    }
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::{build_status, render_json, render_text};
    use crate::domain::{Language, Profile, Risk, Role, TaskInput};
    use crate::store::Store;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn text_is_bilingual_and_json_preserves_unknown_tokens() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("harness-status-{nonce}.db"));
        let mut store = Store::open(&path).unwrap();
        store
            .create_run("run-1", "save tokens", "/repo", "/report")
            .unwrap();
        store
            .add_task(&TaskInput {
                task_id: "task-1".into(),
                run_id: "run-1".into(),
                package_key: "p".into(),
                title: "Compact work".into(),
                sequence: 1,
                role: Role::Worker,
                complexity: Profile::Standard,
                risk: Risk::Medium,
                uncertainty: Profile::Standard,
                write_scope: vec!["src".into()],
                scope_hash: "scope".into(),
                repo_revision: "rev".into(),
                review_boundary: None,
                required_profile: Profile::Standard,
            })
            .unwrap();
        let view = build_status(&store, "run-1").unwrap();
        assert!(render_text(&view, Language::ZhCn).contains("任务"));
        assert!(render_text(&view, Language::EnUs).contains("Tasks"));
        let json = render_json(&view).unwrap();
        assert!(json.contains("\"updated_at\":"), "{json}");
        assert!(json.contains("\"dispatch_policy\":"), "{json}");
        assert!(json.contains("\"max_tasks_per_bundle\": 2"), "{json}");
        assert!(json.contains("\"max_accepted_followups\": 1"), "{json}");
        assert!(json.contains("\"max_effective_tokens\": 200000"), "{json}");
        assert!(json.contains("\"phase_totals\":"), "{json}");
        assert!(json.contains("\"total_effective\": null"));
        assert!(json.contains("task-1"));
        assert!(!json.contains("repo_root"));
        assert!(!json.contains("write_scope"));
        assert!(!json.contains("report_path"));
        assert!(!json.contains("\"handle\""));
        drop(store);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("db-shm"));
        let _ = fs::remove_file(path.with_extension("db-wal"));
    }
}
