#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
UNIT="$ROOT/deploy/ubuntu/canvas-video.service"
CADDY="$ROOT/deploy/ubuntu/Caddyfile"
CONFIG="$ROOT/deploy/ubuntu/production.example.toml"

grep -Fq 'User=canvas-video' "$UNIT"
grep -Fq 'ProtectSystem=strict' "$UNIT"
grep -Fq 'RestrictAddressFamilies=AF_INET AF_INET6' "$UNIT"
grep -Fq 'SJTU_CANVAS_CONFIG=/etc/canvas-video/config.toml' "$UNIT"
! grep -Eiq 'token|password|stable_id_hash' "$UNIT"

grep -Fq 'canvas.1molchuan.top' "$CADDY"
grep -Fq 'reverse_proxy 127.0.0.1:3100' "$CADDY"
grep -Fq 'header_up Accept-Encoding identity' "$CADDY"
! grep -Eq '(^|[[:space:]])(log|encode|file_server)([[:space:]]|$)' "$CADDY"
! grep -Fq 'flush_interval -1' "$CADDY"

grep -Fq 'host = "127.0.0.1"' "$CONFIG"
grep -Fq 'public_origin = "https://canvas.1molchuan.top"' "$CONFIG"
grep -Fq 'frontend_dist = "/opt/canvas-video/current/frontend/dist"' "$CONFIG"
grep -Fq 'secure = true' "$CONFIG"
grep -Fq 'allowed_stable_id_hashes = ["sha256:REPLACE_ME"]' "$CONFIG"

for script in "$ROOT"/scripts/*.sh "$ROOT"/scripts/lib/*.sh "$ROOT"/scripts/tests/*.sh; do
  bash -n "$script"
done

printf 'ubuntu deployment asset checks passed\n'
