use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

macro_rules! wire_enum {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub(crate) enum $name {
            $(#[serde(rename = $wire)] $variant),+
        }

        impl $name {
            pub(crate) const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $wire),+ }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($wire => Ok(Self::$variant)),+,
                    _ => Err(format!("invalid {}: {value}", stringify!($name))),
                }
            }
        }
    };
}

wire_enum!(Profile {
    Light => "light",
    Standard => "standard",
    Deep => "deep",
});

wire_enum!(Risk {
    Low => "low",
    Medium => "medium",
    High => "high",
    Critical => "critical",
});

wire_enum!(Role {
    Discussion => "discussion",
    Explorer => "explorer",
    Worker => "worker",
    Reviewer => "reviewer",
    Fixer => "fixer",
});

wire_enum!(RunStatus {
    Active => "active",
    Complete => "complete",
    Failed => "failed",
    Cancelled => "cancelled",
});

wire_enum!(TaskStatus {
    Queued => "queued",
    Running => "running",
    Blocked => "blocked",
    Reported => "reported",
    Accepted => "accepted",
    Failed => "failed",
    Cancelled => "cancelled",
});

wire_enum!(SessionStatus {
    Starting => "starting",
    Busy => "busy",
    Idle => "idle",
    Closed => "closed",
    Failed => "failed",
    Unknown => "unknown",
});

wire_enum!(RoutingStatus {
    Requested => "requested",
    Applied => "applied",
    Unsupported => "unsupported",
    Unknown => "unknown",
});

wire_enum!(UsageQuality {
    Exact => "exact",
    Partial => "partial",
    Estimated => "estimated",
    Unsupported => "unsupported",
    Unknown => "unknown",
});

wire_enum!(UsagePhase {
    Bootstrap => "bootstrap",
    Context => "context",
    Work => "work",
    Retry => "retry",
    Escalation => "escalation",
    Review => "review",
    Fixer => "fixer",
});

wire_enum!(Language {
    ZhCn => "zh-CN",
    EnUs => "en-US",
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TaskInput {
    pub task_id: String,
    pub run_id: String,
    pub package_key: String,
    pub title: String,
    pub sequence: u64,
    pub role: Role,
    pub complexity: Profile,
    pub risk: Risk,
    pub uncertainty: Profile,
    pub write_scope: Vec<String>,
    pub scope_hash: String,
    pub repo_revision: String,
    pub review_boundary: Option<String>,
    pub required_profile: Profile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TaskRecord {
    pub task_id: String,
    pub run_id: String,
    pub package_key: String,
    pub title: String,
    pub sequence: u64,
    pub role: Role,
    pub complexity: Profile,
    pub risk: Risk,
    pub uncertainty: Profile,
    pub write_scope: Vec<String>,
    pub scope_hash: String,
    pub repo_revision: String,
    pub review_boundary: Option<String>,
    pub required_profile: Profile,
    pub status: TaskStatus,
    pub session_id: Option<String>,
    pub attempt_count: u64,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionInput {
    pub session_id: String,
    pub run_id: String,
    pub host: String,
    pub handle: Option<String>,
    pub role: Role,
    pub profile: Profile,
    pub requested_model: Option<String>,
    pub actual_model: Option<String>,
    pub routing_status: RoutingStatus,
    pub package_key: String,
    pub scope_hash: String,
    pub repo_revision: String,
    pub review_boundary: Option<String>,
    pub status: SessionStatus,
    pub current_task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionRecord {
    pub session_id: String,
    pub run_id: String,
    pub host: String,
    pub handle: Option<String>,
    pub role: Role,
    pub profile: Profile,
    pub requested_model: Option<String>,
    pub actual_model: Option<String>,
    pub routing_status: RoutingStatus,
    pub package_key: String,
    pub scope_hash: String,
    pub repo_revision: String,
    pub review_boundary: Option<String>,
    pub status: SessionStatus,
    pub current_task_id: Option<String>,
    pub reuse_count: u64,
    pub last_used_at: String,
    pub final_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UsageInput {
    pub usage_id: String,
    pub run_id: String,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub phase: UsagePhase,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub source: String,
    pub quality: UsageQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UsageRecord {
    pub usage_id: String,
    pub run_id: String,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub phase: UsagePhase,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub source: String,
    pub quality: UsageQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActivityInput {
    pub run_id: String,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub kind: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActivityRecord {
    pub activity_id: u64,
    pub run_id: String,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub kind: String,
    pub summary: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RunRecord {
    pub run_id: String,
    pub goal: String,
    pub status: RunStatus,
    pub repo_root: String,
    pub report_path: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoreSnapshot {
    pub run: RunRecord,
    pub tasks: Vec<TaskRecord>,
    pub sessions: Vec<SessionRecord>,
    pub usage: Vec<UsageRecord>,
    pub activity: Vec<ActivityRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TaskBundle {
    pub package_key: String,
    pub tasks: Vec<TaskRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RouteDemand {
    pub complexity: Profile,
    pub risk: Risk,
    pub role: Role,
    pub uncertainty: Profile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RouteDecision {
    pub profile: Profile,
    pub reason_codes: Vec<String>,
    pub manual_lowering_rejected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionSignature {
    pub host: String,
    pub role: Role,
    pub profile: Profile,
    pub package_key: String,
    pub scope_hash: String,
    pub repo_revision: String,
    pub review_boundary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DispatchAction {
    ExecuteOnMain,
    ReuseSession,
    BatchThenSpawn,
    SpawnSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DispatchRequest {
    pub run_id: String,
    pub task_id: String,
    pub signature: SessionSignature,
    pub trivial: bool,
    pub isolation_required: bool,
    pub related_ready_count: usize,
    pub delegation_value_exceeds_cost: bool,
    pub host_supports_followup: bool,
    pub reuse_budget: ReuseBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReuseBudget {
    pub max_accepted_followups: u64,
    pub max_effective_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionClaimResult {
    pub session_id: Option<String>,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DispatchDecision {
    pub action: DispatchAction,
    pub session_id: Option<String>,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Operation {
    Spawn,
    Followup,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TemplateValues {
    pub prompt: Option<String>,
    pub session: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HostTemplate {
    pub name: String,
    pub spawn_command: Vec<String>,
    pub followup_command: Option<Vec<String>>,
    pub close_command: Option<Vec<String>>,
    pub profile_arguments: BTreeMap<Profile, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TokenTotals {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub reasoning: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
    pub total_effective: Option<u64>,
    pub quality: UsageQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PhaseTokenTotals {
    pub phase: UsagePhase,
    pub totals: TokenTotals,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct EfficiencyReport {
    pub totals: TokenTotals,
    pub phase_totals: Vec<PhaseTokenTotals>,
    pub assignments_per_spawn: Option<f64>,
    pub churn_rate: Option<f64>,
    pub reuse_count: u64,
    pub estimated_saved_tokens: Option<u64>,
    pub estimate_sample_count: usize,
    pub estimate_quality: UsageQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StatusView {
    pub run: RunStatusView,
    pub tasks: Vec<TaskStatusView>,
    pub sessions: Vec<SessionStatusView>,
    pub efficiency: EfficiencyReport,
    pub recent_activity: Vec<ActivityStatusView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RunStatusView {
    pub run_id: String,
    pub goal: String,
    pub status: RunStatus,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TaskStatusView {
    pub task_id: String,
    pub package_key: String,
    pub title: String,
    pub role: Role,
    pub required_profile: Profile,
    pub status: TaskStatus,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionStatusView {
    pub session_id: String,
    pub host: String,
    pub role: Role,
    pub profile: Profile,
    pub requested_model: Option<String>,
    pub actual_model: Option<String>,
    pub routing_status: RoutingStatus,
    pub status: SessionStatus,
    pub current_task_id: Option<String>,
    pub reuse_count: u64,
    pub last_used_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActivityStatusView {
    pub activity_id: u64,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub kind: String,
    pub summary: String,
    pub occurred_at: String,
}

#[cfg(test)]
mod tests {
    use super::{
        Language, Profile, Risk, Role, RoutingStatus, SessionStatus, TaskStatus, UsagePhase,
        UsageQuality,
    };
    use std::str::FromStr;

    #[test]
    fn wire_enums_are_strict_lowercase_values() {
        assert_eq!(Profile::from_str("light").unwrap(), Profile::Light);
        assert_eq!(Profile::from_str("standard").unwrap(), Profile::Standard);
        assert_eq!(Profile::from_str("deep").unwrap(), Profile::Deep);
        assert!(Profile::from_str("Deep").is_err());
        assert_eq!(
            serde_json::to_string(&Risk::Critical).unwrap(),
            "\"critical\""
        );
        assert_eq!(Role::Reviewer.as_str(), "reviewer");
        assert_eq!(TaskStatus::Accepted.as_str(), "accepted");
        assert_eq!(SessionStatus::Idle.as_str(), "idle");
        assert_eq!(UsagePhase::Fixer.as_str(), "fixer");
        assert_eq!(RoutingStatus::Unsupported.as_str(), "unsupported");
        assert_eq!(UsageQuality::Unknown.as_str(), "unknown");
        assert_eq!(Language::from_str("zh-CN").unwrap(), Language::ZhCn);
        assert_eq!(Language::from_str("en-US").unwrap(), Language::EnUs);
    }

    #[test]
    fn profiles_have_a_stable_safety_order() {
        assert!(Profile::Light < Profile::Standard);
        assert!(Profile::Standard < Profile::Deep);
    }
}
