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
4. Ubuntu ↔ Caddy: Axum binds to loopback; only Caddy exposes 80/443 and terminates origin TLS.
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

一次性邀请是可选的登记边界，不替代 jAccount 身份验证。原始邀请令牌只在维护者的管理命令输出、浏览器 URL fragment 和扫码启动 POST body 中短暂存在；数据库只保存令牌 SHA-256 哈希。登录获得稳定身份后，服务端原子消费邀请并持久化规范化稳定身份哈希。动态白名单数据库不包含姓名、Cookie、上游 token 或课程数据。

邀请预留绑定 pending login，避免同一链接并发登记多人。失败的协议流程释放预留；成功、过期或正在使用的链接不能再次开始登录。动态用户撤销阻止后续 Session，但不主动终止已经建立的内存 Session。

Pending login and authenticated session are distinct records. Pending login has a five-minute maximum
age, owns its backend WebSocket cancellation handle, and is consumed once. Authenticated sessions have
an eight-hour absolute expiry; Phase 2 does not implement an idle timeout. The production browser cookie
is `HttpOnly`, `Secure`, `SameSite=Lax`, `Path=/`, has no Domain, and contains no upstream identity or
token.

An authenticated session owns its upstream client/cookie store and course-scoped video authorization.
Course authorization is replaced only for that same course and user. Switching courses cannot mutate
another session or silently reuse the previous course token.

## Request protections

- Mutating endpoints require same-origin checks and a CSRF token tied to the site session.
- CORS is disabled unless an explicit same-origin deployment requirement changes.
- QR start is rate-limited by direct peer address, and pending capacity is globally bounded. Download
  streams are bounded by per-user and global semaphores.
- Request bodies have small explicit limits; IDs and Range syntax are validated at the boundary.
- API errors use stable public codes and Chinese messages; internal causes remain in redacted logs.
- Production responses set CSP, `frame-ancestors 'none'`, Referrer Policy, nosniff, and no-store where
  authentication or download data is involved.

## Download resource controls

Global and per-user semaphores enforce four total downloads and one per user by default. Permit
acquisition occurs before contacting the source and is released on completion, upstream error, browser
cancellation, or task cancellation. API calls use finite connect and total deadlines. A video stream
uses a finite connect timeout and intentionally has no short whole-body timeout.

Only approved end-to-end response headers are forwarded. Hop-by-hop headers, upstream cookies,
authentication headers, server implementation headers, and arbitrary cache headers are discarded.
The service sets safe attachment disposition, `Cache-Control: private, no-store`, and nosniff.

## Logging and audit

Allowed fields: request ID, method, route template, status, duration, byte count, completion outcome, and
error class. Disallowed fields: raw session/pending/ticket IDs, stable IDs, course/video identifiers,
filenames, cookies, authorization headers, token values, `tokenId`, QR signatures, full URLs/queries,
raw HTML/JSON responses, legal names, and unnecessary profile data.

邀请 URL fragment 和原始邀请令牌同样禁止进入日志。只有管理 CLI 的 `create` 命令会把原始邀请链接输出给维护者。

Redaction is applied before formatting a log event. Debug mode cannot bypass it. The first release does
not require persistent audit storage; if SQLite is later justified, it stores only the allowed minimal
fields and never authentication state.

## Historical limitations at the end of Phase 0

At the Phase 0 baseline, only loopback configuration validation and a health route existed. Session,
CSRF, headers, rate limits, ticketing, and streaming were then unimplemented design requirements. This
paragraph is historical; Phase 1.5 later validated the real upstream chain and Phase 2 implemented the
Web boundary described below.

## Phase 1 implemented controls

- Each CLI run constructs an independent `ProtocolContext`, Cookie Store, redirect-disabled client,
  and stateless video-content client. No Cookie Store or course token is global.
- A potentially broad upstream `JAAuthCookie` is removed before host-only copies are installed for
  jAccount and mySJTU. Mock tests prove it is absent on Canvas and video-content requests.
- Canvas SSO follows only exact Canvas, jAccount, and mySJTU origins. The OIDC POST uses a
  redirect-disabled client and follows at most eight individually validated redirects on the exact
  Canvas origin. LTI form actions and the final token-bearing Location remain exact Video API origins.
  Range redirects must remain VideoContent origins.
- Production URL policy requires the secure scheme, exact host, no userinfo, no IP literal, and the
  standard HTTPS port. Video DNS results are rejected if any result is loopback, private, link-local,
  multicast, unspecified, carrier-grade NAT, benchmarking, mapped-private IPv4, or reserved.
- Video list/detail requests use a course-bound `SecretString`; only the explicit 401/403 token-expiry
  branch performs one LTI relaunch. A second failure is surfaced.
- Range probes use a Cookie-free client with `Range: bytes=0-0`, `Accept-Encoding: identity`, and the
  source-derived Referer. They inspect headers without writing or caching the response body.
- CLI QR output is a terminal matrix, not a textual signed URL. Stable identity and video IDs are
  hashed in diagnostic output; source paths and queries are replaced by a keyed path hash.
- Real requests are disabled unless the operator explicitly sets `SJTU_REAL_PROTOCOL_TEST=1`.
  `.local/protocol-report.json` contains only step states, Go/No-Go, video host, and Range support.

These were protocol-validation controls at the Phase 1 baseline. Phase 2 now adds the browser session,
CSRF, ticket, and streaming boundary; there is still no production UI.

## Phase 1.5 real-environment evidence

On 2026-07-17, an explicitly gated run using the user's own authorized account confirmed the backend
QR flow, Canvas SSO, stable Canvas identity source, Cookie-only course discovery, bounded OIDC/LTI
flow, course-bound video token, video details, and an HTTPS source on `live.sjtu.edu.cn`. The probe sent
only `Range: bytes=0-0`, consumed no full recording, and wrote no video data.

The run exposed two safe compatibility differences: duplicate copies of the same UUID in the UUID
page, and missing display fields in some authorized Canvas course entries. It also confirmed that OIDC
uses a Canvas redirect chain. Regression tests preserve strict distinct-UUID rejection, tolerate only
missing display metadata, validate every redirect host, reject redirect loops after eight hops, and
reject cross-purpose hosts.

The generated `.local/protocol-report.json` contains step statuses, Go/No-Go, source host, and Range
support only. It is ignored by Git and was scanned for forbidden identity, Cookie, token, course,
video, path, and query fields. This evidence covers one account, one authorized course, and one
selected track; Phase 2 still had to implement and test the per-user browser boundary independently.

## Phase 2 implemented Web controls

- A QR pending record has a 256-bit random ID plus a separate `HttpOnly` browser-binding cookie. The
  SSE route requires both. Login completion is claimed once and rotates to a different 256-bit website
  session ID.
- Whitelist comparison uses NFC-normalized, case-sensitive stable identity or its canonical `sha256:`
  digest. Display name is never an authorization field. Rejected identities retain no Cookie Store.
- Every website session owns its independent `ProtocolContext`, Cookie Store, course authorization,
  CSRF secret, resource registry, semaphores, and revocation token. Network calls do not hold a global
  store write lock.
- Production cookie validation enforces Secure `__Host-` semantics. An insecure non-Host cookie is
  accepted only for an exact loopback HTTP origin. CORS is absent; state-changing routes verify exact
  Origin, reject cross-site fetch metadata, and compare session-bound CSRF tokens in constant time.
- Course, video, and track identifiers exposed to the browser are random, session-local handles with
  parent binding and a five-minute TTL. A browser cannot submit raw upstream course/video identifiers.
- Download tickets are random server-side records bound to session and exact track. They contain no
  encoded URL, expire after 60 seconds by default, and are removed on logout or session expiry.
- The streaming client is Cookie-free and redirect-disabled. It revalidates the registered resource and
  up to three manual redirects, sends the validated Referer, accepts only one parsed byte range, and
  supports upstream 200/206/416 without buffering a full body.
- Only six approved response metadata headers are forwarded. Upstream `Set-Cookie`, hop-by-hop headers,
  implementation headers, and source `Content-Disposition` are discarded. Attachment names are rebuilt
  from sanitized track metadata.
- Cleanup cancels expired pending work, sessions, tickets, and streams. Graceful shutdown stops new
  requests, revokes in-memory state, waits for the configured grace period, and persists no secret.
- The production server binary refuses startup unless `SJTU_REAL_PROTOCOL_TEST` is exactly `1`. Unit and
  Mock Router tests do not need or bypass this binary startup gate.

## Residual risks and evidence limits

Phase 2's DNS safety check and reqwest's connection resolution are separate operations, leaving a
residual DNS time-of-check/time-of-use rebinding window. Exact purpose-specific host allowlists and
revalidation of every redirect constrain this risk but do not eliminate it cryptographically.

The Phase 1.5 real evidence covers one account, one authorized course, and one selected track. Phase 2
Mock tests cover multi-session isolation and adverse Web branches. Real Phase 2 browser/API acceptance
is recorded separately and must not be inferred from Mock results.

## Phase 2 real Web security evidence

On 2026-07-17, the user completed a separate explicitly gated Web run against a listener confirmed as
`127.0.0.1`. The browser-equivalent client retained both pending and website cookies only in memory. It
verified pending-bound SSE, allowlist acceptance, rotated Session creation, CSRF ticket issue, opaque
resource handles, ticketed HEAD/Range, response header filtering, cancellation permit release, logout,
and immediate old-ticket rejection.

The source returned `206` with one byte for `bytes=0-0`. A second bounded request was cancelled after
2920 bytes; an immediate one-byte request succeeded, and no video artifact appeared in the workspace.
The real response contained no upstream `Set-Cookie`.

Log canary checks found no actual QR, pending, CSRF, ticket, course/video/track handle, URL, Cookie, or
authorization value. They did find the public endpoint template name `getAccessTokenByTokenId`. Although
no value was present, Phase 2 replaced that log-only label with `<video-api-token-exchange>` and added a
regression test so the sensitive parameter name no longer appears in future logs.

## Phase 3 browser and edge controls

- The browser holds CSRF, QR URL, pending ID, handles and ticket only in memory. No authentication or
  course response is written to localStorage, sessionStorage, IndexedDB or a Service Worker cache.
- QR rendering is local. No remote QR image, external font, icon CDN, analytics or third-party script is
  loaded.
- Proxy delivery uses a temporary native anchor. Explicit direct delivery uses the File System Access
  API and pipes the cross-origin response stream into a user-selected file. Neither path creates a Blob
  or buffers the complete video in JavaScript; direct delivery exposes the short-lived source URL to the
  browser and requires the source's CORS policy.
- Axum serves the built SPA and API from one origin. `/api` and `/api/*` have an explicit JSON 404 and
  cannot reach the SPA fallback; missing assets also remain 404.
- `index.html` is `no-cache`, hashed assets are immutable, ordinary API responses are `no-store`, and
  downloads remain `private, no-store`.
- Production validation requires loopback bind, HTTPS public origin, an absolute existing frontend
  distribution, Secure `__Host-` Cookie semantics, and a non-example allowlist.
- The Cloudflare rule must bypass cache for `/api/*`. Origin headers do not replace a public-domain check
  of `206`, `Content-Range` and non-HIT cache status.

## Phase 3.5 Ubuntu production boundary

- The recommended origin is Ubuntu with systemd and Caddy; Cloudflare Tunnel is not used.
- Axum remains bound to `127.0.0.1:3100`; Caddy alone accepts public HTTP/HTTPS.
- Caddy does not serve files, cache, encode, or access-log capability-bearing URIs.
- `/api/` is bypassed at the Cloudflare cache layer in addition to origin no-store headers.
- Releases are root-owned, immutable directories selected through an atomic symlink.
- The service account has no shell, home, sudo, write access to releases, or Linux capabilities.
- Production config is outside releases and readable only by root and the service group.
- Restart intentionally destroys all sessions, upstream cookies, course tokens, and tickets.
- Cloudflare credentials are absent from application config, releases, systemd, Caddy and Git.

Phase 3.5 does not widen trust in proxy headers: `trust_proxy_headers` remains false. The direct peer
seen by Axum is Caddy on loopback; security decisions continue to use Session, pending binding, Origin,
CSRF and server-side authorization rather than client-supplied forwarding headers.
