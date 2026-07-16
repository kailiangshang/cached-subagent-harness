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
APP_PATH = (
    "skills/cached-subagent-harness/scripts/harnessctl/assets/app.js"
)
STYLES_PATH = (
    "skills/cached-subagent-harness/scripts/harnessctl/assets/styles.css"
)
METHOD_PATH = (
    "skills/cached-subagent-harness/references/standalone-methodology.md"
)
DESIGN_PATH = "docs/specs/2026-07-14-lightweight-token-harness-design.md"
INVARIANT_HEADING = "## Non-negotiable Invariants"
SKILL_INVARIANT_END = "\n## Run, Task, Subagent, and Session"

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

    def test_skill_invariant_section_excludes_execution_terminology(
        self,
    ) -> None:
        skill = self.read(SKILL_PATH)
        invariants = extract_section(
            skill,
            INVARIANT_HEADING,
            SKILL_INVARIANT_END,
        )
        self.assertNotIn("## Run, Task, Subagent, and Session", invariants)
        self.assertNotIn(
            "Subagent is the delegated logical executor or role",
            invariants,
        )

    def test_release_validator_uses_the_execution_terminology_boundary(
        self,
    ) -> None:
        result = self.run_mutated_validation(
            SKILL_PATH,
            "\n## Run, Task, Subagent, and Session",
            "\n## Execution Terminology",
        )
        self.assertNotEqual(
            result.returncode,
            0,
            result.stdout + result.stderr,
        )

    def test_skill_gates_delegation_value_before_batching_and_routing(
        self,
    ) -> None:
        skill = self.read(SKILL_PATH)
        decision_order = extract_section(
            skill,
            "Apply Token decisions in this order:",
            "\nUse `references/standalone-methodology.md`",
        )
        delegation_gate = (
            "Execute on main when delegation value does not exceed complete "
            "cost."
        )
        self.assertIn(delegation_gate, " ".join(decision_order.split()))
        self.assertLess(
            decision_order.index("delegation value"),
            decision_order.index("Derive known compatible queued Tasks"),
        )

    def test_skill_explains_subagent_task_and_session_boundaries(self) -> None:
        skill = " ".join(self.read(SKILL_PATH).split())
        for required in [
            "Subagent is the delegated logical executor or role",
            "Session is the concrete host CLI/model context",
            "A new delegated Session normally creates a new Subagent instance",
            "Session is not an account login",
        ]:
            self.assertIn(required, skill)

    def test_public_docs_explain_execution_model_and_token_flow(self) -> None:
        for relative in ["README.md", "docs/current-state.md"]:
            with self.subTest(relative=relative):
                text = self.read(relative)
                normalized = " ".join(text.split())
                for term in ["Run", "Task", "Subagent", "Session"]:
                    self.assertIn(term, text)
                self.assertIn("Session is not an account login", normalized)
                self.assertIn("```mermaid", text)
                self.assertIn("Count complete effective Tokens", text)

        dashboard_design = self.read(
            "docs/specs/2026-07-15-results-dashboard-design.md"
        )
        self.assertIn("Subagent sessions", dashboard_design)
        self.assertIn("static release policy", dashboard_design)

    def test_public_docs_define_the_v020_binary_release_contract(self) -> None:
        readme = self.read("README.md")
        install = extract_section(readme, "## Install", "\n## Host Support")
        normalized = " ".join(readme.split())
        for required in [
            "long-running",
            "Token-aware control plane",
            "does not claim positive end-to-end Token savings",
            "prevents known high-cost Session regressions",
        ]:
            self.assertIn(required, normalized)
        for required in [
            "Prebuilt binary (recommended)",
            "scripts/install.sh",
            "scripts/install.ps1",
            "SHA256SUMS",
            "--binary-source",
            "auto",
            "download",
            "build",
            "none",
            "exact checked-out version",
            "unsigned",
            "Source build fallback",
        ]:
            self.assertIn(required, install)
        self.assertLess(
            install.index("Prebuilt binary (recommended)"),
            install.index("Source build fallback"),
        )
        for target in [
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
        ]:
            self.assertIn(target, install)
        for boundary in [
            "Ubuntu 24.04 / glibc 2.39",
            "macOS 15",
            "windows-latest",
            "Older operating-system releases are not certified",
        ]:
            self.assertIn(boundary, install)

        notes_path = REPO_ROOT / "docs" / "releases" / "0.2.0.md"
        self.assertTrue(notes_path.is_file(), "missing v0.2.0 release notes")
        notes = notes_path.read_text(encoding="utf-8")
        for asset in [
            "harnessctl-v0.2.0-x86_64-unknown-linux-gnu.tar.gz",
            "harnessctl-v0.2.0-aarch64-unknown-linux-gnu.tar.gz",
            "harnessctl-v0.2.0-x86_64-apple-darwin.tar.gz",
            "harnessctl-v0.2.0-aarch64-apple-darwin.tar.gz",
            "harnessctl-v0.2.0-x86_64-pc-windows-msvc.zip",
            "SHA256SUMS",
            "auto",
            "download",
            "build",
            "none",
            "unsigned",
            "5,053,165",
            "2,642,029",
        ]:
            self.assertIn(asset, notes)
        self.assertIn("Ubuntu 24.04 / glibc 2.39", notes)
        self.assertIn("Older operating-system releases are not certified", notes)

        current_state = self.read("docs/current-state.md")
        for link in [
            "specs/2026-07-16-binary-release-design.md",
            "plans/2026-07-16-binary-release-plan.md",
            "../binary-release-implementation.md",
            "releases/0.2.0.md",
        ]:
            self.assertIn(link, current_state)

        for relative in [
            "docs/specs/2026-07-16-binary-release-design.md",
            "docs/plans/2026-07-16-binary-release-plan.md",
            "binary-release-implementation.md",
            ".github/workflows/release.yml",
        ]:
            text = self.read(relative)
            self.assertNotIn("immutable GitHub Release", text)
            self.assertNotIn("immutable tag", text)
        self.assertIn(
            "repository settings, not this workflow",
            " ".join(notes.split()),
        )

    def test_current_state_attributes_this_increment_to_its_own_report(
        self,
    ) -> None:
        current_state = self.read("docs/current-state.md")
        self.assertIn(
            "subagent-session-token-strategy-implementation.md",
            current_state,
        )
        self.assertNotIn(
            "for this increment's full verification, final review, and "
            "lifecycle audit",
            " ".join(current_state.split()),
        )

    def test_dashboard_strategy_returns_nonvaluable_delegation_to_main(
        self,
    ) -> None:
        app = self.read(APP_PATH)
        for required in [
            "简单任务或委派净收益不为正 → 主线程；仅在收益为正时继续",
            "Simple Task or no net delegation value → main; continue only "
            "when valuable",
            "仅在委派门槛通过后：可证明且预算充足则续接 1 次；无可复用会话才新建",
            "Only after delegation passes: reuse once with exact proof and "
            "budget; spawn only when no eligible Session",
        ]:
            self.assertIn(required, app)

    def test_dashboard_locale_keys_match(self) -> None:
        app = self.read(APP_PATH)
        zh_start = app.index('    "zh-CN": {')
        en_start = app.index('    "en-US": {')
        copy_end = app.index("\n    }\n  };", en_start)
        key_pattern = re.compile(r"(?m)^      ([A-Za-z][A-Za-z0-9_]*):")
        zh_keys = set(key_pattern.findall(app[zh_start:en_start]))
        en_keys = set(key_pattern.findall(app[en_start:copy_end]))
        self.assertTrue(zh_keys)
        self.assertSetEqual(zh_keys, en_keys)

    def test_dashboard_explanatory_copy_uses_body_text_size(self) -> None:
        styles = self.read(STYLES_PATH)
        for selector in [
            ".strategy-heading > p:last-child",
            ".strategy-step small",
            ".session-definition",
        ]:
            with self.subTest(selector=selector):
                match = re.search(
                    rf"{re.escape(selector)}\s*\{{(?P<body>.*?)\n\}}",
                    styles,
                    re.DOTALL,
                )
                self.assertIsNotNone(match, f"missing rule: {selector}")
                self.assertIn("font-size: 12px;", match.group("body"))

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

    def test_release_validator_rejects_noncanonical_semver(self) -> None:
        result = self.run_mutated_validation(
            ".codex-plugin/plugin.json",
            '"version": "0.2.0"',
            '"version": "01.2.3"',
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("plugin version must be semver", result.stderr)

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
