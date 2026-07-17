# SJTU Canvas Video Web

`Canvas Video Helper` 是一个非官方、仅供受邀用户使用的私人课程录像下载工具。用户使用自己的 jAccount 扫码，服务端只代理该用户当前 Canvas 会话有权访问的录像；浏览器无需安装桌面客户端。

本项目不属于上海交通大学、Canvas 或课程录像平台，不提供公开注册、分享、在线播放或批量下载，也不会缓存、落盘或上传课程视频。

## 当前状态

Phase 1.5 已在用户本人授权环境中真实验证完整协议链：

```text
jAccount QR → express login → Canvas Cookie Session → 课程发现
→ OIDC/LTI → 视频列表 → 视频详情/轨道 → Range 探测
```

Phase 2 已真实验收浏览器后端：扫码 SSE、白名单 Session、课程/录像 API、ticket、`206` 流式代理、取消、permit 释放、登出和 ticket 失效均通过。

Phase 3 增加 React 产品界面、Axum 同源静态资源和可复现发布包。当前推荐生产路线为 Ubuntu + systemd + Caddy + Cloudflare；公网结果必须按 [Ubuntu 生产验收清单](docs/production-ubuntu-acceptance.md) 真实记录，本地 `206` 不能替代 Cloudflare 下的 Range 与不缓存验证。

## 架构

```text
浏览器
  │ HTTPS、同源 API、HttpOnly Session Cookie
Cloudflare HTTPS
  │
Caddy Ubuntu :443
  │ HTTP loopback
Axum 127.0.0.1:3100
  ├── React 静态资源
  ├── 网站 Session / CSRF / opaque handles / tickets
  └── 每 Session 独立 ProtocolContext + Cookie Jar
       └── jAccount / Canvas / LTI / 视频源
```

协议核心、Web 状态和界面相互分离：

```text
crates/canvas-core/   jAccount、Canvas、LTI、视频协议；不依赖 Axum
crates/protocol-cli/  显式门控的真实协议验证 CLI
crates/server/        Session、SSE、API、ticket、Range 流式代理、静态资源
frontend/             React + TypeScript + Vite
config/               开发与生产配置示例
deploy/               Ubuntu systemd/Caddy；历史 macOS/cloudflared 模板
scripts/              构建、安装、健康检查、更新、回滚
docs/                 协议、安全、API、前端和部署说明
```

参考项目 `Okabe-Rintarou-0/SJTU-Canvas-Helper` 固定在 commit `b5d895af57aaa74dfd53cef80dfb64c76c023c20` (`v3.0.8`)。归属与 MIT License 见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

## 本地开发

要求：Rust（由 `rust-toolchain.toml` 固定）、Node.js、npm，以及运行 Playwright E2E 时可用的 Google Chrome。

1. 复制开发配置并替换白名单占位值：

```powershell
Copy-Item config/example.toml config/local.toml
$stableId = Read-Host "Stable ID" -MaskInput
$stableId | cargo run -q -p server --bin hash-stable-id
Remove-Variable stableId
```

2. 启动后端。真实协议默认关闭，只有操作者显式开启才会访问交大服务：

```powershell
$env:SJTU_REAL_PROTOCOL_TEST = "1"
$env:SJTU_CANVAS_CONFIG = "config/local.toml"
$env:RUST_LOG = "server=info,canvas_core=info"
cargo run -p server
```

3. 另一个终端启动 Vite：

```powershell
Set-Location frontend
npm ci
npm run dev
```

浏览器访问 `http://127.0.0.1:5173`。Vite 将同源 `/api` 代理到 `127.0.0.1:3100`，后端不启用宽泛 CORS。开发配置使用非 Secure 的本地 Cookie；生产配置不会自动继承该例外。

## 前端命令

```bash
cd frontend
npm ci
npm run typecheck
npm run lint
npm run test
npm run test:e2e
npm run build
```

Playwright 使用纯测试 fixture server，不在生产后端添加 Mock 登录入口。trace、video、screenshot 与生产 source map 默认关闭。

## Rust 质量门

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
```

Mock 测试不会访问真实交大服务。真实 CLI 与真实 server 均要求 `SJTU_REAL_PROTOCOL_TEST=1`，不得在 CI 自动设置。

## API

```text
GET  /api/health
POST /api/auth/qr/start
GET  /api/auth/qr/events/:pending_id
GET  /api/auth/session
POST /api/auth/logout
GET  /api/me
GET  /api/courses
GET  /api/courses/:course_handle/videos
GET  /api/courses/:course_handle/videos/:video_handle
POST /api/courses/:course_handle/videos/:video_handle/tracks/:track_handle/ticket
HEAD /api/download/:ticket
GET  /api/download/:ticket
```

登录成功 SSE 事件不能设置 Session Cookie。前端收到 `authenticated` 后调用现有 `GET /api/auth/session` 完成 pending claim，浏览器才获得正式 Cookie。完整 schema 见 [docs/api.md](docs/api.md)。

## 下载模型

前端先用 Session 内存中的 CSRF token 签发 60 秒 ticket，再创建临时同源 `<a>` 触发浏览器原生下载。它不会调用 `fetch().blob()`、IndexedDB、Service Worker 或前端视频缓存。

ticket 只存在内存并绑定 Session、课程、录像和轨道；URL 中不编码上游地址。后端校验单 Range、上游 host 与 DNS/IP，流式转发 `200`、`206`、`416`，过滤 `Set-Cookie` 和 hop-by-hop headers。详见 [docs/frontend-download-model.md](docs/frontend-download-model.md) 与 [docs/download-proxy-security.md](docs/download-proxy-security.md)。

## 生产构建

Ubuntu 在精确源码 SHA 上执行：

```bash
SOURCE_GIT_SHA=<exact-sha> CARGO_BUILD_JOBS=1 \
  ./scripts/build-release-linux.sh
```

Windows 可验证和生成 Windows 包：

```powershell
./scripts/build-release.ps1
```

Linux 输出目录：

```text
release-linux/
├── bin/sjtu-canvas-video-server
├── frontend/dist/
├── VERSION
└── manifest.txt
```

`VERSION` 只记录 Git 身份、UTC 构建时间、工具版本与 target triple，不记录用户名、构建路径或 secret。Windows 构建不能冒充 Linux 二进制。

## Ubuntu、Caddy 与 Cloudflare

生产配置使用 [deploy/ubuntu/production.example.toml](deploy/ubuntu/production.example.toml) 的结构，真实文件位于 `/etc/canvas-video/config.toml`，权限为 `root:canvas-video 0640`。生产模式在启动时拒绝：

- 非 loopback bind；
- 非 HTTPS `public_origin`；
- 非 Secure 或非 `__Host-` Session Cookie；
- Cookie Domain、非根 Path 或非 HttpOnly；
- 相对/缺失的前端 dist；
- 示例白名单。

生产拓扑为 `Cloudflare → Caddy → http://127.0.0.1:3100`。Caddy 不提供静态文件、不缓存、不压缩且不开 access log；安装、systemd、DNS、Full (strict)、Cache Rule、更新与回滚见：

- [docs/deployment-ubuntu-cloudflare.md](docs/deployment-ubuntu-cloudflare.md)
- [docs/update-and-rollback.md](docs/update-and-rollback.md)
- [docs/production-ubuntu-acceptance.md](docs/production-ubuntu-acceptance.md)

Cloudflare 必须对 `/api/*` 配置 Bypass cache。公网验收需确认下载响应仍为 `206`、`Content-Range` 正确、`Cache-Control: private, no-store`，且 `CF-Cache-Status` 不是缓存命中。

历史 Mac mini + Cloudflare Tunnel 文件保留为替代方案，见 [docs/deployment-macos-cloudflare.md](docs/deployment-macos-cloudflare.md)，不代表当前生产状态。

## 安全不变量

- 每个网站 Session 独占 `ProtocolContext` 与 Cookie Jar；不存在全局上游 Cookie 或课程 token。
- 上游 Cookie、`tokenId`、视频 token、真实资源 ID 和视频 URL不进入浏览器 JSON或持久化存储。
- Stable ID 只用于服务端白名单；界面不显示原文或可离线比对的哈希。
- CSRF token、QR URL、pending ID、handles 与 ticket 只在必要的内存生命周期内存在。
- `/api/*` 永不进入 SPA fallback；API/HTML 不缓存，只有 Vite 哈希静态资源长期缓存。
- 下载全程流式转发，不生成视频临时文件；登出、过期、关机或重启销毁内存会话。
- CORS 默认关闭，Origin 精确匹配，生产 Cookie 为 `Secure; HttpOnly; SameSite=Lax; Path=/` 且无 Domain。
- 日志只记录 request ID、路由模板、脱敏句柄和状态，不记录实际 URI、Cookie、token、文件名或上游 URL。

详细威胁边界见 [docs/security-model.md](docs/security-model.md)。

## 已知限制

- 这是私人 MVP，不是学校官方服务，也不适合公开运营。
- Session、handles、tickets 和上游 Cookie 只在内存中；应用重启后所有用户必须重新扫码。
- 第一版只支持单轨、单 Range 下载；不支持多 Range、批量下载、在线播放、字幕、PPT、转码或合并。
- 已观察到某些已授权课程的视频接口可能返回显式 `502`；前端显示错误和 request ID，不伪装为空课程。
- 真实轨道类型可能为 `unknown`；前端中性显示“视频轨道 1/2”，不根据顺序猜测。
- Ubuntu 公网、Cloudflare 与 Safari/Android 的状态只可按实际验收结果记录，不能由 Mock 或 Windows 测试推断。

## 隐私与许可

服务不保存账号密码，不把上游 Cookie 发给浏览器，不长期保存 jAccount/Canvas 会话，也不缓存课程视频。用户只能下载本人当前有权限访问的内容，并应遵守课程与著作权要求，不得公开传播受访问控制保护的录像。

本项目使用 MIT License。真实账号、稳定身份、白名单、生产配置、Cloudflare credential、协议报告、浏览器 trace、HAR、截图和视频文件均不得提交 Git。
