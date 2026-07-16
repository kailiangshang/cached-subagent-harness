#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import io
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
INSTALLER = REPO_ROOT / "scripts" / "install.sh"
BUILD_SCRIPT = REPO_ROOT / "scripts" / "build-harnessctl.sh"
PACKAGE_SCRIPT = REPO_ROOT / "scripts" / "package-release.py"
RELEASE_TARGETS = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
]


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
        self.cargo_log = self.root / "cargo.log"
        self.curl_log = self.root / "curl.log"
        self.real_cp = shutil.which("cp")
        self.real_find = shutil.which("find")
        self.real_curl = shutil.which("curl")
        self.real_uname = shutil.which("uname")
        self.real_true = shutil.which("true")
        self.assertIsNotNone(self.real_cp)
        self.assertIsNotNone(self.real_find)
        self.assertIsNotNone(self.real_curl)
        self.assertIsNotNone(self.real_uname)
        self.assertIsNotNone(self.real_true)

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

        fake_curl = self.fake_bin / "curl"
        fake_curl.write_text(
            """#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$FAKE_CURL_LOG"
exec "$REAL_CURL" "$@"
""",
            encoding="utf-8",
        )
        fake_curl.chmod(0o755)

        fake_uname = self.fake_bin / "uname"
        fake_uname.write_text(
            """#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  -s) printf '%s\n' "${FAKE_UNAME_S:-Linux}" ;;
  -m) printf '%s\n' "${FAKE_UNAME_M:-x86_64}" ;;
  *) exec "$REAL_UNAME" "$@" ;;
esac
""",
            encoding="utf-8",
        )
        fake_uname.chmod(0o755)

        fake_cargo = self.fake_bin / "cargo"
        fake_cargo.write_text(
            """#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$FAKE_CARGO_LOG"
manifest=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--manifest-path" ]; then
    manifest="$2"
    break
  fi
  shift
done
if [ -z "$manifest" ]; then
  exit 71
fi
crate_dir="$(dirname "$manifest")"
mkdir -p "$crate_dir/target/release"
"$REAL_CP" "$REAL_TRUE" "$crate_dir/target/release/harnessctl"
chmod 755 "$crate_dir/target/release/harnessctl"
""",
            encoding="utf-8",
        )
        fake_cargo.chmod(0o755)

        self.release_dir = self.root / "release"
        self.release_dir.mkdir()
        self._create_release_fixture()

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def run_install(
        self,
        *extra_args: str,
        git_mode: str = "success",
        cp_mode: str = "success",
        skip_build: bool = True,
        env_overrides: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env["PATH"] = f"{self.fake_bin}:{env['PATH']}"
        env["FAKE_GIT_LOG"] = str(self.git_log)
        env["FAKE_GIT_MODE"] = git_mode
        env["FAKE_CP_LOG"] = str(self.cp_log)
        env["FAKE_CP_MODE"] = cp_mode
        env["FAKE_FIND_LOG"] = str(self.find_log)
        env["FAKE_CARGO_LOG"] = str(self.cargo_log)
        env["FAKE_CURL_LOG"] = str(self.curl_log)
        env["REAL_CP"] = str(self.real_cp)
        env["REAL_FIND"] = str(self.real_find)
        env["REAL_CURL"] = str(self.real_curl)
        env["REAL_UNAME"] = str(self.real_uname)
        env["REAL_TRUE"] = str(self.real_true)
        if env_overrides:
            env.update(env_overrides)
        command = [
            "bash",
            str(INSTALLER),
            "--codex-home",
            str(self.codex_home),
        ]
        if skip_build:
            command.append("--skip-build")
        command.extend(extra_args)
        return subprocess.run(
            command,
            cwd=REPO_ROOT,
            env=env,
            text=True,
            capture_output=True,
            check=False,
        )

    def _create_release_fixture(self) -> None:
        license_path = self.root / "LICENSE"
        license_path.write_text("MIT fixture\n", encoding="utf-8")
        for target in RELEASE_TARGETS:
            name = "harnessctl.exe" if target.endswith("windows-msvc") else "harnessctl"
            target_dir = self.root / f"binary-{target}"
            target_dir.mkdir()
            binary = target_dir / name
            shutil.copy2(self.real_true, binary)
            result = subprocess.run(
                [
                    sys.executable,
                    str(PACKAGE_SCRIPT),
                    "archive",
                    "--binary",
                    str(binary),
                    "--license",
                    str(license_path),
                    "--version",
                    "0.2.0",
                    "--target",
                    target,
                    "--output-dir",
                    str(self.release_dir),
                ],
                cwd=REPO_ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
        result = subprocess.run(
            [
                sys.executable,
                str(PACKAGE_SCRIPT),
                "checksums",
                "--input-dir",
                str(self.release_dir),
                "--output",
                str(self.release_dir / "SHA256SUMS"),
            ],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def _replace_linux_archive_with_link(self, link_type: bytes, target: str) -> None:
        asset = "harnessctl-v0.2.0-x86_64-unknown-linux-gnu.tar.gz"
        archive_path = self.release_dir / asset
        with tarfile.open(archive_path, "w:gz") as archive:
            license_data = b"MIT fixture\n"
            license_info = tarfile.TarInfo("LICENSE")
            license_info.size = len(license_data)
            archive.addfile(license_info, fileobj=io.BytesIO(license_data))
            link_info = tarfile.TarInfo("harnessctl")
            link_info.type = link_type
            link_info.linkname = target
            archive.addfile(link_info)

        checksum_path = self.release_dir / "SHA256SUMS"
        replacement = f"{hashlib.sha256(archive_path.read_bytes()).hexdigest()}  {asset}"
        lines = checksum_path.read_text(encoding="utf-8").splitlines()
        checksum_path.write_text(
            "\n".join(replacement if line.endswith(asset) else line for line in lines)
            + "\n",
            encoding="utf-8",
        )

    @property
    def installed_runtime(self) -> Path:
        return (
            self.codex_home
            / "skills"
            / "cached-subagent-harness"
            / "scripts"
            / "bin"
            / "harnessctl"
        )

    @property
    def release_base_url(self) -> str:
        return self.release_dir.as_uri()

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

    def test_download_installs_verified_runtime_without_cargo(self) -> None:
        result = self.run_install(
            "--binary-source",
            "download",
            "--release-base-url",
            self.release_base_url,
            skip_build=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.installed_runtime.is_file())
        self.assertTrue(os.access(self.installed_runtime, os.X_OK))
        self.assertEqual(
            subprocess.run([str(self.installed_runtime)], check=False).returncode,
            0,
        )
        self.assertFalse(self.cargo_log.exists())

    def test_download_rejects_checksum_mismatch(self) -> None:
        checksum_path = self.release_dir / "SHA256SUMS"
        lines = checksum_path.read_text(encoding="utf-8").splitlines()
        checksum_path.write_text(
            "\n".join(
                f"{'0' * 64}  {line.split('  ', 1)[1]}"
                if "x86_64-unknown-linux-gnu" in line
                else line
                for line in lines
            )
            + "\n",
            encoding="utf-8",
        )
        result = self.run_install(
            "--binary-source",
            "download",
            "--release-base-url",
            self.release_base_url,
            skip_build=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum", result.stderr.lower())
        self.assertFalse(self.installed_runtime.exists())

    def test_download_rejects_missing_checksum_entry(self) -> None:
        checksum_path = self.release_dir / "SHA256SUMS"
        lines = checksum_path.read_text(encoding="utf-8").splitlines()
        checksum_path.write_text(
            "\n".join(
                line
                for line in lines
                if "x86_64-unknown-linux-gnu" not in line
            )
            + "\n",
            encoding="utf-8",
        )
        result = self.run_install(
            "--binary-source",
            "download",
            "--release-base-url",
            self.release_base_url,
            skip_build=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum", result.stderr.lower())

    def test_download_rejects_symlink_runtime_member(self) -> None:
        self._replace_linux_archive_with_link(tarfile.SYMTYPE, str(self.real_true))
        result = self.run_install(
            "--binary-source",
            "download",
            "--release-base-url",
            self.release_base_url,
            skip_build=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsafe member", result.stderr.lower())
        self.assertFalse(self.installed_runtime.exists())

    def test_download_rejects_hardlink_runtime_member(self) -> None:
        self._replace_linux_archive_with_link(tarfile.LNKTYPE, "LICENSE")
        result = self.run_install(
            "--binary-source",
            "download",
            "--release-base-url",
            self.release_base_url,
            skip_build=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsafe member", result.stderr.lower())
        self.assertFalse(self.installed_runtime.exists())

    def test_forced_download_does_not_fallback_to_cargo(self) -> None:
        result = self.run_install(
            "--binary-source",
            "download",
            "--release-base-url",
            (self.root / "missing-release").as_uri(),
            skip_build=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.cargo_log.exists())
        self.assertFalse(self.installed_runtime.exists())

    def test_auto_falls_back_to_locked_cargo_build(self) -> None:
        result = self.run_install(
            "--binary-source",
            "auto",
            "--release-base-url",
            (self.root / "missing-release").as_uri(),
            skip_build=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.cargo_log.exists())
        self.assertIn("--locked", self.cargo_log.read_text(encoding="utf-8"))
        self.assertTrue(self.installed_runtime.is_file())

    def test_build_source_never_attempts_download(self) -> None:
        result = self.run_install(
            "--binary-source",
            "build",
            skip_build=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.cargo_log.exists())
        self.assertFalse(self.curl_log.exists())
        self.assertTrue(self.installed_runtime.is_file())

    def test_none_and_legacy_skip_build_install_no_runtime(self) -> None:
        result = self.run_install(
            "--binary-source",
            "none",
            skip_build=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("runtime", result.stderr.lower())
        self.assertFalse(self.installed_runtime.exists())

        second_home = self.root / "legacy-home"
        self.codex_home = second_home
        result = self.run_install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("deprecated", result.stderr.lower())
        self.assertFalse(self.installed_runtime.exists())

    def test_unsupported_platform_is_explicit_before_network(self) -> None:
        result = self.run_install(
            "--binary-source",
            "download",
            "--release-base-url",
            self.release_base_url,
            skip_build=False,
            env_overrides={"FAKE_UNAME_S": "Plan9", "FAKE_UNAME_M": "mips"},
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsupported", result.stderr.lower())
        self.assertFalse(self.curl_log.exists())


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
