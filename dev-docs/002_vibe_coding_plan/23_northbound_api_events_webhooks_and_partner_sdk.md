# 23. 北向 API、事件与生态 SDK

## 1. API

资源覆盖 projects、datasets/versions/assets、annotation projects/tasks/reviews、experiments/jobs、models/versions/artifacts、applications/versions、deployments、nodes、operations 和 audit。写请求支持 `Idempotency-Key`；乐观更新使用 ETag/revision；列表使用不透明 cursor；长操作返回 `202 + Operation URL`。

错误采用 RFC 9457 Problem Details，含稳定 code/request_id，不暴露堆栈。SSE 按租户过滤并支持保留窗口内游标恢复。

## 2. Webhook 与 SDK

Webhook 原始 body 使用 event/delivery/timestamp HMAC，具备超时、退避、熔断、死信和重放。每次 DNS/重定向重新做 SSRF 校验。Rust、Python、TypeScript SDK 从 OpenAPI/Proto 生成薄层，手写部分仅包装分页、Operation 轮询和重试。

## 3. 任务

- [ ] `API-001` 实现认证、授权、request ID、trace、限流、body/timeout/CORS 中间件。
- [ ] `API-002` 完成资源 API、OpenAPI 示例和 breaking 门禁。
- [ ] `EVENT-001` 实现 SSE cursor、心跳、有界慢消费者和权限过滤。
- [ ] `HOOK-001` 实现签名、持久化投递、SSRF 防护、死信和审计重放。
- [ ] `SDK-001` 发布版本匹配的 SDK、最小样例和兼容测试。

完成条件：每个端点覆盖成功、校验、未认证、越权、租户越界、幂等和过载测试。
