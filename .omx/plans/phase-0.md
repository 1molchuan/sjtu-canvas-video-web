# SJTU Canvas Video Web — Phase 0 执行计划

## 需求摘要

本阶段仅完成参考仓库调研、协议验证设计和新项目骨架。不得把 mock 或源码推断写成真实环境验证，不实现前端页面，也不向浏览器暴露任何上游 Cookie、token 或视频 URL。

## 验收标准

- 固定并记录 `Okabe-Rintarou-0/SJTU-Canvas-Helper` 的参考提交 SHA。
- 阅读用户指定的 8 个源码文件及其直接调用链，而不是只阅读 README。
- `docs/reference-analysis.md` 覆盖 jAccount、Canvas、LTI、视频列表/详情/下载、Range 与 Referer。
- `docs/protocol-validation.md` 给出 Phase 1 CLI 的逐步验证计划，并显式区分源码确认、mock 验证和真实环境待验证。
- 创建可编译的 Rust workspace 骨架，协议核心层不依赖 Axum，服务层与部署层分离。
- 保留上游 MIT 许可证归属，且不复制无关 Tauri 功能。
- 本阶段能执行的格式化、静态检查和测试全部通过；缺失工具或真实凭据必须明确记录。

## 实施步骤

1. 盘点工作区、Git/Rust/Node 工具链与本地约束。
2. 在 `research_sjtu_canvas_helper/` 写研究计划，浅克隆参考仓库并记录提交 SHA。
3. 分别追踪认证/Canvas、LTI/视频、前端调用/下载三条证据链。
4. 创建 `canvas-core`、`protocol-cli`、`server`、`frontend` 占位层和部署配置层骨架。
5. 写分析、协议验证、安全边界和阶段状态文档。
6. 安装或启用必要工具链后执行 `cargo fmt --check`、`cargo check --workspace`、`cargo test --workspace` 和敏感信息扫描。

## 风险与缓解

- 上游接口可能已变化：所有常量标注“参考提交中的值”，Phase 1 必须用真实流量重新验证。
- Canvas Cookie 是否可列课程未知：作为首个 Go/No-Go，不预设 Personal Access Token 可省略。
- Windows 本地环境可能缺少 Rust：使用官方 rustup 安装用户级工具链，并在文档记录版本。
- 参考代码包含桌面端单用户假设：只提取协议证据，不直接移植全局 Client 或 Tauri 状态。

## 验证步骤

- 所有文档中的源码结论附参考文件、符号和固定提交链接。
- `cargo metadata --no-deps` 确认 workspace 边界。
- `cargo fmt --check`、`cargo check --workspace`、`cargo test --workspace` 返回 0。
- `rg` 扫描账号、Cookie、token、私有 URL 等高风险字样，人工复核示例占位符。

