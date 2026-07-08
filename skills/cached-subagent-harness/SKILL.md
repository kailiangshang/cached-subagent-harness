---
name: cached-subagent-harness
description: Use when Codex coordinates subagents for long-running development work where token budget, stable prompts, repo-backed lifecycle tracking, complete-development gates, or resume-safe handoffs matter.
---

# Cached Subagent Harness

Use this skill before dispatching subagents for long-running development work. This skill is the controller protocol: it owns agent budget, repo-backed lifecycle ledger, prompt/cache discipline, role gates, and final audit.

It may use superpowers skills as phase references, but it must remain operational under the local minimum contract below when those skills are unavailable, too costly to load, or version-drifted.

## Core Rules

1. Harness first: every long task needs a brief, repo report, agent ledger, gates, and final audit.
2. PSOC first: Problem, Scenarios, Options, Chosen Plan before worker code.
3. Information density first: prefer structured facts, paths, status, evidence, and decisions over narrative.
4. Complete development: do not use "MVP" to skip required behavior, tests, error handling, integration, documentation, review, or verification.
5. Stable prompt prefixes: keep role prompts stable; put task-specific data only after the dynamic marker.
6. Subagents are token investments, not default acceleration. Use them only when they reduce controller context load, isolate read-heavy discovery, or provide independent review.
7. Do not add version suffixes to skill, agent, or role names. Use stable names such as `explorer`, `worker`, `reviewer`, and `fixer`.
8. Protect the control plane: skill files, harness files, and agent-management rules are read-only to subagents unless a worker brief explicitly grants those paths as its write scope.

## Superpowers Relationship

- Use superpowers skills as phase references, not startup bulk context.
- Load TDD, review, verification, planning, or worktree guidance only when entering that phase and the added context is worth the cost.
- Local minimum contract when degraded: problem first, read-heavy parallel/write-heavy serial, verify key behavior before edits, review gate, final verification, and ledger complete.
- Record degraded mode in the repo report when a referenced superpowers skill or harness tool is unavailable or intentionally skipped for token cost.

## Required Loop

Before any worker writes code, create or confirm a brief with:

```text
Problem:
Scenarios:
Options:
Chosen Plan:
```

Loop rule: if exploration, implementation, tests, or review invalidate an earlier section, return to the earliest invalid section, revise the brief or report, and re-enter the gate. Ask the user only when the loop exposes a product decision, plan contradiction, or scope change the controller cannot resolve.

See `references/gates.md` for the full gate flow.

## Agent Budget and Lifecycle

Before spawning, create or update the repo report with `PSOC`, `Agent Budget`, and `Agent Ledger`.

Default budget:

- max concurrent agents: `2`;
- max total agents for the task: `4`;
- exceed either limit only when the PSOC or budget explains why the token/lifecycle cost is justified;
- read-heavy roles may run in parallel; write-heavy roles run serially;
- do not recursively dispatch subagents unless the user explicitly requests it.

The repo report is the authoritative lifecycle source for agents created by this harness. For known harness-created handles, do not ask the user to paste `/agent` output. If the UI or user reports extra unknown agents and the platform lacks `list_agents`, ask for one `/agent` reconciliation only when it affects budget, cleanup, or correctness; record the result as `externally-unknown` rather than pretending the harness can close unknown handles.

Each spawned agent must move through the ledger: `planned`, `spawned` or `running`, `reported`, then `closed`; or `failed`, `abandoned`, or `externally-unknown` with a reason. Completed or closed agents may remain visible in the UI; treat that as platform state, not harness state.

See `references/report-contracts.md` for ledger fields and audit requirements.

## Prompt Discipline

Use the stable prefix shape from `references/prompt-layering.md`. Dynamic fields belong at the tail:

```text
--- DYNAMIC TASK CONTEXT ---
ROLE=worker
TASK_BRIEF_PATH=...
REPORT_PATH=...
AGENT_LEDGER_PATH=...
BASE_COMMIT=...
REVIEW_PACKAGE_PATH=...
ALLOWED_WRITE_PATHS=...
```

Do not paste full plans, diffs, long logs, or historical summaries into dispatch prompts. Pass file paths instead.

Use the bundled Rust harness binary when available:

```text
scripts/bin/harnessctl render-prompt
scripts/bin/harnessctl check-prompt
scripts/bin/harnessctl ledger-init
scripts/bin/harnessctl ledger-add
scripts/bin/harnessctl ledger-update
scripts/bin/harnessctl ledger-audit
```

`scripts/harnessctl/` contains the Rust source. `scripts/bin/harnessctl` is the local runtime binary and stores durable lifecycle state in a small SQLite database chosen by `--db`.

Existing Python scripts are legacy/development helpers, not the formal runtime path. If no binary tool is available, build it with Cargo or follow the manual templates in this skill and record degraded mode in the report.

## Role Gates

- `explorer`: read-only context gathering. Use for unknown code areas, dependency mapping, or risk discovery.
- `discussion`: read-only product, architecture, or skill discussion. It may ask questions and propose changes, but must not edit files, commit, or mutate skills unless the controller later promotes the work into an explicit worker brief.
- `worker`: the only normal write role. Use TDD, run focused tests, commit changes, and write a report. Every worker dispatch needs an explicit `ALLOWED_WRITE_PATHS` value.
- `reviewer`: read-only review against brief, report, and diff package. Do not ask it to rediscover the project.
- `fixer`: one batched fix pass for all Critical/Important findings from a review.

Avoid recursive subagents unless the user explicitly asks for nested delegation.

## Completion Gate

A task is not complete until:

- the report file records PSOC, agent budget, lifecycle ledger, status, files changed, commits, tests run, known risks, degraded mode notes, and final audit;
- every harness-created agent is `closed`, or is `failed`, `abandoned`, or `externally-unknown` with a reason;
- the relevant focused tests pass;
- the controller runs the project harness or documented equivalent;
- Critical/Important review findings are fixed or explicitly escalated;
- progress is recorded in the repo-backed durable ledger.

See `references/report-contracts.md` for report fields and status names.
