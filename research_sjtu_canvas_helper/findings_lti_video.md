# LTI / 视频链路调研

调研对象：`research_sjtu_canvas_helper/SJTU-Canvas-Helper`
固定提交：`b5d895af57aaa74dfd53cef80dfb64c76c023c20`

## 概览

### 源码事实

- Canvas 视频入口来自前端 `src/page/video.tsx:320-325`，调用 Tauri 命令 `get_canvas_videos`；视频详情入口来自 `src/page/video.tsx:289-302`，调用 `get_canvas_video_info`；下载任务入队来自 `src/page/video.tsx:331-347`，实际执行下载的是 `src/components/video_download_table.tsx:63-107`。
- Tauri 暴露层只是薄封装：`src-tauri/src/main.rs:821-847,861-870,1060-1066` 将 `get_canvas_videos`、`get_canvas_video_info`、`download_video` 暴露给前端；`src-tauri/src/app/video.rs:64-82` 再薄封装到 `client/video.rs`。
- `Client` 内部持有一个共享 `reqwest::Client`、共享 cookie jar 和单个 `RwLock<String>` token：`src-tauri/src/client/mod.rs:17-22`。实例初始化时 token 为空串：`src-tauri/src/client/basic.rs:70-77`。

## 逐跳链路

### 1. `external_tools/8329` -> OIDC 表单

### 源码事实

- `get_form_data_for_canvas_course_id` 对选中的 Canvas 课程发起 `GET https://oc.sjtu.edu.cn/courses/{course_id}/external_tools/8329`，然后在 HTML 中查找 `action="https://v.sjtu.edu.cn/jy-application-canvas-sjtu/oidc/login_initiations"` 的表单：`src-tauri/src/client/video.rs:254-268`。
- 表单解析逻辑 `get_form_data_from_doc` 只收集 `<input name=... value=...>`，存入 `HashMap<String, String>`：`src-tauri/src/client/video.rs:226-249`。

### 基于源码可见的风险

- 解析严格依赖完整 `action` 绝对 URL；一旦 LTI 页面改为相对路径、不同 host、或表单 action 改名，代码会直接报 `No Form Found`：`src-tauri/src/client/video.rs:232-236`。
- 解析只看 `input`，不看 `textarea`、`select`、`button`，且 `HashMap` 会覆盖同名字段；如果真实 LTI 表单出现同名多值字段，当前实现会丢信息：`src-tauri/src/client/video.rs:240-247`。

### 真实待验证

- `external_tools/8329` 当前真实返回的表单字段名、字段数量、是否包含非 `input` 字段，源码里没有固定 schema，需要抓真实页面确认。

### 2. OIDC 表单 -> LTI 表单 -> redirect -> `tokenId`

### 源码事实

- `get_token_id` 先把上一步表单直接 `POST` 到 `https://v.sjtu.edu.cn/jy-application-canvas-sjtu/oidc/login_initiations`：`src-tauri/src/client/video.rs:270-286`。
- 这一步返回 HTML 后，再次从页面中提取 `action="https://v.sjtu.edu.cn/jy-application-canvas-sjtu/lti3/lti3Auth/ivs"` 的表单：`src-tauri/src/client/video.rs:288-295`。
- 第二次提交时，代码显式新建 `reqwest::Client` 并设置 `redirect(Policy::none())`，但继续复用同一个 cookie jar：`src-tauri/src/client/video.rs:297-301`。
- 随后把第二个表单 `POST` 到 `https://v.sjtu.edu.cn/jy-application-canvas-sjtu/lti3/lti3Auth/ivs`，不跟随 302，而是从响应头 `location` 中解析 `tokenId`：`src-tauri/src/client/video.rs:303-329`。
- `tokenId` 提取方式是把 `location` 按 `?` 和 `&` 切分，然后找 `tokenId=` 前缀：`src-tauri/src/client/video.rs:320-327`。

### 基于源码可见的风险

- `tokenId` 解析依赖 `Location` 头存在；如果服务端改成 HTML 跳转、JS 跳转或 body 内返回 token，这里会直接报 `Redirect URL not found`：`src-tauri/src/client/video.rs:311-314`。
- `tokenId` 解析只认查询串样式；如果未来改成 fragment、path segment 或不同参数名，当前 split/strip 逻辑会失败：`src-tauri/src/client/video.rs:320-327`。

### 真实待验证

- 真实环境里 `lti3Auth/ivs` 的返回码、`Location` 具体格式、是否稳定包含 `tokenId`，需要实际抓包确认。

### 3. `tokenId` -> token 交换 -> `canvasCourseId`

### 源码事实

- `get_canvas_course_id_token_by_token_id` 会请求 `GET https://v.sjtu.edu.cn/jy-application-canvas-sjtu/lti3/getAccessTokenByTokenId?tokenId={token_id}`：`src-tauri/src/client/video.rs:334-345`。
- 它从 JSON 路径 `data.token` 取业务 token，从 `data.params.courId` 取 `canvas_course_id`，并返回 `(canvas_course_id, token)`：`src-tauri/src/client/video.rs:346-358`。
- `get_canvas_course_id_token` 只是串联 `get_token_id` 和 `get_canvas_course_id_token_by_token_id`：`src-tauri/src/client/video.rs:361-366`。

### 基于源码可见的风险

- 这里没有强类型结构体，直接用 `serde_json::Value` 和硬编码路径取字段；接口字段漂移会在运行时报 `Token not found` / `Canvas Course Id not found`：`src-tauri/src/client/video.rs:346-357`。

### 真实待验证

- `data.params.courId` 在真实返回里到底是“Canvas 课程 ID”还是“视频系统内部课程 ID”，源码只能看到它随后被命名为 `canvas_course_id`，语义需要拿真实响应确认。

### 4. token + `canvasCourseId` -> `findVodVideoList`

### 源码事实

- `get_canvas_videos` 调 `get_canvas_course_id_token(course_id)`，其中 `course_id` 来自前端当前选中的 Canvas 课程：`src/page/video.tsx:320-325`，`src-tauri/src/client/video.rs:369-370`。
- 它向 `https://v.sjtu.edu.cn/jy-application-canvas-sjtu/directOnDemandPlay/findVodVideoList` 发送 `POST`：`src-tauri/src/client/video.rs:372-394`。
- 请求体是 JSON，不是表单，且只放一个字段：`canvasCourseId = urlencoding::encode(canvas_course_id)`：`src-tauri/src/client/video.rs:374-380`。
- 请求头显式带 `Referer: https://v.sjtu.edu.cn/jy-application-canvas-sjtu-ui/` 和 `token: {token}`：`src-tauri/src/client/video.rs:384-392`。
- 这一步会把拿到的 token 写入 `self.token` 共享缓存，供后续 `get_canvas_video_info` / 字幕 / PPT 调用复用：`src-tauri/src/client/video.rs:382`。
- 响应解析为 `CanvasVideoResponse`，里面的 `records` 是 `Vec<CanvasVideo>`；`CanvasVideo` 只含 `video_id`、`video_name`、`classroom_name`、`course_begin_time`、`course_end_time` 等轻量字段：`src-tauri/src/client/video.rs:396-406`，`src-tauri/src/model/mod.rs:990-1028`。

### 基于源码可见的风险

- `findVodVideoList` 的 JSON 解析用了 `unwrap()`，接口返回非 JSON、登录页 HTML 或字段变更时会 panic，不是普通错误返回：`src-tauri/src/client/video.rs:400`。
- token 是 `Client` 实例级单值缓存，不按课程或视频隔离；后一次 `get_canvas_videos` 会覆盖前一次课程的 token：`src-tauri/src/client/mod.rs:17-22`，`src-tauri/src/client/video.rs:382`。

### 真实待验证

- `canvasCourseId` 是否必须 URL encode、服务端是否真的区分 encode/不 encode，需要真实请求确认。
- token 是否严格按课程隔离、是否短时失效、是否允许跨课程复用，源码无法证明。

### 5. `videoId` -> `getVodVideoInfos`

### 源码事实

- 前端选中某个 `CanvasVideo` 后，调用 `get_canvas_video_info(videoId)`：`src/page/video.tsx:289-302`。
- 后端向 `https://v.sjtu.edu.cn/jy-application-canvas-sjtu/directOnDemandPlay/getVodVideoInfos` 发 `POST` 表单，请求字段固定为 `playTypeHls=true`、`id={video_id}`、`isAudit=true`：`src-tauri/src/client/video.rs:591-606`。
- 这一步只带 `token` 头，不再携带 `canvasCourseId`，token 来自上一步写入的 `self.token`：`src-tauri/src/client/video.rs:603`。
- 响应体结构是 `GetCanvasVideoInfoResponse { data: VideoInfo }`：`src-tauri/src/client/video.rs:607-609`，`src-tauri/src/model/mod.rs:1030-1036`。
- `VideoInfo` 里与本链路最相关的字段有：`cour_id`、`rtmp_url_hdv`、`video_play_response_vo_list: Vec<VideoPlayInfo>`：`src-tauri/src/model/mod.rs:554-587`。
- `VideoPlayInfo` 里每个轨道/机位只暴露 `id`、`rtmp_url_hdv`、`cdvi_channel_num`、`cdvi_view_num`：`src-tauri/src/model/mod.rs:589-598`。

### 基于源码可见的风险

- `get_canvas_video_info` 完全依赖先前缓存的 `self.token`；如果用户先切到课程 A 获取列表，再切到课程 B 覆盖 token，之后回头对 A 的 `videoId` 拉详情，代码不会重新按课程刷新 token：`src-tauri/src/client/video.rs:382,591-609`。

### 真实待验证

- `getVodVideoInfos` 是否只靠 token 就能唯一绑定课程，还是服务端还会校验 `videoId` 与 token 的课程关联，需要真实交叉测试。

### 6. 轨道选择、URL host、播放代理

### 源码事实

- 前端把 `videoInfo.videoPlayResponseVoList` 直接当“轨道/机位列表”；给每项补 `key`、`index`、`name`，并把 `index === 0` 当主轨，其他轨命名为 `_录屏`：`src/page/video.tsx:294-301`。
- 播放 URL 不是直接使用 `rtmpUrlHdv`，而是把其中的 `https://live.sjtu.edu.cn` 替换成 `http://localhost:{proxyPort}`：`src/page/video.tsx:530-534`。
- 前端最多只支持双屏播放；第二个轨道如果 `index === 0` 会和主画面交换，否则作为副画面：`src/page/video.tsx:561-596`。
- 本地代理由 `prepare_proxy` 启动，监听 `127.0.0.1:{proxy_port}`，默认端口 `3030`：`src-tauri/src/app/basic.rs:259-325`，`src-tauri/src/model/mod.rs:176-177,212,232-233`。
- 代理只接收 `GET /vod/*`，把路径转发到 `https://live.sjtu.edu.cn/vod/{tail}`，保留 query string：`src-tauri/src/app/basic.rs:265-282`。
- 代理会把浏览器传来的 `Range` 头转发给上游，并强制加 `Referer: https://courses.sjtu.edu.cn`；响应时把上游所有 header 原样抄回，再流式转发 body：`src-tauri/src/app/basic.rs:271-307`。
- 视频元素只挂本地生成的字幕轨 `<track kind="subtitles">`，不是后端返回的多音轨元数据：`src/page/video.tsx:733-748,1117-1140`。

### 基于源码可见的风险

- host 改写逻辑只替换精确前缀 `https://live.sjtu.edu.cn`；如果 `rtmp_url_hdv` 未来换域名、协议或 CDN host，播放器将不会走代理：`src/page/video.tsx:530-534`。
- 代理只代理 `/vod/*` GET 请求；如果真实播放链路需要额外 host、POST、鉴权 header 或不同路径，当前代理层不覆盖：`src-tauri/src/app/basic.rs:265-315`。

### 真实待验证

- `rtmp_url_hdv` 在真实环境里是否恒定落在 `live.sjtu.edu.cn/vod/...`，需要抓多个课程、多个机位确认。
- 浏览器直连 `live.sjtu.edu.cn` 是否确实会被跨域/Referer 限制，导致必须经本地代理，源码只能说明“作者按这个假设实现了代理”。

### 7. 下载：Range / 响应头 / 重试

### 源码事实

- 下载任务前端会监听 `video_download://progress` 更新进度：`src/components/video_download_table.tsx:63-76`；实际下载调用 `invoke("download_video", { video, saveName })`：`src/components/video_download_table.tsx:88-107`。
- 前端失败重试逻辑固定 `maxRetries = 3`，每次失败后 sleep 1 秒再重试：`src/components/video_download_table.tsx:93-105`。
- Tauri 下载入口接收整个 `VideoPlayInfo`，不是重新按 `videoId` 拉详情：`src-tauri/src/main.rs:861-870`，`src-tauri/src/app/video.rs:72-83`。
- `download_video` 一开始就 `File::create(save_path)`，每次调用都会重建目标文件：`src-tauri/src/client/video.rs:475-483`。
- 下载前先对 `video.rtmp_url_hdv` 发 `GET`，带 `Range: bytes=0-0` 和 `Referer: https://courses.sjtu.edu.cn`：`src-tauri/src/client/video.rs:456-472`。
- `parse_download_probe` 通过 `206 Partial Content`、`Content-Range`、`Accept-Ranges: bytes` 判断是否支持 Range；优先从 `Content-Range` 取总大小，否则从 `Content-Length` 取：`src-tauri/src/client/video.rs:54-87`。
- 如果上游不支持 Range，就退化为单连接流式下载，仍然带 `Referer: https://courses.sjtu.edu.cn`：`src-tauri/src/client/video.rs:498-528`。
- 如果支持 Range，就按 `num_cpus::get()` 切分大区间，再在每个任务里用固定块大小 `VIDEO_CHUNK_SIZE = 4 MiB` 重复请求 `Range`：`src-tauri/src/client/video.rs:534-586`，`src-tauri/src/client/constants.rs:21-22`。
- 分块请求接受 `200 OK` 或 `206 Partial Content` 两种状态码：`src-tauri/src/client/video.rs:561-565`。

### 基于源码可见的风险

- 前端重试不会刷新 `VideoPlayInfo`、不会刷新 LTI token、也不会重新换 URL；它只是把同一个 `video` 对象再传一遍：`src/components/video_download_table.tsx:88-107`。
- 因为每次重试都会重新 `File::create`，所以失败后重试不是断点续传，而是覆盖式重下：`src-tauri/src/client/video.rs:481`。
- 并发 Range 下载时，每个 worker 的请求上界是 `current_begin + VIDEO_CHUNK_SIZE`，不是当前分片的 `end`；如果上游对越界 Range 处理严格，行为要看服务端：`src-tauri/src/client/video.rs:552-560`。
- 下载链路只依赖 `rtmp_url_hdv + Referer`，不带 `token`；如果真实上游后来开始校验 token/cookie，这条链会失效：`src-tauri/src/client/video.rs:456-465,502-507`。

### 真实待验证

- `live.sjtu.edu.cn` 是否稳定返回 `Accept-Ranges` / `Content-Range` / `Content-Length`，以及对越界 Range、并发 Range 的实际行为，需要真实响应头确认。
- `rtmp_url_hdv` 是否是短时签名 URL、是否会在三次重试窗口内过期，源码无法证明。

## 课程绑定总结

### 源码事实

- 链路的“起点课程”是 Canvas 课程列表里的 `course.id`，由 `get_canvas_videos(courseId)` 传入：`src/page/video.tsx:320-325`。
- 进入视频系统后，真正参与 `findVodVideoList` 的课程标识变成 `getAccessTokenByTokenId` 返回的 `data.params.courId`：`src-tauri/src/client/video.rs:347-358,377-380`。
- 之后字幕、PPT、AI 总结都不再用前端原始 `course.id`，而是用 `get_canvas_video_info` 返回的 `videoInfo.courId`：`src/page/video.tsx:362-367,381-403,739-744`。

### 基于源码可见的风险

- 代码里同时存在三种“课程标识”：Canvas `course.id`、token 交换得到的 `params.courId`、`VideoInfo.courId`。源码默认它们在业务上可串接，但没有显式校验三者一致性。

## 非 Canvas 的并行旧链路

### 源码事实

- `get_video_info` 是另一条旧视频站链路，不走 LTI；它对 `https://courses.sjtu.edu.cn/app/system/resource/vodVideo/getvideoinfos` 发请求，并通过 `oauth-consumer-key`、`oauth-nonce`、`oauth-path`、`oauth-signature` 头鉴权：`src-tauri/src/app/video.rs:59-61`，`src-tauri/src/client/video.rs:612-646`，`src-tauri/src/client/constants.rs:8-20`。
- Canvas 页面当前实际用的是 `get_canvas_video_info`，不是这条旧链路：`src/page/video.tsx:289-302`。

### 基于源码可见的风险

- `VideoInfo` / `VideoPlayInfo` 被新旧两条链路共用；如果两端返回字段语义不完全一致，问题会在更靠后的播放/下载阶段暴露，而不是在模型层区分。
