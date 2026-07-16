# SJTU-Canvas-Helper 认证与 Canvas 调研

参考仓库：`research_sjtu_canvas_helper/SJTU-Canvas-Helper`
固定提交：`b5d895af57aaa74dfd53cef80dfb64c76c023c20`
调研日期：`2026-07-17`
范围：`src-tauri/src/app/basic.rs`、`client/basic.rs`、`client/constants.rs`、`model/mod.rs`，以及认证/Canvas/视频登录相关直接调用方。

## 一、源码事实

### 1. 配置、身份与课程数据的落点

| 主题 | 源码事实 | 证据 |
| --- | --- | --- |
| 持久化配置 | `AppConfig` 直接持久化 `token`、`ja_auth_cookie`、`video_cookies`、`oauth_consumer_key`、`proxy_port`、`mcp_enabled`、`mcp_port` 等字段。 | `src-tauri/src/model/mod.rs:148-196` |
| 默认值 | `token`、`ja_auth_cookie`、`video_cookies` 默认空串；`proxy_port` 默认 `3030`；`mcp_port` 默认 `3100`。 | `src-tauri/src/model/mod.rs:198-220` |
| 用户身份模型 | 当前 Canvas 用户读取为 `User { id, name, created_at, sortable_name, short_name, login_id, email }`。 | `src-tauri/src/model/mod.rs:243-254` |
| 课程模型 | 课程列表使用 `Course { id, uuid, name, course_code, enrollments, access_restricted_by_date, teachers, term, syllabus_body }`。 | `src-tauri/src/model/mod.rs:5-23` |
| 课程过滤 | `Course::is_access_restricted()` 仅看 `access_restricted_by_date`；`list_courses` 会过滤掉受限课程。 | `src-tauri/src/model/mod.rs:48-51`, `src-tauri/src/client/basic.rs:651-661` |
| 本地配置文件 | `App::save_config()` 将整个 `AppConfig` JSON 写入账户配置文件；`get_config()` 直接返回内存中的完整配置。 | `src-tauri/src/app/basic.rs:377-385`, `src-tauri/src/app/basic.rs:959-980`, `src-tauri/src/main.rs:353-355`, `src-tauri/src/main.rs:659-661` |

### 2. Canvas PAT / 用户身份 / 课程列表主链路

#### 2.1 前端入口

- 设置页把 `token` 作为必填字段，文案明确引导用户去 `https://oc.sjtu.edu.cn/profile/settings` 创建 API Token。
  - `src/page/settings.tsx:61`
  - `src/page/settings.tsx:500-559`
  - `src/page/settings.tsx:846-860`
  - `src/page/settings.tsx:868-881`
- 前端配置缓存 `CONFIG` 在 `getConfig()/saveConfig()` 中通过 Tauri `get_config` / `save_config` 命令读写。
  - `src/lib/config.ts:9-26`

#### 2.2 后端调用顺序

1. 用户在设置页填写 `token`，点击保存。
2. 前端 `saveConfig(config)` 调用 Tauri `save_config`。
   - `src/lib/config.ts:22-25`
   - `src-tauri/src/main.rs:659-661`
   - `src-tauri/src/app/basic.rs:959-980`
3. 测试 Token 时，前端 `invoke("test_token", { token })`。
   - `src/page/settings.tsx:546-559`
   - `src-tauri/src/main.rs:343-349`
   - `src-tauri/src/app/basic.rs:576-581`
4. `APP.test_token()` 直接调用 `client.get_me(token)`，请求 `GET {base_url}/api/v1/users/self`，头为 `Authorization: Bearer {token}`。
   - `src-tauri/src/app/basic.rs:576-581`
   - `src-tauri/src/client/basic.rs:747-750`
   - `src-tauri/src/client/common.rs:18-36`
5. 课程列表入口 `list_courses` 读取保存后的 `config.token`，请求 `GET {base_url}/api/v1/courses?include[]=teachers&include[]=term`，再过滤 `access_restricted_by_date=true` 的课程。
   - `src/lib/hooks.tsx:625-666`
   - `src/page/calendar.tsx:139-150`
   - `src-tauri/src/main.rs:241-243`
   - `src-tauri/src/app/basic.rs:388-397`
   - `src-tauri/src/client/basic.rs:651-661`
6. 当前用户身份入口 `get_me` 使用保存后的 `config.token` 调 `GET /api/v1/users/self`。
   - `src/lib/hooks.tsx:664-666`
   - `src-tauri/src/main.rs:347-349`
   - `src-tauri/src/app/basic.rs:580-581`
   - `src-tauri/src/client/basic.rs:747-750`

#### 2.3 直接调用方

- Tauri commands：
  - `list_courses`: `src-tauri/src/main.rs:240-243`
  - `test_token`: `src-tauri/src/main.rs:342-345`
  - `get_me`: `src-tauri/src/main.rs:347-349`
  - `get_config` / `save_config`: `src-tauri/src/main.rs:352-355`, `src-tauri/src/main.rs:658-661`
- MCP server 直接暴露：
  - `list_courses`: `src-tauri/src/mcp/mod.rs:65-68`
  - `get_me`: `src-tauri/src/mcp/mod.rs:70-73`
  - `test_token`: `src-tauri/src/mcp/mod.rs:325-333`
- Canvas agent 直接暴露：
  - `list_courses`: `src-tauri/src/canvas_agent/mod.rs:112-133`
  - `get_me`: `src-tauri/src/canvas_agent/mod.rs:137-158`

### 3. UUID、二维码、WebSocket、`express_login`、`JAAuthCookie`

#### 3.1 前端状态机

- 二维码登录不是 Rust WebSocket；WebSocket 由前端直接连 `wss://jaccount.sjtu.edu.cn/jaccount/sub/{uuid}`。
  - `src/lib/hooks.tsx:48-52`
  - `src/lib/hooks.tsx:454-573`
- 设置页 `InlineQRCodePanel` 挂载时自动 `showQRCode()`。
  - `src/page/settings.tsx:138-150`
- WebSocket 打开后前端每 25 秒发送一次 `{"type":"UPDATE_QR_CODE"}`，收到：
  - `UPDATE_QR_CODE` 时，用 `uuid + ts + sig` 拼接二维码 URL。
  - `LOGIN` 时，调用 `express_login`。
  - `src/lib/hooks.tsx:503-571`

#### 3.2 后端调用顺序

1. 前端 `invoke("get_uuid")`。
   - `src/lib/hooks.tsx:468-483`
   - `src-tauri/src/main.rs:799-807`
   - `src-tauri/src/app/video.rs:10-15`
2. `client.get_uuid()` 访问 `MY_SJTU_URL` 页面，从 HTML 正文用正则抓取 UUID。
   - `src-tauri/src/client/video.rs:116-131`
3. 前端用该 UUID 建立 WebSocket，并等待二维码签名参数。
   - `src/lib/hooks.tsx:473-479`
   - `src/lib/hooks.tsx:559-562`
4. 扫码成功后，前端 `invoke("express_login", { uuid })`。
   - `src/lib/hooks.tsx:485-500`
   - `src-tauri/src/main.rs:805-807`
   - `src-tauri/src/app/video.rs:14-15`
5. `client.express_login()` 请求 `GET https://jaccount.sjtu.edu.cn/jaccount/expresslogin?uuid=...`，随后从 `AUTH_URL` 对应 cookie jar 中读取 `JAAuthCookie`。
   - `src-tauri/src/client/constants.rs:13`
   - `src-tauri/src/client/video.rs:134-145`
6. 前端把返回值写入 `config.ja_auth_cookie` 并 `saveConfig()`；这里保存的是 cookie 值，不含 `JAAuthCookie=` 前缀。
   - `src/lib/hooks.tsx:493-497`
   - `src/lib/config.ts:22-25`
   - `src-tauri/src/model/mod.rs:171-173`
7. Rust 侧真正使用时再拼成 `JAAuthCookie=<value>`。
   - `src-tauri/src/app/video.rs:18-20`

### 4. `JAAuthCookie` 到 Canvas 额外登录，再到课程视频

#### 4.1 Cookie 状态

| 名称 | 位置 | 生命周期 | 用途 |
| --- | --- | --- | --- |
| `token` | `AppConfig.token` | 持久化 | Canvas REST API 的 `Authorization: Bearer ...` |
| `ja_auth_cookie` | `AppConfig.ja_auth_cookie` | 持久化 | 额外登录凭证，按需拼成 `JAAuthCookie=...` 注入 jar |
| `video_cookies` | `AppConfig.video_cookies` | 持久化 | 视频站登录后的整串 cookie；应用启动 `init()` 时回灌到 jar |
| `Client.jar` | `reqwest::cookie::Jar` | 进程内 | 承载 `JAAuthCookie`、视频站 cookie、JBox cookie |
| `Client.token` | `RwLock<String>` | 进程内 | `get_canvas_videos()` 取回的视频系统 token，供后续视频详情/字幕/PPT 请求复用 |

证据：

- `Client` 持有共享 `jar` 与独立 `token`。`src-tauri/src/client/mod.rs:17-24`
- `App::init()` 若 `video_cookies` 非空，会 `client.init_cookie(cookies)`。`src-tauri/src/app/basic.rs:226-237`
- `attach_ja_auth_cookie()` 会把同一个 `JAAuthCookie` 注入 `AUTH_URL` 与 `MY_SJTU_URL` 两个 host。`src-tauri/src/client/video.rs:108-114`
- `get_canvas_videos()` 会把 LTI 返回的 token 写入 `self.token`。`src-tauri/src/client/video.rs:369-392`
- `get_canvas_video_info()` / `get_subtitle()` / `get_ppt()` 都读 `self.token` 请求。`src-tauri/src/client/video.rs:591-609`, `src-tauri/src/client/video.rs:651-674`, `src-tauri/src/client/video.rs:701-716`

#### 4.2 额外登录检测与 Canvas 登录

1. 设置页初始化时会静默执行一次 `check_extra_login_status()`。
   - `src/page/settings.tsx:445-451`
   - `src/page/settings.tsx:377-400`
2. `client.check_extra_login_status()` 先附加 `JAAuthCookie`，再访问 `MY_SJTU_URL` 预热，再访问 `MY_SJTU_ACCOUNT_URL`；`401`、非成功状态、空响应体都算未登录。
   - `src-tauri/src/client/video.rs:177-195`
3. `App.check_extra_login_status()` 在上一步返回 `true` 后，还会追加一次 `login_canvas_website()`；若收到 `AppError::LoginError`，会转成 `Ok(false)`。
   - `src-tauri/src/app/video.rs:42-52`
4. 视频页首次进入时也会主动 `login_canvas_website()`；失败则清空 `config.ja_auth_cookie` 并落盘。
   - `src/page/video.tsx:142-149`
   - `src/page/video.tsx:260-274`
5. `client.login_canvas_website()` 的成功判定很窄：附加 `JAAuthCookie` 后请求 `CANVAS_LOGIN_URL`，若最终落到 `jaccount.sjtu.edu.cn` 域则返回 `LoginError`，否则视为成功。
   - `src-tauri/src/client/constants.rs:4`
   - `src-tauri/src/client/video.rs:165-174`

#### 4.3 Canvas 课程视频请求顺序

1. `get_canvas_videos(course_id)` 前提是本进程里已有可用 `JAAuthCookie`，通常由扫码登录后保存并在本次调用时注入 jar。
2. 请求 `GET https://oc.sjtu.edu.cn/courses/{course_id}/external_tools/8329`，解析表单。
   - `src-tauri/src/client/video.rs:254-268`
3. 提交到 `.../oidc/login_initiations`，解析第二个表单。
   - `src-tauri/src/client/video.rs:270-295`
4. 关闭自动跳转后提交到 `.../lti3/lti3Auth/ivs`，从 `Location` 里抽取 `tokenId`。
   - `src-tauri/src/client/video.rs:297-331`
5. `GET .../lti3/getAccessTokenByTokenId?tokenId=...`，取回 `data.token` 与 `data.params.courId`。
   - `src-tauri/src/client/video.rs:336-358`
6. 把上一步的 `token` 写入 `Client.token`，再 `POST .../directOnDemandPlay/findVodVideoList`，头里带 `token` 与视频站 referer。
   - `src-tauri/src/client/video.rs:369-399`
7. 后续 `get_canvas_video_info(video_id)`、字幕、PPT 都依赖第 6 步写入的 `Client.token`。
   - `src-tauri/src/client/video.rs:591-609`
   - `src-tauri/src/client/video.rs:651-674`
   - `src-tauri/src/client/video.rs:701-716`

### 5. 失败分支

| 位置 | 失败条件 | 结果 |
| --- | --- | --- |
| `get_uuid()` | HTML 里正则抓不到 UUID | 返回 `Ok(None)`，前端显示“未能获取登录二维码标识” |
| 前端 WebSocket | 连接关闭且尚未拿到二维码 | 前端报“二维码连接未建立成功，请重新获取” |
| 前端二维码超时 | 8 秒内未收到二维码，且已重试一次 | 前端报“二维码获取超时，请重新获取” |
| `express_login()` | cookie jar 中没有 `JAAuthCookie` | 返回 `Ok(None)`；前端这里直接 `return`，无额外报错 |
| `login_canvas_website()` / `login_video_website()` | 最终域仍是 `jaccount.sjtu.edu.cn` | `Err(AppError::LoginError)` |
| `get_form_data_from_doc()` | 页面里没有目标 form | `VideoDownloadError("No Form Found")` |
| `get_token_id()` | 表单为空、`Location` 缺失、`tokenId` 缺失 | `VideoDownloadError(...)` |
| `getAccessTokenByTokenId` 解析 | `data.token` 或 `data.params.courId` 缺失 | `VideoDownloadError(...)` |
| `get_subtitle()` | `resp.data` 为空 | `VideoDownloadError("No Subtitle Found")` |
| `get_ppt()` | `resp.data` 为空 | `VideoDownloadError("No PPT Found")` |
| Canvas REST API | 任一 `error_for_status()` 命中 401/403/5xx | 直接冒泡成 `AppError::Network` / 序列化后的错误字符串 |

证据：

- 前端二维码逻辑：`src/lib/hooks.tsx:468-571`
- 登录错误类型：`src-tauri/src/error/mod.rs:19-57`
- 视频链路失败点：`src-tauri/src/client/video.rs:234-236`, `src-tauri/src/client/video.rs:270-331`, `src-tauri/src/client/video.rs:347-358`, `src-tauri/src/client/video.rs:671-716`

### 6. Web 服务化风险

#### 6.1 已由源码证实

- `save_config` 命令会 `tracing::info!("Receive config: {:?}", config)`；而 `AppConfig` 含 `token`、`ja_auth_cookie`、`video_cookies`，且 `AppConfig` 派生了 `Debug`。
  - `src-tauri/src/main.rs:659-661`
  - `src-tauri/src/model/mod.rs:148-196`
- 网络调试日志不会脱敏 header 值。`sanitize_header_value()` 直接原样返回，`request_headers` 和响应体都会存入 `NetworkDebugStore`；如果开启 `debug_mode`，Bearer token、cookie、返回体都可能被本地查看。
  - `src-tauri/src/client/debug.rs:54-173`
  - `src-tauri/src/client/common.rs:18-52`
- 本地视频代理 `prepare_proxy()` 监听 `127.0.0.1:{proxy_port}`，无鉴权，暴露 `/vod/*` 与 `/ready`。它会把任意 `/vod/*` tail 转发到 `https://live.sjtu.edu.cn/vod/...`，并强制加 `Referer: https://courses.sjtu.edu.cn`。
  - `src-tauri/src/app/basic.rs:240-325`
- 本地 MCP HTTP/SSE 服务监听 `127.0.0.1:{mcp_port}`，无额外鉴权代码；一旦启用，任一本机进程都可经 `list_courses`、`get_me` 等工具间接消费当前保存的 Canvas token。
  - `src-tauri/src/model/mod.rs:191-193`
  - `src-tauri/src/mcp/mod.rs:63-73`
  - `src-tauri/src/mcp/mod.rs:355-386`

#### 6.2 会话串用 / 缓存错配风险

- `Client` 的 `jar` 与视频 `token` 是长生命周期进程内状态；`switch_account()` 和 `save_config()` 只会改 `base_url` / 配置，没有显式清空 cookie jar 或 `Client.token`。
  - `src-tauri/src/client/mod.rs:17-24`
  - `src-tauri/src/app/basic.rs:155-176`
  - `src-tauri/src/app/basic.rs:959-980`
- `list_courses()` 有缓存，但 `save_config()` 只有在 `base_url` 变化时才 `invalidate_cache()`；同账号直接换 `token` 时，课程缓存不会被清掉。
  - `src-tauri/src/app/basic.rs:388-397`
  - `src-tauri/src/app/basic.rs:963-966`
- `get_canvas_video_info()` / 字幕 / PPT 都依赖最近一次 `get_canvas_videos()` 写入的进程内 `Client.token`；如果服务被长期驻留或多会话复用，调用顺序和调用者身份会耦合。
  - `src-tauri/src/client/video.rs:369-392`
  - `src-tauri/src/client/video.rs:591-609`
  - `src-tauri/src/client/video.rs:651-716`

## 二、真实待验证

以下内容是代码写死的外部协议假设，不是本次只读调研能在本地证实的“真实线上事实”：

- `MY_SJTU_URL` 页面在 `2026-07-16` 仍然把 UUID 暴露在 HTML 中，且仍匹配 `client/video.rs:120-121` 的正则格式。
- jAccount WebSocket 仍然使用 `wss://jaccount.sjtu.edu.cn/jaccount/sub/{uuid}`，并继续下发 `UPDATE_QR_CODE` / `LOGIN` 两类消息，且 `payload` 仍包含 `ts`、`sig`。
- `express_login?uuid=...` 在线上仍会把 `JAAuthCookie` 写入当前会话 cookie jar，且 host/domain 与 `AUTH_URL` 的读取方式兼容。
- `https://oc.sjtu.edu.cn/login/openid_connect` 不跳回 jAccount 就足以代表“Canvas 额外登录可用”；源码没有更强的功能性校验。
- 课程视频外部工具固定仍是 `/courses/{course_id}/external_tools/8329`，OIDC 表单 action 仍是：
  - `.../oidc/login_initiations`
  - `.../lti3/lti3Auth/ivs`
- `getAccessTokenByTokenId` 响应 JSON 仍然包含 `data.token` 与 `data.params.courId`。
- `VIDEO_OAUTH_KEY_URL` 页面里 `id="xForSecName"` 的 `<meta>` 标签仍然存在，且属性名仍是源码写死的 `vaule`。
- 设置页引导的 `https://oc.sjtu.edu.cn/profile/settings` 在真实 Canvas 实例中仍然是创建 API Token 的入口。

## 三、结论摘要

- Canvas 主链路是“持久化 `token` -> `Bearer` 调 `users/self` 与 `courses` API -> 课程列表缓存”。
- 扫码额外登录链路是“前端直连 jAccount WebSocket -> Rust 取 UUID / `express_login` -> 持久化 `ja_auth_cookie` -> 按需注入 jar -> `login_canvas_website` / LTI 视频链路”。
- 视频子系统额外引入两类进程内状态：`Client.jar` 与 `Client.token`；这对桌面单用户流程是可用的，但对 Web 服务化/多账号/多会话复用是不安全的默认形态。
