---
name: cached-subagent-harness
description: Use when an agentic CLI coordinates subagents for long-running development work where token budget, stable prompts, repo-backed lifecycle tracking, complete-development gates, or resume-safe handoffs matter.
---

# Cached Subagent Harness

Use this skill before dispatching subagents for long-running development work.
This skill owns the controller protocol: PSOC, work packaging, agent budget,
repo-backed lifecycle state, prompt discipline, role gates, review, and final
audit.

Standalone is the normal operating mode. The built-in method in
references/standalone-methodology.md owns PSOC, bounded work, test-first
behavior changes, review, verification, and lifecycle audit. Optional
methodology adapters load only when explicitly enabled and only at the phase
where their context is useful. Their absence is not degraded mode.

## Non-negotiable Invariants

These invariants are the skill's constitution. New features, host adapters,
methodology adapters, routing policies, dashboards, and optimization work must
preserve them. When a lower-priority feature conflicts with an invariant, the
invariant wins.

### P0: Correctness and control-plane safety

1. **Harness first.** Every long task has a brief, durable report, budget,
   lifecycle state, gates, and final audit. Record a durable queued task and
   dispatch decision before invoking a host.
2. **PSOC first.** Define Problem, Scenarios, Options, and Chosen Plan before
   worker code. When evidence invalidates PSOC, return to the earliest invalid
   section before continuing. Resolve internal engineering corrections
   autonomously; ask the user when the loop changes product behavior or scope,
   contradicts the approved plan, or exposes a product decision the controller
   cannot resolve.
3. **Complete development.** Do not use `MVP` or token pressure to skip required
   behavior, tests, error handling, integration, documentation, review,
   verification, or cleanup.
4. **Explicit write scope.** Every writer has bounded allowed paths. Read-only
   roles remain read-only except for schema-limited telemetry about their own
   session or assignment.
5. **Protect the control plane.** Skill files and references, harness source and
   runtime, prompt templates, policies, ledgers, routing state, and
   agent-management rules are immutable to delegated agents unless an approved
   worker brief grants exact paths and validation duties.
6. **Independent gates.** A writer cannot approve its own high-risk work.
   Architecture boundaries, workflow or service contracts, shared data models,
   connectors or repositories, phase-end work, and whole-branch work always
   require independent review. Critical and Important findings are fixed or
   explicitly escalated.
7. **Evidence before completion.** Every reported writer or fixer assignment is
   waited and consumed, runs the project harness, records its report and commit
   checkpoint, and passes its deterministic dispatch gate before session reuse.
   Assignment acceptance additionally requires its configured independent
   review or quality gate. Relevant focused tests, review status, and lifecycle
   audit support any completion claim.

### P1: Lifecycle and concurrency discipline

8. **Durable state is authoritative.** Resume and compaction recover from the
   repository-backed report and machine ledger, not conversation memory. A
   planned row exists before spawn and is updated immediately after every host
   lifecycle result.
9. **Read-heavy parallel, write-heavy serial.** Independent read work may run in
   parallel. Only one assignment may actively write to overlapping scope at a
   time. Reusing one writer session across compatible assignments does not
   violate this rule.
10. **Close deliberately.** Close expired, failed, abandoned, cancelled, and
    superseded sessions promptly. Every temporary or replacement session has an
    explicit expiry predicate before spawn; when it fires, close that session
   before further dispatch. Keep a session open only while its compatibility
   signature and known near-term work make reuse more valuable than closure. Final audit closes or
    explicitly finalizes every session.
11. **No uncontrolled fan-out.** Nested delegation remains disabled unless the
    user explicitly authorizes it and the budget records the reason.
12. **Budget every session.** Initial per-run defaults remain at most two open
    delegated sessions and four total spawned sessions. Idle reusable sessions
    count against the open limit. Raising either limit requires an
    evidence-backed budget.

### P2: Context and token discipline

13. **Information density first.** Prefer structured facts, paths, status,
    evidence, and decisions over repeated narrative. Optional methodology
    guidance loads only on entry to the relevant phase and only when its context
    cost is justified; never bulk-load it at startup.
14. **Stable prompt prefixes.** Stable role policy precedes the dynamic marker;
    task-specific values stay in the dynamic tail. Pass large artifacts by path.
    Reviewers receive brief, report, and review-package paths and do not
    rediscover context already present there. Agents write full file reports and
    return only compact status and report location to the controller.
15. **Subagents are investments.** Spawn only for real parallelism, context
    isolation, capability separation, or independent judgment. Batch or reuse
    related small assignments when that lowers complete-development cost.
16. **Quality-constrained optimization.** Select the lowest model and reasoning
    profile that satisfies role, risk, uncertainty, and quality floors. Count
    retries, escalation, review, and fixer work in total token use.

### P3: Portability, truth, and stable identity

17. **Requested is not actual.** Record requested and observed host, model,
    reasoning, budget, status, and usage separately.
18. **Unknown is honest.** Unsupported or unavailable telemetry remains
    `unknown`; never convert it to zero, success, or an inferred fact.
19. **Facts do not depend on an LLM.** Validated host results, lifecycle
    operations, and compact current state produce dashboard facts. The display
    never guesses missing state.
20. **Stable names, no version suffixes.** Keep skill, role, agent/session
    profile, and policy names stable. Unique session IDs and versions are data,
    not name suffixes.

Every implementation increment must map tests and acceptance evidence back to
these numbered invariants.

## Controller Loop

Before writer code, create or confirm a durable brief with:

```text
Problem:
Scenarios:
Options:
Chosen Plan:
```

If exploration, implementation, tests, or review invalidate an earlier PSOC
section, return to the earliest invalid section and revise it before continuing.
Resolve engineering corrections autonomously. Ask the user only for product
behavior, scope, an approved-plan contradiction, or an unresolved product
decision.

After PSOC:

- choose bounded work packages and decide whether compatible assignments can
  share one worker batch;
- require test-first implementation for behavior changes;
- wait for and consume every writer or fixer report, then run focused tests and
  the project harness and record the report and commit checkpoint;
- apply an independent review whenever a mandatory trigger is present, batch
  all Critical and Important findings into one fixer pass, and re-review;
- run the final lifecycle audit before claiming completion.

Use `references/standalone-methodology.md` for the complete built-in method and
`references/gates.md` for the executable gate flow.

Use the bundled Rust runtime when available:

```text
scripts/bin/harnessctl render-prompt
scripts/bin/harnessctl check-prompt
scripts/bin/harnessctl init
scripts/bin/harnessctl decide
scripts/bin/harnessctl status
scripts/bin/harnessctl watch
scripts/bin/harnessctl dashboard
scripts/bin/harnessctl audit
```

`scripts/harnessctl/` contains the Rust source. `scripts/bin/harnessctl` stores
durable run, task, session, usage, and activity state in the SQLite database selected by `--db`. Existing
Python helpers are legacy development aids. If a required harness/runtime
capability is unavailable, use only a documented equivalent that preserves the
gates and record the actual degraded capability in the report.

## Agent Budget and Lifecycle

Before any spawn, record the agent budget, a durable `planned` ledger row, and
spawn authorization. Defaults are at most two open delegated sessions and four
total spawned sessions per run; idle reusable sessions count against the open
limit. Raise either limit only with an evidence-backed budget. Independent
read-heavy roles may run in parallel. Write-heavy assignments with overlapping
scope remain serial. Nested delegation is disabled unless the user explicitly
authorizes it and the budget records why.

The repository-backed report and compact SQLite state are authoritative for every
harness-created session. Tasks progress through `queued`, `running`, `reported`,
and `accepted`; sessions progress through `starting`, `busy`, `idle`, and
`closed`, or terminate as `failed`/`unknown` with `final_reason`. Completed or closed sessions may remain visible in a host UI. When the
platform cannot list sessions, reconcile user-reported external unknowns only
when they affect budget, cleanup, or correctness; never pretend the harness can
close an unknown handle.

Give every temporary or replacement session an expiry predicate before spawn,
such as `superseded_by:<agent_id>` or `expires_when:original_resumed`. Close it
before further dispatch when that predicate fires. Keep a session open only
while its exact compatibility signature and known near-term work make reuse
more valuable than closure.

Determine role, risk, uncertainty, and quality floors before optimizing token
cost. Security-sensitive, destructive, and control-plane changes have a deep
capability floor. Focused tests and available retry time do not lower that
floor. Select the lowest eligible model and reasoning profile only after these
floors are fixed, and count retries, escalation, review, and fixer work in total
cost.

See `references/report-contracts.md` for ledger fields, report fields, status
names, and audit exceptions.

## Role Gates

- `discussion`: read-only product, architecture, or skill discussion. It may
  ask questions and propose changes but cannot edit, commit, or mutate skills.
- `explorer`: read-only context gathering for unknown code, dependency mapping,
  or risk discovery. It reports paths, interfaces, risks, and test targets.
- `worker`: the normal write role. It receives bounded
  `ALLOWED_WRITE_PATHS`, follows TDD for behavior changes, runs focused tests,
  commits completed changes, writes a full file report, and returns compact
  status.
- `reviewer`: read-only independent judgment against brief, report, and review
  package paths. It does not rediscover context or approve its own work.
- `fixer`: one bounded write pass for the complete Critical and Important
  findings set, followed by covering tests, report append, commit, and re-review.

Discussion, explorer, and reviewer roles use `ALLOWED_WRITE_PATHS=none`, except
for schema-limited telemetry about their own session or assignment. A worker or
fixer may edit control-plane files only when an approved brief names the exact
paths, grants them in `ALLOWED_WRITE_PATHS`, and states their validation duties.

## Prompt Discipline

Use the stable role prefix from `references/prompt-layering.md`. Put
task-specific values only after the exact marker:

```text
--- DYNAMIC TASK CONTEXT ---
ROLE=worker
TASK_BRIEF_PATH=...
REPORT_PATH=...
AGENT_LEDGER_PATH=...
BASE_COMMIT=...
REVIEW_PACKAGE_PATH=...
ALLOWED_WRITE_PATHS=...
```

Pass full plans, diffs, long logs, and historical summaries by path rather than
pasting them. Reviewers receive brief, report, and review-package paths. Agents
write full reports to files and return only compact status, commit, tests,
risks, and report location.

## Session Reuse Boundary

Batch compatible assignments when role, required profile, risk, write scope,
base revision, dependency order, and review boundary align. Reuse is supported
only when an exact session signature matches and the runtime atomically claims
an `idle` session as `busy`. Record `reuse_count` only after the host accepts the
follow-up. If a host lacks follow-up support, report reuse as unsupported and
use a bounded batch or a new session. Never keep an unrestricted permanent role
pool; write-heavy execution remains serial.

## Completion Gate

A task is not complete until:

- the report records PSOC, agent budget, lifecycle ledger, status, write scope,
  files changed, commits, tests, review findings, risks, degraded capability
  notes when applicable, and final audit;
- every reported writer or fixer has been waited and consumed, and its focused
  tests, project harness, report, commit checkpoint, and deterministic dispatch
  gate are recorded;
- every configured independent review or quality gate passes, and all Critical
  and Important findings are fixed or explicitly escalated;
- every harness-created session is `closed`, or is `failed`/`unknown` with
  `final_reason` and next action;
- every temporary or replacement session is validly busy/idle for known
  compatible work or closed when its expiry predicate fires;
- the controller runs the final lifecycle audit and records progress in durable
  repository-backed state.

Optional methodology absence is normal and creates no degraded-mode entry. See
`references/report-contracts.md` for the exact final-audit exceptions.
