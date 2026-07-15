# Results Dashboard Implementation Report

Status: COMPLETE

Current entry point: [Current Product State](docs/current-state.md). This report
is the complete delivery and audit trail; its historical retries and interim
review states are retained intentionally.

## PSOC

### Problem

The embedded Dashboard is functionally correct but visually generic, uses a
weak information hierarchy, cannot show a truthful terminal run state or
persisted freshness, and lacks phase-level token evidence. Its demo data is not
connected to the repository's realistic Signal Sweep workload. The completed
live run then exposed a more important core defect: repeated compatible
follow-ups grew cumulative model context until the Harness arm cost 5.90× the
equal-quality Baseline despite a high cache-hit rate.

### Scenarios

1. An active Harness run shows outcome, current task, session/model, blockers,
   and last persisted change in the first viewport.
2. One compatible session visibly carries several accepted assignments.
3. Exact, partial, estimated, unsupported, and unknown token facts remain
   distinguishable through the public DTO and both locales.
4. A completed run becomes durably `complete` only after lifecycle audit.
5. A separate full Signal Sweep A/B run measures effectiveness without exposing
   baseline data in the product Dashboard; a negative result remains evidence
   and loops back into routing policy.
6. Known compatible ready work batches before reuse. Later reuse stops when its
   accepted-follow-up or effective-Token budget is exhausted or unknown.

### Options

1. Add a dedicated A/B page: direct evidence, but wrong product boundary.
2. Build generic multi-run comparison: flexible, but over-designed for the
   token-efficiency goal.
3. Build a results-first single-run Dashboard and keep A/B evidence external.
4. Keep unlimited follow-up reuse because cached-input percentage is high.

### Chosen Plan

Use option 3 for presentation. Preserve the five-table store and shared
`StatusView`; add only typed run terminal state, truthful persisted freshness,
and per-phase token
totals. Redesign embedded assets without a frontend framework. Make the
existing Signal Sweep benchmark runnable, then execute equal-quality isolated
baseline and Harness paths and retain only sanitized aggregate evidence. The
live evidence rejects option 4: route known compatible ready work to one bounded
batch first, and permit a later follow-up only within explicit count and Token
budgets.

## Agent Budget

- Implementation: zero fresh delegated sessions; controller executes serially.
- Independent review: reuse an existing reviewer when available; no new
  reviewer unless the required gate otherwise cannot be satisfied.
- Signal Sweep baseline run: maximum open sessions `1`, maximum total sessions
  `4`, all serial and fresh by benchmark definition.
- Signal Sweep Harness run: maximum open sessions `1`, maximum total sessions
  `1`, with three accepted follow-up assignments in the historical RED arm.
- Corrected runtime default: batch known compatible ready work first; a later
  reusable Session allows one accepted follow-up and 200,000 effective Tokens
  unless an evidence-backed decision changes either limit.
- Nested delegation: disabled.
- Rationale: the A/B session topology is the feature under measurement; all
  production writes otherwise remain on the controller to avoid churn.

## Agent Ledger

| handle | role | task | status | report_path | spawned_at | waited | closed | write_scope | token_risk | session_budget | final_reason | next_action |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `019f64b7-28d6-7cc0-a648-a4f1099a9bcd` | worker | Signal Sweep engine baseline | closed | `/tmp/signal-sweep-ab-20260715/baseline-worker-01-report.md` | recorded | yes | yes | `src/game,tests/game` | realized bootstrap | fresh only | turn complete | none |
| `019f64bc-97d4-7190-8f9d-58b2e2bb8f80` | worker | Signal Sweep UI baseline | closed | `/tmp/signal-sweep-ab-20260715/baseline-worker-02-report.md` | recorded | yes | yes | `src/ui,src/styles,tests/ui` | realized bootstrap | fresh only | turn complete | none |
| `019f64c3-2fcf-7721-9da6-c8791eb75646` | worker | Signal Sweep records baseline | closed | `/tmp/signal-sweep-ab-20260715/baseline-worker-03-report.md` | recorded | yes | yes | `src/session,tests/session` | realized bootstrap | fresh only | turn complete | none |
| `019f64c6-8cfb-7112-825b-0f6ccf13da8a` | worker | Signal Sweep integration baseline | closed | `/tmp/signal-sweep-ab-20260715/baseline-worker-04-report.md` | recorded | yes | yes | `tests,docs/benchmarks,src/main.js,index.html,package.json` | realized bootstrap | fresh only | turn complete | none |
| `019f64cd-55ab-76a3-b80a-aa6027551a15` | worker | Signal Sweep four-slice Harness run | closed | `/tmp/signal-sweep-ab-20260715/cached-harness-run-report.md` | recorded | yes | yes | bounded union of four ordered slices | realized high context growth | historical 3 follow-ups; now ineligible | package complete | retained as RED evidence |

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
- 2026-07-15: Retain the negative live result. Offline prompt estimates do not
  substitute for complete Codex turn telemetry.
- 2026-07-15: Replace unlimited compatible continuation with batch-first
  routing plus accepted-follow-up and effective-Token budgets.
- 2026-07-15: Normalize Codex provider totals into non-overlapping
  input/cache/output/reasoning categories and retain the read-only retry.

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
- Task 3 generated byte-identical `baseline-project` and
  `cached-harness-project` starter trees with fixed cross-module interfaces.
  Offline prompt economics after the final brief: raw estimated savings
  `39.81%`, cache-adjusted estimated savings `76.84%`, and stable-prefix ratio
  `82.04%`. These remain estimates, not provider telemetry.
- The preflight fairness audit found that the initial cached prompt pointed
  only to the shared design and did not name its exact worker slice. Before any
  live model call, a regression test drove the split into one stable shared
  brief plus four small dynamic assignment briefs; every cached prompt now
  names its worker, task, scope, gate, and valid `BASE_COMMIT=HEAD`.
- Both generated implementations completed from the same starter tree hash
  `2c8858f5c867bf856dc33dd5bfdf9cb1cdaad31f` with requested/actual model
  `gpt-5.6-sol` and medium reasoning. Baseline used four fresh serial Sessions;
  Harness used one Session with three accepted follow-ups and one rejected
  read-only retry.
- Equal quality gates passed: Baseline 21 tests, Harness 30 tests, both syntax
  checks, six required HTTP assets per arm, desktop 1280×800 and compact
  390×844 screenshots, and scripted interaction coverage.
- Exact normalized totals: Baseline `2,974,064`; Harness `17,551,878`, including
  `999,618` retry Tokens. Observed effective saving is `-490.16%`, so the old
  continuation policy is rejected rather than presented as a saving.
- The final Harness ledger contains four accepted tasks, one closed Session
  with `current_task_id=null`, three historical accepted reuses, five exact
  usage rows, and a durably complete run after audit.
- Sanitized evidence:
  `docs/benchmarks/2026-07-15-signal-sweep-real-ab.md`.

## Changed Files

- `docs/specs/2026-07-15-results-dashboard-design.md`
- `docs/specs/2026-07-14-lightweight-token-harness-design.md`
- `docs/plans/2026-07-15-results-dashboard-and-signal-sweep-plan.md`
- `docs/benchmarks/2026-07-15-signal-sweep-real-ab.md`
- `results-dashboard-implementation.md`
- `skills/cached-subagent-harness/SKILL.md`
- `skills/cached-subagent-harness/references/standalone-methodology.md`
- `skills/cached-subagent-harness/references/gates.md`
- `skills/cached-subagent-harness/references/report-contracts.md`
- `skills/cached-subagent-harness/scripts/harnessctl/src/domain.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/bundle.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/store.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/sessions.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/accounting.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/status.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/main.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/src/dashboard.rs`
- `skills/cached-subagent-harness/scripts/harnessctl/assets/index.html`
- `skills/cached-subagent-harness/scripts/harnessctl/assets/styles.css`
- `skills/cached-subagent-harness/scripts/harnessctl/assets/app.js`
- `scripts/game_dev_ab_benchmark.py`
- `scripts/test_game_dev_ab_benchmark.py`
- `scripts/test_standalone_contract.py`
- `scripts/validate-release.py`
- `scripts/verify.sh`
- `docs/game-dev-ab-benchmark.md`

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
- Task 3 RED: artifact test failed because `baseline-project/package.json` did
  not exist; interface test failed because `write_starter_project` did not
  exist.
- Task 3 resume-safety RED: regenerating artifacts overwrote a developed
  `src/main.js`; the minimal fix now writes starter files only when absent.
- Task 4 preflight RED: cached assignment helper was absent, exposing unequal
  worker specificity between A/B arms. The new helper test passes for exact
  worker, slice, task, gate, shared brief, and no-nested-delegation fields.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest
  scripts/test_game_dev_ab_benchmark.py` — PASS, 8 tests.
- `python3 scripts/game_dev_ab_benchmark.py --format markdown` — PASS; all
  configured economics thresholds passed.
- Generated-project smoke: `diff -qr` found no starter differences; `npm test`
  passed in both dependency-free starters and `node --check src/main.js`
  passed.
- Live A/B quality gates: Baseline `npm test` PASS with 21 tests; Harness
  `npm test` PASS with 30 tests. Both arms passed syntax checks, all six
  required HTTP resources, desktop and compact visual inspection, and the
  required start/move/pause/game-over/restart-or-export interaction path.
- Policy-correction RED: the former six-task continuation test encoded the
  rejected unlimited-reuse behavior. Batch-first and dual-budget tests failed
  before `ReuseBudget`, usage-aware claiming, and the new decision order were
  implemented; a focused boundary RED then proved that usage exactly equal to
  the Token cap was still accepted, and the comparison was corrected from
  `>` to `>=`. The obsolete five-follow-up test was removed only after the new
  behavior passed.
- Revision-safety RED: queued base revisions had no state-limited
  compare-and-swap command. The focused store test now proves successful
  refresh, stale expected-revision rejection without mutation, and running-task
  rejection.
- Terminal-session RED: a closed Session could retain `current_task_id`, and a
  deliberately injected legacy terminal row was not rejected by final audit.
  Store update and audit tests now cover both paths.
- Telemetry RED: missing observations collapsed to zero, Codex provider totals
  overlapped cache/reasoning categories, retry rows were not part of total
  cost, and observed savings did not require equal-quality success. Twelve
  Benchmark tests now cover unknown preservation, non-overlapping
  normalization, retry inclusion, worker coverage, and the explicit
  `quality_passed` contract.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest
  scripts/test_game_dev_ab_benchmark.py` — PASS, 12 tests.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest
  scripts/test_standalone_contract.py` — PASS, 9 tests.
- `python3 scripts/validate-release.py .` — PASS.
- `python3 .../skill-creator/scripts/quick_validate.py
  skills/cached-subagent-harness` — PASS.
- `cargo fmt --check --manifest-path .../Cargo.toml` — initial formatting-only
  RED on four changed regions; `cargo fmt` applied the mechanical correction.
- `cargo test --manifest-path .../Cargo.toml` — PASS, 44 tests.
- `cargo clippy --manifest-path .../Cargo.toml --all-targets -- -D warnings` —
  PASS.
- `node --check skills/cached-subagent-harness/scripts/harnessctl/assets/app.js`
  — PASS.
- `git diff --check` — PASS.
- `scripts/verify.sh` — PASS on 2026-07-15 after the final policy and telemetry
  changes; rebuilt the release binary and passed release metadata, Rust 44/44,
  Python 6/6 + 9/9 + 3/3 + 12/12, Clippy, Standalone install, prompt-cache,
  token-effectiveness, game A/B, and final lifecycle audit gates.
- Independent review RED: both reviewers returned `Ready: No`, with zero
  Critical findings. Their overlapping Important findings reduced to five
  root causes: caller-asserted batching, stale/non-exact release evidence,
  cross-run usage ownership, contradictory Session/task state, and incomplete
  Benchmark comparability proof. A sixth policy gap allowed CLI budget raises
  without durable evidence.
- Authoritative-batch RED: three compatible queued tasks plus a falsified ready
  count of one selected `ReuseSession`. The count field is removed; one
  `BEGIN IMMEDIATE` decision now derives the full compatible unassigned queued
  set, and assigned queued tasks cannot be bundled or claimed twice.
- Usage-gate RED: all-numeric partial/estimated/unsupported/unknown rows proved
  a reuse budget, and old exact rows could release a later assignment. Reuse
  now requires every counted row to be exact and complete; release requires a
  complete exact task/session observation strictly after the accepted
  assignment's transactional causal boundary.
- Ownership RED: independently valid foreign keys allowed a usage row to mix a
  Run with another Run's Task or Session. Transactional checks now reject task,
  Session, and linked task/Session mismatches without changing either Run.
- Session-state RED: `idle + current_task`, `busy + no task`, and duplicate task
  ownership were accepted. State-shape validation, unassigned-only linking,
  verified busy-to-idle release, null-only idle claims, and terminal cleanup now
  enforce one owner.
- Budget-policy RED: CLI flags could raise the one-follow-up/200,000-Token
  defaults without persistent evidence. Current runtime flags are lower-only;
  increases are rejected until a future versioned durable policy exists.
- Benchmark-evidence RED: closed-only workers, unknown worker IDs, inconsistent
  provider splits, and one generic quality event could produce comparable exact
  savings. The report now requires every named Worker lifecycle event, exact
  normalized arithmetic, exact expected worker IDs, and one event per named
  quality gate; duplicates and inconsistent splits fail closed.
- `cargo test --manifest-path .../Cargo.toml` — PASS, 50 tests after review
  fixes.
- `cargo clippy --manifest-path .../Cargo.toml --all-targets -- -D warnings` —
  PASS after review fixes.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest
  scripts/test_game_dev_ab_benchmark.py` — PASS, 15 tests after review fixes.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest
  scripts/test_standalone_contract.py` — PASS, 9 tests after review fixes.
- `python3 scripts/validate-release.py .`, Skill quick validation,
  `node --check`, current diff check, and full `8c6263b` range diff check —
  PASS after review fixes.
- `scripts/verify.sh` — PASS on 2026-07-15 after the complete review-fix pass;
  rebuilt the release binary and passed release metadata, Rust 50/50,
  Benchmark Python 15/15, Standalone 9/9, remaining Python 6/6 + 3/3,
  Clippy, prompt-cache, token-effectiveness, game A/B, and final lifecycle
  audit gates.
- Closure re-review found one remaining causal-ordering defect: millisecond
  timestamp equality let pre-accept exact usage release a Session, and release
  did not require the task's durable `reuse_accepted` state. Both reviewers
  reproduced the issue independently; no other prior Important finding
  remained open.
- Causal-release RED: the focused Session regression failed because exact
  usage written after claim but before `accept_followup` released successfully.
  The same test also fixes pre-accept usage to the acceptance millisecond and
  freezes the wall clock behind the boundary.
- Causal-release GREEN: release now requires `reuse_accepted=1`; acceptance
  advances a transactionally serialized logical timestamp beyond all existing
  assignment usage; new usage advances beyond that boundary; release requires
  strict ordering. The focused regression and Rust 50/50 pass without sleeps or
  a schema/table migration.
- `scripts/verify.sh` — PASS on 2026-07-15 after the causal-release fix;
  release metadata, Rust 50/50, Benchmark Python 15/15, Standalone 9/9,
  remaining Python 6/6 + 3/3, Clippy, prompt-cache, token-effectiveness, game
  A/B, release build, and final lifecycle audit all passed.
- Final causal review closed the acceptance/usage defect but found one
  fix-induced freshness regression: terminal `run update` still used raw wall
  time and could overwrite a later logical `updated_at` during clock rollback.
- Terminal-freshness RED fixed the active Run's `updated_at` in the future and
  observed completion write it backward. GREEN now derives terminal
  `updated_at`/`ended_at` strictly after the persisted boundary and writes the
  matching close activity in the same transaction. Rust 50/50, Clippy,
  Standalone 9/9, release validation, and diff check pass.
- `scripts/verify.sh` — PASS on 2026-07-15 after the terminal-freshness fix,
  with the same complete Rust 50/50, Python 6/6 + 9/9 + 3/3 + 15/15, Clippy,
  release metadata/build, cache, Token, Benchmark, and lifecycle gates.

## Review Findings

Initial spec/compliance review by `final_reviewer` and code/security review by
`focused_reviewer`: zero Critical findings, `Ready: No`. All Important findings
were reproduced or verified against source and addressed in one controller fix
pass. The arbitrary budget-raise suggestion was resolved by rejecting raises
instead of adding a migration/table, preserving the lightweight product
boundary. The first closure re-review closed every original finding except the
fresh-usage causal boundary and found no Critical issue. That final defect is
now fixed. The spec reviewer returned `Ready: Yes`; the focused reviewer closed
the causal defect but found a terminal freshness regression introduced by its
logical-clock fix. That regression is now fixed. Both targeted final reviews
returned `Ready: Yes`; Critical, Important, and Minor findings are all zero.

## Risks

- The completed live run disproves the old continuation policy but does not yet
  prove positive savings for the corrected single-batch policy. Release claims
  are deliberately limited to preventing the observed unbounded reuse path.
- Provider telemetry can omit fields; missing values remain unknown and block
  both observed savings claims and Session reuse.
- The visual checkpoint used Playwright Chromium after Firefox software
  framebuffer startup failed in this environment. Scripted interaction and
  HTTP/DOM checks remain browser-independent, but future host/browser variants
  still need their own compatibility evidence.
- The default one-follow-up/200,000-Token limits are conservative safety
  defaults. Raising them requires host- and workload-specific evidence.

## Next Actions

None for this delivery. A separate real run is required before making any
positive saving claim for the corrected bounded policy.

## External Agent Reconciliation

The environment exposes existing `final_reviewer` and `focused_reviewer`
handles. They were not spawned by this task and remain outside the spawn
budget. Both were deliberately reused for read-only closure reviews, waited,
and reported final `Ready: Yes` without modifying the checkout. Their platform
status is completed; no Harness-created reviewer handle remains open.

## Degraded Mode Notes

None. Standalone methodology and runtime are available.

## Final Audit

- Lifecycle Audit: all five live A/B Sessions are closed; the completed Harness
  Run has four accepted Tasks, one closed Session, and no current task. The
  exact ledger passes `harnessctl audit`. Existing reviewer handles are
  reconciled above and completed.
- Harness Commands: focused Rust tests, all Rust tests, Clippy, release
  validation, Standalone contract, exact real-ledger status, and exact
  real-ledger final audit passed.
- Focused Tests: causal acceptance/release, equal-millisecond ordering,
  backward-clock usage ordering, and terminal Run freshness regressions pass.
- Project Harness: final post-review `scripts/verify.sh` — PASS; release
  metadata, Rust 50/50, Python 6/6 + 9/9 + 3/3 + 15/15, Clippy, release build,
  cache/Token/Benchmark gates, and synthetic final lifecycle audit passed.
- Review Status: `final_reviewer` and `focused_reviewer` both `Ready: Yes`;
  zero open Critical, Important, or Minor findings.
- Open Risks: no blocking risk. Positive savings for the corrected bounded
  batch-first policy remain unclaimed until a separate real run measures them.
- External Agent Reconciliation: existing reviewers completed; no fresh Agent
  was spawned for implementation or closure.
- Degraded Mode: none; Standalone runtime and methodology were used directly.
- Dashboard Delivery: the exact completed Signal Sweep ledger is served on
  `127.0.0.1:7347`; `/health` is OK; `/api/status` reports one complete Run,
  four accepted Tasks, one closed Session with no current task, reuse count 3,
  and exact total 17,551,878. Served product assets contain no Baseline/A-B
  comparison surface. Temporary benchmark servers on ports 4174 and 4175 were
  stopped.
