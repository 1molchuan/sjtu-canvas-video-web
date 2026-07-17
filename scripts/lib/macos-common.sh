#!/bin/bash
# State is consumed by sourcing scripts.
# shellcheck disable=SC2034
set -euo pipefail

SERVICE_ROOT="${SJTU_CANVAS_SERVICE_ROOT:-$HOME/Services/sjtu-canvas-video}"
RELEASES_DIR="$SERVICE_ROOT/releases"
CURRENT_LINK="$SERVICE_ROOT/current"
CONFIG_FILE="$SERVICE_ROOT/config/local.toml"
LOG_DIR="$SERVICE_ROOT/logs"
LAUNCH_LABEL="${SJTU_CANVAS_LAUNCH_LABEL:-com.example.canvas-video.agent}"
AGENT_PLIST="$HOME/Library/LaunchAgents/$LAUNCH_LABEL.plist"
HEALTH_URL="${SJTU_CANVAS_HEALTH_URL:-http://127.0.0.1:3100/api/health}"
KEEP_RELEASES=3

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

validate_service_root() {
  case "$SERVICE_ROOT" in
    "$HOME"/*) ;;
    *) fail "service root must be an absolute path inside the current user home" ;;
  esac
  case "$SERVICE_ROOT" in
    *$'\n'*|*'&'*|*'<'*|*'>'*|*'|'*|*'"'*) fail "service root contains unsupported characters" ;;
  esac
}

validate_release_source() {
  local source="$1"
  [ -d "$source" ] || fail "release source is not a directory"
  [ -x "$source/bin/sjtu-canvas-video-server" ] || fail "release server binary is missing"
  [ -f "$source/frontend/dist/index.html" ] || fail "release frontend index is missing"
  [ -f "$source/VERSION" ] || fail "release VERSION is missing"
  [ ! -e "$source/.local" ] || fail "release must not contain .local data"
}

release_sha() {
  local source="$1" sha
  sha="$(sed -n 's/^git_sha=//p' "$source/VERSION")"
  [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || fail "VERSION contains an invalid Git SHA"
  printf '%s\n' "$sha"
}

install_release_copy() {
  local source="$1" sha destination
  sha="$(release_sha "$source")"
  destination="$RELEASES_DIR/$(date -u +%Y%m%d%H%M%S)-$sha"
  [ ! -e "$destination" ] || fail "release destination already exists"
  mkdir -p "$RELEASES_DIR"
  ditto "$source" "$destination"
  printf '%s\n' "$destination"
}

activate_release() {
  local destination="$1"
  case "$destination" in
    "$RELEASES_DIR"/*) ;;
    *) fail "refusing to activate a path outside releases" ;;
  esac
  ln -sfn "$destination" "$CURRENT_LINK"
}

restart_agent() {
  launchctl kickstart -k "gui/$UID/$LAUNCH_LABEL"
}

run_healthcheck() {
  "$CURRENT_LINK/scripts/healthcheck.sh" "$HEALTH_URL"
}

cleanup_old_releases() {
  local count=0 release
  while IFS= read -r release; do
    count=$((count + 1))
    if [ "$count" -gt "$KEEP_RELEASES" ] && [ "$release" != "$(readlink "$CURRENT_LINK")" ]; then
      case "$release" in
        "$RELEASES_DIR"/*) rm -rf "$release" ;;
        *) fail "refusing to remove a path outside releases" ;;
      esac
    fi
  done < <(find "$RELEASES_DIR" -mindepth 1 -maxdepth 1 -type d -print | sort -r)
}

validate_service_root
