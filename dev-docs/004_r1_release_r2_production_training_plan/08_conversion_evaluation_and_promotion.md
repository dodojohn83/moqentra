# 08. 模型转换、评估与晋级

## 1. ConversionProfile

- [ ] `R2-CONVERT-001` profile 固定 source/target format、converter image digest、toolchain、target runtime/hardware、参数 schema 和 support tier。
- [ ] `R2-CONVERT-002` profile 更新创建新版本；已开始 Operation 始终使用提交时 snapshot。
- [ ] `R2-CONVERT-003` converter 运行于隔离 Worker，输入只读、输出临时、无 shell/特权/任意 host mount。
- [ ] `R2-CONVERT-004` admission 校验源 Artifact、signature、opset、dynamic shape、target capability、license 和 approval。

## 2. 格式矩阵

- [ ] `R2-CONVERT-005` ONNX 达到 GA：三个 R1 模板完成导出、加载、shape、数值和固定 fixture 回归。
- [ ] `R2-CONVERT-006` TensorRT 在真实 NVIDIA 环境完成 engine build、加载、warm-up、fixture 推理、精度和性能报告后标记 preview。
- [ ] `R2-CONVERT-007` OpenVINO 在固定 CPU/runtime 完成 IR build、加载和数值/性能报告后标记 preview。
- [ ] `R2-CONVERT-008` Ascend OM、RKNN、Sophon 完成版本化 profile、manifest、dispatch 和隔离 adapter；无实机加载证据保持 compile-only。
- [ ] `R2-CONVERT-009` INT8/量化 profile 必须保存校准 DatasetVersion、采样规则、摘要和精度损失，不复用未授权生产数据。

## 3. 报告与血缘

- [ ] `R2-EVAL-001` ConversionReport 记录源/目标 Artifact、工具链、硬件、命令参数、构建日志摘要、耗时、size 和 checksum。
- [ ] `R2-EVAL-002` EvaluationReport 保存 fixture/dataset、指标、阈值、基线差异、环境和 pass/fail；NaN/Inf 直接失败。
- [ ] `R2-EVAL-003` 后处理、类别映射、NMS、预处理和 tensor signature 作为 Artifact 兼容契约，不由部署端猜测。
- [ ] `R2-EVAL-004` 重复转换以 source digest + profile version + parameters digest 去重；失败重试创建 attempt，不重复发布 Artifact。

## 4. 晋级审批

- [ ] `R2-PROMOTE-001` 晋级要求 clean scan、license、signature、必需评估、目标 runtime evidence 和 ApprovalRequest。
- [ ] `R2-PROMOTE-002` 申请人不能自批；决定保存指标与 support matrix snapshot，过期或 Artifact revision 变化需重新申请。
- [ ] `R2-PROMOTE-003` production 只能选择 target profile 允许的 support tier；compile-only Artifact 不可绕过策略部署。
- [ ] `R2-PROMOTE-004` 模型降级/撤销不删除被 deployment 引用的 Artifact，并向受影响应用产生事件。

## 5. TAS-029 解释与完成条件

TAS-029 的“支持 ONNX、TensorRT、OpenVINO、Ascend OM、RKNN、Sophon”定义为平台能够版本化表达、调度、追踪和按支持等级拒绝不满足条件的目标，而不是所有格式均已获得真实硬件认证。

- ONNX 必须 GA；TensorRT/OpenVINO 至少 preview。
- 其他厂商格式无实机证据时保持 compile-only，并带明确 blocker。
- 任一成功 Artifact 可追溯源模型、转换 profile、镜像、参数、评估和审批。
