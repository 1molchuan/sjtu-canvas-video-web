# 前端下载模型

## 数据流

```text
代理模式：
用户点击轨道下载
  → POST .../ticket（同源 Cookie + 内存 CSRF）
  → 创建临时 <a href="download_url">
  → Axum 按 Range 流式代理上游

直连实验模式：
用户点击轨道下载
  → 立即调用系统“另存为”（保留用户手势）
  → POST .../ticket（同源 Cookie + 内存 CSRF）
  → fetch(download_url) 跟随受控 307
  → Response.body.pipeTo(本地文件 WritableStream)
  → 视频 body 从交大视频源直接到用户浏览器
```

代理模式不会用 `fetch()` 读取视频 body。直连实验模式会读取流，但只通过 `pipeTo()` 将分块响应写入用户明确选择的文件；两种模式都不调用 `response.blob()`，不使用 IndexedDB、Cache API、Service Worker 或前端视频缓存，也不会把完整录像放进 JavaScript heap。

## Ticket

`download_url` 是 60 秒内存能力值，但只知道 URL仍不能下载：请求还必须带同一网站 Session Cookie。ticket 固定绑定 Session、课程、录像、轨道、服务端登记的上游资源和安全文件名；它不编码上游 URL。

实验性的 `redirect_experimental` 服务端模式通过 Session API声明 `direct_stream`。前端先打开系统文件选择器，再签发 ticket 并流式写入文件。短期上游 URL会进入浏览器网络边界，服务器不承载视频 body，且无法继续控制源站响应与传输并发。该模式不支持 File System Access API时会明确报错，不会静默回退到服务器代理。风险与探测方法见 `direct-download-experiment.md`。

前端只短暂持有 ticket：

- 不显示、不复制、不写日志；
- 不进入 localStorage、sessionStorage 或 analytics；
- 代理模式的 anchor click 后立即从 DOM 删除；直连模式不创建 anchor；
- `410` 时允许用户重新点击以签发新 ticket，不复用旧值；
- `429` 显示已有下载正在进行，不伪造排队或进度；
- 代理模式只提示“下载已开始”；直连模式只提示进行中或完成，不伪造精确百分比。

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
