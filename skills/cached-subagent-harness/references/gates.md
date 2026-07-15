# Gates

Use this reference when deciding whether to dispatch a subagent, how many to run, and when to loop.

## Gate -1: Report, Budget, Ledger

Before any dispatch, the controller must create or update the repo report with:

- `Agent Budget`: default max open delegated sessions `2`, default max total spawned sessions `4`, with justification for exceptions.
- `Agent Ledger`: one row per planned or spawned agent.
- `Write Scope`: explicit allowed write paths for any worker or fixer; `none` for read-only roles.
- `Degraded Mode Notes`: present only when a required harness/runtime capability is unavailable or an explicitly requested methodology adapter fails. Optional methodology absence is not degraded.
- `Expiry`: present for temporary or replacement agents, for example `superseded_by:<agent_id>` or `expires_when:original_resumed`.

Do not spawn from memory. The ledger is the source of truth for agents created by this harness.

Use `scripts/bin/harnessctl init --db <path> ...` to create compact machine state. Mirror dense human-readable status into the repo report.

## Gate 0: PSOC

Before implementation, the controller must have:

- `Problem`: the specific defect, capability, or architecture gap.
- `Scenarios`: two to four plausible cases, including the expected normal path and meaningful edge cases.
- `Options`: two to three reasonable implementation approaches with tradeoffs.
- `Chosen Plan`: the selected approach and why it is appropriate.

If any field is missing, do not dispatch a worker.

If exploration changes any field, update the report before dispatching more agents.

## Gate 1: Context

Use an `explorer` only when the controller lacks enough context to write a dense brief.

Explorer constraints:

- read-only;
- no code edits;
- no commits;
- report file output only;
- focus on file paths, interfaces, risks, and test targets.

Read-heavy explorers may run in parallel when they inspect independent areas.

Ledger constraints:

- write a `planned` row before spawn;
- update to `spawned` or `running` immediately after spawn;
- after wait, record report path, short outcome, and next action;
- close the agent after its report is consumed and mark `closed`.
- close temporary replacement agents immediately when their expiry condition becomes true.

Use `harnessctl task add` before dispatch, `session record` after spawn,
task/session updates after report, and `session close` at lifecycle completion.
After a verified commit, refresh a still-valid unassigned queued task through
`task refresh-revision --from-revision OLD --revision NEW`. If intent, scope,
profile, or review boundary changed, revise PSOC and register the task when it
is ready instead of mutating it into compatibility.

Discussion agents are read-only. Use them for product, architecture, or skill discussion only. If they identify a needed edit, they must return a proposed change or brief; the controller decides whether to create a separate worker task.

## Gate 2: Write

Use exactly one write-active `worker` at a time. One worker may execute one
bounded batch of compatible assignments when role, required capability, risk,
write scope, base revision, dependency order, and review boundary align.
Derive all known compatible ready assignments from durable queued state and
batch them before attempting a follow-up; never trust a caller-supplied count.
Every reusable session has an accepted-follow-up cap and total effective token
budget. Runtime flags may lower but not raise the release defaults. Record
complete exact normalized usage linked to the current assignment before
release, and require run/task/session ownership to agree; non-exact usage,
either exhausted budget, or a changed signature makes the session ineligible
for reuse.

Worker constraints:

- follow the brief and PSOC;
- write only inside `ALLOWED_WRITE_PATHS`;
- write failing tests before implementation when behavior changes;
- keep edits scoped;
- run focused tests;
- commit completed changes;
- write the report before returning.

If the worker discovers that the Problem, Scenarios, Options, or Chosen Plan is wrong, it must stop the affected work path and report `LOOP_REQUIRED`.

Until assignment/session lease enforcement lands, keep write-heavy execution
serial. Wait for and consume the current worker's report, then close the worker
or mark it with an exception state before dispatching another worker.

## Gate 3: Harness

The controller runs the project harness after a worker or fixer returns.

If harness fails:

1. classify the failure;
2. update the report with the failing command and short failure summary;
3. dispatch a fixer only if the failure is not a trivial controller mistake;
4. rerun the harness after the fix.

## Gate 4: Review

Use a `reviewer` for:

- architecture boundary changes;
- workflow or service contract changes;
- shared data model changes;
- connector or repository changes;
- phase-end review;
- whole-branch review.

Reviewer input must be file paths: task brief, report, and review package. The reviewer should not receive pasted diffs or long summaries.

Reviewers must not rediscover the whole project when brief, report, and diff package exist.

## Gate 5: Fix

Use one `fixer` for the complete Critical/Important findings list.

Fixer constraints:

- fix only the findings;
- write only inside `ALLOWED_WRITE_PATHS`;
- update or add focused tests;
- run the covering tests;
- append fix results to the existing report;
- commit the fix.

Re-review only after the report contains commands run and outcomes.

## Gate 6: Lifecycle Audit

Before final response, audit the report ledger:

- every harness-created agent is `closed`; or
- the row is `failed`, `abandoned`, or `externally-unknown` with `final_reason` and next action.
- every temporary replacement agent spawned in the current turn is either the active chosen agent or is closed as superseded.
- every busy session has one current task; every idle or terminal session has
  no current assignment.

Completed or closed agents may remain visible in UI. If the platform lacks agent listing, audit the handles recorded by this harness. If the user or UI reports additional unknown agents that affect budget or cleanup, request one `/agent` reconciliation and record unknown handles as `externally-unknown`.

Run `scripts/bin/harnessctl audit --db <path> --run <id>` before claiming completion.

## Control Plane Safety

The harness skill, skill references, prompt templates, lifecycle ledgers, and agent-management rules are control plane. Treat them as read-only for `discussion`, `explorer`, and `reviewer` roles. A `worker` or `fixer` may edit control-plane files only when:

- the brief explicitly names the control-plane file paths;
- `ALLOWED_WRITE_PATHS` includes those paths;
- the report records why the edit is needed and how it was validated.

## PSOC Loop

The loop can restart at any earlier section:

- New evidence changes the bug or capability definition: return to `Problem`.
- An uncovered edge case changes expected behavior: return to `Scenarios`.
- A chosen design becomes unsafe or too broad: return to `Options`.
- Implementation details need adjustment but the strategy is valid: revise `Chosen Plan`.

Continue autonomously when the correction is internal engineering detail. Escalate to the user when the loop changes product behavior, project scope, or contradicts the approved plan.
