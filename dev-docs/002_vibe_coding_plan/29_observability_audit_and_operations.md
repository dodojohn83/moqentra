# 29. 可观测性、审计与运维

## 1. 遥测

统一 OpenTelemetry。日志字段包括 tenant/project/request/operation/job/attempt/deployment/node；secret、签名 URL、原始图片帧和训练参数中的敏感值禁止输出。高基数业务 ID 不作为 Prometheus label。

指标至少覆盖 API、DB/S3、outbox、队列/配额、训练状态和时延、worker/agent lease、checkpoint、artifact、部署 replica、dyun element 队列/拷贝、reconciler backlog。

## 2. 任务

- [x] `OBS-001` 实现 `TraceContext`（trace/span/sampled、traceparent header）与 `StructuredLog`（tenant/project/request/operation/tags）。
- [x] `OBS-002` 实现 `MetricName`（namespace/subsystem/name/unit）与基数控制占位；dashboard/recording rules 后续补充。
- [x] `OBS-003` 实现 `AuditRecord`（id/timestamp/actor/action/resource/outcome）与 `integrity_hash` 校验；不可变查询/导出/保留/legal hold 后续由 storage 层实现。
- [x] `OPS-001` `DiagnosticBundle` 收集脱敏日志、指标、审计 tail；完整 CLI 在 `moqentra-control-plane`/`moqentra-node-agent` 后续补充。
- [x] `OPS-002` 新增 `ops/runbooks/README.md`：数据库、对象存储、队列、节点故障、证书、磁盘、训练风暴 runbook。
- [x] `OPS-003` `DiagnosticBundle.add_log` 先调用 `sanitize` 脱敏；`is_safe` 检查无 `password=`/`token=` 泄露。

## 29. 完成证据

- 提交：新增/重写 `crates/observability/src/lib.rs`；更新 `crates/observability/Cargo.toml`；新增 `ops/runbooks/README.md`。
- `TraceContext` 生成 `traceparent` header；`StructuredLog` 携带 trace/request/tenant/project/operation/fields。
- `StructuredLog::sanitize` 对 `password/secret/token/api_key/private_key` 字段脱敏。
- `MetricName` 规范命名：`{namespace}_{subsystem}_{name}`。
- `AuditRecord` 含完整性哈希 `integrity_hash`；`verify_integrity` 基于内容重算。
- `DiagnosticBundle` 聚合脱敏日志、指标和审计 tail；`is_safe` 做泄露检查。
- `ops/runbooks/README.md` 提供 7 个运维 runbook 模板。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`moqentra-observability` tests 通过；crate graph 合规。

完成条件：故障演练能仅依据指标、日志、trace、审计和 runbook 定位；遥测后端故障不阻塞业务。
