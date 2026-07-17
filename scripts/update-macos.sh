#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Resolved relative to this script at runtime.
# shellcheck disable=SC1091
source "$ROOT/scripts/lib/macos-common.sh"

[ "$#" -eq 1 ] || fail "usage: update-macos.sh /absolute/path/to/release"
case "$1" in /*) ;; *) fail "release path must be absolute" ;; esac
[ -L "$CURRENT_LINK" ] || fail "current release symlink is missing"
[ -f "$CONFIG_FILE" ] || fail "local production config is missing"
SOURCE="$(cd "$1" && pwd)"
validate_release_source "$SOURCE"

PREVIOUS="$(readlink "$CURRENT_LINK")"
DESTINATION="$(install_release_copy "$SOURCE")"
activate_release "$DESTINATION"

if restart_agent && run_healthcheck; then
  cleanup_old_releases
  printf 'Updated to %s\n' "$DESTINATION"
  exit 0
fi

printf 'Update failed; restoring previous release.\n' >&2
activate_release "$PREVIOUS"
restart_agent
run_healthcheck
exit 1
