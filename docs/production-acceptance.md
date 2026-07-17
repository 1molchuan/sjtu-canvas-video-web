# Phase 3 生产验收清单

本清单只能由用户本人在授权账号、Mac mini 和生产域名上执行。Mock、Windows 本地或 Phase 2 本地 `206` 不能替代公网结论。每项只记录 `passed`、`failed`、`blocked` 或 `not_run`，不记录账号、course/video/track handle、Cookie、ticket、URL path/query 或文件名。

## 构建与服务

- [ ] macOS 本机 `scripts/build-release.sh`：____
- [ ] release install：____
- [ ] LaunchAgent 或 LaunchDaemon loaded：____
- [ ] cloudflared connected：____
- [ ] `127.0.0.1:3100/api/health`：____
- [ ] `lsof` 只显示 loopback：____
- [ ] 公网 HTTPS 页面与静态资源：____
- [ ] CSP 无严重错误：____

## 登录

- [ ] 公网 QR start：____
- [ ] SSE QR 与等待期间 heartbeat：____
- [ ] 用户本人扫码：____
- [ ] 白名单：____
- [ ] Secure `__Host-` Cookie（无 Domain、Path=/、HttpOnly、SameSite=Lax）：____
- [ ] Session endpoint：____
- [ ] 刷新页面恢复 Session：____
- [ ] DevTools 响应中无上游 Cookie/token：____

## 课程与录像

- [ ] 课程列表：____
- [ ] 已知成功课程录像：____
- [ ] 已知 `502` 课程显示错误而非空列表：____
- [ ] video detail：____
- [ ] unknown 轨道中性显示：____
- [ ] opaque handles 不显示在页面正文：____

## 下载与缓存

- [ ] ticket 签发：____
- [ ] 公网 `Range: bytes=0-0` 返回 `206`：____
- [ ] `Content-Range`、`Content-Length`、`Accept-Ranges` 正确：____
- [ ] `Cache-Control: private, no-store`：____
- [ ] `CF-Cache-Status` 不是 `HIT`：____
- [ ] 第二次请求仍不是缓存命中：____
- [ ] 浏览器取消下载：____
- [ ] 取消后再次下载证明 permit 释放：____
- [ ] 原生浏览器下载：____
- [ ] 一个授权的小轨道完整下载并可打开：____
- [ ] Mac mini 没有对应视频文件：____
- [ ] 服务器磁盘没有视频大小增长：____
- [ ] ticket 到期：____
- [ ] 登出：____
- [ ] 旧 ticket 失效：____

若没有合适的小轨道，完整下载标记 `blocked`；公网 `206`、取消、permit 释放和缓存绕过仍必须独立验证。

## 重启

- [ ] 应用重启：____
- [ ] 旧 Session 失效：____
- [ ] 重新扫码成功：____
- [ ] cloudflared 重启后恢复：____
- [ ] Mac mini 重启后服务启动（仅 boot daemon）：____
- [ ] 重启后 healthcheck：____

## 安全证据检查

不要保存真实 HAR、Playwright trace、截图、Cookie 导出或完整响应。只允许记录：日期、OS/Rust/cloudflared 版本、状态码、header 名与安全值、计数、duration、request ID 和结论。

检查以下内容未出现在 Git diff、浏览器 console、服务日志、Cloudflare 配置、plist 和构建产物：QR URL、pending/Session ID、CSRF、ticket、handles、stable ID/hash、JAAuthCookie、Canvas Cookie、tokenId、视频 token、上游 URL/query。

## 结果记录模板

```text
environment: macOS <version>, Rust <version>, cloudflared <version>
public_origin: configured (hostname omitted from shared report)
macOS build: not_run
public QR/SSE/login: not_run
courses/videos/detail: not_run
public Range 206: not_run
Cloudflare cache bypass: not_run
complete small download: not_run
logout/ticket invalidation: not_run
service restart: not_run
notes: no sensitive values retained
```
