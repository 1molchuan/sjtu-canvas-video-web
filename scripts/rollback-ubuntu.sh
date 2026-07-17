#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# Resolved relative to this script at runtime.
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/ubuntu-common.sh"

TARGET_SHA="${1:-}"
HEALTHCHECK="$SCRIPT_DIR/healthcheck-ubuntu.sh"

rollback() {
  local target previous
  require_root
  [[ "$TARGET_SHA" =~ ^[0-9a-f]{40}$ ]] || fail "provide an exact 40-character release SHA"
  target="$RELEASES_DIR/$TARGET_SHA"
  verify_release "$target"
  previous="$(current_target)"
  switch_current "$target"
  if restart_service && "$HEALTHCHECK" && "$HEALTHCHECK" --public; then
    printf 'rolled back to %s\n' "$TARGET_SHA"
    return
  fi
  [ -n "$previous" ] && switch_current "$previous"
  restart_service
  "$HEALTHCHECK"
  fail "rollback failed; previous release restored"
}

rollback
