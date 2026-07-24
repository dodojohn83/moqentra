# 04. 公平队列、Volcano 与 ResourceClass

## 1. 权威边界

- PostgreSQL 保存 QueuePolicy、PriorityClass、reservation、job desired state 和 workload binding。
- Volcano Queue/PodGroup/Job 是由 adapter 对账生成的集群投影，不是业务状态权威。
- Kubernetes node/device status 是 observed capacity；平台只保存带时间戳的 capability snapshot。

## 2. 队列与公平性

- [x] `R2-SCHED-001` 建立 tenant/project queue hierarchy、weight、capacity、max running、priority allowlist 和 admission policy。
- [x] `R2-SCHED-002` 排序先比较授权 priority class，再使用等待时间 aging；同 score 按 submitted_at、job_id 稳定排序。
- [x] `R2-SCHED-003` 使用加权 dominant-resource share 选择下一租户，资源维度至少包含 CPU、内存和各 accelerator class。
- [x] `R2-SCHED-004` queue decision 保存 policy revision、resource snapshot、候选集摘要和解释，便于审计与重放。
- [x] `R2-SCHED-005` 防止饥饿：持续有容量时，达到 aging 上限的有效任务必须优先于新提交的同权限任务。
- [x] `R2-SCHED-006` bounded queue、分页和 batch admission；1000 个排队任务不能形成全表锁或无界内存集合。

## 3. ResourceClass 与异构资源

- [x] `R2-RESOURCE-001` ResourceClass 表达 vendor、family、memory、driver/runtime、collective backend、topology、sharing mode 和 support tier。
- [x] `R2-RESOURCE-002` adapter 把 ResourceClass 映射到 device resource、node affinity、taint/toleration、runtimeClass 和 topology constraints。
- [ ] `R2-RESOURCE-003` node capability 过期或不健康时从可调度容量移除；已运行 job 进入 degraded/恢复决策，不静默迁移。
- [x] `R2-RESOURCE-004` 整卡训练默认使用厂商 device plugin；HAMi 仅为显式 shareable preview class 创建资源请求。
- [x] `R2-RESOURCE-005` DDP、NCCL 或需要 P2P/NVLink 的任务禁止使用共享 GPU class。
- [ ] `R2-RESOURCE-006` capacity snapshot 与 reservation 定期对账；外部 workload 占用导致的差异只降低可用容量，不删除未知 workload。

## 4. Volcano 执行

- [x] `R2-VOLCANO-001` 确定性编译 VolcanoJob、PodGroup、service、ConfigMap 和受控 credential refs；所有资源带 tenant/project/job/attempt ownership labels。
- [x] `R2-VOLCANO-002` minAvailable 等于完整 gang；任何 rank 未就绪时训练不开始，deadline 到期进入明确 unschedulable/failed。
- [x] `R2-VOLCANO-003` watch 从持久 resourceVersion 恢复，处理 410/relist、重复/乱序事件、API timeout 和控制面重启。
- [ ] `R2-VOLCANO-004` desired cancel、retry 和 preempt 先持久化，再执行外部 mutation；generation/fencing 防止旧 controller 删除新 workload。
- [ ] `R2-VOLCANO-005` 抢占仅针对允许抢占、优先级更低且可 checkpoint 的 job；checkpoint grace 到期后才能删除 workload。
- [ ] `R2-VOLCANO-006` orphan GC 只处理 ownership、generation 和 lease 都匹配的资源；dry-run 报告必须先于实际删除。

## 5. 完成条件与测试

- 10 个租户、每租户 100 个任务的模拟负载保持权重公平，无永久饥饿，重启后排序可解释。
- quota reservation、queue admit 和 Volcano workload 创建不会在并发场景产生重复 Job/PodGroup。
- NVIDIA 整卡、HAMi 共享 preview 和无匹配 ResourceClass 分别得到正确执行或明确拒绝。
- Kubernetes/Volcano 不可用时 readiness 和 Operation 报告真实原因，不回退本地 executor。
