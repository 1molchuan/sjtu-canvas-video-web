# Experimental direct downloads

The default `download_delivery = "proxy"` keeps the Phase 2 security boundary: the browser receives
only a session-bound ticket and Axum streams the validated upstream resource.

`download_delivery = "redirect_experimental"` is an explicit bandwidth experiment. Ticket creation,
website Session validation, course/video/track ownership checks, TTL enforcement, and SSRF validation
remain on the server. A valid GET or HEAD then receives `307 Temporary Redirect` with
`Cache-Control: private, no-store` and `Referrer-Policy: no-referrer`; the browser requests the video
source directly and the video body does not traverse the deployment server.

This mode changes the confidentiality boundary:

- the short-lived upstream URL becomes visible to the browser and intermediaries handling the redirect;
- the deployment cannot enforce upstream response headers, filenames, Range behavior, cancellation, or
  per-user/global streaming concurrency after the redirect;
- direct access can fail when the video source requires the Canvas Referer;
- the upstream URL lifetime and replay behavior have not yet been established.

The setting is therefore opt-in, is never selected automatically, and has no silent proxy fallback.
Disable it by restoring `download_delivery = "proxy"` and restarting the in-memory service. A restart
invalidates website Sessions and tickets.

The protocol CLI supports a one-byte compatibility probe:

```powershell
$env:SJTU_REAL_PROTOCOL_TEST = "1"
cargo run -p protocol-cli -- --no-course-discovery inspect-course --course-id YOUR_OWN_COURSE_ID --probe-direct
```

The probe uses the stateless client, sends `Range: bytes=0-0`, omits Referer, follows only validated
video-content redirects, reads no complete video, and never prints the source URL.
