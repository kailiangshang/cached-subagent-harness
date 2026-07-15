#!/usr/bin/env python3
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
INSTALLER = REPO_ROOT / "scripts" / "install.sh"
BUILD_SCRIPT = REPO_ROOT / "scripts" / "build-harnessctl.sh"


class InstallScriptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.codex_home = self.root / "codex-home"
        self.fake_bin = self.root / "bin"
        self.fake_bin.mkdir()
        self.git_log = self.root / "git.log"
        self.cp_log = self.root / "cp.log"
        self.find_log = self.root / "find.log"
        self.real_cp = shutil.which("cp")
        self.real_find = shutil.which("find")
        self.assertIsNotNone(self.real_cp)
        self.assertIsNotNone(self.real_find)

        fake_cp = self.fake_bin / "cp"
        fake_cp.write_text(
            """#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$FAKE_CP_LOG"
if [ "${FAKE_CP_MODE:-success}" = "fail_optional" ] \
  && [[ "$*" == *"/superpowers/skills/"* ]]; then
  exit 74
fi
exec "$REAL_CP" "$@"
""",
            encoding="utf-8",
        )
        fake_cp.chmod(0o755)

        fake_find = self.fake_bin / "find"
        fake_find.write_text(
            """#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$FAKE_FIND_LOG"
exec "$REAL_FIND" "$@"
""",
            encoding="utf-8",
        )
        fake_find.chmod(0o755)

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
        self,
        *extra_args: str,
        git_mode: str = "success",
        cp_mode: str = "success",
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env["PATH"] = f"{self.fake_bin}:{env['PATH']}"
        env["FAKE_GIT_LOG"] = str(self.git_log)
        env["FAKE_GIT_MODE"] = git_mode
        env["FAKE_CP_LOG"] = str(self.cp_log)
        env["FAKE_CP_MODE"] = cp_mode
        env["FAKE_FIND_LOG"] = str(self.find_log)
        env["REAL_CP"] = str(self.real_cp)
        env["REAL_FIND"] = str(self.real_find)
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
        marker = (
            self.codex_home
            / "plugins"
            / "cache"
            / "sentinel"
            / "skills"
            / "using-superpowers"
            / "SKILL.md"
        )
        marker.parent.mkdir(parents=True)
        marker.write_text("---\nname: using-superpowers\n---\n", encoding="utf-8")

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
        self.assertFalse(self.find_log.exists())
        self.assertNotIn(
            "/superpowers/skills/",
            self.cp_log.read_text(encoding="utf-8"),
        )
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

    def test_optional_copy_failure_is_visible_and_preserves_core(self) -> None:
        result = self.run_install(
            "--with-superpowers",
            cp_mode="fail_optional",
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("optional Superpowers integration failed", result.stderr)
        self.assertTrue(
            (
                self.codex_home
                / "skills"
                / "cached-subagent-harness"
                / "SKILL.md"
            ).is_file()
        )
        self.assertFalse(
            (
                self.codex_home
                / "skills"
                / "using-superpowers"
                / "SKILL.md"
            ).exists()
        )
        self.assertIn(
            "/superpowers/skills/using-superpowers",
            self.cp_log.read_text(encoding="utf-8"),
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


class BuildScriptTests(unittest.TestCase):
    def test_running_binary_is_replaced_atomically(self) -> None:
        sleep_binary = shutil.which("sleep")
        true_binary = shutil.which("true")
        self.assertIsNotNone(sleep_binary)
        self.assertIsNotNone(true_binary)

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            skill_dir = root / "cached-subagent-harness"
            crate_dir = skill_dir / "scripts" / "harnessctl"
            release_dir = crate_dir / "target" / "release"
            bin_dir = skill_dir / "scripts" / "bin"
            fake_bin = root / "fake-bin"
            release_dir.mkdir(parents=True)
            bin_dir.mkdir(parents=True)
            fake_bin.mkdir()
            (crate_dir / "Cargo.toml").write_text(
                '[package]\nname = "harnessctl"\nversion = "0.0.0"\n',
                encoding="utf-8",
            )

            staged_binary = release_dir / "harnessctl"
            live_binary = bin_dir / "harnessctl"
            shutil.copy2(true_binary, staged_binary)
            shutil.copy2(sleep_binary, live_binary)
            staged_binary.chmod(0o755)
            live_binary.chmod(0o755)

            fake_cargo = fake_bin / "cargo"
            fake_cargo.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
            fake_cargo.chmod(0o755)

            running = subprocess.Popen(
                [str(live_binary), "30"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            try:
                env = os.environ.copy()
                env["PATH"] = f"{fake_bin}:{env['PATH']}"
                env["SKILL_DIR"] = str(skill_dir)
                result = subprocess.run(
                    ["bash", str(BUILD_SCRIPT)],
                    cwd=REPO_ROOT,
                    env=env,
                    text=True,
                    capture_output=True,
                    check=False,
                )
            finally:
                running.terminate()
                running.wait(timeout=5)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                subprocess.run([str(live_binary)], check=False).returncode,
                0,
            )


if __name__ == "__main__":
    unittest.main()
