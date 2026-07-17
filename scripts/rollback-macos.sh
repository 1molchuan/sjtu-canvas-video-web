#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/lib/macos-common.sh"

[ -L "$CURRENT_LINK" ] || fail "current release symlink is missing"
PREVIOUS="$(readlink "$CURRENT_LINK")"
TARGET="${1:-}"

if [ -z "$TARGET" ]; then
  while IFS= read -r candidate; do
    if [ "$candidate" != "$PREVIOUS" ]; then
      TARGET="$candidate"
      break
    fi
  done < <(find "$RELEASES_DIR" -mindepth 1 -maxdepth 1 -type d -print | sort -r)
else
  TARGET="$RELEASES_DIR/$TARGET"
fi

[ -n "$TARGET" ] || fail "no previous release is available"
[ -d "$TARGET" ] || fail "requested rollback release does not exist"
activate_release "$TARGET"

if restart_agent && run_healthcheck; then
  printf 'Rolled back to %s\n' "$TARGET"
  exit 0
fi

printf 'Rollback target failed; restoring current release.\n' >&2
activate_release "$PREVIOUS"
restart_agent
run_healthcheck
exit 1
