# Subagent Session Token Strategy Implementation

Status: in progress

## PSOC

### Problem

The runtime still has Subagents, but public documentation and the Dashboard
mainly expose `Session`. Users can mistake Session for account login or conclude
that the Subagent concept disappeared. The complete Token routing strategy is
distributed across prose and is not visible at the point where current results
are inspected.

### Scenarios

1. A new user can distinguish Run, Task, Subagent, and Session without reading
   implementation code.
2. A Dashboard user sees which Subagent Session is doing which Task and knows
   that the card is a host/model context rather than an authentication session.
3. A user can see the release's Token decision order while clearly separating
   static policy from the latest observed route and current run data.
4. Compact zh-CN/en-US layouts remain readable, truthful, and free of Baseline
   comparison UI or sensitive controller fields.

### Options

1. Add a separate Subagent data model and UI: explicit, but duplicates Session
   lifecycle and expands the control plane.
2. Rename Session to Subagent everywhere: simple, but hides the concrete host
   context and makes bounded reuse hard to explain.
3. Keep Session as durable truth, define Subagent as the logical executor, and
   label the presentation `Subagent sessions`: smallest consistent change.

### Chosen Plan

Use option 3. Add a four-term contract and Token flow to the Skill and public
docs, then add a dependency-free bilingual policy map and Subagent Session note
to the existing Dashboard. Do not change routing, persistence, API shape, or
release limits.

## Agent Budget

- Maximum open delegated Sessions: 2.
- Maximum total spawned Sessions: 2.
- Planned use: one fresh read-only baseline comprehension Session and one fresh
  read-only final review Session. Implementation remains on main to avoid the
  short-lived worker churn this product is designed to prevent.
- Nested delegation: disabled.

## Agent Ledger

| handle | role | task | status | report_path | spawned_at | waited | closed | write_scope | token_risk | session_budget | final_reason | next_action |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| baseline-terminology | discussion | baseline-term-clarity | planned | pending | pending | no | no | none | low | no reuse; close after report | pending | Explain current artifacts without expected-answer hints |

## Write Scope

- Main controller: approved files listed in the implementation plan.
- Baseline and final review Sessions: `none`.

## Decision Log

- 2026-07-16: User approved the four-term model, dependency-free Dashboard map,
  and autonomous detail execution.
- 2026-07-16: Continue on clean `main` under the user's direct-development and
  push authorization; do not create a second worktree.
- 2026-07-16: Do not install or overwrite the user's active Skill.

## Routing and Batch Policy

- Baseline comprehension is an isolated read-only discussion assignment with a
  light floor; it does not share an implementation batch.
- Main performs the tightly coupled documentation/UI edits serially.
- Final review is independent, read-only, and starts only after verification.
- Release defaults remain two Tasks per compatible micro-batch, one accepted
  follow-up, and 200,000 effective Tokens; no increase is proposed.

## Evidence

- User feedback: the current presentation made it appear that the Subagent
  concept may have disappeared.
- Existing README/current-state define Run, Task, and Session, but do not define
  Subagent as a separate logical concept or show the complete decision flow at
  the Dashboard.
- Approved design:
  `docs/specs/2026-07-16-subagent-session-token-strategy-design.md`.
- Implementation plan:
  `docs/plans/2026-07-16-subagent-session-token-strategy-plan.md`.

## Changed Files

Pending.

## Tests

Pending RED/GREEN evidence and full verification.

## Review Findings

Pending independent whole-diff review.

## Risks

- Static policy could be mistaken for live progress; label it explicitly.
- Extra explanatory UI could reduce density; keep one compact map and one note.
- Terminology could diverge between locales; enforce exact asset markers.

## Next Actions

1. Initialize durable runtime state and record the planned baseline Task.
2. Run clean baseline tests and the fresh comprehension scenario.
3. Execute the TDD plan, visual audit, full verification, and independent review.

## External Agent Reconciliation

Only Sessions created for this increment count toward this report. Previously
completed review threads are outside this Run and require no cleanup.

## Degraded Mode Notes

None.

## Final Audit

Pending.
