#!/usr/bin/env python3
"""Validate release metadata without relying on local runtime skills."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any


PLUGIN_NAME = "cached-subagent-harness"
SKILL_NAME = "cached-subagent-harness"
DESIGN_RELATIVE = "docs/specs/2026-07-10-agent-control-plane-design.md"
INVARIANT_HEADING = "## Non-negotiable Invariants"
SKILL_INVARIANT_END = "\n## Controller Loop"
DESIGN_INVARIANT_END = "\n### Existing-contract disposition map"
SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")
SKILL_NAME_RE = re.compile(r"^[a-z0-9-]+$")
REQUIRED_PLUGIN_FIELDS = {
    "name",
    "version",
    "description",
    "author",
    "skills",
    "interface",
}
REQUIRED_INTERFACE_FIELDS = {
    "displayName",
    "shortDescription",
    "longDescription",
    "developerName",
    "category",
    "capabilities",
    "defaultPrompt",
}
REQUIRED_METHOD_HEADINGS = [
    "## PSOC Loop",
    "## Work Packages and Compatible Batching",
    "## Quality-Constrained Routing",
    "## Test and Harness Gate",
    "## Independent Review",
    "## Optional Methodology Adapters",
    "## Quick Reference",
    "## Red Flags",
]
REQUIRED_METHOD_SEMANTICS = [
    "When the runtime cannot prove lease-aware follow-up, place compatible "
    "assignments in one bounded worker brief and report reuse as unsupported.",
    "Never emulate reuse with an unrestricted permanent role pool.",
    "Set role, risk, uncertainty, and quality floors before choosing a model "
    "or reasoning profile.",
    "Security-sensitive, destructive, and control-plane changes require deep.",
    "Strong tests and retry capacity do not lower that floor.",
    "Behavior changes are test-first.",
    "The controller waits, consumes the report, runs focused tests and the "
    "project harness, and records the commit checkpoint before acceptance or "
    "another writer assignment.",
    "Architecture boundaries, workflow or service contracts, shared data "
    "models, connectors or repositories, phase-end work, and whole-branch "
    "work require an independent reviewer.",
    "A writer or fixer cannot review its own work.",
    "Batch all Critical and Important findings into one fixer pass, then "
    "re-review.",
    "Standalone is complete without another methodology.",
    "Adapter absence when not requested is normal.",
    "An explicitly requested adapter failure is visible, but it does not make "
    "the standalone core degraded.",
]


def fail(message: str) -> None:
    raise SystemExit(message)


def load_json(path: Path) -> dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        fail(f"missing file: {path}")
    except json.JSONDecodeError as error:
        fail(f"invalid JSON in {path}: {error}")
    if not isinstance(payload, dict):
        fail(f"{path} must contain a JSON object")
    return payload


def reject_todos(value: Any, path: str) -> None:
    if isinstance(value, str):
        if "[TODO:" in value:
            fail(f"{path} contains a TODO placeholder")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            reject_todos(item, f"{path}[{index}]")
    elif isinstance(value, dict):
        for key, item in value.items():
            reject_todos(item, f"{path}.{key}")


def extract_section(text: str, start: str, end: str, source: str) -> str:
    start_index = text.find(start)
    if start_index < 0:
        fail(f"{source} missing section start: {start}")
    end_index = text.find(end, start_index)
    if end_index < 0:
        fail(f"{source} missing section end: {end.strip()}")
    return text[start_index:end_index]


def validate_plugin(repo: Path) -> None:
    manifest = load_json(repo / ".codex-plugin" / "plugin.json")
    reject_todos(manifest, "plugin")

    missing = sorted(REQUIRED_PLUGIN_FIELDS - set(manifest))
    if missing:
        fail(f"plugin missing required fields: {missing}")
    if manifest["name"] != PLUGIN_NAME:
        fail(f"plugin name must be {PLUGIN_NAME}")
    if not SEMVER_RE.fullmatch(manifest["version"]):
        fail("plugin version must be semver")
    if manifest["skills"] != "./skills/":
        fail("plugin skills path must be ./skills/")

    author = manifest.get("author")
    if not isinstance(author, dict) or not author.get("name"):
        fail("plugin author.name is required")

    interface = manifest.get("interface")
    if not isinstance(interface, dict):
        fail("plugin interface must be an object")
    missing_interface = sorted(REQUIRED_INTERFACE_FIELDS - set(interface))
    if missing_interface:
        fail(f"plugin interface missing fields: {missing_interface}")
    prompts = interface["defaultPrompt"]
    if not isinstance(prompts, list) or not prompts:
        fail("interface.defaultPrompt must be a non-empty list")
    if len(prompts) > 3:
        fail("interface.defaultPrompt must contain at most 3 entries")


def parse_frontmatter(text: str) -> dict[str, str]:
    if not text.startswith("---\n"):
        fail("SKILL.md missing YAML frontmatter")
    try:
        _, frontmatter, _ = text.split("---", 2)
    except ValueError:
        fail("SKILL.md has invalid YAML frontmatter")
    result: dict[str, str] = {}
    for raw_line in frontmatter.splitlines():
        line = raw_line.strip()
        if not line or ":" not in line:
            continue
        key, value = line.split(":", 1)
        result[key.strip()] = value.strip().strip('"')
    return result


def validate_skill(repo: Path) -> None:
    skill_root = repo / "skills" / SKILL_NAME
    skill_md = skill_root / "SKILL.md"
    if not skill_md.is_file():
        fail(f"missing {skill_md}")
    text = skill_md.read_text(encoding="utf-8")
    frontmatter = parse_frontmatter(text)
    if frontmatter.get("name") != SKILL_NAME:
        fail(f"skill name must be {SKILL_NAME}")
    if not SKILL_NAME_RE.fullmatch(frontmatter.get("name", "")):
        fail("skill name must be lowercase hyphen-case")
    description = frontmatter.get("description", "")
    if not description.startswith("Use when"):
        fail("skill description must start with 'Use when'")
    if len(description) > 1024:
        fail("skill description is too long")

    required_files = [
        "references/standalone-methodology.md",
        "references/gates.md",
        "references/prompt-layering.md",
        "references/report-contracts.md",
        "scripts/harnessctl/Cargo.toml",
        "scripts/harnessctl/src/main.rs",
    ]
    for relative in required_files:
        if not (skill_root / relative).is_file():
            fail(f"missing skill file: {relative}")

    design_path = repo / DESIGN_RELATIVE
    if not design_path.is_file():
        fail(f"missing canonical design file: {DESIGN_RELATIVE}")
    design = design_path.read_text(encoding="utf-8")
    actual_invariants = extract_section(
        text,
        INVARIANT_HEADING,
        SKILL_INVARIANT_END,
        "SKILL.md",
    )
    canonical_invariants = extract_section(
        design,
        INVARIANT_HEADING,
        DESIGN_INVARIANT_END,
        DESIGN_RELATIVE,
    )
    if actual_invariants != canonical_invariants:
        fail("SKILL.md invariant block must exactly match the approved design")
    if "Standalone is the normal operating mode" not in text:
        fail("SKILL.md must declare standalone normal mode")
    if "## Superpowers Relationship" in text:
        fail("SKILL.md must not make Superpowers a core relationship")

    method = (skill_root / "references/standalone-methodology.md").read_text(
        encoding="utf-8"
    )
    for heading in REQUIRED_METHOD_HEADINGS:
        if heading not in method:
            fail(f"standalone methodology missing heading: {heading}")
    normalized_method = " ".join(method.split())
    for required in REQUIRED_METHOD_SEMANTICS:
        if required not in normalized_method:
            fail(f"standalone methodology missing binding contract: {required}")


def main() -> None:
    repo = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    validate_plugin(repo)
    validate_skill(repo)
    print("release metadata validation passed")


if __name__ == "__main__":
    main()
