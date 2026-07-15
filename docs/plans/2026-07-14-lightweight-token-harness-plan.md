# Lightweight Token-Efficiency Harness Implementation Plan

Status: **Completed with an evidence-driven Session correction.** The original
five-follow-up example in this plan is rejected historical RED evidence, not
current routing policy. Known compatible work now batches first; later reuse is
bounded by one accepted follow-up, 200,000 effective Tokens, and exact causal
usage. See [Current Product State](../current-state.md) and the
[real A/B evidence](../benchmarks/2026-07-15-signal-sweep-real-ab.md). Unchecked
boxes below preserve the implementation recipe and do not indicate unfinished
delivery.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the unfinished event-sourced control plane with a small standalone harness that measurably reduces total effective token use through task batching, compatible session reuse, safe model routing, honest accounting, and a mandatory bilingual dashboard.

**Architecture:** SQLite current-state tables are authoritative; a compact activity feed is informational and never replayed. Focused Rust modules own bundling, routing, sessions, host command templates, accounting, status, and the embedded loopback dashboard. The existing standalone methodology and prompt discipline remain, while the 67-event runtime, replay, provenance, scanner, bridge, and observer are removed.

**Tech Stack:** Rust 2024, bundled SQLite through `rusqlite`, `serde`/`serde_json`, `tiny_http`, embedded HTML/CSS/JavaScript, Python `unittest` release contracts.

## Global Constraints

- Standalone is the normal mode; installation and startup perform no Superpowers file or network action.
- Preserve PSOC, test-first behavior, serialized overlapping writes, independent review, and final lifecycle audit.
- Optimize total effective tokens only after role, risk, uncertainty, and quality floors are set.
- Count bootstrap, context, work, retry, escalation, review, and fixer usage; missing telemetry remains null/unknown.
- Requested and actual model data are separate; unsupported routing is never reported as applied.
- Host commands are rendered as argument arrays and never evaluated through a shell.
- The dashboard is mandatory, read-only, loopback-only by default, zh-CN/en-US, Moonlight Indigo liquid glass, 13 px body, 12 px secondary text, and metadata no smaller than 11 px.
- Do not retain the prior 67-event registry, replay, field provenance, lease entity, scanner, adapter framework, desktop bridge, active probe, or observer.
- Do not install or update the currently installed Skill during development.
- Every task ends with focused tests and a commit; the final task runs the complete repository harness and independent review.

---

## File Structure

### Runtime files

- `src/domain.rs`: validated enums, identifiers, records, and decision/result types.
- `src/store.rs`: compact SQLite schema, transactions, state transitions, and read projections.
- `src/bundle.rs`: compatible-task grouping.
- `src/routing.rs`: provider-neutral profile floors and route decisions.
- `src/sessions.rs`: compatibility signatures, atomic reuse claims, idle/close operations.
- `src/hosts.rs`: fixed command-template registry and safe argument rendering.
- `src/accounting.rs`: complete token totals and honest reuse estimates.
- `src/status.rs`: shared terminal/JSON/dashboard projection.
- `src/dashboard.rs`: loopback HTTP server and JSON endpoints.
- `assets/index.html`, `assets/styles.css`, `assets/app.js`: embedded bilingual dashboard.
- `src/main.rs`: CLI parsing and composition only.

### Removed runtime files

- `src/event_store.rs`
- `src/schema.rs`
- `src/ledger.rs`

### Skill and release files

- `skills/cached-subagent-harness/SKILL.md`
- `references/standalone-methodology.md`
- `references/gates.md`
- `references/report-contracts.md`
- `references/host-templates.json`
- `scripts/validate-release.py`
- `scripts/test_standalone_contract.py`
- `scripts/verify.sh`

## Task 1: Compact Domain and Current-State Store

**Files:**

- Create: `skills/cached-subagent-harness/scripts/harnessctl/src/domain.rs`
- Create: `skills/cached-subagent-harness/scripts/harnessctl/src/store.rs`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/main.rs`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml`
- Delete after GREEN: `src/event_store.rs`, `src/schema.rs`, `src/ledger.rs`

**Interfaces:**

- Produces `HarnessError`, `Profile`, `Risk`, `Role`, `Language`,
  `TaskStatus`, `SessionStatus`, `RoutingStatus`, `UsageQuality`, `UsagePhase`,
  `TaskInput`, `TaskRecord`, `TaskBundle`, `RouteDemand`, `RouteDecision`,
  `SessionInput`, `SessionSignature`, `DispatchRequest`, `DispatchDecision`,
  `UsageInput`, `ActivityInput`, `HostTemplate`, `Operation`, `TemplateValues`,
  `StoreSnapshot`, and `StatusView`.
- Later tasks must use `Store` operations instead of issuing SQL directly.

- [ ] **Step 1: Add dependencies and write failing fresh-store tests**

Add to `Cargo.toml`:

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tiny_http = "0.12"
```

Write tests in `store.rs` that call the not-yet-implemented API:

```rust
let store = Store::open(temp.path()).unwrap();
assert_eq!(store.schema_version().unwrap(), 1);
assert_eq!(store.table_names().unwrap(), ["activity", "runs", "sessions", "tasks", "usage"]);
assert!(store.foreign_keys_enabled().unwrap());
```

- [ ] **Step 2: Run the RED test**

Run:

```bash
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml store::tests::fresh_store_has_only_compact_tables -- --nocapture
```

Expected: compile failure because `Store` and the compact modules do not exist.

- [ ] **Step 3: Define exact domain types**

Implement serializable enums with lowercase wire values and `FromStr` validation:

```rust
pub enum Profile { Light, Standard, Deep }
pub enum Risk { Low, Medium, High, Critical }
pub enum Role { Discussion, Explorer, Worker, Reviewer, Fixer }
pub enum TaskStatus { Queued, Running, Blocked, Reported, Accepted, Failed, Cancelled }
pub enum SessionStatus { Starting, Busy, Idle, Closed, Failed, Unknown }
pub enum RoutingStatus { Requested, Applied, Unsupported, Unknown }
pub enum UsageQuality { Exact, Partial, Estimated, Unsupported, Unknown }
pub enum UsagePhase { Bootstrap, Context, Work, Retry, Escalation, Review, Fixer }
pub enum Language { ZhCn, EnUs }

pub type Result<T> = std::result::Result<T, HarnessError>;

pub struct TaskBundle { pub package_key: String, pub tasks: Vec<TaskRecord> }
pub struct RouteDecision { pub profile: Profile, pub reason_codes: Vec<String>, pub manual_lowering_rejected: bool }
pub struct DispatchDecision { pub action: DispatchAction, pub session_id: Option<String>, pub reason_codes: Vec<String> }

pub struct RouteDemand {
    pub complexity: Profile,
    pub risk: Risk,
    pub role: Role,
    pub uncertainty: Profile,
}

pub struct DispatchRequest {
    pub run_id: String,
    pub task_id: String,
    pub signature: SessionSignature,
    pub trivial: bool,
    pub isolation_required: bool,
    pub related_ready_count: usize,
    pub delegation_value_exceeds_cost: bool,
    pub host_supports_followup: bool,
}

pub enum Operation { Spawn, Followup, Close }
pub struct TemplateValues { pub prompt: Option<String>, pub session: Option<String>, pub model: Option<String> }
pub struct HostTemplate {
    pub name: String,
    pub spawn_command: Vec<String>,
    pub followup_command: Option<Vec<String>>,
    pub close_command: Option<Vec<String>>,
    pub profile_arguments: std::collections::BTreeMap<Profile, Vec<String>>,
}
```

Every external identifier and nonempty text field is validated before SQL.

- [ ] **Step 4: Implement the five-table schema**

Create only `runs`, `tasks`, `sessions`, `usage`, and `activity`, plus indexes for `tasks(run_id,status)`, `sessions(run_id,status)`, `usage(run_id,phase)`, and `activity(run_id,activity_id)`.

Use `PRAGMA user_version=1`, `foreign_keys=ON`, `journal_mode=WAL`, and short `IMMEDIATE` transactions. Numeric token columns are nullable and nonnegative. Corrupt or unexpected versioned files return errors and are never replaced.

- [ ] **Step 5: Write RED transition tests**

Cover:

```rust
assert_rejected_without_mutation(TaskStatus::Accepted, TaskStatus::Running);
assert_rejected_without_mutation(SessionStatus::Closed, SessionStatus::Busy);
assert_final_audit_rejects_active_task_and_session();
assert_unknown_tokens_round_trip_as_none();
```

Run `cargo test ... store::tests -- --nocapture`; expected failures name missing transition and audit operations.

- [ ] **Step 6: Implement state operations**

Provide:

```rust
impl Store {
    pub fn create_run(&mut self, run_id: &str, goal: &str, repo_root: &str, report_path: &str) -> Result<()>;
    pub fn add_task(&mut self, input: &TaskInput) -> Result<()>;
    pub fn update_task(&mut self, task_id: &str, status: TaskStatus, next_action: Option<&str>) -> Result<()>;
    pub fn add_session(&mut self, input: &SessionInput) -> Result<()>;
    pub fn record_usage(&mut self, input: &UsageInput) -> Result<()>;
    pub fn append_activity(&mut self, input: &ActivityInput) -> Result<()>;
    pub fn snapshot(&self, run_id: &str) -> Result<StoreSnapshot>;
    pub fn final_audit(&self, run_id: &str) -> Result<(), Vec<String>>;
}
```

Each transition and its matching activity row commit atomically.

- [ ] **Step 7: Remove the obsolete runtime**

Remove the three old modules and their `mod` declarations only after compact-store tests are GREEN. Confirm searches return no production references to `control_plane_events`, `replay_run_into_empty`, `projection_field_sources`, or `EventInput`.

- [ ] **Step 8: Verify and commit**

Run:

```bash
cargo fmt --check --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml store::tests domain::tests
cargo clippy --all-targets --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml -- -D warnings
```

Expected: compact tests pass and no warnings.

Commit: `refactor: replace event control plane with compact state store`.

## Task 2: Task Bundling and Model Routing

**Files:**

- Create: `src/bundle.rs`
- Create: `src/routing.rs`
- Modify: `src/domain.rs`
- Test: module-local Rust tests

**Interfaces:**

```rust
pub fn compatible_for_batch(left: &TaskRecord, right: &TaskRecord) -> bool;
pub fn bundle_ready(tasks: &[TaskRecord]) -> Vec<TaskBundle>;
pub fn required_profile(demand: &RouteDemand) -> Profile;
pub fn route(demand: &RouteDemand, manual: Option<Profile>) -> RouteDecision;
```

- [ ] **Step 1: Write RED bundling tests**

Use six compatible tasks and assert one ordered bundle. Clone the fixture and change role, profile, risk, package key, write scope, repository revision, review boundary, and dependency readiness one at a time; each changed fixture must split.

Run `cargo test ... bundle::tests`; expected compile failure for missing module.

- [ ] **Step 2: Implement exact compatibility**

Compatibility is equality across all required fields. Sort tasks by declared sequence, never by hash-map order. Empty write scope is invalid for worker/fixer tasks.

- [ ] **Step 3: Write RED routing tests**

Assert:

```rust
assert_eq!(required_profile(low_read_only()), Profile::Light);
assert_eq!(required_profile(scoped_worker()), Profile::Standard);
assert_eq!(required_profile(control_plane_change()), Profile::Deep);
assert_eq!(route(standard_demand(), Some(Profile::Deep)).profile, Profile::Deep);
assert!(route(deep_demand(), Some(Profile::Light)).manual_lowering_rejected);
```

- [ ] **Step 4: Implement floor routing**

Map each demand dimension to a profile, take the maximum, accept manual elevation, and reject manual lowering. Store explanations as stable reason codes rather than prose-only decisions.

- [ ] **Step 5: Verify and commit**

Run focused tests plus Clippy. Commit: `feat: add compatible task bundling and profile routing`.

## Task 3: Session Reuse and Host Command Templates

**Files:**

- Create: `src/sessions.rs`
- Create: `src/hosts.rs`
- Create: `skills/cached-subagent-harness/references/host-templates.json`
- Modify: `src/store.rs`
- Modify: `src/domain.rs`

**Interfaces:**

```rust
pub struct SessionSignature {
    pub host: String,
    pub role: Role,
    pub profile: Profile,
    pub package_key: String,
    pub scope_hash: String,
    pub repo_revision: String,
    pub review_boundary: Option<String>,
}

pub enum DispatchAction { ExecuteOnMain, ReuseSession, BatchThenSpawn, SpawnSession }
pub fn decide(store: &mut Store, request: &DispatchRequest) -> Result<DispatchDecision>;
pub fn accept_followup(store: &mut Store, session_id: &str, task_id: &str) -> Result<()>;
pub fn release_verified(store: &mut Store, session_id: &str, task_id: &str, revision: &str) -> Result<()>;
pub fn render_command(template: &HostTemplate, operation: Operation, values: &TemplateValues) -> Result<Vec<String>>;
```

- [ ] **Step 1: Write RED atomic-reuse tests**

Create one idle compatible session and two simultaneous SQLite connections. The first `decide` returns `ReuseSession` and marks it busy. The second cannot claim the same row and selects another valid action. Assert `reuse_count` stays zero until `accept_followup`.

- [ ] **Step 2: Write RED invalidation matrix**

Change each `SessionSignature` field independently and assert no reuse. Assert busy, closed, failed, unknown, and stale-revision sessions are ineligible.

- [ ] **Step 3: Implement session claiming**

Use `BEGIN IMMEDIATE`; select by exact signature and `status='idle'`; conditional-update the selected row to `busy`; require one changed row; link the task; append `reuse` activity only after follow-up acceptance.

Verified release updates the revision and returns the session to idle. Failed/unverified work closes the reusable path with `final_reason`.

- [ ] **Step 4: Write RED host-template tests**

Load the bundled JSON and assert Codex, Claude, and OpenCode entries exist. Render spawn/follow-up/close arrays, substitute only declared placeholders, and reject missing values, unknown placeholders, unsupported operations, or an empty executable. Assert no rendered value contains a shell wrapper inserted by the harness.

- [ ] **Step 5: Implement data-only templates**

Use `Vec<String>` commands and exact placeholder replacement. Custom JSON templates merge by host name and may override bundled data without code changes.

- [ ] **Step 6: Verify the six-task reuse scenario**

The integration fixture records one spawn, accepts five follow-ups, ends with `reuse_count=5`, `assignments_per_spawn=6`, and no second session row.

- [ ] **Step 7: Verify and commit**

Run session/host tests and Clippy. Commit: `feat: reuse compatible sessions through host templates`.

## Task 4: Complete Token Accounting

**Files:**

- Create: `src/accounting.rs`
- Modify: `src/domain.rs`
- Modify: `src/store.rs`

**Interfaces:**

```rust
pub struct TokenTotals {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub reasoning: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
    pub total_effective: Option<u64>,
    pub quality: UsageQuality,
}

pub struct EfficiencyReport {
    pub totals: TokenTotals,
    pub assignments_per_spawn: Option<f64>,
    pub churn_rate: Option<f64>,
    pub reuse_count: u64,
    pub estimated_saved_tokens: Option<u64>,
    pub estimate_sample_count: usize,
    pub estimate_quality: UsageQuality,
}

pub fn efficiency_report(snapshot: &StoreSnapshot) -> EfficiencyReport;
```

- [ ] **Step 1: Write RED phase-total tests**

Record exact bootstrap, context, work, retry, escalation, review, and fixer rows. Assert all phases contribute to total effective tokens. Remove one required value and assert total becomes partial/unknown rather than treating the missing value as zero.

- [ ] **Step 2: Write RED estimate tests**

Assert zero, one, and two exact overhead samples produce no saving estimate. Three samples `[100, 300, 200]` with two avoided spawns produce `400`, sample count `3`, and `estimated` quality. Partial/unknown samples do not qualify.

- [ ] **Step 3: Implement checked aggregation**

Use checked integer addition. Compute the median deterministically. Require equal quality-gate outcomes before an A/B comparison is labeled valid. Currency savings remain absent without explicit compatible prices.

- [ ] **Step 4: Verify legacy benchmarks against the new report**

Update the token-effectiveness and game-development fixtures to consume `EfficiencyReport` while retaining their quality thresholds. No benchmark may claim an observed saving from an estimate.

- [ ] **Step 5: Verify and commit**

Run Rust accounting tests and Python benchmark tests. Commit: `feat: account for complete agent token cost`.

## Task 5: CLI Status, Watch, and Lifecycle Commands

**Files:**

- Create: `src/status.rs`
- Rewrite: `src/main.rs`
- Modify: `src/store.rs`

**Interfaces:**

```rust
pub fn build_status(store: &Store, run_id: &str) -> Result<StatusView>;
pub fn render_text(view: &StatusView, language: Language) -> String;
pub fn render_json(view: &StatusView) -> Result<String>;
```

Commands:

```text
harnessctl init --db DB --run ID --goal TEXT --repo-root PATH --report PATH
harnessctl task add|update ...
harnessctl decide ...
harnessctl session record|accept-followup|release|close ...
harnessctl usage add ...
harnessctl status --db DB --run ID [--json] [--lang zh-CN|en-US]
harnessctl watch --db DB --run ID [--interval-ms 1500] [--iterations N]
harnessctl audit --db DB --run ID
```

Retain `render-prompt` and `check-prompt`.

- [ ] **Step 1: Write RED parser and command tests**

Test every required flag, invalid enum, noncanonical number, missing database,
and help output. CLI errors go to stderr and return nonzero without partial state.

- [ ] **Step 2: Implement command composition**

Keep parsing in `main.rs`; call module APIs for all behavior. Do not duplicate SQL or routing rules in command handlers.

- [ ] **Step 3: Write RED status parity tests**

Build a fixture with queued/running/blocked/accepted tasks, busy/idle/closed sessions, exact and unknown usage, and recent actions. Assert text contains translated labels and JSON preserves nulls.

- [ ] **Step 4: Implement deterministic status**

Sort tasks by creation/id, sessions by status/last use/id, and activities by descending ID. `watch --iterations` exists for deterministic tests; normal watch runs until interrupted.

- [ ] **Step 5: Verify and commit**

Run CLI/status tests and Clippy. Commit: `feat: expose lightweight harness status and watch`.

## Task 6: Mandatory Embedded Dashboard

**Files:**

- Create: `src/dashboard.rs`
- Create: `assets/index.html`
- Create: `assets/styles.css`
- Create: `assets/app.js`
- Modify: `src/main.rs`

**Interfaces:**

```rust
pub struct DashboardOptions { pub bind: IpAddr, pub port: u16, pub language: Language }
pub fn serve(store_path: &Path, run_id: &str, options: DashboardOptions) -> Result<SocketAddr>;
```

Endpoints:

```text
GET /                 -> embedded HTML
GET /assets/styles.css
GET /assets/app.js
GET /api/status       -> the exact StatusView JSON
GET /health           -> {"status":"ok"}
```

- [ ] **Step 1: Write RED HTTP tests**

Start on `127.0.0.1:0`, request all endpoints, assert content type, CSP/security headers, JSON null preservation, and 404 behavior. Assert a non-loopback bind fails unless `--allow-remote true` is explicit.

- [ ] **Step 2: Implement the loopback server**

Use `tiny_http`; only GET is accepted. Set `Content-Security-Policy: default-src 'self'`, `X-Content-Type-Options: nosniff`, and `Cache-Control: no-store` for JSON.

- [ ] **Step 3: Implement the four-panel page**

HTML contains Tasks, Agents, Token Economy, and Recent Actions landmarks. JavaScript polls `/api/status` every 1500 ms, uses `textContent` for all dynamic values, renders null as `—`, and stores only the language preference in `localStorage`.

- [ ] **Step 4: Apply the approved visual contract**

CSS uses system sans-serif, Moonlight Indigo variables, restrained blur/translucency, 13 px body, 12 px secondary, minimum 11 px metadata, keyboard-visible focus, reduced-motion support, and a dense responsive grid.

- [ ] **Step 5: Add bilingual and accessibility tests**

Static contract tests assert every displayed key has zh-CN/en-US text, landmarks and buttons have accessible names, contrast variables exist, and no font drops below the approved floor.

- [ ] **Step 6: Verify polling parity**

Update a task in SQLite between two `/api/status` requests and assert the second response exactly matches `harnessctl status --json` for the same run.

- [ ] **Step 7: Verify and commit**

Run dashboard tests and Clippy. Commit: `feat: add mandatory bilingual token dashboard`.

## Task 7: Skill Contract, Release Cleanup, and Final Gates

**Files:**

- Modify: `skills/cached-subagent-harness/SKILL.md`
- Modify: `references/standalone-methodology.md`
- Modify: `references/gates.md`
- Modify: `references/report-contracts.md`
- Modify: `scripts/validate-release.py`
- Modify: `scripts/test_standalone_contract.py`
- Modify: `scripts/verify.sh`
- Modify: `README.md`
- Preserve as superseded history: earlier architecture/delta/plan documents

- [ ] **Step 1: Write RED release-contract tests**

Tests require the six lightweight modules, dashboard assets, host templates,
new canonical design, and commands. Tests reject production references to the
deleted event registry, replay, provenance, scanner, bridge, probe, or observer.

- [ ] **Step 2: Run RED release tests**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts/test_standalone_contract.py
python3 scripts/validate-release.py .
```

Expected: fail until Skill/reference/runtime metadata is updated.

- [ ] **Step 3: Rewrite the Skill around the token-efficiency loop**

The primary loop becomes:

```text
PSOC -> group compatible work -> set quality floors ->
main/reuse/batch/spawn decision -> record actual route ->
verify/report/review -> record complete usage -> close or retain compatible idle session -> audit
```

Keep the pre-existing standalone principles. Describe Dashboard/status as
required visibility. Remove the superseded event-control-plane language.

- [ ] **Step 4: Update release validation and verification**

Require the new canonical design and focused files. Build the binary, run the
six-task reuse integration, profile routing fixtures, accounting fixtures,
status/dashboard parity smoke, prompt contract, installation tests, Rust tests,
Clippy, and release validation.

- [ ] **Step 5: Run full fresh verification**

Run:

```bash
cargo fmt --check --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
cargo clippy --all-targets --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml -- -D warnings
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/validate-release.py .
scripts/verify.sh
git diff --check
```

Expected: all commands exit zero with no ignored tests or warnings.

- [ ] **Step 6: Independent review**

Generate one review package from commit `8e811f8` to final HEAD. The reviewer
checks the approved lightweight design, deleted-scope absence, token-saving
truthfulness, session race safety, host argument safety, dashboard security,
and tests. Fix all Critical/Important findings in one pass and re-review only
the fix range plus the prior findings report.

- [ ] **Step 7: Final lifecycle and release audit**

Confirm every harness-created session is closed or has a truthful terminal
reason, the worktree is clean, no installed Skill was changed, and every
acceptance criterion in the design maps to fresh test or runtime evidence.

- [ ] **Step 8: Commit final contract**

Commit: `docs: publish lightweight token harness contract`.
