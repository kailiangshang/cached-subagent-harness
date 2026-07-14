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

## Re-review

Fix range reviewed: `d16a993..95853f8`. No full suite was rerun; `git diff --check d16a993..95853f8` passed.

1. **Authoritative routing and atomic reuse — Closed.** `cmd_decide` now loads the stored task and derives its route demand/signature from that record (`skills/cached-subagent-harness/scripts/harnessctl/src/main.rs:550`). `add_task` rejects a `required_profile` below the computed safety floor (`src/store.rs:843`), and `claim_idle_session` re-reads and compares the authoritative task fields inside the same immediate transaction before selecting or mutating a session (`src/store.rs:457`). Focused tests cover malicious relabeling and the six-task reuse path (`src/sessions.rs:219`, `:307`).

2. **Idempotent accepted reuse and truthful accounting — Open.** The ratios and host/profile grouping are materially corrected, and tests now cover mixed hosts and unaccepted reuse (`src/accounting.rs:43`, `:59`, `:317`). However, duplicate acceptance is detected by querying the informational `activity` feed (`src/store.rs:566`), even though the approved architecture says deleting activity must not change authoritative current state. Deleting/pruning that feed makes the same `(session, task)` acceptable again and inflates `reuse_count`; the new duplicate test does not exercise that contract. **Required fix:** persist acceptance identity in authoritative state (for example a unique assignment/acceptance row or task acceptance marker with a uniqueness constraint) and make the increment a single compare-and-set transaction independent of `activity`; add a regression test that removes activity before retrying acceptance. Also report the sample count actually used by qualifying groups, rather than summing samples from all groups (`src/accounting.rs:84`).

3. **Custom templates and complete/safe dashboard projection — Open.** Runtime JSON templates now merge through `--templates`, and the public status DTO removes internal repo/report/scope/handle fields (`src/hosts.rs:10`; `src/main.rs:718`; `src/status.rs:24`). Current task, last activity, actual model, churn, estimate sample count, and estimate quality reach the page. But the Agents renderer still displays only `actual_model`, not requested and actual as separate facts (`assets/app.js:19`), and Token Economy still omits the required estimate method (`assets/app.js:24`). The added static assertions check `actual_model` and sample count only (`src/dashboard.rs:182`), so they do not catch either omission. **Required fix:** label/render requested and actual models separately, render the median-per-host/profile method, and assert those exact fields/labels in the dashboard contract test.

**Final assessment: Ready No.** Finding 1 is closed; Findings 2 and 3 retain Important correctness/acceptance gaps and require another focused fix and re-review.

## Second Fix Pass

- Reuse acceptance identity now lives in authoritative task state
  (`reuse_accepted`) and is updated by compare-and-set in the same immediate
  transaction; deleting the informational activity feed no longer changes
  idempotency. The regression test deletes all activity before retrying.
- Estimate sample count now covers only host/profile groups that have accepted
  reuse, while the three-sample threshold still controls whether a saving is
  emitted.
- Agent rows label requested and actual models separately. Token Economy now
  shows the `median overhead · host/profile` method, sample count, and quality;
  static dashboard tests require those fields.

Focused evidence: 34/34 Rust tests and Clippy with warnings denied pass after
the second fix pass.

## Final Re-review

Tiny fix range reviewed: `95853f8..0c39d8f`. No full tests were run; `git diff --check 95853f8..0c39d8f` passed.

1. **Authoritative reuse idempotency and relevant estimate sample count — Open.** Reuse idempotency is now **closed**: `reuse_accepted` is authoritative task state, acceptance uses a compare-and-set inside the immediate transaction, and the regression deletes all activity before retrying (`skills/cached-subagent-harness/scripts/harnessctl/src/store.rs:43`, `:551`; `src/sessions.rs:275`). However, sample-count relevance is not fully closed. `estimate_sample_count` is incremented before the three-sample eligibility check (`src/accounting.rs:110`), so an accepted-reuse group with one or two samples is included even though that group contributes nothing to `estimated_saved_tokens`. When another group qualifies, the UI can therefore report more samples than the estimate actually used. **Required fix:** increment the displayed count only after `samples.len() >= 3` (or expose separate eligible/observed counts with explicit labels) and add a fixture containing one qualifying group plus one under-threshold accepted-reuse group.

2. **Requested/actual model and estimate-method dashboard disclosure — Closed.** Agent rows now label and render requested and actual models separately in both languages, and Token Economy renders `median overhead · host/profile` as the estimate method (`skills/cached-subagent-harness/scripts/harnessctl/assets/app.js:2`, `:19`, `:29`). The dashboard test now requires both `requested_model` and the median method in the served asset (`src/dashboard.rs:182`).

**Final assessment: Ready No.** The idempotency and dashboard gaps are closed, but the estimate sample count remains materially misleading for mixed qualifying/under-threshold groups.

## Final Resolution

The remaining sample-count finding is fixed: only groups that both have
accepted reuse and meet the three-exact-sample threshold contribute to
`estimate_sample_count`. A mixed fixture with one qualifying group and one
under-threshold accepted-reuse group now reports only the three samples actually
used by the median estimate. Focused accounting tests (5/5) and Clippy pass.
