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

- [x] `DB-001` 建立迁移 CLI：新增 `moqentra-migrate` 二进制，支持 status/validate/migrate。
- [x] `DB-002` 在初始迁移中为 outbox、idempotency、processed_messages 及核心表建立索引。
- [x] `DB-003` 定义 `UnitOfWork`、`Paginated`、`Cursor` 和 `pagination_clause`。
- [x] `DB-004` 定义 `OutboxStore` 端口与内存实现；PostgreSQL `FOR UPDATE SKIP LOCKED` 实现后续补充。
- [x] `DB-005` 定义 `IdempotencyStore` 端口与内存实现；软删除/GC 策略后续补充。
- [x] `DB-006` 为内存 outbox/idempotency 建立测试；PostgreSQL 集成测试后续补充。

## 8. 完成证据

- 提交：新增 `crates/storage`（含 `outbox`、`idempotency`、`pool`、`unit_of_work` 模块）、
  `crates/storage/migrations/0001_init.sql` 与 `crates/storage/src/bin/migrate.rs`。
- `crates/storage` 依赖 `moqentra-types`、`sqlx`、`tokio`、`clap` 等。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：36 个 tests 通过；crate graph 合规。

完成条件：崩溃不会产生孤儿权威状态；重放消息不会创建第二个业务结果；每条关键 SQL 有索引与 explain 基线。
