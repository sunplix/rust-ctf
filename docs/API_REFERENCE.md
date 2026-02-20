# Rust CTF API Reference

最后更新：2026-02-19  
适用后端版本：`backend`（Axum / `/api/v1` 路由）

## 1. 维护约定

本文件是当前后端接口的单一事实来源（SSOT），用于前端重构与联调排查。后续开发请遵循：

- 任何新增/删除/修改 API（路径、方法、字段、权限、错误语义）必须在同一次提交中同步更新本文件。
- 若行为变更但路径不变（例如判题窗口、权限策略、字段含义变更），也必须更新本文件。
- 发布前需至少完成一次按本文档的端到端自测（注册 -> 队伍 -> 参赛 -> 提交）。

## 2. 通用约定

### 2.1 Base URL

- 前缀：`/api/v1`

### 2.2 鉴权

- 受保护接口使用：`Authorization: Bearer <access_token>`
- 访问令牌过期或会话失效返回 `401 unauthorized`
- 刷新令牌仅用于 `POST /auth/refresh`

### 2.3 角色模型

- `player`：普通选手
- `judge`：裁判（可访问大多数管理接口）
- `admin`：管理员（可访问全部管理接口）

### 2.4 通用错误响应

```json
{
  "error": {
    "code": "bad_request",
    "message": "具体错误信息"
  }
}
```

`code` 可取：

- `bad_request` (400)
- `unauthorized` (401)
- `forbidden` (403)
- `too_many_requests` (429)
- `conflict` (409)
- `internal_error` (500)

### 2.5 通用数据格式

- 时间：UTC ISO8601（如 `2026-02-15T08:07:08Z`）
- 主键：UUID 字符串
- 除特别说明外，请求体均为 `application/json`

## 3. 健康检查

### `GET /health`

- 鉴权：无需
- 说明：检查服务、数据库、Redis 状态

成功响应（200）：

```json
{
  "status": "ok",
  "service": "rust-ctf-backend",
  "version": "0.1.0",
  "now_utc": "2026-02-15T12:00:00Z",
  "api_port": 8080,
  "dependencies": {
    "database": true,
    "redis": true
  }
}
```

## 4. 认证与用户中心 API

## `GET /site/settings`

- 鉴权：无需
- 用途：获取前台站点文案配置（品牌名、副标题、首页标题/简介/签名、Footer 文案）
- 成功：`200`
- 响应字段：
  - `site_name`
  - `site_subtitle`
  - `home_title`
  - `home_tagline`
  - `home_signature`
  - `footer_text`

## `GET /auth/password-policy`

- 鉴权：无需
- 返回当前后端生效的密码策略配置（长度、强度分、字符类别、弱模式拦截规则）

## `POST /auth/register`

- 鉴权：无需
- 请求体：
  - `username`：必填，3-32，仅字母/数字/`_`/`-`
  - `email`：必填，合法邮箱
  - `password`：必填，需满足后端密码策略
  - `password_confirm`：必填，必须与 `password` 一致
  - `captcha_token`：可选；当后端启用人机验证时必填（Cloudflare Turnstile token）
- 成功：
  - `201`：注册成功并已登录（返回 `auth`）
  - `202`：注册成功但要求先邮箱验证（`requires_email_verification=true`，`auth=null`）
- 失败：
  - 用户名或邮箱重复：`409 conflict`
  - 密码不满足策略或两次密码不一致：`400 bad_request`
  - 人机验证失败：`400 bad_request`

## `POST /auth/login`

- 鉴权：无需
- 请求体：
  - `identifier`：必填，用户名或邮箱
  - `password`：必填
  - `captcha_token`：可选；当后端启用人机验证时必填（Cloudflare Turnstile token）
- 成功：`200`
- 失败：
  - 账号不存在或密码错误：`401 unauthorized`
  - 账号被禁用：`403 forbidden`
  - 启用强制邮箱验证且邮箱未验证：`400 bad_request`
  - 人机验证失败：`400 bad_request`

## `POST /auth/refresh`

- 鉴权：无需（通过 body 中 refresh token）
- 请求体：
  - `refresh_token`：必填
- 成功：`200`
- 失败：无效刷新令牌或会话失效 `401 unauthorized`

## `GET /auth/me`

- 鉴权：必须
- 返回当前登录用户信息（激活状态用户）

## `PATCH /auth/profile`

- 鉴权：必须
- 请求体（至少一个字段）：
  - `username`：可选，校验同注册
  - `email`：可选，校验同注册
- 失败：
  - 字段都为空：`400`
  - 用户名/邮箱冲突：`409`

## `POST /auth/change-password`

- 鉴权：必须
- 请求体：
  - `current_password`：必填
  - `new_password`：必填，需满足后端密码策略，且不能与旧密码相同
  - `new_password_confirm`：可选；若提供必须与 `new_password` 一致
- 成功后：会撤销该用户所有旧会话并签发新 token

## `POST /auth/email-verification/request`

- 鉴权：无需
- 请求体：
  - `email`：必填
- 说明：如果账号存在且尚未验证，会发送验证邮件（防枚举返回通用提示）

## `POST /auth/email-verification/confirm`

- 鉴权：无需
- 请求体：
  - `token`：必填
- 失败：令牌无效或过期 `400 bad_request`

## `POST /auth/password-reset/request`

- 鉴权：无需
- 请求体：
  - `email`：必填
- 说明：如果账号存在，会发送重置邮件（防枚举返回通用提示）

## `POST /auth/password-reset/confirm`

- 鉴权：无需
- 请求体：
  - `token`：必填
  - `new_password`：必填，需满足后端密码策略
  - `new_password_confirm`：必填，必须与 `new_password` 一致
- 失败：令牌无效或过期/密码不满足策略 `400 bad_request`

## `DELETE /auth/account`

- 鉴权：必须
- 作用：删除当前账号（逻辑删除：匿名化并禁用，立即撤销全部会话）
- 约束：
  - 若当前用户是队长，需先移交队长或解散队伍，否则返回 `409`
  - 若当前用户是最后一个 active admin，返回 `409`
- 成功：`204`

## `GET /auth/login-history`

- 鉴权：必须
- Query：
  - `limit`：可选，默认 `30`，范围 `1..200`
- 返回当前用户认证相关审计日志（`auth.register/login/refresh/password.change`）

### 认证模块公共响应模型

`AuthResponse`（login/refresh/change-password，及 register 成功自动登录场景）：

```json
{
  "access_token": "string",
  "refresh_token": "string",
  "token_type": "Bearer",
  "access_expires_in_seconds": 3600,
  "refresh_expires_in_seconds": 604800,
  "user": {
    "id": "uuid",
    "username": "string",
    "email": "string",
    "role": "player|judge|admin",
    "email_verified": true,
    "email_verified_at": "datetime|null",
    "created_at": "datetime"
  }
}
```

`RegisterResponse`：

```json
{
  "requires_email_verification": false,
  "message": "registration succeeded",
  "auth": {
    "access_token": "string",
    "refresh_token": "string",
    "token_type": "Bearer",
    "access_expires_in_seconds": 3600,
    "refresh_expires_in_seconds": 604800,
    "user": {
      "id": "uuid",
      "username": "string",
      "email": "string",
      "role": "player|judge|admin",
      "email_verified": true,
      "email_verified_at": "datetime|null",
      "created_at": "datetime"
    }
  }
}
```

## 5. 队伍管理 API

全部接口均需鉴权。

### 5.1 队伍基础

- `GET /teams`
  - Query：`keyword`（可选）、`limit`（默认50，1..200）
  - 返回队伍列表（含队长名与成员数）
- `GET /teams/me`
  - 返回当前用户队伍；无队伍时 `{"team": null}`
- `GET /teams/{team_id}`
  - 返回指定队伍详情
- `POST /teams`
  - 请求体：`name`(必填,<=64)、`description`(可选,<=500)
  - 约束：用户只能在一个队伍中；同名队伍冲突返回 `409`
- `POST /teams/join`
  - 请求体：`team_id` 或 `team_name` 二选一
  - 约束：用户只能在一个队伍中
- `POST /teams/leave`
  - 普通成员可离队
  - 队长离队规则：
    - 若仍有其他成员：`409`（需先移交队长或解散）
    - 若仅自己：自动解散队伍
- `PATCH /teams/{team_id}`
  - 仅队长可修改
  - 请求体：`name?`、`description?`，至少一个
- `DELETE /teams/{team_id}`
  - 仅队长可解散，成功返回 `204`

### 5.2 队长扩展能力

- `POST /teams/{team_id}/transfer-captain`
  - 请求体：`new_captain_user_id`
  - 目标必须是当前队伍成员，且不能是自己
- `DELETE /teams/{team_id}/members/{member_user_id}`
  - 仅队长可移除成员
  - 队长不能移除自己

### 5.3 邀请机制

- `POST /teams/invitations`
  - 仅队长
  - 请求体：
    - `invitee_user_id` 或 `invitee_username`（二选一）
    - `message`（可选，<=500）
- `GET /teams/invitations/received`
  - Query：`status`（`pending|accepted|rejected|canceled|expired`）、`limit`（默认50，1..200）
- `GET /teams/invitations/sent`
  - 仅队长
  - Query 同上
- `POST /teams/invitations/{invitation_id}/respond`
  - 请求体：`action` 必须为 `accept` 或 `reject`
  - 接受邀请前会校验当前用户是否已在其他队伍
- `POST /teams/invitations/{invitation_id}/cancel`
  - 仅发起队伍队长可取消，且仅 `pending` 可取消

### 5.4 主要响应模型

- `TeamListItem`：`id,name,description,captain_user_id,captain_username,member_count,created_at,updated_at`
- `TeamProfile`：`id,name,description,captain_user_id,captain_username,created_at,updated_at,members[]`
- `TeamMemberItem`：`user_id,username,member_role(captain|member),joined_at`
- `LeaveTeamResponse`：`team_id,disbanded,message`
- `TeamInvitationItem`：`id,team_id,team_name,inviter_user_id,inviter_username,invitee_user_id,invitee_username,status,message,created_at,updated_at,responded_at`
- `InvitationRespondResponse`：`invitation,team?`

## 6. 选手侧比赛与题目 API

## `GET /contests`

- 鉴权：无需
- 仅返回 `public` 且状态在 `scheduled|running|ended` 的比赛
- 响应字段：
  - `id,title,slug,description,poster_url,status,scoring_mode,dynamic_decay,latest_announcement_title,latest_announcement_content,latest_announcement_published_at,start_at,end_at`
  - `poster_url` 为空表示未上传海报
  - `latest_announcement_*` 为空表示暂无已发布公告
- 排序：
  - `running` 优先，其次 `scheduled`，最后 `ended`
  - `running` 内按 `end_at` 升序，其他按 `start_at` 升序

## `GET /contests/{contest_id}/poster`

- 鉴权：无需
- 用于比赛中心展示海报图片
- 访问控制：
  - 仅 `public` 且状态为 `scheduled|running|ended` 的比赛可访问
- 成功：返回海报二进制（`Content-Type` 为上传时记录的 `image/*`）
- 失败：
  - 比赛不存在或海报不存在：`400`
  - 比赛不对公众可见：`403`

## `GET /contests/{contest_id}/challenges`

- 鉴权：必须
- 访问控制：
  - 私有比赛：仅 `admin|judge`
  - `draft|archived` 比赛：仅 `admin|judge`
- 仅返回：
  - `is_visible=true` 的题目
  - 比赛状态 `running|ended`
  - 已到发布时间（`release_at <= now` 或为空）
- 响应字段：
  - `id,title,category,difficulty,challenge_type,static_score,release_at`

## `GET /contests/{contest_id}/announcements`

- 鉴权：必须
- 访问控制同上
- 仅返回已发布公告（`is_published=true` 且 `published_at<=now` 或为空）
- 排序：置顶优先，再按发布时间/创建时间倒序
- `content` 支持 Markdown 文本；前端按安全策略渲染为富文本展示

## 7. 判题提交 API

## `POST /submissions`

- 鉴权：必须
- 请求体：
  - `contest_id`：UUID
  - `challenge_id`：UUID
  - `flag`：字符串（去除首尾空白后不能为空）
- 前置约束：
  - 用户必须属于某个队伍，否则 `403`
  - 题目必须已挂载到比赛
  - 题目需可见且已到发布时间
  - 比赛状态必须是 `running`
- 限频：
  - 每用户每比赛 30 秒最多 10 次
  - 超限返回 `verdict=rate_limited`（HTTP 200，业务层限频）
- 判题模式：
  - `static`：支持明文或 Argon2 哈希
  - `dynamic`：从 Redis 键读取动态 flag  
    `flag:dynamic:{contest_id}:{challenge_id}:{team_id}`
  - `script`：从题目 `metadata.script_verifier` 读取脚本配置执行
- 动态积分：
  - 比赛 `scoring_mode=dynamic` 时生效
  - 依据已解队伍数和 `dynamic_decay` 衰减，分数范围受 `min_score/max_score` 限制
- 一二三血加成：
  - 比赛可配置 `first_blood_bonus_percent/second_blood_bonus_percent/third_blood_bonus_percent`
  - 当前题目首个/第二个/第三个完成解题的队伍，会在基础分上按百分比追加加成

响应模型 `SubmitFlagResponse`：

```json
{
  "verdict": "accepted|wrong|invalid|rate_limited",
  "score_awarded": 100,
  "total_score": 300,
  "message": "string",
  "submitted_at": "datetime"
}
```

## 8. 实例生命周期 API

全部接口需鉴权，且用户必须有队伍成员身份。

### 请求体（通用）

```json
{
  "contest_id": "uuid",
  "challenge_id": "uuid"
}
```

### `POST /instances/start`

- 启动（或复用）队伍题目实例
- 题目限制：
  - `challenge_type` 必须是 `dynamic` 或 `internal`
  - 运行模式二选一：
    - `metadata.runtime.mode=compose`（默认）：使用 `compose_template`
    - `metadata.runtime.mode=single_image`：使用 `metadata.runtime.image + metadata.runtime.internal_port` 自动生成运行模板
  - 题目需可见且已到发布时间
  - 非 `admin|judge` 时比赛需 `running`
- 访问入口：
  - `compose` 模式默认启用 `ssh_bastion`（可通过 `metadata.runtime.access_mode=direct|wireguard` 切换）
  - `single_image` 模式会自动分配随机高位端口并映射到指定内部端口
- 启动自愈：
  - 首次 `compose up` 失败时，后端会自动尝试一次 `compose down` + `compose up --force-recreate` 自愈重试
  - 若重试仍失败，接口返回 `400`，并在实例记录上标记为 `failed`
- 实例 TTL：2 小时（会写入 `expires_at`）

### `POST /instances/stop`

- 停止实例（状态改为 `stopped`）

### `POST /instances/reset`

- 先 `down` 再 `up --force-recreate` 重置实例
- 若 `up` 阶段失败，同样会触发一次自动自愈重试（`down` + `up --force-recreate`）

### `POST /instances/destroy`

- 销毁实例（`down --volumes --remove-orphans`），状态改为 `destroyed`

### `POST /instances/heartbeat`

- 刷新运行中实例心跳时间（`last_heartbeat_at=now`）
- 仅当该队伍对应实例处于 `running` 状态时成功
- 若实例不存在或非运行态，返回 `400`（`running instance not found`）

### `POST /instances/heartbeat/report`

- 供靶机内“心跳上报器”调用的无会话心跳接口
- 请求体：
  - `token`：实例心跳令牌（由后端在 `compose_template` 渲染阶段注入）
- 行为：
  - 校验令牌签名与有效期
  - 将目标实例 `last_heartbeat_at` 刷新为 `now`
- 错误：
  - 令牌无效或过期：`401`
  - 实例不存在或非 `running`：`400`

### `GET /instances/{contest_id}/{challenge_id}`

- 查询当前用户所属队伍在该题目的实例

### `GET /instances/{contest_id}/{challenge_id}/wireguard-config`

- 获取当前队伍实例的 WireGuard 客户端配置
- 前置条件：
  - 当前实例访问模式为 `wireguard`
  - 实例未销毁，且 WireGuard 容器已生成配置
- 返回：`contest_id,challenge_id,team_id,endpoint,filename,content`
- `content` 为完整客户端配置文本，可直接保存为 `.conf` 导入 WireGuard 客户端
- 若实例刚启动，可能短暂返回 `400 wireguard config is not ready`，应按 1~2 秒间隔重试

### 生命周期补充说明

- 后端默认启用后台实例回收器：按配置周期扫描 `expires_at <= now` 且未销毁实例，自动执行销毁与运行目录清理。
- 回收器配置项：`INSTANCE_REAPER_ENABLED`、`INSTANCE_REAPER_INTERVAL_SECONDS`、`INSTANCE_REAPER_INITIAL_DELAY_SECONDS`、`INSTANCE_REAPER_BATCH_SIZE`。
- 实例默认资源配额支持配置：`INSTANCE_DEFAULT_CPU_LIMIT`、`INSTANCE_DEFAULT_MEMORY_LIMIT_MB`。
- 主机随机端口配置：`INSTANCE_HOST_PORT_MIN`、`INSTANCE_HOST_PORT_MAX`。
- 选手外部访问主机名配置：`INSTANCE_PUBLIC_HOST`。
- 靶机心跳上报配置：`INSTANCE_HEARTBEAT_REPORT_URL`、`INSTANCE_HEARTBEAT_REPORT_INTERVAL_SECONDS`。
- 心跳超时阈值与自动处置配置：`INSTANCE_HEARTBEAT_STALE_SECONDS`、`INSTANCE_STALE_REAPER_ENABLED`、`INSTANCE_STALE_REAPER_BATCH_SIZE`（默认仅告警，不自动销毁）。

### 响应模型 `InstanceResponse`

`id,contest_id,challenge_id,team_id,status,subnet,compose_project_name,entrypoint_url,cpu_limit,memory_limit_mb,started_at,expires_at,destroyed_at,last_heartbeat_at,network_access?,message`

`network_access`（`ssh_bastion` 或 `wireguard` 模式返回）：

- `mode`：`ssh_bastion` 或 `wireguard`
- `host`：连接主机
- `port`：随机高位端口
- `username`：跳板账号
- `password`：跳板密码
- `download_url`：WireGuard 配置下载路径（仅 `wireguard`）
- `note`：接入说明（`ssh_bastion` 默认支持 `sudo`，可在跳板中按需安装扫描工具）

`wireguard-config` 响应示例：

```json
{
  "contest_id": "uuid",
  "challenge_id": "uuid",
  "team_id": "uuid",
  "endpoint": "wg://127.0.0.1:51888",
  "filename": "contestid-challengeid-teamid.conf",
  "content": "[Interface]\\nAddress = 10.13.13.2\\n..."
}
```

## 9. 排行榜 API

### `GET /contests/{contest_id}/scoreboard`

- 鉴权：必须
- 访问控制：
  - 私有比赛：仅 `admin|judge`
  - `draft|archived` 比赛：仅 `admin|judge`
- 排序：`score DESC` -> `solved_count DESC` -> `last_submit_at ASC`
- 平分并列名次（`rank` 相同）

`ScoreboardEntry` 字段：

- `rank,team_id,team_name,score,solved_count,last_submit_at`

### `GET /contests/{contest_id}/scoreboard/timeline`

- 鉴权：必须
- 用途：返回积分/排名趋势快照，用于绘制动态折线图与导出动画
- Query（可选）：
  - `max_snapshots`（默认 800，范围 1..5000）
  - `top_n`（默认 12，范围 1..200）
- 返回：
  - `contest_id,generated_at`
  - `snapshots[]`：
    - `trigger_submission_id,timestamp,entries[]`
    - `entries[]` 结构同 `ScoreboardEntry`
  - `latest_entries[]`（当前榜单快照，结构同 `ScoreboardEntry`）

### `GET /contests/{contest_id}/scoreboard/ws`

- 鉴权：必须（两种方式二选一）
  - Header：`Authorization: Bearer <access_token>`
  - Query：`?access_token=...`（或 `?token=...`）
- 连接成功后先推送全量快照，再在 Redis 频道更新时推送
- 推送 payload：

```json
{
  "event": "scoreboard_update",
  "contest_id": "uuid",
  "entries": [
    {
      "rank": 1,
      "team_id": "uuid",
      "team_name": "string",
      "score": 500,
      "solved_count": 5,
      "last_submit_at": "datetime"
    }
  ]
}
```

## 10. 管理端 API

管理端全部在 `/admin/*` 下，均需鉴权。

- 用户管理与站点设置：仅 `admin`
- 其余管理接口：`admin|judge`

## 10.1 站点设置（admin only）

- `GET /admin/site-settings`
  - 返回当前站点文案配置与更新时间
- `PATCH /admin/site-settings`
  - Body（至少一个字段）：
    - `site_name`（1..80）
    - `site_subtitle`（0..160）
    - `home_title`（1..160）
    - `home_tagline`（0..2000）
    - `home_signature`（0..200）
    - `footer_text`（0..240）
  - 说明：仅更新请求中提供的字段；会记录审计日志 `admin.site.settings.update`

`AdminSiteSettingsItem`：
`site_name,site_subtitle,home_title,home_tagline,home_signature,footer_text,updated_by,updated_at`

## 10.2 用户管理（admin only）

- `GET /admin/users`
  - Query：
    - `keyword`（用户名/邮箱模糊）
    - `role`（`player|admin|judge`）
    - `status`（`active|disabled`）
    - `limit`（默认200，1..1000）
- `PATCH /admin/users/{user_id}/status`
  - Body：`status`
  - 不能禁用当前管理员自身
  - 禁用用户会撤销其全部会话
- `PATCH /admin/users/{user_id}/role`
  - Body：`role`
  - 不能把当前管理员自己降级为非 admin
- `POST /admin/users/{user_id}/reset-password`
  - Body：`new_password`（>=8）
  - 成功后撤销目标用户全部会话
- `DELETE /admin/users/{user_id}`
  - 删除账号（逻辑删除：匿名化并禁用）
  - 限制：
    - 不能删除当前管理员自己
    - 不能删除最后一个 active admin
  - 成功：`204`

`AdminUserItem`：`id,username,email,role,status,created_at,updated_at`

## 10.3 题目管理（admin|judge）

### 题目类别管理

- `GET /admin/challenge-categories`
  - 返回：`id,slug,display_name,sort_order,is_builtin,created_at,updated_at`
- `POST /admin/challenge-categories`
  - Body：`slug`（必填）、`display_name?`、`sort_order?`
  - 约束：
    - `slug` 仅允许 `[a-z0-9_-]`，长度 1..32，大小写不敏感唯一
    - `display_name` 最长 64
    - `sort_order` 范围 `-100000..100000`
- `PATCH /admin/challenge-categories/{category_id}`
  - 可更新字段：`slug,display_name,sort_order`
  - 内置类别（`is_builtin=true`）不可修改 `slug`
- `DELETE /admin/challenge-categories/{category_id}`
  - 内置类别不可删除
  - 被题目使用中的类别不可删除（返回 `409`）

### `GET /admin/challenges`

- 列表字段：  
  `id,title,slug,category,difficulty,static_score,challenge_type,flag_mode,status,is_visible,tags,writeup_visibility,current_version,created_at,updated_at`

### `GET /admin/challenges/{challenge_id}`

- 返回题目完整配置（用于管理端“编辑题目”）
- 字段包含：  
  `id,title,slug,category,difficulty,description,static_score,min_score,max_score,challenge_type,flag_mode,status,flag_hash,compose_template,metadata,is_visible,tags,writeup_visibility,writeup_content,current_version,created_at,updated_at`

### `GET /admin/challenges/runtime-template/lint`

- 用途：批量扫描题库运行模板规范性（`compose_template` 或 `metadata.runtime.single_image`）
- Query：
  - `limit`（默认500，1..5000）
  - `challenge_type`（`static|dynamic|internal`，可选）
  - `status`（`draft|published|offline`，可选）
  - `keyword`（按标题/slug 模糊过滤，可选）
  - `only_errors`（默认 `false`；为 `true` 时只返回错误项）
- 返回：
  - `generated_at`
  - `scanned_total`（扫描总数）
  - `returned_total`（实际返回项数）
  - `ok_count`
  - `error_count`
  - `items[]`

`items[]` 字段：

- `id,title,slug,challenge_type,status,is_visible,has_compose_template,lint_status,message,updated_at`
- `lint_status` 目前为 `ok|error`

### `POST /admin/challenges/runtime-template/test-image`

- 用途：管理员在创建/编辑容器题目时测试镜像可用性，并查看拉取与构建探测日志
- Body：
  - `image`（必填，镜像引用）
  - `force_pull`（可选，默认 `true`）
  - `run_build_probe`（可选，默认 `true`，会执行一次基于该镜像的最小构建探测）
  - `timeout_seconds`（可选；默认后端 compose 命令超时值，范围 10..900）
- 返回：
  - `image,force_pull,run_build_probe,succeeded,generated_at,steps[]`
  - `steps[]` 字段：`step,success,exit_code,duration_ms,output,truncated`
  - 常见 step：`runtime_pull`、`runtime_config_validate`、`runtime_build_probe`、`runtime_cleanup_probe`
  - 执行器策略：优先 `docker compose`，不可用时自动回退 `docker-compose`（`output` 会带 `[executor=docker-compose]` 前缀）

### `POST /admin/challenges`

- 支持字段：
  - 基础：`title,slug,category,description,difficulty,tags`
  - 判题：`challenge_type,flag_mode,flag_hash,metadata`
  - 分值：`static_score,min_score,max_score`
  - 环境：`compose_template`
  - 题解：`writeup_visibility,writeup_content`
  - 发布控制：`status,is_visible`
  - 版本备注：`change_note`
- 关键约束：
  - `category` 必须存在于 `challenge_categories`
  - 默认内置类别：`misc,crypto,web,reverse,mobile,osint,pwn,penetration`
  - `difficulty`：`easy|normal|hard|insane`
  - `challenge_type`：`static|dynamic|internal`
  - `flag_mode`：`static|dynamic|script`
  - `status`：`draft|published|offline`
  - `status` 与 `is_visible` 必须一致（`published <=> true`）
  - `static_score > 0`
  - `min_score >= 0` 且 `max_score >= min_score`
  - `tags` 最多 32 项，每项最长 32
  - `writeup_content` 最长 20000
  - `slug` 唯一
- 成功后自动写入 `challenge_versions` 初始快照
- `metadata.runtime` 支持：
  - `mode`：`compose`（默认）或 `single_image`
  - `access_mode`：`direct` 或 `ssh_bastion` 或 `wireguard`（`compose` 模式默认 `ssh_bastion`）
  - `single_image` 模式专用：
    - `image`：镜像仓库地址（如 `nginx:alpine`）
    - `internal_port`：容器内部端口（1..65535）
    - `protocol`：`http|https|tcp`（默认 `http`）
- `compose_template` 常用占位符（渲染后替换）：
  - 已有：`{{SUBNET}}`、`{{ENTRYPOINT_HOST}}`、`{{DYNAMIC_FLAG}}`、`{{CPU_LIMIT}}`、`{{MEMORY_LIMIT_MB}}` 等
  - 端口映射/接入新增：`{{HOST_PORT}}`、`{{PUBLIC_HOST}}`、`{{ACCESS_HOST_PORT}}`、`{{ACCESS_USERNAME}}`、`{{ACCESS_PASSWORD}}`
  - 心跳上报新增：`{{HEARTBEAT_REPORT_URL}}`、`{{HEARTBEAT_REPORT_TOKEN}}`、`{{HEARTBEAT_INTERVAL_SECONDS}}`
  - 自定义变量：`{{VAR:NAME}}`
- `compose_template` schema 约束：
  - 仅允许使用“保留占位符”与 `{{VAR:NAME}}` 两类占位符
  - `{{VAR:NAME}}` 必须在 `metadata.compose_variables` 中定义
  - `dynamic/internal` 题型必须满足：
    - `metadata.runtime.mode=compose` 时提供有效 `compose_template`
    - `metadata.runtime.mode=single_image` 时提供有效 `metadata.runtime.image/internal_port`
- `metadata.compose_variables` 支持两种格式：
  - 对象映射：`{"APP_PORT":"8080","DB_HOST":{"value":"db","required":true}}`
  - 数组定义：`[{"name":"APP_PORT","value":"8080","required":true}]`

### `PATCH /admin/challenges/{challenge_id}`

- 可更新字段：除 `min_score/max_score` 外的大部分题目字段
- `status/is_visible` 一致性规则同创建
- 成功后 `current_version + 1` 并写入版本快照

### `DELETE /admin/challenges/{challenge_id}`

- 销毁题目（含实例清理）
- 行为：
  - 先尝试销毁该题目相关所有运行实例
  - 若实例清理失败（如 compose down 失败），返回 `409`，题目不会被删除
  - 清理通过后删除题目记录（级联删除挂载、提交、版本、附件元数据等），并回收附件文件目录
- 成功：`204`

### `GET /admin/challenges/{challenge_id}/versions`

- Query：`limit`（默认30，1..200）
- 返回版本历史：
  - `id,challenge_id,version_no,change_note,created_by,created_by_username,created_at`

### `POST /admin/challenges/{challenge_id}/rollback`

- Body：
  - `version_no`（>=1）
  - `change_note`（可选）
- 行为：
  - 将题目字段还原到指定历史快照
  - 再次递增版本并记录“回滚后”的新快照

## 10.4 题目附件管理（admin|judge）

- `POST /admin/challenges/{challenge_id}/attachments`
  - Body：
    - `filename`（必填，<=255）
    - `content_base64`（必填）
    - `content_type`（可选，默认 `application/octet-stream`）
  - 约束：附件内容不能为空，且 <= 20MB
- `GET /admin/challenges/{challenge_id}/attachments`
  - Query：`limit`（默认100，1..500）
- `DELETE /admin/challenges/{challenge_id}/attachments/{attachment_id}`
  - 成功 `204`

`AdminChallengeAttachmentItem`：

- `id,challenge_id,filename,content_type,storage_path,size_bytes,uploaded_by,uploaded_by_username,created_at`

说明：当前仅提供“上传/查询/删除”元数据接口，未提供单独下载 API。

## 10.5 比赛管理（admin|judge）

### `GET /admin/contests`

- 返回字段：  
  `id,title,slug,description,poster_url,visibility,status,scoring_mode,dynamic_decay,first_blood_bonus_percent,second_blood_bonus_percent,third_blood_bonus_percent,start_at,end_at,freeze_at,created_at,updated_at`

### `POST /admin/contests`

- Body：
  - `title,slug,start_at,end_at` 必填
  - 可选：`description,visibility,status,scoring_mode,dynamic_decay,first_blood_bonus_percent,second_blood_bonus_percent,third_blood_bonus_percent,freeze_at`
- 约束：
  - `visibility`：`public|private`
  - `status`：`draft|scheduled|running|ended|archived`
  - `scoring_mode`：`static|dynamic`
  - `dynamic_decay`：`1..100000`
  - `first_blood_bonus_percent`：`0..500`（默认 10）
  - `second_blood_bonus_percent`：`0..500`（默认 5）
  - `third_blood_bonus_percent`：`0..500`（默认 2）
  - `end_at` 必须晚于 `start_at`
  - `freeze_at` 必须在 `[start_at, end_at]` 区间内
  - `slug` 唯一

### `PATCH /admin/contests/{contest_id}`

- 可更新字段：`title,slug,description,visibility,status,scoring_mode,dynamic_decay,first_blood_bonus_percent,second_blood_bonus_percent,third_blood_bonus_percent,start_at,end_at,freeze_at,clear_freeze_at`
- `clear_freeze_at=true` 时清空封榜时间
- 时间窗口、`dynamic_decay`、血量加成百分比约束同创建

### `PATCH /admin/contests/{contest_id}/status`

- Body：`status`（同上枚举）

### `DELETE /admin/contests/{contest_id}`

- 销毁比赛（含实例清理）
- 行为：
  - 先尝试销毁该比赛下所有运行实例
  - 若实例清理失败，返回 `409`，比赛不会被删除
  - 清理通过后删除比赛记录（级联删除挂载、公告、提交、实例等），并回收海报文件目录
- 成功：`204`

### `POST /admin/contests/{contest_id}/poster`

- 上传或替换比赛海报
- Body：
  - `filename`（必填，<=255）
  - `content_base64`（必填）
  - `content_type`（可选，但必须是 `image/*`；未填时按默认值处理后校验）
- 约束：海报内容不能为空，且 <= 8MB
- 成功：返回更新后的 `AdminContestItem`（包含 `poster_url`）

### `DELETE /admin/contests/{contest_id}/poster`

- 删除比赛海报
- 成功：`204`

## 10.6 比赛题目挂载（admin|judge）

- `GET /admin/contests/{contest_id}/challenges`
- `POST /admin/contests/{contest_id}/challenges`
  - Body：`challenge_id,sort_order?,release_at?`
  - 行为：同 `challenge_id` 重复时执行 upsert（覆盖排序与发布时间）
- `PATCH /admin/contests/{contest_id}/challenges/{challenge_id}`
  - Body：`sort_order?,release_at?,clear_release_at?`
  - 至少一个字段
- `DELETE /admin/contests/{contest_id}/challenges/{challenge_id}`
  - 成功 `204`

`AdminContestChallengeItem`：

- `contest_id,challenge_id,challenge_title,challenge_category,challenge_difficulty,sort_order,release_at`

## 10.7 公告管理（admin|judge）

- `GET /admin/contests/{contest_id}/announcements`
  - Query：`limit`（默认200，1..1000）
- `POST /admin/contests/{contest_id}/announcements`
  - Body：`title,content,is_published?,is_pinned?`
  - `content` 支持 Markdown（建议使用，便于结构化公告）
  - 若 `is_published=true`，创建时自动写入 `published_at=now`
- `PATCH /admin/contests/{contest_id}/announcements/{announcement_id}`
  - Body：`title?,content?,is_published?,is_pinned?`
  - `content` 可继续使用 Markdown 更新
  - 至少一个字段
  - 发布状态切换逻辑：
    - 置为发布：若历史 `published_at` 为空则补当前时间
    - 置为未发布：清空 `published_at`
- `DELETE /admin/contests/{contest_id}/announcements/{announcement_id}`
  - 成功 `204`

`AdminContestAnnouncementItem`：

- `id,contest_id,title,content,is_published,is_pinned,published_at,created_by,created_by_username,updated_by,updated_by_username,created_at,updated_at`

## 10.8 运行态与审计（admin|judge）

- `GET /admin/instances`
  - Query：
    - `status`（`creating|running|stopped|destroyed|expired|failed`）
    - `limit`（默认100，1..500）
- `GET /admin/instances/{instance_id}/runtime-metrics`
  - 用途：采集该实例所属 compose 项目的容器运行指标（CPU、内存、网络、健康状态）
  - 说明：
    - 指标基于 `docker ps/inspect/stats --no-stream` 实时采样
    - 若实例已无容器（例如已销毁），`services` 为空，并在 `warnings[]` 说明原因
- `GET /admin/audit-logs`
  - Query：
    - `action`（精确匹配）
    - `target_type`（精确匹配）
    - `actor_user_id`
    - `limit`（默认200，1..1000）
- `GET /admin/runtime/overview`
  - 返回平台运行概览聚合统计
- `GET /admin/runtime/alerts`
  - Query：
    - `status`（`open|acknowledged|resolved`）
    - `severity`（`info|warning|critical`）
    - `alert_type`（精确匹配）
    - `limit`（默认100，1..500）
- `POST /admin/runtime/alerts/scan`
  - 触发一次运行时告警扫描（失败实例、即将过期、过期未销毁、心跳超时）
  - 自动去重（按 `fingerprint`）、刷新 `last_seen_at`，并自动关闭不再命中的历史告警
  - 说明：后端默认也会按配置后台定时执行同一套扫描逻辑
  - 心跳超时判定阈值由 `INSTANCE_HEARTBEAT_STALE_SECONDS` 控制（默认 300 秒）
- `POST /admin/runtime/reaper/expired`
  - 立即执行一次“过期实例回收”（同后台 reaper 逻辑）
  - 批大小使用 `INSTANCE_REAPER_BATCH_SIZE`
- `POST /admin/runtime/reaper/stale`
  - 立即执行一次“心跳超时实例回收”
  - 超时阈值使用 `INSTANCE_HEARTBEAT_STALE_SECONDS`
  - 批大小使用 `INSTANCE_STALE_REAPER_BATCH_SIZE`
- `POST /admin/runtime/alerts/{alert_id}/ack`
  - 将告警标记为 `acknowledged`
  - 可选 Body：`{"note":"..."}`（用于审计备注）
- `POST /admin/runtime/alerts/{alert_id}/resolve`
  - 将告警标记为 `resolved`
  - 可选 Body：`{"note":"..."}`（用于审计备注）

`AdminInstanceItem`：

- `id,contest_id,contest_title,challenge_id,challenge_title,team_id,team_name,status,subnet,compose_project_name,entrypoint_url,started_at,expires_at,destroyed_at,last_heartbeat_at,created_at,updated_at`

`AdminInstanceRuntimeMetricsResponse`：

- `generated_at,instance,summary,services,warnings`
- `summary`：
  - `services_total,running_services,unhealthy_services,restarting_services,cpu_percent_total,memory_usage_bytes_total,memory_limit_bytes_total`
- `services[]`：
  - `container_id,container_name,service_name,image,state,health_status,restart_count,started_at,finished_at,ip_addresses,cpu_percent,memory_usage_bytes,memory_limit_bytes,memory_percent,net_rx_bytes,net_tx_bytes,block_read_bytes,block_write_bytes,pids`

`AdminAuditLogItem`：

- `id,actor_user_id,actor_username,actor_role,action,target_type,target_id,detail,created_at`

`AdminRuntimeOverview`：

- `generated_at,total_users,total_teams,total_contests,running_contests,total_challenges,total_submissions,submissions_last_24h,instances_total,instances_running,instances_failed,instances_expiring_within_30m,instances_expired_not_destroyed,recent_failed_instances[]`

`AdminRuntimeAlertItem`：

- `id,alert_type,severity,status,source_type,source_id,fingerprint,title,message,detail,first_seen_at,last_seen_at,acknowledged_at,acknowledged_by,acknowledged_by_username,resolved_at,resolved_by,resolved_by_username,created_at,updated_at`

`AdminRuntimeAlertScanResponse`：

- `generated_at,upserted,auto_resolved,open_count,acknowledged_count,resolved_count`

`AdminInstanceReaperRunResponse`：

- `generated_at,mode,heartbeat_stale_seconds,scanned,reaped,failed,skipped`

## 11. 常见排障提示

- `403 permission denied`（提交/实例启动）：通常是“用户不在队伍中”或角色无权限。
- `400 contest is not running`：提交与实例（非管理员）都受比赛状态约束。
- `400 challenge has not been released yet`：题目已挂载但 `release_at` 未到。
- `200 verdict=rate_limited`：不是 HTTP 失败，而是业务限频（30 秒内超过 10 次）。
- `400 challenge type does not require runtime instance`：仅 `dynamic/internal` 题型可启动实例。
- `runtime alert: instance_heartbeat_stale`：见 `docs/STALE_HEARTBEAT_REMEDIATION_RUNBOOK.md` 进行定位与处置。
- 推荐先执行 `backend/scripts/runtime/runtime_full_regression.sh` 做基线排查（包含 health、运行模板 lint、WireGuard/SingleImage 冒烟、runtime-metrics/reaper 健全性、scoreboard ws 冒烟）。

## 12. 后续维护建议

- 后端每次新增路由时，先更新本文件再提测。
- 前端 API 层建议严格按本文件建类型（TS interface）并做枚举约束。
- 若未来改为 OpenAPI 自动导出，可保留本文件做“业务语义补充说明”。
