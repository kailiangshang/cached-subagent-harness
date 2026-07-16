# Binary Release Productization Design

Date: 2026-07-16
Status: approved by user for autonomous implementation and public release
Target release: `v0.2.0`

## Product Position

`cached-subagent-harness` is a token-aware control plane for long-running,
multi-stage agentic development. Its proven value is durable recovery,
bounded delegation, quality-constrained routing, lifecycle enforcement,
truthful accounting, and prevention of known high-cost Session patterns. It
does not claim positive end-to-end Token savings until equal-quality exact
usage evidence supports that claim.

This increment productizes distribution. It does not change the routing,
batching, reuse, accounting, dashboard, or 20-invariant contracts.

## Problem

The repository builds `harnessctl` locally, but the executable is ignored and
GitHub has no Release assets. Installation therefore requires Cargo on the
target machine or leaves the required deterministic runtime unavailable. That
friction conflicts with a lightweight standalone Skill and prevents a normal
upgrade path.

## Scenarios

1. A Linux or macOS user installs from a checkout. The installer detects the
   platform, downloads the matching `v0.2.0` archive, verifies the published
   SHA-256 digest, and atomically installs the executable without Cargo.
2. A native Windows user runs the PowerShell installer, receives the verified
   Windows executable, and installs the same Skill layout.
3. The network, release, archive, platform, or checksum is unavailable. `auto`
   mode falls back to a locked Cargo build; `download` mode fails closed; an
   explicitly selected `none` mode installs only the Skill and reports the
   missing runtime boundary.
4. A maintainer pushes a version tag. CI verifies that the tag, plugin version,
   and Cargo version agree, builds every supported target, packages deterministic
   archives, creates `SHA256SUMS`, and publishes one GitHub Release only after
   every build succeeds.
5. A later bug fix publishes a new semantic version. Existing releases remain
   immutable and installers derive the requested asset names from the checked
   out package version rather than silently selecting `latest`.

## Options

### Option A: keep compile-on-install

Smallest repository change, but every user needs Rust and a platform toolchain.
It preserves the current product gap and is rejected.

### Option B: publish verified binaries with source-build fallback

Publish platform archives and checksums, download the exact package version by
default, and preserve a locked Cargo fallback. This minimizes installation
friction without committing platform binaries or hiding verification failures.
This is the chosen option.

### Option C: commit binaries into the Skill

The Skill becomes immediately runnable from a checkout, but the repository
grows per platform, diffs become opaque, and provenance is weaker. It is
rejected.

## Release Contract

The release matrix is:

| Platform | Rust target | Archive |
|---|---|---|
| Linux x86-64 | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| macOS Intel | `x86_64-apple-darwin` | `.tar.gz` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` |
| Windows x86-64 | `x86_64-pc-windows-msvc` | `.zip` |

Asset names are stable data:

```text
harnessctl-v<VERSION>-<RUST_TARGET>.tar.gz
harnessctl-v<VERSION>-<RUST_TARGET>.zip
SHA256SUMS
```

Every archive contains only the executable and `LICENSE`. GitHub's generated
source archives remain the Skill/source distribution. Release binaries are not
committed to Git.

## Release Pipeline

`.github/workflows/release.yml` runs only for `v*` tags and manual dry runs.
Manual runs build and retain workflow artifacts but never publish a GitHub
Release. Tag publication uses this sequence:

1. validate repository metadata and require tag/version equality;
2. run the existing full verification on Linux;
3. build and test the five target artifacts on native GitHub runners;
4. package each binary through one cross-platform deterministic Python tool;
5. aggregate artifacts and generate `SHA256SUMS` from the final bytes;
6. create the GitHub Release with the immutable tag and generated notes.

The workflow uses GitHub's own token and CLI for publication. It never embeds
credentials in scripts or artifacts. A failed matrix job prevents publication.

## Installer Contract

Both installers use one policy with four binary sources:

| Source | Behavior |
|---|---|
| `auto` | verified download, then locked Cargo fallback |
| `download` | verified download or nonzero failure |
| `build` | locked Cargo build only |
| `none` | install Skill only and disclose missing runtime |

The existing `--skip-build` Bash flag remains a deprecated compatibility alias
for `none`. Installers use the version in `.codex-plugin/plugin.json`, never an
unbounded `latest` lookup. Tests may override the release base URL but normal
users receive the fixed GitHub repository URL.

Download installation is fail-closed:

- unsupported platforms are rejected before network access;
- both the archive and `SHA256SUMS` must download successfully;
- the exact asset must have exactly one checksum entry;
- digest mismatch, unsafe archive shape, or missing executable is fatal;
- extraction occurs in a temporary directory;
- the runtime is replaced atomically and marked executable where applicable.

In `auto` mode only, a download failure may enter the Cargo fallback. If neither
path succeeds, the installer returns nonzero while preserving the copied Skill
and explaining how to retry. Optional Superpowers integration remains separate
and does not affect binary acquisition.

## Components

- `scripts/package-release.py`: deterministic archive and checksum creation.
- `scripts/install.sh`: Bash platform detection, verified download, and Cargo
  fallback.
- `scripts/install.ps1`: native Windows install with the same source policy.
- `scripts/validate-release.py`: version and release-contract validation.
- `.github/workflows/release.yml`: build matrix and GitHub Release publication.
- installer/package/release contract tests: offline, deterministic coverage.
- public documentation: exact install paths, unsigned-binary boundary, product
  position, and evidence limits.

## Security and Error Handling

- SHA-256 provides transport/artifact integrity; this release does not add code
  signing or notarization and documents that limitation.
- Tag, plugin, Cargo, and asset versions must agree.
- Archives reject absolute paths, parent traversal, extra executables, and an
  unexpected root layout.
- Downloaded bytes are never executed before verification.
- Release publication requires all tests and matrix artifacts.
- Requested release facts remain separate from observed installed facts; an
  unavailable binary is never reported as successful installation.

## Testing and Acceptance

Behavior changes follow RED/GREEN tests. Acceptance requires:

- package creation is deterministic and rejects invalid inputs;
- Bash installer tests cover platform mapping, download success, checksum
  mismatch, missing checksum, unsupported platform, Cargo fallback, forced
  download failure, and backward-compatible flags;
- PowerShell smoke tests run on Windows CI;
- release validation rejects version or asset-matrix drift;
- the existing Rust, Python, benchmark, prompt, Skill, and lifecycle suites
  remain green;
- a fresh independent review covers supply-chain safety, shell quoting,
  workflow permissions, release immutability, and documentation truthfulness;
- the release tag produces all five archives plus `SHA256SUMS`, and the public
  GitHub Release is inspected before completion.

## Non-goals

- no auto-updater inside `harnessctl`;
- no background service, desktop bridge, observer LLM, or package registry;
- no Apple notarization, Windows Authenticode signing, or Linux package-manager
  repository in this increment;
- no change to model routing, Session budgets, batching, Dashboard schema, or
  positive Token-saving claims;
- no committed platform binary.

## Invariant Mapping

This release preserves all 20 Skill invariants. The most directly exercised
ones are complete development and evidence gates (3, 6, 7), durable truth and
recovery (8, 17-19), bounded Token behavior (12-16), and stable identity (20).
Release versions and target triples are data, not versioned role or policy
names.
