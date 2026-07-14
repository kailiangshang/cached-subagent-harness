# Report Contracts

The repo task report is the authoritative state source for long-running work. Subagents write full reports to files and return only a short status line to the controller.

Use `scripts/bin/harnessctl` with a task-local SQLite database for machine-enforced run, task, session, usage, and activity state. The Markdown report remains the human-readable audit surface.

## Task Report

```text
PSOC:
Agent Budget:
Agent Ledger:
Write Scope:
Decision Log:
Evidence:
Changed Files:
Tests:
Review Findings:
Risks:
Next Actions:
External Agent Reconciliation:
Degraded Mode Notes:
Final Audit:
```

Keep entries dense. Link file paths for details; do not paste long logs, diffs, plans, or histories.

Optional methodology absence is not degraded. Populate `Degraded Mode Notes`
only when a required harness/runtime capability is unavailable or an explicitly
requested methodology adapter fails.

## Agent Ledger

Every harness-created agent must have a row:

```text
handle | role | task | status | report_path | spawned_at | waited | closed | write_scope | token_risk | final_reason | next_action
```

Create and update machine rows with `harnessctl task add|update` and
`harnessctl session record|accept-followup|release|close`.

Allowed statuses:

```text
planned
spawned
running
reported
closed
failed
abandoned
externally-unknown
```

Lifecycle rules:

- write `planned` before spawn;
- update to `spawned` or `running` immediately after spawn;
- after wait, set `reported`, fill `report_path`, and summarize `next_action`;
- after close, set `closed` and `closed=yes`;
- use `failed`, `abandoned`, or `externally-unknown` only with `final_reason`.
- set `write_scope=none` for read-only roles; set explicit paths for `worker` and `fixer`.
- for temporary replacement agents, record the expiry condition in `next_action`, then close the agent when it is superseded.

## Status Names

- `DONE`: task completed, tests reported, no known blocker.
- `DONE_WITH_CONCERNS`: task completed, but there are risks or follow-up questions.
- `LOOP_REQUIRED`: PSOC became invalid and the controller must revise the brief or decision.
- `NEEDS_CONTEXT`: missing information prevents progress.
- `BLOCKED`: cannot continue after reasonable local investigation.

## Worker Report

```text
Status:
Problem:
Scenarios:
Options:
Chosen Plan:
Allowed Write Paths:
Files Changed:
Tests:
Commits:
Risks:
Follow-up:
```

Keep entries concise. Put long logs in separate files and link their paths.

## Explorer Report

```text
Status:
Question Investigated:
Relevant Files:
Interfaces:
Risks:
Suggested Test Targets:
PSOC Impact:
```

`PSOC Impact` must say whether the current Problem, Scenarios, Options, or Chosen Plan should change.

## Reviewer Report

```text
Spec Verdict:
Quality Verdict:
Findings:
Cannot Verify:
Recommended Gate:
```

Findings must be ordered by severity. Use Critical, Important, Minor, or Note.

## Fixer Report Append

```text
Fix Status:
Findings Addressed:
Files Changed:
Tests:
Commits:
Remaining Risks:
```

Do not create one report per finding. Append one batched fix section to the existing report.

## Final Audit

Before completion, the controller records:

```text
Lifecycle Audit:
Harness Commands:
Focused Tests:
Project Harness:
Review Status:
Open Risks:
External Agent Reconciliation:
Degraded Mode:
```

All ledger rows must be `closed`, or must be `failed`, `abandoned`, or `externally-unknown` with an explicit `final_reason`. UI-visible completed agents are not a failure when the ledger says they were waited and closed. Unknown UI-visible agents are recorded only when the controller cannot inspect them and the user-provided `/agent` reconciliation indicates they matter for budget or cleanup. Temporary replacement agents must be closed as soon as their original agent is resumed or their task is cancelled.
