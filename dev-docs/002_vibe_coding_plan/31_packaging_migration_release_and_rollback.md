# 31. 打包、迁移、发布与回滚

## 1. 交付物

- 控制面、scheduler、node-agent、dyun-agent/runner、Web 和各硬件 worker OCI。
- Helm chart、Compose 单机包、离线 bundle、CLI、OpenAPI/Proto/JSON Schema、SDK。
- 数据库迁移、默认策略、dashboard、告警、runbook、SBOM、provenance、签名与支持矩阵。

## 2. 兼容

仅支持相邻版本 N/N+1 滚动升级。先扩 schema/契约，再发布双读写代码，最后收缩；迁移只追加。worker/agent 握手声明 contract 与 build capability，不兼容实例拒绝接单但保持可诊断。

回滚不得依赖数据库向下迁移；应用回滚必须可读取扩展阶段 schema。对象 manifest 和事件 envelope 保留旧 reader。

## 3. 任务

- [ ] `REL-001` 建立可复现多架构构建、签名、SBOM 和 provenance。
- [ ] `REL-002` Helm/Compose 支持外部 DB/S3/OIDC、资源限制、PDB、NetworkPolicy 和升级 hooks。
- [ ] `REL-003` 从空环境及上一正式版本执行安装、升级、回滚、备份恢复。
- [ ] `REL-004` 验证 N/N+1 control-plane、worker、agent、schema 和 SDK 兼容矩阵。
- [ ] `REL-005` 汇总安全、许可证、硬件、性能、72 小时耐久和 DR 演练证据。
- [ ] `REL-006` 发布说明列出 breaking change、迁移、已知限制、支持等级和回滚触发条件。

## 4. 最终完成标准

- 视觉数据到 dyun 推理的单机与集群闭环通过。
- PostgreSQL RLS、对象权限、mTLS、RBAC 和审计通过安全测试。
- 三类硬件按支持等级提供真实证据。
- 任一控制面/worker/agent 故障都收敛到权威状态，无永久孤儿资源。
- 离线安装、备份恢复、滚动升级和应用回滚由非开发人员按 runbook 完成。

所有条件满足且风险登记无未接受的 blocker 后，方可创建正式版本。
