# Prompt Layering

Use this reference when writing or checking a subagent dispatch prompt.

## Stable Prefix

Keep the prompt prefix stable across dispatches for the same role. The prefix should contain only information that rarely changes:

- role contract;
- harness discipline;
- built-in standalone controller contract;
- file handoff discipline;
- PSOC loop requirement;
- lifecycle ledger requirement;
- report contract;
- write-scope boundary;
- allowed and forbidden behavior.

Do not place dates, branch names, commit hashes, task titles, test logs, diffs, or ad hoc status summaries in the stable prefix.

Do not rewrite a role prefix for a specific task. If the same role needs a different instruction, first decide whether it is truly stable role policy or task-specific dynamic context.

## Dynamic Tail

Put all task-specific fields after this exact marker:

```text
--- DYNAMIC TASK CONTEXT ---
```

Use one assignment per line:

```text
ROLE=worker
TASK_BRIEF_PATH=/abs/path/to/task-brief.md
REPORT_PATH=/abs/path/to/task-report.md
AGENT_LEDGER_PATH=/abs/path/to/task-report.md#agent-ledger
BASE_COMMIT=abcdef1
REVIEW_PACKAGE_PATH=/abs/path/to/review.diff
ALLOWED_WRITE_PATHS=issue_feedback_agent/services/*.py,issue_feedback_agent/tests/**
```

Omit fields that do not apply. Do not invent new names when an existing field fits.

Use `ALLOWED_WRITE_PATHS=none` for `discussion`, `explorer`, and `reviewer`. Use explicit path globs for `worker` and `fixer`; do not rely on prose write permissions.

## Dense Handoffs

Dispatch prompts should route work, not carry bulk data.

Prefer:

```text
TASK_BRIEF_PATH=/repo/.agent-harness/task-11-brief.md
```

Avoid:

```text
Here is the full task brief:
...
```

Never paste:

- full `git diff`;
- full test logs;
- full implementation plan;
- accumulated task history;
- large source files.

If a subagent needs large content, give it the path and exact read target.

## Cache Checks

Before dispatch, check:

- Dynamic fields appear only after the marker.
- The prompt has no `diff --git` blocks.
- The prompt has no long test logs.
- The prompt has no repeated historical summaries.
- The prompt names files instead of embedding their content.
- The task report path and ledger path are present when the role can create, update, or depend on lifecycle state.
- `ROLE` is present and is one of `discussion`, `explorer`, `worker`, `reviewer`, or `fixer`.
- `worker` prompts include `TASK_BRIEF_PATH`.
- `fixer` prompts include `FINDINGS_PATH`.
- The role's write scope is present. Read-only roles use `ALLOWED_WRITE_PATHS=none`; writing roles use explicit paths.

Use the bundled Rust harness binary when available:

```text
scripts/bin/harnessctl render-prompt --role worker --brief <path> --report <path> --ledger <db> --allowed-write-paths <path>
scripts/bin/harnessctl check-prompt --file <prompt-file>
```

If only legacy Python helpers exist, use them as development aids or follow the checklist manually and record degraded mode in the task report.
