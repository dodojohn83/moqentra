# 02. API、身份与持久化控制面

## 1. 固定接口决策

- 北向 API 基础路径为 `/v1`，OpenAPI 是 HTTP 契约唯一来源。
- 资源集合至少包含 tenants/projects、datasets/versions/upload-sessions、annotation-projects/tasks、experiments/training-jobs、models/versions、applications/versions、deployments 和 operations。
- 写请求要求 `Idempotency-Key`；有 revision 的更新要求 `If-Match`；异步命令返回 `202` 和 `Operation/v1`。
- 错误使用 RFC 9457 Problem Details；无权访问跨租户资源时与资源不存在统一返回 404。
- 浏览器身份使用 OIDC Authorization Code + PKCE；内部服务使用 mTLS 服务身份。浏览器提供的租户/项目 header 不能成为授权依据。

## 2. Application ports 与事务边界

- [ ] `R1-API-001` 把 control-plane 中的 handler DTO 和路由移入 `moqentra-http-api`，由 app 入口只负责配置、依赖注入、middleware 和生命周期。
- [x] `R1-API-002` 在 application 层定义 dataset、annotation、training、model、application、deployment repositories；所有方法显式携带 `RequestContext`（operation/audit/outbox/idempotency ports 在后续 storage adapter 中补齐）。
  - Evidence: `crates/application/src/ports.rs` (`DatasetRepository`, `AnnotationRepository`, `TrainingJobRepository`, `ModelRepository`, `ApplicationRepository`, `DeploymentRepository`), `Versioned<T>` with `Revision`/`ETag`.
- [ ] `R1-API-003` 定义 `UnitOfWork`：聚合变更、Operation、outbox、audit 和 idempotency response 必须在同一 PostgreSQL 事务提交。
- [ ] `R1-API-004` 统一 cursor pagination、过滤、稳定排序、revision 和 ETag；禁止无上限 list。
- [x] `R1-API-005` 新增 `Operation/v1` 与 `EventEnvelope/v1` schema，覆盖状态、进度、资源引用、错误、deadline、取消、重试、事件序号和 SSE cursor。
  - Evidence: `proto/moqentra/common/v1/operation.proto`, `proto/moqentra/common/v1/event_envelope.proto`; generated Rust types in `moqentra-contracts`, roundtrip tests in `crates/contracts/src/lib.rs`.
- [ ] `R1-API-006` 生成并校验 Rust server types、TypeScript client 和 Python client；生成结果必须确定性且 CI 工作区无差异。

## 3. PostgreSQL

- [x] `R1-DB-001` 从 `0002` 起追加核心资源表、不可变版本表、关系表、状态历史、Operation、租约和对象引用；为所有租户查询和调度查询建立复合索引。
  - Evidence: `crates/storage/migrations/0002_init_resources.sql` (dataset_versions, annotation_projects, annotation_tasks, training_jobs, models, model_versions, applications, application_versions, deployments, operations, operation_events, leases, object_references, resource_state_history plus indexes).
- [ ] `R1-DB-002` 实现全部 PostgreSQL repositories，使用显式 SQL 和行数校验；更新以 `(id, tenant_id, revision)` 实现乐观并发。
- [x] `R1-DB-003` 实现 PostgreSQL outbox：`FOR UPDATE SKIP LOCKED` 有界领取、lease、重试退避、dead-letter、processed message 去重和重启恢复。
  - Evidence: `crates/storage/src/pg_outbox.rs` (`PgOutboxStore`), `crates/storage/migrations/0003_outbox_lease.sql` (lease/retry columns), tests `pg_outbox_append_poll_and_complete` and `pg_outbox_retry_then_dead_letter`.
- [x] `R1-DB-004` 实现 PostgreSQL idempotency：请求指纹冲突、in-progress、完成响应回放、TTL 和安全 GC；同 key 不得关联不同请求。
  - Evidence: `crates/storage/src/pg_idempotency.rs` (`PgIdempotencyStore`), `IdempotencyScope.fingerprint`, `crates/storage/migrations/0005_idempotency_id.sql`, `crates/storage/migrations/0004_idempotency_admin.sql`; tests `pg_idempotency_begin_complete_and_replay`, `pg_idempotency_fingerprint_conflict`, `pg_idempotency_in_progress_conflict`.
- [x] `R1-DB-005` 修正 RLS 为 fail-closed。没有 `app.current_tenant` 时业务表返回零行/拒绝写入；tenant 表的跨租户管理只允许独立管理员策略。
  - Evidence: `crates/storage/migrations/0002_init_resources.sql` (`tenant_matches()` helper, fail-closed policies on all new tables, `tenants` policy scoped to tenant id or `current_admin()`).
- [x] `R1-DB-006` 连接池 checkout 设置租户事务上下文，归还前强制 reset。
  - Evidence: `crates/storage/src/pool.rs` (`ScopedConnection::set_tenant`, `clear_tenant` called before returning to pool, `ConnectionPool::acquire` resets `app.current_tenant`).

## 4. OIDC、RBAC 与审计

- [x] `R1-IAM-001` 实现 OIDC discovery、JWKS 缓存与轮换、issuer/audience/nonce 校验、clock skew 和失败降级；移除生产 HMAC 开发 token 路径。
  - Evidence: `crates/auth/src/oidc.rs` (`OidcConfig`, `JwkSetValidator`), `crates/auth/src/jwt.rs` (`TokenClaims.nonce`, async `TokenValidator`, `CompositeTokenValidator.with_oidc`); `apps/control-plane/src/main.rs` uses `MOQENTRA_OIDC_ISSUER`/`MOQENTRA_OIDC_AUDIENCE` and OIDC takes precedence, HMAC only as local test fallback.
- [ ] `R1-IAM-002` 从 token 得到 principal，只从数据库成员关系解析 tenant/project role；切换租户必须重新授权。
- [x] `R1-IAM-003` 实现 tenant admin、data engineer、annotator、reviewer、algorithm engineer、operator 的 deny-by-default 权限矩阵。
  - Evidence: `crates/auth/src/rbac.rs` (`Authorizer` with `Role::{TenantAdmin,DataEngineer,Annotator,Reviewer,AlgorithmEngineer,Operator}`, deny-by-default, project/tenant isolation, tests).
- [~] `R1-IAM-004` 每个写操作、审核、发布、下载授权和拒绝结果写入结构化审计；审计内容脱敏且不可被普通租户用户修改。
  - Evidence: `crates/storage/migrations/0006_audit_logs.sql` (append-only, RLS, forced RLS); `crates/storage/src/pg_audit.rs` (`PgAuditLog`); `crates/auth/src/audit.rs` async `AuditLog` trait; `apps/control-plane/src/main.rs` `authorize` logs every authorization success/denial to `state.audit`. Write/publish/download audit integration pending per handler.
- [x] `R1-IAM-005` 对 health 以外路由启用认证、限流、请求大小、timeout、request/correlation ID 和安全响应头 middleware。
  - Evidence: `apps/control-plane/src/main.rs` `require_auth_middleware`, `security_headers`, `DefaultBodyLimit`; `x-request-id`/`x-correlation-id`; per-tenant `check_rate_limit`. Timeout layer deferred to reverse proxy / container runtime.

## 5. 完成条件与测试

- 真实 PostgreSQL repository contract tests 覆盖 CRUD、分页、并发 revision、RLS、事务回滚和进程重启。
- 控制面创建资源后重启，GET/list/Operation/outbox 状态保持一致。
- 租户 A 不能通过 ID 枚举、header 伪造、复用连接或审计接口推断租户 B 资源。
- `docs/openapi/openapi.yaml` 与运行路由完全一致，无重复 operationId 或遗留非 `/v1` 路径。
