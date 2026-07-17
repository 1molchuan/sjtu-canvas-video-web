# SJTU Canvas Video Web

Private, browser-based access to recordings that a signed-in SJTU Canvas user is already authorized to
view. The intended production origin is a Mac mini reached through Cloudflare Tunnel; the Axum process
must listen only on `127.0.0.1`.

## Status

Phase 1 and Phase 1.5 are complete. A user-initiated real run on Windows with Rust 1.97.1 verified the
full protocol chain on 2026-07-17:

```text
jAccount QR → express login → Canvas Cookie session → course discovery
→ OIDC/LTI → video list → video detail/tracks → one-byte Range probe
```

That run used the user's own account and one course they could already open. Cookie-only Canvas course
discovery succeeded without a Personal Access Token. No account, course, video, Cookie, token, path,
query, or complete recording was retained.

Phase 2 implements the formal Axum backend: browser-bound QR pending state and SSE, an allowlisted
website session, CSRF and Origin checks, opaque course/video/track handles, session-bound download
tickets, strict single-Range streaming, concurrency limits, cleanup, security headers, and graceful
shutdown. The complete Web layer is covered by Mock integration tests. A separate user-initiated real
Web acceptance run on 2026-07-17 passed QR start/SSE, whitelist login, website Session, courses, videos,
video detail, ticket issue, HEAD, one-byte `206`, bounded-stream cancellation, permit release, logout,
and ticket invalidation.

There is intentionally no React frontend yet. Phase 2 also contains no video player, cache, database,
object storage, full-video disk write, subtitles, PPT, AI summary, batch download, PAT input, deployment
automation, or public proxy.

## Repository layout

```text
.
├── crates/
│   ├── canvas-core/       # injectable jAccount/Canvas/LTI/video protocol; no Axum
│   ├── protocol-cli/      # explicitly gated real-protocol validator
│   └── server/            # formal Axum session/API/streaming backend
├── config/example.toml    # safe production-shaped example; no real user data
├── docs/
│   ├── api.md
│   ├── download-proxy-security.md
│   ├── phase-1-runbook.md
│   ├── phase-2-backend.md
│   ├── protocol-validation.md
│   ├── reference-analysis.md
│   ├── security-model.md
│   └── web-session-model.md
├── frontend/              # reserved for the later React phase
├── deploy/                # reserved for the deployment phase
└── THIRD_PARTY_NOTICES.md
```

The ignored reference clone under `research_sjtu_canvas_helper/SJTU-Canvas-Helper/` is research input,
not a runtime dependency. Attribution and the upstream MIT text are retained in
`THIRD_PARTY_NOTICES.md`.

## Automated verification

The workspace is pinned by `rust-toolchain.toml`. Mock tests require no SJTU account and never contact
real university services:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo test -p canvas-core --test full_mock_protocol
cargo test -p server --tests
```

## Protocol validator

Real commands are disabled unless the operator explicitly sets `SJTU_REAL_PROTOCOL_TEST=1`. A supplied
course ID must come from a Canvas course URL the operator can already open; the CLI never enumerates
IDs.

PowerShell:

```powershell
$env:SJTU_REAL_PROTOCOL_TEST = "1"
cargo run -p protocol-cli -- login
cargo run -p protocol-cli -- discover-courses
cargo run -p protocol-cli -- full --course-id 12345
```

macOS/Linux:

```bash
export SJTU_REAL_PROTOCOL_TEST=1
cargo run -p protocol-cli -- login
```

The `full` command writes only a sanitized, Git-ignored `.local/protocol-report.json`. See
`docs/phase-1-runbook.md` before scanning.

## Run the Phase 2 backend

Copy `config/example.toml` to the Git-ignored `config/local.toml`, replace the placeholder with a private
stable-identity hash, and keep production settings loopback-only with a Secure `__Host-` cookie. The
protocol CLI prints a deterministic `whitelist_hash=sha256:...` after authenticated identity discovery
without printing the stable ID itself. A stdin-only helper is also available:

```powershell
$stableId = Read-Host "Stable ID" -MaskInput
$stableId | cargo run -q -p server --bin hash-stable-id
Remove-Variable stableId
```

The production server also requires the explicit real-protocol gate:

```powershell
$env:SJTU_REAL_PROTOCOL_TEST = "1"
$env:SJTU_CANVAS_CONFIG = "config/local.toml"
$env:RUST_LOG = "server=info,canvas_core=info"
cargo run -p server
```

For local HTTP acceptance, use a non-`__Host-` cookie with `secure = false` and an exact loopback
`public_origin`. Configuration validation permits that exception only for loopback HTTP. The full setup,
safe evidence rules, and manual flow are in `docs/phase-2-backend.md`.

## Implemented API

```text
GET  /api/health
POST /api/auth/qr/start
GET  /api/auth/qr/events/:pending_id
GET  /api/auth/session
POST /api/auth/logout
GET  /api/me
GET  /api/courses
GET  /api/courses/:course_handle/videos
GET  /api/courses/:course_handle/videos/:video_handle
POST /api/courses/:course_handle/videos/:video_handle/tracks/:track_handle/ticket
HEAD /api/download/:ticket
GET  /api/download/:ticket
```

See `docs/api.md` for response models and status codes.

## Security invariants

- Every authenticated website session owns one independent in-memory `ProtocolContext` and Cookie Jar.
- Upstream cookies, `tokenId`, course tokens, source URLs, and real upstream identifiers never enter
  browser JSON or browser storage.
- Stable identity authorization uses a private exact value or normalized `sha256:` digest, never a name.
- Login success creates a new random website session ID; a pending ID cannot become the session ID.
- Session cookies are `HttpOnly`, `SameSite=Lax`, `Path=/`; production uses Secure `__Host-` semantics.
- State-changing authenticated routes require an exact Origin and session-bound CSRF token; CORS is off.
- Course/video/track handles and tickets are random, memory-only, parent-bound, and session-bound.
- Download URLs cannot encode or accept an upstream URL. The source is registered only from an
  authenticated video-detail response and revalidated before use and after every redirect.
- Only one validated byte range is forwarded. Upstream `Set-Cookie` and hop-by-hop headers are removed.
- Response bodies stream through Axum; complete recordings are not buffered, cached, or written to disk.
- Logout, expiry, shutdown, or process restart destroys the relevant in-memory authentication state.
- Logs use request IDs and route templates, never raw session, pending, ticket, URL, filename, or token.

Detailed trust boundaries are in `docs/security-model.md`, `docs/web-session-model.md`, and
`docs/download-proxy-security.md`.

Real Web evidence remains deliberately narrow: one allowlisted user, one discovered authorized course
with recordings, one video, and one track. The run read one byte for the Range probe and 2920 bytes
before deliberately cancelling a bounded request; it did not download a complete recording. One other
authorized course returned an explicit `502` instead of being misreported as an empty video list, and
the observed track labels currently classify as `unknown`.

## Reference and license

Protocol research is based on the actual Rust and React implementation of
`Okabe-Rintarou-0/SJTU-Canvas-Helper` at commit
`b5d895af57aaa74dfd53cef80dfb64c76c023c20` (`v3.0.8`), not only its README. Desktop-only global state,
persistent cookies/tokens, download workers, and unrelated features were not copied.

This project is MIT licensed. Source evidence, derived protocol facts, copyright attribution, and the
upstream MIT text are documented in `docs/reference-analysis.md` and `THIRD_PARTY_NOTICES.md`.
