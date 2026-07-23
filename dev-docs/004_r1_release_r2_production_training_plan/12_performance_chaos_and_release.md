# 12. 性能、混沌、安全与 R2 发布

## 1. 自动质量门禁

- [ ] `R2-QA-001` Rust fmt/clippy `-D warnings`/nextest，包含 PostgreSQL、Kubernetes 和 gRPC feature 组合。
- [ ] `R2-QA-002` Buf/OpenAPI/JSON Schema lint、golden、generated diff 和对 `v0.1.0` breaking comparison。
- [ ] `R2-QA-003` Web lint/typecheck/Vitest/Playwright/production build/bundle secret scan。
- [ ] `R2-QA-004` Python Ruff/mypy/pytest/wheel、DDP launcher、checkpoint 和 converter contract tests。
- [ ] `R2-QA-005` cargo-deny、npm/pip/container/model license/vulnerability、SBOM、provenance 和 signature policy。
- [ ] `R2-QA-006` 真实 PostgreSQL、MinIO、OIDC、mTLS、Kubernetes、Volcano、NVIDIA 和转换环境 integration suites。

## 2. 配额、调度与安全验收

- [ ] `R2-E2E-001` 多租户并发提交验证 reservation 原子性、审批、结算、失败补偿和无超卖。
- [ ] `R2-E2E-002` 10 租户×100 排队任务验证权重、公平、aging、priority、无饥饿和重启后稳定决策。
- [ ] `R2-E2E-003` 抢占 checkpointable 低优先级 job，验证 checkpoint、终止确认、reservation 释放和高优任务启动。
- [ ] `R2-E2E-004` 渗透测试覆盖 quota/approval/usage/queue/audit/RLS/S3/log/metric 和管理员跨租户访问。

## 3. DDP、Checkpoint 与转换验收

- [ ] `R2-E2E-005` 两节点 NVIDIA ResNet18 DDP 成功，保存 topology、环境、吞吐、指标、checkpoint 和模型血缘。
- [ ] `R2-E2E-006` SSDlite 多节点 smoke 成功并完成 ONNX、TensorRT preview 转换与评估。
- [ ] `R2-E2E-007` 分别杀死 rank 0、非 rank 0、整节点，验证 gang 终止、新 fencing 和最新 complete checkpoint 恢复。
- [ ] `R2-E2E-008` 在 shard upload、manifest transaction、complete marker 和 restore 注入故障，半完成 checkpoint 永不被选择。
- [ ] `R2-E2E-009` OpenVINO preview 完成真实加载；AMD/Ascend/OM/RKNN/Sophon 按可用证据保持或提升支持等级。

## 4. HA 与灾备验收

- [ ] `R2-E2E-010` 滚动删除 control-plane replica，Agent/Worker 重连后 command/ack/result 无丢失或重复副作用。
- [ ] `R2-E2E-011` scheduler leader 切换和旧 leader 网络恢复不重复创建/删除 workload。
- [ ] `R2-E2E-012` PostgreSQL failover、对象存储短暂不可用和 outbox backlog 最终收敛。
- [ ] `R2-E2E-013` 从备份恢复独立环境，验证 RPO ≤ 15 分钟、RTO ≤ 60 分钟、对象引用、RLS 和审计链。
- [ ] `R2-E2E-014` 执行 N→N+1 expand-first 升级和旧 R2 代码回滚；R1 client/agent 兼容路径仍通过。
- [ ] `R2-E2E-015` 72 小时 burn 中持续提交训练/转换并注入进程、节点和网络短暂故障，无无界 backlog、永久 lease 或孤儿 workload。

## 5. 性能基线

- [ ] `R2-PERF-001` 同步 API 在 50 RPS、正常数据库条件下 p95 < 300 ms，错误率和连接池等待记录入报告。
- [ ] `R2-PERF-002` 1000 个排队任务时 scheduler 新任务决策 p95 < 5 秒，reconcile 不持有长事务或全表锁。
- [ ] `R2-PERF-003` 建立 DDP 吞吐、GPU 利用率、网络、gang startup、checkpoint 和恢复时间基线。
- [ ] `R2-PERF-004` 后续候选版本相同环境关键指标无批准不得下降超过 10%；环境变化必须创建新基线而非混比。

## 6. 文档、追踪与发布

- [ ] `R2-REL-001` 更新 product scope，明确 NVIDIA 硬门禁和 AMD/Ascend 分层门禁。
- [ ] `R2-REL-002` 将 TAS-028～030 细化为配额审批、DDP/恢复、转换支持等级、HA/DR 和企业审计子场景。
- [ ] `R2-REL-003` capability tracking 增加 004 章节、R2 状态和证据；R1 未 accepted 能力不得自动继承。
- [ ] `R2-REL-004` 更新 blocker：新增多节点 NVIDIA 环境；AMD/Ascend 仅由真实硬件报告关闭。
- [ ] `R2-REL-005` 生成 `v0.2.0` ReleaseManifest、镜像/Artifact digest、SBOM、provenance、签名、支持矩阵和 runbook。
- [ ] `R2-REL-006` ReleaseGate 验证真实报告引用，不接受手工布尔值或 simulator 替代硬件证据。

## 7. 发布裁决

以下任一项失败即阻止 `v0.2.0`：R1 Gate 0、NVIDIA 多节点 DDP/恢复、配额无超卖、跨租户渗透、控制面/leader HA、RPO/RTO、72 小时 burn、升级回滚或供应链签名。

AMD/Ascend 无真实 runner 时保持 compile-only，不阻止其他满足门禁的 R2 能力发布，但 release notes 必须明确限制。
