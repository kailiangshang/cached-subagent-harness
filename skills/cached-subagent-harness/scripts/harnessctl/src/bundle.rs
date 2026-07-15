use crate::domain::{TaskBundle, TaskRecord, TaskStatus};

pub(crate) fn compatible_for_batch(left: &TaskRecord, right: &TaskRecord) -> bool {
    left.status == TaskStatus::Queued
        && right.status == TaskStatus::Queued
        && left.session_id.is_none()
        && right.session_id.is_none()
        && left.run_id == right.run_id
        && left.package_key == right.package_key
        && left.role == right.role
        && left.complexity == right.complexity
        && left.risk == right.risk
        && left.uncertainty == right.uncertainty
        && left.write_scope == right.write_scope
        && left.scope_hash == right.scope_hash
        && left.repo_revision == right.repo_revision
        && left.review_boundary == right.review_boundary
        && left.required_profile == right.required_profile
}

pub(crate) fn bundle_ready(tasks: &[TaskRecord]) -> Vec<TaskBundle> {
    let mut ready = tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Queued && task.session_id.is_none())
        .cloned()
        .collect::<Vec<_>>();
    ready.sort_by(|left, right| {
        left.sequence
            .cmp(&right.sequence)
            .then_with(|| left.task_id.cmp(&right.task_id))
    });
    let mut bundles: Vec<TaskBundle> = Vec::new();
    for task in ready {
        if let Some(bundle) = bundles
            .iter_mut()
            .find(|bundle| compatible_for_batch(&bundle.tasks[0], &task))
        {
            bundle.tasks.push(task);
        } else {
            bundles.push(TaskBundle {
                package_key: task.package_key.clone(),
                tasks: vec![task],
            });
        }
    }
    bundles
}

#[cfg(test)]
mod tests {
    use super::{bundle_ready, compatible_for_batch};
    use crate::domain::{Profile, Risk, Role, TaskInput, TaskRecord, TaskStatus};

    fn task(id: &str, sequence: u64) -> TaskRecord {
        let input = TaskInput {
            task_id: id.into(),
            run_id: "run-1".into(),
            package_key: "package-a".into(),
            title: format!("Task {sequence}"),
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
        };
        TaskRecord {
            task_id: input.task_id,
            run_id: input.run_id,
            package_key: input.package_key,
            title: input.title,
            sequence: input.sequence,
            role: input.role,
            complexity: input.complexity,
            risk: input.risk,
            uncertainty: input.uncertainty,
            write_scope: input.write_scope,
            scope_hash: input.scope_hash,
            repo_revision: input.repo_revision,
            review_boundary: input.review_boundary,
            required_profile: input.required_profile,
            status: TaskStatus::Queued,
            session_id: None,
            attempt_count: 0,
            next_action: None,
        }
    }

    #[test]
    fn six_compatible_tasks_form_one_ordered_bundle() {
        let tasks = (1..=6)
            .rev()
            .map(|sequence| task(&format!("task-{sequence}"), sequence))
            .collect::<Vec<_>>();
        let bundles = bundle_ready(&tasks);
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].tasks.len(), 6);
        assert_eq!(bundles[0].tasks[0].sequence, 1);
        assert_eq!(bundles[0].tasks[5].sequence, 6);
    }

    #[test]
    fn incompatible_or_unready_tasks_split() {
        let baseline = task("task-1", 1);
        let mut variants = Vec::new();
        let mut changed = task("task-role", 2);
        changed.role = Role::Reviewer;
        variants.push(changed);
        let mut changed = task("task-profile", 3);
        changed.required_profile = Profile::Deep;
        variants.push(changed);
        let mut changed = task("task-risk", 4);
        changed.risk = Risk::High;
        variants.push(changed);
        let mut changed = task("task-package", 5);
        changed.package_key = "package-b".into();
        variants.push(changed);
        let mut changed = task("task-scope", 6);
        changed.write_scope = vec!["tests".into()];
        variants.push(changed);
        let mut changed = task("task-revision", 7);
        changed.repo_revision = "def456".into();
        variants.push(changed);
        let mut changed = task("task-review", 8);
        changed.review_boundary = Some("review-b".into());
        variants.push(changed);
        let mut changed = task("task-blocked", 9);
        changed.status = TaskStatus::Blocked;
        variants.push(changed);
        let mut changed = task("task-assigned", 10);
        changed.session_id = Some("session-1".into());
        variants.push(changed);

        for variant in variants {
            assert!(!compatible_for_batch(&baseline, &variant));
        }
    }
}
