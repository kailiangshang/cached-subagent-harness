#!/usr/bin/env python3
"""Create deterministic harnessctl release archives and checksum manifests."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import re
import stat
import tarfile
import zipfile
from pathlib import Path


SEMVER_NUMERIC = r"(?:0|[1-9][0-9]*)"
SEMVER_PRERELEASE_ID = (
    r"(?:0|[1-9][0-9]*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)"
)
SEMVER_BUILD_ID = r"[0-9A-Za-z-]+"
SEMVER_PATTERN = (
    rf"{SEMVER_NUMERIC}\.{SEMVER_NUMERIC}\.{SEMVER_NUMERIC}"
    rf"(?:-(?:{SEMVER_PRERELEASE_ID})(?:\.(?:{SEMVER_PRERELEASE_ID}))*)?"
    rf"(?:\+(?:{SEMVER_BUILD_ID})(?:\.(?:{SEMVER_BUILD_ID}))*)?"
)
VERSION_RE = re.compile(rf"^{SEMVER_PATTERN}$")
SUPPORTED_TARGETS: dict[str, tuple[str, str]] = {
    "x86_64-unknown-linux-gnu": ("harnessctl", ".tar.gz"),
    "aarch64-unknown-linux-gnu": ("harnessctl", ".tar.gz"),
    "x86_64-apple-darwin": ("harnessctl", ".tar.gz"),
    "aarch64-apple-darwin": ("harnessctl", ".tar.gz"),
    "x86_64-pc-windows-msvc": ("harnessctl.exe", ".zip"),
}
ASSET_RE = re.compile(
    rf"^harnessctl-v(?P<version>{SEMVER_PATTERN})-"
    r"(?P<target>.+?)(?:\.tar\.gz|\.zip)$"
)


def _validate_version(version: str) -> None:
    if not VERSION_RE.fullmatch(version):
        raise ValueError(f"invalid release version: {version}")


def asset_name(version: str, target: str) -> str:
    _validate_version(version)
    try:
        _, extension = SUPPORTED_TARGETS[target]
    except KeyError as error:
        raise ValueError(f"unsupported release target: {target}") from error
    return f"harnessctl-v{version}-{target}{extension}"


def _member_info(name: str, data: bytes, mode: int) -> tarfile.TarInfo:
    info = tarfile.TarInfo(name=name)
    info.size = len(data)
    info.mode = mode
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    return info


def _create_tar_gz(output: Path, members: list[tuple[str, bytes, int]]) -> None:
    with output.open("xb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
            with tarfile.open(
                mode="w", fileobj=compressed, format=tarfile.USTAR_FORMAT
            ) as archive:
                for name, data, mode in members:
                    archive.addfile(_member_info(name, data, mode), io.BytesIO(data))


def _create_zip(output: Path, members: list[tuple[str, bytes, int]]) -> None:
    with zipfile.ZipFile(
        output, mode="x", compression=zipfile.ZIP_DEFLATED, compresslevel=9
    ) as archive:
        for name, data, mode in members:
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = ((stat.S_IFREG | mode) & 0xFFFF) << 16
            archive.writestr(info, data, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)


def create_archive(
    binary: Path,
    license_path: Path,
    version: str,
    target: str,
    output_dir: Path,
) -> Path:
    _validate_version(version)
    try:
        expected_binary, extension = SUPPORTED_TARGETS[target]
    except KeyError as error:
        raise ValueError(f"unsupported release target: {target}") from error

    binary = Path(binary)
    license_path = Path(license_path)
    output_dir = Path(output_dir)
    if not binary.is_file():
        raise ValueError(f"release binary is not a file: {binary}")
    if binary.name != expected_binary:
        raise ValueError(
            f"release target {target} requires binary named {expected_binary}"
        )
    if not license_path.is_file():
        raise ValueError(f"license is not a file: {license_path}")

    output_dir.mkdir(parents=True, exist_ok=True)
    output = output_dir / asset_name(version, target)
    members = [
        (expected_binary, binary.read_bytes(), 0o755),
        ("LICENSE", license_path.read_bytes(), 0o644),
    ]
    if extension == ".zip":
        _create_zip(output, members)
    else:
        _create_tar_gz(output, members)
    return output


def _release_archives(input_dir: Path) -> tuple[str, list[Path]]:
    files = sorted(
        path
        for path in Path(input_dir).iterdir()
        if path.is_file() and path.name != "SHA256SUMS"
    )
    versions: set[str] = set()
    for path in files:
        match = ASSET_RE.fullmatch(path.name)
        if match and match.group("target") in SUPPORTED_TARGETS:
            versions.add(match.group("version"))
    if len(versions) != 1:
        raise ValueError("release archive set must contain exactly one version")
    version = versions.pop()
    expected_names = {asset_name(version, target) for target in SUPPORTED_TARGETS}
    actual_names = {path.name for path in files}
    if actual_names != expected_names:
        missing = sorted(expected_names - actual_names)
        unexpected = sorted(actual_names - expected_names)
        raise ValueError(
            "release archive set mismatch: "
            f"missing={missing}, unexpected={unexpected}"
        )
    return version, files


def write_checksums(input_dir: Path, output: Path) -> Path:
    input_dir = Path(input_dir)
    output = Path(output)
    if not input_dir.is_dir():
        raise ValueError(f"release input directory does not exist: {input_dir}")
    _, archives = _release_archives(input_dir)
    if output.parent.resolve() != input_dir.resolve():
        raise ValueError("checksum manifest must be written inside the release directory")
    lines = [
        f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}"
        for path in sorted(archives, key=lambda path: path.name)
    ]
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return output


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    archive = subparsers.add_parser("archive", help="create one target archive")
    archive.add_argument("--binary", type=Path, required=True)
    archive.add_argument("--license", dest="license_path", type=Path, required=True)
    archive.add_argument("--version", required=True)
    archive.add_argument("--target", required=True)
    archive.add_argument("--output-dir", type=Path, required=True)

    checksums = subparsers.add_parser(
        "checksums", help="create SHA256SUMS for the exact release matrix"
    )
    checksums.add_argument("--input-dir", type=Path, required=True)
    checksums.add_argument("--output", type=Path, required=True)
    return parser


def main() -> None:
    args = _build_parser().parse_args()
    try:
        if args.command == "archive":
            output = create_archive(
                args.binary,
                args.license_path,
                args.version,
                args.target,
                args.output_dir,
            )
        else:
            output = write_checksums(args.input_dir, args.output)
    except (OSError, ValueError, zipfile.BadZipFile, tarfile.TarError) as error:
        raise SystemExit(f"error: {error}") from error
    print(output)


if __name__ == "__main__":
    main()
