# Signal Sweep Real A/B Evidence

Date: 2026-07-15
Status: comparable quality, exact Codex CLI telemetry, original reuse strategy rejected

## Decision

The live run disproved the original assumption that a compatible Session should
be resumed for every later assignment. Stable prompt prefixes were small and
Codex cache hit rates were high, but the resumed Session's cumulative context
grew much faster than the saved bootstrap cost. The Harness arm used 5.90× the
Baseline's total effective Token count after including one rejected retry.

This result is intentionally retained. The runtime and Skill now batch all
known compatible ready work before considering a follow-up, and a later
follow-up is eligible only while both an accepted-follow-up cap and an observed
effective-token budget remain. No positive live saving claim is made for the
corrected policy until it receives a separate real run.

## Fairness Boundary

| Fact | Baseline | Harness arm |
|---|---|---|
| Starting tree | `2c8858f5c867bf856dc33dd5bfdf9cb1cdaad31f` | same |
| Model | `gpt-5.6-sol` | `gpt-5.6-sol` |
| Reasoning profile | medium | medium |
| Assignment topology | 4 serial fresh Sessions | 1 Session, 3 accepted follow-ups |
| Extra failed attempts | none | 1 read-only resume retry, retained in cost |
| Nested delegation | disabled | disabled |
| Automated tests | 21 passed | 30 passed |
| Syntax and HTTP assets | passed | passed |
| Desktop / compact screenshots | inspected at 1280×800 and 390×844 | same |
| Interaction coverage | start, move, pause, game-over, restart/export | same |

The difference in test count reflects independently produced implementations,
not a relaxed Harness gate. Both arms passed their complete project suites,
served every required module over HTTP with status 200, rendered a playable
first screen without overlap at both widths, and covered the same required
workflow.

## Token Semantics

Codex `--json` emits `turn.completed.usage` with `input_tokens`,
`cached_input_tokens`, `output_tokens`, and `reasoning_output_tokens`. The
Benchmark stores non-overlapping Harness categories:

```text
input       = input_tokens - cached_input_tokens
cache_read  = cached_input_tokens
output      = output_tokens - reasoning_output_tokens
reasoning   = reasoning_output_tokens
cache_write = 0  # Codex exposes no additional cache-write category here
```

The last line prevents inventing or double-counting a separate write quantity;
it does not claim that the provider performed no internal cache operation.
Missing fields remain unknown. The CLI event shape is documented in the Codex
Manual's non-interactive JSONL section; the arithmetic above is the Benchmark's
explicit normalization contract.

## Exact Observed Result

| Metric | Baseline | Harness arm | Harness / Baseline |
|---|---:|---:|---:|
| Provider input | 2,931,163 | 17,391,760 | 5.93× |
| Noncached input | 195,291 | 729,232 | 3.73× |
| Cached input | 2,735,872 | 16,662,528 | 6.09× |
| Provider output | 42,901 | 160,118 | 3.73× |
| Visible output | 32,768 | 110,217 | 3.36× |
| Reasoning output | 10,133 | 49,901 | 4.92× |
| Total effective | 2,974,064 | 17,551,878 | 5.90× |

Observed provider-input saving was `-493.34%`; total-effective saving was
`-490.16%`. Negative saving means the Harness arm consumed more. The rejected
read-only retry accounted for 999,618 effective Tokens and remains part of the
primary total. Even without that retry, the four successful resumed turns
would still total 16,552,260 effective Tokens, or 5.57× Baseline.

## Why Offline Prompt Estimates Were Wrong

The generated prompt artifacts looked favorable:

- raw prompt-size estimate: 39.81% smaller;
- cache-adjusted prompt estimate: 76.84% smaller;
- stable-prefix ratio: 82.04%.

Those estimates measured dispatch text, not the complete model context
processed throughout long agent turns. In the live resumed Session, provider
input grew from 730,930 on the first assignment to 2,943,749, 5,241,494, and
7,490,196 on later successful assignments. A high cached-input percentage did
not prevent cumulative processed context from dominating total Token use.

## Corrections Driven by This RED Case

1. `BatchThenSpawn` now wins before idle-session reuse whenever the durable
   queued ledger contains multiple compatible assignments; a caller-supplied
   ready count is not authoritative.
2. `harnessctl decide` defaults each reusable Session to one accepted follow-up
   and 200,000 effective Tokens. Runtime flags may lower but cannot raise these
   defaults; unknown usage or either exhausted limit makes reuse ineligible.
3. Complete exact normalized usage linked to the current assignment must be
   recorded before a Session can be reclaimed, and run/task/session ownership
   must agree. Missing or non-exact telemetry is not treated as zero.
4. A queued task's base revision can be refreshed only with a compare-and-swap
   update while it remains unassigned.
5. Terminal Sessions clear `current_task_id`, and final audit rejects legacy
   terminal rows that retain an assignment.
6. The Benchmark counts cached, reasoning, and retry Token categories and
   computes observed savings only after every expected worker completes all
   lifecycle events, provider totals match normalized splits, and both arms
   explicitly pass every named quality gate.

## Interpretation Limit

This experiment is evidence against unlimited Session continuation, not
evidence against stable prompts, safe model routing, or bounded batching. The
correct release claim is narrower: the known 5.90× regression is now prevented
by routing and budget gates. A future live run may measure the corrected
single-batch path, but until then the product must display the exact Harness
total and must not label the corrected policy as proven Token saving.

Raw prompts, JSONL streams, generated source, authentication artifacts, and
Session logs remain outside the repository.
