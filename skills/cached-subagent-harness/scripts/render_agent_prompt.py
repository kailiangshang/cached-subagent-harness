#!/usr/bin/env python3
"""Render a cache-aware dispatch prompt for a subagent role."""

from __future__ import annotations

import argparse
from pathlib import Path


ROLE_RULES = {
    "discussion": [
        "Read only. Do not edit files, commit, or mutate skills.",
        "Discuss product, architecture, or process questions and write conclusions to REPORT_PATH if requested.",
        "If an edit looks necessary, return the proposed worker brief instead of changing files.",
    ],
    "explorer": [
        "Read only. Do not edit files or commit.",
        "Investigate only the requested scope and write findings to REPORT_PATH.",
        "Return status plus the report path only.",
    ],
    "worker": [
        "You are the only writer for this gate.",
        "Use TDD for behavior changes, run focused tests, commit completed work.",
        "If PSOC becomes invalid, stop that path and report LOOP_REQUIRED.",
    ],
    "reviewer": [
        "Read only. Review the brief, report, and review package.",
        "Lead with findings ordered by severity.",
        "Do not run broad rediscovery unless a provided artifact is missing.",
    ],
    "fixer": [
        "Fix only the provided Critical/Important findings.",
        "Run covering tests, append results to the existing report, and commit.",
        "Do not broaden scope while fixing.",
    ],
}


STABLE_PREFIX = """Use the cached-subagent-harness skill for this dispatch.

Stable operating rules:
- Follow harness-first validation. Work is not complete without reported tests.
- Keep information dense. Read large artifacts from paths; do not ask for pasted diffs or logs.
- Preserve complete-development quality. Do not skip required behavior, tests, error handling, integration, or docs by calling the work an MVP.
- Maintain the PSOC loop: Problem, Scenarios, Options, Chosen Plan.
- If new evidence invalidates PSOC, return LOOP_REQUIRED with the earliest invalid section.
- Use stable role behavior. Do not spawn nested subagents unless explicitly instructed.
- Require ledger state. A planned ledger row, budget, and report path must exist before spawn.
- Keep lifecycle closed. After reporting, the controller must wait, consume the report, then close or mark a final exception with final_reason.
- Close superseded agents. Temporary replacement agents expire when the original agent is resumed or the task is cancelled.
- Follow the report contract. Reports must cover PSOC, files, tests, risks, degraded mode, and final audit evidence.
- Respect ALLOWED_WRITE_PATHS. Read-only roles must treat it as none; writing roles must stay inside it.
- Treat control-plane files and agent-management rules as read-only unless explicitly granted.
- Reconcile unknown UI agents through one /agent snapshot only when they affect budget, cleanup, or correctness.
- Write the full report to REPORT_PATH and return only status, commits, tests, risks, and report path.
"""


def existing_path(value: str | None) -> str | None:
    if not value:
        return None
    return str(Path(value).expanduser().resolve())


def build_prompt(args: argparse.Namespace) -> str:
    write_scope = ",".join(args.allowed_write_paths) if args.allowed_write_paths else "none"
    lines = [STABLE_PREFIX.rstrip(), "", f"Role: {args.role}"]
    lines.extend(f"- {rule}" for rule in ROLE_RULES[args.role])

    if args.context:
        lines.append("")
        lines.append("Stable context files to read if needed:")
        for context_path in args.context:
            lines.append(f"- {existing_path(context_path)}")

    lines.append("")
    lines.append("--- DYNAMIC TASK CONTEXT ---")
    fields = {
        "ROLE": args.role,
        "TASK_BRIEF_PATH": existing_path(args.brief),
        "REPORT_PATH": existing_path(args.report),
        "AGENT_LEDGER_PATH": args.ledger or existing_path(args.report),
        "BASE_COMMIT": args.base_commit,
        "REVIEW_PACKAGE_PATH": existing_path(args.review_package),
        "FINDINGS_PATH": existing_path(args.findings),
        "HARNESS_COMMAND": args.harness_command,
        "ALLOWED_WRITE_PATHS": write_scope,
    }
    for name, value in fields.items():
        if value:
            lines.append(f"{name}={value}")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("role", choices=sorted(ROLE_RULES))
    parser.add_argument("--brief", help="Task brief file path")
    parser.add_argument("--report", required=True, help="Report file path")
    parser.add_argument("--ledger", help="Agent ledger path or report anchor")
    parser.add_argument("--base-commit", help="Base commit for this task")
    parser.add_argument("--review-package", help="Review package path")
    parser.add_argument("--findings", help="Review findings file for fixer")
    parser.add_argument(
        "--context",
        action="append",
        default=[],
        help="Stable context file path. Repeat as needed.",
    )
    parser.add_argument(
        "--harness-command",
        default=".venv/bin/python scripts/feedback_agent_harness.py",
        help="Project harness command",
    )
    parser.add_argument(
        "--allowed-write-paths",
        action="append",
        default=[],
        help="Allowed write path or glob. Repeat as needed. Required for worker/fixer.",
    )
    args = parser.parse_args()
    if args.role in {"worker", "fixer"} and not args.allowed_write_paths:
        parser.error(f"{args.role} requires --allowed-write-paths")
    if args.role == "worker" and not args.brief:
        parser.error("worker requires --brief with PSOC/TASK_BRIEF_PATH")
    if args.role == "fixer" and not args.findings:
        parser.error("fixer requires --findings with FINDINGS_PATH")
    if args.role in {"discussion", "explorer", "reviewer"} and args.allowed_write_paths:
        parser.error(f"{args.role} is read-only and cannot accept --allowed-write-paths")
    print(build_prompt(args), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
