# 前端认证流程

## 正式 Session claim

SSE 建立后，后续 SSE event 不能设置新的浏览器 Cookie。前端严格沿用 Phase 2 已真实验收的 claim 流程：

```text
POST /api/auth/qr/start
  ├── Set-Cookie: 临时 HttpOnly pending cookie
  └── pending_id + events_url

GET events_url（EventSource，自动携带 pending cookie）
  └── started / qr / scanned / authenticating / authenticated

收到 authenticated
  └── GET /api/auth/session
       ├── 服务端原子 claim 已完成 pending
       ├── 新随机 Session ID，防止 fixation
       ├── Set-Cookie: 正式 HttpOnly Session cookie
       └── 返回内存 CSRF token + Session 摘要
```

只有最后一次 Session 查询成功后，界面才进入课程页。仅收到 `authenticated` event 不等价于正式登录完成。

## 登录状态机

```text
idle
  → starting
  → waiting(qrUrl)
  → scanned
  → authenticating
  → completing-session
  → authenticated

terminal: rejected | expired | error
```

一次只有一个 pending。Hook 给每一轮登录分配 generation；旧 EventSource 的迟到事件、错误或自动重连不能修改新一轮状态。组件卸载、terminal event 和重新开始都会关闭旧 EventSource，但普通 SSE 断线不会创建新的 pending。

QR URL 与 pending ID 只存在 Hook 内存中。页面可以在新标签打开当前 QR URL，并设置 `noopener noreferrer`；全站 `Referrer-Policy: no-referrer`。它们不进入 console、storage 或错误报告。

## Session 恢复与过期

应用状态只有：

```text
checking | authenticated | anonymous | expired | error
```

- 启动与页面恢复焦点时查询 Session；
- `401` 或 `SESSION_EXPIRED` 清除 CSRF 和 Query cache，并跳转登录；
- `403` 显示仅限受邀账号，不伪装为网络错误；
- CSRF token 页面刷新后重新从 Session API获取；
- 不轮询、不自动延长 Session、不把 token 写入 storage。

## 登出

`POST /api/auth/logout` 发送当前内存 CSRF。成功后前端清除 CSRF、认证状态和全部 TanStack Query cache，再跳转 `/login`。服务端同步删除 Session、上游 Cookie Jar、handles 与 tickets，并清除 Cookie。重复登出保持幂等。

## 错误与隐私

统一 API client 解析结构化错误码、HTTP 状态和 `request_id`。界面不序列化原始错误对象，也不显示 pending、Session、CSRF、handles、ticket、上游 host 或响应正文。
