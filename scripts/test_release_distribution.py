#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import re
import stat
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
PACKAGER_PATH = REPO_ROOT / "scripts" / "package-release.py"
VALIDATOR_PATH = REPO_ROOT / "scripts" / "validate-release.py"
INSTALL_PS1 = REPO_ROOT / "scripts" / "install.ps1"
TEST_INSTALL_PS1 = REPO_ROOT / "scripts" / "test_install.ps1"
RELEASE_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "release.yml"
CI_WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"
EXPECTED_TARGETS = {
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
}
PINNED_ACTIONS = {
    "actions/checkout": "34e114876b0b11c390a56381ad16ebd13914f8d5",
    "actions/setup-python": "a26af69be951a213d495a4c3e4e4022e16d87065",
    "actions/upload-artifact": "ea165f8d65b6e75b540449e92b4886f43607fa02",
    "actions/download-artifact": "d3f86a106a0bac45b974a628896c90dbdf5c8093",
    "dtolnay/rust-toolchain": "4be7066ada62dd38de10e7b70166bc74ed198c30",
}


def load_packager():
    spec = importlib.util.spec_from_file_location("package_release", PACKAGER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot import {PACKAGER_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def archive_members(path: Path) -> set[str]:
    if path.suffix == ".zip":
        with zipfile.ZipFile(path) as archive:
            return set(archive.namelist())
    with tarfile.open(path, "r:gz") as archive:
        return set(archive.getnames())


class ReleaseArchiveTests(unittest.TestCase):
    def setUp(self) -> None:
        self.module = load_packager()
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.license_path = self.root / "LICENSE"
        self.license_path.write_text("MIT test license\n", encoding="utf-8")

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def make_binary(self, target: str, content: bytes = b"binary\n") -> Path:
        name = "harnessctl.exe" if target.endswith("windows-msvc") else "harnessctl"
        binary = self.root / name
        binary.write_bytes(content)
        binary.chmod(0o755)
        return binary

    def test_asset_names_cover_exact_release_matrix(self) -> None:
        self.assertEqual(set(self.module.SUPPORTED_TARGETS), EXPECTED_TARGETS)
        self.assertEqual(
            self.module.asset_name("0.2.0", "x86_64-unknown-linux-gnu"),
            "harnessctl-v0.2.0-x86_64-unknown-linux-gnu.tar.gz",
        )
        self.assertEqual(
            self.module.asset_name("0.2.0", "x86_64-pc-windows-msvc"),
            "harnessctl-v0.2.0-x86_64-pc-windows-msvc.zip",
        )

    def test_tar_archive_is_reproducible_and_minimal(self) -> None:
        target = "x86_64-unknown-linux-gnu"
        binary = self.make_binary(target)
        first_dir = self.root / "first"
        second_dir = self.root / "second"
        first = self.module.create_archive(
            binary, self.license_path, "0.2.0", target, first_dir
        )
        second = self.module.create_archive(
            binary, self.license_path, "0.2.0", target, second_dir
        )
        self.assertEqual(first.read_bytes(), second.read_bytes())
        self.assertEqual(archive_members(first), {"harnessctl", "LICENSE"})

        with tarfile.open(first, "r:gz") as archive:
            runtime = archive.getmember("harnessctl")
            license_member = archive.getmember("LICENSE")
            self.assertEqual(runtime.mode, 0o755)
            self.assertEqual(license_member.mode, 0o644)
            self.assertEqual(runtime.mtime, 0)
            self.assertEqual(runtime.uid, 0)
            self.assertEqual(runtime.gid, 0)

    def test_windows_zip_is_reproducible_and_minimal(self) -> None:
        target = "x86_64-pc-windows-msvc"
        binary = self.make_binary(target)
        first = self.module.create_archive(
            binary, self.license_path, "0.2.0", target, self.root / "zip-a"
        )
        second = self.module.create_archive(
            binary, self.license_path, "0.2.0", target, self.root / "zip-b"
        )
        self.assertEqual(first.read_bytes(), second.read_bytes())
        self.assertEqual(archive_members(first), {"harnessctl.exe", "LICENSE"})
        with zipfile.ZipFile(first) as archive:
            self.assertEqual(
                {entry.date_time for entry in archive.infolist()},
                {(1980, 1, 1, 0, 0, 0)},
            )
            for entry in archive.infolist():
                self.assertEqual(
                    (entry.external_attr >> 16) & 0o170000,
                    stat.S_IFREG,
                )

    def test_checksum_manifest_is_sorted_and_exact(self) -> None:
        dist = self.root / "dist"
        for target in sorted(EXPECTED_TARGETS):
            self.module.create_archive(
                self.make_binary(target, target.encode("utf-8")),
                self.license_path,
                "0.2.0",
                target,
                dist,
            )

        output = self.module.write_checksums(dist, dist / "SHA256SUMS")
        lines = output.read_text(encoding="utf-8").splitlines()
        self.assertEqual(lines, sorted(lines, key=lambda line: line.split("  ", 1)[1]))
        self.assertEqual(len(lines), 5)
        for line in lines:
            digest, name = line.split("  ", 1)
            self.assertEqual(digest, hashlib.sha256((dist / name).read_bytes()).hexdigest())

    def test_checksum_manifest_rejects_missing_or_unexpected_archive(self) -> None:
        dist = self.root / "dist"
        dist.mkdir()
        (dist / "unexpected.tar.gz").write_bytes(b"unexpected")
        with self.assertRaisesRegex(ValueError, "release archive set"):
            self.module.write_checksums(dist, dist / "SHA256SUMS")

    def test_invalid_version_target_and_binary_suffix_are_rejected(self) -> None:
        for invalid in [
            "v0.2.0",
            "01.2.3",
            "1.02.3",
            "1.2.03",
            "1.2.3-01",
            "1.2.3-alpha..1",
            "1.2.3+build..1",
            "1.2.3-",
            "1.2.3+",
        ]:
            with self.subTest(version=invalid):
                with self.assertRaisesRegex(ValueError, "version"):
                    self.module.asset_name(invalid, "x86_64-unknown-linux-gnu")
        self.assertEqual(
            self.module.asset_name(
                "1.2.3-alpha.1+build.5",
                "x86_64-unknown-linux-gnu",
            ),
            "harnessctl-v1.2.3-alpha.1+build.5-x86_64-unknown-linux-gnu.tar.gz",
        )
        with self.assertRaisesRegex(ValueError, "target"):
            self.module.asset_name("0.2.0", "powerpc-unknown-linux-gnu")
        wrong_binary = self.root / "harnessctl"
        wrong_binary.write_bytes(b"wrong")
        with self.assertRaisesRegex(ValueError, "harnessctl.exe"):
            self.module.create_archive(
                wrong_binary,
                self.license_path,
                "0.2.0",
                "x86_64-pc-windows-msvc",
                self.root / "bad",
            )


class ReleaseMetadataTests(unittest.TestCase):
    def run_validator(self, tag: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(VALIDATOR_PATH), str(REPO_ROOT), "--tag", tag],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_release_tag_matches_plugin_and_cargo_versions(self) -> None:
        result = self.run_validator("v0.2.0")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_release_tag_mismatch_is_rejected(self) -> None:
        result = self.run_validator("v0.2.1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("version", result.stderr.lower() + result.stdout.lower())


class PowerShellInstallerContractTests(unittest.TestCase):
    def test_native_installer_has_verified_source_contract(self) -> None:
        self.assertTrue(INSTALL_PS1.is_file())
        text = INSTALL_PS1.read_text(encoding="utf-8")
        for marker in [
            "Get-PackageVersion",
            "Get-ReleaseTarget",
            "Copy-ReleaseFile",
            "Test-ZipEntryIsRegularFile",
            "Install-StagedRuntime",
            "Install-VerifiedRelease",
            "Build-HarnessRuntime",
            "Invoke-HarnessInstall",
            "x86_64-pc-windows-msvc",
            "Get-FileHash",
            "-Algorithm SHA256",
            "Invoke-WebRequest",
            "Expand-Archive",
            "Move-Item",
            "Auto",
            "Download",
            "Build",
            "None",
        ]:
            self.assertIn(marker, text)
        self.assertNotIn("Invoke-Expression", text)
        self.assertIn("SHA256SUMS", text)
        self.assertIn("harnessctl.exe", text)

    def test_native_installer_has_dependency_free_smoke_test(self) -> None:
        self.assertTrue(TEST_INSTALL_PS1.is_file())
        text = TEST_INSTALL_PS1.read_text(encoding="utf-8")
        self.assertIn("Get-ReleaseTarget", text)
        self.assertIn("Get-PackageVersion", text)
        self.assertIn("Invoke-HarnessInstall", text)
        self.assertIn("BinarySource None", text)
        for scenario in [
            "Start-ReleaseHttpFixture",
            "Test-DownloadSuccess",
            "Test-HttpDownloadSuccess",
            "Test-ChecksumMismatch",
            "Test-MissingAndDuplicateChecksum",
            "Test-UnsafeZipMember",
            "Test-ForcedDownloadNeverBuilds",
            "Test-AutoFallsBackToBuild",
            "Test-BuildNeverDownloads",
            "Test-PathWithSpaces",
            "Test-ReplacementFailure",
        ]:
            self.assertIn(scenario, text)
        self.assertNotIn("Pester", text)


class ReleaseWorkflowContractTests(unittest.TestCase):
    def test_release_workflow_has_exact_matrix_and_publication_gate(self) -> None:
        self.assertTrue(RELEASE_WORKFLOW.is_file())
        text = RELEASE_WORKFLOW.read_text(encoding="utf-8")
        matrix_entries = re.findall(
            r"- os: (\S+)\n\s+target: (\S+)\n\s+binary: (\S+)\n\s+archive: (\S+)",
            text,
        )
        self.assertEqual(
            set(matrix_entries),
            {
                ("ubuntu-24.04", "x86_64-unknown-linux-gnu", "harnessctl", "tar.gz"),
                ("ubuntu-24.04-arm", "aarch64-unknown-linux-gnu", "harnessctl", "tar.gz"),
                ("macos-15-intel", "x86_64-apple-darwin", "harnessctl", "tar.gz"),
                ("macos-15", "aarch64-apple-darwin", "harnessctl", "tar.gz"),
                ("windows-latest", "x86_64-pc-windows-msvc", "harnessctl.exe", "zip"),
            },
        )
        for marker in [
            "workflow_dispatch:",
            "tags:",
            "contents: write",
            "cargo test --locked",
            "package-release.py archive",
            "merge-multiple: true",
            "package-release.py checksums",
            "gh release create",
            "--verify-tag",
            "--notes-file docs/releases/0.2.0.md",
            "refs/tags/v",
            "scripts/verify.sh",
            "scripts/validate-release.py",
        ]:
            self.assertIn(marker, text)
        self.assertNotIn("softprops/", text)
        self.assertNotIn("/releases/latest", text)
        self.assertNotIn("download/latest", text)
        self.assertNotIn("dist/*", text)
        for action, revision in PINNED_ACTIONS.items():
            self.assertIn(f"uses: {action}@{revision}", text)
        self.assertIn("toolchain: 1.96.1", text)
        self.assertNotRegex(text, r"uses: [^\s]+@(?:v\d+|stable)(?:\s|$)")
        extensions = {
            "x86_64-unknown-linux-gnu": "tar.gz",
            "aarch64-unknown-linux-gnu": "tar.gz",
            "x86_64-apple-darwin": "tar.gz",
            "aarch64-apple-darwin": "tar.gz",
            "x86_64-pc-windows-msvc": "zip",
        }
        for target, extension in extensions.items():
            self.assertIn(
                f'"dist/harnessctl-${{GITHUB_REF_NAME}}-{target}.{extension}"',
                text,
            )
        self.assertIn('"dist/SHA256SUMS"', text)

    def test_ci_runs_native_windows_installer_smoke(self) -> None:
        text = CI_WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("windows-install", text)
        self.assertIn("runs-on: windows-latest", text)
        self.assertIn("scripts/test_install.ps1", text)
        self.assertIn("permissions:\n  contents: read", text)
        self.assertIn(
            f"uses: actions/checkout@{PINNED_ACTIONS['actions/checkout']}",
            text,
        )
        self.assertIn(
            f"uses: dtolnay/rust-toolchain@{PINNED_ACTIONS['dtolnay/rust-toolchain']}",
            text,
        )
        self.assertIn("toolchain: 1.96.1", text)


if __name__ == "__main__":
    unittest.main()
