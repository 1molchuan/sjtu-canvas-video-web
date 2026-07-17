# Ubuntu public production acceptance

只允许用户本人在授权账号下执行。记录状态仅使用 `passed`、`failed`、
`blocked`、`not_run`，不得保存 HAR、trace、二维码截图、Cookie dump、ticket、
handle、真实课程/视频 ID 或完整 URL。

## Build and origin

- [ ] exact source SHA verified: ____
- [ ] frontend install/typecheck/lint/unit/build: ____
- [ ] Rust fmt/check/clippy/test/release: ____
- [ ] Linux binary architecture and `ldd`: ____
- [ ] release manifest: ____
- [ ] system user and config permissions: ____
- [ ] systemd verify/start/hardening: ____
- [ ] Axum `127.0.0.1:3100` only: ____
- [ ] local health and SPA/API routing: ____
- [ ] no source map/video/config in release: ____

## Caddy and Cloudflare

- [ ] Caddy validates and runs: ____
- [ ] HTTP redirects to HTTPS: ____
- [ ] valid origin certificate: ____
- [ ] Cloudflare DNS A proxied: ____
- [ ] Full (strict): ____
- [ ] `/api/` Cache Rule bypass: ____
- [ ] public frontend/CSP/API JSON 404: ____
- [ ] public SSE is event-stream/no-store and stays alive: ____

## Login and browsing

- [ ] public QR start and local QR SVG: ____
- [ ] public SSE state flow: ____
- [ ] user scan and whitelist: ____
- [ ] Secure `__Host-` Cookie attributes: ____
- [ ] Session restores after refresh: ____
- [ ] course list: ____
- [ ] successful course videos: ____
- [ ] known 502 course remains an error: ____
- [ ] video detail and unknown track labels: ____
- [ ] opaque handles are not displayed: ____

## Download

- [ ] ticket issued without being displayed/logged: ____
- [ ] public `Range: bytes=0-0` returns 206: ____
- [ ] Content-Range/Length/Accept-Ranges preserved: ____
- [ ] Cache-Control is `private, no-store`: ____
- [ ] CF cache is absent/BYPASS/DYNAMIC, never HIT: ____
- [ ] client cancellation releases permit: ____
- [ ] immediate retry returns 206: ____
- [ ] authorized small complete download: ____
- [ ] client file opens, then is deleted: ____
- [ ] server disk and video-file scan unchanged: ____

## Logout and restart

- [ ] logout invalidates Session and old ticket: ____
- [ ] app restart invalidates old Session: ____
- [ ] new QR login works after app restart: ____
- [ ] Caddy restart restores public site: ____
- [ ] no Ubuntu host reboot was required: ____

## Security evidence

- [ ] public listeners are only SSH/80/443: ____
- [ ] 3100 is not exposed by cloud firewall/UFW: ____
- [ ] service account has no shell/sudo: ____
- [ ] Caddy access log is disabled: ____
- [ ] journals contain no raw secrets or capability IDs: ____
- [ ] release is root-owned and immutable to service user: ____
- [ ] no development/Playwright/Mock server remains: ____
- [ ] production config and acceptance report are not tracked: ____
