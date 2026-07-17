#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# Resolved relative to this script at runtime.
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/ubuntu-common.sh"

RELEASE_SOURCE="${1:-}"
UNIT_SOURCE="${CANVAS_VIDEO_UNIT_SOURCE:-$SCRIPT_DIR/../deploy/ubuntu/canvas-video.service}"
UNIT_TARGET="${CANVAS_VIDEO_UNIT_TARGET:-/etc/systemd/system/canvas-video.service}"

create_service_user() {
  if getent passwd canvas-video >/dev/null; then
    return
  fi
  useradd --system --no-create-home --home-dir /nonexistent --shell /usr/sbin/nologin canvas-video
}

install_unit() {
  [ -f "$UNIT_SOURCE" ] || fail "systemd unit template missing"
  if [ -e "$UNIT_TARGET" ]; then
    cp -a "$UNIT_TARGET" "$UNIT_TARGET.backup.$(date -u +%Y%m%dT%H%M%SZ)"
  fi
  install -o root -g root -m 0644 "$UNIT_SOURCE" "$UNIT_TARGET"
  "$SYSTEMCTL_BIN" daemon-reload
  "$SYSTEMCTL_BIN" enable "$SERVICE_NAME"
}

prepare_directories() {
  install -d -o root -g root -m 0755 "$APP_ROOT" "$RELEASES_DIR"
  install -d -o root -g canvas-video -m 0750 "$(dirname "$CONFIG_FILE")"
}

require_root
[ -n "$RELEASE_SOURCE" ] || fail "usage: install-ubuntu.sh /absolute/release/path"
[ -f "$CONFIG_FILE" ] || fail "create the production config before installation"
create_service_user
prepare_directories
chown root:canvas-video "$CONFIG_FILE"
chmod 0640 "$CONFIG_FILE"
install_unit
DEPLOY_PUBLIC_HEALTHCHECK=0 "$SCRIPT_DIR/deploy-ubuntu.sh" "$RELEASE_SOURCE"
