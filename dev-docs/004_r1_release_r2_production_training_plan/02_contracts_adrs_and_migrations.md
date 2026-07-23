# 02. R2 契约、ADR 与数据库迁移

## 1. 必需 ADR

- [ ] `R2-ADR-001` 记录 `TrainingJobSpec/v1` 的 replica、process 和 world size 语义以及 R1 兼容 mapper。
- [ ] `R2-ADR-002` 记录 Volcano gang、c10d rendezvous、整 gang 重启和 fencing 设计。
- [ ] `R2-ADR-003` 记录 distributed checkpoint 两阶段完成协议、兼容性和 GC。
- [ ] `R2-ADR-004` 记录 quota reservation、usage ledger、approval 和公平队列权威边界。
- [ ] `R2-ADR-005` 记录 scheduler leader election、Agent session/command HA 和外部系统幂等。
- [ ] `R2-ADR-006` 记录转换格式的 supported/preview/compile-only 判定和分层发布门禁。
- [ ] `R2-ADR-007` 记录 Kubernetes production profile 的 99.9%、RPO/RTO、审计保留和 Onebox 非 HA 边界。

## 2. 公共契约

- [ ] `R2-CONTRACT-001` 向 `TrainingJobSpec/v1` 添加 distributed、processesPerReplica、checkpointPolicy、queueRef、priorityClassRef、preemptionPolicy 和 resourceClassRef。
- [ ] `R2-CONTRACT-002` 定义 `resources.replicas` 为 Worker Pod/节点数；`worldSize = replicas × processesPerReplica`，两个值都必须大于零且受上限约束。
- [ ] `R2-CONTRACT-003` R1 spec 缺少新字段时 canonicalize 为单 replica、单 process、无抢占；原始 spec 与 canonical snapshot 都进入血缘。
- [ ] `R2-CONTRACT-004` 新增 `QuotaPolicy/v1`、`QuotaUsage/v1`、`ApprovalRequest/v1`、`QueuePolicy/v1` 和 `ResourceClass/v1` JSON Schema/API types。
- [ ] `R2-CONTRACT-005` 新增 `CheckpointManifest/v1`，包含 attempt、step、world size、shards、训练状态、兼容签名、摘要和 complete marker。
- [ ] `R2-CONTRACT-006` 新增 `ConversionProfile/v1`，包含 converter image、source/target format、runtime/hardware、参数 schema 和支持等级。
- [ ] `R2-CONTRACT-007` 向 Worker/Cluster Proto 追加 rank、rendezvous、checkpoint、session resume 和 command sequence；不得复用或改变已有 field number。
- [ ] `R2-CONTRACT-008` 生成 Rust、TypeScript 和 Python client；使用 `v0.1.0` tag 执行 Buf/OpenAPI/JSON Schema breaking tests。

## 3. 数据库迁移

- [ ] `R2-DB-001` 从 `0018` 起增加 quota_policies、quota_reservations、usage_ledger 和 usage_rollups，并建立 tenant/project/time 索引与 RLS。
- [ ] `R2-DB-002` 增加 approval_requests、approval_decisions 和 policy_snapshot；申请与决定不可原地覆盖。
- [ ] `R2-DB-003` 增加 queues、priority_classes、resource_classes、cluster_resources 和 workload_bindings。
- [ ] `R2-DB-004` 增加 ranks、rendezvous_members、checkpoints、checkpoint_shards 和 checkpoint_holds。
- [ ] `R2-DB-005` 增加 agent_sessions、agent_commands、command_acks、scheduler_leases 和 reconciler_cursors。
- [ ] `R2-DB-006` 增加 conversion_profiles、conversion_reports 和 model_promotion_requests。
- [ ] `R2-DB-007` 审计表按时间分区并支持归档 manifest；普通租户角色不能 update/delete audit partition。
- [ ] `R2-DB-008` 提供 expand-first 迁移、双读写验证、N/N+1 compatibility 和旧代码读取扩展 schema 的回滚测试。

## 4. API 约定

北向 API 增加 `/v1/quotas`、`/v1/quota-usage`、`/v1/approval-requests`、`/v1/queues`、`/v1/priority-classes`、`/v1/resource-classes` 和 `/v1/conversion-profiles`。

训练增加 `:resume`、`:retry` 和有权限的 `:preempt` 操作；所有长操作返回 `202 Operation`。更新策略和审批决定要求 Idempotency-Key、If-Match 和结构化审计。

## 5. 完成条件

- R1 client/Worker/Agent 可连接 R2 控制面并执行 R1 单机任务。
- R2 特性只分发给声明相应 contract/capability 的实例；不兼容实例保持可诊断但不得接单。
- 所有新表、API 和后台单元可追踪到 R2 acceptance scenario。
