# Rust-CTF 部署指南（单机 Docker 版）

最后更新：2026-02-20  
适用范围：当前仓库 `master`（以 `deploy/docker-compose.dev.yml` 为基础）

## 1. 目标与范围

本文提供一套可直接落地的单机部署方案，目标是：

- 从本地开发编排平滑迁移到线上可运维形态
- 保留当前项目的动态实例能力（`/var/run/docker.sock` 模式）
- 降低日常运维复杂度（启动、升级、备份、回滚）

说明：

- 当前仓库仍以开发编排为主，尚未内置生产编排文件。
- 本文会给出 `deploy/docker-compose.prod.yml` 的建议模板与上线流程。

## 2. 资源建议

按你当前服务栈（PostgreSQL + Redis + Backend + Frontend）：

- 最低可运行：`2 vCPU / 4 GB RAM / 40 GB SSD`（仅小规模联调）
- 生产起步：`4 vCPU / 8 GB RAM / 80 GB SSD`（小型比赛）
- 建议可用：`8 vCPU / 16 GB RAM / 160 GB SSD`（有一定并发开题）

补充：

- 当前默认实例资源上限为 `1.0 CPU` + `512MB`（`INSTANCE_DEFAULT_CPU_LIMIT`、`INSTANCE_DEFAULT_MEMORY_LIMIT_MB`）。
- 动态实例并发越高，越需要额外 CPU/内存余量。

## 3. 上线前准备

### 3.1 域名与网络

- 准备域名，例如 `ctf.example.com`
- 将域名 A 记录解析到服务器公网 IP
- 放通端口：
  - `80/443`（反向代理 + HTTPS）
  - 题目实例访问端口范围（与你配置的 `INSTANCE_HOST_PORT_MIN/MAX` 一致）

### 3.2 服务器基础环境

推荐系统：`Ubuntu 22.04+` 或 `Debian 12+`

```bash
sudo apt-get update
sudo apt-get install -y ca-certificates curl gnupg git

# 安装 Docker Engine + Compose Plugin（若未安装）
curl -fsSL https://get.docker.com | sh
sudo systemctl enable --now docker
```

建议把当前用户加入 docker 组（避免长期使用 `sudo docker`）：

```bash
sudo usermod -aG docker "$USER"
```

## 4. 目录规划

建议统一部署在 `/opt/rust-ctf`：

```bash
sudo mkdir -p /opt/rust-ctf
sudo chown -R "$USER":"$USER" /opt/rust-ctf
cd /opt/rust-ctf
git clone <你的仓库地址> rust-ctf
cd rust-ctf
mkdir -p runtime
```

## 5. 生产配置文件

### 5.1 后端环境变量（`backend/.env.prod`）

```bash
cp backend/.env.example backend/.env.prod
```

至少修改以下关键项：

- `JWT_SECRET`：改为强随机值（建议 32+ 字符）
- `DEFAULT_ADMIN_USERNAME` / `DEFAULT_ADMIN_EMAIL` / `DEFAULT_ADMIN_PASSWORD`
- `DEFAULT_ADMIN_FORCE_PASSWORD_RESET=false`（按需）
- `INSTANCE_PUBLIC_HOST=ctf.example.com`
- `INSTANCE_HOST_PORT_MIN=40000`
- `INSTANCE_HOST_PORT_MAX=40100`
- `INSTANCE_HEARTBEAT_REPORT_URL=https://ctf.example.com/api/v1/instances/heartbeat/report`
- `DATABASE_URL` 与 `REDIS_URL`（与 compose 内服务保持一致）

注意：

- `backend/.env.prod` 不应提交到 Git（当前 `.gitignore` 已覆盖 `.env.*`）。
- 上线后请第一时间登录后台修改默认管理员密码。

### 5.2 Compose 变量文件（`deploy/.env.prod.stack`）

创建文件 `deploy/.env.prod.stack`：

```dotenv
POSTGRES_USER=ctf
POSTGRES_PASSWORD=replace_with_strong_db_password
POSTGRES_DB=rust_ctf
CTF_DOMAIN=ctf.example.com
```

### 5.3 生产编排文件（`deploy/docker-compose.prod.yml`）

创建 `deploy/docker-compose.prod.yml`：

```yaml
name: rust-ctf-prod

services:
  postgres:
    image: postgres:16-alpine
    container_name: rust-ctf-postgres
    restart: unless-stopped
    environment:
      POSTGRES_USER: ${POSTGRES_USER}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: ${POSTGRES_DB}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER} -d ${POSTGRES_DB}"]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - ctf_internal

  redis:
    image: redis:7-alpine
    container_name: rust-ctf-redis
    restart: unless-stopped
    command: ["redis-server", "--save", "60", "1", "--loglevel", "warning"]
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - ctf_internal

  backend:
    build:
      context: ../backend
      dockerfile: Dockerfile
    container_name: rust-ctf-backend
    restart: unless-stopped
    env_file:
      - ../backend/.env.prod
    environment:
      APP__HOST: 0.0.0.0
      APP__PORT: 8080
      DATABASE_URL: postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/${POSTGRES_DB}
      REDIS_URL: redis://redis:6379
    ports:
      - "127.0.0.1:8080:8080"
    volumes:
      - ../runtime:/runtime
      - /var/run/docker.sock:/var/run/docker.sock
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    networks:
      - ctf_internal

  frontend:
    build:
      context: ../frontend
      dockerfile: Dockerfile
    container_name: rust-ctf-frontend
    restart: unless-stopped
    environment:
      VITE_API_BASE_URL: https://${CTF_DOMAIN}/api/v1
    ports:
      - "127.0.0.1:5173:5173"
    depends_on:
      - backend
    networks:
      - ctf_internal

volumes:
  postgres_data:

networks:
  ctf_internal:
    driver: bridge
```

说明：

- `postgres` / `redis` 不对公网暴露端口。
- `backend` / `frontend` 仅绑定到本机回环地址（`127.0.0.1`），由反向代理对外发布。
- 保留 `/var/run/docker.sock` 挂载，支持题目实例生命周期管理。

## 6. 反向代理与 HTTPS（Nginx 示例）

安装 Nginx 后，新增站点配置（例如 `/etc/nginx/sites-available/rust-ctf`）：

```nginx
server {
    listen 80;
    server_name ctf.example.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name ctf.example.com;

    # 使用 certbot 或其他方式签发证书
    ssl_certificate     /etc/letsencrypt/live/ctf.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/ctf.example.com/privkey.pem;

    client_max_body_size 50m;

    location /api/ {
        proxy_pass http://127.0.0.1:8080/api/;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location / {
        proxy_pass http://127.0.0.1:5173/;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## 7. 首次启动

在仓库根目录执行：

```bash
sudo docker compose \
  --env-file deploy/.env.prod.stack \
  -f deploy/docker-compose.prod.yml \
  up -d --build
```

自检：

```bash
curl -sS http://127.0.0.1:8080/api/v1/health
sudo docker compose --env-file deploy/.env.prod.stack -f deploy/docker-compose.prod.yml ps
```

## 8. 日常运维命令

查看状态：

```bash
sudo docker compose --env-file deploy/.env.prod.stack -f deploy/docker-compose.prod.yml ps
```

查看日志：

```bash
sudo docker compose --env-file deploy/.env.prod.stack -f deploy/docker-compose.prod.yml logs -f backend
sudo docker compose --env-file deploy/.env.prod.stack -f deploy/docker-compose.prod.yml logs -f frontend
```

重启单服务：

```bash
sudo docker compose --env-file deploy/.env.prod.stack -f deploy/docker-compose.prod.yml restart backend
```

## 9. 升级与回滚

### 9.1 升级

```bash
cd /opt/rust-ctf/rust-ctf
git fetch --all
git pull --ff-only

sudo docker compose \
  --env-file deploy/.env.prod.stack \
  -f deploy/docker-compose.prod.yml \
  up -d --build
```

升级后建议执行：

- `curl http://127.0.0.1:8080/api/v1/health`
- 关键链路冒烟（登录、开题、提交通关）
- `backend/scripts/m5/security_smoke.sh`（可选）

### 9.2 回滚

```bash
cd /opt/rust-ctf/rust-ctf
git log --oneline -n 20
git checkout <上一稳定提交或标签>

sudo docker compose \
  --env-file deploy/.env.prod.stack \
  -f deploy/docker-compose.prod.yml \
  up -d --build
```

## 10. 备份与恢复

### 10.1 PostgreSQL 备份

```bash
set -a
source deploy/.env.prod.stack
set +a

mkdir -p /opt/rust-ctf/backups
ts="$(date +%Y%m%d_%H%M%S)"
sudo docker exec rust-ctf-postgres \
  pg_dump -U "$POSTGRES_USER" "$POSTGRES_DB" \
  > "/opt/rust-ctf/backups/rust_ctf_${ts}.sql"
```

### 10.2 恢复示例

```bash
set -a
source deploy/.env.prod.stack
set +a

cat /opt/rust-ctf/backups/rust_ctf_xxx.sql | \
  sudo docker exec -i rust-ctf-postgres psql -U "$POSTGRES_USER" "$POSTGRES_DB"
```

建议同时备份：

- `deploy/.env.prod.stack`
- `backend/.env.prod`（加密存储）
- `runtime/`（如需保留运行态工件）

## 11. 上线前硬化清单

- 修改所有默认密码与默认密钥（`JWT_SECRET`、管理员密码、DB 密码）
- `postgres`/`redis` 不暴露公网端口
- 强制 HTTPS（80 跳转 443）
- 限制 SSH 来源 IP，启用 fail2ban（可选）
- 明确开放的题目端口范围，最小化 `INSTANCE_HOST_PORT_MIN/MAX`
- 定期备份数据库并演练恢复

## 12. 已知限制与下一步

当前 `frontend/Dockerfile` 仍以开发服务器方式运行（`npm run dev`）。  
建议下一步补齐生产化前端镜像（构建静态资源 + Nginx/Caddy 托管），并新增：

- `frontend/Dockerfile.prod`
- `deploy/docker-compose.prod.yml`（正式入库）
- CI/CD 部署脚本（发布分支自动构建与滚动更新）
