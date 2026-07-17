# Phase 1 真实协议验证 Runbook

本 Runbook 只用于操作者验证自己有权访问的课程。程序不会枚举 course ID，不接受 Canvas
Personal Access Token，也不会下载完整视频或写入视频文件。

## 1. 先运行 Mock 与质量检查

Mock 不需要交大账号，也不会访问真实交大服务：

```bash
cargo test -p canvas-core --test full_mock_protocol
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

完整 mock 测试覆盖 UUID、WebSocket、express login、Canvas SSO、稳定身份、Cookie 课程列表、
两段 LTI、token exchange、录像列表、详情、多轨和 Range 206。其余测试分别覆盖 200、416、
错误表单、恶意重定向、私网 IP、token 单次刷新、多上下文隔离和敏感输出扫描。

## 2. 获取合法的 course_id

只从本人已登录 Canvas 后能够打开的课程 URL 取值，例如：

```text
https://oc.sjtu.edu.cn/courses/12345
```

其中 `12345` 才是可传入的 `--course-id`。不要猜测、递增、扫描或从他人链接收集 ID。

## 3. 启用真实测试

真实请求默认关闭。macOS/Linux：

```bash
export SJTU_REAL_PROTOCOL_TEST=1
cargo run -p protocol-cli -- full --course-id 12345
```

PowerShell：

```powershell
$env:SJTU_REAL_PROTOCOL_TEST = "1"
cargo run -p protocol-cli -- full --course-id 12345
```

未设置精确值 `1` 时，CLI 在创建网络上下文前退出。该变量不是密码，只是防止 CI 或误操作
触发真实登录。

## 4. 命令与选项

```bash
cargo run -p protocol-cli -- login
cargo run -p protocol-cli -- discover-courses
cargo run -p protocol-cli -- inspect-course --course-id 12345
cargo run -p protocol-cli -- full --course-id 12345
```

全局选项：

- `--video-id <ID>`：仅适用于课程检查；ID 必须出现在该课程刚返回的授权录像列表中。
- `--debug`：增加安全诊断，不放宽脱敏。
- `--json-output`：最终报告写到 stdout；二维码和交互进度仍写到 stderr。
- `--timeout-seconds <1..900>`：扫码总超时；普通 HTTP 请求仍最多 30 秒。
- `--no-course-discovery`：已知合法 course ID 时跳过课程发现实验，直接验证 LTI。

CLI 在终端渲染二维码，不打印含 `sig` 的完整二维码 URL。按 Ctrl+C 会触发取消并等待后端
WebSocket 清理。

## 5. 解读结果

`full` 总会尽力写入 `.local/protocol-report.json`。报告中的步骤只使用脱敏状态：

- `passed`：该步骤在本次真实运行完成；
- `failed`：该步骤实际执行并失败；
- `not_run`：命令范围未包含或被显式跳过；
- `blocked`：上游前置步骤失败；
- `requires_personal_access_token`、`cookie_session_rejected`、`csrf_required`、
  `unsupported_response`、`upstream_changed`：课程发现实验的明确分类。

Go/No-Go：

- `go_a`：扫码、Canvas、Cookie 课程发现、LTI、录像、轨道、Range 均通过；
- `go_b`：视频链通过，但 Cookie 课程发现被明确拒绝或要求另一个授权机制；
- `no_go_c`：扫码、Canvas、LTI 或视频关键链真实失败；
- `undetermined`：关键步骤未运行、被阻塞，或证据仍不足。

`--no-course-discovery` 的成功视频链仍会保持 `undetermined`，因为本次没有取得课程发现证据。

## 6. 本地报告与清理

报告被 `.gitignore` 排除，且不得手动 `git add -f`：

```bash
rm -rf .local
```

PowerShell：

```powershell
Remove-Item -LiteralPath .local -Recurse -Force
```

报告不会包含 UUID、Cookie、token、tokenId、真实姓名、完整账号、course/video 名称、course ID
或完整 URL。它可以包含视频 host 和是否支持 Range。

## 7. Issue 中禁止上传的信息

不要上传：

- 终端二维码截图；
- `JAAuthCookie`、Canvas Cookie、请求 Cookie 头；
- `tokenId`、`token` header、LTI hidden-field 值；
- 完整 `Location`、视频 URL、URL query、HTML/JSON 响应体；
- 真实姓名、学号、账号、course/video 名称或本地 report 之外的资料。

可以安全提供：

- commit SHA、操作系统、Rust 版本；
- 失败步骤和结构化 error class；
- HTTP 状态、Content-Type、响应长度；
- 仅 host（无路径/query）、Cookie 名称列表、hashed ID；
- `.local/protocol-report.json`，但上传前仍应人工复核。

真实页面或接口发生变化时，应提交脱敏的“缺失字段名/错误类别”，不要提交完整响应来换取
临时兼容。

## 8. 2026-07-17 Phase 1.5 实际验证记录

本次验证在 Windows、Rust 1.97.1 下执行。真实测试门控已显式设置，二维码由用户本人扫描，
课程 ID 来自用户本人能够打开的 Canvas 课程 URL；仓库和文档均不保留具体值。

实际结果：jAccount UUID、WebSocket、QR、express login、Canvas login、稳定身份、Cookie-only
课程发现、LTI、录像列表、录像详情、轨道和 Range probe 全部 `passed`，判定为 `go_a`。
课程接口在不发送 Bearer token 时返回 JSON 课程数组；身份来源为 Canvas authenticated-self
响应；视频源 host 为 `live.sjtu.edu.cn`；`bytes=0-0` 探测确认支持 Range。未下载完整视频，
也未测试移除 Referer 后的行为。

真实环境要求两项兼容修复：课程数组中的部分条目缺少显示字段；OIDC 首次 POST 后经过
Canvas 同源重定向链。实现对显示字段使用明确缺省值，并通过 redirect-disabled client 手动
处理最多八跳的精确 Canvas origin；循环、跨用途 host 和缺失 Location 均继续失败。对应
Mock 回归测试必须在后续变更中保持通过。
