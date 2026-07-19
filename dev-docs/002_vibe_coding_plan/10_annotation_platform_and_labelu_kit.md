# 10. 标注平台与 LabelU-Kit

## 1. 集成边界

LabelU-Kit 只负责浏览器标注交互。项目、任务、分配、版本、权限、自动保存和审核由 Moqentra 权威管理。前端通过 adapter 把 `AnnotationProjectSpec/v1` 映射到固定版本的 LabelU React packages；组件不得直连数据库或长期 S3 凭据。

首版任务：图片/视频分类、2D 框、多边形分割、关键点、目标关联与视频跟踪。每个 annotation 保存 schema version、asset、frame/time range、label ontology、revision 和 actor。

## 2. 并发与保存

任务 lease 有期限和 fencing token。自动保存使用 `(task_id, revision, client_update_id)` 幂等；冲突返回服务器 revision 和安全 diff，不做静默 last-write-wins。最终提交冻结快照，返工创建新 revision。

## 3. 任务

- [ ] `LABEL-001` 完成 LabelU-Kit fit-gap：任务类型、事件 API、撤销、视频 seek、性能、可访问性和浏览器矩阵。
- [ ] `LABEL-002` 实现 ontology、tool config 和 annotation schema mapper/golden fixtures。
- [ ] `LABEL-003` 实现任务切分、领取、释放、超时续租、批量分配和进度统计。
- [ ] `LABEL-004` 实现自动保存、离线短暂重试、冲突提示、提交与返工。
- [ ] `LABEL-005` 媒体仅通过短期签名 URL 或受权代理读取；禁止 URL 出现在日志和埋点。
- [ ] `LABEL-006` 实现 COCO、VOC、YOLO 和平台原生格式导入导出，记录损失字段。
- [ ] `LABEL-007` 测试跨租户、过期 lease、双客户端编辑、超长视频和恶意标注 payload。

## 4. 本地修改门槛

若公开扩展点无法满足签名 URL、幂等保存、任务类型或 CSP，才在 `third_party/labelu-kit` 维护固定 commit 的本地分支；必须保存 LICENSE/NOTICE、patch 清单、构建复现和上游升级测试。

完成条件：组件升级不改变平台权威数据；浏览器刷新、断网重连和重复提交不丢失已确认标注。
