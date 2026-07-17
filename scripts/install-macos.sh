#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Resolved relative to this script at runtime.
# shellcheck disable=SC1091
source "$ROOT/scripts/lib/macos-common.sh"

[ "$#" -eq 1 ] || fail "usage: install-macos.sh /absolute/path/to/release"
case "$1" in /*) ;; *) fail "release path must be absolute" ;; esac
[ ! -e "$CURRENT_LINK" ] || fail "an installation already exists; use update-macos.sh"
SOURCE="$(cd "$1" && pwd)"
validate_release_source "$SOURCE"

mkdir -p "$SERVICE_ROOT/config" "$LOG_DIR" "$HOME/Library/LaunchAgents"
DESTINATION="$(install_release_copy "$SOURCE")"
activate_release "$DESTINATION"

if [ ! -f "$CONFIG_FILE" ]; then
  umask 077
  sed "s|/Users/YOUR_USER/Services/sjtu-canvas-video|$SERVICE_ROOT|g" \
    "$SOURCE/config/example.toml" > "$CONFIG_FILE"
fi
chmod 600 "$CONFIG_FILE"

sed \
  -e "s|__SERVICE_ROOT__|$SERVICE_ROOT|g" \
  -e "s|__USER_HOME__|$HOME|g" \
  -e "s|__LABEL__|$LAUNCH_LABEL|g" \
  "$SOURCE/deploy/macos/com.example.canvas-video.agent.plist" > "$AGENT_PLIST"
plutil -lint "$AGENT_PLIST" >/dev/null

if grep -Eq 'REPLACE_ME|canvas-video\.example\.com' "$CONFIG_FILE"; then
  printf 'Installed without starting. Replace placeholders in %s, then bootstrap %s.\n' \
    "$CONFIG_FILE" "$AGENT_PLIST"
  exit 0
fi

launchctl bootstrap "gui/$UID" "$AGENT_PLIST"
restart_agent
run_healthcheck
