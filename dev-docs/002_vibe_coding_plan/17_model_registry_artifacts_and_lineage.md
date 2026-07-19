# 17. 模型注册、产物与血缘

## 1. 模型层级

Model 是业务名称；ModelVersion 是不可变发布单元；Artifact 是内容寻址文件。版本保存训练 job、数据/标注版本、代码/镜像、超参、指标、输入输出签名、许可证和创建者。

状态：Draft → Validating → Ready → Approved → Deprecated/Rejected。部署只能引用 Approved，开发租户可由策略允许 Ready。

## 2. 任务

- [ ] `MODEL-001` 实现模型、版本、artifact、signature、metric 和 lineage repository。
- [ ] `MODEL-002` 校验文件 digest、大小、格式、恶意内容和安全反序列化策略。
- [ ] `MODEL-003` 支持 original checkpoint 与 ONNX 主交换格式；记录 opset 和动态 shape。
- [ ] `MODEL-004` 实现审批、拒绝、废弃、保留和引用保护。
- [ ] `MODEL-005` 生成 `ModelArtifactManifest/v1` 与软件物料/许可证附件。
- [ ] `MODEL-006` 测试重复上传、对象损坏、审批竞争、删除引用和血缘重放。

完成条件：任何部署 artifact 可回溯到模型、转换、训练、数据和审核人；对象缺失会阻断发布。
