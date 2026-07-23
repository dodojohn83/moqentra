# 05. 多节点 PyTorch DDP 与弹性恢复

## 1. 固定运行模型

- 每个 replica 对应一个 Worker Pod/节点。
- 每个 Pod 启动 `processesPerReplica` 个 rank，并与本地整卡设备一一绑定。
- `worldSize = replicas × processesPerReplica`。
- R2 GPU backend 固定为 NCCL；CPU 合约测试使用 Gloo。
- rendezvous 使用 `torchrun --rdzv_backend=c10d` 和稳定 headless service。
- 任一 rank/node 失败时终止整个 gang，以新 attempt 和 fencing token 恢复；残留 rank 不得继续写产物。

## 2. Launcher 与环境

- [ ] `R2-DDP-001` 将 canonical distributed spec 编译为 torchrun argv，禁止 shell 拼接；记录 nnodes、nproc_per_node、rdzv_id、endpoint、backend 和 max_restarts。
- [ ] `R2-DDP-002` 为每个 rank 设置确定性 RANK、LOCAL_RANK、WORLD_SIZE、MASTER_ADDR/PORT、attempt 和 fencing；用户 env 不能覆盖保留字段。
- [ ] `R2-DDP-003` NCCL interface、timeout、debug 和 topology 由受控 runtime profile 注入；不把 host secret 或任意 NCCL 参数暴露给普通用户。
- [ ] `R2-DDP-004` Pod 使用稳定 DNS 和 NetworkPolicy，仅允许 gang、对象存储、控制面和必要 DNS 通信。
- [ ] `R2-DDP-005` 启动前验证全部节点 driver/runtime/framework/NCCL 和 model template compatibility；混合不兼容节点直接拒绝。

## 3. Rank 与 attempt 状态

- [ ] `R2-DDP-006` 持久化 global/local rank、node、pod、device、heartbeat、exit code、restart count 和 observed generation。
- [ ] `R2-DDP-007` coordinator 只有在全部 rank 完成 rendezvous 后把 attempt 标记 Running；部分加入最终超时失败。
- [ ] `R2-DDP-008` rank progress、metrics 和日志携带 rank；聚合指标明确 reduction 规则，原始数据保留可追踪来源。
- [ ] `R2-DDP-009` 任一 rank 非零退出触发 gang failure；旧 fencing token 的 late Result、metric、checkpoint 或 success 被拒绝。
- [ ] `R2-DDP-010` 用户取消、deadline、preempt 和基础设施丢失映射到不同终态和 retry eligibility。

## 4. 恢复策略

- [ ] `R2-DDP-011` failure classifier 区分 user code、data、OOM、device、node、network、scheduler 和 platform failure，决定是否自动重试。
- [ ] `R2-DDP-012` 新 attempt 选择最近兼容 complete checkpoint；不存在时按 policy 从头开始或终止，不使用半完成文件。
- [ ] `R2-DDP-013` 自动恢复次数、总 attempt 次数和退避分别受限；重试耗尽后进入 Failed 并保留诊断。
- [ ] `R2-DDP-014` world size 默认不变；只有 template 与 checkpoint manifest 都声明支持时允许改变，且必须重新校验 optimizer/shard layout。
- [ ] `R2-DDP-015` scheduler/control-plane 重启后根据 PostgreSQL 和 Kubernetes observed state 恢复 attempt，不重复启动 gang。

## 5. 硬件验收

- [ ] `R2-DDP-016` 准备至少两个 Kubernetes 节点、每节点至少一张 NVIDIA 数据中心 GPU，冻结 driver、CUDA、NCCL、PyTorch、网络和镜像 digest。
- [ ] `R2-DDP-017` ResNet18 完成两节点 DDP 正常训练，指标与单机基线在预先记录容差内。
- [ ] `R2-DDP-018` SSDlite 完成多节点 smoke、指标聚合、最终 checkpoint 和模型注册。
- [ ] `R2-DDP-019` 分别终止 rank 0、非 rank 0、整个节点和 scheduler leader，验证整 gang 收敛及恢复。
- [ ] `R2-DDP-020` 记录吞吐、GPU 利用率、网络、rendezvous、checkpoint 和恢复时间，建立后续候选版本回归基线。

## 6. 完成条件

- 没有 NVIDIA 多节点真实证据时，本章不能标记 accepted。
- 任一失败场景不会生成重复 ModelVersion、双重 quota 结算或多个 active attempt。
- 所有 rank 只从已验证的 DatasetManifest 读取数据，并且分布式 sampler 不重复/遗漏样本超出模板定义。
