# SJTU Canvas Video Web — Phase 1 执行计划

## 起点

- Phase 0 baseline：`ddb54bbae5015cb74be769ed1a097abd38811e9b`
- 起始工作区：clean
- 参考实现：`SJTU-Canvas-Helper@b5d895af57aaa74dfd53cef80dfb64c76c023c20`
- 真实交大请求：默认关闭，仅在 `SJTU_REAL_PROTOCOL_TEST=1` 时由用户主动执行

## 阶段边界

本阶段只实现 `canvas-core` 协议能力、`protocol-cli` 编排、本地脱敏报告和 Mock 测试。保留
现有 health server，不实现 React、正式 Web Session、CSRF、白名单接入、ticket、流式下载、
视频写盘或部署。

## 架构决定

1. 每个 `ProtocolContext` 创建独立 `reqwest::Client` 与内存 Cookie Store。
2. 同一 Cookie Store 由受控重定向 client 和禁用重定向 client 共享。
3. 所有真实端点由 `ProtocolEndpoints` 注入；Mock 使用显式 `Mock` URL 策略。
4. URL 校验按 `UpstreamPurpose` 使用精确 host，不依赖字符串后缀。
5. LTI 表单保存有序重复字段，token 使用 `SecretString` 并绑定 Canvas course ID。
6. CLI 只编排和显示脱敏结果；协议 HTTP/WebSocket/HTML/JSON 逻辑全部位于 `canvas-core`。
7. Range 验证只读取响应头并立即丢弃 body，不写文件。

## TDD 顺序

每一项先提交失败测试并运行确认 RED，再实现最小生产代码转 GREEN：

1. URL allowlist、URL 摘要、ID hash、Cookie 名和错误 Display 脱敏。
2. UUID HTML、QR WebSocket 消息、未知事件和二维码 URL 构造。
3. 独立 Cookie Context、express login Cookie 捕获和连接关闭/超时。
4. Canvas SSO 最终 host、Cookie 名、身份 JSON 和课程发现分类。
5. Dashboard bootstrap 课程提取与 REST Cookie 拒绝分支。
6. OIDC/LTI 表单 action、重复字段、Location/tokenId 和 token 交换。
7. 视频列表、详情、多轨、token 过期和课程 token 绑定。
8. Range `206`、无 Range 的 `200`、`416`、允许/拒绝重定向。
9. 完整 Mock 流程与 CLI 参数/报告序列化。

## Mock 证据矩阵

- HTTP：UUID、express cookie、Canvas SSO/API/dashboard、external tool、OIDC、LTI、token exchange、
  视频列表、视频详情、Range。
- WebSocket：QR update、LOGIN、未知消息、连接关闭、超时。
- 结构变化：缺表单、错 action、缺 Location、缺 tokenId、缺 token/course/video/track。
- 安全：非 allowlist、IP literal、HTTP 真实模式、secret 日志/错误/报告扫描。
- 状态：课程 A/B token 独立、详情 token 过期后只刷新一次。

## 完成标准

- `protocol-cli` 支持 `login`、`discover-courses`、`inspect-course`、`full` 与要求的全局选项。
- Mock 覆盖完整成功链和要求的失败分支，且不访问真实交大服务。
- `.local/protocol-report.json` 只含步骤状态和脱敏元数据，并被 Git 忽略。
- 文档明确区分自动测试、Mock、真实 `passed/failed/not_run/blocked` 和 Go/No-Go。
- fmt、check、Clippy `-D warnings`、全部测试及安全扫描通过。
