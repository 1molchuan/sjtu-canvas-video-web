# Security model

## Scope and assets

The service is a private convenience layer for a few allowlisted users. It does not grant access that
the user's own jAccount/Canvas session lacks, and it does not become a sharing or archival service.

Sensitive assets are jAccount and Canvas cookies, LTI `tokenId`, video access tokens, signed source
URLs, stable user identifiers, pending-login state, authenticated sessions, and download tickets.
Course metadata and video names are private user data even when they are not authentication secrets.

## Trust boundaries

1. Browser ↔ Axum: the browser receives only this site's random session cookie and public models.
2. Axum ↔ jAccount/Canvas/video systems: one independently constructed client and cookie store per
   authenticated user session.
3. Download registry ↔ source host: source URLs enter only from authenticated upstream video details.
4. Mac mini ↔ Cloudflare Tunnel: the origin binds to loopback; cloudflared initiates the connection.
5. Process memory ↔ disk/logs: authentication state is memory-only and logs contain sanitized IDs.

## Non-negotiable invariants

- No global upstream cookie jar, Canvas cookie, jAccount cookie, LTI token, or current-course token.
- No secret or signed source URL in frontend JSON, HTML, browser storage, query strings, or logs.
- Login success creates a new random session ID; pending IDs cannot become authenticated IDs.
- Logout removes the session, course tokens, tickets, and per-user counters immediately.
- Expired sessions cannot mint or consume tickets. Session expiry invalidates all owned tickets.
- A ticket is random, single-purpose, short-lived, and bound to session, user, course, video, track,
  registered source, expiry, and nonce. User A cannot consume user B's ticket.
- Browser input can never select a proxy URL, host, Referer, or filename.
- Source scheme is HTTPS unless a separately reviewed exception is documented.
- Redirect and DNS targets must pass the same allowlist and prohibited-address checks as the original
  source. Rebinding to private or local addresses is rejected.
- Downloads stream; the server does not buffer or persist the full recording.

## Session lifecycle

Pending login and authenticated session are distinct records. Pending login has a five-minute maximum
age, owns its backend WebSocket cancellation handle, and is consumed once. Authenticated sessions have
an eight-hour absolute expiry plus tracked last activity. The browser cookie is `HttpOnly`, `Secure`,
`SameSite=Lax`, `Path=/`, and contains no upstream identity or token.

An authenticated session owns its upstream client/cookie store and course-scoped video authorization.
Course authorization is replaced only for that same course and user. Switching courses cannot mutate
another session or silently reuse the previous course token.

## Request protections

- Mutating endpoints require same-origin checks and a CSRF token tied to the site session.
- CORS is disabled unless an explicit same-origin deployment requirement changes.
- Login and ticket endpoints receive per-IP and per-session rate limits.
- Request bodies have small explicit limits; IDs and Range syntax are validated at the boundary.
- API errors use stable public codes and Chinese messages; internal causes remain in redacted logs.
- Production responses set CSP, `frame-ancestors 'none'`, Referrer Policy, nosniff, and no-store where
  authentication or download data is involved.

## Download resource controls

Global and per-user semaphores enforce four total downloads and one per user by default. Permit
acquisition occurs before contacting the source and is released on completion, upstream error, browser
cancellation, or task cancellation. API calls use finite connect and total deadlines; a video stream
uses connect/header/idle controls rather than a short whole-body timeout.

Only approved end-to-end response headers are forwarded. Hop-by-hop headers, upstream cookies,
authentication headers, server implementation headers, and arbitrary cache headers are discarded.
The service sets safe attachment disposition, `Cache-Control: private, no-store`, and nosniff.

## Logging and audit

Allowed fields: keyed session hash, stable user ID, course/video IDs, status, duration, byte count, and
error class. Disallowed fields: cookies, authorization headers, token values, `tokenId`, QR signatures,
full URLs/queries, raw HTML/JSON responses, legal names, and unnecessary profile data.

Redaction is applied before formatting a log event. Debug mode cannot bypass it. The first release does
not require persistent audit storage; if SQLite is later justified, it stores only the allowed minimal
fields and never authentication state.

## Phase 0 limitations

Only loopback configuration validation and a health route exist. Session, CSRF, headers, rate limits,
SSRF checks, ticketing, and streaming are design requirements for Phase 3, not completed controls.
No real upstream behavior has been validated, and the candidate host list in the reference analysis is
not yet a production allowlist.
