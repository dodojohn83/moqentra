# 07. 多租户、身份、RBAC 与审计

## 1. 身份模型

生产使用 OIDC Authorization Code + PKCE；服务使用 client credentials 或 mTLS workload identity。开发模式可使用本地 Keycloak。JWT 只作为会话输入，权限以服务端策略和资源归属裁决。

角色基线：`viewer`、`labeler`、`reviewer`、`ml_engineer`、`operator`、`project_admin`、`tenant_admin`、`system_admin`。角色映射为细粒度 scope；项目成员关系进一步收窄资源。

## 2. 隔离

所有表含 `tenant_id`；repository 方法显式接收 TenantId；PostgreSQL 强制 RLS，应用层仍校验。对象 key 以不可伪造内部 ID 分区，下载使用短期签名 URL。缓存、事件、日志和指标不得泄漏租户数据。

## 3. 任务

- [ ] `IAM-001` 定义 Principal、service account、role、scope 和项目成员模型。
- [ ] `IAM-002` 实现 OIDC 验签、issuer/audience/clock skew 和 key rotation。
- [ ] `IAM-003` 建立资源×动作授权矩阵及 deny-by-default policy。
- [ ] `IAM-004` 实现 RLS session context，连接归还池前必须清理。
- [ ] `IAM-005` 审计登录、授权失败、数据导出、训练、模型发布、部署和密钥操作。
- [ ] `IAM-006` 测试横向/纵向越权、混淆代理、缓存污染和批量接口漏检。

完成条件：任何无 tenant context 的业务查询失败关闭；system admin 的跨租户行为也产生审计。
