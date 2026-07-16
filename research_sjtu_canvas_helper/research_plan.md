# SJTU Canvas Helper 源码调研计划

## 主问题

在固定版本的 `Okabe-Rintarou-0/SJTU-Canvas-Helper` 中，jAccount 扫码登录、Canvas Web 会话、课程视频 LTI 跳转、视频列表/详情和下载代理所依赖的真实协议调用链是什么；哪些逻辑能安全重构到多人 Web 服务，哪些结论仍需真实账号验证？

## 子课题

### 1. jAccount 与 Canvas 登录

调查 UUID、二维码签名、WebSocket LOGIN 事件、`express_login`、`JAAuthCookie`、Canvas 登录态和用户身份相关的函数、请求参数、Cookie 边界与错误处理。预期输出：调用顺序、来源文件/符号、敏感状态清单，以及 Web 服务化时必须拆除的单用户假设。

### 2. LTI 1.3 与课程级视频授权

调查 Canvas external tool、OIDC initiation、LTI 表单、重定向、`tokenId`、token 交换和课程绑定。预期输出：每个 HTTP 跳转、隐藏字段、重定向控制、解析假设和潜在失效点。

### 3. 视频列表、详情与下载语义

调查 `findVodVideoList`、`getVodVideoInfos`、轨道模型、源 URL、Referer、Range 请求及响应头处理。预期输出：接口与模型映射、浏览器实际调用路径、下载安全约束和 mock 测试输入。

### 4. 许可、维护状态与失效风险

调查上游 MIT 许可证、版权归属、参考提交日期、依赖/常量和仓库近期变更。预期输出：合规做法、可借鉴范围、不可复制内容，以及所有必须在 Phase 1 真实验证的常量/接口。

## 综合方法

先浅克隆并固定提交 SHA，再按三个互不重叠的源码区域形成 findings 文件。最终由主任务逐项回看原始源码，将证据综合进 `docs/reference-analysis.md`；每条结论标为“源码确认”“合理推断”或“真实环境待验证”，并使用固定提交的 GitHub permalink。
