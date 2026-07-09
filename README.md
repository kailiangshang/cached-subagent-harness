# Cached Subagent Harness 🧭

Cache-aware subagent orchestration for agentic CLIs. This repository packages the
`cached-subagent-harness` skill plus a Rust `harnessctl` tool for stable
dispatch prompts, SQLite lifecycle ledgers, write-scope gates, and final audits.

It is designed to work with [Superpowers](https://github.com/obra/superpowers).
The installer detects Superpowers and installs its skills when they are missing.

## Why This Exists 🔍

Subagents are useful, but unmanaged subagents get expensive and messy quickly:

- each worker receives a large repeated handoff;
- prompt-cache hits become accidental instead of designed;
- lifecycle state lives in chat history instead of a durable report;
- finished agents stay open and keep counting against concurrency limits;
- write scopes are unclear, so parallel work can collide;
- the controller cannot easily audit whether workers actually reported,
  verified, and closed.

This harness treats subagents as a controlled workflow, not just extra chat
tabs. It makes the controller keep a stable prompt contract, pass dynamic
context by path, record lifecycle state in a ledger, and verify gates before
claiming completion.

## Design Principles 🧱

- 🧩 **Problem first**: every worker brief starts with Problem, Scenarios,
  Options, and Chosen Plan before code changes.
- 🪪 **Stable identity, dynamic tail**: reusable rules stay in a stable prompt
  prefix; task-specific paths and values stay after `--- DYNAMIC TASK CONTEXT ---`.
- 📉 **Cache-aware, not magic**: the harness is valuable when repeated dispatches
  can reuse the stable prefix. It is not guaranteed to reduce raw tokens for
  tiny one-agent tasks.
- 🧾 **Durable lifecycle**: every agent has a ledger row with role, status,
  report path, write scope, token risk, next action, and final reason when
  needed.
- 🔐 **Explicit write scope**: worker prompts require `ALLOWED_WRITE_PATHS`;
  discussion, explorer, and reviewer roles remain read-only.
- 🔁 **Loop before drift**: if exploration, tests, or review invalidate the
  plan, update the brief/report and return to the earliest invalid gate.
- ✅ **Complete development**: tests, review, verification, docs, cleanup, and
  final audit are part of the work, not optional follow-up.

## How It Works ⚙️

1. **Controller frames the work** with PSOC: Problem, Scenarios, Options, Chosen
   Plan.
2. **Controller creates durable state**: report path, lifecycle ledger, agent
   budget, and write scopes.
3. **`harnessctl render-prompt` generates dispatch prompts** with a stable
   prefix and a small dynamic tail.
4. **Subagents work inside role gates**:
   - `explorer`: read-only context gathering;
   - `discussion`: product or architecture discussion, read-only;
   - `worker`: bounded writes only inside `ALLOWED_WRITE_PATHS`;
   - `reviewer`: read-only review against brief, report, and diff;
   - `fixer`: one batched fix pass for review findings.
5. **`harnessctl ledger-audit` checks lifecycle state** before budget expansion
   or final completion.
6. **Controller closes superseded agents** and records any failed, abandoned, or
   externally unknown agents with explicit reasons.

## When To Use It 🚦

Use this harness when the task has one or more of these properties:

- multiple subagents or repeated worker dispatches;
- long task briefs that would otherwise be pasted into every worker prompt;
- strict write boundaries across parallel workers;
- lifecycle cleanup matters because open agents consume budget;
- the task needs durable status across resumes or context compaction;
- you want a reviewer/fixer gate before claiming completion.

It is usually not worth it for a single small edit, a one-off question, or a task
where prompt-cache behavior is irrelevant.

## What It Adds 📦

- Stable prompt prefixes with dynamic task context at the tail.
- Problem, Scenarios, Options, Chosen Plan before worker code.
- Read-heavy parallelism and write-heavy serialized gates.
- Explicit `ALLOWED_WRITE_PATHS` for writer roles.
- SQLite-backed agent lifecycle ledgers.
- Final audit enforcement before claiming completion.
- Rust CLI checks for prompt shape, budget, and open agents.

## Install

Recommended install from a checkout:

```bash
git clone https://github.com/kailiangshang/cached-subagent-harness
cd cached-subagent-harness
scripts/install.sh
```

The installer is tested for Linux, macOS, and WSL-style Bash environments for
Codex-compatible skill directories. On native Windows, use WSL for now or
install the skill manually by copying `skills/cached-subagent-harness` into the
runtime skill directory, such as `%USERPROFILE%\.codex\skills` for Codex.

Use a custom Codex home:

```bash
scripts/install.sh --codex-home /path/to/.codex
```

Replace an existing local install:

```bash
scripts/install.sh --force
```

Skip automatic Superpowers handling when you manage it another way:

```bash
scripts/install.sh --skip-superpowers
```

Pin the Superpowers version, branch, or commit used by the installer:

```bash
SUPERPOWERS_REF=v6.0.3 scripts/install.sh
```

After install, restart your CLI runtime so the new skill is loaded.

## Superpowers Dependency

This harness is intentionally small and relies on Superpowers for the broader
development methodology: brainstorming, TDD, planning, code review, and
finishing workflows.

The installer checks:

- `$CODEX_HOME/skills/using-superpowers/SKILL.md`
- `$CODEX_HOME/superpowers/skills/using-superpowers/SKILL.md`
- `$CODEX_HOME/plugins/cache/**/skills/using-superpowers/SKILL.md`

If none are present, it clones `https://github.com/obra/superpowers` into
`$CODEX_HOME/superpowers` and copies its skills into `$CODEX_HOME/skills`
without replacing existing skill directories. Use `SUPERPOWERS_REF` to pin the
branch, tag, or commit. The default is `main`.

See [docs/superpowers.md](docs/superpowers.md) for details.

## Verify

Run the repository verification:

```bash
scripts/verify.sh
```

This validates plugin metadata and skill frontmatter, runs Rust formatting,
tests, optional clippy, builds `harnessctl`, runs the token-effectiveness task,
and runs prompt plus ledger smoke tests.

GitHub Actions runs the same release verification on push and pull request.

## Benchmarks 📊

This repo has two benchmark layers.

The benchmark design intentionally separates three different claims:

- **Raw prompt estimate**: how many prompt bytes are generated before provider
  prompt-cache effects.
- **Cache-adjusted estimate**: the stable harness prefix is counted once, while
  each dynamic tail is counted per dispatch.
- **Runtime observation**: real status and token telemetry from an actual
  agentic CLI run.

Only runtime observations can prove end-to-end savings for a specific model,
CLI, cache policy, and task. The offline benchmarks are regression tests and
planning signals.

### Prompt-shape Regression

```bash
scripts/build-harnessctl.sh
python3 scripts/token_effectiveness_task.py --format markdown
```

This low-cost CI task compares a baseline embedded handoff against the cached
harness handoff for repeated worker dispatches. The representative task is a
feedback-agent / inspection-platform refactor brief with PSOC, read-only source
constraints, future workflow needs, and explicit write scopes.

The estimator is a deterministic `bytes/4` proxy. It is meant to prove prompt
shape and regressions in CI; it is not provider billing telemetry. Raw prompt
size is informational because a stronger stable prefix can make a compact
single prompt larger while improving cache-adjusted cost.

Current checked-in fixture result with 4 worker dispatches:

| Metric | Baseline embedded handoff | Cached harness handoff |
|---|---:|---:|
| Estimated tokens total | 1784 | 2164 |
| Cache-adjusted estimated tokens | n/a | 856 |
| Stable prefix ratio | n/a | 80.59% |
| Repeated cacheable tokens | n/a | 1308 |

Raw estimated savings is `-21.3%` because the stable safety prefix is larger.
Cache-adjusted estimated savings is `52.02%`, which is the CI gate that matters.

This result does not prove unconditional token savings. It proves that repeated
worker dispatches keep reusable harness rules cacheable and dynamic tails small.

See [docs/token-effectiveness-task.md](docs/token-effectiveness-task.md) for the
task fixture, comparison method, and limits.

### Game-development A/B Protocol

```bash
scripts/build-harnessctl.sh
python3 scripts/game_dev_ab_benchmark.py --format markdown
```

This stronger benchmark generates equivalent worker prompts for a small browser
game development task in two modes:

- baseline: each worker receives a self-contained embedded handoff;
- cached harness: each worker receives the stable harness prefix plus dynamic
  paths to a shared brief and lifecycle ledger.

Latest local offline estimate with 4 workers:

| Metric | Baseline embedded handoff | Cached harness handoff |
|---|---:|---:|
| Estimated tokens total | 2892 | 2135 |
| Cache-adjusted estimated tokens | 2892 | 827 |
| Stable prefix ratio | n/a | 81.69% |

Raw estimated savings is `26.18%`; cache-adjusted estimated savings is `71.4%`.

The key difference from the prompt-shape fixture is that the game workload has a
larger realistic brief and four independent worker slices. That makes it closer
to the kind of task where subagent orchestration is actually useful.

Generate artifacts for a real A/B run:

```bash
python3 scripts/game_dev_ab_benchmark.py \
  --output-dir /tmp/game-dev-ab \
  --output /tmp/game-dev-ab/report.json \
  --format json
```

The generated observation template can ingest real status and token telemetry
from two actual agent runs. Without observations, the report marks runtime
status as `not-observed`.

See [docs/game-dev-ab-benchmark.md](docs/game-dev-ab-benchmark.md) for the
status schema, quality gates, and interpretation.

## Rust Tool

Build only the Rust harness binary:

```bash
scripts/build-harnessctl.sh
```

The runtime binary is written to:

```text
skills/cached-subagent-harness/scripts/bin/harnessctl
```

The binary is not committed because it is platform-specific. The source lives in:

```text
skills/cached-subagent-harness/scripts/harnessctl
```

For public releases, prefer building `harnessctl` from source with Cargo on the
target machine. Prebuilt binaries should be published as a release matrix rather
than committed into the skill directory.

## Usage

Invoke the skill in a supported CLI runtime:

```text
Use cached-subagent-harness to coordinate this long-running development task.
```

For direct CLI checks:

```bash
skills/cached-subagent-harness/scripts/bin/harnessctl --help
```

## License

MIT
