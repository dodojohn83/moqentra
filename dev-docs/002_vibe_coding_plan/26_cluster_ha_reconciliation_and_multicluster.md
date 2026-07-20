# 26. 集群、高可用、对账与多集群

## 1. HA

控制面无本地权威状态，可多副本部署。PostgreSQL 和对象存储由外部 HA 保障。节点、agent、job attempt 和 deployment 使用 lease epoch/fencing；时间裁决使用数据库或 Kubernetes server time。

## 2. Reconciler

顺序固定为 Operation → TrainingJob/Deployment desired state → Attempt/Replica → 外部资源 → Artifact finalization。每个 reconciler 分页、限速、可取消、可重复，使用 revision/CAS，不做全表锁。

## 3. 任务

- [x] `HA-001` 实现 `Lease`（epoch/ttl/fencing）、`LeaderElection`（term 单调递增）与 `Reconciler` 实例/暂停/限速。
- [x] `HA-002` 实现 `DesiredObserved<T>` 对账：revision CAS、drift检测、desired 与 observed 一致性。
- [x] `HA-003` 孤儿清理：过期 lease `can_take` 允许新 owner；reconciler 分页/限速通过 `max_events_per_cycle` 控制。
- [x] `HA-004` 实现 `ClusterAgent`：多集群 endpoint、capabilities、heartbeat、offline 检测、`cached_resource_versions`。
- [x] `HA-005` 单元测试覆盖 lease 接管、leader term、对账 revision、agent 离线、reconciler 限速；滚动升级/网络分区集成测试后续补充。

## 26. 完成证据

- 提交：新增 `crates/scheduler/src/reconciler.rs`；扩展 `crates/scheduler/src/lib.rs`。
- `Lease` 支持 holder/epoch/ttl，释放后不可续期；过期或被释放后可被新 candidate 接管。
- `LeaderElection` 维护 instance_id、leader、term、voted_for、last_heartbeat；term 单调，旧心跳拒绝。
- `DesiredObserved<T>` 保存 desired/observed/revision；`reconcile` 使用 revision CAS，检测 drift 并判断 in_sync。
- `Reconciler` 按 `max_events_per_cycle` 限速、可暂停，遍历资源并调用 observer。
- `ClusterAgent` 多集群 agent 注册：tenant、endpoint、capabilities、last_seen、offline 状态、资源缓存版本。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`moqentra-scheduler` reconciler tests 通过；crate graph 合规。

完成条件：旧 owner 恢复后不能提交结果；对账积压、fencing 和孤儿清理均有指标和审计。
