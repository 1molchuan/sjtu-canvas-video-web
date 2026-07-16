# SJTU Canvas Video Web

Private, browser-based access to course recordings that a signed-in SJTU Canvas user is already
authorized to view. The intended production host is a Mac mini reachable through Cloudflare Tunnel;
the origin must listen only on `127.0.0.1`.

## Current status

Phase 0 is complete at the source-tree level: the reference implementation has been pinned and
analyzed, the security boundaries are documented, and a minimal Rust workspace skeleton exists.
There is intentionally no frontend implementation and no live jAccount/Canvas/LTI implementation yet.

| Evidence class | Status |
| --- | --- |
| Reference source review | Completed against commit `b5d895a` |
| Local unit tests | Passed: 4 tests on 2026-07-17 |
| Local runtime health check | Passed with listener ownership confirmed on `127.0.0.1` |
| Mock protocol integration tests | Not implemented yet |
| Real local jAccount/Canvas/LTI validation | Not performed |
| Cookie-only Canvas course discovery | Go/No-Go unresolved |
| Mac mini / Cloudflare deployment | Not performed |

The reference desktop application uses a manually configured Canvas Personal Access Token for
`/api/v1/users/self` and `/api/v1/courses`. Its QR-created Canvas Web session is used for the course
video LTI flow, but its source does not prove that the same Cookie session can list courses. This is
the first Phase 1 validation gate; the web application will not silently request a Canvas token.

## Phase 0 layout

```text
.
├── crates/
│   ├── canvas-core/       # protocol-domain boundary; no Axum and no live calls yet
│   └── server/            # loopback-only config validation and /api/health
├── config/example.toml
├── docs/
│   ├── reference-analysis.md
│   ├── protocol-validation.md
│   └── security-model.md
├── frontend/              # Phase 4 boundary only
├── deploy/                # Phase 5 boundary only
├── research_sjtu_canvas_helper/
│   └── research_plan.md   # reproducible Phase 0 research plan
└── tests/                 # future mock cross-crate integration tests
```

The shallow reference clone under `research_sjtu_canvas_helper/SJTU-Canvas-Helper/` is ignored by
Git. The permanent evidence and attribution live in `docs/` and `THIRD_PARTY_NOTICES.md`.

## Build and run the Phase 0 skeleton

Requirements: rustup with the toolchain pinned by `rust-toolchain.toml`. No SJTU credentials are
needed for Phase 0 tests.

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
```

Run the health-only server on macOS/Linux:

```bash
SJTU_CANVAS_CONFIG=config/example.toml cargo run -p server
curl http://127.0.0.1:3000/api/health
```

PowerShell:

```powershell
$env:SJTU_CANVAS_CONFIG = "config/example.toml"
cargo run -p server
```

The example allowlist contains a non-user placeholder. Phase 1 must identify and document a stable
jAccount identifier before any real user can be allowed.

Phase 0 was verified with Rust `1.97.1`: `cargo fmt --check`, `cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`, and
`cargo test --workspace --all-targets` all returned success. The runtime health check used an
unoccupied temporary loopback port and confirmed that the listener belonged to the started server
process.

## Security posture

- No jAccount password automation, MFA bypass, course enumeration, or access-control bypass.
- No upstream Cookie, LTI token, video token, `tokenId`, or source URL may reach browser storage.
- Every authenticated website session will own a distinct upstream client and cookie jar.
- Secrets remain memory-only and disappear at logout, expiry, or process restart.
- Video downloads will be ticketed, allowlisted, Range-aware streams; the server will not cache full
  recordings or write temporary video files.
- CORS remains disabled by default, and production bind validation rejects non-loopback addresses.

See [the security model](docs/security-model.md) for the complete trust boundaries. Phase 0's health
router is not the production API and does not yet claim the Phase 3 header, CSRF, session, rate-limit,
or download-proxy controls.

## Reference and license

The protocol research is based on the actual Rust and React implementation of
`Okabe-Rintarou-0/SJTU-Canvas-Helper`, not only its README. The source evidence and known unsafe
desktop assumptions are recorded in [the reference analysis](docs/reference-analysis.md).

This project is MIT licensed. Upstream attribution and the original MIT text are retained in
`THIRD_PARTY_NOTICES.md`.
