# 一次性邀请链接

一次性邀请链接用于让受邀朋友自行完成 jAccount 扫码和白名单登记。维护者不需要替朋友扫码，也不需要接触对方的账号、Cookie 或稳定身份。

## 工作流

1. 维护者在服务器上生成一个短期邀请链接。
2. 链接中的令牌位于 `#invite=...` URL fragment；Cloudflare、反向代理和普通 HTTP 请求日志不会收到 fragment。
3. 浏览器读取令牌后立即清除地址栏 fragment，只在 React 内存中保存。
4. 开始扫码时，浏览器在 `POST /api/auth/qr/start` 的 JSON body 中提交令牌。
5. 用户用自己的 jAccount 扫码。稳定身份验证成功后，服务端原子消费邀请并把该身份哈希加入动态白名单。
6. 此后该用户可以直接从普通登录页扫码，不再需要邀请链接。

每个链接只能成功登记一个稳定身份。链接被使用、过期或正在另一场登录中时，服务端会明确拒绝。二维码登录失败时，短期预留会释放。

## 配置

```toml
[invites]
database_path = "/absolute/private/path/invites.sqlite3"
default_ttl_hours = 24
```

生产环境必须使用绝对路径。数据库只保存邀请令牌的 SHA-256 哈希、受邀用户稳定身份的规范化哈希、时间戳和状态；不会保存原始邀请令牌、账号密码、Cookie、token、姓名或课程数据。Unix 上新数据库会设置为 `0600`。

## 管理命令

发布包内包含 `canvas-video-invite-admin`。命令读取和服务端相同的私有配置：

```bash
/opt/canvas-video/current/bin/canvas-video-invite-admin \
  --config /etc/canvas-video/config.toml create

/opt/canvas-video/current/bin/canvas-video-invite-admin \
  --config /etc/canvas-video/config.toml create --ttl-hours 8

/opt/canvas-video/current/bin/canvas-video-invite-admin \
  --config /etc/canvas-video/config.toml list

/opt/canvas-video/current/bin/canvas-video-invite-admin \
  --config /etc/canvas-video/config.toml revoke INVITE_ID
```

`create` 是唯一会输出原始邀请链接的操作，应通过私聊发送，不要放入群聊、issue、日志或截图。`list` 只显示邀请 ID 和登记时间，不显示稳定身份哈希。`revoke` 会禁止该动态用户以后建立新 Session；已经存在的 Session 会持续到登出、过期或服务重启。

静态配置白名单仍然有效。若静态白名单用户使用邀请链接，该链接也会被消费，避免继续转发给第二人。

## 数据生命周期

- 邀请数据库是唯一新增的持久化认证数据，必须与生产配置同等保护和备份。
- 网站 Session、上游 Cookie、Canvas/LTI/video token、handles 和下载 ticket 仍只存在内存。
- 删除邀请数据库会删除动态白名单，但不会影响静态配置白名单。
- 应用重启后动态白名单仍有效；所有现有网站 Session 仍会失效。
