# Standalone Methodology

## PSOC Loop

Record Problem, Scenarios, Options, and Chosen Plan before writer code. Return
to the earliest invalid section when evidence changes it. Continue autonomously
for engineering corrections; ask the user only for product behavior, scope, or
approved-plan contradictions the controller cannot resolve.

## Work Packages and Compatible Batching

Group related assignments only when role, required capability, risk, write
scope, base revision, dependency order, and independence boundary are
compatible. Execute trivial work on main. Reuse only after an exact signature
match and an atomic `idle` to `busy` claim; increment reuse only after the host
accepts the follow-up. When a host cannot follow up, use one bounded worker
brief and report reuse as unsupported. Never emulate reuse with an unrestricted
permanent role pool.

## Quality-Constrained Routing

Set role, risk, uncertainty, and quality floors before choosing a model or
reasoning profile. Security-sensitive, destructive, and control-plane changes
require deep. Strong tests and retry capacity do not lower that floor. Only
after the floors are fixed may the controller select the lowest eligible route;
total cost includes retries, escalation, review, and fixer work.

## Test and Harness Gate

Behavior changes are test-first. Every writer or fixer writes a file report and
returns compact status. The controller waits, consumes the report, runs focused
tests and the project harness, and records the commit checkpoint before
acceptance or another writer assignment.

## Independent Review

Architecture boundaries, workflow or service contracts, shared data models,
connectors or repositories, phase-end work, and whole-branch work require an
independent reviewer. A writer or fixer cannot review its own work. Batch all
Critical and Important findings into one fixer pass, then re-review.

## Optional Methodology Adapters

Standalone is complete without another methodology. An explicitly enabled
adapter may provide compatible planning, TDD, review, or finishing artifacts.
Load it only when that phase begins and its context cost is justified. Adapter
absence when not requested is normal. An explicitly requested adapter failure
is visible, but it does not make the standalone core degraded.

## Quick Reference

| Decision | Required action |
|---|---|
| Trivial, no isolation value | Execute on main and record the assignment. |
| Compatible micro-work, host cannot follow up | Batch into one bounded worker brief. |
| Incompatible role, model, risk, scope, base, or review boundary | Use an isolated execution path. |
| Mandatory review trigger | Create an independent reviewer assignment. |
| Missing optional methodology | Continue standalone without degraded mode. |

## Rationalization Check

| Rationalization | Contract |
|---|---|
| One plan item needs one fresh agent | Assignment boundaries are not session boundaries; batch compatible work. |
| Superpowers is missing, so quality is degraded | The standalone kernel owns the complete gates. |
| The cheapest model always saves tokens | Count retries, escalation, review, and fixer work. |
| Token pressure justifies skipping a gate | Complete development is a quality floor. |
| An idle agent might be useful later | Keep it only for known compatible near-term work. |

## Red Flags

- Fresh agents for compatible micro-assignments without a recorded reason.
- Calling unrequested optional-method absence degraded.
- Selecting a route before role/risk/quality floors.
- Skipping tests, review, documentation, or audit to save tokens.
- Keeping an agent open without known compatible pending work.

Stop and return to the relevant PSOC or lifecycle gate when any red flag
appears.
