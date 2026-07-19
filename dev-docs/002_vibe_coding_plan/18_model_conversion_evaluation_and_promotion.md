# 18. 模型转换、评测与晋级

## 1. 转换任务

ConversionJob 输入为已验证模型版本与目标 profile，输出派生 artifact。目标包括 TensorRT engine、OpenVINO IR、RKNN、Sophon bmodel、Ascend OM；每种目标使用独立镜像、SDK 版本和真实设备验证。

转换缓存键覆盖输入 digest、工具链 digest、参数、目标芯片和精度。不同 GPU compute capability 或芯片型号不得错误复用。

## 2. 评测

EvaluationRun 固定模型 artifact、数据版本、指标实现、阈值和硬件 profile。晋级 policy 同时检查精度、性能、兼容、安全扫描和审批。

## 3. 任务

- [ ] `CONVERT-001` 为每个后端定义 profile schema、capability 和工具链镜像。
- [ ] `CONVERT-002` 实现转换状态机、缓存、日志、artifact manifest 和失败分类。
- [ ] `EVAL-001` 实现离线精度、混淆矩阵、检测 mAP、分割 mIoU 和性能评测。
- [ ] `EVAL-002` 建立参考输出、容差、预后处理版本和可重复 seed。
- [ ] `PROMOTE-001` 实现 policy-as-data、人工审批、签名和审计。
- [ ] `PROMOTE-002` 测试量化退化、动态 shape、芯片不匹配、工具链升级和回滚。

完成条件：转换成功不自动等于可发布；每种 supported 产物都有真实目标硬件加载与推理证据。
