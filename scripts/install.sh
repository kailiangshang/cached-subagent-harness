#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
codex_home="${CODEX_HOME:-"$HOME/.codex"}"
superpowers_ref="${SUPERPOWERS_REF:-main}"
force=0
skip_superpowers=0
skip_build=0

usage() {
  cat <<'USAGE'
usage: scripts/install.sh [--codex-home PATH] [--force] [--skip-superpowers] [--skip-build]

Installs cached-subagent-harness into $CODEX_HOME/skills and checks Superpowers.

Environment:
  SUPERPOWERS_REF  Branch, tag, or commit to install from obra/superpowers. Defaults to main.
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --codex-home)
      codex_home="$2"
      shift 2
      ;;
    --force)
      force=1
      shift
      ;;
    --skip-superpowers)
      skip_superpowers=1
      shift
      ;;
    --skip-build)
      skip_build=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

skills_dir="$codex_home/skills"
skill_src="$repo_root/skills/cached-subagent-harness"
skill_dst="$skills_dir/cached-subagent-harness"

has_superpowers() {
  [ -f "$skills_dir/using-superpowers/SKILL.md" ] && return 0
  [ -f "$codex_home/superpowers/skills/using-superpowers/SKILL.md" ] && return 0
  if [ -d "$codex_home/plugins/cache" ]; then
    find "$codex_home/plugins/cache" -maxdepth 5 -path '*/skills/using-superpowers/SKILL.md' -print -quit | grep -q .
  else
    return 1
  fi
}

copy_superpowers_skills() {
  local source_dir="$1"
  mkdir -p "$skills_dir"
  for skill_path in "$source_dir"/skills/*; do
    [ -d "$skill_path" ] || continue
    local name
    name="$(basename "$skill_path")"
    if [ -e "$skills_dir/$name" ]; then
      echo "superpowers skill already exists, keeping: $name"
    else
      cp -a "$skill_path" "$skills_dir/"
      echo "installed superpowers skill: $name"
    fi
  done
}

install_superpowers() {
  if has_superpowers; then
    echo "superpowers detected"
    return 0
  fi
  if [ "$skip_superpowers" -eq 1 ]; then
    echo "warning: superpowers not detected; continuing because --skip-superpowers was set" >&2
    return 0
  fi
  if ! command -v git >/dev/null 2>&1; then
    echo "error: superpowers not detected and git is unavailable" >&2
    echo "install superpowers from https://github.com/obra/superpowers, then rerun this script" >&2
    exit 1
  fi
  mkdir -p "$codex_home"
  if [ ! -d "$codex_home/superpowers/.git" ]; then
    git clone --depth 1 --branch "$superpowers_ref" https://github.com/obra/superpowers "$codex_home/superpowers"
  else
    git -C "$codex_home/superpowers" fetch --depth 1 origin "$superpowers_ref"
    git -C "$codex_home/superpowers" checkout --detach FETCH_HEAD
  fi
  copy_superpowers_skills "$codex_home/superpowers"
}

install_cached_skill() {
  mkdir -p "$skills_dir"
  if [ -e "$skill_dst" ]; then
    if [ "$force" -ne 1 ]; then
      echo "error: $skill_dst already exists; rerun with --force to replace it" >&2
      exit 1
    fi
    rm -rf "$skill_dst"
  fi
  cp -a "$skill_src" "$skill_dst"
  echo "installed cached-subagent-harness skill: $skill_dst"
}

build_harnessctl() {
  if [ "$skip_build" -eq 1 ]; then
    echo "skipping harnessctl build"
    return 0
  fi
  if command -v cargo >/dev/null 2>&1; then
    SKILL_DIR="$skill_dst" "$repo_root/scripts/build-harnessctl.sh"
  else
    echo "warning: cargo not found; harnessctl binary was not built" >&2
    echo "the skill can still use legacy Python helpers, but ledger enforcement is degraded" >&2
  fi
}

install_superpowers
install_cached_skill
build_harnessctl

echo "done. Restart Codex to pick up the installed skill."
