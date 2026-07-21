# R1 固定集成环境清单

本清单定义 `R1-GOV-004` 要求的集成测试与验收环境。所有版本号、commit 和配置必须与本文件一致，方可作为 R1 证据。

| 组件 | 版本/来源 | 用途 | 验证命令 | 备注 |
|---|---|---|---|---|
| Rust toolchain | `1.94.1` | 控制面、scheduler、agent | `rustup show` | `rust-toolchain.toml` |
| Node.js | `22.14.0` | Web 控制台构建 | `node --version` | CI 缓存 pnpm |
| pnpm | `10.8.1` | 包管理 | `pnpm --version` | `package.json#packageManager` |
| Python | `>=3.13` | Worker SDK | `python --version` | `pyproject.toml` |
| PostgreSQL | `16` | 元数据与状态权威来源 | `psql --version` / readiness | Onebox Compose |
| MinIO | `RELEASE.2025-04-22T22-14-18Z` 或 `latest` pinned by digest | 大文件对象存储 | `mc --version` / bucket policy | Onebox Compose；签名 URL TTL ≤15min |
| Dex | `dexidp/dex:v2.42.0` | OIDC issuer | `dex version` | Onebox Compose；静态 clients 由 `init.sh` 生成 |
| Docker / Podman | Docker 25+ 或 Podman 5+ | OCI 运行 | `docker version` / `podman version` | Node Agent 探测后选择 |
| k3s | `v1.33.x` | 单节点 Kubernetes 验证 | `kubectl version` | 可选；preflight 检查 kubeconfig |
| Volcano | `v1.11.0` | gang 调度 | `kubectl get pods -n volcano-system` | 需要时手动安装 |
| NVIDIA Container Toolkit | `1.17.0` | GPU OCI 运行 | `nvidia-ctk --version` | 与 driver 兼容 |
| NVIDIA device plugin | `v0.17.0` | k3s/Kubernetes GPU 暴露 | `kubectl describe node` | Volcano/K8s 训练 |
| RTX 3090 | Driver 570+ / CUDA 12.8+ / PyTorch 2.7 | 真实训练证据 | `nvidia-smi`, `torch.__version__` | 仅标记为 `preview` |
| PyTorch | `2.7` | 训练模板 | `python -c "import torch; print(torch.__version__)"` | Worker 镜像固定 digest |
| dyun-gu | pinned commit (TBD in `R1-DYUN-001`) | 推理 runner | `dg --version` 或构建产物 | 无 tag 时固定 commit |

## 一次性配置

- `MOQENTRA_DATABASE_URL`: PostgreSQL DSN，含 `application_name=moqentra-control-plane`。
- `MOQENTRA_OBJECT_STORE_ENDPOINT` + `MOQENTRA_OBJECT_STORE_ACCESS_KEY/SECRET_KEY`：MinIO。
- `MOQENTRA_OIDC_ISSUER_URL`：Dex 外部 URL。
- `MOQENTRA_NODE_AGENT_WORKSPACE_ROOT`：可写目录，推荐 `/var/lib/moqentra/node-agent`。
- `MOQENTRA_DYUN_AGENT_WORKSPACE_ROOT`：可写目录，推荐 `/var/lib/moqentra/dyun-agent`。
- `MOQENTRA_CONTROL_PLANE_URL`、`MOQENTRA_NODE_AGENT_URL`、`MOQENTRA_SCHEDULER_ADDR`、`MOQENTRA_NODE_AGENT_ADDR`、`MOQENTRA_DYUN_AGENT_ADDR`：服务发现地址。

## 说明

- 浮动镜像标签（`latest`、`stable`）不得出现在 R1 证据环境中；所有镜像使用 digest。
- 环境首次建立后需运行 `tools/benchmarks/run-hardware-test.sh nvidia` 并把摘要写入 `artifacts/r1-evidence/<build-id>/`。
- 本文件随 003 计划任务更新，变更必须绑定到 `R1-GOV-004` 或相关任务 PR。
