# Mac mini + Cloudflare Tunnel 部署

## 目标拓扑

```text
https://canvas-video.example.com
  → Cloudflare Tunnel
  → http://127.0.0.1:3100
  → 单个 Axum 进程（React + /api）
```

`cloudflared` 从 Mac mini 主动建立出站连接；Mac mini 不需要公网 IP、端口转发或开放 3100 入站。Axum 继续只绑定 loopback，不增加 Caddy/Nginx。

Cloudflare 的[本地 Tunnel 配置文档](https://developers.cloudflare.com/tunnel/advanced/local-management/configuration-file/)要求 ingress 末尾有 catch-all；[macOS service 文档](https://developers.cloudflare.com/tunnel/advanced/local-management/as-a-service/macos/)区分登录启动 LaunchAgent 与开机启动 LaunchDaemon。Apple 也将 `launchd` 作为后台 agent/daemon 的标准管理方式，配置位置见 [Creating Launch Daemons and Agents](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html)。

## 1. 在 Mac mini 构建

安装 Rust、Node/npm、Google Chrome（Mock E2E）和 `cloudflared`。在干净仓库执行：

```bash
./scripts/build-release.sh
```

这会运行前端 typecheck/lint/unit/E2E/build、Rust fmt/test/release build，然后生成 `release/`。macOS 二进制必须在 Mac mini 或可信 macOS CI 构建；Windows `server.exe` 不可部署到 Mac。

## 2. 安装用户级服务

推荐私人 MVP 使用 LaunchAgent：

```bash
chmod +x release/scripts/*.sh release/scripts/lib/*.sh
release/scripts/install-macos.sh /absolute/path/to/repository/release
```

默认目录：

```text
~/Services/sjtu-canvas-video/
├── releases/<timestamp>-<git-sha>/
├── current -> releases/<timestamp>-<git-sha>
├── config/local.toml
└── logs/
```

初次安装会把示例配置复制为 `config/local.toml`、设置权限 `600`、生成 `~/Library/LaunchAgents/com.example.canvas-video.agent.plist`。只要配置仍含 `REPLACE_ME` 或示例域名，脚本会明确停止而不启动服务。

编辑私有配置：

```toml
[server]
mode = "production"
host = "127.0.0.1"
port = 3100
public_origin = "https://canvas-video.example.com"
frontend_dist = "/Users/YOUR_USER/Services/sjtu-canvas-video/current/frontend/dist"

[cookie]
name = "__Host-sjtu_canvas_video_session"
secure = true
http_only = true
same_site = "Lax"
path = "/"
```

白名单 hash 在私有终端生成，不放 plist 或 Git：

```bash
read -s STABLE_ID
printf '%s\n' "$STABLE_ID" | cargo run -q -p server --bin hash-stable-id
unset STABLE_ID
```

然后加载：

```bash
launchctl bootstrap "gui/$UID" "$HOME/Library/LaunchAgents/com.example.canvas-video.agent.plist"
launchctl kickstart -k "gui/$UID/com.example.canvas-video.agent"
./scripts/healthcheck.sh
```

LaunchAgent 随用户登录运行。若要求无人登录也开机运行，按 `deploy/macos/com.example.canvas-video.daemon.plist` 替换所有绝对路径和 `YOUR_USER`，用 `plutil -lint` 检查后安装到 `/Library/LaunchDaemons/`。Daemon 模式涉及 root 安装权限，但应用本身仍以指定普通用户运行；不要把白名单、Cookie 或 Tunnel token 放进 plist。

## 3. 创建 Tunnel

使用本地管理方式的示例：

```bash
brew install cloudflared
cloudflared tunnel login
cloudflared tunnel create canvas-video
cp deploy/cloudflare/config.example.yml "$HOME/.cloudflared/config.yml"
chmod 600 "$HOME/.cloudflared/config.yml" "$HOME/.cloudflared/YOUR_TUNNEL_UUID.json"
```

配置必须使用绝对 credential path：

```yaml
tunnel: YOUR_TUNNEL_UUID
credentials-file: /Users/YOUR_USER/.cloudflared/YOUR_TUNNEL_UUID.json

ingress:
  - hostname: canvas-video.example.com
    service: http://127.0.0.1:3100
  - service: http_status:404
```

验证并建立 DNS route：

```bash
cloudflared tunnel ingress validate
cloudflared tunnel ingress rule https://canvas-video.example.com
cloudflared tunnel route dns YOUR_TUNNEL_UUID canvas-video.example.com
cloudflared tunnel run YOUR_TUNNEL_UUID
```

确认手动运行正常后安装 service。Cloudflare 当前文档给出的两种方式：

```bash
# 登录后启动，读取 ~/.cloudflared/config.yml
cloudflared service install

# 开机启动，先把配置/credential 安全复制到 /etc/cloudflared
sudo cloudflared service install
```

credential JSON、Tunnel token 与真实 `config.yml` 不得复制进仓库、应用 plist 或发布包。

## 4. Cloudflare Cache Rule

Cloudflare 的 [Origin Cache Control](https://developers.cloudflare.com/cache/concepts/cache-control/) 说明 `no-store` 不应进入共享缓存；但 [Cache Rules](https://developers.cloudflare.com/cache/how-to/cache-rules/settings/) 的 Edge TTL/eligible 设置可以改变源站行为，因此仍需显式规则：

```text
When: URI Path starts with /api/
Then: Cache eligibility = Bypass cache
```

不要设置 Cache Everything、Edge TTL、下载 Worker 或会改变 Range 的压缩/转换规则。静态 `/assets/*` 才使用一年 immutable；`index.html` 为 `no-cache`，普通 API 为 `no-store`，下载为 `private, no-store`。

Cloudflare 对 Range 的行为受 `Content-Length` 影响，见[默认缓存行为中的客户端 Range 说明](https://developers.cloudflare.com/cache/concepts/default-cache-behavior/)。因此生产域名必须实际验证 `206`、`Content-Range` 和无缓存命中。

## 5. 本地与公网检查

```bash
curl --fail http://127.0.0.1:3100/api/health
curl --fail --silent --show-error https://canvas-video.example.com/api/health
lsof -nP -iTCP:3100 -sTCP:LISTEN
```

监听结果必须是 `127.0.0.1:3100`（或 IPv6 loopback），不能是 `0.0.0.0`。公网完整认证、SSE、Cookie、Range 与缓存验收按 `production-acceptance.md`，不得上传 DevTools HAR、真实 ticket、Cookie 或截图。

## 日志

应用日志默认位于部署目录 `logs/`；系统级 cloudflared service 的默认输出位置以 Cloudflare service 安装结果为准。日志只可用于状态、request ID 和错误分类；若发现 QR URL、Cookie、token、真实 URI 或 ticket，应立即停止公开验收并修复。
