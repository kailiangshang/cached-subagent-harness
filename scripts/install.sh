#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$repo_root/scripts/install-runtime.sh"
codex_home="${CODEX_HOME:-"$HOME/.codex"}"
superpowers_ref="${SUPERPOWERS_REF:-main}"
force=0
with_superpowers=0
binary_source="auto"
release_base_url="${HARNESS_RELEASE_BASE_URL:-}"

usage() {
  cat <<'USAGE'
usage: scripts/install.sh [--codex-home PATH] [--force] [--binary-source auto|download|build|none] [--release-base-url URL] [--skip-build] [--with-superpowers] [--skip-superpowers]

Installs cached-subagent-harness into $CODEX_HOME/skills as a standalone skill by default.
Use --with-superpowers to install the optional Superpowers integration.
The default binary source is auto: verified exact-version download, then a locked Cargo build fallback.

Environment:
  SUPERPOWERS_REF  Branch, tag, or commit to install from obra/superpowers. Defaults to main.
  HARNESS_RELEASE_BASE_URL  Exact-version release asset directory used instead of GitHub.
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
    --with-superpowers)
      with_superpowers=1
      shift
      ;;
    --skip-superpowers)
      echo "warning: --skip-superpowers is deprecated and is now a no-op; standalone is the default" >&2
      shift
      ;;
    --skip-build)
      echo "warning: --skip-build is deprecated; use --binary-source none" >&2
      binary_source="none"
      shift
      ;;
    --binary-source)
      binary_source="$2"
      shift 2
      ;;
    --release-base-url)
      release_base_url="$2"
      shift 2
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

case "$binary_source" in
  auto|download|build|none) ;;
  *)
    echo "error: --binary-source must be auto, download, build, or none" >&2
    usage >&2
    exit 1
    ;;
esac

skills_dir="$codex_home/skills"
skill_src="$repo_root/skills/cached-subagent-harness"
skill_dst="$skills_dir/cached-subagent-harness"
package_version="$(
  awk -F '"' '
    /^[[:space:]]*"version"[[:space:]]*:/ { print $4; found += 1 }
    END { if (found != 1) exit 1 }
  ' "$repo_root/.codex-plugin/plugin.json"
)" || {
  echo "error: cannot read package version" >&2
  exit 1
}
if [ -z "$release_base_url" ]; then
  release_base_url="https://github.com/kailiangshang/cached-subagent-harness/releases/download/v$package_version"
fi

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
  local usable_skills=0
  if [ ! -d "$source_dir/skills" ]; then
    echo "error: optional Superpowers checkout is missing its skills directory" >&2
    return 1
  fi
  if ! mkdir -p "$skills_dir"; then
    return 1
  fi
  for skill_path in "$source_dir"/skills/*; do
    [ -d "$skill_path" ] || continue
    local name
    name="${skill_path##*/}"
    if [ ! -f "$skill_path/SKILL.md" ]; then
      echo "error: optional Superpowers skill has invalid layout: $name" >&2
      return 1
    fi
    if [ -e "$skills_dir/$name" ]; then
      if [ ! -f "$skills_dir/$name/SKILL.md" ]; then
        echo "error: existing optional skill has invalid layout: $name" >&2
        return 1
      fi
      echo "superpowers skill already exists, keeping: $name"
    else
      if ! cp -a "$skill_path" "$skills_dir/"; then
        return 1
      fi
      if [ ! -f "$skills_dir/$name/SKILL.md" ]; then
        echo "error: copied optional skill has invalid layout: $name" >&2
        return 1
      fi
      echo "installed superpowers skill: $name"
    fi
    usable_skills=$((usable_skills + 1))
  done
  if [ "$usable_skills" -eq 0 ]; then
    echo "error: optional Superpowers checkout contains no usable skills" >&2
    return 1
  fi
  return 0
}

install_superpowers() {
  if has_superpowers; then
    echo "superpowers detected"
    return 0
  fi
  if ! command -v git >/dev/null 2>&1; then
    echo "error: superpowers not detected and git is unavailable" >&2
    echo "install superpowers from https://github.com/obra/superpowers, then rerun this script" >&2
    return 1
  fi
  if ! mkdir -p "$codex_home"; then
    return 1
  fi
  if [ ! -d "$codex_home/superpowers/.git" ]; then
    if ! git clone --depth 1 --branch "$superpowers_ref" \
      https://github.com/obra/superpowers "$codex_home/superpowers"; then
      return 1
    fi
  else
    if ! git -C "$codex_home/superpowers" fetch --depth 1 origin \
      "$superpowers_ref"; then
      return 1
    fi
    if ! git -C "$codex_home/superpowers" checkout --detach FETCH_HEAD; then
      return 1
    fi
  fi
  if [ ! -d "$codex_home/superpowers/.git" ]; then
    echo "error: optional Superpowers checkout has invalid repository layout" >&2
    return 1
  fi
  if ! copy_superpowers_skills "$codex_home/superpowers"; then
    return 1
  fi
  return 0
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
  rm -f \
    "$skill_dst/scripts/bin/harnessctl" \
    "$skill_dst/scripts/bin/harnessctl.exe"
  echo "installed cached-subagent-harness skill: $skill_dst"
}

install_harnessctl() {
  case "$binary_source" in
    none)
      echo "warning: harnessctl runtime was not installed (--binary-source none)" >&2
      ;;
    download)
      if ! install_verified_release "$package_version" "$release_base_url" "$skill_dst"; then
        echo "error: verified harnessctl download failed; installed Skill is preserved" >&2
        return 1
      fi
      ;;
    build)
      if ! build_runtime "$skill_dst" "$repo_root"; then
        echo "error: harnessctl source build failed; installed Skill is preserved" >&2
        return 1
      fi
      ;;
    auto)
      if install_verified_release "$package_version" "$release_base_url" "$skill_dst"; then
        return 0
      fi
      echo "warning: verified harnessctl download unavailable; falling back to locked Cargo build" >&2
      if ! build_runtime "$skill_dst" "$repo_root"; then
        echo "error: no verified harnessctl runtime could be installed; installed Skill is preserved" >&2
        return 1
      fi
      ;;
  esac
}

install_cached_skill
install_harnessctl

if [ "$with_superpowers" -eq 1 ]; then
  if ! install_superpowers; then
    echo "error: optional Superpowers integration failed; standalone core remains installed" >&2
    exit 1
  fi
fi

echo "done. Restart your CLI runtime to pick up the installed skill."
