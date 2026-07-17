# Phase 1 protocol validation implementation and evidence

## Truthful status at the end of Phase 0

| Area | Source reviewed | Mock verified | Real SJTU verified |
| --- | --- | --- | --- |
| UUID extraction | Yes | No | No |
| jAccount WebSocket and QR update | Yes | No | No |
| Express login / `JAAuthCookie` | Yes | No | No |
| Canvas Web-session establishment | Yes | No | No |
| Stable user identity | Partial source evidence | No | No |
| Cookie-only course discovery | Reference uses PAT instead | No | No |
| LTI forms and `tokenId` | Yes | No | No |
| Video list and detail | Yes | No | No |
| Actual video host / Referer / Range | Source assumptions only | No | No |

No live request has been made with a real account in Phase 0.

## Truthful status after Phase 1.5 real validation

The protocol client and CLI exist, the deterministic mock suite is green, and a user explicitly ran
the complete validator against their own authorized SJTU account and course on 2026-07-17. Mock and
real evidence remain separate: mocks cover adverse branches, while the real run proves only the one
authorized path and upstream behavior observed on that date.

| Area | Unit/mock verified | Real SJTU status |
| --- | --- | --- |
| UUID HTML parsing | passed | passed |
| backend WebSocket, QR update, LOGIN, timeout/cancel | passed | passed |
| express login and in-memory `JAAuthCookie` capture | passed | passed |
| controlled Canvas SSO and authenticated-page probe | passed | passed |
| stable identity probing without display-name fallback | passed | passed (`CanvasSelf`) |
| Cookie-only REST and dashboard course experiments | passed | REST passed without PAT |
| external tool, OIDC/LTI forms, redirect and `tokenId` | passed | passed |
| course-bound token exchange, list, detail and multi-track parsing | passed | passed |
| stateless `bytes=0-0` probe, DNS/IP checks, 200/206/416 | passed | passed for observed source |

Current Go/No-Go: **Go A**. The real run passed login, identity, Cookie-only course discovery, LTI,
video list, video detail, track extraction, and the one-byte Range probe.

## CLI boundary

Phase 1 provides a `protocol-cli` binary that owns one ephemeral upstream context and one cookie jar
for its entire run. It never reads the desktop helper's config and never accepts a Canvas Personal
Access Token. Real execution is gated by `SJTU_REAL_PROTOCOL_TEST=1`:

```bash
SJTU_REAL_PROTOCOL_TEST=1 cargo run -p protocol-cli -- full --course-id 12345
```

`--course-id` is a development-only, manually supplied course the operator already has permission to
access. It is used only after the course-discovery gate and is never exposed in the production UI.

The CLI renders a terminal QR without printing its signed URL, waits for the operator to scan it,
then prints only stage names and sanitized outcomes. It does not automate passwords or MFA.

## Logging contract

Allowed diagnostics:

- stage name, HTTP status, duration, content type, response byte count;
- scheme and host, without path/query, for redirect and video-source validation;
- cookie names and count, never values;
- stable user ID only as a hash or partial redaction suitable for correlation;
- course count and hashed video/track IDs; the supplied course ID is not included in the report;
- Range capability and sanitized size headers.

Forbidden diagnostics:

- `JAAuthCookie`, Canvas cookies, `Authorization`, LTI token, video token, `tokenId`;
- QR signature, full QR URL, full redirect URL, full HTML, JSON bodies containing secrets;
- signed video path/query, display name, legal name, email, or unrelated account profile fields.

Debug mode changes verbosity, not redaction.

## Validation sequence

### Gate A — jAccount QR state machine

1. Create a fresh cookie store and request `my.sjtu.edu.cn/ui/appmyinfo`.
2. Parse exactly one UUID; reject missing or multiple candidates as a structured error.
3. Connect backend-side to the observed jAccount WebSocket URL with explicit connect/read deadlines.
4. Send the QR refresh message and validate the message envelope before using `ts`/`sig`.
5. Surface QR-ready, login, expiry, disconnect, and error events to the CLI state machine.
6. On `LOGIN`, call express login exactly once and assert that `JAAuthCookie` exists by name.
7. Close the WebSocket and erase the pending QR state after success or timeout.

Evidence to record: endpoint host, event type names, refresh interval behavior, expiry behavior, and
whether a distinct scan event exists. Do not record UUID/signature values.

### Gate B — Canvas Web session and stable identity

1. Attach `JAAuthCookie` only to the observed jAccount/mySJTU origins.
2. Perform Canvas OpenID login with manual redirect handling and an explicit host allowlist.
3. Verify the final origin is Canvas and probe a benign authenticated Canvas page.
4. Request `my.sjtu.edu.cn/api/account`; inspect its schema locally without dumping it to logs.
5. Identify candidate stable fields, documenting type, stability semantics, and whether they are
   jAccount identifiers rather than display names.
6. Cross-check against a cookie-authenticated Canvas `/api/v1/users/self` response if available.

If no stable identity can be safely established, Phase 1 must report that fact. A one-time invite-code
bridge may be designed later, but cannot be mislabeled as stable identity verification.

### Gate C — Cookie-only Canvas course discovery (first Go/No-Go)

Using the same Canvas cookie jar and no `Authorization` header:

1. Call `/api/v1/courses?include[]=teachers&include[]=term&per_page=100`.
2. Record status, content type, redirect host, pagination headers, and parsed course count.
3. If rejected, inspect Canvas HTML bootstrap data used by the authenticated courses page.
4. If absent, inspect the GraphQL or other endpoint actually requested by that page.
5. Never inject a PAT as a fallback.

Decision outcomes:

- **Go:** a Cookie-bound endpoint returns only the signed-in user's authorized courses. Document the
  endpoint and implement it in `canvas-core`.
- **No-Go:** all supported discovery paths require PAT or cannot be safely identified. Implement
  `CanvasCourseDiscoveryUnavailable`, document optional developer-only course IDs, and stop before a
  formal user-facing course picker.
- **Inconclusive:** page behavior is unstable or evidence is insufficient. Keep the gate open; do not
  claim course discovery.

### Gate D — LTI launch for an authorized course

1. Request the external-tool page for a course obtained from Gate C or the explicit test course ID.
2. Parse the form using DOM rules, resolve its action against the response URL, and require the
   expected HTTPS host.
3. Preserve every successful-control field, including duplicate names, in original order.
4. Submit each form with automatic redirects disabled. Follow the OIDC Canvas authorization chain for
   at most eight hops, validating the exact Canvas origin at every hop; keep the final LTI POST
   redirect-disabled and validate its Video API `Location`.
5. Parse `tokenId` with a URL parser, exchange it, and immediately wrap the returned token as secret.
6. Confirm the returned `courId` relationship to the original Canvas course without logging token data.

Capture only sanitized field names and counts. Save redacted fixtures for mock tests only when they
contain no session-specific or personal data.

### Gate E — video list and tracks

1. Request `findVodVideoList` with the course-scoped secret and returned course ID.
2. Treat missing `data`, non-success business codes, and malformed JSON as errors, not empty lists.
3. Select one authorized video and request `getVodVideoInfos` with the same course binding.
4. Record track count and non-secret metadata fields such as channel/view numbers.
5. Correlate those fields with observed blackboard/screen content before assigning semantic labels.
6. Test switching A → B → A to prove a token from one course is never reused for another.
7. Observe token-expiry behavior and verify that one fresh LTI launch recovers; do not retry again.

### Gate F — source host, Referer, and Range

For one short authorized recording track:

1. Parse its source URL server-side; record only scheme and host.
2. Resolve DNS and reject loopback, private, link-local, multicast, unspecified, and metadata ranges.
3. Send `Range: bytes=0-0` with the reference Referer and record status plus sanitized
   `Accept-Ranges`, `Content-Range`, `Content-Length`, `Content-Type`, `Last-Modified`, and `ETag`.
4. Repeat without Referer only if necessary to establish whether Referer is required; do not download
   the complete object.
5. Test a valid small range, an unsatisfiable range, and redirect behavior.
6. Add only observed source and redirect hosts to the candidate allowlist.

## Mock-first implementation result

The deterministic suite now covers:

- UUID HTML extraction and missing/ambiguous UUIDs;
- WebSocket QR update, login, timeout, cancellation, and missing-cookie failures;
- express-login cookie capture without exposing values;
- Canvas redirect allowlisting, Cookie-only course responses, and missing display fields;
- both LTI forms, duplicate fields, relative/invalid actions, bounded Canvas OIDC redirects, redirect
  loops, cross-purpose redirects, and token redirect parsing;
- token exchange, video list/details, token expiry then one refresh;
- `200`, `206`, `416`, malformed `Content-Range`, rejected redirect, and Range header parsing;
- one complete mock chain from UUID through Range using one isolated context;
- two-context Cookie/token isolation and A → B → A token use;
- CLI argument/gate/report contracts and a production-source sensitive-output scan.

Mocks never become evidence that a real SJTU endpoint works.

## Current real-validation record

Recorded on 2026-07-17 on Windows with Rust 1.97.1. The user set the explicit real-test gate, completed
the QR scan personally, and supplied one course ID from a course they could already open. No account,
course, or video identifier is retained in this document.

| Step | Status |
| --- | --- |
| jAccount UUID | passed |
| jAccount WebSocket | passed |
| QR | passed |
| express login | passed |
| Canvas login | passed |
| identity | passed |
| course discovery | passed |
| LTI | passed |
| video list | passed |
| video detail | passed |
| Range probe | passed |

Sanitized observations: identity came from the authenticated Canvas self endpoint; Canvas REST course
discovery returned a JSON array without a Bearer token; the OIDC initiation required a bounded Canvas
redirect chain; the video source host was `live.sjtu.edu.cn`; and the one-byte request reported Range
support. The request used the reference Referer, but this run did not test whether omitting it fails.
`.local/protocol-report.json` is ignored by Git and passed the local forbidden-key/value scan.

## Phase 1.5 evidence boundaries

This record distinguishes:

- automatically unit-tested;
- mock integration-tested;
- locally real-environment verified, including date and sanitized endpoint host;
- adverse branches verified only by mocks;
- Go A for Cookie course discovery and the authorized video chain;
- stable identity source without the underlying value;
- observed video host and Range support;
- known limitations: one account, one authorized course, one selected video/track, no full download,
  no Referer-negative experiment, and upstream protocol drift remains possible.

## Phase 2 relationship to this evidence

Phase 2 reuses this already verified `canvas-core` implementation; it does not replace real protocol
evidence with Web mocks. The formal server wraps each protocol context in one isolated website session,
exposes only opaque resource handles, and uses a Cookie-free streaming client for ticketed Range
requests. The production server binary now enforces the same explicit `SJTU_REAL_PROTOCOL_TEST=1` gate
before it can start real login work.

Web integration tests cover pending/SSE binding, whitelist enforcement, session and handle isolation,
CSRF/Origin checks, ticket lifetime, Range streaming, cancellation, header filtering, and safe logs using
Mock upstreams only. Real Phase 2 browser/API acceptance is a distinct evidence class and must not be
reported as passed solely because the Phase 1.5 CLI chain passed.
