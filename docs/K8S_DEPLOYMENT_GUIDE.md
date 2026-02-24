# Rust-CTF Kubernetes 部署指南

最后更新：2026-02-24

## 1. 说明

仓库已提供可直接部署的 Kubernetes 清单，位置：

- `deploy/k8s/`

该方案覆盖：

- PostgreSQL（StatefulSet + PVC）
- Redis（StatefulSet + PVC）
- Rust Backend（Deployment + PVC）
- Vue Frontend（Deployment）
- Ingress（`/api` 转发到 backend，`/` 转发到 frontend）

## 2. 前置条件

- 可用的 Kubernetes 集群（1.24+）
- 默认存储类（支持动态创建 PVC）
- 已安装 Ingress Controller（示例按 NGINX Ingress 编写）
- `kubectl` 可访问目标集群

## 3. 构建并推送镜像

在仓库根目录执行：

```bash
docker build -t <registry>/rust-ctf-backend:<tag> backend
docker build -t <registry>/rust-ctf-frontend:<tag> frontend
docker push <registry>/rust-ctf-backend:<tag>
docker push <registry>/rust-ctf-frontend:<tag>
```

然后修改：

- `deploy/k8s/backend.yaml` 的 `image`
- `deploy/k8s/frontend.yaml` 的 `image`

## 4. 修改部署参数

部署前至少替换以下占位值：

1. `deploy/k8s/secret.yaml`
   - `postgres-password`
   - `database-url`
   - `jwt-secret`（至少 32 字符）
   - `default-admin-password`
2. `deploy/k8s/configmap-backend.yaml`
   - `INSTANCE_PUBLIC_HOST`
   - `AUTH_EMAIL_BASE_URL`
   - `INSTANCE_HEARTBEAT_REPORT_URL`
3. `deploy/k8s/configmap-frontend.yaml`
   - `VITE_API_BASE_URL`
   - `VITE_ALLOWED_HOSTS`
4. `deploy/k8s/ingress.yaml`
   - `host`
   - `tls.secretName`

## 5. 部署命令

普通部署（推荐起点）：

```bash
kubectl apply -k deploy/k8s
```

该模式默认关闭后端实例回收扫描（`RUNTIME_ALERT_SCAN_ENABLED=false`、`INSTANCE_REAPER_ENABLED=false`），可在没有 Docker Socket 的集群先稳定运行。

## 6. 可选：启用 docker.sock（动态实例兼容模式）

如果你的节点可安全提供 `/var/run/docker.sock`，可以对 backend 执行补丁：

```bash
kubectl -n rust-ctf patch deployment backend --type strategic --patch-file deploy/k8s/backend-docker-sock-patch.yaml
```

该补丁会为 backend 注入：

- `/var/run/docker.sock` hostPath 挂载
- `RUNTIME_ALERT_SCAN_ENABLED=true`
- `INSTANCE_REAPER_ENABLED=true`

注意：挂载 `docker.sock` 等同于高权限宿主机访问能力，只建议在受控环境使用。

## 7. 验证

```bash
kubectl -n rust-ctf get pods
kubectl -n rust-ctf get svc
kubectl -n rust-ctf get ingress
kubectl -n rust-ctf logs deploy/backend --tail=200
```

健康检查：

- `GET https://<your-domain>/api/v1/health`

## 8. 升级与回滚

更新镜像后：

```bash
kubectl -n rust-ctf rollout restart deploy/backend deploy/frontend
kubectl -n rust-ctf rollout status deploy/backend
kubectl -n rust-ctf rollout status deploy/frontend
```

回滚：

```bash
kubectl -n rust-ctf rollout undo deploy/backend
kubectl -n rust-ctf rollout undo deploy/frontend
```

## 9. 已知限制

- 当前 frontend 镜像运行的是 Vite dev server，适合教学/中小规模场景；如要更高并发，建议后续改为静态构建 + Nginx。
- 动态实例模块目前依赖 Docker Compose 语义；在纯容器运行时（如 containerd 且无 docker.sock）环境下，建议先使用普通部署模式。
