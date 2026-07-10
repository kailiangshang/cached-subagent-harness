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
printf '%s\n' "$*" >> "$FAKE_GIT_LOG"
if [ "${FAKE_GIT_MODE:-success}" = "fail" ]; then
  exit 73
fi
if [ "${1:-}" = "clone" ]; then
  target="${@: -1}"
  mkdir -p "$target/.git" "$target/skills/using-superpowers"
  printf '%s\n' '---' 'name: using-superpowers' '---' \
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
