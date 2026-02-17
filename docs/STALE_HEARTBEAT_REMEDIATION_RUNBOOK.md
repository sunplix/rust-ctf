# Stale Heartbeat Remediation Runbook

适用场景：管理员在运行告警中看到 `alert_type=instance_heartbeat_stale`（实例心跳超时）。

## 1. 目标

- 快速确认是否为真实故障
- 避免误回收仍在被队伍使用的实例
- 在必要时完成自动化或人工处置，并保留审计记录

## 2. 基线配置（默认）

- `INSTANCE_HEARTBEAT_STALE_SECONDS=300`
- `INSTANCE_STALE_REAPER_ENABLED=false`
- `INSTANCE_STALE_REAPER_BATCH_SIZE=20`

默认策略是“只告警，不自动销毁”。

## 3. 快速排查

1. 拉取 open 告警：

```bash
api=http://127.0.0.1:8080/api/v1
# token 需先通过 /auth/login 获取
curl -sS "$api/admin/runtime/alerts?status=open&alert_type=instance_heartbeat_stale&limit=200" \
  -H "Authorization: Bearer $token"
```

2. 查看实例当前状态（重点关注 `status`、`expires_at`、`last_heartbeat_at`）：

```bash
curl -sS "$api/admin/instances?status=running&limit=500" \
  -H "Authorization: Bearer $token"
```

3. 如需立即触发一次扫描收敛：

```bash
curl -sS -X POST "$api/admin/runtime/alerts/scan" \
  -H "Authorization: Bearer $token"
```

## 4. 处置决策

1. `instance` 已过期或比赛结束：
- 先 `ack` 告警并备注“过期待回收”。
- 等待后台 reaper 回收，或安排队伍自行销毁后重启实例。

2. `instance` 仍在 `running` 且业务确认仍在使用：
- 优先修复靶机内心跳上报器（`heartbeat_reporter.sh`）或容器内定时任务。
- 修复后观察下一个心跳周期，确认 `last_heartbeat_at` 恢复推进。

3. `instance` 异常且无法恢复：
- 通知队伍执行 `reset/destroy/start` 流程重建环境。
- 若需要平台侧强处置，建议短时开启 stale reaper（见第 6 节）。

## 5. 告警审计动作（ack / resolve）

1. 确认告警：

```bash
curl -sS -X POST "$api/admin/runtime/alerts/$alert_id/ack" \
  -H "Authorization: Bearer $token" \
  -H "Content-Type: application/json" \
  -d '{"note":"investigating stale heartbeat"}'
```

2. 问题恢复后关闭告警：

```bash
curl -sS -X POST "$api/admin/runtime/alerts/$alert_id/resolve" \
  -H "Authorization: Bearer $token" \
  -H "Content-Type: application/json" \
  -d '{"note":"heartbeat resumed"}'
```

> 建议每次 ack/resolve 都填写 `note`，便于后续审计复盘。

## 6. 临时启用自动处置（谨慎）

仅在连续大量 stale 告警且人工处理不可持续时使用。

1. 在部署配置中启用：

```env
INSTANCE_STALE_REAPER_ENABLED=true
INSTANCE_HEARTBEAT_STALE_SECONDS=300
INSTANCE_STALE_REAPER_BATCH_SIZE=20
```

2. 重启后端，使配置生效。

3. 观察后端日志关键字：
- `stale instance reaper tick completed`
- `stale instance reaper tick failed`

4. 压力解除后，可回退为 `INSTANCE_STALE_REAPER_ENABLED=false`。

## 7. 预防建议

- 所有动态/内网题模板统一接入 `{{HEARTBEAT_REPORT_TOKEN}}` + `heartbeat_reporter.sh`。
- 将上报周期设置为 `30s`，并保持 `STALE_SECONDS >= 5 * 上报周期`。
- 比赛前做一次“实例启动 10 分钟+持续上报”巡检，避免赛中集中告警。
