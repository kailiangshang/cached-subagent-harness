# Subagent Session Token Strategy Implementation

Status: in progress — attribution fix verified; final narrow re-review pending

## PSOC

### Problem

The runtime still has Subagents, but public documentation and the Dashboard
mainly expose `Session`. Users can mistake Session for account login or conclude
that the Subagent concept disappeared. The complete Token routing strategy is
distributed across prose and is not visible at the point where current results
are inspected.

### Scenarios

1. A new user can distinguish Run, Task, Subagent, and Session without reading
   implementation code.
2. A Dashboard user sees which Subagent Session is doing which Task and knows
   that the card is a host/model context rather than an authentication session.
3. A user can see the release's Token decision order while clearly separating
   static policy from the latest observed route and current run data.
4. Compact zh-CN/en-US layouts remain readable, truthful, and free of Baseline
   comparison UI or sensitive controller fields.

### Options

1. Add a separate Subagent data model and UI: explicit, but duplicates Session
   lifecycle and expands the control plane.
2. Rename Session to Subagent everywhere: simple, but hides the concrete host
   context and makes bounded reuse hard to explain.
3. Keep Session as durable truth, define Subagent as the logical executor, and
   label the presentation `Subagent sessions`: smallest consistent change.

### Chosen Plan

Use option 3. Add a four-term contract and Token flow to the Skill and public
docs, then add a dependency-free bilingual policy map and Subagent Session note
to the existing Dashboard. Do not change routing, persistence, API shape, or
release limits.

## Agent Budget

- Maximum open delegated Sessions: 2.
- Maximum total spawned Sessions: 4.
- Planned use: one fresh read-only baseline comprehension Session, one fresh
  read-only final review Session, one fresh read-only whole-diff re-review
  Session, and one narrowly scoped final attribution-review Session. The third
  and fourth Sessions are mandatory re-reviews after Important findings; exact
  Token telemetry is unavailable, so invariant 18 forbids Session reuse.
  Implementation remains on main to avoid short-lived writer churn.
- Nested delegation: disabled.

## Agent Ledger

| handle | role | task | status | report_path | spawned_at | waited | closed | write_scope | token_risk | session_budget | final_reason | next_action |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| /root/baseline_term_clarity | discussion | baseline-term-clarity | closed | collaboration final response | 2026-07-16 | yes | yes | none | low | no reuse | Baseline explanation consumed | Implement the terminology contract |
| /root/subagent_session_final_review | reviewer | final-whole-diff-review | closed | collaboration final response | 2026-07-16 | yes | yes | none | medium | no reuse | Four Important and two Minor findings consumed | Apply one bounded fix pass, verify, then re-review |
| /root/subagent_session_rereview | reviewer | final-whole-diff-rereview | closed | collaboration final response | 2026-07-16 | yes | yes | none | medium | no reuse | One Important stale-attribution finding consumed | Fix with a focused contract, then run narrow re-review |
| /root/subagent_session_attribution_review | reviewer | final-attribution-review | planned | pending | pending | no | no | none | low | no reuse; close after verdict | pending | Verify only the attribution fix and release-document consistency |

## Write Scope

- Main controller: approved files listed in the implementation plan.
- Baseline and final review Sessions: `none`.

## Decision Log

- 2026-07-16: User approved the four-term model, dependency-free Dashboard map,
  and autonomous detail execution.
- 2026-07-16: Continue on clean `main` under the user's direct-development and
  push authorization; do not create a second worktree.
- 2026-07-16: Do not install or overwrite the user's active Skill.

## Routing and Batch Policy

- Baseline comprehension is an isolated read-only discussion assignment with a
  light floor; it does not share an implementation batch.
- Main performs the tightly coupled documentation/UI edits serially.
- Final review is independent, read-only, and starts only after verification.
- Release defaults remain two Tasks per compatible micro-batch, one accepted
  follow-up, and 200,000 effective Tokens; no increase is proposed.

## Evidence

- User feedback: the current presentation made it appear that the Subagent
  concept may have disappeared.
- Existing README/current-state define Run, Task, and Session, but do not define
  Subagent as a separate logical concept or show the complete decision flow at
  the Dashboard.
- Fresh-context baseline result: the reader could infer that Subagents still
  exist from role language, but found no formal Subagent-to-Session cardinality,
  noted that the Web UI reports all Session records rather than live Subagent
  count, and found CLI/Web terminology mixed. This reproduces the user's
  confusion without exposing the intended fix to the evaluator.
- Approved design:
  `docs/specs/2026-07-16-subagent-session-token-strategy-design.md`.
- Implementation plan:
  `docs/plans/2026-07-16-subagent-session-token-strategy-plan.md`.

## Changed Files

- `skills/cached-subagent-harness/SKILL.md`: four-term execution model and
  explicit Token decision order; all 20 invariants preserved.
- `README.md`: public Subagent/Session contract and Mermaid decision flow.
- `docs/current-state.md`: authoritative terminology, Mermaid flow, and
  Dashboard policy/live-state boundary.
- `docs/specs/2026-07-15-results-dashboard-design.md`: six-region amendment for
  the static policy map and Subagent Session dock.
- `scripts/test_standalone_contract.py` and `scripts/validate-release.py`:
  RED/GREEN documentation, policy-order, locale, typography, and invariant
  boundary contracts.
- Dashboard `index.html`, `app.js`, `styles.css`, and Rust asset tests: bilingual
  semantic policy map and responsive Subagent Session presentation.
- Design, plan, and this report record the approved scope and evidence.

## Tests

- Clean pre-change baseline: `scripts/verify.sh` passed with Rust 52/52 and
  Python 38/38.
- Documentation RED: two focused tests failed on the missing Subagent mapping,
  account-login distinction, Mermaid flow, and Dashboard design terminology.
- Documentation GREEN: both focused tests passed; the full standalone contract
  passed 12/12 and release validation passed.
- Dashboard RED: the focused Rust test failed on missing
  `data-view="strategy-map"`.
- Dashboard GREEN: Rust Dashboard tests passed 2/2; JavaScript syntax, Rust
  formatting, Python standalone contracts 12/12, and `git diff --check` passed.
- Visual audit: populated zh-CN/en-US 1440×960 and exact 390×844 plus extended
  compact captures inspected. The policy map is explicitly static, both locales
  fit, the compact flow stacks without horizontal clipping, live Task/Session
  surfaces remain primary, and the Session note is readable. Artifacts live
  only under ignored `target/visual-audit/`.
- Review-fix RED: the new focused suite exposed the stale invariant boundary,
  late delegation-value gate, missing no-value Dashboard branch, and three
  11px explanatory rules; the pre-existing locale keys were already equal.
- Review-fix GREEN: all six focused contracts passed, including exact zh-CN /
  en-US key parity and policy-order assertions.
- Attribution RED/GREEN: a focused current-state contract failed on the stale
  Signal Sweep evidence attribution, then passed after the current increment
  was pointed only to its own implementation report.
- The complete Python suite now contains 47 tests (install 7, standalone 19,
  Token effectiveness 3, game A/B 18); the final full verification will rerun
  it after review closure.
- Full post-fix `scripts/verify.sh` passed before the attribution contract: Rust
  52/52; Python 46/46 (install 7, standalone 18, Token effectiveness 3, game A/B
  18); release
  metadata, Rust formatting, Clippy, release build, both offline Benchmark
  thresholds, prompt-cache check, lifecycle smoke, and audit passed.
- The system Skill package validator reported `Skill is valid!` without copying
  or installing the Skill.
- Post-fix visual audit: zh-CN/en-US at 1440×960 and constrained 390×844 have
  no horizontal overflow or clipped strategy copy; the five-card map remains
  static policy, live Task/Session surfaces remain primary, and 12px body notes
  remain readable. Temporary preview and WebDriver processes were closed; the
  user's existing 7347 preview was untouched.

## Review Findings

The first independent whole-diff review found no Critical issue, four Important
issues, and two Minor issues:

1. The Skill checked delegation value after batching/routing rather than before.
2. The Dashboard omitted the no-net-value path back to main and could imply
   every non-reusable Session should spawn a replacement.
3. Test and release-validator invariant extraction included the new terminology
   section because the end marker was stale.
4. Current-state, authority links, implementation status, and Task 3 closure
   evidence were stale.
5. Strategy and Session explanation text used 11px instead of the planned 12px.
6. Locale parity and decision-order/branch semantics lacked automated contracts.

Items 1–6 are fixed; behavioral and boundary changes are covered by RED/GREEN
tests, and the authoritative documents now record the current increment. A
fresh independent re-review is mandatory before release closure.

The fresh whole-diff re-review verified the invariant hash, decision order,
terminology, bilingual policy branch, responsive artifacts, privacy boundary,
and absence of scope expansion. It found no Critical or Minor issue and one
Important issue: a historical paragraph in `docs/current-state.md` still
attributed the current increment's final evidence to
`corrected-signal-sweep-implementation.md`. The attribution is fixed under a
focused RED/GREEN contract; a final narrow re-review remains required.

## Risks

- Static policy could be mistaken for live progress; label it explicitly.
- Extra explanatory UI could reduce density; keep one compact map and one note.
- Terminology could diverge between locales; enforce exact asset markers.

## Next Actions

1. Run the final narrow independent attribution re-review.
2. Close the lifecycle ledger, finalize authoritative verification state, and
   push `origin/main`.

## External Agent Reconciliation

Only Sessions created for this increment count toward this report. Previously
completed review threads are outside this Run and require no cleanup.

## Degraded Mode Notes

None.

## Final Audit

Pending.
