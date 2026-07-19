# 24. Web 前端架构与数据安全

## 1. 架构

采用单 React Shell 和领域 packages：core、auth、data、annotation、training、models、orchestration、deployments、admin。模块只能通过公开 package API 和 typed client 交互；路由级懒加载。LabelU-Kit 位于 annotation package。

微前端不在首版启用。只有模块拥有独立团队、独立发布节奏且共享依赖/认证协议稳定后，才通过 ADR 拆分；禁止仅因页面数量拆分。

## 2. 安全

OIDC 使用 Authorization Code + PKCE；token 优先保存在内存并通过安全刷新机制续期。实施 CSP、Trusted Types、CSRF、点击劫持防护、依赖完整性、文件下载白名单和敏感字段屏蔽。前端权限只控制展示，服务端始终重新授权。

## 3. 任务

- [ ] `WEB-001` 建立 Shell、路由、设计系统、错误边界、国际化和可访问性。
- [ ] `WEB-002` 生成 typed API client，禁止散落 fetch 和手写 DTO。
- [ ] `WEB-003` 实现租户/项目上下文切换并清空 query/cache/form 状态。
- [ ] `WEB-004` 实现大型上传、进度、取消、断点和失败恢复。
- [ ] `WEB-005` 完成训练日志/指标、模型、图编排和部署状态页面。
- [ ] `WEB-006` 建立 CSP/XSS、越权导航、缓存串租户、浏览器和 E2E 测试。

完成条件：构建产物无 secret/source map 泄漏；切换租户后旧请求结果不能渲染。
