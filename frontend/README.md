# Canvas Video Helper frontend

React/Vite 单页应用通过同源 `/api` 使用后端，不保存 Cookie、CSRF token、登录 pending、下载 ticket 或课程数据。视频下载由浏览器原生导航到短期下载地址，不经过 `fetch().blob()`。

```text
npm ci
npm run dev
npm run typecheck
npm run lint
npm run test
npm run test:e2e
npm run build
```

开发服务器监听 `127.0.0.1:5173`，并把 `/api` 代理到 `127.0.0.1:3100`。生产环境由 Axum 同源提供 `dist/`。
