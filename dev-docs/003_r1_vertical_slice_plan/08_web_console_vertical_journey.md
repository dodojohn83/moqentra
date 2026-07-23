# 08. Web 控制台纵向用户旅程

## 1. 前端架构

- [x] `R1-WEB-001` 增加 React Router、OIDC PKCE session、受保护路由、错误边界、设计 token、i18n 基线和可访问性检查；不引入微前端。
- [x] `R1-WEB-002` 以 OpenAPI 生成的 TypeScript types/client 为基础封装唯一 API client；业务组件禁止直接 `fetch`。
- [ ] `R1-WEB-003` 使用 query/cache 层按 tenant、project、resource 和 revision 建 key；切换租户时取消请求、上传与 SSE，并清空前一租户缓存。
- [~] `R1-WEB-004` 统一 Problem Details、401 refresh/login、403、404、409 revision、429 和 202 Operation 交互；失败不得显示虚假成功 toast。
- [ ] `R1-WEB-005` SSE client 持久化 cursor、自动重连、事件去重并在 cursor 失效时重新获取权威资源。

## 2. 业务页面

- [ ] `R1-WEB-006` 项目与导航：whoami、租户/项目选择、角色可见性和审计入口；租户不能通过手输 ID 越权切换。
- [ ] `R1-WEB-007` 数据集：列表、创建、multipart 上传、S3 导入、进度、资产预览、失败诊断、版本 split 与冻结。
- [ ] `R1-WEB-008` 标注：嵌入固定 LabelU-Kit，完成任务领取、自动保存、冲突提示、提交、审核、退回和导出。
- [ ] `R1-WEB-009` 实验与训练：选择冻结版本和模板、参数 schema 表单、资源选择、提交、取消、日志/指标/checkpoint 实时查看。
- [ ] `R1-WEB-010` 模型：血缘、Artifact、signature、评估、转换、发布申请、审批和短期下载。
- [ ] `R1-WEB-011` 应用：React Flow 组件目录、类型化连线、参数编辑、静态错误定位、版本发布、binding 预览和 bundle digest。
- [ ] `R1-WEB-012` 部署：目标 agent、发布、Operation 进度、replica 状态、日志/metrics、停止和重新发布。

## 3. 安全与体验

- [x] `R1-WEB-013` CSP 默认拒绝内联脚本；OIDC、API、媒体和必要 WebSocket/SSE 域使用精确 allowlist。
- [x] `R1-WEB-014` Cookie 使用 Secure/HttpOnly/SameSite；若使用 bearer token，只保存在内存，不进入 localStorage、URL 或日志。
- [~] `R1-WEB-015` 文件下载、图片/视频预览和 LabelU payload 使用受控 URL；禁止 `dangerouslySetInnerHTML` 展示后端错误或用户标签。
- [ ] `R1-WEB-016` 大列表分页/虚拟化，上传和 Operation 可取消；刷新页面后从服务端恢复进度，不依赖浏览器内存状态。
- [~] `R1-WEB-017` 关键页面满足键盘操作、焦点、表单标签和颜色对比基线；LabelU-Kit 已知缺口单独登记。

## 4. 完成条件与测试

- Vitest 覆盖 API error、tenant cache isolation、SSE reconnect、upload resume 和 LabelU mapper。
- Playwright 使用真实 control-plane/Dex/PostgreSQL/MinIO 完成 data engineer、annotator、reviewer、algorithm engineer 四类旅程。
- 安全 E2E 覆盖 token 过期、租户切换、恶意 label/filename、过期签名 URL、CSRF、CSP 和越权资源 ID。
- 构建产物无 source map secret、环境凭据或动态加载未固定第三方代码。
