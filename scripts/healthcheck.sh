#!/bin/bash
set -euo pipefail

HEALTH_URL="${1:-http://127.0.0.1:3100/api/health}"
ATTEMPTS=15
CONNECT_TIMEOUT=2

for ((attempt = 1; attempt <= ATTEMPTS; attempt += 1)); do
  if curl --fail --silent --show-error \
    --connect-timeout "$CONNECT_TIMEOUT" \
    --max-time "$CONNECT_TIMEOUT" \
    "$HEALTH_URL" >/dev/null; then
    printf 'healthy: %s\n' "$HEALTH_URL"
    exit 0
  fi
  sleep 1
done

printf 'health check failed after %s attempts\n' "$ATTEMPTS" >&2
exit 1
