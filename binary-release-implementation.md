# Binary Release Implementation

Status: independent review fixes implemented and locally verified; re-review,
native CI, and public Release acceptance pending

## PSOC

### Problem

`harnessctl` has verified source and a local release build, but no public binary
Release. Default installation therefore depends on Cargo or leaves the required
runtime unavailable.

### Scenarios

1. Verified platform binary download installs without Cargo.
2. Native Windows has an equivalent PowerShell path.
3. Download and verification failures fail closed or enter the explicitly
   documented `auto` Cargo fallback.
4. A `v0.2.0` tag publishes one versioned five-target GitHub Release only after
   complete verification.

### Options

1. Keep compile-on-install.
2. Publish verified binaries with locked source-build fallback.
3. Commit binaries to the Skill.

### Chosen Plan

Option 2. The approved design is
`docs/specs/2026-07-16-binary-release-design.md`.

## Agent Budget

- Main controller performs the coupled design, installer, packaging, CI, test,
  documentation, versioning, and release work serially.
- Maximum concurrent delegated Sessions: 2.
- Maximum total delegated Sessions: 3.
- Delegation is reserved for independent release/security review and required
  re-review. No worker delegation and no nested delegation.
- Exact delegated Token telemetry may be unavailable, so no Session reuse is
  planned.

## Agent Ledger

The durable Run currently tracks:

| Task | Role | State | Evidence |
|---|---|---|---|
| `release-workflow` | main controller | accepted | `094d634` |
| `release-docs` | main controller | accepted | `94bb940` |
| `release-review-publish` | independent reviewer | failed | Session stalled without a report and was closed with an explicit failure reason |
| `release-review-retry` | independent reviewer | accepted | `/tmp/cached-subagent-harness-v0.2.0-security-review.md` |
| `release-review-fixes` | main controller/fixer | running | current batched review-fix diff |

## Write Scope

Main controller: release workflow, installers, packaging and validation tools,
their tests, version metadata, public documentation, design/plan/report files,
and generated Cargo lock metadata.

## Decision Log

- Target version: `0.2.0` / tag `v0.2.0`.
- Distribution: five platform archives plus `SHA256SUMS`.
- Default install: exact-version verified download, then locked Cargo fallback.
- Public claim: long-running Token-aware control; no positive live Token-saving
  claim.
- User authorized autonomous intermediate decisions and creation of the public
  GitHub Release.

## Evidence

- Public GitHub Releases API returned an empty list before this increment.
- The local Linux x86-64 binary is 3.6 MB and intentionally ignored.
- Existing `v0.1.0` is a Git tag without a GitHub Release.
- Existing real A/B results remain negative and constrain product claims.
- Exact-version archives, checksums, Bash acquisition, native PowerShell, and
  the tag-gated workflow are covered by offline contracts.
- The workflow publishes only five explicit archives plus `SHA256SUMS`; it
  does not use a wildcard release upload.
- The independent security review found no Critical issue, five Important
  issues, and two Minor issues. Every finding has one bounded disposition in
  the current review-fix diff.
- Focused RED/GREEN evidence covers tar symlink/hardlink rejection, ZIP
  regular-file metadata, native Windows acquisition scenarios, immutable
  action references, strict SemVer, compatibility claims, and Release
  mutability language.

## Changed Files

Task 1 adds deterministic five-target release packaging, checksum generation,
tag/plugin/Cargo version validation, and version `0.2.0` metadata.
Task 2 adds exact-version Bash download, SHA-256 and archive-member validation,
atomic runtime replacement, locked Cargo fallback, and explicit source modes.
Task 3 adds the native PowerShell installer with the same exact-version,
checksum, archive-member, atomic replacement, source selection, and stale
runtime removal boundaries. Native execution is assigned to the Windows CI gate
in Task 4 because PowerShell is unavailable in the local Linux environment.
Task 4 adds the five-runner build matrix, full verification dependency,
tag-only publication gate, and explicit six-asset release command (`094d634`).
Task 5 adds recommended prebuilt installation, source-policy and unsigned
binary documentation, `v0.2.0` notes, current-state authority links, and public
contract tests.
The review-fix pass rejects link-like archive members, adds native PowerShell
behavior coverage for all acquisition boundaries, pins every Action and Rust
version, narrows compatibility certification to the build/test runners,
removes unenforced immutability claims, declares CI permissions explicitly,
and enforces canonical Semantic Versioning.

## Tests

Implementation plan written at
`docs/plans/2026-07-16-binary-release-plan.md`. Task 1 RED failed on the absent
packager and absent tag gate. GREEN passed 8/8 focused distribution tests,
release metadata validation for `v0.2.0`, Python syntax checks, and locked Cargo
check for `harnessctl 0.2.0`.
- Task 2 RED rejected the absent binary-source interface. Its first GREEN run
  exposed non-portable `awk` interval syntax and a stale ignored runtime copied
  from a developer checkout. Both root causes were fixed without weakening the
  tests. GREEN passed 15/15 install tests, 8/8 distribution tests, Bash syntax,
  and release validation.
- Task 3 RED failed on the absent PowerShell installer. GREEN passed 10/10 local
  release distribution and PowerShell static-contract tests; the dependency-free
  native smoke script is ready for Windows CI.
- Task 4 workflow RED rejected missing matrix/archive facts and wildcard asset
  publication. GREEN passed 12/12 release distribution/workflow tests, YAML
  parsing, tag validation, and the full project harness.
- Task 5 public-contract RED rejected the missing Release contract. GREEN
  passed 20/20 standalone tests. `scripts/verify.sh` then passed on the current
  Task 5 tree: 52 Rust tests and 68 Python tests, Clippy/format/release build,
  both offline benchmark gates, prompt/lifecycle smoke, and all 20 invariants.
- Review fixes followed focused RED/GREEN cycles. The current focused suites
  pass 17/17 Bash installer tests, 12/12 release distribution/workflow tests,
  and 21/21 standalone contract tests.
- Fresh post-fix `scripts/verify.sh` passed 52/52 Rust tests and 71/71 Python
  tests, release metadata validation, formatting, Clippy with warnings denied,
  a release build, both offline benchmark gates, prompt/lifecycle smoke, and
  all 20 invariants. Explicit `--tag v0.2.0` validation, workflow YAML parsing,
  invariant byte-preservation against `9537711`, and `git diff --check` also
  exited 0. Native Windows execution remains an external CI gate.

## Review Findings

The independent release/security review reported 0 Critical, 5 Important, and
2 Minor findings:

- I1, archive links: fixed by rejecting every non-regular tar/ZIP member and
  rechecking extracted paths; symlink and hardlink regressions are covered.
- I2, Windows behavior unproved: fixed locally by expanding the dependency-free
  PowerShell suite across Download, checksum failure, duplicate/missing digest,
  unsafe ZIP type, Auto fallback, Build isolation, forced Download isolation,
  and paths with spaces. Native proof remains mandatory in Windows CI.
- I3, mutable Actions/toolchain: fixed with reviewed full Action SHAs and Rust
  `1.96.1` in normal CI and Release workflows.
- I4, undefined compatibility floor: resolved by narrowing certification to
  Ubuntu 24.04 / glibc 2.39, macOS 15, and the current Windows runner. Older
  systems are not claimed; locked Cargo build remains the fallback.
- I5, unenforced immutability: resolved by removing the false property and
  documenting that tag/Release mutability is governed by repository settings.
- M1 and M2: normal CI now declares `contents: read`; release packaging and
  validation now enforce canonical Semantic Versioning.

Fresh independent re-review of the batched fixes is pending.

## Risks

- Native PowerShell behavior must pass on GitHub's Windows runner before the
  tag is created; local Linux static contracts are not accepted as a substitute.
- GitHub runner-label availability and platform toolchain drift remain external
  publication risks and will be resolved from observed CI rather than assumed.
- Public binaries are checksummed but unsigned in this increment.
- Compatibility is intentionally certified only for the documented runners;
  matching target triples on older systems may require the Cargo build path.

## Next Actions

Commit this locally verified batched fix and obtain a fresh read-only re-review.
Then merge and push source, require normal CI including the native
PowerShell suite to pass, publish tag `v0.2.0`, inspect and execute the public
Linux asset, and close the lifecycle audit.

## External Agent Reconciliation

- `binary-release-review-20260716`: failed and closed after it stalled without
  a report; no result was used.
- `binary-release-review-retry-20260716`: reported the accepted security
  review and is closed.
- UI-visible historical Sessions are outside this Run and do not affect this
  task's Agent budget or closure.

## Degraded Mode Notes

The local Linux environment has no `pwsh`; native PowerShell execution is
therefore an explicit Windows CI gate, not a waived test. Standalone methodology
and the bundled `harnessctl` remain available.

## Final Audit

Pending.
