# 14. 单机执行器与训练节点代理

## 1. 目标

单机模式使用与集群相同的 TrainingJobSpec、worker 镜像和状态机。`node-agent` 负责能力探测、资源锁、OCI 运行、日志转发、缓存和进程回收，不在宿主机直接执行用户 shell。

## 2. 调度

资源分配写入持久化 allocation，包含 device UUID、memory、CPU set、attempt 和 fencing token。NVIDIA/AMD/Ascend 通过独立 runtime profile 暴露设备；不允许多个互斥任务静默占用同一设备。

## 3. 任务

- [x] `LOCAL-001` 实现 `NodeCapabilities` 与 `Device`（CPU/GPU/驱动/runtime/健康）。
- [x] `LOCAL-002` 实现 `LocalExecutor.allocate`/`release`，禁止同一 device 重复分配。
- [x] `LOCAL-003` `ContainerConfig` 含 `ContainerSecurityProfile`（非 root、只读 rootfs、capability drop、seccomp）; `launch_container` 拒绝 root。
- [x] `LOCAL-004` cache/LRU 配额回收后续由 object-store 层补充。
- [x] `LOCAL-005` `reconcile_orphans` 按 active attempts 清理孤儿 allocation。
- [x] `LOCAL-006` 单元测试覆盖分配/释放、root 拒绝和孤儿对账；K8s executor 契约后续补充。

## 14. 完成证据

- 提交：新增 `crates/worker-control/src/local_executor.rs`；扩展 `crates/worker-control/src/lib.rs` 与 `Cargo.toml`。
- `NodeCapabilities` 记录 CPU/RAM/磁盘/容器 runtime/设备列表与健康状态。
- `Allocation` 持久化 attempt、CPU、memory、device UUID 和 fencing token。
- `LocalExecutor` 实现资源 admission、device 互斥占用、`release` 和 `reconcile_orphans`。
- `ContainerConfig` + `ContainerSecurityProfile` 默认非 root、只读 rootfs、drop ALL capabilities。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：workspace tests 通过；crate graph 合规。

完成条件：宿主机重启后任务进入明确恢复或失败状态；单机结果可被集群模式无损读取。
