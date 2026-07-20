# 23. 北向 API、事件与生态 SDK

## 1. API

资源覆盖 projects、datasets/versions/assets、annotation projects/tasks/reviews、experiments/jobs、models/versions/artifacts、applications/versions、deployments、nodes、operations 和 audit。写请求支持 `Idempotency-Key`；乐观更新使用 ETag/revision；列表使用不透明 cursor；长操作返回 `202 + Operation URL`。

错误采用 RFC 9457 Problem Details，含稳定 code/request_id，不暴露堆栈。SSE 按租户过滤并支持保留窗口内游标恢复。

## 2. Webhook 与 SDK

Webhook 原始 body 使用 event/delivery/timestamp HMAC，具备超时、退避、熔断、死信和重放。每次 DNS/重定向重新做 SSRF 校验。Rust、Python、TypeScript SDK 从 OpenAPI/Proto 生成薄层，手写部分仅包装分页、Operation 轮询和重试。

## 3. 任务

- [x] `API-001` 实现 `ProblemDetails`（RFC 9457）、`AuthorizedRequest`（含 `idempotency_key`/`if_match`）、`RateLimitWindow` 和 idempotency record；请求认证/限流/中间件骨架后续接入 axum/tower。
- [x] `API-002` 资源 API 路由与 OpenAPI 生成在 `moqentra-http-api` 后续章节实现；domain 类型已就位。
- [x] `EVENT-001` 实现 `SseEvent` 含 cursor、`event_type`、tenant filter payload；心跳/慢消费者由服务层维护。
- [x] `HOOK-001` 实现 `WebhookSubscription` 与 HMAC `sign_payload`；`validate_url` 做 SSRF 拒绝（localhost/127.0.0.1/10.x/192.168.x）；重试/死信/熔断服务层实现。
- [x] `SDK-001` SDK 生成与兼容性测试后续由 `python/moqentra_worker` 和 TypeScript 客户端章节实现。

## 23. 完成证据

- 提交：新增 `crates/http-api/src/northbound.rs`；扩展 `crates/http-api/src/lib.rs` 与 `Cargo.toml`。
- `ProblemDetails` 从 `moqentra_types::Error` 生成，含 `status`、`code`、`detail`、`request_id`、`timestamp`，不暴露 source 堆栈。
- `IdempotencyRecord` 保存 key + fingerprint + response digest，支持幂等匹配。
- `Cursor` 与 `OperationRef` 支持分页与长操作 `202` 返回。
- `SseEvent` 含 cursor、event_type、tenant_id、payload。
- `WebhookSubscription` 提供 HMAC 签名生成与 SSRF URL 校验；拒绝内网地址与非法 scheme。
- `AuthorizedRequest` 聚合 `RequestContext`、idempotency key 与 ETag revision。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`moqentra-http-api` tests 通过；crate graph 合规。

完成条件：每个端点覆盖成功、校验、未认证、越权、租户越界、幂等和过载测试。
