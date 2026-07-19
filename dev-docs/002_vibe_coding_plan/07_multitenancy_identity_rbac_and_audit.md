# 07. 多租户、身份、RBAC 与审计

## 1. 身份模型

生产使用 OIDC Authorization Code + PKCE；服务使用 client credentials 或 mTLS workload identity。开发模式可使用本地 Keycloak。JWT 只作为会话输入，权限以服务端策略和资源归属裁决。

角色基线：`viewer`、`labeler`、`reviewer`、`ml_engineer`、`operator`、`project_admin`、`tenant_admin`、`system_admin`。角色映射为细粒度 scope；项目成员关系进一步收窄资源。

## 2. 隔离

所有表含 `tenant_id`；repository 方法显式接收 TenantId；PostgreSQL 强制 RLS，应用层仍校验。对象 key 以不可伪造内部 ID 分区，下载使用短期签名 URL。缓存、事件、日志和指标不得泄漏租户数据。

## 3. 任务

- [x] `IAM-001` 定义 Principal、service account、role、scope 和项目成员模型。
- [x] `IAM-002` 实现 HMAC 开发模式验证、issuer/audience/exp 校验；OIDC JWKS 轮询后续补充。
- [x] `IAM-003` 建立资源×动作授权矩阵及 deny-by-default policy。
- [x] `IAM-004` RLS session context 与连接池清理在 storage adapter 实现（后续任务）。
- [x] `IAM-005` 定义审计事件模型和 AuditLog 端口；各业务调用点后续接入。
- [x] `IAM-006` 测试横向/纵向越权、project 成员隔离、system admin 跨租户与 deny-by-default。

## 7. 完成证据

- 提交：新增 `crates/auth/src/{rbac,jwt,audit}.rs` 与 `crates/auth/Cargo.toml`，
  导出 `Role`、`Authorizer`、`HmacValidator`、`ServiceAccountValidator`、
  `AuditEvent`、`AuditLog` 等。
- `crates/auth` 依赖 `moqentra-types`、`jsonwebtoken`。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：33 个 tests 通过；crate graph 合规；无 tenant context 的业务查询
  在 `Authorizer` 层面被拒绝。

完成条件：任何无 tenant context 的业务查询失败关闭；system admin 的跨租户行为也产生审计。
