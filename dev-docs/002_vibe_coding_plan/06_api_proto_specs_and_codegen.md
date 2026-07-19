# 06. API、Proto、Spec 与代码生成

## 1. 契约

Proto package：`moqentra.common.v1`、`worker.v1`、`dyun.v1`、`cluster.v1`。北向 API 使用 `/api/v1` 与 OpenAPI 3.1。资源 spec 使用 `apiVersion/kind/metadata/spec`，status 只由系统写入。

首版 schema：`DatasetManifest/v1`、`AnnotationProjectSpec/v1`、`TrainingJobSpec/v1`、`WorkerCapabilities/v1`、`ModelArtifactManifest/v1`、`ApplicationSpec/v1`、`DeploymentSpec/v1`、`DeploymentStatus/v1`、`DyunGraphBundle/v1`。

## 2. Envelope

命令/事件包含 message、tenant、correlation、causation、operation、occurred_at、deadline、source、fencing token、trace context。核心命令禁止使用 Proto `Any`；enum 0 值为 UNSPECIFIED；删除字段必须 reserved。

## 3. Agent 协议

worker 与 agent 主动调用 `Connect(stream AgentFrame) returns (stream ControlFrame)`。frame 覆盖 hello/capabilities、heartbeat、lease、command、ack、progress、log chunk、metric batch、result 和 drain。ack 只表示接收，不代表业务成功。

## 4. 任务

- [x] `CONTRACT-001` 编写 common、错误、分页、资源引用和 envelope。
- [x] `CONTRACT-002` 编写 worker/agent 握手、credit、lease、fencing 和恢复协议。
- [x] `CONTRACT-003` 编写首批 JSON Schema，并定义 canonical JSON 与 digest（后续任务补全映射）。
- [x] `CONTRACT-004` 配置 Buf lint/breaking、OpenAPI 骨架和确定性 prost 代码生成。
- [x] `CONTRACT-005` 编写 domain↔Proto mapper 占位，未知 enum 解码不 panic（完全映射后续补全）。
- [x] `CONTRACT-006` 增加 prost binary roundtrip 测试。

## 6. 完成证据

- 提交：新增 `proto/` 目录（common、worker、dyun、cluster）与 `proto/buf.yaml`、
  `proto/buf.gen.yaml`；新增 `proto/specs/v1/` JSON Schema；新增
  `docs/openapi/openapi.yaml`；更新 `crates/contracts/Cargo.toml`、`build.rs`、
  `src/lib.rs` 以使用 `prost-build` + `protoc-bin-vendored` 生成 Rust stub。
- 测试命令：
  - `buf lint proto`
  - `buf format -w proto`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`buf lint proto` 通过；`cargo` 全绿；24 个 tests 通过；
  crate graph 合规。

完成条件：Rust、Python、TypeScript client 可完成契约 roundtrip；连续生成无 diff。
