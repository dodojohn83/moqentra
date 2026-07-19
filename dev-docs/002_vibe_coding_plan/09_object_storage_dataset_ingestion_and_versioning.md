# 09. 对象存储、数据导入与版本

## 1. 数据模型

`Dataset` 是可变容器，`DatasetVersion` 是不可变快照。Asset 保存 digest、size、media type、尺寸/时长、来源、对象引用和安全扫描状态；版本通过 manifest 有序引用 asset，不复制对象。

导入支持浏览器分片上传、服务端受控 URL 拉取、S3 前缀扫描和已登记对象复用。外部 URL 默认拒绝内网、metadata、重定向越界和超额响应。

## 2. 状态

ImportJob：Pending → Inspecting → Transferring → Validating → Completed；任一阶段可 Failed/Cancelled。DatasetVersion：Draft → Validating → Published → Deprecated。Published 后仅能创建新版本。

## 3. 任务

- [ ] `DATA-001` 实现 S3 port、MinIO adapter、multipart、checksum 和短期签名 URL。
- [ ] `DATA-002` 规范对象 key、服务端加密、租户配额和生命周期策略。
- [ ] `DATA-003` 实现媒体探测、病毒/恶意文件扫描、重复检测和失败隔离。
- [ ] `DATA-004` 生成 canonical `DatasetManifest/v1`，digest 覆盖成员与关键元数据。
- [ ] `DATA-005` 实现 train/val/test 切分，随机 seed 和规则写入版本。
- [ ] `DATA-006` 实现引用计数式 GC；legal hold、训练或模型血缘引用禁止删除。
- [ ] `DATA-007` 测试分片重试、digest 冲突、超额、SSRF、对象丢失和并发发布。

完成条件：同一 manifest 可重建相同版本；控制面和 Web 永不持有长期对象存储密钥。
