# 08. PostgreSQL、Outbox 与幂等

## 1. 表组

身份：`tenants/projects/project_members/service_accounts`。数据：`datasets/dataset_versions/assets/version_assets/import_jobs`。标注：`annotation_projects/tasks/assignments/annotations/reviews`。训练：`experiments/training_jobs/job_attempts/job_ranks/checkpoints/metric_series`。模型：`models/model_versions/artifacts/conversions/evaluations`。应用：`applications/application_versions/deployments/deployment_replicas`。平台：`nodes/node_leases/resource_allocations/operations/outbox_events/processed_messages/idempotency_keys/audit_logs`。

核心查询字段不得藏在 JSON；扩展 JSON 必须带 schema version。迁移只追加，已发布迁移禁止修改。

## 2. 事务不变量

- 聚合 revision 更新和 outbox 写入同事务。
- 数据集版本发布后不可修改成员。
- 同一幂等 scope 只能创建一个 Operation。
- 任务 attempt 使用 fencing token，旧 attempt 不能覆盖新结果。
- artifact 记录只有在对象 digest 校验成功后才能 Available。

## 3. 任务

- [ ] `DB-001` 建立迁移 CLI：status、validate、migrate；生产启动默认不自动迁移。
- [ ] `DB-002` 为租户、状态、deadline、未发布 outbox、lease 和 digest 建立索引。
- [ ] `DB-003` 实现 UnitOfWork、repository 和游标分页。
- [ ] `DB-004` outbox 使用 `FOR UPDATE SKIP LOCKED`，投递至少一次，消费者幂等。
- [ ] `DB-005` 实现分批保留、软删除、legal hold 和对象 GC 候选。
- [ ] `DB-006` 建立真实 PostgreSQL contract tests、并发冲突和迁移升级测试。

完成条件：崩溃不会产生孤儿权威状态；重放消息不会创建第二个业务结果；每条关键 SQL 有索引与 explain 基线。
