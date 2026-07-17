# 更新与回滚

## 原则

- 发布包不包含 `config/local.toml`、`.local/`、Tunnel credential、浏览器 trace 或视频；
- 私有配置固定在 `~/Services/sjtu-canvas-video/config/`，不随 release 替换；
- `current` symlink 是唯一激活指针；
- 新 release 完整复制后才切换；
- 切换后先重启再检查本地 `/api/health`；
- 健康检查失败时立即恢复旧 symlink 并再次重启；
- 成功后只保留最近三个 release。

## 更新

在 Mac mini 的干净 checkout 构建：

```bash
git fetch --tags
git checkout <reviewed-commit>
./scripts/build-release.sh
./release/scripts/update-macos.sh /absolute/path/to/repository/release
```

脚本验证 `VERSION` 的 40 位 Git SHA、服务二进制、`frontend/dist/index.html` 和 `.local` 不存在。它不会覆盖私有配置，也不会在复制失败时中断当前版本。

更新会使所有内存 Session、上游 Cookie 和 ticket 失效，这是安全模型的一部分；用户需重新扫码。长下载在重启时中断，发布前应告知受邀用户。

## 自动失败回滚

`update-macos.sh` 保存旧 symlink，切换新 release，执行：

```text
launchctl kickstart -k gui/$UID/<label>
→ http://127.0.0.1:3100/api/health
```

任一步失败都会恢复旧 link、重启旧进程并再次 healthcheck，然后以失败状态退出。脚本不会用健康 API 成功代替真实扫码或下载验收。

## 手动回滚

回滚到最近一个非当前 release：

```bash
~/Services/sjtu-canvas-video/current/scripts/rollback-macos.sh
```

回滚到指定目录名：

```bash
~/Services/sjtu-canvas-video/current/scripts/rollback-macos.sh <timestamp>-<git-sha>
```

目标仍须通过 healthcheck；失败则恢复回滚前版本。回滚后旧 Session 同样失效。

## 配置变更回滚

脚本故意不版本化私有配置。修改前手工创建仅限本机的权限 600 备份，文件名和内容不得进入 Git。若新版本需要配置字段，先根据 `config/production.example.toml` 做最小差异更新；不要直接覆盖。

## 验收

更新或回滚完成后至少验证：

1. loopback health；
2. 进程只监听 loopback；
3. 公网 HTTPS 与静态资源；
4. 重新扫码、课程和一条最小 Range；
5. `CF-Cache-Status` 非 `HIT`；
6. 登出和旧 ticket 失效；
7. 日志和磁盘无视频或 secret。
