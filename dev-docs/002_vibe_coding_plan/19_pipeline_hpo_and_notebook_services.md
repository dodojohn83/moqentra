# 19. Pipeline、HPO 与 Notebook

## 1. Pipeline

PipelineSpec 是有向无环任务图，节点引用已注册 task template，边传递 artifact reference 而非本地路径。运行快照固定 spec、参数、镜像和输入；节点级缓存键必须内容寻址。

## 2. HPO 与 Notebook

HPO controller 只生成普通 TrainingJob，搜索算法不得绕过配额。Notebook 是有期限、受网络策略和资源配额约束的开发环境；凭据为短期 token，不挂载控制面服务账号。

## 3. 任务

- [x] `PIPE-001` 实现 `PipelineSpec`（DAG 节点、依赖、参数、缓存键输入）与 `validate_dag` 环/缺失依赖检测；取消传播已实现。
- [x] `PIPE-002` 实现 `PipelineRun` 与 `NodeRunState`；`update_node` 检查依赖前序为 Succeeded/Cached；`recompute_state` 汇总完成状态。
- [x] `HPO-001` 实现 `HpoRun`、search space `SearchParam`、trial budget、`suggest_trial/report_trial`、best trial 跟踪与 `should_stop_early` 启发式。
- [x] `NOTE-001` 定义 `Notebook` 含 image digest、resource profile、expires_at、idle timeout；安全校验由 `validate_security` 处理。
- [x] `NOTE-002` `validate_security` 拒绝特权、hostPath、空镜像；egress 列表与资源上限由策略后续执行。
- [x] `PIPE-003` 单元测试覆盖 DAG 环、依赖顺序、HPO best trial、Notebook 安全拒绝；缓存污染/配额耗尽集成测试后续补充。

## 19. 完成证据

- 提交：新增 `crates/domain/src/pipeline.rs`；扩展 `moqentra-types` ID 与 `crates/domain/src/lib.rs`。
- `PipelineSpec` 与 `PipelineNode` 支持 DAG、参数、依赖与缓存键输入；`validate_dag` 检测环与缺失节点。
- `PipelineRun` 在 `start` 时初始化所有节点为 Pending；`update_node` 强制依赖节点为 Succeeded/Cached。
- `HpoRun` 支持 trial budget、并行上限、best trial 与 early stop；每个 trial 记录 `TrainingJobId`。
- `Notebook` 包含 image digest、resource profile、expires_at、idle timeout、`allowed_egress` 与安全开关；`validate_security` 拒绝特权与 hostPath。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：workspace tests 通过；crate graph 合规。

完成条件：高级服务复用训练、artifact、审计和租户模型，不形成第二套调度系统。
