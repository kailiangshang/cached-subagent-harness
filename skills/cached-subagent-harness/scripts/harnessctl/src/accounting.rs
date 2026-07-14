use crate::domain::{
    EfficiencyReport, StoreSnapshot, TokenTotals, UsagePhase, UsageQuality, UsageRecord,
};

pub(crate) fn efficiency_report(snapshot: &StoreSnapshot) -> EfficiencyReport {
    let input = aggregate(&snapshot.usage, |row| row.input_tokens);
    let output = aggregate(&snapshot.usage, |row| row.output_tokens);
    let reasoning = aggregate(&snapshot.usage, |row| row.reasoning_tokens);
    let cache_read = aggregate(&snapshot.usage, |row| row.cache_read_tokens);
    let cache_write = aggregate(&snapshot.usage, |row| row.cache_write_tokens);
    let total_effective = [input, output, reasoning, cache_read, cache_write]
        .into_iter()
        .try_fold(0_u64, |total, value| total.checked_add(value?));
    let any_known = snapshot.usage.iter().any(|row| {
        row.input_tokens.is_some()
            || row.output_tokens.is_some()
            || row.reasoning_tokens.is_some()
            || row.cache_read_tokens.is_some()
            || row.cache_write_tokens.is_some()
    });
    let quality = if total_effective.is_some()
        && !snapshot.usage.is_empty()
        && snapshot
            .usage
            .iter()
            .all(|row| row.quality == UsageQuality::Exact)
    {
        UsageQuality::Exact
    } else if any_known {
        UsageQuality::Partial
    } else {
        UsageQuality::Unknown
    };
    let reuse_count = snapshot
        .sessions
        .iter()
        .try_fold(0_u64, |total, session| {
            total.checked_add(session.reuse_count)
        })
        .unwrap_or(u64::MAX);
    let spawns = snapshot.sessions.len() as u64;
    let assignments = spawns.checked_add(reuse_count);
    let assignments_per_spawn = (spawns > 0)
        .then(|| assignments.map(|count| count as f64 / spawns as f64))
        .flatten();
    let churn_rate = assignments
        .filter(|count| *count > 0)
        .map(|count| spawns as f64 / count as f64);

    let mut overhead_samples = snapshot
        .usage
        .iter()
        .filter(|row| {
            row.quality == UsageQuality::Exact
                && matches!(row.phase, UsagePhase::Bootstrap | UsagePhase::Context)
        })
        .filter_map(row_total)
        .collect::<Vec<_>>();
    overhead_samples.sort_unstable();
    let estimate_sample_count = overhead_samples.len();
    let estimated_saved_tokens = if estimate_sample_count >= 3 {
        let median = overhead_samples[estimate_sample_count / 2];
        median.checked_mul(reuse_count)
    } else {
        None
    };
    let estimate_quality = if estimated_saved_tokens.is_some() {
        UsageQuality::Estimated
    } else {
        UsageQuality::Unknown
    };

    EfficiencyReport {
        totals: TokenTotals {
            input,
            output,
            reasoning,
            cache_read,
            cache_write,
            total_effective,
            quality,
        },
        assignments_per_spawn,
        churn_rate,
        reuse_count,
        estimated_saved_tokens,
        estimate_sample_count,
        estimate_quality,
    }
}

fn aggregate(rows: &[UsageRecord], value: impl Fn(&UsageRecord) -> Option<u64>) -> Option<u64> {
    if rows.is_empty() {
        return None;
    }
    rows.iter()
        .try_fold(0_u64, |total, row| total.checked_add(value(row)?))
}

fn row_total(row: &UsageRecord) -> Option<u64> {
    [
        row.input_tokens,
        row.output_tokens,
        row.reasoning_tokens,
        row.cache_read_tokens,
        row.cache_write_tokens,
    ]
    .into_iter()
    .try_fold(0_u64, |total, value| total.checked_add(value?))
}

#[cfg(test)]
mod tests {
    use super::efficiency_report;
    use crate::domain::{
        ActivityRecord, Profile, Role, RoutingStatus, RunRecord, SessionRecord, SessionStatus,
        StoreSnapshot, UsagePhase, UsageQuality, UsageRecord,
    };

    fn usage(id: &str, session: &str, phase: UsagePhase, tokens: Option<u64>) -> UsageRecord {
        UsageRecord {
            usage_id: id.into(),
            run_id: "run-1".into(),
            task_id: None,
            session_id: Some(session.into()),
            phase,
            input_tokens: tokens,
            output_tokens: Some(0),
            reasoning_tokens: Some(0),
            cache_read_tokens: Some(0),
            cache_write_tokens: Some(0),
            source: "host".into(),
            quality: UsageQuality::Exact,
        }
    }

    fn snapshot(usage: Vec<UsageRecord>, reuse_count: u64) -> StoreSnapshot {
        let sessions = ["s1", "s2", "s3"]
            .into_iter()
            .map(|id| SessionRecord {
                session_id: id.into(),
                run_id: "run-1".into(),
                host: "codex".into(),
                handle: Some(id.into()),
                role: Role::Worker,
                profile: Profile::Standard,
                requested_model: None,
                actual_model: None,
                routing_status: RoutingStatus::Unknown,
                package_key: "p".into(),
                scope_hash: "scope".into(),
                repo_revision: "rev".into(),
                review_boundary: None,
                status: SessionStatus::Closed,
                current_task_id: None,
                reuse_count: if id == "s1" { reuse_count } else { 0 },
                final_reason: Some("done".into()),
            })
            .collect();
        StoreSnapshot {
            run: RunRecord {
                run_id: "run-1".into(),
                goal: "measure".into(),
                status: "active".into(),
                repo_root: "/repo".into(),
                report_path: "/report".into(),
            },
            tasks: Vec::new(),
            sessions,
            usage,
            activity: Vec::<ActivityRecord>::new(),
        }
    }

    #[test]
    fn every_usage_phase_contributes_and_missing_data_stays_unknown() {
        let phases = [
            UsagePhase::Bootstrap,
            UsagePhase::Context,
            UsagePhase::Work,
            UsagePhase::Retry,
            UsagePhase::Escalation,
            UsagePhase::Review,
            UsagePhase::Fixer,
        ];
        let rows = phases
            .into_iter()
            .enumerate()
            .map(|(index, phase)| usage(&format!("u{index}"), "s1", phase, Some(10)))
            .collect();
        let report = efficiency_report(&snapshot(rows, 0));
        assert_eq!(report.totals.input, Some(70));
        assert_eq!(report.totals.total_effective, Some(70));
        assert_eq!(report.totals.quality, UsageQuality::Exact);

        let report = efficiency_report(&snapshot(
            vec![usage("unknown", "s1", UsagePhase::Work, None)],
            0,
        ));
        assert_eq!(report.totals.input, None);
        assert_eq!(report.totals.total_effective, None);
        assert_ne!(report.totals.quality, UsageQuality::Exact);
    }

    #[test]
    fn three_exact_overhead_samples_enable_a_median_savings_estimate() {
        let rows = vec![
            usage("u1", "s1", UsagePhase::Bootstrap, Some(100)),
            usage("u2", "s2", UsagePhase::Bootstrap, Some(300)),
            usage("u3", "s3", UsagePhase::Bootstrap, Some(200)),
        ];
        let report = efficiency_report(&snapshot(rows, 2));
        assert_eq!(report.estimated_saved_tokens, Some(400));
        assert_eq!(report.estimate_sample_count, 3);
        assert_eq!(report.estimate_quality, UsageQuality::Estimated);
        assert_eq!(report.assignments_per_spawn, Some(5.0 / 3.0));
    }

    #[test]
    fn fewer_than_three_exact_samples_never_claim_savings() {
        for count in 0..=2 {
            let rows = (0..count)
                .map(|index| usage(&format!("u{index}"), "s1", UsagePhase::Context, Some(100)))
                .collect();
            let report = efficiency_report(&snapshot(rows, 2));
            assert_eq!(report.estimated_saved_tokens, None);
            assert_eq!(report.estimate_sample_count, count);
        }
    }
}
