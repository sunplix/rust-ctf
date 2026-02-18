# M5 压测与验收指南

本文档定义 M5 阶段的可执行验收流程，目标覆盖：

- 并发压测（提交吞吐与延迟）
- 安全基线（鉴权、鉴权边界、比赛窗口约束、运行时约束）
- 比赛链路演练（运行时回归 + 实时榜单推送）

## 1. 前置条件

- Docker Daemon 已启动
- 本地开发栈可用（`postgres/redis/backend/frontend` 正常）
- `curl`、`jq`、`node` 可用

健康检查：

```bash
curl -sS http://127.0.0.1:8080/api/v1/health | jq .
```

## 2. 脚本清单

- 运行时回归：`backend/scripts/runtime/runtime_full_regression.sh`
- 并发压测：`backend/scripts/m5/load_benchmark.sh`
- 安全回归：`backend/scripts/m5/security_smoke.sh`
- 一键验收：`backend/scripts/m5/full_acceptance.sh`

## 3. 推荐执行顺序

1. 先跑运行时回归，确认 M3/M4 基线稳定。
2. 跑安全回归，确认关键权限与约束未回退。
3. 跑并发压测，输出吞吐/延迟指标。
4. 用一键验收脚本生成统一报告归档。

## 4. 压测脚本说明

### 4.1 命令

```bash
API_BASE=http://127.0.0.1:8080/api/v1 \
TEAM_COUNT=20 \
REQUESTS_TOTAL=400 \
CONCURRENCY=20 \
VALID_FLAG_PERCENT=35 \
WARMUP_ROUNDS=1 \
backend/scripts/m5/load_benchmark.sh
```

### 4.2 输出

- 控制台打印 `summary.json`
- 工件目录默认：`/tmp/rust-ctf-m5-load-<timestamp>/`
- 关键文件：
  - `player_tokens.txt`
  - `submission_results.tsv`
  - `summary.json`

### 4.3 关键指标

- `requests_per_second`
- `response_time_seconds.p95`
- `error_total`
- `rate_limited_total`（用于观察限频压力下行为）

## 5. 安全脚本说明

### 5.1 命令

```bash
API_BASE=http://127.0.0.1:8080/api/v1 \
backend/scripts/m5/security_smoke.sh
```

### 5.2 检查项

- 未登录访问受保护接口返回 `401`
- 普通选手访问管理接口返回 `403`
- 无队伍用户提交返回 `403 permission denied`
- 草稿赛提交返回 `400 contest is not running`
- 对静态题启动实例返回 `400 challenge type does not require runtime instance`
- 榜单接口要求鉴权（未登录返回 `401`）

## 6. 一键验收说明

```bash
API_BASE=http://127.0.0.1:8080/api/v1 \
LOAD_TEAM_COUNT=20 \
LOAD_REQUESTS_TOTAL=400 \
LOAD_CONCURRENCY=20 \
backend/scripts/m5/full_acceptance.sh
```

输出目录：`/tmp/rust-ctf-m5-acceptance-<timestamp>/`

- `M5_ACCEPTANCE_REPORT.md`
- `summary.json`
- `runtime_full_regression.log`
- `security_smoke.log`
- `load_benchmark.log`

## 7. 建议阈值（开发环境基线）

- `runtime_full_regression`: `PASSED`
- `security_smoke`: 全部用例通过
- `load_benchmark.error_total = 0`
- `load_benchmark.response_time_seconds.p95 <= 1.0`（本地开发环境参考阈值）

说明：不同硬件与 Docker 负载会影响绝对数值，建议按同机型长期趋势评估。

## 8. CI 集成

仓库内已提供：

- `.github/workflows/ci.yml`
  - `push/pull_request` 自动执行后端编译检查与前端构建。
- `.github/workflows/m5-acceptance.yml`
  - `workflow_dispatch` 手动触发 M5 验收。
  - 支持输入压测参数（队伍数、请求量、并发等）。
  - 默认 `enable_wireguard_smoke=false`，用于提升 hosted runner 稳定性。

若需在具备 WireGuard 能力的 self-hosted runner 上做全量回归，可将该输入切换为 `true`。
