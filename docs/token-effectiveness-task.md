# Token Effectiveness Task

This repository includes an offline task test for prompt size and cache shape.
It is designed to answer one question:

Can repeated subagent dispatches move reusable instructions into a stable prefix
while keeping task-specific context in a small dynamic tail?

The task does not call any model API. It uses a deterministic `bytes/4` token
proxy so CI can run without credentials, network calls, or provider-specific
billing behavior.

## Representative Task

The fixture simulates a four-worker refactor dispatch for a feedback-agent /
inspection-platform project. The embedded brief includes:

- problem, scenarios, options, and chosen plan;
- read-only `src/**` constraints;
- future workflow, memory, report, database, forum, and code-suggestion needs;
- required evidence paths instead of pasted source files or logs.

The baseline prompt repeats the full brief in every worker dispatch. The harness
prompt renders the stable control contract once per role and pushes task-specific
values behind:

```text
--- DYNAMIC TASK CONTEXT ---
ROLE=worker
TASK_BRIEF_PATH=...
REPORT_PATH=...
AGENT_LEDGER_PATH=...
ALLOWED_WRITE_PATHS=...
```

## Run

```bash
scripts/build-harnessctl.sh
python3 scripts/token_effectiveness_task.py --format markdown
```

The full release verification also runs it:

```bash
scripts/verify.sh
```

## Current Comparison

Latest local run with 4 worker dispatches:

| Metric | Baseline embedded handoff | Cached harness handoff |
|---|---:|---:|
| Prompt count | 4 | 4 |
| Estimated tokens total | 1784 | 2164 |
| Average tokens per prompt | 446.0 | 541.0 |
| Cache-adjusted estimated tokens | n/a | 856 |
| Stable prefix tokens counted once | n/a | 436 |
| Dynamic tail tokens total | n/a | 420 |
| Repeated cacheable tokens | n/a | 1308 |
| Stable prefix ratio | n/a | 80.59% |

Raw estimated savings: `-21.3%`

Cache-adjusted estimated savings: `52.02%`

## Interpretation

Raw prompt size is allowed to increase because the stable prefix now carries
more safety rules: lifecycle closure, prompt gates, write scopes, final audit,
control-plane safety, and superseded-agent cleanup.

The gate that matters is cache-adjusted behavior:

- stable prefix ratio must stay high;
- dynamic tail must stay small;
- cache-adjusted estimated savings must remain above the configured threshold;
- generated prompts must still pass `harnessctl check-prompt`.

This makes the test useful as a regression guard: if future changes paste bulky
task context into prompts, remove the dynamic marker, weaken ROLE validation, or
grow the dynamic tail too far, CI can fail before the skill is published.
