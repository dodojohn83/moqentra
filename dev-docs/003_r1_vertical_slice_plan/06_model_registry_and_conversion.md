# 06. 模型注册、血缘与 ONNX 转换

## 1. 模型注册

- [x] `R1-MODEL-001` 持久化 Model、ModelVersion、Artifact、Signature、Lineage、Attachment、Approval 和状态历史；版本号在模型内唯一。
- [x] `R1-MODEL-002` 训练结果进入 quarantined/staging；Artifact reconciler 校验大小、SHA-256、媒体类型、反序列化安全、病毒扫描和 manifest 后才可标记 ready。
- [ ] `R1-MODEL-003` 强制记录 dataset version/manifest digest、training job/attempt、template/version、代码和镜像 digest、framework、参数、指标、checkpoint 与硬件环境。
- [ ] `R1-MODEL-004` Worker 重试或 Result 重放使用 `(tenant, attempt, artifact manifest digest)` 去重，不产生重复 ModelVersion。
- [x] `R1-MODEL-005` 实现 draft → validating → ready → approved/published → deprecated 状态；发布需要算法工程师申请和有权限审批人确认并审计。
- [ ] `R1-MODEL-006` 被 ApplicationVersion 或 Deployment 引用的 Artifact 受引用保护；弃用不删除内容，GC 只处理无引用临时对象。

## 2. ONNX 与评估

- [x] `R1-CONVERT-001` Conversion Operation 运行于隔离 Worker，输入输出均按 digest；禁止控制面加载模型或执行厂商转换器。
- [x] `R1-CONVERT-002` 三个基线模板导出 ONNX，记录 opset、dynamic axes、输入输出 TensorSpec、预处理、类别映射和依赖版本。
- [x] `R1-CONVERT-003` 使用 ONNX Runtime 加载、shape 校验和固定 fixture 推理；比较 PyTorch 与 ONNX 输出，阈值按模板版本固定并写入评估报告。
- [ ] `R1-CONVERT-004` 检测模型保存后处理契约：置信度、NMS、box 坐标和类别映射；dyun compiler 不猜测缺失参数。
- [ ] `R1-CONVERT-005` TensorRT/OpenVINO 只有真实转换、加载和 fixture 推理通过后才标记 `preview`；否则支持矩阵保持 compile-only/unsupported。
- [ ] `R1-CONVERT-006` 发布策略检查扫描状态、必需指标、评估结果、许可证 attachment、审批和目标 runtime compatibility。

## 3. API 行为

- `POST /v1/models` 创建模型族；`POST /v1/models/{id}/versions` 只供受控导入或 reconciler 使用。
- `POST /v1/model-versions/{id}:convert|:evaluate|:request-publish|:approve` 返回 Operation 或更新后的 revision。
- Artifact 下载先鉴权再生成短期 URL；API 不代理长期大文件，不暴露 object key 或存储凭据。
- 模型详情返回不可变 lineage snapshot，不能用当前模板或数据版本覆盖历史字段。

## 4. 完成条件与测试

- 任一 ONNX Artifact 可追溯到准确 DatasetManifest、训练 attempt、代码、镜像、环境与评估报告。
- 破损、摘要不符、包含不安全内容、缺失 signature 或不兼容 runtime 的 Artifact 无法 ready/publish。
- PostgreSQL 事务或对象上传任一失败都不会产生指向不存在对象的 published 版本。
- 检测 ONNX 在固定 fixture 上通过数值和语义对齐，并能被下一章 dyun 链路加载。
