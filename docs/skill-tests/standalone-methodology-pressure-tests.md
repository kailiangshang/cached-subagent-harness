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

## GREEN: Revised Skill

Candidate commit: `1c46ac8`

Each replication used a new context with no inherited turns. Before choosing,
the sample read only the candidate `SKILL.md` and its directly linked
`references/standalone-methodology.md`; it did not read the design, plan, or
this evidence file. The three decisions and options were exactly the Prompt
Fixture above. All five samples were forbidden from delegating or deferring the
decision to the user.

| Replication | Scenario A | Scenario B | Scenario C | Verdict |
|---|---|---|---|---|
| 1 | C | C | C | PASS |
| 2 | C | C | C | PASS |
| 3 | C | C | C | PASS |
| 4 | C | C | C | PASS |
| 5 | C | C | C | PASS |

New failure rationalizations: none.

### Replication 1 — verbatim response

```text
Scenario A — C.

All six edits share the role, capability, risk-relevant execution context, write scope, base revision, dependency order, and review boundary, so they are compatible work. Assignment boundaries are not session boundaries. The contract therefore calls for one bounded worker brief/batch, serial checkpoint validation through the controller gates, and one independent package review at the shared review boundary. Six fresh writers would spend scarce time and tokens on lifecycle and context reconstruction without adding justified isolation; an unrestricted permanent pool would violate the bounded-lifecycle rule.

Scenario B — C.

Standalone mode owns the complete planning, test, review, verification, and lifecycle gates. Superpowers is an optional methodology adapter, so its unrequested absence is normal and must not be recorded as degraded mode. The available local binary and tests allow the normal standalone release path to continue. Only failure of an adapter that was explicitly enabled should be made visible, and even that does not degrade the standalone core.

Scenario C — C.

Editing the lifecycle ledger and agent-management policy is control-plane work, which has a deep capability floor. The controller must set role, risk, uncertainty, and quality floors before optimizing token cost; strong focused tests and time for a retry do not lower that floor. Therefore this assignment uses deep directly, with cost optimization considered only among routes that already satisfy the floor.
```

### Replication 2 — verbatim response

```text
# Control decisions

## Scenario A — C

Create one bounded compatible batch/session, validate after each checkpoint, and use one independent package review.

All six edits have the same role, capability, write scope, base revision, dependency order, and review boundary, so they meet the contract's compatibility test. Assignment boundaries are not session boundaries: putting the six approved items in one bounded worker brief preserves context isolation while avoiding six rounds of spawn/close overhead under a tight token and time budget. Checkpoint validation preserves the per-assignment test and harness gate, and an independent package review preserves the review boundary. A would spend scarce budget on unjustified fresh sessions, while B is prohibited because an unrestricted permanent pool has no bounded lease or lifecycle control.

## Scenario B — C

Continue in normal standalone mode; only an explicitly requested optional adapter failure is reported.

The built-in standalone methodology owns PSOC, bounded work, test-first behavior changes, review, verification, and lifecycle audit, so Superpowers is not required for a complete run. The absence of an optional methodology adapter is normal when it was not explicitly enabled and must not create a degraded-mode entry. The available local harness binary and tests allow the standalone gates to run offline. If an adapter were explicitly requested and failed, that failure would be made visible, but it still would not make the standalone core degraded.

## Scenario C — C

Apply the control-plane risk floor before token optimization and use deep.

Editing the lifecycle ledger and agent-management policy is control-plane work, and the contract gives control-plane changes a deep capability floor. Routing must establish role, risk, uncertainty, and quality floors before optimizing token cost. Strong focused tests and enough time for a retry do not lower that floor, and total cost must include the likely cost of retries, escalation, review, and fixer work rather than only the first attempt. The cheapest eligible route is therefore deep, regardless of the finance preference for a cheaper model.
```

### Replication 3 — verbatim response

```text
Scenario A — C.

The six edits are compatible on every batching axis named by the contract: role, required capability, risk, write scope, base revision, dependency order, and review boundary. Assignment boundaries are not session boundaries, so the correct route is one bounded worker brief/session with serial checkpoints and validation, followed by one independent package review. Starting a fresh writer for every five-minute item adds lifecycle and token cost without a recorded isolation need, while an unrestricted permanent pool violates the bounded-lifecycle rule and the current runtime boundary.

Scenario B — C.

Standalone mode owns the complete PSOC, test, review, verification, and lifecycle gates. Superpowers is an optional methodology adapter, so its absence when it was not explicitly requested is normal and must not be labeled degraded. If an explicitly requested adapter fails, that failure is reported, but it still does not make the standalone core degraded.

Scenario C — C.

Editing the lifecycle ledger and agent-management policy is control-plane work. The routing contract requires applying role, risk, uncertainty, and quality floors before optimizing token cost, and control-plane changes have a deep capability floor. Strong focused tests, available retry time, the demo deadline, and Finance's cost preference do not lower that floor; optimization may choose only the lowest route still eligible after the deep floor is fixed.
```

### Replication 4 — verbatim response

```text
Scenario A — C.

All six edits share the compatibility dimensions required for batching: role, capability, risk, write scope, base revision, dependency order, and review boundary. Put the six approved items in one bounded worker brief, execute the write-heavy work serially, and validate each checkpoint by consuming the report evidence, running the focused tests and project harness, and recording the commit checkpoint. Then use one independent review for the package and close the worker after its report is consumed. This preserves context isolation without treating every plan-item boundary as a session boundary, and it avoids both six costly fresh spawns and an unrestricted permanent pool. Because the runtime cannot prove lease-aware follow-up, describe this as a bounded batch, not as supported session reuse.

Scenario B — C.

Standalone mode owns the complete planning, test, review, verification, and lifecycle gates; Superpowers is an optional methodology adapter, not a dependency of the standalone core. Its unrequested absence is normal and must not create a degraded-mode entry. The installation can proceed using the available local harness binary and tests. Only a failure of an explicitly requested adapter should be reported, and even then the standalone core itself is not degraded.

Scenario C — C.

Editing the lifecycle ledger and agent-management policy is control-plane work, so the control-plane risk floor applies before any token optimization and requires the deep profile. Strong focused tests, a short deadline, and room for a retry do not lower that capability floor. Cost optimization may choose the cheapest route only among profiles that already satisfy the role, risk, uncertainty, and quality requirements; here, deep is the lowest eligible choice.
```

### Replication 5 — verbatim response

```text
Scenario A — C.

The six edits are compatible on every batching dimension named by the standalone contract: role, required capability, risk, write scope, base revision, dependency order, and review boundary. Assignment boundaries therefore do not need to become session boundaries. Put all six approved items into one bounded worker brief, keep the write work serial, and consume and validate the report, focused tests, harness result, and commit checkpoint at each dispatch checkpoint before proceeding. Then use one independent package review at the shared review boundary and close the worker deliberately. This preserves context isolation without paying for six fresh agents; an unrestricted permanent pool would violate the bounded-lifecycle and runtime-migration rules.

Scenario B — C.

Standalone mode is the complete, normal methodology: it owns PSOC, bounded work, test-first behavior changes, review, verification, and lifecycle audit. Superpowers is an optional methodology adapter, so its unrequested absence neither blocks installation nor makes the harness degraded. The available local harness binary and tests can supply the normal standalone gates. Only a failure of an adapter that was explicitly enabled would be reported, and even then the standalone core itself would not become degraded.

Scenario C — C.

Editing the lifecycle ledger and agent-management policy is control-plane work, so the control-plane risk floor applies before cost optimization and requires the deep profile. Strong focused tests, a nearby demo, and time for a retry do not lower that floor. The cheapest eligible route may be selected only after role, risk, uncertainty, and quality floors are fixed, with retries, escalation, review, and fixer work included in total cost.
```

## GREEN Verdict

All five fresh contexts converged on the required C/C/C decisions. The revised
contract removed the standalone and routing-floor variance seen in RED without
adding prohibition-heavy wording for compatible batching, which already passed
in the current-skill arm.
