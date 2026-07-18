# Web API

This document describes the same-origin Axum API implemented in Phase 2 and consumed by the Phase 3 React frontend. All opaque identifiers are
random, memory-only handles. They are not Canvas course IDs, video-system IDs, track URLs, or tokens.

## General rules

- The API is same-origin only. No CORS layer is installed.
- The browser must send cookies on every authenticated request.
- JSON errors have the form below. Internal causes and upstream response bodies are never returned.
- Every response includes a request ID and the global security headers.
- `POST` requests must have an exact configured `Origin`. Ticket creation and authenticated logout
  also require the session CSRF token in the configured header, normally `X-CSRF-Token`.
- Request bodies are capped by `security.max_request_body_bytes`.
- Unknown `/api` routes return a structured JSON `404`; they never fall back to React `index.html`.
- Ordinary API responses use `Cache-Control: no-store`; downloads use `private, no-store`.

```json
{
  "error": {
    "code": "SESSION_EXPIRED",
    "message": "登录状态已过期，请重新扫码登录。",
    "request_id": "random-request-id"
  }
}
```

## Health

### `GET /api/health`

Returns `200`:

```json
{"status":"ok"}
```

This route proves only that the process and router are available. It does not call SJTU services.

## QR login

### `POST /api/auth/qr/start`

Requires an exact configured `Origin`; cross-site `Sec-Fetch-Site` is rejected. The direct peer address
is rate-limited and proxy headers are ignored by default. A successful response creates a temporary,
`HttpOnly` pending cookie and returns:

The request body is optional. Ordinary allowlisted login uses an empty body. A browser opened through a
one-time invitation submits:

```json
{"invite_token":"43-character-base64url-token"}
```

The invitation is reserved for this pending login and consumed only after a stable identity is verified.
Invalid invitations return `403`; expired, consumed, or currently reserved invitations return `410`.
The invitation URL uses `/login#invite=...`, so the raw token is never part of the HTTP request target or
reverse-proxy access path. The frontend removes the fragment before starting login.

```json
{
  "pending_id": "opaque-random-value",
  "events_url": "/api/auth/qr/events/opaque-random-value",
  "expires_in_seconds": 300
}
```

The pending ID has 256 bits of randomness and contains no jAccount UUID. The pending cookie carries a
separate browser binding. Both are required to subscribe or claim the completed login.

### `GET /api/auth/qr/events/:pending_id`

Returns `text/event-stream`. Each SSE `data` field is one JSON value:

```json
{"type":"started"}
{"type":"qr","url":"signed-url-used-only-to-render-a-local-qr"}
{"type":"scanned"}
{"type":"authenticating"}
{"type":"authenticated"}
{"type":"rejected"}
{"type":"expired"}
{"type":"error","code":"SAFE_CODE","message":"安全的用户提示"}
```

The signed QR URL is intentionally exposed only to the bound browser. It must not be logged, stored, or
loaded as a remote image; the frontend should render the QR locally. Reconnecting replays bounded event
history without duplicating already observed state. Closing SSE does not cancel the backend login.

After `authenticated`, call `GET /api/auth/session` with the same pending cookie. SSE itself cannot set
the authenticated website cookie. This call atomically claims the completed pending login, rotates to a
new Session ID, sets the formal Cookie, and returns the in-memory CSRF token.

## Website session

### `GET /api/auth/session`

With no active or completed pending login:

```json
{"authenticated":false}
```

When a bound pending login has completed, this call atomically consumes it, creates a fresh website
session, sets the configured session cookie, and clears the pending cookie. With an active session:

```json
{
  "authenticated": true,
  "user": {
    "display_label": "已登录用户",
    "identity_source": "canvas"
  },
  "csrf_token": "session-bound-random-token",
  "expires_at": "2030-01-01T00:00:00Z",
  "download_delivery": "native_navigation"
}
```

`download_delivery` is either `native_navigation` for the server proxy or `direct_stream` for the
explicit direct experiment. It contains no resource identifier. The response never contains a stable
ID, account, name, upstream cookie, LTI token, video token, or source URL. An expired session returns
`401 SESSION_EXPIRED`.

### `GET /api/me`

Requires an active session. Returns only `display_label`, `identity_source`, and `expires_at`.

### `POST /api/auth/logout`

Requires exact Origin and a valid CSRF header while a session is active. Returns `204`, removes the
session and every owned ticket, cancels active session streams, clears both website cookies, and
destroys the in-memory upstream client state. Repeating logout without a session is idempotent.

## Courses and videos

### `GET /api/courses`

Discovers courses using the authenticated Canvas Cookie session and returns session-bound handles:

```json
{
  "courses": [
    {
      "id": "course-handle",
      "name": "课程显示名称",
      "course_code": null,
      "term_name": null
    }
  ]
}
```

The server does not accept a raw Canvas course ID in any Web API. Course handles expire after the
session-local resource TTL and cannot cross sessions.

### `GET /api/courses/:course_handle/videos`

Validates the course handle, performs the course-bound LTI flow as needed, and returns:

```json
{
  "videos": [
    {"id":"video-handle","name":"录像名称","started_at":null}
  ]
}
```

Video handles are bound to the current course and website session. A course token expiry triggers at
most one validated LTI refresh.

### `GET /api/courses/:course_handle/videos/:video_handle`

Validates both parent handles, obtains current video details, and registers track handles:

```json
{
  "video": {
    "id": "video-handle",
    "name": "录像名称",
    "tracks": [
      {
        "id": "track-handle",
        "kind": "screen",
        "suggested_filename": "safe-name.mp4"
      }
    ]
  }
}
```

No real video ID, course ID, upstream host, URL, path, query, Referer, or token is returned.

### `GET /api/courses/:course_handle/videos/:video_handle/subtitle`

Validates the session-bound course and video handles, refreshes the course-bound video authorization at
most once, fetches the original recognition cues, and returns a UTF-8 SRT attachment. The upstream
subtitle token and recording `courId` never enter the browser. The response uses
`Content-Type: application/x-subrip; charset=utf-8` and `Cache-Control: private, no-store`.

If the video platform has not generated a subtitle, the API returns an explicit
`404 SUBTITLE_NOT_FOUND` JSON error instead of an empty file. Subtitle text is held only for the request
and is not written to disk or retained in the website session.

## Download ticket

### `POST /api/courses/:course_handle/videos/:video_handle/tracks/:track_handle/ticket`

Requires an active session, exact Origin, and CSRF token. Every parent-child handle relationship is
checked before a ticket is issued:

```json
{
  "download_url": "/api/download/random-ticket",
  "expires_in_seconds": 60
}
```

The random ticket is stored only in server memory and contains no encoded URL. It is bound to the
current session and exact registered track. Within its TTL, the same session may reuse it for multiple
Range requests. Logout or session expiry invalidates it immediately.

## Streaming download

### `GET /api/download/:ticket`

Requires the website session cookie. The optional `Range` header accepts one byte range only:

- `bytes=0-1023`
- `bytes=1024-`
- `bytes=-1024`

Malformed, oversized, non-byte, overflowing, or multiple ranges return `416` with a structured error.
The server forwards a validated single range and supports upstream `200`, `206`, and `416`. If a Range
request receives upstream `200`, the API returns `502 UPSTREAM_REJECTED_RANGE` rather than silently
serving the whole recording. An upstream `416` preserves safe range metadata but discards the upstream
error body and advertises a zero-length response.

### `HEAD /api/download/:ticket`

Performs the same session, ticket, semaphore, URL, redirect, and upstream status checks but returns an
empty body with the approved metadata headers.

Download responses can forward only `Content-Type`, `Content-Length`, `Content-Range`, `Accept-Ranges`,
`Last-Modified`, and `ETag`. The service sets a sanitized attachment filename, no-store, and nosniff.
Upstream `Set-Cookie`, hop-by-hop headers, server headers, and arbitrary cache headers are discarded.

The default limit is one active stream per user and four globally. Capacity exhaustion returns `429`
with `Retry-After: 5`; the server does not form an unbounded wait queue.

## Important status codes

| Status | Meaning |
| --- | --- |
| `401` | Missing, invalid, or expired website session |
| `403` | Origin/CSRF rejection or ticket/session mismatch |
| `404` | Pending, course, video, track, or ticket handle is invalid |
| `410` | Download ticket expired |
| `413` | Request body exceeds configured limit |
| `416` | Range is invalid, multiple, or rejected by the upstream source |
| `429` | QR-start or download concurrency limit reached |
| `502` | Authenticated SJTU upstream operation failed safely |
| `500` | Internal error with no implementation detail in the response |
