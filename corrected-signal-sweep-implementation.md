# Corrected Signal Sweep Implementation Report

Status: COMPLETE — implementation, full verification, two independent reviews,
visual checks, both lifecycle audits, commit, and remote delivery complete.

## PSOC

### Problem

The retained one-Session/three-follow-up run cost 5.90× its equal-quality
Baseline. The proposed correction—put all known ready work in one fresh
Session—had not been measured and the product could not display its routing
limits clearly. A truthful release needed a fresh A/B, a policy based on that
evidence, and a minimal Dashboard improvement without creating another control
plane.

### Scenarios

1. Four serial fresh Baseline Sessions and one fresh four-slice Session start
   from byte-identical trees, use the same model/profile, and pass the same
   quality gates; exact telemetry supports a clean comparison.
2. Failed host attempts remain operational cost even when they stop before
   product writes; the clean comparable sample stays separate.
3. The large batch is equal or more expensive. The release retains the result,
   makes no positive saving claim, and tightens routing from evidence.
4. Missing or partial usage remains unknown; no comparison coerces it to zero.
5. Normal runtime Tasks preserve their compatibility facts and are partitioned
   into small batches. A benchmark pressure topology cannot redefine product
   routing.
6. The Dashboard shows Harness results only, including enforced route/budget
   limits and factual route activity, without a Batch database entity or an
   observer LLM.

### Options

1. Keep the prior batch-first rule and publish the favorable offline prompt
   estimate. Rejected: dispatch bytes do not model growing inference context.
2. Replace the Session with another complex scheduler or permanent worker
   pool. Rejected: it expands control-plane cost without evidence.
3. Measure the four-slice pressure topology, split comparable and retry cost,
   then enforce strictly compatible micro-batches of at most two. Expose the
   enforced constants and route activity through the existing StatusView.

### Chosen Plan

Choose option 3. Preserve all 20 numbered Skill invariants. The runtime derives
the ready set from durable queued Tasks, never normalizes compatibility fields,
and partitions it into at most two Tasks per bundle by default. CLI overrides
may lower but not raise that limit. Larger batches or higher follow-up limits
require versioned equal-quality exact-usage evidence. The Dashboard remains a
read-only single-Run results view.

## Agent Budget

- Comparable benchmark topology: four successful fresh Baseline Sessions and
  one successful fresh Harness Session; zero follow-ups and no nested
  delegation.
- Recovery attempts: one pre-model authentication rejection, one missing
  task-local ledger failure, and one installed-Skill schema-drift failure.
- Controller ordering mistake: one planned Task failed before host spawn and
  had zero Token cost.
- Open implementation agents: none; controller writes are serial.
- Independent review: reuse the existing `focused_reviewer` and
  `final_reviewer`; do not create another reviewer.
- Budget exception: the A/B requires five successful isolated samples. Every
  extra started attempt remains visible with its terminal reason and cost.

## Agent Ledger

| handle | role | task | status | waited | closed | report/evidence | final reason / next action |
|---|---|---|---|---|---|---|---|
| `baseline-worker-01` | worker | initial engine plan | failed | yes | n/a | development ledger | controller pre-spawn ordering error; zero host cost; replaced |
| `baseline-session-01` | worker | engine auth attempt | failed | yes | n/a | development ledger | inherited invalid API key rejected before model execution; validated zero Tokens |
| `baseline-session-01-chatgpt` | worker | engine slice | accepted | yes | yes | `runtime/baseline/worker-01.final.md` | complete, 622,235 Tokens |
| `baseline-session-02` | worker | rendering slice | accepted | yes | yes | `runtime/baseline/worker-02.final.md` | complete, 1,021,680 Tokens |
| `baseline-session-03` | worker | records slice | accepted | yes | yes | `runtime/baseline/worker-03.final.md` | complete, 479,674 Tokens |
| `baseline-session-04` | worker | integration slice | accepted | yes | yes | `runtime/baseline/worker-04.final.md` | complete, 518,440 Tokens |
| `harness-batch-session` | worker | four-slice attempt | failed | yes | n/a | `runtime/harness/batch-ledger-failed.final.md` | missing batch-local ledger; no product writes; 258,054 Tokens |
| `harness-bounded-batch-session` | worker | four-slice attempt | failed | yes | n/a | `runtime/harness/batch-skill-version-failed.final.md` | installed Skill schema drift; no product writes; 401,769 Tokens |
| `harness-isolated-batch-session` | worker | isolated four-slice sample | accepted | yes | yes | `runtime/harness/batch.final.md` | complete, 5,053,165 Tokens |
| `focused_reviewer` | reviewer | focused policy/UI/code review | closed | yes | yes | collaboration review | Ready; two Important findings fixed, none remain |
| `final_reviewer` | reviewer | whole-diff final review | closed | yes | yes | collaboration review | Ready; no Critical or Important finding; two Minor findings fixed |

Benchmark report paths are under
`/tmp/signal-sweep-corrected-ab-20260715/`; the repository keeps only sanitized
aggregate evidence.

## Write Scope

- Benchmark Sessions: isolated generated project only, with scope fixed by the
  generated brief; no repository writes.
- Controller: Benchmark generator/tests, Rust runtime/status/Dashboard,
  Skill/references, release validation, canonical docs, and this report.
- Reviewers: read-only.

## Routing and Batch Policy

- Ready source: durable queued Task state.
- Compatibility: package, role, complexity, risk, uncertainty, write scope and
  hash, base revision, review boundary, and required profile must match.
- Release batch limit: `2` Tasks; `harnessctl bundle --max-tasks N` may lower
  only.
- Reuse limits: one accepted follow-up and 200,000 effective Tokens; lower only.
- Increase gate: versioned equal-quality exact-usage evidence.
- Accounting: comparable closed usage and retry-inclusive operational cost are
  both reported.

## Decision Log

- 2026-07-15: use `gpt-5.6-sol` with medium reasoning for all successful A/B
  samples.
- 2026-07-15: do not install or overwrite the user's active Skill; isolate the
  successful Harness sample in a temporary Codex home.
- 2026-07-15: treat the missing-ledger and installed-Skill failures as Token
  cost, but not as part of the equal-quality clean sample.
- 2026-07-15: the four-slice batch cost 1.91× Baseline before retries; abandon
  one-large-batch routing.
- 2026-07-15: default to two-Task strictly compatible micro-batches. Do not
  manufacture compatibility.
- 2026-07-15: keep Baseline comparisons out of product UI.
- 2026-07-15: add only a compact dispatch-policy strip and visible activity
  summaries; no Batch table, observer, service, scanner, or bridge.
- 2026-07-15: push `origin/main` only after verification, two reviews, and
  lifecycle audit. User approval for push is explicit.

## Evidence

- Starter commit: `13269310dce1160c2b95a3a8bafa0c58e8883e34`.
- Starter tree: `2c8858f5c867bf856dc33dd5bfdf9cb1cdaad31f`.
- Raw artifacts: `/tmp/signal-sweep-corrected-ab-20260715/`.
- Normalized report: `/tmp/signal-sweep-corrected-ab-20260715/real-report.json`.
- Sanitized report:
  `docs/benchmarks/2026-07-15-signal-sweep-corrected-ab.md`.
- Historical follow-up RED:
  `docs/benchmarks/2026-07-15-signal-sweep-real-ab.md`.
- Development ledger: `corrected-signal-sweep-implementation.db` (ignored,
  run complete and final audit passed).

### Exact result

| Scope | Baseline | Harness | Saving |
|---|---:|---:|---:|
| Comparable effective Tokens | 2,642,029 | 5,053,165 | -91.26% |
| Retry effective Tokens | 0 | 659,823 | n/a |
| Operational effective Tokens | 2,642,029 | 5,712,988 | -116.23% |
| Comparable provider input | 2,604,494 | 5,012,466 | -92.45% |
| Operational provider input | 2,604,494 | 5,662,675 | -117.42% |

## Changed Files

Final pre-commit diff: 29 files, grouped as:

- Benchmark generator and tests;
- bundle policy, CLI parsing, StatusView, Dashboard assets, and Rust tests;
- atomic release-binary replacement and its live-process regression test;
- Skill, standalone gates/contracts/methodology, and release validator;
- README, current-state, Benchmark/specification/pressure-test documents, and
  this report.

## Tests

Recorded RED→GREEN evidence:

- Benchmark topology RED: missing `build_cached_batch_brief` and
  `corrected_runtime_topology`; GREEN: 17/17 Benchmark tests.
- Runtime limit RED: missing `parse_bundle_limit`; GREEN: six compatible Tasks
  become three two-Task bundles and raising `--max-tasks` is rejected.
- Skill contract RED: standalone tests failed on missing at-most-two,
  strict-compatibility, and evidence-limit language; GREEN: 10/10 tests.
- Dashboard RED: StatusView lacked `dispatch_policy` and HTML lacked the policy
  surface; GREEN: focused status/dashboard tests pass.
- Accounting integrity RED: usage attached to a `running` event was counted a
  second time in operational cost; GREEN: only `closed` and `retry` may carry
  usage and all 18 Benchmark tests pass.
- Sequence integrity RED: compatible A1/A3 work backfilled across incompatible
  B2 and flattened as `[1, 3, 2]`; GREEN: only the final contiguous compatible
  bundle may grow, the A/B/A regression stays `[1, 2, 3]`, and all three bundle
  tests pass.
- Live-binary replacement RED: a running Dashboard made the release copy fail
  with `Text file busy`; GREEN: same-directory temporary copy plus atomic rename,
  covered by an integration regression.
- Visual check: fresh zh-CN/en-US 1280×800 and compact zh-CN screenshots from
  the release binary; no visible horizontal overflow. The page shows 5,053,165
  exact Tokens, the enforced 2/1/200,000 limits, the corrected reuse-eligibility
  label, and `deep -> SpawnSession` from persisted activity.

Fresh final `scripts/verify.sh` evidence after all focused/final-review fixes,
while both Dashboard preview processes remained live:

- release metadata and standalone Skill validation passed;
- Rust 52/52 and Python 38/38 passed;
- `cargo fmt --check` and Clippy with `-D warnings` passed;
- the optimized release binary built successfully;
- Token-effectiveness and game A/B offline regression gates passed;
- prompt validation and the lifecycle smoke final audit passed.

The stricter accounting code also regenerated the real report from retained
observations and reproduced `-91.26%`, `-116.23%`, and 5,712,988 operational
effective Tokens exactly.

Dashboard checks used the successful real-run Ledger at
`http://127.0.0.1:7347`; `/health` returned `{"status":"ok"}` and `/api/status`
returned exact 5,053,165 effective Tokens plus policy values 2/1/200,000.

## Review Findings

Focused review found two Important issues and no Critical issue:

1. Non-settlement events could duplicate exact usage in operational totals.
   Fixed fail-closed with a focused RED→GREEN regression and protocol update.
2. Bundle backfilling could reorder declared Task sequence across an
   incompatible boundary. Fixed with contiguous-only append and an A/B/A
   RED→GREEN regression.

Focused verdict: Ready, with no remaining Critical or Important finding. Its two
release-record Minor notes were resolved by updating this report and separating
the corrected increment's verification from the prior Dashboard delivery in
`docs/current-state.md`.

Final whole-diff verdict: Ready, with no Critical or Important finding. Its two
Minor findings were resolved: the release binary is now replaced atomically
while a Dashboard is running, and the evidence document no longer contains
trailing whitespace. Fresh full verification passed after both fixes.

## Risks

- The two-Task default is conservative evidence containment, not proof of
  positive saving. Future measurements may lower it to one or justify another
  versioned policy.
- The four-slice stress ledger contains one aggregate Task because it predates
  the micro-batch correction. Normal runtime stores individual queued Tasks, so
  the Dashboard shows their rows and Session chain without inferring hidden
  slices.
- Requested/actual model fields remain separate; the successful fixture has no
  observed actual-model update, so the Dashboard correctly shows unknown.
- Raw authentication and Session artifacts stay outside Git.

## Next Actions

None. Future policy changes require new equal-quality exact-usage evidence.

## External Agent Reconciliation

The two supplied reviewers are the only planned review capacity. No new
reviewer or nested delegation is authorized.

## Final Audit

- Lifecycle Audit: successful Benchmark run and development run both passed
  `harnessctl audit`; both Runs are `complete`.
- Harness Commands: fresh `scripts/verify.sh` passed with live Dashboard
  processes; both `/health` endpoints remained healthy after atomic replacement.
- Focused Tests: accounting event boundary, A/B/A sequence, lower-only limit,
  Dashboard projection, and live-binary replacement regressions passed.
- Project Harness: Rust 52/52; Python 38/38; formatting, Clippy, release build,
  release validation, offline Benchmarks, and lifecycle smoke passed.
- Review Status: focused and final reviewers both Ready; zero open Critical or
  Important finding; all reported Minor findings fixed.
- Open Risks: no positive live Token-saving claim; the two-Task default remains
  conservative until equal-quality exact-usage evidence justifies a change.
- External Agent Reconciliation: only the two supplied reviewers were used;
  both completed; no nested or unknown agent remains.
- Degraded Mode: none. Standalone operation is intentional, not degraded.
- Git Delivery: delivery commit
  `70765a1771206523265c3ed0e6f44ccd5f2117fa` was pushed to `origin/main`; local
  `HEAD` and `git ls-remote origin refs/heads/main` matched exactly. This
  documentation-only closure records that verified transaction.
