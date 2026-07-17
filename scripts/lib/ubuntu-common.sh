#!/usr/bin/env bash
set -euo pipefail

APP_ROOT="${CANVAS_VIDEO_APP_ROOT:-/opt/canvas-video}"
RELEASES_DIR="$APP_ROOT/releases"
CURRENT_LINK="$APP_ROOT/current"
CONFIG_FILE="${CANVAS_VIDEO_CONFIG:-/etc/canvas-video/config.toml}"
SERVICE_NAME="${CANVAS_VIDEO_SERVICE:-canvas-video.service}"
SYSTEMCTL_BIN="${CANVAS_VIDEO_SYSTEMCTL_BIN:-systemctl}"
KEEP_RELEASES="${CANVAS_VIDEO_KEEP_RELEASES:-3}"

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_root() {
  if [ "${CANVAS_VIDEO_TEST_MODE:-0}" = "1" ]; then
    return
  fi
  [ "$(id -u)" -eq 0 ] || fail "run this command as root"
}

require_absolute_dir() {
  local path="${1:?path required}"
  [ "${path#/}" != "$path" ] || fail "release path must be absolute"
  [ -d "$path" ] || fail "release directory does not exist"
}

read_release_sha() {
  local release="${1:?release required}"
  local sha
  sha="$(sed -n 's/^git_sha=//p' "$release/VERSION")"
  [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || fail "VERSION has no valid git_sha"
  printf '%s\n' "$sha"
}

verify_release() {
  local release="${1:?release required}"
  require_absolute_dir "$release"
  [ -x "$release/bin/sjtu-canvas-video-server" ] || fail "server binary missing"
  [ -f "$release/frontend/dist/index.html" ] || fail "frontend index missing"
  [ -f "$release/VERSION" ] || fail "VERSION missing"
  [ -f "$release/manifest.txt" ] || fail "manifest missing"
  (cd "$release" && sha256sum --check --strict manifest.txt >/dev/null)
  verify_forbidden_files "$release"
}

verify_forbidden_files() {
  local release="${1:?release required}"
  local forbidden
  forbidden="$(find "$release" -type f \( -name '*.map' -o -name '*.mp4' -o -name '*.mkv' -o -name '*.webm' -o -name 'config.toml' \) -print -quit)"
  [ -z "$forbidden" ] || fail "release contains a forbidden file"
  [ ! -e "$release/.local" ] || fail "release contains .local"
}

install_release() {
  local source="${1:?source required}"
  local sha destination incoming
  verify_release "$source"
  sha="$(read_release_sha "$source")"
  destination="$RELEASES_DIR/$sha"
  incoming="$RELEASES_DIR/.incoming-$sha-$$"
  if [ -e "$destination" ]; then
    INSTALLED_RELEASE="$destination"
    return
  fi
  mkdir -p "$RELEASES_DIR"
  cp -a "$source" "$incoming"
  if [ "${CANVAS_VIDEO_TEST_MODE:-0}" != "1" ]; then
    chown -R root:root "$incoming"
  fi
  find "$incoming" -type d -exec chmod 755 {} +
  find "$incoming" -type f -exec chmod 644 {} +
  chmod 755 "$incoming/bin/sjtu-canvas-video-server"
  mv "$incoming" "$destination"
  INSTALLED_RELEASE="$destination"
}

switch_current() {
  local release="${1:?release required}"
  local temporary="$APP_ROOT/.current-$$"
  [ -d "$release" ] || fail "release target does not exist"
  ln -s "$release" "$temporary"
  mv -Tf "$temporary" "$CURRENT_LINK"
}

current_target() {
  [ -L "$CURRENT_LINK" ] || return 0
  readlink -f "$CURRENT_LINK"
}

restart_service() {
  "$SYSTEMCTL_BIN" restart "$SERVICE_NAME"
}

prune_releases() {
  local current candidate
  current="$(current_target)"
  mapfile -t candidates < <(find "$RELEASES_DIR" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' | sort -nr | awk '{print $2}')
  for candidate in "${candidates[@]:$KEEP_RELEASES}"; do
    [ "$candidate" = "$current" ] || rm -rf -- "$candidate"
  done
}
