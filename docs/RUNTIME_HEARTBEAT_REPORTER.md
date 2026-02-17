# Runtime Heartbeat Reporter

本文件说明如何在动态题目容器中接入“实例心跳上报器”。

## 1. 后端接口

- 内部上报接口：`POST /api/v1/instances/heartbeat/report`
- 请求体：

```json
{
  "token": "instance-heartbeat-token"
}
```

说明：

- 该接口用于靶机内部上报，不依赖选手登录会话。
- `token` 由后端在 `compose_template` 渲染阶段生成并注入。

## 2. 可用占位符

在题目 `compose_template` 中可使用：

- `{{HEARTBEAT_REPORT_URL}}`
- `{{HEARTBEAT_REPORT_TOKEN}}`
- `{{HEARTBEAT_INTERVAL_SECONDS}}`

## 3. 推荐脚本

可复用脚本：`backend/scripts/runtime/heartbeat_reporter.sh`

脚本依赖环境变量：

- `HEARTBEAT_REPORT_URL`
- `HEARTBEAT_REPORT_TOKEN`
- `HEARTBEAT_INTERVAL_SECONDS`（可选，默认 30）

## 4. compose 示例

```yaml
services:
  target:
    image: alpine:3.20
    command: >
      sh -c "apk add --no-cache curl >/dev/null 2>&1;
             /opt/ctf/heartbeat_reporter.sh &
             sleep 3600"
    environment:
      HEARTBEAT_REPORT_URL: "{{HEARTBEAT_REPORT_URL}}"
      HEARTBEAT_REPORT_TOKEN: "{{HEARTBEAT_REPORT_TOKEN}}"
      HEARTBEAT_INTERVAL_SECONDS: "{{HEARTBEAT_INTERVAL_SECONDS}}"
```

## 5. 运行参数建议

- `INSTANCE_HEARTBEAT_STALE_SECONDS=300`
- `INSTANCE_STALE_REAPER_ENABLED=false`（初期建议关闭自动处置）
- 先观察告警与误报，再按需启用 `INSTANCE_STALE_REAPER_ENABLED=true`
