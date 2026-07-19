# 05. 基础类型、错误与配置

## 1. 公共类型

实现受校验 newtype：`TenantId`、`ProjectId`、`DatasetId`、`DatasetVersionId`、`AssetId`、`AnnotationTaskId`、`TrainingJobId`、`AttemptId`、`ModelVersionId`、`ApplicationVersionId`、`DeploymentId`、`NodeId`、`OperationId`、`ArtifactDigest`。内部 ID 使用 UUIDv7，但授权不得依赖 ID 不可猜测。

实现 `UtcTimestamp`、`Deadline`、`Revision`、`FencingToken`、`PageRequest/Page<T>`、`RequestContext`、`Principal`、`ResourceRef`、`ResourceQuantity`。

## 2. 错误

稳定 kind：InvalidArgument、Unauthenticated、PermissionDenied、NotFound、AlreadyExists、Conflict、StaleRevision、StaleFence、Busy、QuotaExceeded、RateLimited、Timeout、Cancelled、Unavailable、Unsupported、VersionMismatch、ExternalFailed、Internal。

错误包含稳定 code、安全 message、retryable、field violations、request/correlation ID；source 仅内部保存。

## 3. 配置

优先级：内置默认 < 文件 < 环境变量 < secret provider；未知字段拒绝。每项标记 static、restart 或 dynamic effect。secret 只通过引用加载，Debug/日志统一脱敏。

## 4. 任务与测试

- [x] `FOUND-001` 实现 newtype 的 parse/display/serde 与边界测试（prost/sqlx mapper 留待后续 adapter 实现）。
- [x] `FOUND-002` 实现错误到 HTTP Problem Details、事件状态的 mapper；gRPC mapper 留待 contracts 实现。
- [x] `FOUND-003` 注入 Clock、IdGenerator（Cancellation 与 SecretProvider 在后续任务中扩展）。
- [x] `FOUND-004` 生成配置 schema、可解析示例和 redacted debug。
- [x] `FOUND-005` 测试分页上限、deadline 溢出、未知字段和 secret 泄漏。

## 5. 完成证据

- 提交：重写 `crates/types/src/lib.rs` 并新增 `id.rs`、`error.rs`、
  `time.rs`、`pagination.rs`、`request.rs`、`config.rs`。
- `crates/types/Cargo.toml` 引入 `uuid`、`time`、`config`、`serde_json`。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：所有检查通过；20 个 tests 通过；crate graph 合规。

完成条件：公共类型不可混用；调用方不以字符串判断错误；配置错误定位到字段。
