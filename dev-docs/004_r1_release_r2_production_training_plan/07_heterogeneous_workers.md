# 07. NVIDIA、AMD 与 Ascend Worker

## 1. 镜像与能力隔离

- [ ] `R2-HW-001` 分别维护 CUDA、ROCm、CANN/torch-npu Worker image、lockfile、SBOM、provenance 和许可证；禁止混装厂商 SDK。
- [ ] `R2-HW-002` Worker handshake 上报 vendor/device、driver、runtime、framework、collective、model format、memory、topology 和 build digest。
- [ ] `R2-HW-003` capability 由运行时探测而非环境字符串声明；探测失败时设备不进入 schedulable capacity。
- [ ] `R2-HW-004` scheduler 验证 ResourceClass、Worker image 和 node capability 三方兼容后才能创建 workload。
- [ ] `R2-HW-005` 每个 vendor 提供独立 smoke/accuracy/checkpoint/recovery suite，报告格式统一但阈值按模板和硬件 profile 固定。

## 2. NVIDIA 硬门禁

- [ ] `R2-NVIDIA-001` 冻结数据中心 GPU、driver、CUDA 12.8、cuDNN、NCCL、PyTorch 2.7 和 device plugin 兼容组。
- [ ] `R2-NVIDIA-002` 完成分类、检测、分割的单机回归，以及分类 DDP、检测 DDP smoke 和 checkpoint 恢复。
- [ ] `R2-NVIDIA-003` 采集 DCGM/NVML 指标、Xid/ECC、温度、功耗、显存和 NCCL error，并映射为可诊断 failure class。
- [ ] `R2-NVIDIA-004` 升级 driver/runtime/Worker image 必须重跑完整硬件与恢复套件，不能只验证容器启动。

## 3. AMD/Ascend 分层门禁

- [ ] `R2-AMD-001` 构建 ROCm 6.4/PyTorch 2.7 Worker，完成静态、单元、镜像启动和 capability contract；无 MI300X 时保持 compile-only。
- [ ] `R2-AMD-002` 取得真实 MI300X/MI250X 后完成训练、ROCm collective、checkpoint/resume、Artifact 和监控，才可提升 preview。
- [ ] `R2-ASCEND-001` 构建 CANN 9/torch-npu 或 MindSpore 独立 Worker，固定 OS/toolkit/ops/nnal 组合；无 Atlas runner 时保持 compile-only。
- [ ] `R2-ASCEND-002` 取得真实 Atlas 后完成训练、HCCL、checkpoint/resume、OM 转换和监控，才可提升 preview。
- [ ] `R2-HW-006` baseline blocker 的关闭必须引用真实 runner、日志、产物和精度；交叉构建/mock 不可关闭硬件 blocker。

## 4. 支持矩阵行为

- [ ] `R2-HW-007` API/UI 同时显示 capability available、support tier 和 evidence date，避免“可调度”被误解为“受支持”。
- [ ] `R2-HW-008` 请求 preview/compile-only profile 时返回显式风险说明和审批要求；生产 namespace 默认只允许 supported。
- [ ] `R2-HW-009` 无对应 runtime/toolchain 时 Conversion/Training Operation 在 admission 阶段失败，不进入无限等待队列。

## 5. 完成条件

- NVIDIA 数据中心多节点证据是 `v0.2.0` 硬门禁。
- AMD/Ascend 无真实硬件时不阻止其他 R2 能力，但支持等级必须保持 compile-only。
- 每份硬件报告包含环境 digest、模板、数据、seed、指标、故障恢复、监控和已知限制。
