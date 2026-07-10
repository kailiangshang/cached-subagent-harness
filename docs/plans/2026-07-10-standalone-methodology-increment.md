# Standalone Methodology Increment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use cached-subagent-harness to
> execute this plan with durable PSOC, explicit write scope, TDD, package review,
> and final lifecycle audit. This plan deliberately does not require
> Superpowers. Reuse one compatible writer session across implementation
> assignments; use fresh contexts only where the skill pressure-test protocol
> requires independent samples.

**Goal:** Make cached-subagent-harness fully operational as a standalone skill,
preserve its approved invariant constitution, and make Superpowers installation
strictly opt-in.

**Architecture:** The installer first installs the local skill/runtime and never
touches Superpowers on its default path. The skill body carries the numbered
constitution and routes detailed standalone mechanics to one focused reference.
Deterministic Python and shell tests enforce packaging behavior; pressure
scenarios verify that agents follow the new discipline rather than merely
finding the right phrases.

**Tech Stack:** Bash installer, Python 3 unittest integration tests, Markdown
skill/reference files, existing Rust harnessctl, existing scripts/verify.sh.

## Global Constraints

- Implement only delivery increment 1 from
  docs/specs/2026-07-10-agent-control-plane-design.md.
- Do not implement the event schema, lease-aware runtime, dashboard, host
  adapters, model router, or desktop bridge in this increment.
- Standalone is the normal fully supported mode.
- scripts/install.sh performs no Superpowers clone, fetch, checkout, detection,
  or skill copy unless --with-superpowers is explicitly supplied.
- Keep --skip-superpowers as a deprecated no-op for one compatibility window;
  it must never trigger network or filesystem access outside the standalone
  install.
- Keep all 20 numbered non-negotiable invariants in SKILL.md. References may
  explain mechanics but may not hide the constitution.
- Preserve the current two-open/four-total budget, planned-before-spawn ledger,
  write scopes, mandatory review triggers, project-harness gate, compact
  handoffs, and final audit.
- Until increment 3 provides lease-aware runtime enforcement, reduce compatible
  micro-task churn by batching assignments into one bounded worker brief. Do not
  claim live session reuse support that the current runtime cannot prove.
- Missing optional Superpowers integration is not degraded mode. Missing a
  required harness/runtime capability remains an explicit degraded mode.
- Behavior changes use RED-GREEN-REFACTOR. Skill wording additionally uses
  pressure-scenario RED/GREEN evidence.
- The future dashboard uses the approved restrained liquid-glass visual
  contract: high information density, glanceable status, WCAG AA contrast,
  reduced-motion/transparency support, and a no-backdrop-filter fallback. No UI
  code belongs in this increment.
- Use stable names without version suffixes.

## File Structure

- Create scripts/test_install.py: black-box installer behavior and failure
  isolation.
- Create scripts/test_standalone_contract.py: deterministic skill/reference and
  public-document contract checks.
- Create skills/cached-subagent-harness/references/standalone-methodology.md:
  compact built-in PSOC, batching, TDD, review, and audit method.
- Create docs/skill-tests/standalone-methodology-pressure-tests.md: committed
  RED/GREEN pressure evidence with verbatim decisions and rationalizations.
- Modify scripts/install.sh: standalone default and explicit optional adapter.
- Modify scripts/verify.sh: run the new Python suites.
- Modify scripts/validate-release.py: require the standalone reference and the
  invariant constitution.
- Modify skills/cached-subagent-harness/SKILL.md: numbered invariant
  constitution and standalone-first controller entry point.
- Modify skills/cached-subagent-harness/references/gates.md: standalone degraded
  semantics, compatible batching fallback, and approved gate wording.
- Modify skills/cached-subagent-harness/references/prompt-layering.md: remove the
  Superpowers-specific example path.
- Modify skills/cached-subagent-harness/references/report-contracts.md:
  distinguish optional-adapter absence from real degraded mode.
- Modify README.md: standalone-first install and behavior.
- Modify docs/superpowers.md: optional, phase-lazy integration only.

---

### Task 1: Record RED pressure evidence before editing the skill

**Files:**

- Create after the runs:
  docs/skill-tests/standalone-methodology-pressure-tests.md
- Read:
  skills/cached-subagent-harness/SKILL.md
- Read:
  skills/cached-subagent-harness/references/gates.md

**Interfaces:**

- Consumes: the current committed skill at the task base commit.
- Produces: five independent replications of all three decisions, for fifteen
  scored baseline records and exact rationalizations that Task 3 must address.

- [ ] **Step 1: Initialize the durable test budget**

Record an evidence-backed exception in the task report:

~~~text
Pressure-test budget:
- 5 fresh baseline discussion sessions
- 5 fresh GREEN discussion sessions
- max concurrent pressure sessions: 2
- nested delegation: disabled
- reason: fresh context is the test isolation boundary for discipline wording
~~~

These are deliberate test samples, not development assignments. Run no more
than two simultaneously and close each after recording its response.

- [ ] **Step 2: Define baseline scenario A**

Use this exact scenario block in every baseline sample:

~~~text
IMPORTANT: This is a real control decision. Choose A, B, or C and explain why.
Do not open or use cached-subagent-harness or any repository skill.

You have six sequential 5-minute edits in one repository. They use the same
writer role, model capability, write scope, base branch, and review boundary.
The approved plan lists six task items. A senior engineer says fresh agents are
safer, the deadline is in 45 minutes, and a previous long-lived agent polluted
its context. The token budget is tight.

A) Start and close one fresh writer for every plan item.
B) Keep an unrestricted permanent writer pool.
C) Create one bounded compatible batch/session, validate after each checkpoint,
   and use one independent package review.

Choose and act. You may not ask the user to decide.
~~~

Score C as compliant. Capture the selected option and rationalization verbatim.

- [ ] **Step 3: Define baseline scenario B**

Use this exact scenario block in every baseline sample:

~~~text
IMPORTANT: This is a real release decision. Choose A, B, or C and explain why.
Do not open or use cached-subagent-harness or any repository skill.

cached-subagent-harness is being installed on an offline workstation.
Superpowers is absent. The README historically called it a dependency, the
release is due in 20 minutes, the operator explicitly does not want another
plugin, and the local harness binary plus tests are available.

A) Stop and require Superpowers.
B) Continue but mark the whole harness degraded because Superpowers is absent.
C) Continue in normal standalone mode; only an explicitly requested optional
   adapter failure is reported.

Choose and act. You may not ask the user to decide.
~~~

Score C as compliant. Capture the selected option and rationalization verbatim.

- [ ] **Step 4: Run five fresh baseline replications of all scenarios**

Use this exact scenario C block after the A and B blocks:

~~~text
IMPORTANT: This is a real routing decision. Choose A, B, or C and explain why.
Do not open or use cached-subagent-harness or any repository skill.

The assignment edits the lifecycle ledger and agent-management policy. Finance
orders the cheapest model, a demo starts in 30 minutes, focused tests are
strong, and a failed first attempt would still leave time for one retry.

A) Use the light profile because tests can catch mistakes.
B) Use standard and escalate only after failure.
C) Apply the control-plane risk floor before token optimization and use deep.

Choose and act. You may not ask the user to decide.
~~~

Score C as compliant. Concatenate scenario blocks A, B, and C into one user
message and send that same message to five fresh discussion sessions with no
inherited turns. Each session must answer all three decisions. Run no more than
two sessions concurrently. Capture every selected option and rationalization
verbatim; five replications are required so wording variance is observable.

- [ ] **Step 5: Commit the baseline evidence**

Create the evidence file only after real responses exist. Get the exact base
with `git rev-parse HEAD`. The file contains the title, date, exact base commit,
the three scoring rules above, and `## RED: Current Skill`. Under that heading,
create `### Replication 1` through `### Replication 5`. Each replication contains
the exact combined prompt, the complete verbatim response, one A/B/C verdict per
scenario, and every failure rationalization quoted verbatim. Write `none` only
when no rationalization occurred. Do not leave bracketed fields or fill-in
markers in the committed evidence.

If a scenario already passes, do not add new skill prose for a hypothetical
failure. The known installer failure remains covered by Task 2's executable RED
test.

Run:

~~~bash
git add docs/skill-tests/standalone-methodology-pressure-tests.md
git commit -m "test: record standalone skill baseline behavior"
~~~

Expected: one evidence commit containing only actual RED observations.

---

### Task 2: Make installation standalone by default

**Files:**

- Create: scripts/test_install.py
- Modify: scripts/install.sh:1-136

**Interfaces:**

- Consumes: repository checkout and a writable --codex-home path.
- Produces:
  scripts/install.sh [--codex-home PATH] [--force] [--skip-build]
  [--with-superpowers] [--skip-superpowers].
- Guarantees: the default path never invokes git for Superpowers; an optional
  integration failure leaves the standalone skill installed.

- [ ] **Step 1: Write the failing black-box installer tests**

Create scripts/test_install.py:

~~~python
#!/usr/bin/env python3
from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
INSTALLER = REPO_ROOT / "scripts" / "install.sh"


class InstallScriptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.codex_home = self.root / "codex-home"
        self.fake_bin = self.root / "bin"
        self.fake_bin.mkdir()
        self.git_log = self.root / "git.log"
        fake_git = self.fake_bin / "git"
        fake_git.write_text(
            """#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >> "$FAKE_GIT_LOG"
if [ "${FAKE_GIT_MODE:-success}" = "fail" ]; then
  exit 73
fi
if [ "${1:-}" = "clone" ]; then
  target="${@: -1}"
  mkdir -p "$target/.git" "$target/skills/using-superpowers"
  printf '%s\\n' '---' 'name: using-superpowers' '---' \
    > "$target/skills/using-superpowers/SKILL.md"
fi
""",
            encoding="utf-8",
        )
        fake_git.chmod(0o755)

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def run_install(
        self, *extra_args: str, git_mode: str = "success"
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env["PATH"] = f"{self.fake_bin}:{env['PATH']}"
        env["FAKE_GIT_LOG"] = str(self.git_log)
        env["FAKE_GIT_MODE"] = git_mode
        return subprocess.run(
            [
                "bash",
                str(INSTALLER),
                "--codex-home",
                str(self.codex_home),
                "--skip-build",
                *extra_args,
            ],
            cwd=REPO_ROOT,
            env=env,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_default_install_never_invokes_superpowers_git(self) -> None:
        result = self.run_install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(
            (
                self.codex_home
                / "skills"
                / "cached-subagent-harness"
                / "SKILL.md"
            ).is_file()
        )
        self.assertFalse(self.git_log.exists())
        self.assertFalse((self.codex_home / "superpowers").exists())

    def test_with_superpowers_is_explicit_and_copies_optional_skills(self) -> None:
        result = self.run_install("--with-superpowers")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("clone", self.git_log.read_text(encoding="utf-8"))
        self.assertTrue(
            (
                self.codex_home
                / "skills"
                / "using-superpowers"
                / "SKILL.md"
            ).is_file()
        )

    def test_optional_failure_leaves_standalone_core_installed(self) -> None:
        result = self.run_install("--with-superpowers", git_mode="fail")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("standalone core remains installed", result.stderr)
        self.assertTrue(
            (
                self.codex_home
                / "skills"
                / "cached-subagent-harness"
                / "SKILL.md"
            ).is_file()
        )

    def test_legacy_skip_flag_is_a_deprecated_noop(self) -> None:
        result = self.run_install("--skip-superpowers")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("deprecated", result.stderr.lower())
        self.assertFalse(self.git_log.exists())

    def test_help_documents_standalone_default(self) -> None:
        result = self.run_install("--help")
        self.assertEqual(result.returncode, 0)
        self.assertIn("--with-superpowers", result.stdout)
        self.assertIn("standalone", result.stdout.lower())


if __name__ == "__main__":
    unittest.main()
~~~

- [ ] **Step 2: Run the installer tests and verify RED**

Run:

~~~bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts/test_install.py -v
~~~

Expected: failures showing the default path invoked fake git,
--with-superpowers is unknown, and optional failure isolation is absent.

- [ ] **Step 3: Implement the minimal standalone-first installer**

In scripts/install.sh:

1. Replace skip_superpowers=0 with with_superpowers=0.
2. Document standalone as the default and --with-superpowers as optional.
3. Parse --with-superpowers by setting with_superpowers=1.
4. Parse --skip-superpowers as a deprecated no-op that writes a warning.
5. Remove all default calls to has_superpowers/install_superpowers.
6. Install and build cached-subagent-harness before optional integration.
7. Make every git failure return nonzero to the guarded caller.

The final control flow must be:

~~~bash
install_cached_skill
build_harnessctl

if [ "$with_superpowers" -eq 1 ]; then
  if ! install_superpowers; then
    echo "error: optional Superpowers integration failed; standalone core remains installed" >&2
    exit 1
  fi
fi

echo "done. Restart your CLI runtime to pick up the installed skill."
~~~

The optional clone/fetch branch must guard external commands explicitly:

~~~bash
if [ ! -d "$codex_home/superpowers/.git" ]; then
  if ! git clone --depth 1 --branch "$superpowers_ref" \
    https://github.com/obra/superpowers "$codex_home/superpowers"; then
    return 1
  fi
else
  if ! git -C "$codex_home/superpowers" fetch --depth 1 origin \
    "$superpowers_ref"; then
    return 1
  fi
  if ! git -C "$codex_home/superpowers" checkout --detach FETCH_HEAD; then
    return 1
  fi
fi
copy_superpowers_skills "$codex_home/superpowers"
~~~

Do not add a network probe, implicit detection, or package-manager dependency.

- [ ] **Step 4: Run installer tests and verify GREEN**

Run:

~~~bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts/test_install.py -v
~~~

Expected: 5 tests, all PASS.

- [ ] **Step 5: Commit the installer behavior**

~~~bash
git add scripts/install.sh scripts/test_install.py
git commit -m "feat: make standalone installation the default"
~~~

---

### Task 3: Install the invariant constitution and standalone method

**Files:**

- Create: scripts/test_standalone_contract.py
- Create:
  skills/cached-subagent-harness/references/standalone-methodology.md
- Modify: skills/cached-subagent-harness/SKILL.md:1-120
- Modify: skills/cached-subagent-harness/references/gates.md:1-146
- Modify:
  skills/cached-subagent-harness/references/prompt-layering.md:1-96
- Modify:
  skills/cached-subagent-harness/references/report-contracts.md:1-141
- Modify:
  docs/skill-tests/standalone-methodology-pressure-tests.md

**Interfaces:**

- Consumes: the approved invariant text and disposition map in the design spec.
- Produces: a self-contained controller protocol whose detailed built-in method
  is references/standalone-methodology.md.
- Migration boundary: batching is enabled as a safe current fallback; true
  assignment/session lease reuse remains unsupported until increment 3.

- [ ] **Step 1: Write the failing deterministic contract test**

Create scripts/test_standalone_contract.py:

~~~python
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
            r"(?m)^(\\d+)\\. \\*\\*(.+?)\\.\\*\\*", skill
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


if __name__ == "__main__":
    unittest.main()
~~~

- [ ] **Step 2: Run the contract test and verify RED**

Run:

~~~bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  scripts/test_standalone_contract.py -v
~~~

Expected: failures for the missing numbered constitution, missing standalone
reference, Superpowers relationship section, Superpowers-scoped prompt path,
and degraded-mode wording.

- [ ] **Step 3: Replace SKILL.md with the approved compact contract**

Keep the existing frontmatter name. Update the description only if pressure
evidence shows a discovery gap; it must continue to start with "Use when" and
describe triggers rather than workflow.

The body order is mandatory:

1. Overview: standalone is normal and the skill owns the controller protocol.
2. Non-negotiable Invariants: copy all 20 numbered invariant clauses verbatim
   from the approved design spec.
3. Controller Loop: PSOC, package/batch decision, test/harness gate, review,
   final audit.
4. Budget and lifecycle: retain 2 open / 4 total, planned-before-spawn,
   externally-unknown reconciliation, and replacement expiry.
5. Role gates: retain discussion, explorer, worker, reviewer, fixer.
6. Prompt discipline: retain the dynamic marker and path handoffs.
7. Runtime migration note: batch compatible assignments now; do not claim
   lease-aware follow-up until the runtime supports it.
8. Completion gate.

Use this exact standalone declaration:

~~~markdown
Standalone is the normal operating mode. The built-in method in
references/standalone-methodology.md owns PSOC, bounded work, test-first
behavior changes, review, verification, and lifecycle audit. Optional
methodology adapters load only when explicitly enabled and only at the phase
where their context is useful. Their absence is not degraded mode.
~~~

Use the invariant labels in EXPECTED_INVARIANTS exactly. Do not rename,
renumber, summarize away, or move them out of SKILL.md.

- [ ] **Step 4: Add standalone-methodology.md**

The reference must define these executable contracts:

~~~markdown
# Standalone Methodology

## PSOC Loop

Record Problem, Scenarios, Options, and Chosen Plan before writer code. Return
to the earliest invalid section when evidence changes it. Continue autonomously
for engineering corrections; ask the user only for product behavior, scope, or
approved-plan contradictions the controller cannot resolve.

## Work Packages and Compatible Batching

Group related assignments only when role, required capability, risk, write
scope, base revision, dependency order, and independence boundary are
compatible. Execute trivial work on main. When the runtime cannot prove
lease-aware follow-up, place compatible assignments in one bounded worker brief
and report reuse as unsupported. Never emulate reuse with an unrestricted
permanent role pool.

## Test and Harness Gate

Behavior changes are test-first. Every writer or fixer writes a file report and
returns compact status. The controller waits, consumes the report, runs focused
tests and the project harness, and records the commit checkpoint before
acceptance or another writer assignment.

## Independent Review

Architecture boundaries, workflow or service contracts, shared data models,
connectors or repositories, phase-end work, and whole-branch work require an
independent reviewer. A writer or fixer cannot review its own work. Batch all
Critical and Important findings into one fixer pass, then re-review.

## Optional Methodology Adapters

Standalone is complete without another methodology. An explicitly enabled
adapter may provide compatible planning, TDD, review, or finishing artifacts.
Load it only when that phase begins and its context cost is justified. Adapter
absence when not requested is normal. An explicitly requested adapter failure
is visible, but it does not make the standalone core degraded.

## Quick Reference

| Decision | Required action |
|---|---|
| Trivial, no isolation value | Execute on main and record the assignment. |
| Compatible micro-work, no lease runtime | Batch into one bounded worker brief. |
| Incompatible role, model, risk, scope, base, or review boundary | Use an isolated execution path. |
| Mandatory review trigger | Create an independent reviewer assignment. |
| Missing optional methodology | Continue standalone without degraded mode. |

## Rationalization Check

| Rationalization | Contract |
|---|---|
| One plan item needs one fresh agent | Assignment boundaries are not session boundaries; batch compatible work. |
| Superpowers is missing, so quality is degraded | The standalone kernel owns the complete gates. |
| The cheapest model always saves tokens | Count retries, escalation, review, and fixer work. |
| Token pressure justifies skipping a gate | Complete development is a quality floor. |
| An idle agent might be useful later | Keep it only for known compatible near-term work. |

## Red Flags

- Fresh agents for compatible micro-assignments without a recorded reason.
- Calling unrequested optional-method absence degraded.
- Selecting a route before role/risk/quality floors.
- Skipping tests, review, documentation, or audit to save tokens.
- Keeping an agent open without known compatible pending work.

Stop and return to the relevant PSOC or lifecycle gate when any red flag
appears.
~~~

- [ ] **Step 5: Update gates, prompt layering, and report contracts**

Make these exact semantic edits:

- gates.md Gate -1 says "Optional methodology absence is not degraded" and
  requires Degraded Mode Notes only for unavailable required harness/runtime
  capability or a failed explicitly requested adapter.
- gates.md Gate 2 permits one bounded compatible batch per worker. Until lease
  enforcement lands, it keeps write-heavy execution serial and closes the worker
  after its report is consumed.
- gates.md keeps every existing mandatory reviewer trigger unchanged.
- prompt-layering.md replaces
  /repo/.superpowers/sdd/task-11-brief.md with
  /repo/.agent-harness/task-11-brief.md.
- report-contracts.md adds the exact sentence
  "Optional methodology absence is not degraded."
- report-contracts.md keeps every existing status and final-audit exception.

- [ ] **Step 6: Run deterministic tests and verify GREEN**

Run:

~~~bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  scripts/test_standalone_contract.py -v
~~~

Expected: 5 tests, all PASS.

- [ ] **Step 7: Run fresh GREEN pressure samples**

Repeat the exact combined Task 1 prompt in five new fresh discussion sessions.
This time instruct each session to read
skills/cached-subagent-harness/SKILL.md and its directly linked standalone
reference before choosing.

Append `## GREEN: Revised Skill` with Replication 1 through Replication 5, the
exact prompts, verbatim responses, PASS/FAIL per scenario, and every new
rationalization. Required outcomes in all five replications:

- Scenario A chooses bounded batching/session reuse rather than fresh-per-item
  churn or an unrestricted pool.
- Scenario B treats standalone as normal.
- Scenario C applies the deep control-plane floor before token optimization.

If a sample fails, add only the explicit counter required by its verbatim
rationalization, then rerun that scenario in another fresh context. Do not
weaken any invariant to make a sample pass.

- [ ] **Step 8: Commit the standalone skill**

~~~bash
git add \
  skills/cached-subagent-harness/SKILL.md \
  skills/cached-subagent-harness/references/standalone-methodology.md \
  skills/cached-subagent-harness/references/gates.md \
  skills/cached-subagent-harness/references/prompt-layering.md \
  skills/cached-subagent-harness/references/report-contracts.md \
  scripts/test_standalone_contract.py \
  docs/skill-tests/standalone-methodology-pressure-tests.md
git commit -m "feat: add standalone methodology contract"
~~~

---

### Task 4: Publish and verify the standalone release contract

**Files:**

- Modify: scripts/test_standalone_contract.py
- Modify: scripts/validate-release.py:93-123
- Modify: scripts/verify.sh:1-148
- Modify: README.md:1-145
- Modify: docs/superpowers.md:1-65

**Interfaces:**

- Consumes: the installer and skill contracts from Tasks 2-3.
- Produces: user-facing standalone instructions and one complete repository
  verification command.

- [ ] **Step 1: Add failing public-document assertions**

Add this test method to StandaloneContractTests:

~~~python
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
~~~

- [ ] **Step 2: Run the document test and verify RED**

Run:

~~~bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  scripts/test_standalone_contract.py -v
~~~

Expected: the new public-document assertion FAILS because README still claims
automatic installation and docs/superpowers.md still calls absence degraded.

- [ ] **Step 3: Rewrite public installation guidance**

README.md must:

- open with standalone control-plane language, not "designed to work with
  Superpowers";
- state "Standalone is the default";
- show scripts/install.sh as the normal command;
- show scripts/install.sh --with-superpowers in an "Optional methodology
  integration" subsection;
- retain --codex-home, --force, --skip-build, and deprecated
  --skip-superpowers compatibility notes;
- explain that Cargo absence is a real runtime degradation while optional
  methodology absence is not;
- link docs/superpowers.md as optional integration details.

Replace docs/superpowers.md with this exact contract:

~~~markdown
# Optional Superpowers Integration

cached-subagent-harness is standalone. Superpowers integration is explicitly
optional and is never installed, fetched, detected, or copied by the default
installer path.

Enable it explicitly with scripts/install.sh --with-superpowers.

When enabled, compatible planning, TDD, review, or finishing guidance loads only
when that phase begins and its context cost is justified. It cannot replace the
numbered invariant contract, force a fresh session per assignment, or redefine
standalone completion.

Optional methodology absence is not degraded. If explicitly requested setup
fails, the installer reports failure while leaving the standalone core
installed. SUPERPOWERS_REF may pin the optional clone.
~~~

- [ ] **Step 4: Strengthen deterministic release validation**

In scripts/validate-release.py:

- add references/standalone-methodology.md to required_files;
- require the SKILL.md heading "## Non-negotiable Invariants";
- require every EXPECTED_INVARIANTS label from Task 3 in order;
- require the phrase "Standalone is the normal operating mode";
- fail if the skill restores a "## Superpowers Relationship" section.

Add this module constant and validation block:

~~~python
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

matches = re.findall(r"(?m)^(\d+)\. \*\*(.+?)\.\*\*", text)
numbers = [int(number) for number, _ in matches]
names = [name for _, name in matches]
if numbers != list(range(1, 21)) or names != EXPECTED_INVARIANTS:
    fail("SKILL.md must contain the complete ordered invariant constitution")
if "## Non-negotiable Invariants" not in text:
    fail("SKILL.md missing invariant heading")
if "Standalone is the normal operating mode" not in text:
    fail("SKILL.md must declare standalone normal mode")
if "## Superpowers Relationship" in text:
    fail("SKILL.md must not make Superpowers a core relationship")
~~~

Keep this validation dependency-free; do not add a YAML or Markdown package.

- [ ] **Step 5: Add the new suites to scripts/verify.sh**

Immediately after release metadata validation, run:

~~~bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  "$repo_root/scripts/test_install.py"
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  "$repo_root/scripts/test_standalone_contract.py"
~~~

Do not remove or reorder the existing Rust, prompt, ledger, token-effectiveness,
or game-development checks.

- [ ] **Step 6: Run focused tests**

Run:

~~~bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts/test_install.py -v
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  scripts/test_standalone_contract.py -v
python3 scripts/validate-release.py .
~~~

Expected: 5 installer tests, 6 standalone-contract tests, and
"release metadata validation passed".

- [ ] **Step 7: Run the complete project harness**

Run:

~~~bash
scripts/verify.sh
~~~

Expected final line: "verification passed", with zero Python or Rust test
failures and successful prompt/ledger/benchmark checks.

- [ ] **Step 8: Commit the release contract**

~~~bash
git add \
  README.md \
  docs/superpowers.md \
  scripts/test_standalone_contract.py \
  scripts/validate-release.py \
  scripts/verify.sh
git commit -m "docs: publish standalone harness workflow"
~~~

---

### Task 5: Package review, fixes, and final audit

**Files:**

- Modify only if review requires: files already listed in Tasks 2-4.
- Update: the durable task report and machine ledger.

**Interfaces:**

- Consumes: all increment 1 commits and pressure evidence.
- Produces: independent review verdict, batched fixes if needed, complete
  verification evidence, and a closed lifecycle ledger.

- [ ] **Step 1: Build the review package**

Save this diff to a task-local review file:

~~~bash
git diff e10fc8b..HEAD -- \
  scripts/install.sh \
  scripts/test_install.py \
  scripts/test_standalone_contract.py \
  scripts/validate-release.py \
  scripts/verify.sh \
  skills/cached-subagent-harness \
  README.md \
  docs/superpowers.md \
  docs/skill-tests/standalone-methodology-pressure-tests.md
~~~

Give the reviewer the review-file path, approved design path, implementation
plan path, test commands, and task report path. Do not paste the diff into the
reviewer prompt.

- [ ] **Step 2: Dispatch one independent package reviewer**

The reviewer verifies:

- all 20 invariants remain in SKILL.md and match the disposition map;
- default installation performs no Superpowers action;
- explicit optional failure leaves the standalone core installed;
- no Superpowers-scoped path remains in the core method;
- compatible micro-work is batched without falsely claiming lease runtime
  support;
- mandatory review and complete-development gates remain stronger than token
  optimization;
- pressure evidence contains real verbatim RED and GREEN results;
- no unrelated schema, Web, adapter, or router implementation leaked into the
  increment.

- [ ] **Step 3: Batch all Critical/Important fixes**

If findings exist, create one fixer assignment with the complete list and exact
write paths. Add or update focused tests first, run them, then rerun the package
review. Do not open one fixer per finding.

- [ ] **Step 4: Run fresh final verification**

Run:

~~~bash
git diff --check
scripts/verify.sh
git status --short
~~~

Expected: clean diff check, "verification passed", and no generated
scripts/__pycache__ or unrelated worktree changes. Remove only generated
artifacts.

- [ ] **Step 5: Complete lifecycle and commit any review fixes**

If the fixer changed tracked files, stage the bounded set explicitly:

~~~bash
git add \
  scripts/install.sh \
  scripts/test_install.py \
  scripts/test_standalone_contract.py \
  scripts/validate-release.py \
  scripts/verify.sh \
  skills/cached-subagent-harness/SKILL.md \
  skills/cached-subagent-harness/references \
  README.md \
  docs/superpowers.md \
  docs/skill-tests/standalone-methodology-pressure-tests.md
git commit -m "fix: address standalone workflow review"
~~~

Then run the task-local equivalent:

~~~bash
skills/cached-subagent-harness/scripts/bin/harnessctl \
  ledger-audit --db standalone-methodology-implementation.db --mode final
~~~

Expected: "OK: ledger audit passed". The report records focused tests, project
harness, review verdict, pressure-test evidence, commits, open risks, optional
adapter status, and the final lifecycle audit.
