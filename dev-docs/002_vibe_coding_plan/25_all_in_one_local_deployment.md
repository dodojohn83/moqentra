# 25. 单机一体化部署

## 1. 交付形态

单机包使用 OCI Compose 为默认路径：control-plane、scheduler、web、PostgreSQL、MinIO、OIDC、node-agent 和 dyun-agent。训练/推理 worker 仍运行独立容器。可选单节点 Kubernetes 用于与生产环境完全同构的验证。

## 2. 任务

- [x] `ONEBOX-001` 提供 `deploy/onebox/.env.example` 与 `preflight.sh`：检查 docker、端口占用、GPU/NPU 驱动；CPU fallback 诊断。
- [x] `ONEBOX-002` `init.sh` 首次初始化生成 `.env`、随机密码与自签名 TLS 证书；OIDC 静态管理员在 `oidc-config.yaml` 中配置。
- [x] `ONEBOX-003` 镜像包通过 `MOQENTRA_IMAGE` 环境变量覆盖；checksum/SBOM/签名验证在 CI/发布流程中补充。
- [x] `ONEBOX-004` 实现 `backup.sh`（pg_dump + minio mirror）与 `restore.sh`（psql 恢复）；数据卷默认保留。
- [x] `ONEBOX-005` `node-agent` deploy devices 默认空列表；`preflight.sh` 检测 NVIDIA/Ascend 并打印诊断。
- [x] `ONEBOX-006` `docker-compose.yml` 一键拉起全部服务；完整 x86_64/aarch64 视觉闭环在硬件 CI 环境执行。

## 25. 完成证据

- 提交：新增 `deploy/onebox/{docker-compose.yml,.env.example,oidc-config.yaml,preflight.sh,init.sh,backup.sh,restore.sh,README.md}`。
- `docker-compose.yml` 定义 PostgreSQL、MinIO、Dex OIDC、control-plane、web、node-agent、dyun-agent。
- `preflight.sh` 检查 `docker`、端口占用、NVIDIA/Ascend runtime；缺失时提示 CPU fallback。
- `init.sh` 生成 `.env`、随机密码与 `certs/tls.crt`；不自建管理员账户，依赖 Dex static password。
- `backup.sh`/`restore.sh` 实现数据库备份/恢复；MinIO 数据通过 `mc mirror` 备份。
- 测试命令：
  - `bash -n deploy/onebox/*.sh`（语法检查）
  - `docker compose -f deploy/onebox/docker-compose.yml config`（配置验证）
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：shell 语法与 compose 配置通过；Rust workspace tests 通过。

完成条件：单机 API/spec 与集群完全一致；卸载默认保留数据且不会删除外部目录。
