# Codex Cached Subagent Harness

Cache-aware subagent orchestration for Codex. This repository packages the
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
git clone https://github.com/shangkailiang/codex-cached-subagent-harness
cd codex-cached-subagent-harness
scripts/install.sh
```

The installer is tested for Linux, macOS, and WSL-style Bash environments. On
native Windows, use WSL for now or install the skill manually by copying
`skills/cached-subagent-harness` into `%USERPROFILE%\.codex\skills`.

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

After install, restart Codex so the new skill is loaded.

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
tests, optional clippy, builds `harnessctl`, and runs prompt plus ledger smoke
tests.

GitHub Actions runs the same release verification on push and pull request.

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

## Usage

Invoke the skill in Codex:

```text
Use cached-subagent-harness to coordinate this long-running development task.
```

For direct CLI checks:

```bash
skills/cached-subagent-harness/scripts/bin/harnessctl --help
```

## License

MIT
