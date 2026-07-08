#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
skill_dir="${SKILL_DIR:-"$repo_root/skills/cached-subagent-harness"}"
crate_dir="$skill_dir/scripts/harnessctl"
bin_dir="$skill_dir/scripts/bin"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required to build harnessctl" >&2
  exit 1
fi

if [ ! -f "$crate_dir/Cargo.toml" ]; then
  echo "error: missing harnessctl Cargo.toml at $crate_dir" >&2
  exit 1
fi

cargo build --release --manifest-path "$crate_dir/Cargo.toml"
mkdir -p "$bin_dir"
cp "$crate_dir/target/release/harnessctl" "$bin_dir/harnessctl"
chmod +x "$bin_dir/harnessctl"
echo "built $bin_dir/harnessctl"
