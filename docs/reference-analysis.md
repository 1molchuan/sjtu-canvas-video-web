# SJTU-Canvas-Helper reference analysis

## Evidence baseline

- Repository: [`Okabe-Rintarou-0/SJTU-Canvas-Helper`](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper)
- Pinned default-branch commit: [`b5d895af57aaa74dfd53cef80dfb64c76c023c20`](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/tree/b5d895af57aaa74dfd53cef80dfb64c76c023c20)
- Commit timestamp: 2026-07-04 08:54:02 +08:00
- Upstream version at that commit: `3.0.8`
- Upstream license: MIT, `Copyright (c) 2025 Zihong Lin`
- Analysis date: 2026-07-17

The repository was shallow-cloned and the requested Rust and React files plus their direct callers
were inspected. “Source-confirmed” below means only that the pinned source implements the stated
behavior. It does not mean the corresponding SJTU endpoint was contacted successfully.

## Architectural mismatch to preserve during the rewrite

The desktop application has one process-global `App`, one `Client`, one cookie jar, and one mutable
video token. `AppConfig` persists the Canvas token, `JAAuthCookie`, video cookies, and OAuth consumer
key to JSON. Those assumptions are visible in the
[`App` singleton](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/main.rs#L35-L40),
[`Client` fields](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/mod.rs#L17-L24),
and [`AppConfig`](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/model/mod.rs#L148-L196).

The web service must therefore reimplement protocol operations around per-user state. It must not
turn the Tauri singleton, its configuration JSON, or its local proxy into shared server state.

## jAccount QR login call chain

Source-confirmed sequence:

1. React calls Tauri `get_uuid`. Rust requests `https://my.sjtu.edu.cn/ui/appmyinfo`, reads the full
   HTML body, and extracts a UUID with a regular expression
   ([`Client::get_uuid`](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L116-L132)).
2. The React hook directly opens
   `wss://jaccount.sjtu.edu.cn/jaccount/sub/{uuid}`. Every 25 seconds it sends
   `{"type":"UPDATE_QR_CODE"}`
   ([constants and socket setup](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src/lib/hooks.tsx#L48-L52),
   [`useQRCode`](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src/lib/hooks.tsx#L454-L573)).
3. `UPDATE_QR_CODE` supplies `payload.ts` and `payload.sig`; React constructs
   `https://jaccount.sjtu.edu.cn/jaccount/confirmscancode?uuid=...&ts=...&sig=...`.
4. A `LOGIN` event causes React to call Tauri `express_login`. Rust requests
   `/jaccount/expresslogin?uuid=...` and extracts `JAAuthCookie` from its cookie jar
   ([`Client::express_login`](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L134-L146)).
5. React logs the complete cookie and writes it to persistent configuration
   ([cookie handling](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src/lib/hooks.tsx#L485-L500)).

Required web redesign:

- The backend owns the jAccount WebSocket; the browser receives sanitized SSE events.
- A pending-login ID is random, short-lived, single-use, and bound to one browser flow.
- UUID, QR signature, `JAAuthCookie`, and raw WebSocket payloads are never logged or persisted.
- The source exposes only `UPDATE_QR_CODE` and `LOGIN`; a distinct “scanned” event must not be
  invented unless Phase 1 observes a reliable upstream signal.

## Canvas login, identity, and the first Go/No-Go

`login_canvas_website` adds `JAAuthCookie` to the jAccount and mySJTU hosts, requests
`https://oc.sjtu.edu.cn/login/openid_connect`, follows redirects through the shared client, and treats
only a final `jaccount.sjtu.edu.cn` host as a login failure
([implementation](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L165-L175)).
It does not prove that a Canvas API request succeeds.

The helper's identity and course calls are separate and explicitly token-based:

- `GET /api/v1/users/self` receives `Authorization: Bearer {token}`
  ([`get_me`](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/basic.rs#L747-L750),
  [Bearer request builder](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/common.rs#L18-L52)).
- `GET /api/v1/courses?include[]=teachers&include[]=term` uses the same Bearer token
  ([`list_courses`](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/basic.rs#L651-L662)).
- `App::list_courses` reads that token from persisted configuration
  ([call site](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/app/basic.rs#L388-L397)).

`check_extra_login_status` requests `https://my.sjtu.edu.cn/api/account` but only checks for a
successful, non-empty body; it does not parse a stable user ID
([implementation](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L177-L195)).

Conclusion: the pinned source does not establish either Cookie-only Canvas course discovery or a
stable whitelist identity. Both remain real-environment Phase 1 gates. The formal frontend will not
expose a Personal Access Token field if the gate fails; it will return
`CanvasCourseDiscoveryUnavailable`.

## LTI 1.3 and course video authorization

Source-confirmed request chain:

1. `GET https://oc.sjtu.edu.cn/courses/{course_id}/external_tools/8329`.
2. Find the form whose action exactly equals
   `https://v.sjtu.edu.cn/jy-application-canvas-sjtu/oidc/login_initiations`, collecting named inputs
   ([form parser and first launch](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L226-L268)).
3. POST those fields, parse a second exact-action form for
   `.../lti3/lti3Auth/ivs`, then POST with redirects disabled
   ([two submissions](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L270-L309)).
4. Split `Location` on `?` and `&` and extract `tokenId=`
   ([redirect parsing](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L310-L331)).
5. `GET .../lti3/getAccessTokenByTokenId?tokenId=...`; read JSON `data.token` and
   `data.params.courId`
   ([token exchange](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L334-L366)).

The reference parser loses duplicate input names, assumes absolute actions, does not percent-decode
`tokenId`, and does not validate the redirect host. Earlier requests use the default redirect policy.
It also logs the complete `Location`, `tokenId`, and token-exchange body. None of those behaviors may
be retained.

The new parser must preserve all required form fields, resolve relative actions against the response
URL, validate scheme and host before each request, require an expected redirect status, and parse the
URL with a URL library. Missing fields must become structured errors, not panics or empty lists.

## Video list, details, and track model

`findVodVideoList` receives JSON `{canvasCourseId: ...}`, header `token`, and Referer
`https://v.sjtu.edu.cn/jy-application-canvas-sjtu-ui/`
([request](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L369-L406)).
The reference code stores the returned token in a single `Client.token`, panics on malformed JSON,
logs the full response, and maps missing `data` to an empty video list.

`getVodVideoInfos` posts form fields `playTypeHls=true`, `id={video_id}`, and `isAudit=true`, using the
last token stored in that client
([request](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L591-L610)).
The response `VideoInfo.videoPlayResponseVoList` contains `VideoPlayInfo` entries with an ID, source
URL, channel number, and view number
([models](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/model/mod.rs#L552-L598)).

The frontend calls these entries tracks but labels them by array position: index 0 has no suffix and
later entries are called “录屏”
([mapping](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src/page/video.tsx#L289-L302)).
The source does not prove which channel is blackboard or screen capture. Phase 1 must correlate
`cdviChannelNum`/`cdviViewNum` with observed tracks before assigning semantic labels.

The web implementation must bind each video token to both user session and Canvas course. A details
request includes `course_id` and may refresh that course's LTI authorization once on an explicit
token-expiry response; there is no unlimited retry.

## Download and Range behavior

The desktop download accepts a complete `VideoPlayInfo` from React, opens a local file, and probes the
source with `Range: bytes=0-0` plus `Referer: https://courses.sjtu.edu.cn`
([probe and download](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/client/video.rs#L456-L589)).
It derives total size from `Content-Range` or `Content-Length`, then either streams to disk or launches
parallel Range requests. This is useful protocol evidence only; the file and retry architecture is
out of scope for the web service.

The desktop playback proxy fixes the target host to `live.sjtu.edu.cn`, forwards the browser's Range
header, adds the same Referer, streams the body, and copies every upstream response header
([proxy](https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper/blob/b5d895af57aaa74dfd53cef80dfb64c76c023c20/src-tauri/src/app/basic.rs#L259-L325)).
Copying all headers includes hop-by-hop headers and is unsafe for the public web rewrite. It also logs
request/response debug representations that may contain signed query parameters.

The new download path differs deliberately:

- Browser supplies only a random, 60-second server ticket, never a URL or `VideoPlayInfo`.
- The server registry stores the source URL and verifies HTTPS, host allowlist, DNS result, redirect
  target, owning session, course, video, track, nonce, and expiry.
- The browser Range is syntactically validated and forwarded; only the approved end-to-end headers
  are copied back for `200`, `206`, and `416`.
- The body is streamed without a complete memory buffer or disk file. Cancellation releases both
  global and per-user semaphore permits.

## Hard-coded endpoints requiring Phase 1 verification

| Source assumption | Pinned-source purpose | Runtime status |
| --- | --- | --- |
| `my.sjtu.edu.cn/ui/appmyinfo` | UUID HTML | Unverified |
| `my.sjtu.edu.cn/api/account` | jAccount session probe / possible identity | Unverified |
| `jaccount.sjtu.edu.cn/jaccount/sub/{uuid}` | QR WebSocket | Unverified |
| `jaccount.sjtu.edu.cn/jaccount/expresslogin` | `JAAuthCookie` establishment | Unverified |
| `oc.sjtu.edu.cn/login/openid_connect` | Canvas Web login | Unverified |
| Canvas `external_tools/8329` | video LTI launch | Unverified |
| `v.sjtu.edu.cn/jy-application-canvas-sjtu/...` | LTI/token/list/details APIs | Unverified |
| `live.sjtu.edu.cn/vod/...` | inferred recording source host | Unverified |
| Referer `https://courses.sjtu.edu.cn` | source download requirement | Unverified |

No allowlist should be finalized solely from this table. Phase 1 records actual redirect and video
hosts without logging paths, query strings, tokens, or personal data.

## Reuse and MIT compliance

Useful protocol facts to reimplement are the request ordering, expected form purposes, response model
field names, Referer behavior, and Range probing. Tauri commands, file download, local proxy, subtitles,
PPT, AI, JBox, ffmpeg, Canvas assignments, MCP, and desktop configuration are not copied.

The new code is independently structured for multi-user Web security. Because protocol work is
derived from the MIT project and future phases may adapt small implementation portions, the original
copyright and complete MIT text are retained in `THIRD_PARTY_NOTICES.md`.

