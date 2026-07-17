# 更新与回滚

当前推荐生产环境为 Ubuntu + systemd + Caddy。Mac mini 的历史脚本仍保留，
但不代表当前生产状态。

## 原则

- 构建精确 Git SHA，发布目录不可变。
- `/opt/canvas-video/current` 是原子 symlink，不原地覆盖运行目录。
- `/etc/canvas-video/config.toml` 永不进入 release。
- 切换后先做 loopback healthcheck，再做公网 HTTPS healthcheck。
- 任一健康检查失败即恢复旧 symlink、重启旧版本并再次检查。
- 当前版本永不因清理逻辑删除；成功后仅保留最近三个 release。

## 更新

在干净源码上生成 Linux release：

```bash
SOURCE_GIT_SHA=<exact-sha> CARGO_BUILD_JOBS=1 \
  ./scripts/build-release-linux.sh
```

核对 `VERSION`、`manifest.txt` 与二进制架构，再执行：

```bash
sudo ./scripts/update-ubuntu.sh /absolute/path/to/release-linux
```

脚本安装到 `/opt/canvas-video/releases/<sha>`，原子切换 `current`，重启
`canvas-video.service` 并验证本地和公网 health。失败不会报告成功。

## 手动回滚

先只读列出现有 release，并从 `VERSION` 确认目标 SHA。回滚必须显式给出完整
40 字符 SHA，不自动猜测版本：

```bash
sudo ./scripts/rollback-ubuntu.sh <40-character-sha>
```

目标 release 必须通过 manifest 与禁用文件检查。回滚自身若失败，会恢复之前
的 symlink 并给出明确错误。

## 配置回滚

配置不随 release 切换。修改前在 `/etc/canvas-video/` 内创建 root-only 备份，
应用后重启并执行健康检查。不得把真实配置复制到 release 或 Git。若配置导致
启动失败，恢复备份、校正 `root:canvas-video 0640` 后重启。

## Caddy changes

修改 `/etc/caddy/Caddyfile` 前建立带 UTC 时间戳的 root-only 备份。先执行
`caddy validate`，成功后只 reload Caddy。不要停止未知站点，也不要启用包含
下载 ticket 的 access log。

## Verification

```bash
./scripts/verify-production.sh
systemctl status canvas-video caddy --no-pager
ss -ltnp
```

重启服务会按设计销毁所有内存 Session、上游 Cookie、课程 token 与 ticket；
用户需要重新扫码。这不是数据丢失故障。
