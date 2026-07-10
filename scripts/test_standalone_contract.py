#!/usr/bin/env python3
from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = REPO_ROOT / "scripts" / "validate-release.py"
SKILL_PATH = "skills/cached-subagent-harness/SKILL.md"
METHOD_PATH = (
    "skills/cached-subagent-harness/references/standalone-methodology.md"
)
DESIGN_PATH = "docs/specs/2026-07-10-agent-control-plane-design.md"
INVARIANT_HEADING = "## Non-negotiable Invariants"
SKILL_INVARIANT_END = "\n## Controller Loop"
DESIGN_INVARIANT_END = "\n### Existing-contract disposition map"

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


def extract_section(text: str, start: str, end: str) -> str:
    start_index = text.find(start)
    if start_index < 0:
        raise AssertionError(f"missing section start: {start}")
    end_index = text.find(end, start_index)
    if end_index < 0:
        raise AssertionError(f"missing section end: {end}")
    return text[start_index:end_index]


class StandaloneContractTests(unittest.TestCase):
    def read(self, relative: str) -> str:
        return (REPO_ROOT / relative).read_text(encoding="utf-8")

    def run_mutated_validation(
        self,
        relative: str,
        original: str,
        replacement: str,
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            shutil.copytree(
                REPO_ROOT / ".codex-plugin",
                repo / ".codex-plugin",
            )
            shutil.copytree(
                REPO_ROOT / "skills" / "cached-subagent-harness",
                repo / "skills" / "cached-subagent-harness",
            )
            design_destination = repo / DESIGN_PATH
            design_destination.parent.mkdir(parents=True)
            shutil.copy2(REPO_ROOT / DESIGN_PATH, design_destination)

            mutation_path = repo / relative
            text = mutation_path.read_text(encoding="utf-8")
            self.assertIn(original, text)
            mutation_path.write_text(
                text.replace(original, replacement, 1),
                encoding="utf-8",
            )
            return subprocess.run(
                [sys.executable, str(VALIDATOR), str(repo)],
                cwd=REPO_ROOT,
                text=True,
                capture_output=True,
                check=False,
            )

    def test_skill_keeps_canonical_invariant_block(self) -> None:
        skill = self.read(SKILL_PATH)
        design = self.read(DESIGN_PATH)
        actual = extract_section(
            skill,
            INVARIANT_HEADING,
            SKILL_INVARIANT_END,
        )
        canonical = extract_section(
            design,
            INVARIANT_HEADING,
            DESIGN_INVARIANT_END,
        )
        self.assertEqual(actual, canonical)

    def test_skill_declares_standalone_normal_and_optional_adapters(self) -> None:
        skill = self.read("skills/cached-subagent-harness/SKILL.md")
        self.assertIn("Standalone is the normal operating mode", skill)
        self.assertIn("references/standalone-methodology.md", skill)
        self.assertNotIn("## Superpowers Relationship", skill)

    def test_standalone_reference_contains_binding_method(self) -> None:
        method = self.read(METHOD_PATH)
        for heading in [
            "## PSOC Loop",
            "## Work Packages and Compatible Batching",
            "## Test and Harness Gate",
            "## Independent Review",
            "## Optional Methodology Adapters",
            "## Quick Reference",
            "## Red Flags",
        ]:
            self.assertIn(heading, method)
        normalized = " ".join(method.split())
        for required in REQUIRED_METHOD_SEMANTICS:
            self.assertIn(required, normalized)

    def test_release_validator_rejects_invariant_body_mutation(self) -> None:
        result = self.run_mutated_validation(
            SKILL_PATH,
            "Every long task has a brief, durable report, budget,",
            "Every long task may omit its durable report and budget,",
        )
        self.assertNotEqual(
            result.returncode,
            0,
            result.stdout + result.stderr,
        )

    def test_release_validator_rejects_method_semantic_mutation(self) -> None:
        result = self.run_mutated_validation(
            METHOD_PATH,
            "and report reuse as unsupported.",
            "and report reuse as supported.",
        )
        self.assertNotEqual(
            result.returncode,
            0,
            result.stdout + result.stderr,
        )

    def test_prompt_examples_are_not_superpowers_scoped(self) -> None:
        prompt = self.read(
            "skills/cached-subagent-harness/references/prompt-layering.md"
        )
        self.assertNotIn("/.superpowers/", prompt)
        self.assertIn("/.agent-harness/", prompt)

    def test_optional_method_absence_is_not_degraded(self) -> None:
        gates = self.read(
            "skills/cached-subagent-harness/references/gates.md"
        )
        reports = self.read(
            "skills/cached-subagent-harness/references/report-contracts.md"
        )
        self.assertIn("Optional methodology absence is not degraded", gates)
        self.assertIn("Optional methodology absence is not degraded", reports)

    def test_public_docs_present_superpowers_as_optional(self) -> None:
        readme = self.read("README.md")
        integration = self.read("docs/superpowers.md")
        self.assertIn("Standalone is the default", readme)
        self.assertIn("scripts/install.sh --with-superpowers", readme)
        self.assertNotIn(
            "installer detects Superpowers and installs its skills",
            readme,
        )
        self.assertIn("explicitly optional", integration)
        self.assertIn(
            "Optional methodology absence is not degraded",
            integration,
        )


if __name__ == "__main__":
    unittest.main()
