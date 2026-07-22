# 03. 数据导入、版本冻结与标注闭环

## 1. 对象和上传

- [x] `R1-DATA-001` 定义租户/项目/资源/版本分层的对象 key builder，调用者不能传任意 bucket key；数据库只保存受控 key 与摘要。
- [x] `R1-DATA-002` 实现 multipart upload session API：创建、签名分片、列出已传分片、完成、取消和过期；校验 part 数、ETag、总大小、MIME 与内容摘要。
- [x] `R1-DATA-003` 实现浏览器断点续传和取消；签名 URL 只允许一个对象、一个方法和短 TTL，响应不得暴露长期 S3 凭据。
- [x] `R1-DATA-004` 实现 S3/MinIO 导入 Operation，使用服务端 copy/stream、deadline、并发上限、重试和幂等对象去重。
- [x] `R1-DATA-005` 由隔离 Worker 执行媒体探测、图片解码验证、视频元数据提取和恶意文件扫描；失败资产不可进入数据版本。
- [x] `R1-DATA-006` 实现临时对象引用、legal hold 和有界 GC；正在上传、被版本或模型引用的对象不得删除。

## 2. 数据集版本

- [x] `R1-DATA-007` 持久化 Dataset、Asset、DatasetVersion 和 Manifest，冻结事务中验证所有对象、标签 schema 和摘要后原子发布。
- [x] `R1-DATA-008` 使用固定 seed 和明确规则生成 train/val/test split；规则和结果进入 `DatasetManifest/v1`，重新生成摘要稳定。
- [x] `R1-DATA-009` 冻结后禁止增加、删除或替换资产；变更只能派生新版本并保留 parent/source lineage。
- [ ] `R1-DATA-010` 实现 COCO、LabelU native 和平台中间格式导入导出，验证 round-trip 不丢类别、bbox、polygon、frame 和追踪标识。

## 3. LabelU-Kit 与审核

- [x] `R1-LABEL-001` 固定并引入实际可用的 LabelU-Kit v5.11.0，记录许可证与 lockfile；BLOCK-005 未解除前不声明 v5.11.1。
- [x] `R1-LABEL-002` 实现标注项目、ontology、任务切分和进度 API；支持图像分类、矩形检测、语义/实例分割、视频目标与轨迹。
- [x] `R1-LABEL-003` 实现 task claim/renew/release、lease deadline、fencing token 和断线恢复；过期或旧 token 保存必须冲突失败。
- [x] `R1-LABEL-004` 实现草稿自动保存：`client_update_id + base_revision` 幂等，冲突返回可展示 diff，不静默覆盖服务端版本。
- [x] `R1-LABEL-005` 实现 submit → review → approve/reject → rework 状态机和不可变历史；审核人不能审核自己的任务，管理员例外须审计。
- [x] `R1-LABEL-006` 媒体 URL 按任务授权即时签发；保存、导出、日志、SSE 和浏览器缓存中不得出现 secret、对象 key 或已过期 URL。

## 4. 完成条件与测试

- MinIO 真实测试覆盖分片中断恢复、重复完成、摘要冲突、过期清理、签名权限和跨租户 key 拒绝。
- 图片与视频各完成一次 LabelU 标注、审核退回、修订和导出再导入。
- 数据版本冻结过程中杀死控制面或 Worker，恢复后只允许完整 published 或可重试 failed，不产生半发布 manifest。
- 冻结版本可被训练引用；被引用版本和对象不能被 GC。
