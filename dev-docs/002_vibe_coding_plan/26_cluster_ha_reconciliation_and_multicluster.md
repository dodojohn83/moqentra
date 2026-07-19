# 26. 集群、高可用、对账与多集群

## 1. HA

控制面无本地权威状态，可多副本部署。PostgreSQL 和对象存储由外部 HA 保障。节点、agent、job attempt 和 deployment 使用 lease epoch/fencing；时间裁决使用数据库或 Kubernetes server time。

## 2. Reconciler

顺序固定为 Operation → TrainingJob/Deployment desired state → Attempt/Replica → 外部资源 → Artifact finalization。每个 reconciler 分页、限速、可取消、可重复，使用 revision/CAS，不做全表锁。

## 3. 任务

- [ ] `HA-001` 实现实例注册、readiness、drain、leader election 和兼容版本检查。
- [ ] `HA-002` 实现 job、allocation、artifact、deployment、node 和 outbox reconciler。
- [ ] `HA-003` 清理孤儿 Pod/container/runner/object multipart 和过期 lease。
- [ ] `HA-004` 实现多集群 outbound agent 注册、断线缓存、重新同步和区域策略。
- [ ] `HA-005` 验证滚动升级、数据库切换、API server 中断、网络分区和时钟偏差。

完成条件：旧 owner 恢复后不能提交结果；对账积压、fencing 和孤儿清理均有指标和审计。
