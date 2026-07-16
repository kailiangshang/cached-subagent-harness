# Binary Release Implementation

Status: implementation — Bash verified installation complete

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
4. A `v0.2.0` tag publishes one immutable five-target GitHub Release only after
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

No delegated Session has been planned or spawned yet.

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

## Changed Files

Task 1 adds deterministic five-target release packaging, checksum generation,
tag/plugin/Cargo version validation, and version `0.2.0` metadata.
Task 2 adds exact-version Bash download, SHA-256 and archive-member validation,
atomic runtime replacement, locked Cargo fallback, and explicit source modes.

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

## Review Findings

Pending.

## Risks

- Cross-platform runner or toolchain drift.
- Installer quoting, archive traversal, or checksum-selection defects.
- Release workflow could publish partial or version-mismatched assets without
  explicit aggregation gates.
- Public binaries are checksummed but unsigned in this increment.

## Next Actions

Implement test-first, verify, independently review, tag `v0.2.0`, inspect the
public Release, and close the lifecycle audit.

## External Agent Reconciliation

No in-scope delegated Session yet.

## Degraded Mode Notes

None. Standalone methodology is the normal mode.

## Final Audit

Pending.
