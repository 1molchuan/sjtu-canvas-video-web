# 前端下载模型

## 数据流

```text
用户点击轨道下载
  → POST .../ticket（同源 Cookie + 内存 CSRF）
  → { download_url, expires_in_seconds }
  → 创建临时 <a href="download_url">
  → 浏览器原生导航/下载
  → GET /api/download/:ticket（浏览器自动携带 Session Cookie）
  → 默认由 Axum 按 Range 流式代理上游
```

前端不会用 `fetch()` 读取视频 body，不调用 `response.blob()`，不使用 IndexedDB、Cache API、Service Worker 或前端视频缓存。这样浏览器与后端可以保留原生 Range、暂停、续传和取消语义，也不会把完整录像放进 JavaScript heap。

## Ticket

`download_url` 是 60 秒内存能力值，但只知道 URL仍不能下载：请求还必须带同一网站 Session Cookie。ticket 固定绑定 Session、课程、录像、轨道、服务端登记的上游资源和安全文件名；它不编码上游 URL。

实验性的 `redirect_experimental` 服务端模式不改变前端流程，但下载端点会返回 `307`，使浏览器直接访问短期上游 URL。该 URL会进入浏览器网络边界，且服务器无法继续控制 Range、文件名和并发；默认配置不会启用。风险与探测方法见 `direct-download-experiment.md`。

前端只短暂持有 ticket：

- 不显示、不复制、不写日志；
- 不进入 localStorage、sessionStorage 或 analytics；
- anchor click 后立即从 DOM 删除；
- `410` 时允许用户重新点击以签发新 ticket，不复用旧值；
- `429` 显示已有下载正在进行，不伪造排队或进度；
- 只提示“下载已开始”，不声明无法观测的精确百分比。

## Range 与响应

后端允许一个合法 `bytes=` range，支持闭区间、开放区间和 suffix range；多 Range 与非法 Range 返回明确 `416`。上游的 `Content-Type`、`Content-Length`、`Content-Range`、`Accept-Ranges`、`Last-Modified` 和安全 `ETag` 才可转发。

下载响应由后端设置：

```text
Content-Disposition: attachment; ...
Cache-Control: private, no-store
X-Content-Type-Options: nosniff
```

`Set-Cookie`、hop-by-hop headers、上游文件名和任意重定向不会透传。客户端取消会 drop 上游 body 并释放全局/每用户 semaphore permit，不生成完整临时文件。

## Cloudflare 验收边界

本地 `206` 只证明 Axum 行为。生产域名必须重新验证：状态仍为 `206`、`Content-Range` 和长度语义正确、`CF-Cache-Status` 不是 `HIT`、`Cache-Control` 为 `private, no-store`，取消后 permit 释放。完整步骤见 `production-acceptance.md`。
