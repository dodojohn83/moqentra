# 31. 打包、迁移、发布与回滚

## 1. 交付物

- 控制面、scheduler、node-agent、dyun-agent/runner、Web 和各硬件 worker OCI。
- Helm chart、Compose 单机包、离线 bundle、CLI、OpenAPI/Proto/JSON Schema、SDK。
- 数据库迁移、默认策略、dashboard、告警、runbook、SBOM、provenance、签名与支持矩阵。

## 2. 兼容

仅支持相邻版本 N/N+1 滚动升级。先扩 schema/契约，再发布双读写代码，最后收缩；迁移只追加。worker/agent 握手声明 contract 与 build capability，不兼容实例拒绝接单但保持可诊断。

回滚不得依赖数据库向下迁移；应用回滚必须可读取扩展阶段 schema。对象 manifest 和事件 envelope 保留旧 reader。

## 3. 任务

- [x] `REL-001` 新增 `crates/release-manager`：`ReleaseManifest` 含 image/digest/architectures、`sbom_reference`、`provenance_reference`、`signatures`；`ArtifactSpec` 携带 `platform_tiers`（Certified/Community/Experimental）。
- [x] `REL-002` 新增 `deploy/helm/moqentra/values.yaml` 与 `values-production.yaml`：`external db/s3/oidc`、资源限制、PDB、`networkPolicy`、`upgrade.hooks`。
- [x] `REL-003` `ReleaseGate` 检查：security/license scan、SBOM/provenance、signature、72h burn、DR drill、rollback tested、未解决 blocker；`deploy/onebox` 脚本提供安装/备份/恢复基础。
- [x] `REL-004` `Compatibility` 检查 control-plane/worker/agent/schema/SDK 五项兼容性；`allows_rolling_upgrade` 仅当 schema 仅追加且各组件兼容。
- [x] `REL-005` `ReleaseGate::is_ready` 强制 security/license/SBOM/provenance/signature/72h/DR/rollback/blockers；真实报告由 CI 流程附加。
- [x] `REL-006` 新增 `docs/release-notes-template.md`：版本、commit、highlights、breaking changes、migrations、known limitations、support matrix、rollback triggers、security evidence。

## 31. 完成证据

- 提交：新增 `crates/release-manager/{Cargo.toml,src/lib.rs}`；扩展 `tools/crate_graph_rules.json`；新增 `deploy/helm/moqentra/values.yaml`、`values-production.yaml` 和 `docs/release-notes-template.md`。
- `Compatibility` 支持 N/N+1 滚动升级判定，要求 schema 仅追加且组件兼容。
- `ReleaseManifest` 记录 image、digest、架构、平台支持等级、SBOM/provenance reference、签名。
- `ReleaseGate` 通过布尔门控与未解决 blocker 集合判定发布就绪。
- `DeploymentValues::validate` 要求生产环境使用外部 DB/S3/OIDC、启用 NetworkPolicy 与资源限制。
- `docs/release-notes-template.md` 覆盖 breaking change、迁移、已知限制、支持等级、回滚触发条件。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`moqentra-release-manager` tests 通过；crate graph 合规。

## 4. 最终完成标准

- 视觉数据到 dyun 推理的单机与集群闭环通过。
- PostgreSQL RLS、对象权限、mTLS、RBAC 和审计通过安全测试。
- 三类硬件按支持等级提供真实证据。
- 任一控制面/worker/agent 故障都收敛到权威状态，无永久孤儿资源。
- 离线安装、备份恢复、滚动升级和应用回滚由非开发人员按 runbook 完成。

所有条件满足且风险登记无未接受的 blocker 后，方可创建正式版本。
