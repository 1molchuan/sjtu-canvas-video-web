# Ubuntu public production acceptance

只允许用户本人在授权账号下执行。记录状态仅使用 `passed`、`failed`、
`blocked`、`not_run`，不得保存 HAR、trace、二维码截图、Cookie dump、ticket、
handle、真实课程/视频 ID 或完整 URL。

## Recorded state: 2026-07-17

部署使用精确源码 SHA `78772d7b54b81f889ff2976f2d2fb89ea4a4a538`。
服务器位于阿里云中国内地。外部 TCP 443 可以建立连接，但 TLS 随后被云网络
层重置；同一测试期间 Ubuntu 入站抓包未观察到对应流量。Ubuntu 上 UFW 未启用、
INPUT policy 为 ACCEPT，Caddy 在 loopback 上使用匹配域名的有效证书返回 200。
该现象与阿里云未备案域名阻断规则一致。维护者随后要求移除公开 DNS，因此
公网产品链路不能继续验收。

### Build and local origin result

| Item | Status | Sanitized evidence |
| --- | --- | --- |
| exact source transfer and SHA | passed | current release 与部署提交一致 |
| frontend install/typecheck/lint/unit/build | passed | 生产 dist 已安装，无 source map |
| Rust fmt/check/clippy/test/release | passed | Linux release 已构建并安装 |
| Ubuntu Playwright | not_run | Windows Mock E2E 不能替代 Ubuntu 结果 |
| Linux binary and release manifest | passed | x86-64 Linux；manifest 校验通过 |
| service user and config permissions | passed | 无 shell/sudo；配置 `root:canvas-video 0640` |
| systemd verify/start/hardening | passed | enabled、active；security 结果为 OK |
| Axum loopback binding | passed | 只监听 `127.0.0.1:3100`，公网 3100 不可达 |
| local API/SPA/static cache behavior | passed | JSON 404、SPA fallback、immutable/no-cache/no-store 正确 |
| local Caddy HTTP/HTTPS and certificate | passed | 308、证书校验成功、health 200 |
| app and Caddy restart recovery | passed | 两项重启后本机 HTTPS health 200 |
| release/temp video and source-map scan | passed | 视频 0，source map 0，生产配置未进入 release |
| journal secret scan | passed | 已知敏感模式匹配数 0；Caddy access log 未启用 |

### Cloudflare and public result

| Item | Status | Reason |
| --- | --- | --- |
| Cloudflare Full (strict) setting | passed | 由维护者在 Dashboard 确认 |
| `/api/` Cache Rule bypass | passed | 精确 hostname + `/api/`，动作为 Bypass cache |
| DNS A proxied | blocked | 备案条件未满足后已按维护者要求删除 |
| public HTTPS/frontend/SSE | blocked | 域名无 DNS；此前 origin TLS 在虚拟机前被重置 |
| public QR/login/Session/Cookie | blocked | 依赖公网 HTTPS |
| public courses/videos/detail | blocked | 依赖公网登录 Session |
| ticket/public Range 206/cache bypass | blocked | 不能用本机或 Phase 2 的 206 替代 |
| cancellation/permit retry | blocked | 依赖公网下载链路 |
| authorized complete small download | not_run | 未启动公网下载，不产生客户端或服务器视频文件 |
| logout/ticket invalidation | blocked | 公网 Session 未建立 |
| public recovery after restart | blocked | 本机重启恢复已通过，公网 DNS 已移除 |

因此不得创建 `phase-3-ubuntu-public-complete` 标签。恢复公网前，必须完成 ICP
备案和阿里云接入备案，或将 origin 迁移到非中国内地；随后从 Secure Cookie、
QR/SSE 到公网 Range、Cloudflare 不缓存、取消和登出重新执行完整验收。

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
