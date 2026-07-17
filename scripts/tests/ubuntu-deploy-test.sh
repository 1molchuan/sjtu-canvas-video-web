#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

export CANVAS_VIDEO_TEST_MODE=1
export CANVAS_VIDEO_APP_ROOT="$TEMP_DIR/opt/canvas-video"
export CANVAS_VIDEO_CONFIG="$TEMP_DIR/etc/config.toml"
export CANVAS_VIDEO_SYSTEMCTL_BIN="$TEMP_DIR/bin/systemctl"
export CANVAS_VIDEO_HEALTH_ATTEMPTS=1
export CANVAS_VIDEO_HEALTH_INTERVAL=0
export CANVAS_VIDEO_LOCAL_HEALTH_URL="file://$TEMP_DIR/healthy"
export CANVAS_VIDEO_PUBLIC_HEALTH_URL="file://$TEMP_DIR/healthy"

mkdir -p "$TEMP_DIR/bin" "$(dirname "$CANVAS_VIDEO_CONFIG")"
touch "$CANVAS_VIDEO_CONFIG" "$TEMP_DIR/healthy"
printf '#!/usr/bin/env bash\nexit 0\n' >"$CANVAS_VIDEO_SYSTEMCTL_BIN"
chmod +x "$CANVAS_VIDEO_SYSTEMCTL_BIN"

make_release() {
  local sha="${1:?sha required}"
  local destination="$TEMP_DIR/source-$sha"
  mkdir -p "$destination/bin" "$destination/frontend/dist"
  printf '#!/usr/bin/env bash\n' >"$destination/bin/sjtu-canvas-video-server"
  chmod +x "$destination/bin/sjtu-canvas-video-server"
  printf '<!doctype html>\n' >"$destination/frontend/dist/index.html"
  printf 'git_sha=%s\n' "$sha" >"$destination/VERSION"
  (cd "$destination" && find bin frontend VERSION -type f -print0 | sort -z | xargs -0 sha256sum >manifest.txt)
  printf '%s\n' "$destination"
}

SHA_ONE="1111111111111111111111111111111111111111"
SHA_TWO="2222222222222222222222222222222222222222"
ONE="$(make_release "$SHA_ONE")"
TWO="$(make_release "$SHA_TWO")"

DEPLOY_PUBLIC_HEALTHCHECK=0 "$ROOT/scripts/deploy-ubuntu.sh" "$ONE"
[ "$(basename "$(readlink -f "$CANVAS_VIDEO_APP_ROOT/current")")" = "$SHA_ONE" ]

DEPLOY_PUBLIC_HEALTHCHECK=0 "$ROOT/scripts/deploy-ubuntu.sh" "$TWO"
[ "$(basename "$(readlink -f "$CANVAS_VIDEO_APP_ROOT/current")")" = "$SHA_TWO" ]

"$ROOT/scripts/rollback-ubuntu.sh" "$SHA_ONE"
[ "$(basename "$(readlink -f "$CANVAS_VIDEO_APP_ROOT/current")")" = "$SHA_ONE" ]

export CANVAS_VIDEO_LOCAL_HEALTH_URL="file://$TEMP_DIR/missing"
if DEPLOY_PUBLIC_HEALTHCHECK=0 "$ROOT/scripts/deploy-ubuntu.sh" "$TWO"; then
  printf 'expected deployment health failure\n' >&2
  exit 1
fi
[ "$(basename "$(readlink -f "$CANVAS_VIDEO_APP_ROOT/current")")" = "$SHA_ONE" ]
printf 'ubuntu deployment transaction tests passed\n'
