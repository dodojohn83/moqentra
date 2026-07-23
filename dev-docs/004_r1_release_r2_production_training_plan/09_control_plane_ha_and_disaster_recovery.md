# 09. 控制面 HA、Agent 会话与灾备

## 1. Control-plane 与后台单元

- [ ] `R2-HA-001` Kubernetes production profile 运行至少两个 control-plane replica；请求不依赖本地 cache、channel 或连接内权威状态。
- [ ] `R2-HA-002` outbox、Operation、artifact/training/deployment reconciler 和 GC 使用 `SKIP LOCKED`、lease、batch、deadline、retry budget 和 dead-letter。
- [ ] `R2-HA-003` leader-only 工作使用 PostgreSQL lease 和数据库时间，lease 包含 owner、epoch、expiry 和 monotonic fencing。
- [ ] `R2-HA-004` scheduler 失去 lease 后立即停止外部 mutation；旧 epoch 的 Kubernetes/Agent 结果不能修改新状态。
- [ ] `R2-HA-005` readiness 检查 PostgreSQL、对象存储、migration/contract compatibility 和必要 background workers；liveness 不依赖可恢复的外部短暂故障。

## 2. Agent/Worker 会话 HA

- [ ] `R2-SESSION-001` 持久化 agent session、connection owner、lease、last received/sent/acked sequence 和 capability snapshot。
- [ ] `R2-SESSION-002` command 先持久化再由当前 connection owner 领取发送；ack/result 使用 command id 与 sequence 幂等。
- [ ] `R2-SESSION-003` Agent 重连任意 control-plane replica 后提交 resume cursor，服务端重发未确认命令而不重复已完成副作用。
- [ ] `R2-SESSION-004` 同一 Agent 双连接使用 session epoch/fencing 只保留新连接；旧连接消息被拒绝。
- [ ] `R2-SESSION-005` drain、证书轮换和滚动升级等待有界时间；超时 command 留在持久队列供新连接恢复。

## 3. 外部 HA 和备份

- [ ] `R2-DR-001` production values 强制外部 HA PostgreSQL、版本化对象存储和企业 OIDC；Onebox 明确标记非 HA。
- [ ] `R2-DR-002` PostgreSQL 使用持续归档/PITR 或等价托管能力，备份间隔满足 RPO ≤ 15 分钟。
- [ ] `R2-DR-003` 对象存储启用版本/复制或等价保护，数据库备份保存对应对象 inventory/checkpoint。
- [ ] `R2-DR-004` 恢复流程先恢复 PostgreSQL，再验证 object inventory、manifest、引用、RLS 和审计链，最后开放写流量。
- [ ] `R2-DR-005` 灾备环境重新签发 service identity，不复制失效私钥；SecretRef 在目标环境重新绑定。

## 4. 运维目标

- 控制面月可用性：99.9%。
- 元数据和对象 RPO：不超过 15 分钟。
- 服务 RTO：不超过 60 分钟。
- 审计保留：365 天；运行日志热存储默认 30 天。

- [ ] `R2-DR-006` dashboard 和 SLO burn alert 计算可用性、错误预算、RPO lag、backup age 和 restore readiness。
- [ ] `R2-DR-007` runbook 覆盖 replica 故障、scheduler leader、数据库 failover、对象存储故障、证书轮换和完整灾备恢复。
- [ ] `R2-DR-008` 非开发人员按 runbook 完成恢复并记录 RPO/RTO、数据差异和人工步骤。

## 5. 完成条件与测试

- 滚动终止 control-plane replicas 不丢失未确认 command、Operation 或 SSE 事件。
- scheduler leader 切换不重复创建/删除 workload，旧 leader 无法写入新 epoch。
- PostgreSQL 和对象存储恢复满足目标，所有引用与摘要一致。
- 72 小时 burn 中没有永久 lease、孤儿 workload、重复模型版本或无界 backlog。
