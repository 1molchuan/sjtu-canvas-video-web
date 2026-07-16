# SJTU Canvas Video Web

Private, browser-based access to course recordings that a signed-in SJTU Canvas user is already
authorized to view. The intended production host is a Mac mini reachable through Cloudflare Tunnel;
the origin must listen only on `127.0.0.1`.

## Current status

Phase 1 protocol code and the validation CLI are implemented. The complete protocol chain is covered
by deterministic local mocks, but no real jAccount scan or SJTU request was performed during this
implementation pass. There is intentionally still no React frontend, browser session, ticket, or
download proxy.

| Evidence class | Status |
| --- | --- |
| Reference source review | Completed against commit `b5d895a` |
| Local unit and contract tests | Passed on 2026-07-17 |
| Local runtime health check | Passed with listener ownership confirmed on `127.0.0.1` |
| Mock protocol integration tests | Passed, including UUID-to-Range full chain |
| Real local jAccount/Canvas/LTI validation | not_run |
| Cookie-only Canvas course discovery | Go/No-Go unresolved |
| Mac mini / Cloudflare deployment | Not performed |

The reference desktop application uses a manually configured Canvas Personal Access Token for
`/api/v1/users/self` and `/api/v1/courses`. Its QR-created Canvas Web session is used for the course
video LTI flow, but its source does not prove that the same Cookie session can list courses. This is
the first Phase 1 validation gate; the web application will not silently request a Canvas token.

## Current layout

```text
.
├── crates/
│   ├── canvas-core/       # injectable HTTP/WebSocket/Canvas/LTI/video protocol core; no Axum
│   ├── protocol-cli/      # explicitly gated interactive Phase 1 validator
│   └── server/            # loopback-only config validation and /api/health
├── config/example.toml
├── docs/
│   ├── reference-analysis.md
│   ├── protocol-validation.md
│   ├── phase-1-runbook.md
│   └── security-model.md
├── frontend/              # Phase 4 boundary only
├── deploy/                # Phase 5 boundary only
├── research_sjtu_canvas_helper/
│   └── research_plan.md   # reproducible Phase 0 research plan
└── tests/                 # reserved for later repository-wide integration tests
```

The shallow reference clone under `research_sjtu_canvas_helper/SJTU-Canvas-Helper/` is ignored by
Git. The permanent evidence and attribution live in `docs/` and `THIRD_PARTY_NOTICES.md`.

## Build, mock-test, and run the validator

Requirements: rustup with the toolchain pinned by `rust-toolchain.toml`. No SJTU credentials are
needed for automated tests.

```bash
cargo fmt --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo test -p canvas-core --test full_mock_protocol
```

Real protocol commands are disabled unless the operator explicitly sets
`SJTU_REAL_PROTOCOL_TEST=1`. The course ID must come from a Canvas course URL that the operator can
already open; the CLI never enumerates IDs:

```bash
export SJTU_REAL_PROTOCOL_TEST=1
cargo run -p protocol-cli -- login
cargo run -p protocol-cli -- discover-courses
cargo run -p protocol-cli -- full --course-id 12345
```

`full` writes only a sanitized `.local/protocol-report.json`, which is ignored by Git. See the
[Phase 1 runbook](docs/phase-1-runbook.md) before any real scan.

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

The current workspace is verified with Rust `1.97.1`. Mock success is not recorded as real endpoint
success; all real steps remain `not_run` in [the validation record](docs/protocol-validation.md).

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
