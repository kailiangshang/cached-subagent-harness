# Standalone Methodology

## PSOC Loop

Record Problem, Scenarios, Options, and Chosen Plan before writer code. Return
to the earliest invalid section when evidence changes it. Continue autonomously
for engineering corrections; ask the user only for product behavior, scope, or
approved-plan contradictions the controller cannot resolve.

## Work Packages and Compatible Batching

Group related assignments only when role, required capability, risk, write
scope, base revision, dependency order, and independence boundary are
compatible. Execute trivial work on main. Batch known compatible ready
assignments before attempting follow-up reuse. Derive the compatible ready set
from durable queued state rather than a caller-supplied count. Reuse only after
an exact signature match and an atomic `idle` to `busy` claim; increment reuse
only after the host accepts the follow-up. Every reusable session has an
accepted-follow-up cap and a total effective token budget; unknown usage, either
exhausted budget, or a changed compatibility signature closes the reuse path.
Only complete exact usage linked to the current assignment can release a
session for reuse. Release also requires durable follow-up acceptance and exact
usage strictly after its transactional causal boundary. Usage run, task, and
session ownership must agree. The
runtime CLI can lower reuse limits but rejects increases until a versioned
durable policy authorizes them. Refresh a queued task's base revision only
through a compare-and-swap update while the task is unassigned; otherwise
replan or register it when ready. A busy session has one current task; an idle
or terminal session has none. When a host cannot follow up, use one bounded
worker brief and report reuse as unsupported. Never emulate reuse with an
unrestricted permanent role pool.

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
| Known compatible ready work | Derive the durable queued set and batch it before follow-up reuse. |
| One later compatible task, exact assignment usage and both budgets remain | Atomically claim the idle session. |
| Usage unknown or either reuse budget exhausted | Close the reuse path; use main, batch, or spawn. |
| Requested limit exceeds a release default | Reject it until a versioned durable policy carries the evidence. |
| Queued task still valid after a verified commit | Compare-and-swap its base revision before routing. |
| Incompatible role, model, risk, scope, base, or review boundary | Use an isolated execution path. |
| Mandatory review trigger | Create an independent reviewer assignment. |
| Missing optional methodology | Continue standalone without degraded mode. |

## Rationalization Check

| Rationalization | Contract |
|---|---|
| One plan item needs one fresh agent | Assignment boundaries are not session boundaries; batch compatible work. |
| A cache hit makes unlimited follow-ups cheap | Cached input still grows; both follow-up count and effective tokens are capped. |
| Superpowers is missing, so quality is degraded | The standalone kernel owns the complete gates. |
| The cheapest model always saves tokens | Count retries, escalation, review, and fixer work. |
| Token pressure justifies skipping a gate | Complete development is a quality floor. |
| An idle agent might be useful later | Keep it only for known compatible near-term work. |

## Red Flags

- Fresh agents for compatible micro-assignments without a recorded reason.
- Repeated follow-ups when the work was ready for one batch.
- Reuse based on a caller-supplied ready count, stale/non-exact usage, or an
  exhausted session budget.
- A busy session without one current task, or an idle/terminal session that
  still names one.
- Calling unrequested optional-method absence degraded.
- Selecting a route before role/risk/quality floors.
- Skipping tests, review, documentation, or audit to save tokens.
- Keeping an agent open without known compatible pending work.

Stop and return to the relevant PSOC or lifecycle gate when any red flag
appears.
