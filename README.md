# Rust-CTF

一个面向教学与竞赛场景的高性能 CTF 平台，采用 **Rust（Axum + Tokio + SQLx）后端** 与 **Vue3 + TypeScript 前端**，支持传统单题容器与按队伍动态生成的多容器内网靶场。

## 1. 项目目标

当前许多 CTF 平台在高级攻防训练场景上存在明显短板：仅支持单容器、难以模拟真实内网拓扑、实例隔离与资源回收能力不足。本项目聚焦这些痛点，建设一个可扩展、可审计、可自动化运维的现代 CTF 平台。平台不仅服务于常规 Web/Crypto/Pwn 题型，也重点支持“服务发现、横向移动、内网渗透、多节点联动”等实战化训练需求。

## 2. 总体架构

- 后端：Rust + Axum + Tokio + SQLx + Redis
- 前端：Vue3 + TypeScript + Pinia + Vue Router
- 数据层：PostgreSQL（核心业务）+ Redis（缓存、排行榜、会话辅助）
- 容器编排：Docker Compose（按队伍生成实例）
- 实时通信：WebSocket（积分榜、环境状态、系统通知）
- 观测与审计：结构化日志、操作审计、容器运行指标采集

## 3. 功能范围（核心需求与模块说明）

本平台在初始化阶段即以完整竞赛闭环为目标设计，功能分为四大域：平台核心业务模块、多容器编排模块、前端交互模块、系统管理模块。以下为要完成的主要功能与说明。

### 3.1 平台核心业务模块

1. 用户管理  
用户管理核心能力已完成首版闭环，覆盖“账号创建、登录鉴权、资料维护、密码变更、登录历史查询”五类主流程，当前实现如下：

- 认证与会话：`/api/v1/auth/register`、`/api/v1/auth/login`、`/api/v1/auth/refresh`、`/api/v1/auth/me`
- 资料维护：`PATCH /api/v1/auth/profile`（用户名/邮箱更新）
- 密码管理：`POST /api/v1/auth/change-password`（校验旧密码、更新后自动轮换会话）
- 登录历史：`GET /api/v1/auth/login-history`（按时间倒序返回注册/登录/刷新/改密认证事件）
- 管理端用户运维：`GET /api/v1/admin/users`、`PATCH /api/v1/admin/users/{user_id}/status`、`PATCH /api/v1/admin/users/{user_id}/role`、`POST /api/v1/admin/users/{user_id}/reset-password`

安全策略方面，后端采用 Argon2 存储密码哈希，使用 JWT（Access + Refresh）与 Redis 会话键进行刷新令牌轮换；鉴权阶段会校验用户 `active` 状态与会话键有效性，因此禁用用户或管理员重置密码后，历史会话可被立即失效。认证事件会写入 `audit_logs`，记录操作与请求来源信息（如 `user-agent`、`x-forwarded-for`）以支持审计追溯。后续将继续扩展验证码、邮箱激活、双因素认证（2FA）等增强能力。

2. 队伍管理  
队伍管理已从“基础建队/加队”扩展到“完整生命周期管理”，当前实现如下：

- 队伍基础：`GET /api/v1/teams`、`GET /api/v1/teams/me`、`GET /api/v1/teams/{team_id}`、`POST /api/v1/teams`、`POST /api/v1/teams/join`
- 队伍维护：`PATCH /api/v1/teams/{team_id}`、`POST /api/v1/teams/leave`、`DELETE /api/v1/teams/{team_id}`
- 队长能力：`POST /api/v1/teams/{team_id}/transfer-captain`、`DELETE /api/v1/teams/{team_id}/members/{member_user_id}`
- 邀请机制：`POST /api/v1/teams/invitations`、`GET /api/v1/teams/invitations/sent`、`GET /api/v1/teams/invitations/received`、`POST /api/v1/teams/invitations/{invitation_id}/respond`、`POST /api/v1/teams/invitations/{invitation_id}/cancel`

权限与一致性方面，后端会校验“当前是否队长”“目标成员是否属于本队”“被邀请用户是否已在其他队伍”等关键约束，并通过数据库唯一约束保证一个用户同一时刻仅属于一支队伍。该模块与判题、实例分配直接关联，是多容器隔离与竞赛身份边界的基础。

3. 题目管理  
题目管理已完成“配置录入 + 生命周期 + 版本追踪 + 附件管理”的完整闭环，当前实现如下：

- 基础信息与判题配置：`POST /api/v1/admin/challenges`、`PATCH /api/v1/admin/challenges/{challenge_id}` 支持标题、分类、分值、难度、标签、`challenge_type`、`flag_mode`、`flag_hash`、`metadata`、`compose_template`、题解策略与题解内容配置
- 生命周期管理：题目支持 `draft`、`published`、`offline` 三种状态；状态与可见性同步约束，避免“状态与对外展示冲突”
- 版本更新与回滚：每次创建/更新都会写入 `challenge_versions` 快照；支持 `GET /api/v1/admin/challenges/{challenge_id}/versions` 查询历史版本，`POST /api/v1/admin/challenges/{challenge_id}/rollback` 一键回滚
- 附件管理：支持 `POST /api/v1/admin/challenges/{challenge_id}/attachments` 上传、`GET /api/v1/admin/challenges/{challenge_id}/attachments` 查询、`DELETE /api/v1/admin/challenges/{challenge_id}/attachments/{attachment_id}` 删除
- 前端管理入口：管理台已支持标签与题解策略录入、版本列表/回滚、附件上传与删除，满足赛题迭代与教学维护流程

4. 比赛管理  
比赛管理已具备“赛事创建 + 时间窗口 + 状态干预 + 题目编排”主流程能力，当前实现如下：

- 赛事配置：`GET /api/v1/admin/contests`、`POST /api/v1/admin/contests`、`PATCH /api/v1/admin/contests/{contest_id}` 支持标题、slug、描述、可见性、起止时间、封榜时间配置
- 积分规则：比赛级支持 `scoring_mode=static|dynamic` 与 `dynamic_decay` 参数，动态模式下按已解队伍数递减分值
- 状态控制：`PATCH /api/v1/admin/contests/{contest_id}/status` 支持 `draft/scheduled/running/ended/archived` 手动切换，覆盖赛前准备、赛中运行与赛后归档流程
- 题目集合编排：`GET|POST /api/v1/admin/contests/{contest_id}/challenges`、`PATCH|DELETE /api/v1/admin/contests/{contest_id}/challenges/{challenge_id}` 支持挂载题目、排序、按题发布时间控制
- 公告系统：`GET|POST /api/v1/admin/contests/{contest_id}/announcements`、`PATCH|DELETE /api/v1/admin/contests/{contest_id}/announcements/{announcement_id}` 支持公告创建、发布/撤回、置顶、修改、删除；选手侧通过 `GET /api/v1/contests/{contest_id}/announcements` 获取已发布公告
- 选手侧赛事视图：`GET /api/v1/contests`、`GET /api/v1/contests/{contest_id}/challenges`、`GET /api/v1/contests/{contest_id}/scoreboard` 已形成参赛主链路，并与判题模块联动

5. Flag 判题与计分  
提供统一判题接口，兼容静态 flag、动态 flag、脚本校验、内网场景校验。系统对提交频率进行限流，对错误提交做冷却与审计，避免爆破式噪声。判题成功后触发积分更新、解题记录写入与排行榜事件广播，确保竞赛反馈实时且一致。

### 3.2 多容器编排模块（核心创新）

1. Compose 模板渲染引擎  
题目作者提交模板后，系统将按队伍上下文渲染变量（网络名、容器名、flag、端口映射、环境变量等），生成可执行 compose 文件，避免人工改配导致的环境漂移。

2. 子网自动分配器  
平台维护可分配地址池（如 10.x.x.0/24），基于队伍 ID 与实例索引分配唯一子网并记录租约，确保不同队伍内网互不冲突。支持回收后复用、冲突检测、异常恢复，保障大规模并发开题时的稳定性。

3. 实例生命周期管理  
提供实例创建、启动、停止、重启、销毁与状态查询；对每个实例记录所属比赛、队伍、题目与资源配额。实例管理器与 Docker API/Compose 命令联动，支持失败重试、超时控制和异步任务编排。

4. 资源限制与自动回收  
为容器配置 CPU/内存/进程数等约束，防止单队伍占满宿主机。系统按 TTL 与比赛状态自动回收过期实例，对僵尸容器和孤儿网络进行清理，维持平台长期运行稳定。

5. 运行监控与可视化状态  
采集容器健康状态、重启次数、资源使用趋势并反馈到前端，帮助管理员快速定位题目环境故障，也为后续容量规划与故障预警提供数据基础。

### 3.3 前端交互模块（Vue3）

1. 选手端  
提供比赛大厅、题目列表、题目详情、附件下载、环境一键启动/重置、flag 提交、提交结果反馈、实时排行榜、通知消息中心。强调“低学习成本”和“高反馈速度”，减少竞赛过程中的操作阻塞。

2. 管理端  
提供用户与队伍管理、题库维护、比赛编排、实例监控、日志检索、公告发布、统计视图等功能。管理员可在一个控制面板内完成赛前配置、赛中运维与赛后复盘。

3. 实时交互  
通过 WebSocket 推送排行榜变化、实例状态变更、系统公告与错误提示，保证选手和管理员在高并发场景下仍能获得低延迟的信息同步体验。

### 3.4 系统管理与安全模块

1. 审计日志  
对关键操作（登录、权限变更、题目发布、实例创建销毁、管理端操作）全量记录，支持按用户、时间、比赛、对象类型检索，满足教学管理与赛后追溯需求。

2. 运行监控  
对 API 延迟、判题吞吐、队列积压、容器资源、数据库连接池等指标进行采集，配合报警策略实现故障早发现、早定位。

3. 安全策略  
实现鉴权中间件、接口限流、输入校验、敏感信息脱敏、最小权限访问控制；在实例侧强化网络隔离，降低跨队伍访问与环境逃逸风险。

4. 错误追踪与恢复  
统一错误码体系和异常处理链路，关键任务支持幂等重试与补偿逻辑，减少比赛中的“不可恢复失败”。

## 4. 初始化阶段里程碑（建议）

1. M1：基础骨架  
完成 Rust 后端工程、Vue3 前端工程、数据库迁移框架、统一配置管理、基础 CI。

2. M2：核心业务闭环  
实现用户/队伍/比赛/题目/判题最小可用链路，完成基础排行榜。

3. M3：多容器内网能力  
实现模板渲染、子网分配、实例生命周期管理与自动回收。

4. M4：实时化与可观测性  
完成 WebSocket 推送、审计日志、容器监控、告警接入。

5. M5：压测与验收  
开展并发压测、安全测试、比赛流程演练与文档完善，形成可部署版本。

## 5. 目录规划（初始化目标）

```text
rust-ctf/
  backend/                # Rust Axum 服务
  frontend/               # Vue3 + TS 前端
  deploy/                 # Docker/Compose/环境配置
  docs/                   # 架构设计与接口文档
  scripts/                # 初始化、迁移、运维脚本
  README.md
```

## 6. 非功能性目标

- 高并发：异步架构、无阻塞 I/O、关键路径缓存化
- 高可用：实例失败重试、任务幂等、健康检查
- 可扩展：模块化领域设计，支持新增题型与新判题器
- 可维护：统一日志、错误码、配置中心与清晰边界
- 可运营：完整审计、指标看板、赛后复盘数据留存

## 7. 当前状态

当前仓库已完成第一轮工程初始化，已包含：

- `backend/`：Rust + Axum 基础服务骨架（含配置加载、健康检查路由、Dockerfile）
- `backend/`：已完成核心数据库迁移（users/teams/contests/challenges/submissions/instances）
- `backend/`：已提供认证与用户管理接口（`/api/v1/auth/register`、`/api/v1/auth/login`、`/api/v1/auth/refresh`、`/api/v1/auth/me`、`/api/v1/auth/profile`、`/api/v1/auth/change-password`、`/api/v1/auth/login-history`）
- `backend/`：已提供比赛基础接口（`/api/v1/contests`、`/api/v1/contests/{contest_id}/challenges`、`/api/v1/submissions`）
- `backend/`：已提供完整队伍管理接口（建队/加队、邀请与响应、队伍编辑、离队、队长转让、移除成员、解散）
- `backend/`：已提供管理员接口（用户管理、题目管理、比赛创建/编辑/状态控制、比赛题目挂载管理、实例监控）
- `backend/`：已支持默认管理员账号自动初始化（可通过 `DEFAULT_ADMIN_*` 配置）
- `backend/`：提交接口已接入 Redis 限频（30 秒窗口内最多 10 次）
- `backend/`：已提供排行榜接口（`/api/v1/contests/{contest_id}/scoreboard`）
- `backend/`：已提供排行榜 WebSocket 推送接口（`/api/v1/contests/{contest_id}/scoreboard/ws`）
- `backend/`：排行榜 WebSocket 支持浏览器 Token Query 鉴权（`?access_token=...`，同时兼容 `Authorization: Bearer ...`）
- `backend/`：已提供实例生命周期接口（`/api/v1/instances/start|stop|reset|destroy|heartbeat|{contest_id}/{challenge_id}`）
- `backend/`：实例生命周期已接入真实 `docker compose` 编排（模板渲染、compose 文件落盘、启动/停止/重置/销毁）
- `backend/`：实例启动/重置已支持一次自动自愈重试（失败后自动执行 `down` + `up --force-recreate`）
- `backend/`：实例已支持默认资源配额注入（CPU/内存）与心跳刷新（`last_heartbeat_at`）
- `backend/`：已支持实例过期自动回收（后台定时扫描 `expires_at`，批量执行销毁与运行目录清理）
- `backend/`：已支持心跳超时阈值配置与可选自动回收策略（默认关闭，避免误回收未接入心跳上报的靶机）
- `backend/`：已支持靶机内部心跳上报接口（`POST /api/v1/instances/heartbeat/report`）与 compose 模板令牌注入占位符
- `backend/`：已提供管理员审计日志与运行概览接口（`/api/v1/admin/audit-logs`、`/api/v1/admin/runtime/overview`）
- `backend/`：已提供运行告警通知接口（`/api/v1/admin/runtime/alerts`、`/api/v1/admin/runtime/alerts/scan`、`/api/v1/admin/runtime/alerts/{alert_id}/ack|resolve`），并支持后台定时扫描与自动收敛
- `backend/`：已提供实例运行指标采集接口（`GET /api/v1/admin/instances/{instance_id}/runtime-metrics`），可返回容器 CPU/内存/网络/健康状态
- `backend/`：已提供手动回收触发接口（`POST /api/v1/admin/runtime/reaper/expired`、`POST /api/v1/admin/runtime/reaper/stale`），支持赛中即时处置
- `backend/`：已支持 `compose_template` 变量 schema 校验（保留占位符 + `{{VAR:NAME}}` + `metadata.compose_variables` 定义校验）
- `backend/`：已提供比赛公告管理与选手公告读取接口（管理员 CRUD + 选手只读已发布）
- `backend/`：判题已支持静态 flag（明文或 Argon2 哈希）与动态 flag（Redis 键 `flag:dynamic:{contest_id}:{challenge_id}:{team_id}`）
- `backend/`：判题已支持比赛级动态积分（根据已解队伍数按衰减公式计算实际得分）
- `backend/`：`script` 判题已支持按题目 metadata 执行外部校验脚本（返回码 0=正确，1=错误，其他=判题异常）
- `frontend/`：已完成选手最小闭环页面（登录/注册、比赛列表、题目列表、Flag 提交、实例控制、实时榜单显示）
- `frontend/`：已完成 API 客户端与本地登录态持久化（Pinia + localStorage）
- `frontend/`：已新增账户中心页面（个人资料维护、密码修改、登录历史查看）
- `frontend/`：已完成队伍中心增强版（邀请处理、队伍编辑、成员管理、队长转让、离队/解散）
- `frontend/`：已完成管理员 v2 页面（模块/子导航拆分、题目创建/可见性切换、比赛创建与状态切换、题目挂载与排序、公告管理、实例列表监控）
- `frontend/`：管理员页面已新增审计日志、运行概览、运行告警与模板校验面板（支持筛选、触发扫描、ack/resolve）
- `frontend/`：实例监控页已支持单实例“运行指标”详情（容器级资源与健康状态）与手动回收操作入口
- `frontend/`：敏感/细节字段（如判题哈希与 compose 模板）已下沉到二次展开面板，降低主界面拥挤度
- `deploy/`：本地开发用 `docker-compose.dev.yml`（PostgreSQL / Redis / Backend / Frontend）
- `docs/`：初始化后续开发任务说明
- `docs/`：全量后端接口文档 `docs/API_REFERENCE.md`（后续开发持续维护）
- `docs/`：靶机心跳上报接入指南 `docs/RUNTIME_HEARTBEAT_REPORTER.md`
- `docs/`：心跳超时处置手册 `docs/STALE_HEARTBEAT_REMEDIATION_RUNBOOK.md`
- `backend/scripts/m5/`：M5 压测与验收脚本（并发压测、安全回归、一键验收报告）
- `docs/`：M5 验收指南 `docs/M5_ACCEPTANCE.md`
- `docs/`：部署指南（单机 Docker 生产基线）`docs/DEPLOYMENT_GUIDE.md`

当前 M3（多容器编排）与 M4（实时化与可观测性）已完成，M5（压测与验收）已启动并具备可执行脚本基线。

## 8. 本地启动（可用版）

### 8.1 前置要求

- Docker Engine（Daemon 已启动）
- Docker Compose（本仓库默认使用 `docker compose` 命令）
- Rust（建议 stable）
- Node.js 20+

### 8.2 方式 A：容器一键启动（推荐）

```bash
docker compose -f deploy/docker-compose.dev.yml up --build
```

说明：开发编排已包含 `backend` 容器对 Docker Socket 的挂载（`/var/run/docker.sock`），实例生命周期接口可直接调用宿主机 Docker 创建/销毁题目环境。

启动后访问：

- 前端：`http://localhost:5173`
- 后端健康检查：`http://localhost:8080/api/v1/health`

停止服务：

```bash
docker compose -f deploy/docker-compose.dev.yml down
```

### 8.3 方式 B：本地开发模式（后端/前端分开跑）

1. 启动基础依赖（PostgreSQL + Redis）：

```bash
docker compose -f deploy/docker-compose.dev.yml up -d postgres redis
```

2. 启动后端（会自动执行 `backend/migrations/` 下迁移）：

```bash
cd backend
cargo run
```

3. 新开终端启动前端：

```bash
cd frontend
npm install
npm run dev
```

### 8.4 快速自检

```bash
curl http://localhost:8080/api/v1/health
```

返回 `status=ok` 且 `dependencies.database=true`、`dependencies.redis=true` 即表示后端基础可用。

如需验证实例编排链路可用，可再执行：

```bash
docker info >/dev/null && docker compose version
```

### 8.5 已验证的接口链路（2026-02-15）

- `GET /api/v1/health`：健康检查通过，数据库和 Redis 状态正常
- `POST /api/v1/auth/register`、`GET /api/v1/auth/me`、`POST /api/v1/auth/refresh`：认证链路通过
- `PATCH /api/v1/auth/profile`、`POST /api/v1/auth/change-password`、`GET /api/v1/auth/login-history`：用户管理链路通过
- `GET /api/v1/admin/users`、`PATCH /api/v1/admin/users/{user_id}/status`、`PATCH /api/v1/admin/users/{user_id}/role`、`POST /api/v1/admin/users/{user_id}/reset-password`：管理员用户运维链路通过
- `POST /api/v1/teams/invitations`、`POST /api/v1/teams/invitations/{invitation_id}/respond`、`PATCH /api/v1/teams/{team_id}`、`POST /api/v1/teams/{team_id}/transfer-captain`、`DELETE /api/v1/teams/{team_id}/members/{member_user_id}`、`POST /api/v1/teams/leave`、`DELETE /api/v1/teams/{team_id}`：队伍生命周期链路通过
- `POST /api/v1/submissions`：静态哈希 flag 判题通过
- `POST /api/v1/submissions`：动态积分模式验证通过（同题不同队伍得分递减）
- `GET /api/v1/contests/{contest_id}/scoreboard`：排行榜查询通过
- `GET /api/v1/contests/{contest_id}/scoreboard`：未鉴权访问返回 `401`（权限控制生效）
- 提交限频：高频提交后返回 `verdict=rate_limited`
- `GET|POST|PATCH|DELETE /api/v1/admin/contests/{contest_id}/announcements...`：公告管理链路通过
- `GET /api/v1/contests/{contest_id}/announcements`：选手侧仅可见已发布公告，发布后可即时读取

### 8.6 默认管理员账号（可配置）

后端在启动时会自动执行数据库迁移，并根据配置确保存在一个管理员账号（如不存在则创建，存在则提升为 `admin` 并设为 `active`）。

默认配置如下（见 `backend/.env.example` 与 `deploy/docker-compose.dev.yml`）：

- `DEFAULT_ADMIN_ENABLED=true`
- `DEFAULT_ADMIN_USERNAME=admin`
- `DEFAULT_ADMIN_EMAIL=admin@rust-ctf.local`
- `DEFAULT_ADMIN_PASSWORD=admin123456`
- `DEFAULT_ADMIN_FORCE_PASSWORD_RESET=false`
- `INSTANCE_DEFAULT_CPU_LIMIT=1.0`
- `INSTANCE_DEFAULT_MEMORY_LIMIT_MB=512`
- `INSTANCE_PUBLIC_HOST=127.0.0.1`
- `INSTANCE_HOST_PORT_MIN=32768`
- `INSTANCE_HOST_PORT_MAX=60999`
- `RUNTIME_ALERT_SCAN_ENABLED=true`
- `RUNTIME_ALERT_SCAN_INTERVAL_SECONDS=60`
- `RUNTIME_ALERT_SCAN_INITIAL_DELAY_SECONDS=10`
- `INSTANCE_REAPER_ENABLED=true`
- `INSTANCE_REAPER_INTERVAL_SECONDS=60`
- `INSTANCE_REAPER_INITIAL_DELAY_SECONDS=20`
- `INSTANCE_REAPER_BATCH_SIZE=30`
- `INSTANCE_HEARTBEAT_STALE_SECONDS=300`
- `INSTANCE_HEARTBEAT_REPORT_URL=http://host.docker.internal:8080/api/v1/instances/heartbeat/report`
- `INSTANCE_HEARTBEAT_REPORT_INTERVAL_SECONDS=30`
- `INSTANCE_STALE_REAPER_ENABLED=false`
- `INSTANCE_STALE_REAPER_BATCH_SIZE=20`

说明：

- 仅用于初始化与本地检查，生产环境请务必修改默认密码。
- 当 `DEFAULT_ADMIN_FORCE_PASSWORD_RESET=true` 时，每次启动会强制把该账号密码重置为 `DEFAULT_ADMIN_PASSWORD`。

### 8.7 Script 判题 metadata 示例

`flag_mode=script` 的题目可在 `metadata` 中配置：

```json
{
  "script_verifier": {
    "program": "./scripts/verifiers/simple_compare.sh",
    "args": ["ctf{demo_flag}"],
    "timeout_seconds": 5
  }
}
```

后端会向脚本注入环境变量：`SUBMITTED_FLAG`、`CONTEST_ID`、`CHALLENGE_ID`、`TEAM_ID`。

### 8.8 靶机心跳上报接入示例

可在题目 `compose_template` 中使用以下占位符：

- `{{HEARTBEAT_REPORT_URL}}`
- `{{HEARTBEAT_REPORT_TOKEN}}`
- `{{HEARTBEAT_INTERVAL_SECONDS}}`

示例（选手容器内后台上报心跳）：

```yaml
services:
  box:
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

可复用脚本参考：`backend/scripts/runtime/heartbeat_reporter.sh`。

### 8.9 Compose 变量 Schema（M3）

`compose_template` 现支持自定义变量占位符 `{{VAR:NAME}}`，并要求在题目 `metadata.compose_variables` 中定义。

示例：

```yaml
services:
  web:
    image: nginx:alpine
    ports:
      - "{{VAR:WEB_PORT}}:80"
    environment:
      APP_ENV: "{{VAR:APP_ENV}}"
      HEARTBEAT_REPORT_URL: "{{HEARTBEAT_REPORT_URL}}"
```

```json
{
  "compose_variables": [
    { "name": "WEB_PORT", "value": "18080", "required": true },
    { "name": "APP_ENV", "default": "prod" }
  ]
}
```

校验规则：

- 模板仅允许保留占位符与 `{{VAR:NAME}}`
- `NAME` 仅允许 `A-Z0-9_`，最长 64
- 模板引用到的每个变量都必须在 `metadata.compose_variables` 中定义

### 8.10 运行模式：`compose` 与 `single_image`

题目运行时支持两种模式（通过 `metadata.runtime` 控制）：

1. `compose`（默认）
- 使用题目提供的 `compose_template`
- 默认自动注入队伍隔离 SSH 跳板（`access_mode=ssh_bastion`），选手进入跳板后可对 10.x.x.0/24 子网做端口扫描与横向渗透
- 跳板账号默认支持 `sudo`，可按需安装额外扫描工具（如 `sudo apk add --no-cache nmap`）
- 也可切换为 WireGuard VPN（`access_mode=wireguard`），下载专属 `.conf` 后从本地终端直接访问队伍子网
- 若要关闭跳板，可显式设置 `metadata.runtime.access_mode=direct`

2. `single_image`
- 适合 Web/Pwn 等“单镜像单端口”题
- 无需 `compose_template`，由平台根据镜像仓库地址自动生成运行模板
- 启动时自动选择随机高位端口并映射到指定内部端口

示例：

```json
{
  "runtime": {
    "mode": "single_image",
    "image": "nginx:alpine",
    "internal_port": 80,
    "protocol": "http"
  }
}
```

对应实例返回的 `entrypoint_url` 将类似：`http://127.0.0.1:32768`（端口随机）。

### 8.11 WireGuard 接入步骤（选手端）

1. 启动实例：`POST /api/v1/instances/start`
2. 获取配置：`GET /api/v1/instances/{contest_id}/{challenge_id}/wireguard-config`
3. 导入返回的 `content` 到本地 WireGuard 客户端（或保存为返回的 `filename`）
4. 连接后对队伍子网执行扫描与渗透测试（例如 `nmap 10.x.x.0/24`）

常见问题：

- `instance access mode is not wireguard`：题目未设置 `metadata.runtime.access_mode=wireguard`
- `wireguard config is not ready`：实例刚启动，稍后重试下载

### 8.12 WireGuard 一键冒烟测试脚本

仓库内提供脚本：`backend/scripts/runtime/wireguard_smoke.sh`

```bash
backend/scripts/runtime/wireguard_smoke.sh
```

脚本会自动完成：

- 管理员创建运行中赛事
- 创建 `access_mode=wireguard` 的 compose 题目并挂载
- 注册选手、创建队伍、启动实例
- 轮询 `wireguard-config` 直到拿到有效配置

### 8.13 题库模板批量 Lint

可使用管理端 API：

- `GET /api/v1/admin/challenges/runtime-template/lint`

也可直接运行脚本（默认使用 `admin/admin123456`）：

```bash
backend/scripts/runtime/challenge_template_lint.sh
```

仅看错误项：

```bash
ONLY_ERRORS=true backend/scripts/runtime/challenge_template_lint.sh
```

### 8.14 运行时全量回归脚本

可使用统一脚本串行执行：

- 健康检查（`/api/v1/health`）
- 运行模板 Lint 汇总
- WireGuard 动态实例冒烟测试
- Single Image 动态实例冒烟测试（随机端口映射）
- 运行指标与手动回收接口健全性检查（`runtime-metrics` / `reaper`）
- Scoreboard WebSocket 推送冒烟测试（快照 + 提交后增量推送）

```bash
backend/scripts/runtime/runtime_full_regression.sh
```

可选参数：

- `FAIL_ON_LINT_ERRORS=true`：当模板 Lint 存在错误时直接失败
- `LINT_LIMIT=500`：调整 Lint 扫描题目上限
- `ADMIN_USER` / `ADMIN_PASSWORD`：覆盖管理员登录账号
- `USER_PASSWORD`：覆盖冒烟选手密码
- `ENABLE_WIREGUARD_SMOKE=false`：跳过 WireGuard 冒烟（适用于不支持 WireGuard 的 CI 环境）
- `ENABLE_SINGLE_IMAGE_SMOKE=false`：跳过 single image 冒烟
- `ENABLE_RUNTIME_API_SANITY=false`：跳过 runtime-metrics/reaper 健全性检查
- `ENABLE_SCOREBOARD_WS_SMOKE=false`：跳过 scoreboard websocket 冒烟

### 8.15 M5 压测与验收

推荐先阅读：`docs/M5_ACCEPTANCE.md`

并发压测：

```bash
API_BASE=http://127.0.0.1:8080/api/v1 \
TEAM_COUNT=20 REQUESTS_TOTAL=400 CONCURRENCY=20 \
backend/scripts/m5/load_benchmark.sh
```

安全回归：

```bash
API_BASE=http://127.0.0.1:8080/api/v1 \
backend/scripts/m5/security_smoke.sh
```

一键验收（生成报告）：

```bash
API_BASE=http://127.0.0.1:8080/api/v1 \
LOAD_TEAM_COUNT=20 LOAD_REQUESTS_TOTAL=400 LOAD_CONCURRENCY=20 \
backend/scripts/m5/full_acceptance.sh
```

默认报告输出目录：`/tmp/rust-ctf-m5-acceptance-<timestamp>/`。

### 8.16 CI 工作流

仓库已新增 GitHub Actions：

- `.github/workflows/ci.yml`
  - 触发：`push` / `pull_request`
  - 执行：`cargo check`（backend）+ `npm run build`（frontend）
- `.github/workflows/m5-acceptance.yml`
  - 触发：`workflow_dispatch`（手动）
  - 执行：启动依赖 + 后端进程 + M5 一键验收脚本，并上传报告 artifacts
  - 默认 `ENABLE_WIREGUARD_SMOKE=false`（避免 hosted runner 的内核能力差异导致波动）
