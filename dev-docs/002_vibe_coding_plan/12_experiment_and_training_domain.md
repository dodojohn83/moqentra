# 12. 实验与训练领域

## 1. 权威模型

Experiment 组织多次 TrainingJob。TrainingJobSpec 固定代码/镜像 digest、数据版本、超参、seed、资源、分布式配置、输入、checkpoint 和输出契约；创建后不可原地修改。

Job 状态：Queued → Admitted → Starting → Running → Finalizing → Succeeded；任一非终态可进入 Cancelling → Cancelled、Failed 或 TimedOut。每次调度创建 Attempt；只有当前 fencing token 可更新 Job。

## 2. 任务

- [ ] `TRAIN-001` 实现 Experiment、Job、Attempt、Rank、Checkpoint 和 Metric 状态机。
- [ ] `TRAIN-002` 定义资源请求：replica、cpu、memory、ephemeral storage、accelerator kind/count、topology。
- [ ] `TRAIN-003` 定义参数 schema，命令使用 argv 数组，禁止拼接 shell。
- [ ] `TRAIN-004` 实现创建、排队、取消、重试、克隆和恢复 application service。
- [ ] `TRAIN-005` 以批次写入 metric，限制 series/cardinality/频率并支持下采样。
- [ ] `TRAIN-006` Finalizing 校验 manifest、digest、指标和 checkpoint 后才可成功。
- [ ] `TRAIN-007` 测试重复派发、旧 attempt 回报、取消竞争、deadline 和部分 rank 失败。

完成条件：API 超时不等于取消；worker ack 不等于训练成功；任何成功任务均有完整可验证产物。
