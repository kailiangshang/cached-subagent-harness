#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
skill_dir="$repo_root/skills/cached-subagent-harness"
crate_dir="$skill_dir/scripts/harnessctl"
bin="$skill_dir/scripts/bin/harnessctl"
tmp_dir="$(mktemp -d)"

cleanup() {
  rm -rf "$tmp_dir"
  if [ -d "$crate_dir/target" ]; then
    cargo clean --manifest-path "$crate_dir/Cargo.toml" >/dev/null
  fi
}
trap cleanup EXIT

python3 "$repo_root/scripts/validate-release.py" "$repo_root"
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  "$repo_root/scripts/test_install.py"
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  "$repo_root/scripts/test_standalone_contract.py"
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest "$repo_root/scripts/test_token_effectiveness_task.py"
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest "$repo_root/scripts/test_game_dev_ab_benchmark.py"

if command -v cargo >/dev/null 2>&1; then
  cargo fmt --check --manifest-path "$crate_dir/Cargo.toml"
  cargo test --manifest-path "$crate_dir/Cargo.toml"
  if rustup component list --installed 2>/dev/null | grep -q '^clippy'; then
    cargo clippy --manifest-path "$crate_dir/Cargo.toml" -- -D warnings
  else
    echo "warning: clippy not installed; skipping clippy check" >&2
  fi
  "$repo_root/scripts/build-harnessctl.sh"
  python3 "$repo_root/scripts/token_effectiveness_task.py" \
    --harnessctl "$bin" \
    --format json \
    --output "$tmp_dir/token-effectiveness.json"
  python3 "$repo_root/scripts/game_dev_ab_benchmark.py" \
    --harnessctl "$bin" \
    --format json \
    --output "$tmp_dir/game-dev-ab.json" \
    --output-dir "$tmp_dir/game-dev-ab"
else
  echo "error: cargo is required for full verification" >&2
  exit 1
fi

"$bin" render-prompt \
  --role worker \
  --brief "$tmp_dir/brief.md" \
  --report "$tmp_dir/report.md" \
  --ledger "$tmp_dir/harness.db" \
  --allowed-write-paths issue_feedback_agent/services \
  --allowed-write-paths issue_feedback_agent/tests > "$tmp_dir/worker.prompt"

"$bin" check-prompt --file "$tmp_dir/worker.prompt"

if "$bin" render-prompt \
  --role worker \
  --report "$tmp_dir/bad.md" \
  --ledger "$tmp_dir/harness.db" \
  --allowed-write-paths issue_feedback_agent/tests >/dev/null 2>&1; then
  echo "error: worker prompt without brief unexpectedly passed" >&2
  exit 1
fi

if "$bin" render-prompt \
  --role worker \
  --brief "$tmp_dir/brief.md" \
  --report "$tmp_dir/bad.md" \
  --ledger "$tmp_dir/harness.db" >/dev/null 2>&1; then
  echo "error: worker prompt without write scope unexpectedly passed" >&2
  exit 1
fi

if "$bin" render-prompt --role discussion --report "$tmp_dir/bad.md" --allowed-write-paths /tmp >/dev/null 2>&1; then
  echo "error: discussion prompt with write scope unexpectedly passed" >&2
  exit 1
fi

{
  printf '%s\n' "Stable text"
  printf '\n'
  printf '%s\n' "--- DYNAMIC TASK CONTEXT ---"
  printf '%s\n' "REPORT_PATH=$tmp_dir/bad.md"
  printf '%s\n' "AGENT_LEDGER_PATH=$tmp_dir/harness.db"
  printf '%s\n' "ALLOWED_WRITE_PATHS=issue_feedback_agent/tests"
} > "$tmp_dir/missing-role.prompt"

if "$bin" check-prompt --file "$tmp_dir/missing-role.prompt" >/dev/null 2>&1; then
  echo "error: prompt without ROLE unexpectedly passed" >&2
  exit 1
fi

{
  printf '%s\n' "Stable text"
  printf '\n'
  printf '%s\n' "--- DYNAMIC TASK CONTEXT ---"
  printf '%s\n' "ROLE=bogus"
  printf '%s\n' "REPORT_PATH=$tmp_dir/bad.md"
  printf '%s\n' "AGENT_LEDGER_PATH=$tmp_dir/harness.db"
  printf '%s\n' "ALLOWED_WRITE_PATHS=issue_feedback_agent/tests"
} > "$tmp_dir/bogus-role.prompt"

if "$bin" check-prompt --file "$tmp_dir/bogus-role.prompt" >/dev/null 2>&1; then
  echo "error: prompt with bogus ROLE unexpectedly passed" >&2
  exit 1
fi

"$bin" init \
  --db "$tmp_dir/harness.db" \
  --run verify-run \
  --goal "verify lightweight token harness" \
  --repo-root "$repo_root" \
  --report "$tmp_dir/report.md"
"$bin" task add \
  --db "$tmp_dir/harness.db" \
  --run verify-run \
  --task verify-task \
  --package verify-package \
  --title "verify lifecycle" \
  --sequence 1 \
  --role worker \
  --complexity standard \
  --risk medium \
  --uncertainty standard \
  --write-scope scripts \
  --scope-hash verify-scope \
  --revision verify-revision \
  --profile standard
"$bin" decide \
  --db "$tmp_dir/harness.db" \
  --run verify-run \
  --task verify-task \
  --host codex \
  --related-ready 1 > "$tmp_dir/decision.json"
"$bin" session record \
  --db "$tmp_dir/harness.db" \
  --run verify-run \
  --session verify-session \
  --host codex \
  --handle verify-handle \
  --role worker \
  --profile standard \
  --routing-status unknown \
  --package verify-package \
  --scope-hash verify-scope \
  --revision verify-revision \
  --status starting
"$bin" usage add \
  --db "$tmp_dir/harness.db" \
  --run verify-run \
  --usage verify-usage \
  --phase work \
  --source host \
  --quality unknown
"$bin" status --db "$tmp_dir/harness.db" --run verify-run --json true > "$tmp_dir/status.json"
"$bin" bundle --db "$tmp_dir/harness.db" --run verify-run > "$tmp_dir/bundles.json"
"$bin" host-command --host codex --operation spawn --profile standard --model gpt-5 --prompt verify > "$tmp_dir/host-command.json"

if "$bin" audit --db "$tmp_dir/harness.db" --run verify-run >/dev/null 2>&1; then
  echo "error: final audit unexpectedly passed with active work" >&2
  exit 1
fi

"$bin" task update --db "$tmp_dir/harness.db" --task verify-task --status running --next-action implement
"$bin" task update --db "$tmp_dir/harness.db" --task verify-task --status reported --next-action verify
"$bin" task update --db "$tmp_dir/harness.db" --task verify-task --status accepted
"$bin" session close --db "$tmp_dir/harness.db" --session verify-session --reason complete
"$bin" audit --db "$tmp_dir/harness.db" --run verify-run

echo "verification passed"
