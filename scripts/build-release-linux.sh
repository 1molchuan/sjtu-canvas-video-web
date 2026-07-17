#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT="$(realpath -m "${1:-$ROOT/release-linux}")"
STAGING="$OUTPUT.staging.$$"
BUILD_TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

validate_output() {
  case "$OUTPUT/" in
    "$ROOT"/*) ;;
    *) fail "release output must stay inside the source root" ;;
  esac
  [ "$OUTPUT" != "$ROOT" ] || fail "release output cannot replace the source root"
}

source_identity() {
  if git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git -C "$ROOT" diff --quiet || fail "tracked source changes must be committed before build"
    git -C "$ROOT" diff --cached --quiet || fail "staged source changes must be committed before build"
    GIT_SHA="$(git -C "$ROOT" rev-parse HEAD)"
    GIT_DESCRIBE="$(git -C "$ROOT" describe --always)"
    return
  fi
  GIT_SHA="${SOURCE_GIT_SHA:-}"
  GIT_DESCRIBE="${SOURCE_GIT_DESCRIBE:-$GIT_SHA}"
  [[ "$GIT_SHA" =~ ^[0-9a-f]{40}$ ]] || fail "SOURCE_GIT_SHA is required for archive builds"
}

frontend_checks() {
  (
    cd "$ROOT/frontend"
    npm ci
    npm run typecheck
    npm run lint
    npm run test
    npm run build
    if [ "${RUN_PLAYWRIGHT_E2E:-0}" = "1" ]; then
      npm run test:e2e
    else
      printf 'Ubuntu Playwright: not_run (set RUN_PLAYWRIGHT_E2E=1 to enable)\n'
    fi
  )
}

rust_checks() {
  (
    cd "$ROOT"
    cargo fmt --all -- --check
    cargo check --workspace --all-targets
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    timeout 60s cargo test --workspace --all-targets
    cargo build --release -p server
  )
}

write_version() {
  local frontend_version target_triple
  frontend_version="$(node -p "require('$ROOT/frontend/package.json').version")"
  target_triple="$(rustc -vV | sed -n 's/^host: //p')"
  {
    printf 'git_sha=%s\n' "$GIT_SHA"
    printf 'git_describe=%s\n' "$GIT_DESCRIBE"
    printf 'build_timestamp_utc=%s\n' "$BUILD_TIMESTAMP"
    printf 'rust_version=%s\n' "$(rustc --version)"
    printf 'node_version=%s\n' "$(node --version)"
    printf 'frontend_version=%s\n' "$frontend_version"
    printf 'target_triple=%s\n' "$target_triple"
  } >"$STAGING/VERSION"
}

verify_staging() {
  [ -x "$STAGING/bin/sjtu-canvas-video-server" ] || fail "release binary missing"
  [ -f "$STAGING/frontend/dist/index.html" ] || fail "frontend index missing"
  if find "$STAGING" -type f \( -name '*.map' -o -name '*.mp4' -o -name '*.mkv' -o -name '*.webm' -o -name 'config.toml' \) -print -quit | grep -q .; then
    fail "release contains forbidden files"
  fi
}

package_release() {
  rm -rf -- "$STAGING"
  mkdir -p "$STAGING/bin" "$STAGING/frontend"
  install -m 0755 "$ROOT/target/release/server" "$STAGING/bin/sjtu-canvas-video-server"
  cp -a "$ROOT/frontend/dist" "$STAGING/frontend/dist"
  write_version
  verify_staging
  (cd "$STAGING" && find bin frontend VERSION -type f -print0 | sort -z | xargs -0 sha256sum >manifest.txt)
  rm -rf -- "$OUTPUT"
  mv "$STAGING" "$OUTPUT"
}

cleanup() {
  [ ! -e "$STAGING" ] || rm -rf -- "$STAGING"
}

trap cleanup EXIT
validate_output
source_identity
frontend_checks
rust_checks
package_release
printf 'Linux release package: %s\n' "$OUTPUT"
