# Signal Sweep Corrected A/B Evidence

Date: 2026-07-15
Status: equal quality, exact Codex CLI telemetry, four-slice large batch rejected

## Decision

One fresh Session containing all four ready slices did not reduce total Token
use. The comparable Harness sample consumed `5,053,165` effective Tokens versus
Baseline's `2,642,029`: `1.91×` the cost, or `-91.26%` saving. Two control-plane
retries add `659,823` Tokens, producing a retry-inclusive total of `5,712,988`
and `-116.23%` saving.

This rejects the assumption that all compatible ready work should be placed in
one long turn. The release now partitions strictly compatible ready Tasks into
micro-batches of at most two assignments by default. It does not claim positive
live Token savings for batching or Session reuse.

## Fairness Boundary

| Fact | Baseline | Four-slice Harness sample |
|---|---|---|
| Starter commit | `13269310dce1160c2b95a3a8bafa0c58e8883e34` | same |
| Starter tree | `2c8858f5c867bf856dc33dd5bfdf9cb1cdaad31f` | same |
| Model | `gpt-5.6-sol` | `gpt-5.6-sol` |
| Reasoning | medium | medium |
| Assignments | 4 ordered slices | same 4 slices |
| Successful topology | 4 serial fresh Sessions | 1 fresh Session, 0 follow-ups |
| Nested delegation | disabled | disabled |
| Project tests | complete Node suite passed | complete Node suite passed |
| HTTP gate | 6 required assets returned 200 | same |
| Visual gate | 1280×800 and 390×844 | same |
| Interaction gate | start, move, pause/resume, export, game-over, restart | same |
| Browser defects | no console errors, overlap, or horizontal overflow | same |

The four-slice topology was a deliberate pressure test. The slices have
different natural scopes and review concerns, so the current strict
compatibility rules would not manufacture one package by normalizing those
facts.

## Exact Comparable Sample

| Metric | Baseline | Harness | Harness / Baseline |
|---|---:|---:|---:|
| Noncached input | 272,590 | 148,722 | 0.55× |
| Cached input | 2,331,904 | 4,863,744 | 2.09× |
| Provider input | 2,604,494 | 5,012,466 | 1.92× |
| Visible output | 29,451 | 29,014 | 0.99× |
| Reasoning output | 8,084 | 11,685 | 1.45× |
| Provider output | 37,535 | 40,699 | 1.08× |
| Total effective | 2,642,029 | 5,053,165 | 1.91× |

Provider-input saving is `-92.45%`; total-effective saving is `-91.26%`.

The Baseline's four successful Session totals were:

| Execution unit | Effective Tokens |
|---|---:|
| `worker-01` | 622,235 |
| `worker-02` | 1,021,680 |
| `worker-03` | 479,674 |
| `worker-04` | 518,440 |
| **Total** | **2,642,029** |

## Retry-inclusive Operational Cost

Two Harness attempts stopped before product writes but still consumed model
Tokens:

| Attempt | Reason | Effective Tokens |
|---|---|---:|
| 1 | referenced batch-local ledger did not exist | 258,054 |
| 2 | host auto-loaded an older installed Skill with an incompatible ledger schema | 401,769 |
| **Retry total** | | **659,823** |

| Operational metric | Baseline | Harness |
|---|---:|---:|
| Provider input | 2,604,494 | 5,662,675 |
| Provider output | 37,535 | 50,313 |
| Effective Tokens | 2,642,029 | 5,712,988 |

Retry-inclusive provider-input saving is `-117.42%`; retry-inclusive
total-effective saving is `-116.23%`. A Baseline launch rejected an inherited
invalid API key before model execution; the host result validated zero Token
use for that attempt.

## Why One Session Cost More

The Harness saved `123,868` noncached input Tokens in the comparable sample,
but cached input increased by `2,531,840`. Within one long agent turn, every
tool call and reasoning step processes an expanding history containing earlier
code, command output, and decisions. Prompt caching lowers the price or latency
of repeated prefixes on some providers, but cached input is still processed
context and remains part of this Benchmark's effective-Token measure.

The offline prompt estimate saw only dispatch text and reported `83.90%`
cache-adjusted savings. It could not model the growing inference context inside
the Session. Bootstrap count alone is therefore not a safe routing objective;
complete observed usage is the gate.

## Corrections Driven by This RED Case

1. `harnessctl bundle` partitions a compatible ready set into at most two Tasks
   per bundle by default.
2. `harnessctl bundle --max-tasks N` may lower the release default but rejects
   increases. A future increase needs versioned durable equal-quality
   exact-usage evidence.
3. Role, required profile, risk, scope, base revision, dependency order, and
   review boundary cannot be relaxed or normalized to manufacture a batch.
4. The Benchmark reports closed comparable-sample usage separately from retry
   cost and retry-inclusive operational totals.
5. The Dashboard shows the enforced batch, follow-up, and reuse-eligibility
   Token limits, plus the latest factual route summary, without adding a
   separate Batch table or observer.
6. The 20 numbered Skill invariants remain intact; invariant 15 now contains
   the evidence-bounded micro-batch contract.

## Claim Boundary

The evidence supports these claims:

- repeated follow-ups and one four-slice long turn were both more expensive
  than the equal-quality fresh-Session Baselines on this workload;
- the release prevents those exact unlimited policies with conservative,
  lower-only limits;
- stable prompt structure, truthful accounting, routing floors, lifecycle
  audit, and visualization remain useful control-plane capabilities.

It does not support a positive end-to-end Token-saving percentage for the
current two-Task micro-batch policy. That claim requires a new equal-quality
exact-usage comparison of the released policy.

Raw prompts, JSONL streams, generated source, authentication state, and Session
logs remain outside the repository.
