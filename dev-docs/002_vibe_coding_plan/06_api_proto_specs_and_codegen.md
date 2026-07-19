# 06. API、Proto、Spec 与代码生成

## 1. 契约

Proto package：`moqentra.common.v1`、`worker.v1`、`dyun.v1`、`cluster.v1`。北向 API 使用 `/api/v1` 与 OpenAPI 3.1。资源 spec 使用 `apiVersion/kind/metadata/spec`，status 只由系统写入。

首版 schema：`DatasetManifest/v1`、`AnnotationProjectSpec/v1`、`TrainingJobSpec/v1`、`WorkerCapabilities/v1`、`ModelArtifactManifest/v1`、`ApplicationSpec/v1`、`DeploymentSpec/v1`、`DeploymentStatus/v1`、`DyunGraphBundle/v1`。

## 2. Envelope

命令/事件包含 message、tenant、correlation、causation、operation、occurred_at、deadline、source、fencing token、trace context。核心命令禁止使用 Proto `Any`；enum 0 值为 UNSPECIFIED；删除字段必须 reserved。

## 3. Agent 协议

worker 与 agent 主动调用 `Connect(stream AgentFrame) returns (stream ControlFrame)`。frame 覆盖 hello/capabilities、heartbeat、lease、command、ack、progress、log chunk、metric batch、result 和 drain。ack 只表示接收，不代表业务成功。

## 4. 任务

- [ ] `CONTRACT-001` 编写 common、错误、分页、资源引用和 envelope。
- [ ] `CONTRACT-002` 编写 worker/agent 握手、credit、lease、fencing 和恢复协议。
- [ ] `CONTRACT-003` 编写所有 JSON Schema，并定义 canonical JSON 与 digest。
- [ ] `CONTRACT-004` 配置 Buf lint/breaking、OpenAPI breaking 和确定性代码生成。
- [ ] `CONTRACT-005` 编写 domain↔Proto/HTTP/spec mapper，未知 enum 不得 panic。
- [ ] `CONTRACT-006` 增加 golden binary/JSON 和旧读新写兼容测试。

完成条件：Rust、Python、TypeScript client 可完成契约 roundtrip；连续生成无 diff。
