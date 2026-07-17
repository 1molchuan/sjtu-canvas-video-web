# Web session model

## Ownership hierarchy

Each successful QR login creates one new `UserSession`. The store holds an `Arc<UserSession>`, but the
session owns exactly one `ProtocolContext`; cloning the `Arc` never clones or shares a Cookie Store with
another session.

```text
AppState
├── PendingLoginStore
│   └── PendingLogin → browser binding + events + cancellation
├── SessionStore
│   └── UserSession
│       ├── independent ProtocolContext
│       │   ├── reqwest client
│       │   ├── in-memory Cookie Store
│       │   └── stateless streaming client
│       ├── identity and CSRF secret
│       ├── per-user protocol/download semaphores
│       ├── session-local course/video/track registry
│       └── revocation token
└── DownloadTicketStore
    └── ticket → session + exact registered track
```

There is no global Cookie Jar or global course token. `canvas-core` keeps video authorization bound to
the relevant Canvas course within the owning `ProtocolContext`.

## Login transition

1. `POST /api/auth/qr/start` creates a random pending ID and an independent browser-binding secret.
2. The browser receives only an `HttpOnly` pending cookie and the opaque pending ID.
3. A background task runs jAccount QR, Canvas SSO, stable identity discovery, and whitelist validation.
4. SSE reports public state only; it never exposes cookies, UUID, stable ID, or token material.
5. An allowlisted successful login stores the authenticated `ProtocolContext` temporarily in the
   pending record.
6. The bound browser calls `GET /api/auth/session`; the completed pending value is consumed once.
7. A fresh 256-bit website session ID and CSRF secret are generated. The pending ID is never promoted.
8. The session cookie is set and the pending cookie is deleted.

This claim step provides session-fixation resistance. A stolen pending ID alone is insufficient because
the separate pending cookie binding is also required.

## Session cookie

The production example uses `__Host-sjtu_canvas_video_session` with `Secure`, `HttpOnly`,
`SameSite=Lax`, `Path=/`, and no Domain. Configuration validation rejects a non-loopback origin with an
insecure cookie and enforces host-cookie invariants. Loopback HTTP development must use a non-`__Host-`
name with `secure = false`; this exception is accepted only for a loopback public origin.

The cookie contains only the random site session ID. It contains no upstream cookie, identity, course,
video, or authorization data.

## Lifetime and cleanup

- Pending login absolute TTL: five minutes by default.
- Completed but unclaimed pending retention: 30 seconds.
- Authenticated session absolute TTL: eight hours by default.
- Session-local course/video/track handle TTL: five minutes.
- Download ticket TTL: 60 seconds by default.
- Cleanup scan: every 30 seconds.

Phase 2 implements absolute session expiry, not an idle-session timeout. Cleanup cancels expired pending
tasks, revokes expired sessions, removes their tickets, and drops all associated protocol state. Process
restart also drops every session because no authentication state is persisted.

## Concurrency and locking

Concurrent stores use short-lived map access. Network requests are never performed while holding a
global store write lock. Each session has a protocol semaphore to serialize mutable course-authorization
work and a separate download semaphore. Download body ownership holds both global and per-user permits
until end-of-stream, error, cancellation, or body drop.

## Logout and shutdown

Authenticated logout verifies Origin and CSRF, revokes/removes the session, removes every owned ticket,
cancels active stream bodies, clears cookies, and drops the upstream Cookie Store. Repeated logout after
the session is gone remains a `204` operation.

Graceful shutdown stops accepting requests, cancels pending work and streams, clears tickets and
sessions, waits up to the configured grace period, and persists no Cookie or token.

## Public identity surface

Whitelist matching uses a normalized stable identity or its `sha256:` digest. Display name is never an
authorization key. Browser responses use only the fixed label `已登录用户` and the coarse identity
source. Raw stable IDs and their deterministic hashes are not exposed through the Web API.
