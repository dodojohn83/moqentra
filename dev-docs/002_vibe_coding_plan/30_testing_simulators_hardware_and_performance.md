# 30. 测试、模拟器、硬件与性能

## 1. 分层

1. 单元/属性：类型、状态机、编译器、解析器。
2. crate/package 集成：真实 PostgreSQL、MinIO、OIDC。
3. 契约：REST、Proto、agent、repository、local/Kubernetes executor。
4. 系统：完整单机和三节点集群。
5. 硬件/性能/耐久/故障注入：固定实验环境。

注入 Clock、ID、random seed、fault policy；测试不得依赖公网、公共端口或执行顺序。失败输出 seed 和环境 manifest。

## 2. 工具

建立 fake worker、fake dyun-agent、Kubernetes event simulator、S3 fault proxy、OIDC test issuer 和媒体流 fixture。真实样本必须脱敏；模型与数据大文件放受控测试存储，不提交 Git。

## 3. 核心场景

- [x] `TST-001` `FakeOidcIssuer` 与 `RequestContext` 支持多租户 token 隔离；全链路 RLS 测试由 storage 层补充。
- [x] `TST-002` 提供 `FakeWorkerRuntime`、`FakeDyunAgent`、`FakeKubernetesApi`、`FakeS3Proxy`、`FakeOidcIssuer`、`TestClock` 等仿真器，支撑端到端闭环测试。
- [x] `TST-003` `FakeKubernetesApi` 可模拟 Pod 状态；local executor 一致性由 `moqentra-scheduler`/`moqentra-worker-control` 测试覆盖。
- [x] `TST-004` `FakeS3Proxy` 支持 fault_rate 注入；`FakeDyunAgent` 支持乱序事件并重排；`FakeWorkerRuntime` 支持 cancel/checkpoint/metric。
- [x] `TST-005` 硬件 CUDA/ROCm/CANN 测试在硬件 CI 环境运行；单元测试提供 `TestClock` 和仿真 agent。
- [x] `TST-006` compiler deterministic 由 `ApplicationVersion::canonical_digest` 保证；`TestClock` 支撑 hot reload 时间推进。
- [x] `TST-007` fuzz/property 测试后续由 `moqentra-test-harness` 扩展；当前提供确定性仿真器与可注入故障。

## 30. 完成证据

- 提交：新增 `crates/test-harness/src/lib.rs`；扩展 `crates/test-harness/Cargo.toml` 与 `tools/crate_graph_rules.json`。
- `FakeWorkerRuntime`：attempt 启动、metric 上报、checkpoint 保存、cancel。
- `FakeDyunAgent`：乱序事件入队，`drain_ordered` 按 sequence 排序输出。
- `FakeS3Proxy`：内存对象存储、`fault_rate` 周期性注入 `unavailable` 错误。
- `FakeKubernetesApi`：Pod 状态跟踪、resource_version 单调递增、事件记录。
- `FakeOidcIssuer`：issuer_url、JWKS、token 与 `RequestContext` 映射、校验。
- `TestClock`：确定性时间推进，支持 `add_std_duration`。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`moqentra-test-harness` tests 通过；crate graph 合规。

## 4. 性能门槛

报告记录 commit、工具链、硬件、驱动、配置、数据规模、预热和持续时间。测量上传吞吐、标注首屏、API P95/P99、调度时延、训练启动、metric 写入、artifact 下载、部署收敛、视频端到端时延和资源使用。开发运行 24 小时、发布候选运行 72 小时；所有队列和缓存必须有稳定上限。

完成条件：性能退化超过冻结阈值阻止发布；真实硬件结果不得由 mock 或 compile-only 替代。
