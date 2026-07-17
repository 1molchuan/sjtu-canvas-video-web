# Download proxy security

## Data path and non-goals

The only supported path is:

```text
authorized video detail
→ session-local registered track
→ session-bound random ticket
→ validated streaming request
→ browser
```

The API has no arbitrary-URL endpoint. A browser cannot supply a source URL, Referer, upstream host,
filename, Canvas course ID, video-system ID, or authorization token. The server never writes a complete
or partial video to disk and does not cache the response body.

## Ticket binding

A ticket is a 256-bit random ID mapped in memory to:

- website session ID;
- course, video, and track handles with their parent relationship;
- one `ValidatedUpstreamResource` created from an authenticated video-detail response;
- sanitized suggested filename;
- creation and absolute expiry times.

The ticket value does not encode the source URL. Possession of a ticket without the matching website
session cookie cannot download. Tickets remain reusable by that same session during their short TTL so
browsers can issue several Range requests; they do not slide or extend.

## URL and redirect validation

Video resources are accepted only through `canvas-core`'s purpose-specific exact-host policy. Validation
requires HTTPS, standard port, no user information, and no IP literal. DNS answers are rejected if any
address is loopback, private, link-local, multicast, unspecified, carrier-grade NAT, benchmarking,
mapped-private IPv4, or reserved.

The download client has automatic redirects disabled. At most three redirect responses are processed
manually. Every Location is resolved with a URL parser, revalidated for the video-content purpose, and
checked for loops before the next request. A redirect cannot cross into a broader SJTU suffix or another
upstream purpose.

The current DNS check and reqwest connection resolution are separate operations. This leaves a residual
DNS time-of-check/time-of-use rebinding window; the exact host allowlist and rejection of all unsafe DNS
answers reduce, but do not cryptographically eliminate, that risk.

## Range handling

The parser accepts one bounded ASCII header of at most 128 bytes using the `bytes` unit:

- closed: `bytes=start-end`;
- open: `bytes=start-`;
- suffix: `bytes=-length`.

It rejects empty values, non-decimal values, reversed ranges, zero suffixes, integer overflow, unknown
units, and comma-separated multiple ranges. Invalid input returns a structured `416`; it is never copied
blindly to reqwest.

The upstream request includes the validated Phase 1.5 Referer and `Accept-Encoding: identity`. The
server accepts upstream `200`, `206`, and `416`. A GET carrying Range must receive `206` or `416`; an
upstream `200` is treated as `UPSTREAM_REJECTED_RANGE` to prevent an accidental full response.
For `416`, the service keeps the status and safe `Content-Range`, discards the upstream error body, and
uses a zero-length browser response.

## Streaming and cancellation

Global and per-user permits are acquired with `try_acquire_owned` before opening the upstream request.
There is no unbounded waiter queue. Both permits move into the response body and are released when the
body completes, the source fails, the browser drops the body, the session is revoked, or the server
begins shutdown.

Ordinary protocol calls retain finite connect and total request deadlines. The streaming client uses a
finite connect timeout but deliberately has no short whole-body timeout, so large authorized downloads
can complete. Dropping the Axum body drops the reqwest byte stream and closes the upstream transfer.

## Header boundary

Only these upstream response headers may cross into the browser response:

- `Content-Type`
- `Content-Length`
- `Content-Range`
- `Accept-Ranges`
- `Last-Modified`
- `ETag`

The server sets `Content-Disposition`, `Cache-Control: private, no-store`, and
`X-Content-Type-Options: nosniff`. It never forwards upstream `Set-Cookie`, `Connection`,
`Keep-Alive`, proxy authentication headers, `TE`, `Trailer`, `Transfer-Encoding`, `Upgrade`, `Server`,
`Via`, `Alt-Svc`, or arbitrary cache headers.

## Filename handling

The service ignores upstream `Content-Disposition`. The registered track filename is normalized by
removing CR/LF, path separators, control characters, and unsafe punctuation; length is bounded and an
empty result falls back to a neutral video filename. A missing or dangerous extension such as `.html`
is replaced with `.mp4`. The response emits an ASCII `filename` plus RFC 5987 UTF-8 `filename*`.

## Logging boundary

Request logs use the route template `/api/download/:ticket`, never the actual URI. Stream-completion
logs contain request ID, static route template, byte count, and completion/error/cancellation outcome.
They omit raw session IDs, ticket IDs, URLs, paths, queries, filenames, cookies, and tokens.

## Test evidence

Mock Web tests cover GET `200`, GET `206`, upstream `416`, HEAD, closed/open/suffix ranges, malformed
and multiple ranges, header filtering, filename injection, malicious redirects, ticket/session mismatch,
ticket expiry, logout invalidation, global/per-user limits, permit release after errors and body drop,
source interruption, shutdown cancellation, and a delayed stream proving no whole-body buffering or
short total timeout. Mock requests never contact SJTU services.
