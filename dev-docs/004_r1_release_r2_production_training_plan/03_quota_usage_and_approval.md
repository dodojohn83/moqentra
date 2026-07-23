# 03. 配额、用量账本与审批

## 1. 配额模型

- [ ] `R2-QUOTA-001` 支持 tenant 默认策略和 project override，限制并发训练/转换、设备数、GPU/NPU 小时、存储字节和队列深度。
- [ ] `R2-QUOTA-002` policy 使用 revision 和 effective period；策略变更不追溯改变已创建 reservation。
- [ ] `R2-QUOTA-003` 提交训练时按 `accelerator count × replicas × max duration` 原子创建 reservation；无可用额度时不进入队列。
- [ ] `R2-QUOTA-004` 实际 start/stop、设备绑定和 Artifact 写入生成不可变 usage ledger；失败重放以 source event id 去重。
- [ ] `R2-QUOTA-005` job 终结后结算实际用量并释放未使用 reservation；lost job 由 reconciler 在 lease 过期后结算。
- [ ] `R2-QUOTA-006` 存储用量按租户对象引用计费，去重对象只计一次物理占用，同时保留各项目逻辑引用统计。
- [ ] `R2-QUOTA-007` 每日/monthly rollup 可从原始 ledger 重建；账本与 rollup 差异触发告警而非静默修正。

## 2. 审批边界

- [ ] `R2-APPROVAL-001` 配额超限、模型发布和 production deployment 创建 ApprovalRequest，保存发起时的资源、策略和风险快照。
- [ ] `R2-APPROVAL-002` 申请人不能审批自己的请求；审批者必须在相同 tenant/project 范围拥有明确权限。
- [ ] `R2-APPROVAL-003` 决定包含 approve/reject、理由、有效期、限制值和 decision revision；决定不可修改，只能撤销或新建请求。
- [ ] `R2-APPROVAL-004` 审批通过后创建有作用域和到期时间的 override，不永久修改基础 quota policy。
- [ ] `R2-APPROVAL-005` 重复审批、过期批准、策略变更和资源 revision 变化必须拒绝或要求重新申请。
- [ ] `R2-APPROVAL-006` 每次申请、查看敏感详情、决定、撤销和 override 使用都写入企业审计。

## 3. 调度集成

- [ ] `R2-QUOTA-008` admission 在一个 PostgreSQL 事务内校验 policy、reservation、approval 和 job revision，防止并发超卖。
- [ ] `R2-QUOTA-009` scheduler 只领取状态为 admitted 且 reservation 有效的 job；reservation 过期时回到明确 blocked 状态。
- [ ] `R2-QUOTA-010` 抢占不立即返还用量；被抢占 workload 确认停止后才释放设备 reservation。
- [ ] `R2-QUOTA-011` queue、reservation 和 usage 的 desired/observed 差异由有界 reconciler 修复并产生审计事件。

## 4. 完成条件与测试

- 并发提交、重复请求、数据库重试、scheduler 重启和消息重放不造成额度超卖或双重计费。
- 租户 A 无法查询、审批或推断租户 B 的 policy、reservation、usage 和审批理由。
- usage ledger 能从实际 workload/Artifact 事件重建，rollup 与原始账本一致。
- 超额 override 到期后新任务被拒绝，已有任务按批准时 snapshot 继续运行。
