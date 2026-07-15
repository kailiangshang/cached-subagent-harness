# Results Dashboard Implementation Report

Status: IN_PROGRESS

## PSOC

### Problem

The embedded Dashboard is functionally correct but visually generic, uses a
weak information hierarchy, cannot show a truthful terminal run state or
persisted freshness, and lacks phase-level token evidence. Its demo data is not
connected to the repository's realistic Signal Sweep workload.

### Scenarios

1. An active Harness run shows outcome, current task, session/model, blockers,
   and last persisted change in the first viewport.
2. One compatible session visibly carries several accepted assignments.
3. Exact, partial, estimated, unsupported, and unknown token facts remain
   distinguishable through the public DTO and both locales.
4. A completed run becomes durably `complete` only after lifecycle audit.
5. A separate full Signal Sweep A/B run proves effectiveness without exposing
   baseline data in the product Dashboard.

### Options

1. Add a dedicated A/B page: direct evidence, but wrong product boundary.
2. Build generic multi-run comparison: flexible, but over-designed for the
   token-efficiency goal.
3. Build a results-first single-run Dashboard and keep A/B evidence external.

### Chosen Plan

Use option 3. Preserve the five-table store and shared `StatusView`; add only
typed run terminal state, truthful persisted freshness, and per-phase token
totals. Redesign embedded assets without a frontend framework. Make the
existing Signal Sweep benchmark runnable, then execute equal-quality isolated
baseline and Harness paths and retain only sanitized aggregate evidence.

## Agent Budget

- Implementation: zero fresh delegated sessions; controller executes serially.
- Independent review: reuse an existing reviewer when available; no new
  reviewer unless the required gate otherwise cannot be satisfied.
- Signal Sweep baseline run: maximum open sessions `1`, maximum total sessions
  `4`, all serial and fresh by benchmark definition.
- Signal Sweep Harness run: maximum open sessions `1`, maximum total sessions
  `1`, with three accepted follow-up assignments.
- Nested delegation: disabled.
- Rationale: the A/B session topology is the feature under measurement; all
  production writes otherwise remain on the controller to avoid churn.

## Agent Ledger

| handle | role | task | status | report_path | spawned_at | waited | closed | write_scope | token_risk | final_reason | next_action |
|---|---|---|---|---|---|---|---|---|---|---|---|
| baseline-worker-01 | worker | Signal Sweep engine baseline | planned | `/tmp/signal-sweep-ab-20260715/baseline-worker-01-report.md` | — | no | no | `src/game,tests/game` | high: fresh bootstrap | — | spawn only in A/B gate |
| baseline-worker-02 | worker | Signal Sweep UI baseline | planned | `/tmp/signal-sweep-ab-20260715/baseline-worker-02-report.md` | — | no | no | `src/ui,src/styles,tests/ui` | high: fresh bootstrap | — | spawn after worker 01 closes |
| baseline-worker-03 | worker | Signal Sweep records baseline | planned | `/tmp/signal-sweep-ab-20260715/baseline-worker-03-report.md` | — | no | no | `src/session,tests/session` | high: fresh bootstrap | — | spawn after worker 02 closes |
| baseline-worker-04 | worker | Signal Sweep integration baseline | planned | `/tmp/signal-sweep-ab-20260715/baseline-worker-04-report.md` | — | no | no | `tests,docs/benchmarks,src/main.js,index.html,package.json` | high: fresh bootstrap | — | spawn after worker 03 closes |
| harness-worker | worker | Signal Sweep four-slice Harness run | planned | `/tmp/signal-sweep-ab-20260715/cached-worker-reports.md` | — | no | no | bounded union of four ordered slices | medium: one bootstrap plus three follow-ups | — | spawn only in A/B gate |

## Write Scope

- Controller: approved implementation-plan paths only.
- A/B workers: exact paths recorded in generated worker prompts and the ledger
  above.
- Existing reviewers: `none` (read-only).
- Control-plane edits: explicitly authorized by the approved design and plan;
  covering tests and full release verification are mandatory.

## Decision Log

- 2026-07-15: Product UI shows only Harness results; A/B is external evidence.
- 2026-07-15: Use inline execution and do not create fresh implementation
  agents.
- 2026-07-15: Work in the current clean `main` checkout under explicit user
  authorization, matching all prior delivery commits.
- 2026-07-15: Stop the old preview while replacing the embedded binary; restore
  a preview after final verification.

## Evidence

- Approved design: `docs/specs/2026-07-15-results-dashboard-design.md`.
- Execution plan:
  `docs/plans/2026-07-15-results-dashboard-and-signal-sweep-plan.md`.
- Design commit: `c063b40`.
- Plan commit: `86d534e`.
- Baseline `scripts/verify.sh`: PASS on 2026-07-15.
- Baseline counts: Rust 35/35; Python 22/22; release metadata, Clippy,
  standalone install, prompt cache, token-effectiveness, game A/B, and final
  lifecycle audit passed.
- An initial baseline rerun stopped at binary copy because the old preview PID
  held the destination executable. `/proc/3377341/exe` confirmed the exact
  inode; after deliberately stopping that preview, the identical verification
  command passed. No source fix was required.
- Task 2 preview served successfully from the rebuilt embedded binary on
  `127.0.0.1:7347`; `/health` and `/api/status` returned the expected local
  run state.
- Desktop zh-CN, desktop en-US, and compact 390px screenshots were visually
  inspected for hierarchy, clipping, bilingual copy, first-viewport density,
  and restrained Moonlight Indigo glass treatment. Firefox 152 could not map
  its software framebuffer in this environment, so the installed Playwright
  Chromium binary was used for this visual-only checkpoint; the Web runtime
  remains browser-independent.

## Changed Files

- `docs/specs/2026-07-15-results-dashboard-design.md`
- `docs/plans/2026-07-15-results-dashboard-and-signal-sweep-plan.md`
- `results-dashboard-implementation.md`
- `skills/cached-subagent-harness/scripts/harnessctl/src/domain.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/store.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/accounting.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/status.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/main.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/dashboard.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/assets/index.html`
- `skills/cached-subagent-harness/scripts/harnessctl/assets/styles.css`
- `skills/cached-subagent-harness/scripts/harnessctl/assets/app.js`

## Tests

- `scripts/verify.sh` — PASS before implementation.
- Task 1 RED: compilation failed on missing `RunStatus`, `update_run`,
  `updated_at`, and `phase_totals`, as intended.
- Task 1 accounting RED: estimated phase quality rendered `partial`, proving
  the new quality-preservation assertion exercised missing behavior.
- `cargo test ... accounting::tests` — PASS, 7 tests.
- `cargo test ... store::tests` — PASS, 8 tests.
- `cargo test ... status::tests` — PASS, 1 test.
- `cargo test ... tests::run_update_command_marks_an_audited_run_complete` —
  PASS after a missing-command RED compile failure.
- `cargo test --manifest-path .../Cargo.toml` — PASS, 40 tests.
- `cargo clippy --manifest-path .../Cargo.toml --all-targets -- -D warnings` —
  PASS.
- Task 2 RED: the embedded-page contract first failed on the missing results
  hierarchy; after the page replacement, a focused regression RED failed on
  the missing `language-next` DOM target used by the language toggle.
- `cargo test ... dashboard::tests` — PASS, 2 tests.
- `node --check .../assets/app.js` — PASS.
- `cargo test --manifest-path .../Cargo.toml` — PASS, 40 tests after Task 2.
- `cargo fmt --check --manifest-path .../Cargo.toml` — PASS after Task 2.
- `cargo clippy --manifest-path .../Cargo.toml --all-targets -- -D warnings` —
  PASS after Task 2.
- `git diff --check` — PASS after Task 2.

## Review Findings

Pending implementation and independent review.

## Risks

- Live Codex A/B turns consume real tokens; keep both paths serial, identical,
  and bounded.
- Provider telemetry may omit fields; missing values will remain unknown.
- Firefox screenshots can prove layout but not replace scripted interaction
  tests.

## Next Actions

1. Add the failing identical-starter Signal Sweep fixture test.
2. Implement and verify the dependency-free runnable starter.
3. Execute the isolated equal-quality real A/B.
4. Populate the final Harness-only preview from real run facts.
5. Run independent review, full verification, and final audit.

## External Agent Reconciliation

The environment exposes existing `final_reviewer` and `focused_reviewer`
handles. They were not spawned by this task and remain outside the task budget
until one is deliberately assigned the final read-only review.

## Degraded Mode Notes

None. Standalone methodology and runtime are available.

## Final Audit

Pending.
