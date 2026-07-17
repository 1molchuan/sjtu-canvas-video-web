# Phase 2 backend runbook

As delivered, Phase 2 implemented the browser-facing Axum backend only and had no React UI. Phase 3 now
consumes it without changing the verified protocol or Session contract. Use the API contract in
`docs/api.md` for manual checks.

## Automated verification

Automated and Mock tests never contact SJTU services and do not require credentials:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo test -p server --tests
```

Real access remains disabled in the production binary unless `SJTU_REAL_PROTOCOL_TEST` is exactly `1`.
The gate is checked at process startup; it is not a Mock success path.

## Create a private configuration

Copy `config/example.toml` to the Git-ignored `config/local.toml`. Never modify the example with a real
identity. The current example is for Vite local development. Production uses
`config/production.example.toml` and must retain `host = "127.0.0.1"`, an HTTPS `public_origin`, and a
Secure `__Host-` cookie with no Domain.

For local HTTP acceptance only, use:

```toml
[server]
host = "127.0.0.1"
mode = "development"
port = 3100
public_origin = "http://127.0.0.1:3100"
# Keep the remaining server fields from config/example.toml.

[cookie]
name = "sjtu_canvas_video_session"
secure = false
http_only = true
same_site = "Lax"
path = "/"
```

Configuration validation permits this insecure-cookie exception only for loopback HTTP. It rejects
public bind addresses and insecure cookies for non-loopback origins.

## Obtain an allowlist hash

The Phase 1 CLI now prints `whitelist_hash=sha256:...` after a successful, explicitly gated identity
probe. It never prints the underlying stable ID. Run only with your own account:

```powershell
$env:SJTU_REAL_PROTOCOL_TEST = "1"
cargo run -p protocol-cli -- login
```

Place the reported digest in the private configuration:

```toml
[auth]
allowed_stable_ids = []
allowed_stable_id_hashes = ["sha256:replace-with-the-private-result"]
```

If an operator already possesses the raw stable ID, the server helper reads it from redirected stdin
and prints only its normalized digest:

```powershell
$stableId = Read-Host "Stable ID" -MaskInput
$stableId | cargo run -q -p server --bin hash-stable-id
Remove-Variable stableId
```

The helper refuses an interactive terminal input because ordinary terminal echo would expose the value.
Do not paste a stable ID into shell history, source, an issue, or a committed configuration.

## Start the real backend

PowerShell:

```powershell
$env:SJTU_REAL_PROTOCOL_TEST = "1"
$env:SJTU_CANVAS_CONFIG = "config/local.toml"
$env:RUST_LOG = "server=info,canvas_core=info"
cargo run -p server
```

macOS/Linux:

```bash
export SJTU_REAL_PROTOCOL_TEST=1
export SJTU_CANVAS_CONFIG=config/local.toml
export RUST_LOG=server=info,canvas_core=info
cargo run -p server
```

The listener must report `127.0.0.1:3100`. A missing real gate, invalid allowlist, public bind, or unsafe
cookie/origin combination fails startup explicitly.

## Manual same-origin flow

Use a browser page served from the exact configured origin. Requests must retain cookies. The minimum
flow is:

1. `POST /api/auth/qr/start` with exact `Origin`.
2. Open the returned `events_url` with `EventSource`; the pending cookie must be present.
3. Render the received `qr.url` locally and scan it personally.
4. Wait for `authenticated`, then call `GET /api/auth/session` to claim a new website session cookie.
5. Retain `csrf_token` in memory only.
6. Call `GET /api/courses` and select one returned opaque handle.
7. Call its videos endpoint, then one video detail endpoint.
8. Send exact `Origin` plus `X-CSRF-Token` to the selected track ticket endpoint.
9. Request the returned download URL with `Range: bytes=0-0`; expect `206` and `Content-Range` when the
   real source supports Range.
10. Request a small bounded Range and cancel the browser request.
11. `POST /api/auth/logout` with exact Origin and CSRF token.
12. Confirm the prior ticket no longer works and no video file appeared in the repository.

Do not use curl examples that print `Set-Cookie`, ticket URLs, QR URLs, or CSRF values into captured logs.
Browser developer tools are appropriate for local inspection but screenshots must be sanitized.

## Safe evidence to record

Safe manual-acceptance records contain only step status, HTTP status class, response content type,
whether Range and `Content-Range` were present, byte count for the bounded probe, and error class.

Never upload or commit:

- QR URL or signature;
- pending/session/ticket cookie or CSRF token;
- stable ID or account details;
- Canvas course ID/name or video ID/name;
- source URL, host path, query, token, or `tokenId`;
- upstream HTML/JSON bodies;
- `config/local.toml`, `.env`, `.local/`, browser archives, or downloaded video.

Safe troubleshooting logs should use request ID, route template, status, duration, byte count, and error
class. The server deliberately logs `/api/download/:ticket` instead of the concrete request URI.

## Cleanup

Stop the process with Ctrl+C and wait for graceful shutdown. Remove any ad-hoc browser test artifact and
the private local configuration when no longer needed. Authentication state needs no disk cleanup:
cookies, course tokens, source resources, sessions, and tickets live only in memory and are destroyed on
shutdown.

## Real Web acceptance record — 2026-07-17

Environment: Windows, Rust 1.97.1, explicit real-protocol gate enabled, loopback listener at
`127.0.0.1:3100`, local HTTP cookie exception enabled, and the user personally completed QR scans. The
course was selected only from the Cookie-discovered authorized course handles; no raw course ID was
submitted to the Web API.

| Check | Result |
| --- | --- |
| QR start and SSE QR | passed |
| User scan, whitelist, authenticated SSE | passed |
| Website Session claim and CSRF | passed |
| Local Cookie attributes (`HttpOnly`, `SameSite=Lax`, `Path=/`) | passed |
| Cookie-only courses endpoint | passed |
| Authorized course video endpoint | passed |
| Video detail and two track handles | passed |
| Session/CSRF-bound ticket issue | passed |
| HEAD with no body | passed |
| `Range: bytes=0-0`, status `206`, one byte | passed |
| `Content-Range`, `Accept-Ranges`, content type | passed |
| Upstream `Set-Cookie` absent from browser response | passed |
| Cancel after 2920 bytes and immediate next `206` | passed |
| Logout, unauthenticated Session, old ticket rejected | passed |
| Loopback-only listener | passed |
| Video artifact and tracked-private-file scan | passed |

The first authorized course tried by the smoke client returned an explicit `502 UPSTREAM_UNAVAILABLE`;
the next authorized course returned 33 videos. This is recorded as correct error exposure, not an empty
success. The selected video returned two downloadable tracks, both currently classified as `unknown`.

Actual QR, pending, CSRF, ticket, course/video/track handle values were absent from the server logs. A
lexical scan found only the fixed endpoint template name containing the text `TokenId`, not a value.
The logging template was changed to `<video-api-token-exchange>` and a regression test now forbids that
parameter name. The functional real flow was completed before this log-label-only change; the change is
automatically tested and does not alter any request URL.

No complete video was downloaded, no video file was written, `config/local.toml` and `.local/` remained
Git-ignored, and neither local file was committed.
