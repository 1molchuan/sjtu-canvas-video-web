#!/usr/bin/env bash
set -euo pipefail

PUBLIC_ORIGIN="${CANVAS_VIDEO_PUBLIC_ORIGIN:-https://canvas.1molchuan.top}"
LOCAL_ORIGIN="${CANVAS_VIDEO_LOCAL_ORIGIN:-http://127.0.0.1:3100}"
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

fetch_headers() {
  local url="${1:?url required}"
  local output="${2:?output required}"
  curl --fail --silent --show-error --dump-header "$output" --output /dev/null "$url"
}

assert_header() {
  local file="${1:?file required}"
  local pattern="${2:?pattern required}"
  grep -Eiq "$pattern" "$file" || { printf 'missing expected header\n' >&2; return 1; }
}

fetch_headers "$LOCAL_ORIGIN/api/health" "$TEMP_DIR/local-health"
fetch_headers "$PUBLIC_ORIGIN/api/health" "$TEMP_DIR/public-health"
fetch_headers "$PUBLIC_ORIGIN/" "$TEMP_DIR/root"
assert_header "$TEMP_DIR/public-health" '^cache-control:.*no-store'
assert_header "$TEMP_DIR/root" '^content-security-policy:'

http_code="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' "http://canvas.1molchuan.top/")"
[[ "$http_code" =~ ^30[1278]$ ]] || { printf 'HTTP does not redirect to HTTPS\n' >&2; exit 1; }

api_code="$(curl --silent --show-error --output "$TEMP_DIR/api-body" --write-out '%{http_code}' "$PUBLIC_ORIGIN/api/unknown")"
[ "$api_code" = "404" ] || { printf 'unknown API route did not return 404\n' >&2; exit 1; }
grep -Eq '"error"' "$TEMP_DIR/api-body" || { printf 'unknown API response is not JSON error\n' >&2; exit 1; }
printf 'production public checks passed\n'
