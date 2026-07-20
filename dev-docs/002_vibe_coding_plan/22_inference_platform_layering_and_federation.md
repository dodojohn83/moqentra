# 22. 推理平台分层与联动

## 1. 分层

训练开放平台权威管理模型和应用版本；推理平台权威管理 runtime cluster、endpoint、deployment、replica、流 binding 和实时 SLO。两者通过版本化发布契约联动，禁止共享内部数据库表。

发布流程：Promotion → ReleaseBundle → inference admission → staged rollout → observation → complete/rollback。ReleaseBundle 固定模型、ApplicationVersion、dyun bundle、策略和签名。

## 2. 任务

- [x] `INFER-001` 实现 `ReleaseBundle`（固定 model、application version、dyun bundle digest、rollout policy、signature）与 `ReplicaObservedState` 回传。
- [x] `INFER-002` 实现 `Cluster`（zone/region/capabilities/cached models）、`Endpoint`、`Deployment`。
- [x] `INFER-003` `RolloutStrategy` 支持 `RollingUpdate`/`BlueGreen`/`Canary`/`Paused`；`Deployment` 实现 `pause`/`resume`/`rollback`。
- [x] `INFER-004` `Deployment.observe_replica` 按 generation 幂等接收；旧 generation 忽略；`available_ratio` 与 `should_rollback` 基于 observed state。
- [x] `INFER-005` 实现 `PlacementPolicy.score`：region 匹配、data locality、required capabilities、affinity labels。
- [x] `INFER-006` `ReleaseBundle` 已固定所有输入；`Cluster.cached_models` 支持训练平台离线时的模型缓存契约。
- [x] `INFER-007` 单元测试覆盖 rollout 生命周期、rollback 策略、旧 generation 幂等、SLO 触发回滚、placement 评分。

## 22. 完成证据

- 提交：新增 `crates/domain/src/inference.rs`；扩展 `moqentra-types` ID 与 `crates/domain/src/lib.rs`。
- `ReleaseBundle` 将训练产物与推理 runtime 解耦，含 `RolloutPolicy` 和签名。
- `Deployment` 状态机：`Pending → Rolling/Stable → Paused/RollingBack/Failed`。
- `RolloutStrategy` 支持 `RollingUpdate`、`BlueGreen`、`Canary { percent }`、`Paused`。
- `observe_replica` 使用 generation 单调性实现幂等和旧状态忽略。
- `PlacementPolicy` 综合 region、data locality、capabilities 和 affinity 评分。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：workspace tests 通过；crate graph 合规。

完成条件：两个平台可独立升级；推理持续运行不依赖训练控制面在线。
