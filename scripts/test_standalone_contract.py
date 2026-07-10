#!/usr/bin/env python3
from __future__ import annotations

import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]

EXPECTED_INVARIANTS = [
    "Harness first",
    "PSOC first",
    "Complete development",
    "Explicit write scope",
    "Protect the control plane",
    "Independent gates",
    "Evidence before completion",
    "Durable state is authoritative",
    "Read-heavy parallel, write-heavy serial",
    "Close deliberately",
    "No uncontrolled fan-out",
    "Budget every session",
    "Information density first",
    "Stable prompt prefixes",
    "Subagents are investments",
    "Quality-constrained optimization",
    "Requested is not actual",
    "Unknown is honest",
    "Facts do not depend on an LLM",
    "Stable names, no version suffixes",
]


class StandaloneContractTests(unittest.TestCase):
    def read(self, relative: str) -> str:
        return (REPO_ROOT / relative).read_text(encoding="utf-8")

    def test_skill_keeps_all_numbered_invariants_in_order(self) -> None:
        skill = self.read("skills/cached-subagent-harness/SKILL.md")
        matches = re.findall(
            r"(?m)^(\d+)\. \*\*(.+?)\.\*\*", skill
        )
        self.assertEqual(
            [int(number) for number, _ in matches],
            list(range(1, 21)),
        )
        self.assertEqual([name for _, name in matches], EXPECTED_INVARIANTS)

    def test_skill_declares_standalone_normal_and_optional_adapters(self) -> None:
        skill = self.read("skills/cached-subagent-harness/SKILL.md")
        self.assertIn("Standalone is the normal operating mode", skill)
        self.assertIn("references/standalone-methodology.md", skill)
        self.assertNotIn("## Superpowers Relationship", skill)

    def test_standalone_reference_contains_complete_method(self) -> None:
        method = self.read(
            "skills/cached-subagent-harness/references/"
            "standalone-methodology.md"
        )
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
