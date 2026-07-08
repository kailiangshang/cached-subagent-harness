# Superpowers Integration

`cached-subagent-harness` is a control-plane skill for subagent orchestration.
It does not replace Superpowers. It narrows and enforces the parts that become
fragile during long-running, multi-agent Codex work:

- stable prompt prefixes;
- file-path handoffs instead of pasted bulk context;
- explicit writer scopes;
- SQLite lifecycle ledgers;
- completion gates and final audits.

Superpowers remains the recommended source for the broader development process:

- brainstorming;
- writing plans;
- using worktrees;
- test-driven development;
- subagent-driven development;
- requesting and receiving code review;
- verification before completion;
- finishing a development branch.

## Install Behavior

The installer first checks for Superpowers in common Codex locations:

```text
$CODEX_HOME/skills/using-superpowers/SKILL.md
$CODEX_HOME/superpowers/skills/using-superpowers/SKILL.md
$CODEX_HOME/plugins/cache/**/skills/using-superpowers/SKILL.md
```

When Superpowers is missing, the installer clones:

```text
https://github.com/obra/superpowers
```

into:

```text
$CODEX_HOME/superpowers
```

The clone uses `SUPERPOWERS_REF` when set:

```bash
SUPERPOWERS_REF=v6.0.3 scripts/install.sh
```

`SUPERPOWERS_REF` may be a branch, tag, or commit. The default is `main`.

Then it copies Superpowers skill directories into:

```text
$CODEX_HOME/skills
```

Existing skill directories are preserved.

## Degraded Mode

If Superpowers is unavailable, `cached-subagent-harness` still works under its
local minimum contract:

- define Problem, Scenarios, Options, and Chosen Plan;
- use read-heavy parallelism and write-heavy serialization;
- keep prompt prefixes stable;
- keep a ledger of harness-created agents;
- verify focused behavior and project harnesses;
- run final lifecycle audit before completion.

Record degraded mode in the task report whenever Superpowers is unavailable or
intentionally skipped.
