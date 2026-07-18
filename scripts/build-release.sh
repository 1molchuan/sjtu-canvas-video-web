#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGING="$ROOT/.local/release-staging-$$"
OUTPUT="$ROOT/release"

cleanup() {
  rm -rf "$STAGING"
}
trap cleanup EXIT

run_frontend_checks() {
  cd "$ROOT/frontend"
  npm ci
  npm run typecheck
  npm run lint
  npm run test
  npm run test:e2e
  npm run build
}

run_rust_checks() {
  cd "$ROOT"
  cargo fmt --all -- --check
  cargo test --workspace --all-targets
  cargo build --release -p server
}

write_version() {
  local sha build_time frontend_version rust_version
  sha="$(git -C "$ROOT" rev-parse HEAD)"
  build_time="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  frontend_version="$(node -p "require('$ROOT/frontend/package.json').version")"
  rust_version="$(awk -F'\"' '/^version = "/ { print $2; exit }' "$ROOT/Cargo.toml")"
  printf '%s\n' \
    "git_sha=$sha" \
    "built_at=$build_time" \
    "frontend_version=$frontend_version" \
    "rust_package_version=$rust_version" > "$STAGING/VERSION"
}

package_release() {
  mkdir -p "$STAGING/bin" "$STAGING/frontend" "$STAGING/config"
  cp "$ROOT/target/release/server" "$STAGING/bin/sjtu-canvas-video-server"
  cp "$ROOT/target/release/invite-admin" "$STAGING/bin/canvas-video-invite-admin"
  cp -R "$ROOT/frontend/dist" "$STAGING/frontend/dist"
  cp "$ROOT/config/production.example.toml" "$STAGING/config/example.toml"
  cp -R "$ROOT/deploy" "$STAGING/deploy"
  cp -R "$ROOT/scripts" "$STAGING/scripts"
  chmod 755 "$STAGING/bin/sjtu-canvas-video-server" "$STAGING/bin/canvas-video-invite-admin"
  write_version
  rm -rf "$OUTPUT"
  mv "$STAGING" "$OUTPUT"
}

run_frontend_checks
run_rust_checks
package_release
trap - EXIT
printf 'Release package: %s\n' "$OUTPUT"
