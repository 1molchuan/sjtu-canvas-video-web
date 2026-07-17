# Phase 3 前端实现说明

## 范围

Phase 3 将已经真实验收的 Phase 2 API 包装为 React 产品界面，不改变 jAccount、Canvas、LTI 或视频协议。应用名称为 **Canvas Video Helper**，所有页面明确标注“非上海交通大学官方服务”。

实现路由：

```text
/                                      按 Session 跳转
/login                                 jAccount 扫码
/courses                               课程列表
/courses/:courseHandle                 课程录像
/courses/:courseHandle/videos/:videoHandle 轨道与下载
/privacy                               隐私说明
```

## 技术和边界

- React、TypeScript strict、Vite、React Router、TanStack Query、Zod；
- 原生 CSS，不使用外部字体、图标 CDN、运行时 CSS 注入或 `unsafe-inline`；
- QR URL由 `qrcode.react` 在浏览器本地渲染，不请求远程二维码图片；
- API 响应先通过 Zod 严格验证，未知敏感字段不会被界面消费；
- API 固定 `credentials: "same-origin"`；CORS 不开启；
- CSRF、QR URL、pending ID、handles 与 ticket 不写 local/session storage；
- 前端不输出响应正文、Cookie、token、handles 或 ticket 到 console；
- 生产 build 不生成公开 source map。

## 页面行为

应用启动先检查 `/api/auth/session`。检查完成前只显示加载状态，避免课程内容闪烁。匿名用户进入 `/login`，已认证用户进入 `/courses`。窗口恢复焦点时只做一次轻量 Session 复查，不轮询或自动续期。

课程录像接口的 `502` 是上游失败，不是空数组。界面显示“当前无法获取这门课程的录像”、后端 `request_id` 和一次手动重试；TanStack Query 对可重试上游错误最多自动重试一次，对认证、权限、schema 和 handle 错误不自动重试。

`unknown` 轨道不会被猜测为黑板、摄像头或录屏，而按返回顺序显示“视频轨道 1”“视频轨道 2”和中性标签。

## 可访问性和响应式

- 路由切换同步更新文档标题；
- 错误使用 `role="alert"`，加载和状态使用 `aria-live`；
- 所有操作都有文本或 `aria-label`，焦点轮廓可见；
- 键盘可以完成主要导航；
- 状态不只依赖颜色；
- 360px viewport 无横向溢出，长课程/录像名自动换行；
- 下载中按钮禁用，关键操作不隐藏在 hover 后。

## 测试

```bash
cd frontend
npm ci
npm run typecheck
npm run lint
npm run test
npm run test:e2e
npm run build
```

Vitest/RTL 覆盖 Session、登录状态机、SSE 清理、迟到事件、课程/录像、`502`、unknown 轨道、CSRF、ticket、原生 anchor、登出和 schema 错误。Playwright 使用独立 fixture server 覆盖完整产品路径、手机 viewport 和键盘焦点；fixture 不存在于生产 Axum Router。

## 当前证据边界

前端单元测试、Mock E2E 与 Windows 本地 build 属于自动证据。Mac Safari、Android Chrome、Mac mini 二进制、Cloudflare 公网登录和公网下载必须分别标为 `passed`、`failed`、`blocked` 或 `not_run`；不能由 Chromium fixture 推断。
