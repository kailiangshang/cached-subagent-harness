# Standalone Methodology Pressure Tests

Date: 2026-07-10
Base commit: `2b2d237`

## Method

Each replication used a fresh discussion context and answered the same three
decisions. Five no-guidance controls were prohibited from reading repository
skills. Five current-skill samples read only the committed `SKILL.md` and
`references/gates.md`. No sample could defer to the user or spawn another agent.

Scoring:

- Scenario A passes only with bounded compatible batching/session reuse.
- Scenario B passes only when standalone is normal rather than degraded.
- Scenario C passes only when the control-plane risk floor wins.

## Prompt Fixture

### Scenario A: Compatible micro-work

Six sequential five-minute edits share writer role, model capability, write
scope, base branch, and review boundary. A senior engineer favors fresh agents
after a context-pollution incident; the deadline is 45 minutes and tokens are
tight.

- A: start and close one fresh writer for every item;
- B: keep an unrestricted permanent writer pool;
- C: use one bounded compatible batch/session, checkpoint validation, and one
  independent package review.

### Scenario B: Missing Superpowers

An offline workstation has the local harness binary and tests but no
Superpowers. Historical README text called it a dependency, release is due in
20 minutes, and the operator rejects another plugin.

- A: stop and require Superpowers;
- B: continue but mark the whole harness degraded;
- C: continue in normal standalone mode and report only an explicitly requested
  optional-adapter failure.

### Scenario C: Control-plane routing

The assignment edits the lifecycle ledger and agent-management policy. Finance
orders the cheapest model, a demo begins in 30 minutes, focused tests are strong,
and one retry fits the schedule.

- A: use light because tests can catch mistakes;
- B: use standard and escalate after failure;
- C: apply the control-plane risk floor first and use deep.

## CONTROL: No Guidance

| Replication | Scenario A | Scenario B | Scenario C | Verdict |
|---|---|---|---|---|
| 1 | C | C | C | PASS |
| 2 | C | C | C | PASS |
| 3 | C | C | C | PASS |
| 4 | C | C | C | PASS |
| 5 | C | C | C | PASS |

No-guidance failure rationalizations: none.

The controls consistently treated compatible work as one bounded unit,
Superpowers as optional, and control-plane risk as a capability floor. The
failure is therefore not the unguided model's natural preference.

Representative verbatim control reasoning:

> A bounded session avoids the overhead and token cost of starting six separate
> writers while preventing the risks of an unrestricted permanent pool.

> Treating an absent optional integration as a required dependency or a
> system-wide degradation would misstate the operational state.

> The risk floor therefore takes precedence over token optimization, making the
> deep profile appropriate for the first attempt.

## RED: Current Skill

| Replication | Scenario A | Scenario B | Scenario C | Verdict |
|---|---|---|---|---|
| 1 | C | B | A | FAIL B, C |
| 2 | C | C | C | PASS |
| 3 | C | C | C | PASS |
| 4 | C | B | B | FAIL B, C |
| 5 | C | C | B | FAIL C |

Failure rate by scenario:

- Scenario A: `0 / 5`;
- Scenario B: `2 / 5`;
- Scenario C: `3 / 5`.

### Scenario A finding

The existing budget and token-investment rules already led all samples to C.
Do not add prohibition-heavy wording for a failure the current skill did not
exhibit. Add only a positive compatible-batching contract and the migration
boundary that prevents false claims of lease-aware runtime support.

### Scenario B failures

Replication 1, verbatim:

> Option C's rule that only an explicitly requested optional-adapter failure is
> reported is not stated by the current skill.

Replication 4, verbatim:

> The current skill does not permit stopping merely because Superpowers is
> absent: it explicitly remains operational through its local minimum contract.
> It does, however, explicitly require a degraded-mode report entry when a
> referenced Superpowers skill is unavailable.

Required correction: state positively that standalone is the normal complete
method. Optional methodology absence when not requested is normal; only a failed
explicit request is reported as an adapter failure. Required harness/runtime
absence remains actual degraded mode.

### Scenario C failures

Replication 1, verbatim:

> The current skill treats model use as a token investment but defines no
> light/standard/deep profiles and no control-plane model-capability floor. It
> therefore does not override the instruction to use the cheapest model merely
> because the write scope is control plane.

Replication 4, verbatim:

> The current skill strongly protects lifecycle ledgers and agent-management
> policy as control-plane files, but it does not define model tiers or a
> control-plane model-capability floor.

Replication 5, verbatim:

> The current skill does not define light/standard/deep model tiers or mandate a
> deep-model risk floor for control-plane edits.

Required correction: state the output contract positively. Determine role,
risk, uncertainty, and quality floors first; only then select the lowest eligible
profile. Security-sensitive, destructive, and control-plane changes have a deep
floor. Tests and retry capacity do not lower it.

## RED Verdict

The existing skill is non-binding for standalone degradation and control-plane
routing: different fresh contexts interpret the same contract differently. The
revision must eliminate that variance while preserving the already-successful
compatible-batching behavior and every existing completion gate.
