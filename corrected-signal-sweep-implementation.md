# Corrected Signal Sweep Implementation Report

Status: IN_PROGRESS

## PSOC

### Problem

The retained live Signal Sweep run proves that one Session plus three follow-ups
cost 5.90× the equal-quality four-Session Baseline. The batch-first correction
is implemented in routing policy but has not been measured with its intended
topology: one fresh Session receiving all compatible ready work as one bounded
batch. The current Benchmark and Dashboard also model four independent worker
lifecycles, so they cannot yet represent that corrected topology without
inventing follow-ups or hiding the batch boundary.

### Scenarios

1. Four serial fresh Baseline Sessions and one fresh bounded-batch Harness
   Session start from byte-identical trees, use the same model/profile, and pass
   the same complete quality gates; exact telemetry then supports a fair Token
   comparison.
2. The bounded batch is cheaper at equal quality. The Skill may describe the
   saving as evidence for this workload while keeping the scope and measurement
   method explicit.
3. The bounded batch is equal or more expensive. The result remains visible,
   the saving claim stays false, and routing guidance is revised from evidence
   rather than presentation goals.
4. A run lacks exact usage or fails a quality gate. Numeric saving remains
   unknown and the report identifies the missing evidence instead of coercing
   it to zero.
5. The Dashboard shows one Harness run, its batch and four ordered slices,
   routing reason, current/terminal work, exact Token result, and quality state;
   it never exposes Baseline comparison UI.

### Options

1. Reuse the historical one-plus-three run. This is cheap but tests the rejected
   policy and cannot answer the corrected-policy question.
2. Run the corrected topology manually and publish only prose. This obtains the
   number quickly but leaves the Benchmark and product state unable to reproduce
   or display the topology truthfully.
3. Run the corrected topology first from generated artifacts, then use its raw
   evidence to drive the smallest TDD extension to Benchmark, ledger, Skill,
   and Dashboard contracts. Avoid a new database entity unless existing task,
   package, and activity facts cannot express the result honestly.

### Chosen Plan

Choose option 3. Evidence comes before schema or UI work. Generate a fresh
isolated fixture, execute four serial fresh Baseline Sessions and one fresh
bounded-batch Session, normalize only provider-reported facts, and apply
identical quality gates. Then add the minimum durable topology and display
contract required by the observed run. Preserve all 20 Skill invariants,
Standalone operation, the Harness-only product boundary, and the Moonlight
Indigo bilingual Dashboard.

## Agent Budget

- Maximum open benchmark Sessions: `1`.
- Maximum total benchmark Sessions: `5`.
- Exception to the default total of four: a complete A/B needs four isolated
  Baseline slice samples plus one corrected bounded-batch sample. Removing any
  one changes the approved comparison topology.
- Follow-ups: `0`; every benchmark Session is fresh.
- Nested delegation: disabled.
- Fresh implementation agents: `0`; controller performs write work serially.
- Independent review: reuse the two existing read-only reviewers; do not spawn
  another reviewer while either is available.
- Stop conditions: stop a model turn that attempts nested delegation, writes
  outside its generated project, changes the approved interfaces, or cannot
  complete its assigned gate. Retain every started attempt in Token cost.

## Agent Ledger

| handle | role | task | status | report_path | spawned_at | waited | closed | write_scope | token_risk | session_budget | final_reason | next_action |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pending | worker | Baseline engine slice | planned | `/tmp/signal-sweep-corrected-ab-20260715/runtime/baseline/worker-01.final.md` | pending | no | no | generated engine scope | fresh bootstrap | one fresh turn | pending | execute serially |
| pending | worker | Baseline UI slice | planned | `/tmp/signal-sweep-corrected-ab-20260715/runtime/baseline/worker-02.final.md` | pending | no | no | generated UI scope | fresh bootstrap | one fresh turn | pending | after worker-01 |
| pending | worker | Baseline records slice | planned | `/tmp/signal-sweep-corrected-ab-20260715/runtime/baseline/worker-03.final.md` | pending | no | no | generated records scope | fresh bootstrap | one fresh turn | pending | after worker-02 |
| pending | worker | Baseline integration slice | planned | `/tmp/signal-sweep-corrected-ab-20260715/runtime/baseline/worker-04.final.md` | pending | no | no | generated integration scope | fresh bootstrap | one fresh turn | pending | after worker-03 |
| pending | worker | Corrected four-slice bounded batch | planned | `/tmp/signal-sweep-corrected-ab-20260715/runtime/harness/batch.final.md` | pending | no | no | union of four generated scopes | one complete batch | one fresh turn | pending | after Baseline |

## Write Scope

- Benchmark Sessions: their isolated generated project only, with exact scopes
  declared by the generated briefs; no repository writes.
- Controller: Benchmark generator/tests, runtime/store/status/Dashboard files,
  Skill/references, deterministic release checks, and relevant documentation.
- Reviewers: `none` (read-only).

## Decision Log

- 2026-07-15: Do not reuse any old Baseline or historical Harness telemetry.
- 2026-07-15: Use `gpt-5.6-sol` with medium reasoning for all five fresh turns.
- 2026-07-15: Treat a batch as one host Session and one complete usage sample;
  do not fabricate four worker Session lifecycles.
- 2026-07-15: Baseline comparison stays in release evidence only; the Dashboard
  continues to show Harness facts exclusively.
- 2026-07-15: Do not install the repository Skill. Push is explicitly approved
  after verification and review.

## Evidence

- Historical RED: `docs/benchmarks/2026-07-15-signal-sweep-real-ab.md`.
- Approved product design:
  `docs/specs/2026-07-15-results-dashboard-design.md`.
- Execution plan:
  `docs/plans/2026-07-15-results-dashboard-and-signal-sweep-plan.md`.
- Corrected raw artifacts: `/tmp/signal-sweep-corrected-ab-20260715/`
  (outside Git; pending generation).
- Root-cause inspection: `summarize_observations` constructs the same expected
  worker set for both modes and requires all four lifecycle sets and usage
  coverage. `render_cached_prompts` likewise emits four independent prompts.
  The current protocol therefore cannot encode a one-Session batch without
  fabricating worker lifecycles.

## Changed Files

Pending.

## Tests

- Pre-change Benchmark baseline:
  `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts/test_game_dev_ab_benchmark.py`
  — PASS, 15 tests.
- Every behavior change will record its RED and GREEN commands here.

## Review Findings

Pending independent review.

## Risks

- A single bounded batch may need more reasoning/output than four narrow turns;
  the experiment must measure this rather than assume a saving.
- Provider input includes large cached context. Both provider totals and
  non-overlapping normalized categories must remain visible.
- CLI behavior, authentication, or model availability can invalidate a live
  sample; started attempts remain part of cost and receive a terminal reason.
- The current preview binary may hold the build destination open. Stop it only
  when a rebuild is required, then restore the final preview at the same URL.

## Next Actions

Generate byte-identical starter repositories, checkpoint their tree hashes,
then execute the five fresh Sessions serially.

## External Agent Reconciliation

The two existing reviewers are known read-only review capacity. No unknown
external agent currently affects the benchmark budget or cleanup.

## Final Audit

Pending lifecycle audit, full verification, review, commit, push, and final
preview inspection.
