# Results Dashboard and Signal Sweep Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `subagent-driven-development`
> or `executing-plans` to implement this plan task by task. Steps use checkbox
> (`- [ ]`) syntax for tracking. This execution uses the inline
> `executing-plans` path; do not create fresh implementation agents.

**Goal:** Deliver a polished single-run Harness results Dashboard and validate
the token-efficiency claim with a complete, separate Signal Sweep A/B run.

**Architecture:** The existing SQLite state and `StatusView` remain the only
product truth. Rust projects persisted run freshness and phase-level token
facts; embedded HTML/CSS/JavaScript turns those facts into a dense Moonlight
Indigo results surface. A reproducible Signal Sweep starter makes the existing
benchmark runnable, but baseline evidence stays outside the Dashboard.

**Tech Stack:** Rust 2024, bundled SQLite through `rusqlite`, `serde`,
`tiny_http`, embedded HTML/CSS/JavaScript, Python `unittest`, Node's built-in
test runner for the benchmark game, Codex CLI JSONL telemetry, Firefox headless.

## Evidence-Driven Correction

Task 4 executed the planned one-Session/three-follow-up topology and invalidated
it: the comparable Harness arm consumed 17,551,878 effective Tokens versus
2,974,064 for Baseline. The plan therefore looped back to the Session strategy
without changing product scope. Remaining release work uses these corrections:

- known compatible ready assignments return `BatchThenSpawn` before reuse;
- later reuse defaults to one accepted follow-up and 200,000 effective Tokens;
- release requires durable follow-up acceptance and exact assignment usage
  strictly after its transactional causal boundary;
- unknown usage or either exhausted budget makes reuse ineligible;
- queued revision refresh is compare-and-swap and unassigned-only;
- Codex totals are normalized into non-overlapping input/cache/output/reasoning
  categories, and retries remain in total cost;
- terminal Sessions clear their current assignment and audit enforces it.

The original Task 4 topology remains the RED evidence, not a release
recommendation. No positive live saving claim is permitted for the corrected
policy until a separate real run measures it.

## Global Constraints

- Token reduction remains the primary product goal; visualization is a
  read-only supporting feature.
- The Dashboard shows only one Harness run and contains no baseline columns,
  experiment tabs, comparison controls, or benchmark branding.
- Standalone operation must not require Superpowers at installation or runtime.
- Preserve PSOC, test-first behavior changes, serialized overlapping writes,
  independent review, complete-development gates, and final lifecycle audit.
- Do not spawn fresh implementation subagents. The A/B validation may create
  the explicitly approved isolated Codex sessions defined in Task 4.
- Missing telemetry remains null/unknown through SQLite, Rust, JSON, and the
  page. Estimates are labeled with method, quality, and sample count.
- Requested and actual model values remain separate facts.
- Keep the Web runtime dependency-free: no frontend framework, Node service,
  observer, remote asset, or new database table.
- Bind the Dashboard to loopback by default and preserve its CSP and public-safe
  DTO boundary.
- Use zh-CN and en-US with operational body text at `14px`, secondary text at
  `12px`, and machine metadata at no less than `11px`.
- Every source-changing task follows RED-GREEN-REFACTOR and ends with focused
  tests plus a commit.
- Do not install, push, publish, or modify the currently installed Skill.

---

## File Structure

- `skills/cached-subagent-harness/scripts/harnessctl/src/domain.rs`: add typed
  run state and public phase-total DTOs.
- `skills/cached-subagent-harness/scripts/harnessctl/src/store.rs`: persist
  truthful run freshness and explicit terminal run state.
- `skills/cached-subagent-harness/scripts/harnessctl/src/accounting.rs`: compute
  total and per-phase token facts with honest quality.
- `skills/cached-subagent-harness/scripts/harnessctl/src/status.rs`: expose the
  same public-safe facts to CLI JSON and the Dashboard.
- `skills/cached-subagent-harness/scripts/harnessctl/src/main.rs`: add the small
  `run update` command and preserve CLI composition only.
- `skills/cached-subagent-harness/scripts/harnessctl/src/dashboard.rs`: enforce
  the new embedded-page contract and HTTP safety.
- `skills/cached-subagent-harness/scripts/harnessctl/assets/index.html`: define
  the results-first semantic document structure.
- `skills/cached-subagent-harness/scripts/harnessctl/assets/styles.css`: own the
  Moonlight Indigo glass system, responsive layout, states, and accessibility
  fallbacks.
- `skills/cached-subagent-harness/scripts/harnessctl/assets/app.js`: own
  localization, deterministic client projections, safe rendering, polling, and
  last-good-snapshot behavior.
- `scripts/game_dev_ab_benchmark.py`: generate identical runnable starter
  projects in addition to prompt artifacts.
- `scripts/test_game_dev_ab_benchmark.py`: cover the starter and preserved A/B
  report semantics.
- `docs/game-dev-ab-benchmark.md`: document the runnable project artifacts and
  real-run protocol.
- `scripts/verify.sh`: include only deterministic new checks required for the
  release; do not make live Codex calls part of CI.
- `results-dashboard-implementation.md`: durable PSOC, evidence, lifecycle,
  review, and final audit report.
- `docs/benchmarks/2026-07-15-signal-sweep-real-ab.md`: sanitized real A/B
  evidence without raw prompts, source content, or session logs.

## Task 1: Truthful Run State, Freshness, and Token Phase Read Model

**Files:**

- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/domain.rs`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/store.rs`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/accounting.rs`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/status.rs`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/main.rs`
- Create: `results-dashboard-implementation.md`
- Runtime state: `results-dashboard-implementation.db` (ignored, not committed)

**Interfaces:**

```rust
pub(crate) enum RunStatus { Active, Complete, Failed, Cancelled }

pub(crate) struct PhaseTokenTotals {
    pub phase: UsagePhase,
    pub totals: TokenTotals,
}

pub(crate) struct RunRecord {
    // existing public fields
    pub status: RunStatus,
    pub updated_at: String,
}

pub(crate) struct RunStatusView {
    pub run_id: String,
    pub goal: String,
    pub status: RunStatus,
    pub updated_at: String,
}

impl Store {
    pub(crate) fn update_run(
        &mut self,
        run_id: &str,
        target: RunStatus,
    ) -> Result<(), String>;
}

fn token_totals<'a>(
    rows: impl Iterator<Item = &'a UsageRecord>,
) -> TokenTotals;
```

- [ ] **Step 1: Initialize durable execution state before source edits**

Create the implementation report with the approved PSOC, agent budget, empty
implementation-agent ledger, explicit write scope, evidence, tests, review,
risks, and final-audit sections. Initialize the ignored SQLite state:

```bash
skills/cached-subagent-harness/scripts/bin/harnessctl init \
  --db results-dashboard-implementation.db \
  --run results-dashboard-20260715 \
  --goal "results-focused dashboard and Signal Sweep validation" \
  --repo-root /home/shangkailiang/workspace/cached-subagent-harness \
  --report /home/shangkailiang/workspace/cached-subagent-harness/results-dashboard-implementation.md
```

Record five queued work packages matching this plan. No implementation session
is spawned.

- [ ] **Step 2: Write failing typed-run and freshness tests**

Add store tests that require `Active -> Complete|Failed|Cancelled`, reject every
terminal-to-active transition without mutation, and verify that task, session,
usage, and activity writes advance the run's persisted `updated_at`.

```rust
let before = store.snapshot("run-1").unwrap().run.updated_at;
std::thread::sleep(std::time::Duration::from_millis(2));
store.record_usage(&exact_usage("usage-1", UsagePhase::Work, 21)).unwrap();
let after = store.snapshot("run-1").unwrap().run.updated_at;
assert!(after > before);

store.update_run("run-1", RunStatus::Complete).unwrap();
assert_eq!(store.snapshot("run-1").unwrap().run.status, RunStatus::Complete);
assert!(store.update_run("run-1", RunStatus::Active).is_err());
```

Run:

```bash
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml store::tests -- --nocapture
```

Expected: compile failure because `RunStatus`, `updated_at`, and `update_run` do
not exist.

- [ ] **Step 3: Write failing phase-total and status-projection tests**

Require all seven phases in stable enum order. An exact work row must not hide
an unknown retry row, and the public JSON must include freshness and phase facts
while excluding internal fields.

```rust
let report = efficiency_report(&snapshot(vec![
    usage("u1", "s1", UsagePhase::Work, Some(100), UsageQuality::Exact),
    usage("u2", "s1", UsagePhase::Retry, None, UsageQuality::Unknown),
]));
assert_eq!(report.phase_totals.len(), 7);
assert_eq!(report.phase_totals[2].totals.total_effective, Some(100));
assert_eq!(report.phase_totals[3].totals.total_effective, None);
assert_eq!(report.phase_totals[3].totals.quality, UsageQuality::Unknown);
```

Run focused `accounting::tests` and `status::tests`; expected failure names the
missing `phase_totals` and `updated_at` fields.

- [ ] **Step 4: Implement the minimum typed state and accounting changes**

Add `RunStatus` to the wire-enum definitions. Add a shared token-total helper
that accepts an iterator over borrowed usage rows, then call it once for the
whole run and once for each phase:

```rust
const PHASES: [UsagePhase; 7] = [
    UsagePhase::Bootstrap,
    UsagePhase::Context,
    UsagePhase::Work,
    UsagePhase::Retry,
    UsagePhase::Escalation,
    UsagePhase::Review,
    UsagePhase::Fixer,
];

let phase_totals = PHASES
    .into_iter()
    .map(|phase| PhaseTokenTotals {
        phase,
        totals: token_totals(snapshot.usage.iter().filter(|row| row.phase == phase)),
    })
    .collect();
```

Use a short immediate transaction for `record_usage`. Touch `runs.updated_at`
from every successful state mutation; `append_activity_tx` uses its persisted
activity timestamp for the same transaction. Implement only active-to-terminal
run transitions. A `complete` transition first requires `final_audit` to pass;
`failed` and `cancelled` remain truthful terminal states but do not waive the
later lifecycle audit.

- [ ] **Step 5: Add and verify the CLI terminal-state command**

Add:

```text
harnessctl run update --db DB --run ID --status complete|failed|cancelled
```

The command rejects `active` as a target and leaves `audit` read-only. Run:

```bash
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml accounting::tests
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml status::tests
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml store::tests
cargo fmt --check --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
```

Expected: focused tests pass with no formatting changes pending.

- [ ] **Step 6: Commit Task 1**

```bash
git add results-dashboard-implementation.md \
  skills/cached-subagent-harness/scripts/harnessctl/src/domain.rs \
  skills/cached-subagent-harness/scripts/harnessctl/src/store.rs \
  skills/cached-subagent-harness/scripts/harnessctl/src/accounting.rs \
  skills/cached-subagent-harness/scripts/harnessctl/src/status.rs \
  skills/cached-subagent-harness/scripts/harnessctl/src/main.rs
git commit -m "feat: expose truthful dashboard result state"
```

## Task 2: Results-First Embedded Dashboard

**Files:**

- Modify: `skills/cached-subagent-harness/scripts/harnessctl/src/dashboard.rs`
- Replace: `skills/cached-subagent-harness/scripts/harnessctl/assets/index.html`
- Replace: `skills/cached-subagent-harness/scripts/harnessctl/assets/styles.css`
- Replace: `skills/cached-subagent-harness/scripts/harnessctl/assets/app.js`

**Interfaces:**

The HTTP surface remains exactly `/`, `/health`, `/api/status`,
`/assets/styles.css`, and `/assets/app.js`. The page consumes only `StatusView`.
JavaScript defines deterministic helpers `progressOf(tasks)`,
`packagesOf(tasks)`, `assignmentsFor(session, tasks)`, `latestFor(task,
activity)`, and `formatQuality(value)` inside the existing safe IIFE.

- [ ] **Step 1: Write a failing embedded-page contract test**

Extend `dashboard::tests::dashboard_serves_embedded_assets_status_and_security_headers`
to require the new semantic regions and reject product comparison UI:

```rust
for marker in [
    "data-view=\"run-bar\"",
    "data-view=\"outcome-band\"",
    "data-view=\"task-board\"",
    "data-view=\"session-dock\"",
    "data-view=\"evidence-deck\"",
] {
    assert!(html.contains(marker), "missing {marker}");
}
let product_assets = format!("{html}\n{}", get(address, "/assets/app.js"));
for forbidden in ["Baseline", "baseline column", "experiment tab"] {
    assert!(!product_assets.contains(forbidden));
}
assert!(app.contains("phase_totals"));
assert!(app.contains("updated_at"));
assert!(!app.contains("innerHTML"));
```

Run the single test. Expected: FAIL on the first new region marker.

- [ ] **Step 2: Replace the document with the approved hierarchy**

Create one `.app-shell` containing:

```html
<header class="run-bar glass" data-view="run-bar">...</header>
<section class="outcome-band" data-view="outcome-band">...</section>
<main class="operational-grid">
  <section class="task-board glass" data-view="task-board">...</section>
  <aside class="session-dock glass" data-view="session-dock">...</aside>
</main>
<section class="evidence-deck" data-view="evidence-deck">...</section>
```

The outcome band contains one progress surface plus effective tokens, reuse,
assignments per spawn, and avoided-context estimate. The task board precedes
the session dock in source order. Evidence contains phase/token composition and
the activity timeline.

- [ ] **Step 3: Implement the Moonlight Indigo visual system**

Define named tokens for canvas, glass, opaque evidence, ink, indigo, emerald,
amber, red, borders, focus, and tabular numerals. Use a maximum shell width of
`1560px`, body size `14px`, and a desktop operational split near `minmax(0,
1.65fr) minmax(320px, .75fr)`. Use segmented factual progress, package headers,
compact state badges, and connected assignment nodes. Add `820px` and `560px`
breakpoints, `prefers-reduced-motion`, `prefers-reduced-transparency`, and a
no-`backdrop-filter` opaque fallback.

- [ ] **Step 4: Implement bilingual safe rendering**

Keep all copy in complete `zh-CN` and `en-US` dictionaries. Render only through
`textContent` and `createElement`. Compute:

```javascript
const progressOf = tasks => ({
  total: tasks.length,
  accepted: tasks.filter(task => task.status === "accepted").length,
  active: tasks.filter(task => ["running", "reported"].includes(task.status)).length,
  blocked: tasks.filter(task => ["blocked", "failed"].includes(task.status)).length,
  queued: tasks.filter(task => task.status === "queued").length
});

const assignmentsFor = (session, tasks) =>
  tasks.filter(task => task.session_id === session.session_id);
```

Group tasks by `package_key`, associate the latest activity by `task_id`, render
requested and actual model separately, localize known lifecycle/status/quality
values, and display persisted `run.updated_at`. On polling failure, retain the
last good DOM snapshot and change only connectivity state.

- [ ] **Step 5: Verify focused behavior and commit**

Run:

```bash
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml dashboard::tests
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml status::tests
node --check skills/cached-subagent-harness/scripts/harnessctl/assets/app.js
git diff --check
```

Expected: all commands pass. Commit:

```bash
git add skills/cached-subagent-harness/scripts/harnessctl/src/dashboard.rs \
  skills/cached-subagent-harness/scripts/harnessctl/assets/index.html \
  skills/cached-subagent-harness/scripts/harnessctl/assets/styles.css \
  skills/cached-subagent-harness/scripts/harnessctl/assets/app.js \
  results-dashboard-implementation.md
git commit -m "feat: redesign the harness results dashboard"
```

## Task 3: Runnable Signal Sweep Benchmark Fixture

**Files:**

- Modify: `scripts/game_dev_ab_benchmark.py`
- Modify: `scripts/test_game_dev_ab_benchmark.py`
- Modify: `docs/game-dev-ab-benchmark.md`

**Interfaces:**

```python
def write_starter_project(project_dir: Path) -> None: ...

def write_artifacts(
    output_dir: Path,
    *,
    baseline_prompts: list[str],
    cached_prompts: list[str],
) -> None: ...
```

`write_artifacts` additionally creates `baseline-project/` and
`cached-harness-project/` from byte-identical starter files. It never invokes a
model.

- [ ] **Step 1: Write the failing identical-starter test**

```python
def test_artifacts_include_identical_runnable_starters(self) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        bench.write_artifacts(root, baseline_prompts=["base"], cached_prompts=["cached"])
        baseline = root / "baseline-project"
        cached = root / "cached-harness-project"
        for relative in ("package.json", "index.html", "src/main.js"):
            self.assertEqual(
                (baseline / relative).read_bytes(),
                (cached / relative).read_bytes(),
            )
        package = json.loads((baseline / "package.json").read_text(encoding="utf-8"))
        self.assertEqual(package["scripts"]["test"], "node --test")
```

Run the test. Expected: FAIL because neither project directory exists.

- [ ] **Step 2: Implement the minimum fixed scaffold and interface contract**

Generate dependency-free `package.json`, semantic `index.html`, and
`src/main.js`. The main module imports fixed contracts from
`src/game/engine.js`, `src/ui/app.js`, and `src/session/records.js`. Extend the
shared brief with those exact filenames and exported entry points, state that
the design is approved, and forbid nested delegation. Add `src/main.js`,
`index.html`, and `package.json` to the integration worker's allowed scope so it
can repair only final wiring in both modes.

- [ ] **Step 3: Verify prompt economics did not regress**

Run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts/test_game_dev_ab_benchmark.py
python3 scripts/game_dev_ab_benchmark.py --format markdown
```

Expected: all tests pass, cache-adjusted savings remain at least `30%`, stable
prefix ratio remains at least `45%`, and raw/cache-adjusted/runtime claims stay
separate.

- [ ] **Step 4: Update the benchmark protocol and commit**

Document the two starter directories, same-starting-state rule, sequential
write policy, same-model rule, and the fact that Dashboard assets consume only
the Harness database. Commit:

```bash
git add scripts/game_dev_ab_benchmark.py scripts/test_game_dev_ab_benchmark.py \
  docs/game-dev-ab-benchmark.md results-dashboard-implementation.md
git commit -m "feat: make the Signal Sweep benchmark runnable"
```

## Task 4: Complete Real Signal Sweep A/B and Dashboard Preview

> Historical RED execution record: Steps 2–5 below describe the rejected
> one-Session/three-follow-up experiment, not the current runtime policy. Do
> not repeat this topology for normal routing; derive and batch durable queued
> work first, then allow only exact-usage, bounded later follow-ups.

**Generated working directory:** `/tmp/signal-sweep-ab-20260715`

**Durable output:**
`docs/benchmarks/2026-07-15-signal-sweep-real-ab.md`

**Session budget:** Baseline run permits four serial fresh sessions. Harness run
permits one open session and four assignments through three accepted
follow-ups. Each run stays within the skill's four-session budget; no sessions
overlap and nested delegation is forbidden.

- [ ] **Step 1: Generate and checkpoint identical starting projects**

```bash
scripts/build-harnessctl.sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/game_dev_ab_benchmark.py \
  --output-dir /tmp/signal-sweep-ab-20260715 \
  --output /tmp/signal-sweep-ab-20260715/offline-report.json \
  --format json
```

Initialize each generated project as an independent Git repository with the
same author metadata and one `benchmark: checkpoint starter` commit. Record both
starter tree hashes in the implementation report and require equality.

- [ ] **Step 2: Run four serial baseline sessions**

For `worker-01` through `worker-04`, invoke a fresh session in
`baseline-project` with the matching baseline prompt, `--json`, model
`gpt-5.6-sol`, reasoning effort `medium`, `workspace-write`, and the benchmark
directory as an added writable root. Save each JSONL stream and final message
under `/tmp/signal-sweep-ab-20260715/runtime/baseline/`. Wait for every process
and record its report before starting the next session. Never resume a baseline
session.

- [ ] **Step 3: Run one Harness session with three accepted follow-ups**

Invoke `cached_harness/worker-01.prompt` in `cached-harness-project` with the
same model, reasoning, sandbox, and writable roots. Parse the emitted
`thread.started.thread_id`, record it in the Harness ledger, and use that exact
ID with `codex exec resume` for workers 02-04. Wait, verify, and release the
session after every assignment; increment reuse only after the host accepts the
follow-up. Close the session after worker 04.

- [ ] **Step 4: Normalize actual telemetry without claiming missing fields**

For each JSONL stream, read the final `turn.completed.usage` object. Emit four
lifecycle observations per assignment (`spawned` or `reuse`, `running`,
`reported`, `closed`) and attach provider-reported input/output values exactly
once. If the public stream omits usage, read the matching persisted Codex
session's final `token_count.last_token_usage`; if neither exists, keep the
numeric fields absent. Regenerate the benchmark report with `--observations`.

- [ ] **Step 5: Apply identical quality gates**

In both generated projects run `npm test`, serve the directory locally, verify
the entry page and module assets over HTTP, capture Firefox headless screenshots
at `1280x800` and `390x844`, and run the scripted interaction test covering
start, move, pause, game-over, and restart. Record each command, exit status,
and screenshot path. Only compare token totals when all gates pass in both
modes.

- [ ] **Step 6: Populate and launch the Harness-only Dashboard preview**

Record the four Harness tasks, one reused session, real requested/actual model,
actual available usage, lifecycle activity, and final terminal state in a fresh
preview database. Run `audit`, then `run update --status complete`. Launch the
rebuilt Dashboard on `127.0.0.1:7347`. Confirm `/api/status` contains no
baseline data.

- [ ] **Step 7: Inspect desktop and compact Dashboard screenshots**

Capture zh-CN and en-US screenshots at desktop and compact widths with Firefox
headless. Inspect them visually for hierarchy, clipping, state legibility,
liquid-glass restraint, and first-viewport information density. Fix any defect
through a new failing contract test before changing production assets.

- [ ] **Step 8: Write and commit sanitized A/B evidence**

The durable report includes starting-tree equality, model/profile, assignment
and session topology, raw and cache-adjusted estimates, provider-observed usage
with unknown fields labeled, quality-gate results, and interpretation limits.
It links no raw prompts, session logs, or generated source. Commit:

```bash
git add docs/benchmarks/2026-07-15-signal-sweep-real-ab.md \
  results-dashboard-implementation.md
git commit -m "test: record real Signal Sweep token evidence"
```

## Task 5: Independent Review, Release Verification, and Final Audit

**Files:**

- Modify when required: files named by verified Critical/Important findings
- Modify: `results-dashboard-implementation.md`
- Modify when contract coverage changes: `scripts/verify.sh`

- [ ] **Step 1: Run focused and full deterministic verification**

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts/test_game_dev_ab_benchmark.py
cargo fmt --check --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
cargo test --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
cargo clippy --manifest-path skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml -- -D warnings
node --check skills/cached-subagent-harness/scripts/harnessctl/assets/app.js
scripts/verify.sh
git diff --check
```

Expected: every deterministic check passes without warnings.

- [ ] **Step 2: Request independent review using existing reviewer capacity**

Give the reviewer only the approved spec path, this plan path, implementation
report path, base commit, and final diff package. Require separate spec and code
quality verdicts. Do not create a new reviewer when an existing idle reviewer
is available.

- [ ] **Step 3: Fix all Critical and Important findings in one bounded pass**

Write a failing regression test for each verified behavior finding, make the
minimum fix, rerun covering tests, append evidence, and commit once. Re-review
the complete fix set. Explicitly escalate any finding that cannot be closed.

- [ ] **Step 4: Run lifecycle and product-boundary audits**

Require every A/B session to be closed or carry a truthful terminal reason.
Run the implementation ledger audit. Search served product assets and API JSON
for baseline/experiment presentation and sensitive fields. Confirm the installed
Skill was not modified.

- [ ] **Step 5: Record final evidence and leave the preview running**

Append commits, tests, review verdicts, open risks, A/B interpretation, preview
URL/PID/database, and lifecycle audit to the report. Commit the final report,
verify a clean worktree, and return the Dashboard URL plus the most important
Token result to the user.
