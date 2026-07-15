# Current Product State

Date: 2026-07-15

Status: lightweight runtime and results Dashboard complete; corrected Token
policy verified by deterministic gates but not yet proven positive by a new
live A/B run

This is the shortest current-state entry point for `cached-subagent-harness`.
It summarizes the implemented product and points to the binding contracts and
retained evidence. Historical plans remain in the repository for audit, but
their status banners determine whether they still govern implementation.

## Product Priority

The Harness optimizes one objective:

> Minimize total effective Token use while preserving complete development and
> review quality.

The Dashboard is mandatory because users need to see work, Sessions, quality,
and cost. It is still a supporting read-only view, not a second controller and
not the primary product objective.

The 20 numbered invariants in
[`SKILL.md`](../skills/cached-subagent-harness/SKILL.md) remain the constitution.
They preserve PSOC, bounded writes, durable lifecycle state, independent
review, complete-development gates, truthful unknowns, stable prompts, safe
routing, and final audit.

## Implemented Architecture

The product has three compact layers:

1. **Skill policy** defines controller behavior, role gates, prompt discipline,
   Token strategy, and completion requirements.
2. **`harnessctl`** stores current Run, Task, Session, usage, and activity state
   in SQLite. Focused Rust modules handle bundling, routing, Session decisions,
   host command templates, accounting, status, and the embedded Web server.
3. **Presentation** exposes terminal status, JSON, watch mode, and one
   bilingual single-Run Dashboard from the same limited status projection.

Current-state tables are authoritative. The small activity feed is useful for
display and debugging but is never replayed to reconstruct state. There is no
event-sourced platform, capability scanner, desktop bridge, permanent observer,
Node service, or frontend framework.

Standalone is normal operation. Superpowers is an explicitly optional,
phase-lazy methodology integration; its absence is not degraded mode.

## Token Strategy

The controller applies this order:

```text
trivial work with no isolation need              -> execute on main
multiple compatible queued Tasks                -> one bounded batch
one later compatible Task inside both budgets   -> reuse a Session
delegation benefit exceeds complete cost         -> spawn a Session
otherwise                                        -> execute on main
```

Compatibility includes role, required profile, risk, package, write scope,
repository revision, dependency order, and review boundary. The ready set is
derived from durable queued Tasks rather than a caller-provided count.

Known compatible work is batched before Session reuse. A reusable Session
defaults to at most one accepted follow-up and 200,000 total effective Tokens.
Runtime flags may lower these values but cannot raise them. Reuse additionally
requires:

- an exact compatibility signature and atomic `idle` to `busy` claim;
- durable acceptance of the current follow-up;
- complete exact normalized usage linked to the same Run, Task, and Session;
- usage strictly after the acceptance transaction's causal boundary;
- remaining follow-up and effective-Token budgets.

Unknown, partial, stale, non-normalizable, or mismatched usage ends the reuse
path. Busy Sessions own exactly one current Task; idle and terminal Sessions
own none.

Model routing is quality-constrained. The required profile is the maximum of
role, risk, complexity, and uncertainty floors. `light` serves bounded
read-only or formatting work, `standard` serves scoped implementation and
ordinary analysis, and `deep` serves architecture, ambiguous multi-step work,
control-plane changes, security-sensitive work, and high-risk review. Cost is
optimized only after those floors are fixed.

Accounting includes bootstrap, context, work, retry, escalation, review, and
fixer phases. Missing values remain unknown. Estimates disclose method,
eligible sample count, and quality; cross-provider monetary savings remain
unsupported without explicit compatible price data.

## Run, Task, and Session

| Object | Meaning | Important boundary |
|---|---|---|
| Run | One Harness-controlled goal and final-audit scope | Owns all Tasks, Sessions, usage, and activity for that effort |
| Task | One durable unit of work | Has status, compatibility facts, assignment, and acceptance; detailed evidence lives in the Run-level external report |
| Session | One resumable host CLI/model context represented in Harness state | May carry compatible Tasks sequentially; never carries two current Tasks |

Session does not mean account login or browser authentication. It is also not
synonymous with Task: a Task is work, while a Session is the host context that
may perform that work. A Session visible in a host UI is not automatically open
or reusable in Harness state.

## Host Boundary

Bundled command templates cover Codex, Claude Code, and OpenCode. They render
native argument arrays for spawn and supported follow-up/close operations.
`harnessctl` does not run those arrays through a shell or claim an observed
result from a requested command; the controller invokes the host and records
actual behavior separately.

Compatible runtimes such as desktop agents can be added with a JSON template
when they expose equivalent Skill and agent/session commands. This needs no
scanner, bridge, or adapter framework. A custom template proves configuration
compatibility, not live certification. The bundled installer currently targets
a Codex-compatible Skill directory; other runtimes use their own discovery
path.

## Dashboard Boundary

The embedded Dashboard is a dense Moonlight Indigo liquid-glass results view.
It supports zh-CN and en-US, larger operational type, responsive layouts,
reduced motion/transparency, and loopback binding by default.

It shows only Harness facts for one Run:

- progress and factual Run freshness;
- Task states, package grouping, current work, and latest activity;
- Session host/profile/model facts and ordered assignment chains;
- exact or explicitly qualified Token totals and phase composition;
- reuse, churn, assignments per spawn, estimate method, sample count, and
  quality.

The limited projection structurally excludes `repo_root`, `report_path`,
`write_scope`, Host handles, and task-internal next actions. Run goals, Task
titles, and activity summaries are caller-provided display text and are not
sanitized. Controllers must keep prompts, secrets, sensitive paths, source
content, and long logs out of those fields.

The Dashboard never contains Baseline columns, A/B controls, benchmark
branding, observer guesses, or billing claims. The embedded server has no
authentication or TLS. Keep the default loopback binding; a non-loopback bind
requires explicit `--allow-remote true` and is suitable only behind a trusted,
access-controlled network or tunnel.

## Evidence and Claim Boundary

Offline fixtures verify prompt shape and cacheability but do not prove provider
billing savings. The real 2026-07-15 Signal Sweep experiment compared
equal-quality Codex CLI runs with exact telemetry:

| Metric | Baseline | Rejected repeated-follow-up arm |
|---|---:|---:|
| Total effective Tokens | 2,974,064 | 17,551,878 |
| Relative cost | 1.00x | 5.90x |
| Saving | n/a | -490.16% |

The negative result rejected unlimited compatible continuation: accumulated
resumed context dominated despite high cache reads. It drove the current
batch-first, one-follow-up, 200,000-Token policy and strict causal usage gate.

The release may claim that the measured unbounded path is prevented. It may not
claim that the corrected policy has proven positive live savings until a
separate equal-quality real A/B run measures that policy.

## Verification State

The completed delivery passed:

- Rust tests: 50/50;
- Python tests: 33/33 across the release and Benchmark suites;
- Clippy with warnings denied and a release build;
- release metadata and Skill validation;
- prompt-cache, offline Token, and game A/B regression gates;
- exact real-ledger status and final lifecycle audit;
- two independent final reviews with zero open Critical, Important, or Minor
  findings.

See [`results-dashboard-implementation.md`](../results-dashboard-implementation.md)
for the complete implementation and review trail.

## Document Authority

| Document | Current role |
|---|---|
| [`SKILL.md`](../skills/cached-subagent-harness/SKILL.md) | Binding controller constitution and workflow |
| [`2026-07-14 lightweight design`](specs/2026-07-14-lightweight-token-harness-design.md) | Canonical lightweight architecture, amended by live evidence |
| [`2026-07-15 Dashboard design`](specs/2026-07-15-results-dashboard-design.md) | Implemented presentation and validation boundary |
| [`Signal Sweep evidence`](benchmarks/2026-07-15-signal-sweep-real-ab.md) | Exact historical RED result and claim limit |
| [`results-dashboard-implementation.md`](../results-dashboard-implementation.md) | Completed delivery, tests, fixes, and final audit |
| 2026-07-10/12 umbrella and event-runtime documents | Historical/superseded evidence only |
| Implementation plans | Historical execution records; status banners identify completion or supersession |

When a historical plan conflicts with the current Skill, lightweight designs,
runtime gates, or retained live evidence, the current contract wins.
