# Verified Binary Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `executing-plans` to
> implement this plan task-by-task and `test-driven-development` for every
> behavior change. The standalone workflow is complete without Superpowers.

**Goal:** Publish `v0.2.0` with five verified `harnessctl` binary archives,
checksum-enforced Bash and PowerShell installation, and an automated GitHub
Release pipeline.

**Architecture:** Keep source and release binaries separate. A deterministic
Python packager creates target-named archives and checksums; checkout-based
installers fetch the exact package version, verify it, and atomically install
the runtime with an explicit Cargo fallback. A tag-only GitHub Actions pipeline
aggregates all native builds before one release publication gate.

**Tech Stack:** Rust/Cargo, Python 3 standard library, Bash, PowerShell 7,
GitHub Actions, GitHub CLI, SHA-256, tar.gz, zip.

## Global Constraints

- Target version is `0.2.0`; public tag is `v0.2.0`.
- Release exactly five targets: `x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
  `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`.
- Linux/macOS archives use `.tar.gz`; Windows uses `.zip`.
- Every Release includes `SHA256SUMS`; downloaded bytes are verified before
  extraction or execution.
- Default binary source is `auto`: exact-version download, then locked Cargo
  fallback. `download` fails closed, `build` never downloads, and `none`
  installs no runtime.
- Preserve `--skip-build` as a deprecated alias for `none`.
- Never select an unbounded `latest` release.
- Do not commit binaries, add background services, change routing/Session
  policy, or claim positive live Token savings.
- Preserve all 20 Skill invariants byte-for-byte.
- Main performs writes serially; delegation is read-only independent review.

---

### Task 1: Versioned deterministic release artifacts

**Files:**
- Create: `scripts/package-release.py`
- Create: `scripts/test_release_distribution.py`
- Modify: `scripts/validate-release.py`
- Modify: `scripts/verify.sh`
- Modify: `.codex-plugin/plugin.json`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml`
- Modify: `skills/cached-subagent-harness/scripts/harnessctl/Cargo.lock`

**Interfaces:**
- Produces `SUPPORTED_TARGETS: dict[str, tuple[str, str]]`,
  `asset_name(version: str, target: str) -> str`,
  `create_archive(binary: Path, license_path: Path, version: str,
  target: str, output_dir: Path) -> Path`, and
  `write_checksums(input_dir: Path, output: Path) -> Path`.
- CLI:
  `package-release.py archive --binary PATH --license PATH --version VERSION
  --target TARGET --output-dir DIR` and
  `package-release.py checksums --input-dir DIR --output PATH`.
- `validate-release.py REPO --tag vX.Y.Z` requires plugin, Cargo, and tag
  versions to match.

- [x] **Step 1: Write failing package and metadata tests**

Add tests that import `package-release.py` through `importlib`, then assert:

```python
def test_asset_names_cover_exact_release_matrix(self):
    self.assertEqual(
        module.asset_name("0.2.0", "x86_64-unknown-linux-gnu"),
        "harnessctl-v0.2.0-x86_64-unknown-linux-gnu.tar.gz",
    )
    self.assertEqual(
        module.asset_name("0.2.0", "x86_64-pc-windows-msvc"),
        "harnessctl-v0.2.0-x86_64-pc-windows-msvc.zip",
    )
    self.assertEqual(set(module.SUPPORTED_TARGETS), EXPECTED_TARGETS)

def test_archives_are_reproducible_and_contain_only_runtime_and_license(self):
    first = module.create_archive(binary, license_path, "0.2.0", target, out_a)
    second = module.create_archive(binary, license_path, "0.2.0", target, out_b)
    self.assertEqual(first.read_bytes(), second.read_bytes())
    self.assertEqual(read_member_names(first), {"harnessctl", "LICENSE"})

def test_checksums_are_sorted_and_exact(self):
    output = module.write_checksums(dist, dist / "SHA256SUMS")
    self.assertEqual(output.read_text().splitlines(), sorted(expected_lines))
```

Also invoke `validate-release.py . --tag v0.2.1` and expect nonzero version
mismatch, while `--tag v0.2.0` succeeds after the version update.

- [x] **Step 2: Run the focused suite and observe RED**

Run:

```bash
python3 -m unittest scripts.test_release_distribution -v
```

Expected: FAIL because `scripts/package-release.py` and the version/tag
contract do not exist.

- [x] **Step 3: Implement deterministic packaging and validation**

Implement an allowlisted target table. Normalize versions by accepting
`0.2.0` at the function boundary and emitting the `v` only in asset names.
For tar archives, use `gzip.GzipFile(mtime=0)` plus `tarfile.TarInfo` with
`mtime=0`, `uid=gid=0`, empty owner names, runtime mode `0o755`, and license
mode `0o644`. For zip archives, use fixed `1980-01-01` timestamps and explicit
Unix permission attributes. Reject a missing/non-file binary, wrong Windows
suffix, unsupported target, invalid semantic version, or output collision.

Generate checksums only for the five allowlisted archives, sort by asset name,
and emit `<hex>  <asset>`. Reject missing, duplicate, or unexpected archives.

Update plugin and Cargo versions to `0.2.0`; refresh the lockfile with:

```bash
cargo check --locked --manifest-path \
  skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml
```

If `--locked` reports the expected local package version drift, run
`cargo check --manifest-path ...` once to refresh `Cargo.lock`, then rerun with
`--locked`.

- [x] **Step 4: Add the focused suite to full verification and run GREEN**

Modify `scripts/verify.sh` to run:

```bash
python3 -m unittest scripts.test_release_distribution
```

Run:

```bash
python3 -m unittest scripts.test_release_distribution -v
python3 scripts/validate-release.py . --tag v0.2.0
```

Expected: all release distribution tests pass and metadata validation exits 0.

- [x] **Step 5: Commit Task 1**

```bash
git add .codex-plugin/plugin.json scripts/package-release.py \
  scripts/test_release_distribution.py scripts/validate-release.py \
  scripts/verify.sh skills/cached-subagent-harness/scripts/harnessctl/Cargo.toml \
  skills/cached-subagent-harness/scripts/harnessctl/Cargo.lock
git commit -m "feat: package versioned harnessctl releases"
```

### Task 2: Verified Bash runtime acquisition

**Files:**
- Create: `scripts/install-runtime.sh`
- Modify: `scripts/install.sh`
- Modify: `scripts/test_install.py`

**Interfaces:**
- `detect_release_target() -> stdout target` maps only supported Linux/macOS
  OS/architecture pairs.
- `install_verified_release VERSION BASE_URL SKILL_DIR -> 0|nonzero` downloads,
  verifies, validates, and atomically replaces the runtime.
- `build_runtime SKILL_DIR -> 0|nonzero` calls the locked repository build.
- Bash CLI adds `--binary-source auto|download|build|none` and
  `--release-base-url URL`; `HARNESS_RELEASE_BASE_URL` is the test/mirror
  environment equivalent.

- [x] **Step 1: Write failing Bash installer tests**

Extend `InstallScriptTests` with a fixture that calls the Task 1 packager to
create a real local release directory and runs the installer against its
`file://` URL. Cover:

```python
def test_download_installs_verified_runtime_without_cargo(self): ...
def test_download_rejects_checksum_mismatch(self): ...
def test_download_rejects_missing_checksum_entry(self): ...
def test_forced_download_does_not_fallback_to_cargo(self): ...
def test_auto_falls_back_to_locked_cargo_build(self): ...
def test_build_source_never_attempts_download(self): ...
def test_none_and_legacy_skip_build_install_no_runtime(self): ...
def test_unsupported_platform_is_explicit(self): ...
```

The successful assertion is that the installed runtime bytes match the
packaged fixture, are executable, and no fake Cargo invocation was recorded.

- [x] **Step 2: Run tests and observe RED**

Run:

```bash
python3 -m unittest scripts.test_install -v
```

Expected: new cases fail because binary-source parsing and verified download
do not exist.

- [x] **Step 3: Implement the minimal Bash acquisition library**

Use strict Bash mode, quoted arrays, `mktemp -d`, and a cleanup trap. Detect
`Linux`/`Darwin` plus `x86_64|amd64` and `aarch64|arm64`. Use `curl --fail
--location --silent --show-error`; do not use `eval`. Select exactly one
checksum line by exact asset-name comparison, verify through `sha256sum` or
`shasum -a 256`, inspect archive member names before extraction, require exactly
`harnessctl` and `LICENSE`, then atomically rename a `chmod 755` temporary
runtime into `scripts/bin/harnessctl`.

In `install.sh`, copy the Skill first, acquire the runtime according to the
selected source, and return nonzero when a required source fails. Preserve the
installed standalone core on failure. Keep optional Superpowers work after
successful core runtime handling. Print the requested version, selected source,
observed target, and final runtime path without printing secrets or URLs with
credentials.

- [x] **Step 4: Run installer GREEN and regression checks**

Run:

```bash
python3 -m unittest scripts.test_install -v
bash -n scripts/install.sh scripts/install-runtime.sh
```

Expected: every installer test passes and both scripts parse cleanly.

- [x] **Step 5: Commit Task 2**

```bash
git add scripts/install.sh scripts/install-runtime.sh scripts/test_install.py
git commit -m "feat: install verified release binaries"
```

### Task 3: Native Windows installer

**Files:**
- Create: `scripts/install.ps1`
- Create: `scripts/test_install.ps1`
- Modify: `scripts/test_release_distribution.py`

**Interfaces:**
- PowerShell parameters:
  `-CodexHome`, `-Force`,
  `-BinarySource Auto|Download|Build|None`, and `-ReleaseBaseUrl`.
- Functions `Get-PackageVersion`, `Get-ReleaseTarget`,
  `Install-VerifiedRelease`, `Build-HarnessRuntime`, and
  `Invoke-HarnessInstall` mirror the Bash policy.
- `scripts/test_install.ps1` is a dependency-free assertion runner returning
  nonzero on any failed contract.

- [ ] **Step 1: Add failing static and PowerShell contract tests**

In Python, require the PowerShell file and exact source values, fixed Windows
target, checksum use through `Get-FileHash -Algorithm SHA256`, temporary
extraction, and absence of `Invoke-Expression`.

In `test_install.ps1`, dot-source the installer and assert:

```powershell
Assert-Equal (Get-ReleaseTarget) 'x86_64-pc-windows-msvc'
Assert-Equal (Get-PackageVersion $RepoRoot) '0.2.0'
Invoke-HarnessInstall -CodexHome $TempHome -BinarySource None
Assert-True (Test-Path "$TempHome/skills/cached-subagent-harness/SKILL.md")
Assert-False (Test-Path "$TempHome/skills/cached-subagent-harness/scripts/bin/harnessctl.exe")
```

- [ ] **Step 2: Run available tests and observe RED**

Run on Linux:

```bash
python3 -m unittest scripts.test_release_distribution -v
```

Expected: FAIL because the PowerShell contract files do not exist. On Windows,
`pwsh -NoProfile -File scripts/test_install.ps1` must also fail before
implementation.

- [ ] **Step 3: Implement the PowerShell installer**

Use terminating errors, `try/finally` cleanup, `Invoke-WebRequest`, exact
checksum-line selection, `Get-FileHash`, `Expand-Archive`, exact member-set
validation, and `Move-Item` into `scripts/bin/harnessctl.exe`. `Auto` catches
only release-acquisition failure before entering Cargo fallback. `Download` and
`Build` never cross sources. Preserve the copied Skill on a runtime failure and
emit an actionable nonzero error.

- [ ] **Step 4: Run GREEN checks**

Run locally:

```bash
python3 -m unittest scripts.test_release_distribution -v
```

Run on Windows CI:

```powershell
pwsh -NoProfile -File scripts/test_install.ps1
```

Expected: Python contracts and native PowerShell assertions pass.

- [ ] **Step 5: Commit Task 3**

```bash
git add scripts/install.ps1 scripts/test_install.ps1 \
  scripts/test_release_distribution.py
git commit -m "feat: add native Windows installation"
```

### Task 4: Cross-platform release workflow

**Files:**
- Create: `.github/workflows/release.yml`
- Modify: `.github/workflows/ci.yml`
- Modify: `scripts/test_release_distribution.py`

**Interfaces:**
- Workflow supports `workflow_dispatch` dry runs and tag `v*` publication.
- Matrix entries provide `os`, `target`, `binary`, and `archive` for all five
  allowlisted targets.
- `publish` depends on `verify` and every matrix build, downloads merged
  artifacts, generates `SHA256SUMS`, validates the exact asset set, and runs
  `gh release create` only for a tag event.

- [ ] **Step 1: Write failing workflow contract tests**

Read the YAML as text and assert the exact target set, `contents: write`,
manual dispatch, tag trigger, `cargo test --locked`, Task 1 packaging CLI,
artifact aggregation, checksum generation, `gh release create --verify-tag`,
and a publication condition restricted to `refs/tags/v`. Reject `latest`,
wildcard asset upload, and third-party release actions.

- [ ] **Step 2: Run tests and observe RED**

```bash
python3 -m unittest scripts.test_release_distribution -v
```

Expected: FAIL because `release.yml` and the Windows CI job are absent.

- [ ] **Step 3: Implement verify/build/publish jobs**

Use official `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`,
`actions/upload-artifact@v4`, and `actions/download-artifact@v4`. The verify job
runs `scripts/verify.sh` and the tag equality gate. Native runners are
`ubuntu-24.04`, `ubuntu-24.04-arm`, `macos-15`, `macos-15-arm64`, and
`windows-latest`. Every matrix job runs locked tests, builds its explicit
target, packages exactly one archive, and uploads it under a target-specific
artifact name.

The publish job merges artifacts, generates checksums, verifies six files, and
uses GitHub CLI with `${{ github.token }}`. Manual runs stop after uploading
workflow artifacts. Add a Windows CI job that runs the PowerShell test on every
push and pull request.

- [ ] **Step 4: Run GREEN static and full local checks**

```bash
python3 -m unittest scripts.test_release_distribution -v
python3 scripts/validate-release.py . --tag v0.2.0
scripts/verify.sh
```

Expected: contract tests, release validation, and full verification pass.

- [ ] **Step 5: Commit Task 4**

```bash
git add .github/workflows/ci.yml .github/workflows/release.yml \
  scripts/test_release_distribution.py
git commit -m "ci: publish cross-platform harnessctl releases"
```

### Task 5: Product positioning, install documentation, and release notes

**Files:**
- Create: `docs/releases/0.2.0.md`
- Modify: `README.md`
- Modify: `docs/current-state.md`
- Modify: `binary-release-implementation.md`
- Modify: `scripts/test_standalone_contract.py`

**Interfaces:**
- README presents prebuilt installation before source build and explicitly
  describes exact-version download, checksum verification, Cargo fallback,
  supported targets, unsigned-binary boundary, and native PowerShell use.
- Product copy calls the project a long-running Token-aware control plane and
  distinguishes prevention of known regressions from proven net savings.
- Release notes enumerate assets, install modes, compatibility, evidence
  boundary, and known unsigned-binary limitation.

- [ ] **Step 1: Write failing public-contract tests**

Add assertions for:

```python
self.assertIn("long-running", readme)
self.assertIn("does not claim positive", readme)
self.assertIn("SHA256SUMS", readme)
self.assertIn("scripts/install.ps1", readme)
self.assertIn("--binary-source", readme)
self.assertIn("unsigned", readme.lower())
```

Require `docs/releases/0.2.0.md` and current-state links to the release design,
plan, implementation report, and release notes.

- [ ] **Step 2: Run tests and observe RED**

```bash
python3 -m unittest scripts.test_standalone_contract -v
```

Expected: FAIL on missing v0.2.0 release documentation.

- [ ] **Step 3: Write minimal truthful documentation**

Document both installers and all binary-source modes. Keep compile-from-source
instructions as an explicit fallback. State that SHA-256 verifies artifact
integrity but binaries are not code-signed/notarized. Retain the exact negative
A/B values and make no positive batching/reuse savings claim.

- [ ] **Step 4: Run focused and full verification**

```bash
python3 -m unittest scripts.test_standalone_contract -v
scripts/verify.sh
git diff --check
```

Expected: focused contracts and every project gate pass.

- [ ] **Step 5: Commit Task 5**

```bash
git add README.md docs/current-state.md docs/releases/0.2.0.md \
  binary-release-implementation.md scripts/test_standalone_contract.py
git commit -m "docs: prepare v0.2.0 binary release"
```

### Task 6: Independent review, public Release, and lifecycle closure

**Files:**
- Modify: files named by accepted review findings only
- Modify: `binary-release-implementation.md`

**Interfaces:**
- Review package contains design, plan, report, `git diff v0.1.0...HEAD`,
  workflow, installers, packaging tool, tests, and verification output by path.
- Release acceptance uses the GitHub API to confirm tag, five archives,
  `SHA256SUMS`, nonzero asset sizes, and published state.

- [ ] **Step 1: Run final local verification before review**

```bash
scripts/verify.sh
python3 scripts/validate-release.py . --tag v0.2.0
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 2: Dispatch independent release/security review**

Record the planned reviewer in the machine ledger before spawn. Require
severity-ordered findings for archive safety, installer quoting and fallback,
workflow permissions and publication gating, version consistency, platform
matrix, documentation truthfulness, and all 20 invariants.

- [ ] **Step 3: Fix and re-review findings**

Batch all Critical and Important findings into one test-first fixer pass. Run
focused tests and `scripts/verify.sh`, commit the fixes, and dispatch a fresh
read-only re-review. Do not publish with an unresolved Critical or Important
finding.

- [ ] **Step 4: Push source, create and push the immutable tag**

```bash
git push origin main
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

Expected: the release workflow starts from the exact pushed commit.

- [ ] **Step 5: Monitor and inspect the public Release**

Poll GitHub's Actions and Releases APIs without exposing credentials. Require
workflow success and verify the public Release contains exactly:

```text
harnessctl-v0.2.0-x86_64-unknown-linux-gnu.tar.gz
harnessctl-v0.2.0-aarch64-unknown-linux-gnu.tar.gz
harnessctl-v0.2.0-x86_64-apple-darwin.tar.gz
harnessctl-v0.2.0-aarch64-apple-darwin.tar.gz
harnessctl-v0.2.0-x86_64-pc-windows-msvc.zip
SHA256SUMS
```

Download the current Linux archive and checksum, verify it, extract it into a
temporary directory, and run `harnessctl --help` before accepting the Release.

- [ ] **Step 6: Close the durable Run**

Update every reviewer Task and Session to terminal accepted/closed state, run:

```bash
skills/cached-subagent-harness/scripts/bin/harnessctl audit \
  --db binary-release-implementation.db --run binary-release-20260716
```

Record exact tests, commits, workflow URL/status, Release URL/assets, known
unsigned-binary limitation, review disposition, and final audit in
`binary-release-implementation.md`. Commit and push the closure report, then
confirm `HEAD`, `origin/main`, and the public Release source commit agree.
