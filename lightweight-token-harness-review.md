# Lightweight Token Harness Release Review

Reviewed `d16a993` against the approved design and implementation plan. This was a strict read-only, timeboxed review; no product files were changed and no full suite was rerun.

## Strengths

- The idle-session claim uses `BEGIN IMMEDIATE` and conditionally changes the selected row from `idle` to `busy` in the same transaction as task linkage, preventing two connections from successfully claiming the same row.
- Nullable token fields remain nullable through SQLite, Rust, JSON, terminal output, and the page; missing telemetry is not rendered as zero.
- The active Skill still explicitly preserves all 20 numbered invariants, including PSOC, test-first changes, serialized overlapping writes, independent review/fix gates, stable prompt tails, truthful lifecycle closure, requested-versus-actual facts, and honest unknowns (`skills/cached-subagent-harness/SKILL.md:19`).
- Host commands are rendered as argument arrays without shell execution, and the dashboard defaults to loopback, uses `textContent`, and sends CSP/nosniff/referrer/no-store protections.

## Critical

None found in the timeboxed review.

## Important

1. **Atomic reuse and routing trust caller-supplied compatibility facts instead of the authoritative queued task.** `cmd_decide` reconstructs role, risk, complexity, uncertainty, package, scope hash, revision, and review boundary from flags (`skills/cached-subagent-harness/scripts/harnessctl/src/main.rs:550`), while `claim_idle_session` matches the resulting signature only against the session and checks the task merely for id/run/queued status (`skills/cached-subagent-harness/scripts/harnessctl/src/store.rs:455`, especially `:502`). A caller can describe a stored deep reviewer task as light explorer work and atomically assign it to a matching but incompatible session. `task add` also accepts a caller-provided `required_profile` without recomputing its safety floor (`src/main.rs:473`; `src/store.rs:788`). **Fix:** load the task inside the same immediate transaction, derive role/profile/package/scope/revision/review boundary from that row, recompute and validate its route floor, then join/compare those authoritative fields before changing either row. Leave only host choice, manual elevation, and non-authoritative dispatch hints as CLI inputs; add negative CLI tests for disagreement on every dimension.

2. **Reuse and token-efficiency numbers can be inflated and do not implement the specified identities.** Every retry of `accept-followup` while a session remains busy increments `reuse_count` again (`skills/cached-subagent-harness/scripts/harnessctl/src/store.rs:515`). Accounting then assumes every session row completed one accepted assignment and computes `(sessions + reuse_count) / sessions`, rather than accepted delegated tasks / spawned sessions (`skills/cached-subagent-harness/scripts/harnessctl/src/accounting.rs:34`). It also pools overhead samples across all hosts/profiles and multiplies the global median by all reuse (`src/accounting.rs:50`), contrary to the same-host/profile threshold, and does not gate the claim on accepted quality outcomes. **Fix:** persist a unique assignment/acceptance identity and make acceptance compare-and-set/idempotent; derive both ratios from authoritative accepted/completed task assignments; group exact overhead by `(host, profile)` and apply each group only to its accepted reuses, leaving under-three-sample or unequal-quality groups unknown. Add duplicate-retry, failed/unaccepted session, mixed-host/profile, and quality-gate fixtures.

3. **Two mandatory user-facing release contracts are absent: custom templates and complete/safe dashboard projection.** `host-command` always loads only the compile-time bundled map (`skills/cached-subagent-harness/scripts/harnessctl/src/hosts.rs:4`; `src/main.rs:713`), so a custom compatible JSON host cannot work without changing/rebuilding the product. The Agents panel renders neither requested/actual model nor current task nor last activity (`skills/cached-subagent-harness/scripts/harnessctl/assets/app.js:18`); `SessionRecord` does not even project `last_used_at` (`src/domain.rs:168`; `src/store.rs:641`). Token Economy omits churn and estimate method/sample/quality (`assets/app.js:24`). Meanwhile `/api/status` exposes the raw run/task/session records—including internal paths, write scopes, next actions, and host handles—although most are not displayed, unnecessarily widening dashboard disclosure (`src/status.rs:5`; `src/dashboard.rs:87`). **Fix:** add an explicit `--templates FILE` load/validate/merge path with a non-bundled CLI test; define a public-safe `StatusView` DTO shared by CLI JSON and dashboard that includes the required current task, last activity, requested/actual model, churn, and estimate disclosure but excludes unused internal paths/scopes/handles; test exact API parity and absence of sensitive fields.

## Minor

Omitted under the hard-stop instruction.

## Assessment

**Ready: No — Important fixes required.** The implementation has a sound compact base and a genuinely atomic row claim, but the authority boundary permits incompatible routing/reuse, the core efficiency claim is not trustworthy, and mandatory custom-host/dashboard contracts are not delivered. Re-review these fixes before release.

## Fix Pass

Fix status: all three Important findings addressed in one controller pass.

- Authoritative task fields now drive CLI routing, task profile floors are
  validated, and the atomic claim rechecks task/session compatibility inside
  the same immediate transaction.
- Follow-up acceptance is idempotent; efficiency ratios use accepted delegated
  tasks; savings samples are per-session and grouped by host/profile, and only
  accepted reuse contributes to estimates.
- Custom template files merge by host through `--templates`; the shared public
  status DTO omits repo paths, write scopes, reports, and handles while exposing
  current task, requested/actual model, last activity, churn, and estimate
  sample/quality details.

Covering evidence: 34 Rust tests pass, including malicious task relabeling,
profile-floor rejection, duplicate acceptance, six-task/one-spawn/five-followup,
mixed-host estimate rejection, unaccepted reuse rejection, custom host loading,
safe status JSON, and dashboard field contracts. Clippy passes with warnings
denied. Final repository verification is rerun after this fix commit.
