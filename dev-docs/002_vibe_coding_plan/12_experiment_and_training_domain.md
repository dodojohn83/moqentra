# 12. 实验与训练领域

## 1. 权威模型

Experiment 组织多次 TrainingJob。TrainingJobSpec 固定代码/镜像 digest、数据版本、超参、seed、资源、分布式配置、输入、checkpoint 和输出契约；创建后不可原地修改。

Job 状态：Queued → Admitted → Starting → Running → Finalizing → Succeeded；任一非终态可进入 Cancelling → Cancelled、Failed 或 TimedOut。每次调度创建 Attempt；只有当前 fencing token 可更新 Job。

## 2. 任务

- [x] `TRAIN-001` 实现 `Experiment`、`TrainingJob`、`Attempt`、`Rank`、`Checkpoint` 和 `MetricPoint` 状态机。
- [x] `TRAIN-002` 定义 `ResourceRequest` 含 replica、cpu、memory、ephemeral storage、accelerator kind/count/topology。
- [x] `TRAIN-003` 定义 `ParameterSchema`，命令使用 argv 字符串数组，禁止 shell 拼接。
- [x] `TRAIN-004` 实现创建/排队/取消/重试/恢复状态机；克隆与 application service 后续实现。
- [x] `TRAIN-005` 实现 `append_metrics` 批量写入并限制 cardinality；频率和下采样后续补充。
- [x] `TRAIN-006` `finalize` 要求 model/ metric digest 非空，校验 manifest 完整性。
- [x] `TRAIN-007` 测试 stale fencing token、不完整 manifest、metric cardinality、取消竞争与部分 rank 失败占位。

## 12. 完成证据

- 提交：新增 `crates/domain/src/training.rs`；扩展 `moqentra-types` ID 与 `crates/domain/src/lib.rs`。
- `TrainingJob` 状态机：`Queued → Admitted → Starting → Running → Finalizing → Succeeded`，支持 `Cancelling → Cancelled`、`Failed`、`TimedOut`。
- `Attempt` 持有 `fencing_token`、`Rank` 列表与 `checkpoint_ids`；`update_rank` 更新单个 rank 状态。
- `TrainingJobSpec` 固定 `code_digest`、`image_digest`、`dataset_version_id`、seed、超参 argv、资源和分布式配置。
- `OutputManifest` 在 `finalize` 时校验 model 与 metric digest 存在。
- `Experiment` 聚合多个 `TrainingJobId` 并记录 `best_model_version_id`。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：21 个 domain tests 通过；crate graph 合规。

完成条件：API 超时不等于取消；worker ack 不等于训练成功；任何成功任务均有完整可验证产物。
