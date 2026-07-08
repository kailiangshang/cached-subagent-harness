# Cached Subagent Harness

Cache-aware subagent orchestration for agentic CLIs. This repository packages the
`cached-subagent-harness` skill plus a Rust `harnessctl` tool for stable
dispatch prompts, SQLite lifecycle ledgers, write-scope gates, and final audits.

It is designed to work with [Superpowers](https://github.com/obra/superpowers).
The installer detects Superpowers and installs its skills when they are missing.

## What It Adds

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

## Token Effectiveness Task

Run the offline task directly after building `harnessctl`:

```bash
scripts/build-harnessctl.sh
python3 scripts/token_effectiveness_task.py --format markdown
```

The task compares a baseline embedded handoff against the cached harness handoff
for repeated worker dispatches. The representative task is a feedback-agent /
inspection-platform refactor brief with PSOC, read-only source constraints,
future workflow needs, and explicit write scopes.

The estimator is a deterministic `bytes/4` proxy. It is meant to prove prompt
shape and regressions in CI; it is not provider billing telemetry. Raw prompt
size is informational by default because a stronger stable prefix can make a
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

See [docs/token-effectiveness-task.md](docs/token-effectiveness-task.md) for the
task fixture, comparison method, and interpretation.

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
