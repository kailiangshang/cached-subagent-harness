#!/usr/bin/env python3
"""Check a subagent dispatch prompt for cache-hostile patterns."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


DYNAMIC_MARKER = "--- DYNAMIC TASK CONTEXT ---"
DYNAMIC_FIELD_RE = re.compile(
    r"^(ROLE|TASK_BRIEF_PATH|REPORT_PATH|AGENT_LEDGER_PATH|BASE_COMMIT|"
    r"REVIEW_PACKAGE_PATH|FINDINGS_PATH|HARNESS_COMMAND|ALLOWED_WRITE_PATHS)="
)
CACHE_HOSTILE_PATTERNS = {
    "embedded git diff": re.compile(r"^diff --git ", re.MULTILINE),
    "embedded diff hunk": re.compile(r"^@@ .+ @@", re.MULTILINE),
    "pytest session log": re.compile(r"=+ test session starts =+"),
    "long traceback": re.compile(r"Traceback \(most recent call last\):"),
}
READ_ONLY_ROLES = {"discussion", "explorer", "reviewer"}
WRITE_ROLES = {"worker", "fixer"}


def dynamic_fields(lines: list[str], marker_index: int) -> dict[str, str]:
    fields: dict[str, str] = {}
    for line in lines[marker_index + 1 :]:
        if "=" not in line:
            continue
        name, value = line.split("=", 1)
        if DYNAMIC_FIELD_RE.match(f"{name}="):
            fields[name] = value.strip()
    return fields


def check_prompt(text: str, max_lines: int) -> list[str]:
    errors: list[str] = []
    lines = text.splitlines()

    if DYNAMIC_MARKER not in text:
        errors.append(f"missing dynamic marker: {DYNAMIC_MARKER}")
        marker_index = len(lines)
    else:
        marker_index = lines.index(DYNAMIC_MARKER)
    fields = dynamic_fields(lines, marker_index)

    for idx, line in enumerate(lines[:marker_index], start=1):
        if DYNAMIC_FIELD_RE.match(line):
            errors.append(f"dynamic field before marker at line {idx}: {line}")

    role = fields.get("ROLE")
    write_scope = fields.get("ALLOWED_WRITE_PATHS")
    if role in WRITE_ROLES and (not write_scope or write_scope == "none"):
        errors.append(f"{role} prompt must include explicit ALLOWED_WRITE_PATHS")
    if role in READ_ONLY_ROLES and write_scope != "none":
        errors.append(f"{role} prompt must use ALLOWED_WRITE_PATHS=none")
    if role and "REPORT_PATH" not in fields:
        errors.append("missing REPORT_PATH dynamic field")
    if role and "AGENT_LEDGER_PATH" not in fields:
        errors.append("missing AGENT_LEDGER_PATH dynamic field")

    for label, pattern in CACHE_HOSTILE_PATTERNS.items():
        if pattern.search(text):
            errors.append(f"cache-hostile content found: {label}")

    if len(lines) > max_lines:
        errors.append(f"prompt has {len(lines)} lines, above limit {max_lines}")

    fenced_blocks = text.count("```")
    if fenced_blocks > 2:
        errors.append("prompt contains multiple fenced blocks; pass bulky content by path")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("prompt_file", help="Prompt file to check")
    parser.add_argument("--max-lines", type=int, default=120)
    args = parser.parse_args()

    text = Path(args.prompt_file).read_text(encoding="utf-8")
    errors = check_prompt(text, args.max_lines)
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print("OK: dispatch prompt is cache-friendly")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
