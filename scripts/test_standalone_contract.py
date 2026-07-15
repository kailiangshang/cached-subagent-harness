#!/usr/bin/env python3
from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
import re
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = REPO_ROOT / "scripts" / "validate-release.py"
SKILL_PATH = "skills/cached-subagent-harness/SKILL.md"
METHOD_PATH = (
    "skills/cached-subagent-harness/references/standalone-methodology.md"
)
DESIGN_PATH = "docs/specs/2026-07-14-lightweight-token-harness-design.md"
INVARIANT_HEADING = "## Non-negotiable Invariants"
SKILL_INVARIANT_END = "\n## Controller Loop"

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

    def copy_validation_fixture(self, directory: str) -> Path:
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
        return repo

    def run_mutated_validation(
        self,
        relative: str,
        original: str,
        replacement: str,
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.copy_validation_fixture(directory)

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

    def run_validation_without_skill_file(
        self,
        relative: str,
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.copy_validation_fixture(directory)

            (repo / "skills" / "cached-subagent-harness" / relative).unlink()
            return subprocess.run(
                [sys.executable, str(VALIDATOR), str(repo)],
                cwd=REPO_ROOT,
                text=True,
                capture_output=True,
                check=False,
            )

    def test_skill_keeps_all_twenty_invariants(self) -> None:
        skill = self.read(SKILL_PATH)
        invariants = extract_section(
            skill,
            INVARIANT_HEADING,
            SKILL_INVARIANT_END,
        )
        for number in range(1, 21):
            self.assertRegex(invariants, rf"(?m)^{number}\. \*\*")

    def test_skill_limits_batching_to_evidence_bounded_compatible_micro_batches(
        self,
    ) -> None:
        skill = self.read(SKILL_PATH)
        invariants = extract_section(
            skill,
            INVARIANT_HEADING,
            SKILL_INVARIANT_END,
        )
        normalized = " ".join(invariants.split())
        for required in [
            "strictly compatible micro-batches of at most two assignments by "
            "default",
            "Do not relax or normalize role, required profile, risk, write "
            "scope, base revision, dependency order, or review boundary to "
            "manufacture compatibility.",
            "Large batches and follow-ups require versioned durable evidence "
            "from equal-quality exact-usage comparisons.",
        ]:
            self.assertIn(required, normalized)

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

    def test_release_validator_requires_every_rust_module(self) -> None:
        modules = [
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
        ]
        for module in modules:
            with self.subTest(module=module):
                result = self.run_validation_without_skill_file(module)
                self.assertNotEqual(
                    result.returncode,
                    0,
                    result.stdout + result.stderr,
                )
                self.assertIn(
                    f"missing skill file: {module}",
                    result.stderr,
                )


if __name__ == "__main__":
    unittest.main()
