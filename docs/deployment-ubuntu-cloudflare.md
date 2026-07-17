# Ubuntu + Caddy + Cloudflare deployment

这是当前推荐的生产部署路线。历史 Mac mini + Tunnel 方案仍保留在
`docs/deployment-macos-cloudflare.md`，但不是当前生产状态。

## Architecture and invariants

```text
browser -> Cloudflare HTTPS -> Ubuntu Caddy :443
        -> http://127.0.0.1:3100 -> Axum + React + upstream protocol
```

- Axum 只监听 loopback；云防火墙和 UFW 不开放 3100。
- Caddy 只负责 TLS、HTTP 到 HTTPS 和反向代理，不读取 React dist。
- `/api/download/*` 保留 Range 语义，并向上游声明 `Accept-Encoding: identity`。
- Caddy access log 保持关闭，避免下载 ticket 进入 URI 日志。
- API 和视频响应由 Axum 设置 no-store；Cloudflare 另设 `/api/` 缓存绕过。

## Server prerequisites

推荐 Ubuntu 22.04 或更新版本。部署前确认 80/443 未被未知服务占用、SSH
端口仍可达、3100 未公开，并记录 `uname -m`、磁盘与内存。不要盲目启用
UFW；如确需启用，必须先允许实际 SSH 端口，再允许 80/443。

安装构建依赖：

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev \
  ca-certificates curl git xz-utils shellcheck
```

Rust 应通过校验过的官方 `rustup-init` 安装到普通构建用户。Node 使用项目
兼容的官方 Linux 二进制，并校验 `SHASUMS256.txt`；不要用明显过旧的 Ubuntu
Node 包。资源较小的服务器可显式设置 `CARGO_BUILD_JOBS=1`。

## Exact source transfer and build

服务器无法访问 Git remote 时，仅传输已跟踪 HEAD：

```bash
git archive --format=tar HEAD -o phase-3.5-source.tar
git get-tar-commit-id < phase-3.5-source.tar
scp phase-3.5-source.tar canvas:/tmp/
```

在服务器校验输出为预期 SHA 后解压。archive 没有 `.git`，构建时必须显式
传入同一个 SHA：

```bash
SOURCE_GIT_SHA=<40-char-sha> CARGO_BUILD_JOBS=1 \
  ./scripts/build-release-linux.sh
```

脚本执行前端 install/typecheck/lint/unit/build，以及 Rust fmt/check/clippy/test/
release build。Ubuntu Playwright 仅在显式设置 `RUN_PLAYWRIGHT_E2E=1` 时运行；
否则报告 `not_run`。产物为 `release-linux/`，只含二进制、前端 dist、VERSION
与 SHA-256 manifest，不含配置、source map、`.local` 或视频。

## Production config and whitelist

复制 `deploy/ubuntu/production.example.toml` 的结构到：

```text
/etc/canvas-video/config.toml
```

只通过加密 SSH 传输真实 stable ID hash。禁止把原始身份、hash 或配置写入
Git、systemd、Caddyfile、shell history 或发布目录。权限必须为：

```bash
sudo chown root:canvas-video /etc/canvas-video/config.toml
sudo chmod 0640 /etc/canvas-video/config.toml
```

生产配置固定使用 `https://canvas.1molchuan.top`、Secure `__Host-` Cookie、
loopback bind 和绝对 frontend dist。服务启动会拒绝示例白名单及不安全值。

## Install application service

首次安装前先放好私有配置，再执行：

```bash
sudo ./scripts/install-ubuntu.sh /absolute/path/to/release-linux
sudo systemd-analyze verify /etc/systemd/system/canvas-video.service
sudo systemctl status canvas-video.service --no-pager
sudo systemd-analyze security canvas-video.service
curl --fail --silent --show-error http://127.0.0.1:3100/api/health
```

服务用户 `canvas-video` 是无 home、无 shell、无 sudo 的系统用户。release 为
root 所有；服务用户只有读取 release 和配置的权限。确认 `ss -ltnp` 中应用仅为
`127.0.0.1:3100`。

## Install Caddy

使用 Caddy 官方 Ubuntu repository。先安装 keyring 依赖，再下载官方签名 key
和 source list，最后 `apt-get install caddy`。不要执行未知 `curl | bash`。
安装后先备份已有 `/etc/caddy/Caddyfile`，再合并本项目站点；不得覆盖不相关
站点。

```bash
sudo install -m 0644 deploy/ubuntu/Caddyfile /etc/caddy/Caddyfile
sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy
sudo systemctl status caddy --no-pager
```

Caddyfile 不含 `file_server`、`encode`、access log、`flush_interval -1` 或总下载
超时。SSE 由上游 `text/event-stream` 驱动。

## DNS, TLS and Cloudflare

在服务器与 Caddy 都就绪后创建 Cloudflare DNS：

- Type: `A`
- Name: `canvas`
- Content: Ubuntu 公网 IPv4
- Proxy status: Proxied
- TTL: Auto

只有真实可达 IPv6 时才添加 AAAA。SSL/TLS mode 必须为 **Full (strict)**，并
开启 Always Use HTTPS；禁止 Flexible。Caddy 的公开 ACME 证书必须有效且 SAN
包含 `canvas.1molchuan.top`。

为此 hostname 创建 Cache Rule：

```text
hostname equals canvas.1molchuan.top AND URI path starts with /api/
Cache eligibility: Bypass cache
```

确认没有后续 Cache Everything、Edge TTL、Worker 或 Transform Rule 覆盖它。
静态哈希 asset 可按 Axum 的 immutable header 缓存。

## Verification and troubleshooting

```bash
./scripts/verify-production.sh
journalctl -u canvas-video --since today
journalctl -u caddy --since today
```

只记录脱敏的状态、request ID、route template 和错误类别。不得复制完整 URI、
QR URL、Cookie、ticket、handle、token 或上游 URL。若 Caddy 证书签发失败，先
查看 journal 和 DNS A；不得关闭 TLS 校验。若课程返回 502，保留 request ID，
不要把它显示为空课程，也不要引入 PAT。

## Data and backup boundary

只备份私有配置的安全副本和版本化源码。Session、上游 Cookie、课程 token、
ticket 与视频都只在内存中；重启服务后全部失效。应用不写课程视频，发布、
`/tmp` 与 `/var/tmp` 均不应出现视频文件。
