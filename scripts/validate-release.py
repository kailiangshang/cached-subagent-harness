#!/usr/bin/env python3
"""Validate release metadata without relying on local runtime skills."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


PLUGIN_NAME = "cached-subagent-harness"
SKILL_NAME = "cached-subagent-harness"
DESIGN_RELATIVE = "docs/specs/2026-07-14-lightweight-token-harness-design.md"
INVARIANT_HEADING = "## Non-negotiable Invariants"
SKILL_INVARIANT_END = "\n## Run, Task, Subagent, and Session"
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
    "Batch known compatible ready assignments before attempting follow-up "
    "reuse.",
    "Partition the ready set into strictly compatible micro-batches of at "
    "most two assignments by default.",
    "Do not relax or normalize role, required capability, risk, write scope, "
    "base revision, dependency order, or review boundary to manufacture a "
    "batch.",
    "A larger batch or a higher follow-up limit requires versioned durable "
    "evidence from equal-quality exact-usage comparisons.",
    "Derive the compatible ready set from durable queued state rather than a "
    "caller-supplied count.",
    "Reuse only after an exact signature match and an atomic `idle` to `busy` "
    "claim; increment reuse only after the host accepts the follow-up.",
    "Every reusable session has an accepted-follow-up cap and a total "
    "effective token budget; unknown usage, either exhausted budget, or a "
    "changed compatibility signature closes the reuse path.",
    "Only complete exact usage linked to the current assignment can release a "
    "session for reuse.",
    "Release also requires durable follow-up acceptance and exact usage "
    "strictly after its transactional causal boundary.",
    "Usage run, task, and session ownership must agree.",
    "The runtime CLI can lower reuse limits but rejects increases until a "
    "versioned durable policy authorizes them.",
    "Refresh a queued task's base revision only through a compare-and-swap "
    "update while the task is unassigned; otherwise replan or register it "
    "when ready.",
    "A busy session has one current task; an idle or terminal session has none.",
    "When a host cannot follow up, use evidence-bounded micro-batches or new "
    "Sessions and report reuse as unsupported.",
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
REQUIRED_INVARIANT_SEMANTICS = [
    "Every long task has a brief, durable report, budget,",
    "Define Problem, Scenarios, Options, and Chosen Plan before worker code.",
    "Do not use `MVP` or token pressure to skip required",
    "Every writer has bounded allowed paths.",
    "A writer cannot approve its own high-risk work.",
    "Resume and compaction recover from the repository-backed report",
    "Only one assignment may actively write to overlapping scope at a time.",
    "Nested delegation remains disabled unless the user explicitly authorizes it",
    "Stable role policy precedes the dynamic marker",
    "Spawn only for real parallelism, context isolation, capability separation, or independent judgment.",
    "Select the lowest model and reasoning profile that satisfies role, risk, uncertainty, and quality floors.",
    "Unsupported or unavailable telemetry remains `unknown`",
    "Known compatible ready work is batched before follow-up reuse.",
    "strictly compatible micro-batches of at most two assignments by default",
    "Do not relax or normalize role, required profile, risk, write scope, base revision, dependency order, or review boundary to manufacture compatibility.",
    "Large batches and follow-ups require versioned durable evidence from equal-quality exact-usage comparisons.",
    "Every reusable session has both an accepted-follow-up cap and a total effective token budget.",
    "Terminal sessions never retain a current assignment.",
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
        "scripts/harnessctl/src/domain.rs",
        "scripts/harnessctl/src/store.rs",
        "scripts/harnessctl/src/bundle.rs",
        "scripts/harnessctl/src/routing.rs",
        "scripts/harnessctl/src/sessions.rs",
        "scripts/harnessctl/src/hosts.rs",
        "scripts/harnessctl/src/accounting.rs",
        "scripts/harnessctl/src/status.rs",
        "scripts/harnessctl/src/dashboard.rs",
        "scripts/harnessctl/assets/index.html",
        "scripts/harnessctl/assets/styles.css",
        "scripts/harnessctl/assets/app.js",
        "references/host-templates.json",
    ]
    for relative in required_files:
        if not (skill_root / relative).is_file():
            fail(f"missing skill file: {relative}")
    for removed in [
        "scripts/harnessctl/src/event_store.rs",
        "scripts/harnessctl/src/schema.rs",
        "scripts/harnessctl/src/ledger.rs",
    ]:
        if (skill_root / removed).exists():
            fail(f"obsolete runtime file must remain deleted: {removed}")
    runtime_text = "\n".join(
        path.read_text(encoding="utf-8")
        for path in (skill_root / "scripts/harnessctl/src").glob("*.rs")
    )
    for forbidden in [
        "control_plane_events",
        "replay_run_into_empty",
        "projection_field_sources",
        "EventInput",
    ]:
        if forbidden in runtime_text:
            fail(f"obsolete runtime reference remains: {forbidden}")

    design_path = repo / DESIGN_RELATIVE
    if not design_path.is_file():
        fail(f"missing canonical design file: {DESIGN_RELATIVE}")
    design = design_path.read_text(encoding="utf-8")
    invariants = extract_section(
        text, INVARIANT_HEADING, SKILL_INVARIANT_END, "SKILL.md"
    )
    for number in range(1, 21):
        if not re.search(rf"(?m)^{number}\. \*\*", invariants):
            fail(f"SKILL.md missing preserved invariant {number}")
    normalized_invariants = " ".join(invariants.split())
    for required in REQUIRED_INVARIANT_SEMANTICS:
        if required not in normalized_invariants:
            fail(f"SKILL.md missing preserved invariant semantic: {required}")
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


def validate_versions(repo: Path, expected_tag: str | None) -> None:
    manifest = load_json(repo / ".codex-plugin" / "plugin.json")
    plugin_version = manifest.get("version")
    cargo_path = (
        repo
        / "skills"
        / SKILL_NAME
        / "scripts"
        / "harnessctl"
        / "Cargo.toml"
    )
    try:
        cargo_text = cargo_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        fail(f"missing file: {cargo_path}")
    in_package = False
    cargo_version: str | None = None
    for raw_line in cargo_text.splitlines():
        line = raw_line.strip()
        if line.startswith("[") and line.endswith("]"):
            in_package = line == "[package]"
            continue
        if in_package:
            match = re.fullmatch(r'version\s*=\s*"([^"]+)"', line)
            if match:
                cargo_version = match.group(1)
                break
    if cargo_version is None or not SEMVER_RE.fullmatch(cargo_version):
        fail(f"missing or invalid package.version in {cargo_path}")
    if plugin_version != cargo_version:
        fail(
            "release version mismatch: "
            f"plugin={plugin_version}, cargo={cargo_version}"
        )
    if expected_tag is not None:
        if not expected_tag.startswith("v"):
            fail("release tag must start with v")
        if expected_tag[1:] != plugin_version:
            fail(
                "release tag version mismatch: "
                f"tag={expected_tag}, package={plugin_version}"
            )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("repo", nargs="?", default=".")
    parser.add_argument("--tag")
    args = parser.parse_args()
    repo = Path(args.repo).resolve()
    validate_plugin(repo)
    validate_skill(repo)
    validate_versions(repo, args.tag)
    print("release metadata validation passed")


if __name__ == "__main__":
    main()
