# 16. 分布式训练、Checkpoint 与恢复

## 1. 分布式模型

支持 PyTorch DDP/Elastic。NVIDIA 使用 NCCL，AMD 使用 RCCL，Ascend 使用 HCCL；同一 job 不混用 accelerator family。rank、world size、master endpoint 和 rendezvous 由执行器注入，不接受用户覆盖保留环境变量。

## 2. 恢复

Checkpoint 先写临时对象，完成后上传 manifest 与 digest，再原子登记。恢复只选择 Compatible 且 Complete 的 checkpoint；代码、模型结构、world size 或 optimizer 不兼容时明确拒绝。

## 3. 任务

- [ ] `DIST-001` 定义 launcher、rendezvous、rank 和网络端口契约。
- [ ] `DIST-002` 实现 gang start、所有 rank ready、全局取消和退出码归一化。
- [ ] `DIST-003` 实现周期/指标触发 checkpoint、保留策略和上传背压。
- [ ] `DIST-004` 实现弹性重启预算、失败分类和 attempt 级恢复。
- [ ] `DIST-005` 收集每 rank 日志/指标并限制乱序、大小和高基数。
- [ ] `DIST-006` 在三类真实硬件分别验证两节点训练、kill rank、节点丢失和恢复精度。

完成条件：单 rank 成功不能使 job 成功；恢复后的最终指标与固定容差基线一致。
