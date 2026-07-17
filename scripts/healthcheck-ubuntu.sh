#!/usr/bin/env bash
set -euo pipefail

LOCAL_URL="${CANVAS_VIDEO_LOCAL_HEALTH_URL:-http://127.0.0.1:3100/api/health}"
PUBLIC_URL="${CANVAS_VIDEO_PUBLIC_HEALTH_URL:-https://canvas.1molchuan.top/api/health}"
ATTEMPTS="${CANVAS_VIDEO_HEALTH_ATTEMPTS:-15}"
INTERVAL="${CANVAS_VIDEO_HEALTH_INTERVAL:-1}"
CHECK_PUBLIC=0

[ "${1:-}" != "--public" ] || CHECK_PUBLIC=1

check_url() {
  local url="${1:?url required}"
  curl --fail --silent --show-error --location --max-time 10 "$url" >/dev/null
}

wait_for_url() {
  local url="${1:?url required}"
  local attempt
  for ((attempt = 1; attempt <= ATTEMPTS; attempt++)); do
    if check_url "$url"; then
      return
    fi
    sleep "$INTERVAL"
  done
  printf 'healthcheck failed after %s attempts\n' "$ATTEMPTS" >&2
  return 1
}

wait_for_url "$LOCAL_URL"
if [ "$CHECK_PUBLIC" -eq 1 ]; then
  wait_for_url "$PUBLIC_URL"
fi
printf 'healthcheck passed\n'
