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

- [ ] `FOUND-001` 实现 newtype 的 parse/display/serde/prost/sqlx mapper 与边界测试。
- [ ] `FOUND-002` 实现错误到 HTTP Problem Details、gRPC Status 和事件状态的独立 mapper。
- [ ] `FOUND-003` 注入 Clock、IdGenerator、Cancellation 和 SecretProvider。
- [ ] `FOUND-004` 生成配置 schema、可解析示例和 redacted debug。
- [ ] `FOUND-005` 测试分页上限、时间回拨、deadline 溢出、未知字段和 secret 泄漏。

完成条件：公共类型不可混用；调用方不以字符串判断错误；配置错误定位到字段。
