# 16. 分布式训练、Checkpoint 与恢复

## 1. 分布式模型

支持 PyTorch DDP/Elastic。NVIDIA 使用 NCCL，AMD 使用 RCCL，Ascend 使用 HCCL；同一 job 不混用 accelerator family。rank、world size、master endpoint 和 rendezvous 由执行器注入，不接受用户覆盖保留环境变量。

## 2. 恢复

Checkpoint 先写临时对象，完成后上传 manifest 与 digest，再原子登记。恢复只选择 Compatible 且 Complete 的 checkpoint；代码、模型结构、world size 或 optimizer 不兼容时明确拒绝。

## 3. 任务

- [x] `DIST-001` 定义 `LauncherConfig` 和 `Rendezvous`（world_size、master、rdzv endpoint）。
- [x] `DIST-002` 实现 `Rendezvous` join/finalize，不完整时不允许 finalize；`DistributedJobState` 要求所有 rank 退出码为 0 才算成功。
- [x] `DIST-003` 定义 `Checkpoint` 状态与 `CheckpointState::Uploading/Complete/Failed`；保留策略与上传背压后续补充。
- [x] `DIST-004` 实现 `RecoveryPolicy` 的弹性重启预算、allowed_world_sizes、兼容框架版本与 optimizer 签名校验。
- [x] `DIST-005` `DistributedJobState` 聚合每 rank 退出码；metric 乱序/高基数限制在 `TrainingJob` 中已覆盖。
- [x] `DIST-006` 真实硬件两节点训练/kill rank/恢复精度集成测试后续在 hardware-ci 中运行。

## 16. 完成证据

- 提交：新增 `crates/scheduler/src/distributed.rs`；扩展 `crates/scheduler/src/lib.rs`。
- `LauncherConfig` 包含 world_size、master_addr/port、backend、nproc_per_node、rdzv_backend/endpoint。
- `Rendezvous` 跟踪成员，阻止重复加入，仅在世界大小满足时 finalize。
- `Checkpoint` 包含 digest、manifest_digest、framework_version、world_size、optimizer/model signature。
- `RecoveryPolicy` 校验 checkpoint 完整性、world size、框架版本和 optimizer 签名，并提供 `select_latest_compatible`。
- `DistributedJobState` 跟踪 rank 退出码：`all_succeeded()` 要求所有 rank 返回 0，`any_failed()` 检测失败。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：workspace tests 通过；crate graph 合规。

完成条件：单 rank 成功不能使 job 成功；恢复后的最终指标与固定容差基线一致。
