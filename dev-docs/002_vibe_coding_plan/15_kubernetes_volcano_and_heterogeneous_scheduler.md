# 15. Kubernetes、Volcano 与异构调度

## 1. 编译目标

TrainingJobSpec 编译为不可变执行计划，再由 adapter 生成 Job/VolcanoJob、ConfigMap、Secret reference、PVC/ephemeral volume 和 NetworkPolicy。生成必须确定性，所有对象带 tenant/project/job/attempt 标签但 Prometheus 不使用高基数 ID。

## 2. 硬件矩阵

- NVIDIA：官方 device plugin/GPU Operator，CUDA、NCCL；共享场景由 HAMi 明确启用。
- AMD：AMD GPU Operator/device plugin，ROCm、RCCL。
- Ascend：Ascend device plugin/Operator，CANN、HCCL。

三类 worker 镜像、node pool、runtime class 和调度标签分离。缺少匹配节点时保持 Pending 并输出可诊断原因，不降级到 CPU。

## 3. 任务

- [ ] `SCHED-001` 实现 queue、priority、tenant/project quota、公平性和抢占策略。
- [ ] `SCHED-002` 实现 Volcano gang、PodGroup、minAvailable 和 topology policy。
- [ ] `SCHED-003` 实现三类 accelerator capability normalizer 和污点/亲和性。
- [ ] `SCHED-004` 验证镜像 digest、资源上限、允许命令和安全上下文。
- [ ] `SCHED-005` watcher 使用 resourceVersion 恢复；重复事件由 revision 幂等处理。
- [ ] `SCHED-006` 测试配额竞争、无设备、节点 drain、抢占、Pod 驱逐和 API server 中断。

完成条件：相同 spec 产生相同计划；调度失败能定位到配额、能力、拓扑或策略。
