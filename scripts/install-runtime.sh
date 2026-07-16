#!/usr/bin/env bash

# Runtime acquisition helpers for scripts/install.sh. This file is sourced.

detect_release_target() {
  local os arch
  os="$(uname -s)" || return 1
  arch="$(uname -m)" || return 1
  case "$os:$arch" in
    Linux:x86_64|Linux:amd64)
      printf '%s\n' "x86_64-unknown-linux-gnu"
      ;;
    Linux:aarch64|Linux:arm64)
      printf '%s\n' "aarch64-unknown-linux-gnu"
      ;;
    Darwin:x86_64|Darwin:amd64)
      printf '%s\n' "x86_64-apple-darwin"
      ;;
    Darwin:aarch64|Darwin:arm64)
      printf '%s\n' "aarch64-apple-darwin"
      ;;
    *)
      echo "error: unsupported harnessctl release platform: $os $arch" >&2
      return 1
      ;;
  esac
}

release_asset_name() {
  local version="$1"
  local target="$2"
  case "$version" in
    ''|v*|*[!0-9A-Za-z.+-]*)
      echo "error: invalid harnessctl release version" >&2
      return 1
      ;;
  esac
  case "$target" in
    x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu|x86_64-apple-darwin|aarch64-apple-darwin)
      printf 'harnessctl-v%s-%s.tar.gz\n' "$version" "$target"
      ;;
    *)
      echo "error: unsupported harnessctl release target: $target" >&2
      return 1
      ;;
  esac
}

download_release_file() {
  local source_url="$1"
  local destination="$2"
  if ! command -v curl >/dev/null 2>&1; then
    echo "error: curl is required to download harnessctl" >&2
    return 1
  fi
  if ! curl --fail --location --silent --output "$destination" "$source_url"; then
    echo "error: harnessctl release download failed" >&2
    return 1
  fi
}

sha256_file() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
  else
    echo "error: sha256sum or shasum is required to verify harnessctl" >&2
    return 1
  fi
}

expected_checksum() {
  local manifest="$1"
  local asset="$2"
  awk -v expected="$asset" '
    $2 == expected { digest = $1; matches += 1 }
    END {
      if (matches != 1 || length(digest) != 64 || digest !~ /^[0-9a-fA-F]+$/) exit 1
      print tolower(digest)
    }
  ' "$manifest"
}

validate_release_archive() {
  local archive="$1"
  local members="$2"
  local details="${members}.details"
  if ! tar -tzf "$archive" > "$members"; then
    echo "error: harnessctl release archive is unreadable" >&2
    return 1
  fi
  if [ "$(wc -l < "$members" | tr -d ' ')" != "2" ] \
    || [ "$(grep -Fxc 'harnessctl' "$members")" != "1" ] \
    || [ "$(grep -Fxc 'LICENSE' "$members")" != "1" ]; then
    echo "error: harnessctl release archive has an unsafe member set" >&2
    return 1
  fi
  if ! LC_ALL=C tar -tvzf "$archive" > "$details" \
    || [ "$(wc -l < "$details" | tr -d ' ')" != "2" ] \
    || ! awk 'substr($1, 1, 1) != "-" { unsafe = 1 } END { exit unsafe }' "$details"; then
    echo "error: harnessctl release archive has an unsafe member type" >&2
    return 1
  fi
}

install_verified_release() {
  local version="$1"
  local base_url="$2"
  local skill_dir="$3"
  local target asset tmp_dir archive checksums expected actual members extract_dir
  target="$(detect_release_target)" || return 1
  asset="$(release_asset_name "$version" "$target")" || return 1
  tmp_dir="$(mktemp -d)" || return 1
  archive="$tmp_dir/$asset"
  checksums="$tmp_dir/SHA256SUMS"
  members="$tmp_dir/members"
  extract_dir="$tmp_dir/extract"
  mkdir -p "$extract_dir"

  if ! download_release_file "${base_url%/}/$asset" "$archive" \
    || ! download_release_file "${base_url%/}/SHA256SUMS" "$checksums"; then
    rm -rf "$tmp_dir"
    return 1
  fi
  if ! expected="$(expected_checksum "$checksums" "$asset")"; then
    echo "error: release checksum entry is missing, duplicated, or invalid" >&2
    rm -rf "$tmp_dir"
    return 1
  fi
  if ! actual="$(sha256_file "$archive")"; then
    rm -rf "$tmp_dir"
    return 1
  fi
  if [ "$actual" != "$expected" ]; then
    echo "error: harnessctl release checksum mismatch" >&2
    rm -rf "$tmp_dir"
    return 1
  fi
  if ! validate_release_archive "$archive" "$members"; then
    rm -rf "$tmp_dir"
    return 1
  fi
  if ! tar -xzf "$archive" -C "$extract_dir" harnessctl LICENSE; then
    echo "error: harnessctl release extraction failed" >&2
    rm -rf "$tmp_dir"
    return 1
  fi
  if [ -L "$extract_dir/harnessctl" ] \
    || [ -L "$extract_dir/LICENSE" ] \
    || [ ! -f "$extract_dir/harnessctl" ] \
    || [ ! -f "$extract_dir/LICENSE" ]; then
    echo "error: harnessctl release executable is missing" >&2
    rm -rf "$tmp_dir"
    return 1
  fi

  mkdir -p "$skill_dir/scripts/bin"
  local staged="$skill_dir/scripts/bin/.harnessctl.install.$$"
  if ! cp "$extract_dir/harnessctl" "$staged"; then
    rm -rf "$tmp_dir"
    return 1
  fi
  chmod 755 "$staged"
  if ! mv -f "$staged" "$skill_dir/scripts/bin/harnessctl"; then
    rm -f "$staged"
    rm -rf "$tmp_dir"
    return 1
  fi
  rm -rf "$tmp_dir"
  echo "installed verified harnessctl $version for $target"
}

build_runtime() {
  local skill_dir="$1"
  local repo_root="$2"
  if ! command -v cargo >/dev/null 2>&1; then
    echo "error: Cargo is required to build harnessctl" >&2
    return 1
  fi
  SKILL_DIR="$skill_dir" "$repo_root/scripts/build-harnessctl.sh"
}
