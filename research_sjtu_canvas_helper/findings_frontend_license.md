# Findings: Frontend + License

调研对象固定为 `research_sjtu_canvas_helper/SJTU-Canvas-Helper` 的提交 `b5d895af57aaa74dfd53cef80dfb64c76c023c20`。下文中“源码事实”仅引用该提交内可直接看到的证据；“真实待验证”表示需要运行时、线上端点或权利人确认后才能下结论。

## 源码事实

### 1. `src/page/video.tsx` 到 Tauri command / event 的调用矩阵

| 前端位置 | 调用 | 前端参数 | 前端状态/用途 | 后端落点 |
| --- | --- | --- | --- | --- |
| `src/page/video.tsx:142` | `invoke("login_canvas_website")` | 无 | `loginAndCheck` 用它判断视频站额外登录是否有效；失败时会把 `config.ja_auth_cookie` 清空并 `saveConfig`，并驱动 `notLogin`/`loaded`/`showLoginRequiredDialog`。`src/page/video.tsx:260`, `src/lib/config.ts:12`, `src/lib/config.ts:22` | `src-tauri/src/main.rs:826` |
| `src/page/video.tsx:291` | `invoke("get_canvas_video_info", { videoId })` | `{ videoId: string }` | 取 `VideoInfo`，再把 `videoPlayResponseVoList` 派生为 `plays`，并按 index 生成下载名。`VideoInfo` / `VideoPlayInfo` 定义在 `src/lib/model.ts:432`, `src/lib/model.ts:444` | `src-tauri/src/main.rs:846` |
| `src/page/video.tsx:322` | `invoke("get_canvas_videos", { courseId })` | `{ courseId: number }` | 拉课程录像列表，写入 `videos`。`CanvasVideo` 定义在 `src/lib/model.ts:403` | `src-tauri/src/main.rs:821` |
| `src/page/video.tsx:355` | `save(...)` | `defaultPath`, `filters` | Tauri 保存对话框，用于字幕下载目标路径；取消则直接返回。 | Tauri dialog plugin，非 Rust command |
| `src/page/video.tsx:365` | `invoke("download_subtitle", { canvasCourseId, savePath })` | `{ canvasCourseId: number, savePath: string }` | 直接下载字幕到本地路径，成功仅 toast，不维护任务表。 | `src-tauri/src/main.rs:874` |
| `src/page/video.tsx:400` | `invoke("start_subtitle_chat_stream", { requestId, canvasCourseId, messages })` | `{ requestId: string, canvasCourseId: number, messages: LLMChatMessage[] }` | 打开 AI 总结会话，先把 assistant 消息标成 `pending`，再靠事件流更新。`LLMChatMessage` 定义在 `src/lib/model.ts:110` | `src-tauri/src/main.rs:470` |
| `src/page/video.tsx:429` | `invoke("start_subtitle_chat_stream", ...)` | 同上 | 继续追问，受 `summaryChatLoading` 和 `summaryChatCourseId` 控制。 | `src-tauri/src/main.rs:470` |
| `src/page/video.tsx:476` | `invoke("download_ppt", { courseId, savePath })` | `{ courseId: number, savePath: string }` | 先入 `pptDownloadTasks`，初始状态 `"downloading"`；Promise resolve 后改 `"completed"`，reject 后改 `"fail"`。`DownloadTask` 定义在 `src/lib/model.ts:273` | `src-tauri/src/main.rs:884` |
| `src/page/video.tsx:505` | `invoke("delete_file_with_name", { name })` | `{ name: string }` | 删除视频下载任务对应文件，前端只传文件名，不传绝对路径。 | `src-tauri/src/main.rs:563` |
| `src/page/video.tsx:519` | `invoke("delete_path_file", { path })` | `{ path: string }` | 删除 PPT 任务对应文件，优先走绝对路径。 | `src-tauri/src/main.rs:568` |
| `src/page/video.tsx:546` | `invoke("prepare_proxy")` | 无 | 首次播放前启动本地反向代理；成功则销毁 loading toast，失败则 `stop_proxy`。 | `src-tauri/src/main.rs:851` |
| `src/page/video.tsx:555` | `invoke("stop_proxy")` | 无 | 代理启动失败时和组件卸载时停止本地代理。 | `src-tauri/src/main.rs:856` |
| `src/page/video.tsx:562` | `getConfig()` -> `invoke("get_config")` | 无 | 读取 `proxy_port`，把远端 `https://live.sjtu.edu.cn` 替换为 `http://localhost:${proxyPort}` 后再喂给 `<video>`。`AppConfig.proxy_port` 定义在 `src/lib/model.ts:299` | `src/lib/config.ts:12`, `src-tauri/src/main.rs:353` |
| `src/page/video.tsx:739` | `invoke("get_canvas_video_info", { videoId })` | `{ videoId: string }` | 播放时再次拿 `courId`，随后取字幕文本。 | `src-tauri/src/main.rs:846` |
| `src/page/video.tsx:742` | `invoke("get_subtitle", { canvasCourseId })` | `{ canvasCourseId: number }` | 拉字幕文本后在前端 `srtToVtt`，再转成 `Blob URL` 挂到 `<track>`。 | 后端实现可在 `src-tauri/src/client/ai.rs:151` 与 `src-tauri/src/client/video.rs:651` 看到被调用；`main.rs` 注册命令未在本次前端范围内逐段展开 |
| `src/page/video.tsx:173` | `listen("video_ai_chat://chunk")` | event payload: `FileChatStreamChunkPayload` | 将最后一条 assistant 消息追加流式内容。payload 类型在 `src/lib/model.ts:124` | `src-tauri/src/main.rs:481` |
| `src/page/video.tsx:196` | `listen("video_ai_chat://done")` | event payload: `FileChatStreamDonePayload` | 结束当前会话流，清空 `activeSummaryRequestIdRef`，把 `pending` 置回 `false`。payload 类型在 `src/lib/model.ts:129` | `src-tauri/src/main.rs:496` |
| `src/page/video.tsx:222` | `listen("video_ai_chat://error")` | event payload: `FileChatStreamErrorPayload` | 把最后一条 assistant 消息标成 `error: true`，同时结束 loading。payload 类型在 `src/lib/model.ts:134` | `src-tauri/src/main.rs:505` |

### 2. `video.tsx` 直接 imports 里的下载/播放链条

| 前端位置 | 调用/事件 | 前端参数 | 状态机/流程 | 后端落点 |
| --- | --- | --- | --- | --- |
| `src/components/video_download_table.tsx:64` | `appWindow.listen("video_download://progress")` | `ProgressPayload { uuid, processed, total }` | 进度事件到达后，按 `uuid` 更新任务百分比。`ProgressPayload` 定义在 `src/lib/model.ts:348` | `src-tauri/src/main.rs:868` |
| `src/components/video_download_table.tsx:97` | `invoke("download_video", { video, saveName })` | `{ video: VideoPlayInfo, saveName: string }` | 收到 `tasks` 后自动启动下载；每个任务最多重试 3 次，状态只会落到 `"downloading" / "succeed" / "fail"`。`DownloadState` 定义在 `src/lib/model.ts:281` | `src-tauri/src/main.rs:861` |
| `src/components/video_download_table.tsx:148` | `invoke("open_save_dir")` | 无 | 打开本地保存目录。 | `src-tauri/src/main.rs:538` |
| `src/components/ppt_download_table.tsx:51` | `appWindow.listen("ppt_download://progress")` | `ProgressPayload` | 进度 < 100 时状态为 `"downloading"`，进度到 100 时状态先切到 `"merging"`，最终由 `video.tsx` Promise resolve 改成 `"completed"`。 | `src-tauri/src/main.rs:891` |
| `src/components/video_aggregator.tsx:79` | `invoke("is_ffmpeg_installed")` | 无 | 检查本机 `ffmpeg`；结果落到 `"unknown" / "installed" / "uninstalled"`。 | `src-tauri/src/main.rs:53` |
| `src/components/video_aggregator.tsx:93` | `appWindow.listen("ffmpeg://output")` | `string` | 将本地 ffmpeg stdout/stderr 追加到 UI 日志。 | `src-tauri/src/app/basic.rs:1326`, `src-tauri/src/app/basic.rs:1336`, `src-tauri/src/app/basic.rs:1347` |
| `src/components/video_aggregator.tsx:114` | `invoke("run_video_aggregate", { params })` | `VideoAggregateParams` | `params` 来自本地表单，提交前会拼接 `.mp4`；运行期间 `running=true`。`VideoAggregateParams` 定义在 `src/lib/model.ts:610` | `src-tauri/src/main.rs:203` |
| `src/components/path_selector.tsx:44` | `open(...)` | `{ directory, defaultPath, filters }` | 打开本地文件/目录选择器；默认目录来自 `getConfig(true)` 的 `save_path`。 | Tauri dialog plugin，非 Rust command |

### 3. `src/lib/hooks.tsx` 到 Tauri command 的调用矩阵

| 前端位置 | 调用 | 前端参数 | 前端状态/用途 | 后端落点 |
| --- | --- | --- | --- | --- |
| `src/lib/hooks.tsx:312` | `invoke("convert_pptx_to_pdf", { file })` | `{ file: File }` | `useMerger.mergePDFs` 中把 `.pptx` 转成 PDF 字节，再交给 `pdf-merger-js/browser`。 | `src-tauri/src/main.rs:627` |
| `src/lib/hooks.tsx:317` | `invoke("convert_docx_to_pdf", { file })` | `{ file: File }` | 同上，但针对 `.docx`。 | `src-tauri/src/main.rs:633` |
| `src/lib/hooks.tsx:378` | `invoke("save_file_content", { content, fileName })` | `{ content: number[], fileName: string }` | 合并结果下载时按 4 MB 分块写回本地文件。 | `src-tauri/src/main.rs:622` |
| `src/lib/hooks.tsx:473` | `invoke("get_uuid")` | 无 | `useQRCode.showQRCode` 获取扫码登录 uuid，并据此建立 `wss://jaccount.sjtu.edu.cn/jaccount/sub/{uuid}`。 | `src-tauri/src/main.rs:801` |
| `src/lib/hooks.tsx:487` | `invoke("express_login", { uuid })` | `{ uuid: string }` | 扫码成功后取 `JAAuthCookie`，再经 `saveConfig` 写入本地配置。 | `src-tauri/src/main.rs:806`, `src/lib/config.ts:22`, `src-tauri/src/main.rs:659` |
| `src/lib/hooks.tsx:595` | `invoke(command, args)` | 泛型 | `useData` 是统一命令泵，所有只读数据 hook 都走这里。 | 见下表 |
| `src/lib/hooks.tsx:767` | `getConfig(true)` -> `invoke("get_config")` | 无 | `useBaseURL` 根据 `account_type` 在 `BASE_URL` / `JI_BASE_URL` 间切换。 | `src/lib/config.ts:12`, `src-tauri/src/main.rs:353` |

`useData` 包装出的具体 command：

| Hook | 命令 | 参数 | 位置 |
| --- | --- | --- | --- |
| `useCourseSyllabus` | `get_course_syllabus` | `{ courseId }` | `src/lib/hooks.tsx:618` |
| `useCourses` | `list_courses` | 无 | `src/lib/hooks.tsx:625`，后端 `src-tauri/src/main.rs:241` |
| `useTAOrTeacherCourses` | `list_courses` | 无 | `src/lib/hooks.tsx:634`，后端 `src-tauri/src/main.rs:241` |
| `useMe` | `get_me` | 无 | `src/lib/hooks.tsx:664` |
| `useUserSubmissions` | `list_user_submissions` | `{ courseId, studentIds }` | `src/lib/hooks.tsx:668` |
| `useAssignments` | `list_course_assignments` | `{ courseId }` | `src/lib/hooks.tsx:685` |
| `useStudents` | `list_course_students` | `{ courseId }` | `src/lib/hooks.tsx:702` |
| `useRelationship` | `collect_relationship` | 无 | `src/lib/hooks.tsx:715` |
| `useFolderFiles` | `list_folder_files` | `{ folderId }` | `src/lib/hooks.tsx:719` |
| `useFolderFolders` | `list_folder_folders` | `{ folderId }` | `src/lib/hooks.tsx:728` |
| `useCourseFolders` | `list_course_folders` | `{ courseId }` | `src/lib/hooks.tsx:737` |
| `useAnnualReport` | `generate_annual_report` | `{ year }` | `src/lib/hooks.tsx:777` |
| `useExternalFiles` | `list_external_module_items` | `{ courseId }` | `src/lib/hooks.tsx:788` |

### 4. 下载流程与状态机

#### 4.1 视频下载

1. `video.tsx` 点击“下载”只做入队，不直接调用后端；新任务初始值是 `progress: 0`、`state: "downloading"`。`src/page/video.tsx:331`
2. `VideoDownloadTable` 在 `tasks` 变化时发现新 key，就自动执行 `invoke("download_video", { video, saveName })`。`src/components/video_download_table.tsx:78`, `src/components/video_download_table.tsx:97`
3. 后端通过 `video_download://progress` 推送 `{ uuid, processed, total }`；前端按 `(processed / total) * 100` 改写进度。`src/components/video_download_table.tsx:64`, `src-tauri/src/main.rs:868`
4. `updateTaskProgress` 只会把状态落成 `"downloading" / "succeed" / "fail"`，其中 `"succeed"` 由 `progress === 100` 判定。`src/components/video_download_table.tsx:122`
5. 失败后会自动重试至多 3 次，间隔 `sleep(1000)`；手工“重试”只在当前状态是 `"fail"` 时可点。`src/components/video_download_table.tsx:93`, `src/components/video_download_table.tsx:109`
6. 删除任务会先从 React state 中移除，再调用 `delete_file_with_name` 删本地文件。`src/page/video.tsx:500`

#### 4.2 PPT 下载

1. `video.tsx` 先用 `get_canvas_video_info` 拿到 `courId`，再弹出保存对话框。`src/page/video.tsx:450`
2. 选定路径后，任务以 `taskKey = "ppt_" + outputPath` 入队，状态初始为 `"downloading"`。`src/page/video.tsx:461`
3. `download_ppt` Promise 运行期间，`PPTDownloadTable` 监听 `ppt_download://progress`；当 `processed / total === 100%` 时，表格状态先转 `"merging"`。`src/components/ppt_download_table.tsx:51`, `src/components/ppt_download_table.tsx:66`
4. 后端 Promise resolve 后，`video.tsx` 把状态最终改成 `"completed"`；reject 则改 `"fail"`。`src/page/video.tsx:476`
5. 删除优先走 `delete_path_file(outputPath)`；只有没有 `outputPath` 时才退回 `delete_file_with_name(name)`。`src/page/video.tsx:513`

#### 4.3 字幕与 AI 总结

1. 字幕下载是“一次性命令 + 本地保存路径”，没有任务表。`src/page/video.tsx:349`
2. 播放器字幕是另一条链：播放时用 `get_canvas_video_info` -> `get_subtitle` 取纯文本，再前端转 VTT 并挂 `<track>`。`src/page/video.tsx:732`
3. AI 总结把 request id 存在 `activeSummaryRequestIdRef`，靠 `video_ai_chat://chunk/done/error` 三类事件流更新最后一条 assistant 消息。`src/page/video.tsx:167`

#### 4.4 文档合并下载

1. `useMerger.mergePDFs` 在浏览器侧用 `pdf-merger-js/browser` 合并；仅当文件后缀是 `.pptx` / `.docx` 时借助 Tauri 转 PDF。`src/lib/hooks.tsx:285`
2. 结果下载不走浏览器 `download`，而是把 `Blob` 切成 4 MB 数组后多次 `save_file_content`。`src/lib/hooks.tsx:356`

### 5. 多人 Web 不可移植点

1. `video.tsx`、`hooks.tsx`、`video_download_table.tsx`、`ppt_download_table.tsx`、`video_aggregator.tsx`、`path_selector.tsx` 都直接依赖 `@tauri-apps/api/*` 或 `@tauri-apps/plugin-dialog`，当前实现假定前端运行在桌面 WebView，而不是标准浏览器。`src/page/video.tsx:1`, `src/lib/hooks.tsx:8`, `src/components/video_download_table.tsx:1`, `src/components/ppt_download_table.tsx:16`, `src/components/video_aggregator.tsx:23`, `src/components/path_selector.tsx:9`
2. 本地文件系统是主路径，不是可选增强：字幕、PPT、合并 PDF、视频聚合输入输出、打开保存目录、删除文件都依赖用户本机绝对路径。`src/page/video.tsx:355`, `src/page/video.tsx:453`, `src/components/path_selector.tsx:44`, `src/components/video_download_table.tsx:148`, `src/lib/hooks.tsx:378`
3. 视频播放依赖桌面侧本地代理：前端把远端 `https://live.sjtu.edu.cn` 强制改写到 `http://localhost:${proxyPort}`，并用 `prepare_proxy/stop_proxy` 控制本地服务生命周期。这一模型天然不是多用户 Web 的共享后端模型。`src/page/video.tsx:530`, `src/page/video.tsx:536`, `src/lib/model.ts:299`
4. 下载进度、AI 总结、ffmpeg 输出都依赖当前 WebView 窗口上的本地事件总线；在多人 Web 中，这类状态必须替换成按用户隔离的服务端任务和推送通道。`src/page/video.tsx:173`, `src/components/video_download_table.tsx:64`, `src/components/ppt_download_table.tsx:51`, `src/components/video_aggregator.tsx:93`
5. `useQRCode` 直接建立到 `wss://jaccount.sjtu.edu.cn/jaccount/sub/{uuid}` 的浏览器 WebSocket，扫码成功后再把 `JAAuthCookie` 写入本地配置；这是假定“每台机器一个人”的桌面模型。`src/lib/hooks.tsx:454`, `src/lib/hooks.tsx:473`, `src/lib/hooks.tsx:487`, `src/lib/config.ts:22`, `src/lib/model.ts:297`
6. `delete_file_with_name` 前端只传文件名，不传用户上下文和目录；在桌面单用户里问题较小，但如果迁到多人共享后端，这个调用约定本身不够隔离。`src/page/video.tsx:505`, `src-tauri/src/main.rs:563`
7. `save_file_content` 前端也只传 `fileName` 和字节块，不传目录；这同样默认后端拥有单用户默认保存目录。`src/lib/hooks.tsx:378`, `src-tauri/src/main.rs:622`
8. 视频聚合依赖本机 `ffmpeg` 可执行文件和本机进程输出捕获，不是纯前端能力。`src/components/video_aggregator.tsx:79`, `src/components/video_aggregator.tsx:114`, `src-tauri/src/main.rs:53`, `src-tauri/src/main.rs:203`

### 6. 可能失效的常量 / 字面量契约

1. `BASE_URL = "https://oc.sjtu.edu.cn"`、`JI_BASE_URL = "https://jicanvas.com"` 是硬编码站点入口；如果学校域名、反代或租户入口调整，`useBaseURL` 会直接失效。`src/lib/constants.ts:1`, `src/lib/hooks.tsx:763`
2. `QRCODE_BASE_URL = "https://jaccount.sjtu.edu.cn/jaccount/confirmscancode"` 与 `WEBSOCKET_BASE_URL = "wss://jaccount.sjtu.edu.cn/jaccount/sub"` 把扫码协议端点写死了。`src/lib/hooks.tsx:50`
3. `SEND_INTERVAL = 25000` 与 `QRCODE_TIMEOUT = 8000` 是协议时序常量；如果登录端节奏变化，前端会表现为“二维码连接未建立成功”或“二维码获取超时”。`src/lib/hooks.tsx:49`, `src/lib/hooks.tsx:52`, `src/lib/hooks.tsx:515`, `src/lib/hooks.tsx:541`
4. `getVidePlayURL` 里硬编码替换 `"https://live.sjtu.edu.cn"`，把播放链路锁死到当前直播域名和本地代理模型。`src/page/video.tsx:530`
5. `DownloadState` 定义了 `"wait_retry"`，但 `VideoDownloadTable` 的 `stateMeta` 和 `updateTaskProgress` 从不生成或显示它，当前前端真实可达状态只有 `"downloading" / "succeed" / "fail"`；这个枚举成员更像遗留契约。`src/lib/model.ts:281`, `src/components/video_download_table.tsx:33`, `src/components/video_download_table.tsx:122`

## License / Copyright

### 7. 仓库内可见的许可与版权事实

1. `LICENSE` 是标准 MIT 文本，版权行为 `Copyright (c) 2025 Zihong Lin`。`LICENSE:1`, `LICENSE:3`
2. `src-tauri/Cargo.toml` 只有 `authors = ["Okabe"]`，没有 `license` 或 `license-file` 字段。`src-tauri/Cargo.toml:5`
3. `package.json` 是 `private: true` 的前端包，也没有 `license` 字段。`package.json:2`, `package.json:4`
4. 截至固定提交 `b5d895af57aaa74dfd53cef80dfb64c76c023c20`，本地 `git rev-list --count` 结果为 `1`，即当前可见历史只有一个提交。该提交日期是 `2026-07-04`，作者与提交者都是 `Okabe <923048992@qq.com>`。`git log -1 b5d895af57aaa74dfd53cef80dfb64c76c023c20`
5. `git shortlog -sne --all` 只列出一个提交作者：`Okabe <923048992@qq.com>`。
6. 仓库 remote 指向 `https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper.git`。`.git/config:9`
7. 同一仓库的站点元数据把 GitHub 主页映射到作者名 `Zihong Lin`，同时声明 MIT。`website/index.html:39`, `website/index.html:40`, `website/index.html:42`

### 8. 准确的 MIT NOTICE 做法

1. 法律最低要求不是“必须有 NOTICE 文件”，而是必须保留原 MIT 版权声明和许可文本；也就是你分发源码、二进制或其“substantial portions”时，都要把上游 `LICENSE` 文本一并带上。
2. 对这个仓库，最稳妥的做法是“原样保留上游 LICENSE 文本”，不要擅自把 `Copyright (c) 2025 Zihong Lin` 改写成 `Okabe`，因为仓库内虽然存在 `Zihong Lin <-> Okabe-Rintarou-0` 的弱关联（站点元数据），但没有法律层面的明确声明。
3. 如果你的项目已经有 `NOTICE` / `THIRD_PARTY_NOTICES.md`，建议写“来源说明”，但把完整 MIT 文本放在单独文件里，例如：

```text
This product includes code derived from SJTU-Canvas-Helper
(https://github.com/Okabe-Rintarou-0/SJTU-Canvas-Helper),
retrieved from commit b5d895af57aaa74dfd53cef80dfb64c76c023c20 (2026-07-04).

Upstream license: MIT
Upstream copyright notice:
Copyright (c) 2025 Zihong Lin

Modifications in this distribution: [your org/name], [year].
The full MIT license text is reproduced in third_party/licenses/SJTU-Canvas-Helper-MIT.txt.
```

4. 如果是“直接 vendoring 源码子目录”，更简单也更准确的做法是在 vendored 目录保留原 `LICENSE` 副本，并在总项目的第三方许可证清单中记录：
   `SJTU-Canvas-Helper`, upstream URL, pinned commit `b5d895af57aaa74dfd53cef80dfb64c76c023c20`, license `MIT`, modifications `yes/no`。
5. 如果只是“参考实现后重写”，没有复制受保护表达，仅保留思路，则不需要给 MIT NOTICE；但一旦复制了代码、结构化文本、组件样式或 substantial portions，就应按上面的保留文本方式处理。

## 真实待验证

1. `Zihong Lin` 与 `Okabe` 是否为同一法律意义上的著作权主体，仓库源码不能单独证明；如果你要在 NOTICE 里统一写法，应该向上游确认。
2. `BASE_URL`、`JI_BASE_URL`、`QRCODE_BASE_URL`、`WEBSOCKET_BASE_URL`、`https://live.sjtu.edu.cn` 这些硬编码端点在 `2026-07-16` 是否仍然有效，源码本身无法证明，只能通过线上连通性验证。
3. `get_subtitle` 的前端调用链是直接可见的，后端底层实现也能在 `src-tauri/src/app/video.rs:95` 与 `src-tauri/src/client/video.rs:651` 看到；但本次对 `src-tauri/src/main.rs` 的文本检索没有直接命中同名 `async fn get_subtitle`，所以迁移实施前仍应补核 command 暴露路径。
4. `delete_file_with_name` 与 `save_file_content` 在桌面端是否总是落到“每用户独立目录”，前端看不到；多人 Web 迁移前需要确认后端目录隔离策略。
