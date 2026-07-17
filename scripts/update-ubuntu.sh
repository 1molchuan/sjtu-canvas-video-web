#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/ubuntu-common.sh
source "$SCRIPT_DIR/lib/ubuntu-common.sh"

require_root
[ -L "$CURRENT_LINK" ] || fail "no installed release; run install-ubuntu.sh first"
[ -n "${1:-}" ] || fail "usage: update-ubuntu.sh /absolute/release/path"
DEPLOY_PUBLIC_HEALTHCHECK="${DEPLOY_PUBLIC_HEALTHCHECK:-1}" \
  "$SCRIPT_DIR/deploy-ubuntu.sh" "$1"
