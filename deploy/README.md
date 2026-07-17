# Deployment layer

生产拓扑只有一项源站服务：Cloudflare Tunnel 把公网 HTTPS hostname 转发到
`127.0.0.1:3100` 上的 Axum。Axum 同源提供 React 静态资源与 `/api/*`，不需要 Caddy。

- `macos/`：用户登录启动的 LaunchAgent 与开机启动的 LaunchDaemon 模板；
- `cloudflare/`：本地管理 Tunnel 的 ingress 示例，末尾保留 404 catch-all；
- 所有路径、域名、用户和 Tunnel UUID 都是占位符；凭据 JSON 不得进入 Git。

完整操作和验收步骤见 `docs/deployment-macos-cloudflare.md`。
