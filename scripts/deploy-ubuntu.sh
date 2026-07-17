#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# Resolved relative to this script at runtime.
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/ubuntu-common.sh"

RELEASE_SOURCE="${1:-}"
PUBLIC_CHECK="${DEPLOY_PUBLIC_HEALTHCHECK:-1}"
HEALTHCHECK="$SCRIPT_DIR/healthcheck-ubuntu.sh"

restore_previous() {
  local previous="${1:-}"
  [ -n "$previous" ] || return 0
  switch_current "$previous"
  restart_service
  "$HEALTHCHECK"
}

deploy_release() {
  local previous destination
  require_root
  require_absolute_dir "$RELEASE_SOURCE"
  [ -r "$CONFIG_FILE" ] || fail "production config is missing"
  previous="$(current_target)"
  install_release "$RELEASE_SOURCE"
  destination="$INSTALLED_RELEASE"
  switch_current "$destination"
  if ! restart_service; then
    restore_previous "$previous"
    fail "service restart failed; previous release restored"
  fi
  if ! "$HEALTHCHECK"; then
    restore_previous "$previous"
    fail "local healthcheck failed; previous release restored"
  fi
  if [ "$PUBLIC_CHECK" = "1" ] && ! "$HEALTHCHECK" --public; then
    restore_previous "$previous"
    fail "public healthcheck failed; previous release restored"
  fi
  prune_releases
  printf 'deployed release %s\n' "$(basename "$destination")"
}

[ -n "$RELEASE_SOURCE" ] || fail "usage: deploy-ubuntu.sh /absolute/release/path"
deploy_release
