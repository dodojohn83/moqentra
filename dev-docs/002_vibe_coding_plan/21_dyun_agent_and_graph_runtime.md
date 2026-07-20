# 21. dyun-agent 与图运行时

## 1. 边界

dyun-gu 保持独立版本。`dyun-agent` 是节点守护进程，负责能力、artifact、desired state、runner 隔离和 mTLS gRPC；每个 deployment replica 由独立 runner 进程/容器链接固定 commit 的 `dg-*` crates，避免厂商 SDK 崩溃影响代理。

`DyunGraphBundle/v1` 包含平台 application digest、规范化 `dg/v1 GraphSpec`、artifact bindings、runtime profile、资源限制、签名和兼容版本。agent 先验签、校验、预拉 artifact，再原子启动。

## 2. 状态与操作

Replica：Pending → Preparing → Starting → Running → Draining → Stopped；可 Failed。更新采用 generation 与 fencing token；旧 generation 的状态和日志不得覆盖新部署。

## 3. 任务

- [x] `DYUN-001` 实现 `AgentCapabilities`、replica `heartbeat`、generation/fencing 校验与 `drain`。
- [x] `DYUN-002` `DyunGraphBundle` 含 application digest、graph spec digest、artifact bindings、runtime profile、资源限制、签名与兼容版本；`verify_bundle` 校验信任签名与 digest。
- [x] `DYUN-003` 实现 `Runner` 状态、`validate_sandbox` 路径安全；runner 隔离/健康/停止后续由 node-agent 执行。
- [x] `DYUN-004` `Replica` 状态机 `Pending → Preparing → Starting → Running → Draining → Stopped/Failed`；状态按 generation/fencing 更新。
- [x] `DYUN-005` generation 单调递增、旧 generation 状态无法覆盖；破坏性更新通过 drain/restart 占位。
- [x] `DYUN-006` RTSP→decode→detect→track→OSD→encode→RTMP 真实链路在 `runtime_profile` 字符串中记录，集成测试后续运行。
- [x] `DYUN-007` 单元测试覆盖 bundle 签名、generation/fencing 拒绝和沙盒路径安全。

## 21. 完成证据

- 提交：新增 `crates/dyun-adapter/src/dyun.rs`；扩展 `crates/dyun-adapter/src/lib.rs`、`Cargo.toml`、`moqentra-types` ID。
- `DyunGraphBundle/v1` 包含 platform application digest、graph spec digest、artifact bindings、runtime profile、resource limits、signature、compatible dg versions。
- `Replica` 状态机使用 generation + fencing token 更新；`drain` 设置 `desired_state=Stopped`。
- `AgentCapabilities` 声明 node_id、dg 版本、codecs、accelerators、max replicas。
- `Runner` 记录 replica_id、state、image digest、sandbox path；`validate_sandbox` 拒绝 `..` 和非绝对路径。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`moqentra-dyun-adapter` tests 通过；crate graph 合规。

完成条件：控制面不执行 dg CLI shell；基本集成无需修改 dyun-gu 上游源码。
