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
平台支持用户注册、登录、JWT 鉴权、角色权限控制（选手/管理员/裁判），并提供个人资料维护、密码修改、登录历史查看等能力。该模块是整个平台安全边界的第一层，后续将扩展为支持验证码、邮箱激活、双因素认证（2FA）等机制。

2. 队伍管理  
支持队伍创建、邀请成员、成员审批、队长权限变更、队伍信息编辑与解散控制。比赛期间队伍状态与成员关系将被严格校验，防止重复参赛、跨队提交等异常行为。该模块与判题、环境实例分配直接关联，是多容器隔离的身份基础。

3. 题目管理  
管理员可上传题目基础信息（标题、分类、分值、难度、标签、题解可见策略）、附件、判题配置、环境模板（docker-compose 模板与变量定义）。题目支持草稿、发布、下线、版本更新与回滚，以满足教学迭代和赛题维护需求。

4. 比赛管理  
支持创建比赛、设置起止时间、开放题目集合、可见性策略、提交规则、动态积分规则与公告。比赛生命周期（未开始/进行中/已结束）由系统自动驱动，并为管理端提供手动干预接口（暂停、延长、封榜、解榜）。

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
- `backend/`：已提供认证接口（`/api/v1/auth/register`、`/api/v1/auth/login`、`/api/v1/auth/refresh`、`/api/v1/auth/me`）
- `backend/`：已提供比赛基础接口（`/api/v1/contests`、`/api/v1/contests/{contest_id}/challenges`、`/api/v1/submissions`）
- `backend/`：已提供管理员接口（题目管理、比赛创建/编辑/状态控制、比赛题目挂载管理、实例监控）
- `backend/`：已支持默认管理员账号自动初始化（可通过 `DEFAULT_ADMIN_*` 配置）
- `backend/`：提交接口已接入 Redis 限频（30 秒窗口内最多 10 次）
- `backend/`：已提供排行榜接口（`/api/v1/contests/{contest_id}/scoreboard`）
- `backend/`：已提供排行榜 WebSocket 推送接口（`/api/v1/contests/{contest_id}/scoreboard/ws`）
- `backend/`：排行榜 WebSocket 支持浏览器 Token Query 鉴权（`?access_token=...`，同时兼容 `Authorization: Bearer ...`）
- `backend/`：已提供实例生命周期接口（`/api/v1/instances/start|stop|reset|destroy|{contest_id}/{challenge_id}`）
- `backend/`：实例生命周期已接入真实 `docker compose` 编排（模板渲染、compose 文件落盘、启动/停止/重置/销毁）
- `backend/`：已提供管理员审计日志与运行概览接口（`/api/v1/admin/audit-logs`、`/api/v1/admin/runtime/overview`）
- `backend/`：判题已支持静态 flag（明文或 Argon2 哈希）与动态 flag（Redis 键 `flag:dynamic:{contest_id}:{challenge_id}:{team_id}`）
- `backend/`：`script` 判题已支持按题目 metadata 执行外部校验脚本（返回码 0=正确，1=错误，其他=判题异常）
- `frontend/`：已完成选手最小闭环页面（登录/注册、比赛列表、题目列表、Flag 提交、实例控制、实时榜单显示）
- `frontend/`：已完成 API 客户端与本地登录态持久化（Pinia + localStorage）
- `frontend/`：已完成管理员 v2 页面（题目创建/可见性切换、比赛创建与状态切换、题目挂载与排序、实例列表监控）
- `frontend/`：管理员页面已新增审计日志查询与运行概览监控（失败实例告警、提交与实例统计）
- `deploy/`：本地开发用 `docker-compose.dev.yml`（PostgreSQL / Redis / Backend / Frontend）
- `docs/`：初始化后续开发任务说明

下一步进入前端业务页面与后台管理能力的持续实现阶段。

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

### 8.5 已验证的接口链路（2026-02-14）

- `GET /api/v1/health`：健康检查通过，数据库和 Redis 状态正常
- `POST /api/v1/auth/register`、`GET /api/v1/auth/me`、`POST /api/v1/auth/refresh`：认证链路通过
- `POST /api/v1/submissions`：静态哈希 flag 判题通过
- `GET /api/v1/contests/{contest_id}/scoreboard`：排行榜查询通过
- `GET /api/v1/contests/{contest_id}/scoreboard`：未鉴权访问返回 `401`（权限控制生效）
- 提交限频：高频提交后返回 `verdict=rate_limited`

### 8.6 默认管理员账号（可配置）

后端在启动时会自动执行数据库迁移，并根据配置确保存在一个管理员账号（如不存在则创建，存在则提升为 `admin` 并设为 `active`）。

默认配置如下（见 `backend/.env.example` 与 `deploy/docker-compose.dev.yml`）：

- `DEFAULT_ADMIN_ENABLED=true`
- `DEFAULT_ADMIN_USERNAME=admin`
- `DEFAULT_ADMIN_EMAIL=admin@rust-ctf.local`
- `DEFAULT_ADMIN_PASSWORD=admin123456`
- `DEFAULT_ADMIN_FORCE_PASSWORD_RESET=false`

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
