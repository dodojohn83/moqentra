# 10. 标注平台与 LabelU-Kit

## 1. 集成边界

LabelU-Kit 只负责浏览器标注交互。项目、任务、分配、版本、权限、自动保存和审核由 Moqentra 权威管理。前端通过 adapter 把 `AnnotationProjectSpec/v1` 映射到固定版本的 LabelU React packages；组件不得直连数据库或长期 S3 凭据。

首版任务：图片/视频分类、2D 框、多边形分割、关键点、目标关联与视频跟踪。每个 annotation 保存 schema version、asset、frame/time range、label ontology、revision 和 actor。

## 2. 并发与保存

任务 lease 有期限和 fencing token。自动保存使用 `(task_id, revision, client_update_id)` 幂等；冲突返回服务器 revision 和安全 diff，不做静默 last-write-wins。最终提交冻结快照，返工创建新 revision。

## 3. 任务

- [x] `LABEL-001` 完成 LabelU-Kit fit-gap 映射：`LabelUAdapter.ts` 支持任务类型、工具映射和媒体类型；视频 seek/可访问性后续补充。
- [x] `LABEL-002` 实现 `Ontology`、`Label`、`ToolConfig`、`AnnotationProject` 和 `AnnotationTask` 领域模型与 JSON mapper。
- [x] `LABEL-003` 实现任务状态机、领取/释放/续租、lease fencing token、过期校验和批量/进度占位。
- [x] `LABEL-004` 实现 `AnnotationLog` 自动保存、幂等 `(task_id, revision, client_update_id)` 与冲突 diff 返回。
- [x] `LABEL-005` 短期签名 URL 由后端生成，前端 adapter 在导出前剔除 payload 中的 `url`/`signedUrl`/`presignedUrl`/`s3Key`/`secret` 字段。
- [x] `LABEL-006` 实现 COCO 导出（categories/images/annotations/bbox/segmentation）、YOLO 行生成、完整 VOC XML、native JSON 导出和格式探测。
- [x] `LABEL-007` 测试 lease 过期/stale fencing token、幂等保存与冲突、跨租户字段隔离、恶意/超长跑马灯 payload 拒绝（通过 JSON 解析约束）。

## 10. 完成证据

- 提交：新增/扩展 `crates/domain/src/annotation.rs` 与 `export.rs`；新增 `apps/web/src/annotation/LabelUAdapter.ts`。
- `AnnotationProject` / `AnnotationTask` / `Annotation` / `Ontology` 状态机实现。
- `TaskLease` 支持 fencing token 与续租；自动保存幂等键为 `(task_id, revision, client_update_id)`；冲突返回 `server_revision` 和 diff。
- `export.rs` 支持 `CocoDataset`、`yolo_line`、完整 `annotations_to_voc`、`annotations_to_native` 和 `format_by_extension`。
- `apps/web/src/annotation/LabelUAdapter.ts` 提供 `toLabelUProjectConfig` 与 `fromLabelUAnnotations` 以及 `maskLabelFromPayload`（脱敏 URL）。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：workspace tests 通过；crate graph 合规。

## 4. 本地修改门槛

若公开扩展点无法满足签名 URL、幂等保存、任务类型或 CSP，才在 `third_party/labelu-kit` 维护固定 commit 的本地分支；必须保存 LICENSE/NOTICE、patch 清单、构建复现和上游升级测试。

完成条件：组件升级不改变平台权威数据；浏览器刷新、断网重连和重复提交不丢失已确认标注。
