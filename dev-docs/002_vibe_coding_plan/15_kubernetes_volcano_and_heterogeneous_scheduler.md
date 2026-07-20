# 15. Kubernetes、Volcano 与异构调度

## 1. 编译目标

TrainingJobSpec 编译为不可变执行计划，再由 adapter 生成 Job/VolcanoJob、ConfigMap、Secret reference、PVC/ephemeral volume 和 NetworkPolicy。生成必须确定性，所有对象带 tenant/project/job/attempt 标签但 Prometheus 不使用高基数 ID。

## 2. 硬件矩阵

- NVIDIA：官方 device plugin/GPU Operator，CUDA、NCCL；共享场景由 HAMi 明确启用。
- AMD：AMD GPU Operator/device plugin，ROCm、RCCL。
- Ascend：Ascend device plugin/Operator，CANN、HCCL。

三类 worker 镜像、node pool、runtime class 和调度标签分离。缺少匹配节点时保持 Pending 并输出可诊断原因，不降级到 CPU。

## 3. 任务

- [x] `SCHED-001` 实现 `SchedulingQueue`，支持优先级排序、队列容量与 project quota。
- [x] `SCHED-002` 实现 `GangGroup` 与 `ExecutionPlan`，支持 DDP world_size 作为 minAvailable/totalMembers。
- [x] `SCHED-003` 实现 `AcceleratorCapability`、`NodeCapacity` 与 taint-aware `find_placement`。
- [x] `SCHED-004` `PlanCompiler.compile` 验证 image/code digest、replicas > 0；`ContainerSecurityProfile` 在 local executor 实现。
- [x] `SCHED-005` 定义 `WatchEvent` 含 `resource_version` 与 `revision` 供 watcher 幂等恢复。
- [x] `SCHED-006` 单元测试覆盖 quota、placement taints、zero replicas；抢占/drain/API server 中断集成测试后续补充。

## 15. 完成证据

- 提交：新增 `crates/scheduler/src/scheduler.rs`；扩展 `crates/scheduler/src/lib.rs` 与 `Cargo.toml`。
- `SchedulingQueue` 支持 `enqueue` 容量/项目配额、`pop` 按优先级和提交时间排序。
- `ExecutionPlan` 包含 tenant/project/job/attempt labels、replicas、资源请求、volumes、network policy 和 `GangGroup`。
- `ClusterTopology.find_placement` 按 CPU、内存、accelerator 类型与 taint 匹配节点，缺少设备时明确失败且不降级 CPU。
- `WatchEvent` 携带 `resource_version` 与单调 `revision`，支持幂等恢复。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`moqentra-scheduler` 与 workspace tests 通过；crate graph 合规。

完成条件：相同 spec 产生相同计划；调度失败能定位到配额、能力、拓扑或策略。
