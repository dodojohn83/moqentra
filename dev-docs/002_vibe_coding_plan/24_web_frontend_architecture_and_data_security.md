# 24. Web 前端架构与数据安全

## 1. 架构

采用单 React Shell 和领域 packages：core、auth、data、annotation、training、models、orchestration、deployments、admin。模块只能通过公开 package API 和 typed client 交互；路由级懒加载。LabelU-Kit 位于 annotation package。

微前端不在首版启用。只有模块拥有独立团队、独立发布节奏且共享依赖/认证协议稳定后，才通过 ADR 拆分；禁止仅因页面数量拆分。

## 2. 安全

OIDC 使用 Authorization Code + PKCE；token 优先保存在内存并通过安全刷新机制续期。实施 CSP、Trusted Types、CSRF、点击劫持防护、依赖完整性、文件下载白名单和敏感字段屏蔽。前端权限只控制展示，服务端始终重新授权。

## 3. 任务

- [x] `WEB-001` 建立 `Shell` 与 `TenantProvider` React 上下文；路由/设计系统/i18n/a11y 后续章节完善。
- [x] `WEB-002` 建立集中 `apiClient.ts`：统一 `Authorization`、`Idempotency-Key`、`If-Match`、Problem Details 解析、`202` 长操作响应、SSE cursor 流解析；禁止散落 `fetch`。
- [x] `WEB-003` 实现 `TenantContext` 切换租户时重置 `projectId`；服务层请求通过 `AuthorizedRequest` 重新授权。
- [x] `WEB-004` 实现 `UploadManager` 分块上传、进度、`AbortController` 取消、断点续传占位。
- [x] `WEB-005` 训练/模型/部署页面由领域 packages 提供 typed client 后按需渲染；当前提供 `Shell` 入口。
- [x] `WEB-006` 实现 `security.ts`（下载白名单、敏感字段脱敏、HTML 转义）与单元测试；CSP/XSS/越权/E2E 后续补充。

## 24. 完成证据

- 提交：新增 `apps/web/src/core/{apiClient.ts,security.ts,TenantContext.tsx,Shell.tsx,uploadManager.ts}` 与测试；修复 `LabelUAdapter.ts` 类型错误。
- `Shell` 包裹 `TenantProvider`，通过 context 管理租户/项目切换；切换租户会清空当前 project。
- `apiClient` 统一发送 `Idempotency-Key`、`If-Match`；解析 RFC 9457 `ProblemDetails` 并脱敏 `token`/`api_key`；处理 `202` 长操作与 SSE `data:` 行。
- `UploadManager` 按 5MiB 分块，使用 `AbortController` 支持取消，保存 etag 供续传。
- `security.ts` 提供下载类型白名单、敏感信息脱敏、HTML 转义。
- 测试命令：
  - `npm run typecheck`
  - `npm test`
  - `cargo fmt --all -- --check`（后续 Rust 相关任务通用）
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`npm run typecheck` 与 `vitest` 通过；`LabelUAdapter.ts` 类型错误已修复。

完成条件：构建产物无 secret/source map 泄漏；切换租户后旧请求结果不能渲染。
