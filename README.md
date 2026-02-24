# Rust-CTF

Rust-CTF 是一个面向教学与实战演练的 CTF 平台，支持传统题目与动态容器靶场。  
当前版本为**前后端可用**状态，覆盖账号、队伍、比赛、题目、判题、实时榜单、动态实例、管理后台与运行监控。

- 后端：Rust + Axum + Tokio + SQLx + PostgreSQL + Redis
- 前端：Vue 3 + TypeScript + Pinia + Vue Router
- 运行环境：Docker / Docker Compose / Kubernetes

## 功能总览（已实现）

### 选手端功能

- 账号体系
  - 注册 / 登录 / 刷新令牌 / 获取当前用户
  - 个人资料修改、密码修改、登录历史查看、账号注销
  - 邮箱验证、邮箱找回密码（可配置启用）
  - 密码策略下发与前端强度提示
  - 可选 Turnstile 人机验证
- 队伍协作
  - 队伍大厅浏览与搜索
  - 创建队伍、按名称/ID 加入队伍、离队、解散队伍
  - 队长转让、移除成员
  - 邀请机制（发送、取消、接受、拒绝）
- 比赛与题目
  - 比赛列表、状态筛选、海报展示、最新公告预览
  - 比赛报名状态查询与报名申请（含待审核/拒绝/通过）
  - 题目列表（分类分组）、题目描述 Markdown 渲染
  - 题目提示（hints）展示
  - 题目附件列表与下载
- 判题与计分
  - Flag 提交
  - 支持 `static` / `dynamic` / `script` 三种判题模式
  - 支持动态积分衰减与一二三血奖励
  - 提交限流（防爆破）
- 动态实例
  - 实例启动 / 停止 / 重置 / 销毁 / 状态查询
  - 支持 `direct` / `ssh_bastion` / `wireguard` 访问模式（按题目配置）
  - WireGuard 配置下载
- 实时可视化
  - 实时榜单（WebSocket）
  - 趋势时间线（timeline）
  - 独立榜单大屏：队伍排名 + 选手排名 + 一二三血标记矩阵
  - 趋势导出（静态图 / WebM 动图）

### 管理端功能

- 站点设置（Admin）
  - 站点文案（标题、副标题、首页内容、Footer）
  - 题目附件大小上限
  - 全站时间展示模式（UTC / 本地时区）
- 用户管理（Admin）
  - 用户列表筛选
  - 启用/禁用用户
  - 角色调整（player/judge/admin）
  - 重置用户密码
  - 删除用户（带保护逻辑）
- 题目管理（Admin/Judge）
  - 题目类别管理（内置类与自定义类）
  - 题目创建/编辑/删除
  - 题目状态与可见性管理
  - 版本记录与回滚
  - 附件上传/列表/删除（支持 runtime 资源文件）
  - 运行模板 lint 校验
  - 运行模板测试：
    - 镜像测试（普通 + 流式日志）
    - Compose 测试（普通 + 流式日志）
- 比赛管理（Admin/Judge）
  - 比赛创建/编辑/删除
  - 比赛状态控制（draft/scheduled/running/ended/archived）
  - 海报上传/删除
  - 比赛题目挂载、排序、发布时间
  - 公告管理（创建/编辑/发布/置顶/删除）
  - 报名审核（pending/approved/rejected）
- 运行与运维（Admin/Judge）
  - 实例列表与状态过滤
  - 单实例容器运行指标（CPU/内存/网络/健康）
  - 运行概览（用户、队伍、比赛、提交、实例）
  - 运行告警（扫描、确认、恢复）
  - 手动执行实例回收（过期回收 / 心跳超时回收）
  - 审计日志查询

### 后端平台能力

- 统一鉴权：JWT Access/Refresh + Redis 会话
- 关键操作审计日志
- 自动数据库迁移（启动时执行）
- 默认管理员自动初始化（可配置）
- 动态实例生命周期编排（Docker Compose）
- 模板变量渲染与约束校验
- 心跳上报与过期/超时回收机制
- Redis Pub/Sub 实时榜单推送

### 前端体验能力

- 双语界面（中文/English）切换
- 明暗主题切换
- 统一 Toast / Alert / Confirm / Prompt 交互
- 页面级错误提示与状态反馈

## 快速开始

### 1) 环境要求

- Docker Engine（需启动）
- Docker Compose（`docker compose`）
- 可选本地开发：Rust stable、Node.js 20+

### 2) 一键启动（推荐）

```bash
docker compose -f deploy/docker-compose.dev.yml up --build
```

启动后访问：

- 前端：`http://localhost:5173`
- 后端健康检查：`http://localhost:8080/api/v1/health`

默认管理员（开发编排）：

- 用户名：`admin`
- 密码：`Qwe@258852`

停止服务：

```bash
docker compose -f deploy/docker-compose.dev.yml down
```

### 3) 本地分离开发（可选）

先启动依赖：

```bash
docker compose -f deploy/docker-compose.dev.yml up -d postgres redis
```

启动后端：

```bash
cd backend
cargo run
```

启动前端：

```bash
cd frontend
npm ci
npm run dev
```

## 项目结构

```text
rust-ctf/
  backend/      Rust 后端服务（API、判题、实例生命周期、审计）
  frontend/     Vue3 前端（选手端 + 管理端）
  deploy/       Docker Compose 与 Kubernetes 部署相关文件
  docs/         API 文档、部署指南、运行手册、验收指南
  runtime/      运行时目录（实例与附件等）
```

## 文档索引

- API 文档：`docs/API_REFERENCE.md`
- 部署指南：`docs/DEPLOYMENT_GUIDE.md`
- Kubernetes 部署指南：`docs/K8S_DEPLOYMENT_GUIDE.md`
- 心跳上报接入：`docs/RUNTIME_HEARTBEAT_REPORTER.md`
- 心跳故障处置：`docs/STALE_HEARTBEAT_REMEDIATION_RUNBOOK.md`
- M5 验收指南：`docs/M5_ACCEPTANCE.md`
- 前端 i18n 文案流程：`frontend/src/locales/README.md`

## CI 与验收

- CI：`.github/workflows/ci.yml`
  - 后端 `cargo check`
  - 前端 `npm run build`
- M5 验收：`.github/workflows/m5-acceptance.yml`
  - 运行回归、安全回归、压测与报告归档

## 部署

单机 Docker 部署请参考：`docs/DEPLOYMENT_GUIDE.md`。  
Kubernetes 部署请参考：`docs/K8S_DEPLOYMENT_GUIDE.md`。  
文档包含生产编排、反向代理（含 WebSocket）、升级回滚、备份恢复与常见故障处理。
