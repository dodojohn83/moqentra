# Moqentra R1 发布收口与 R2 Production Training 执行包

本目录承接 `dev-docs/003_r1_vertical_slice_plan`。执行顺序严格遵守 `docs/product-scope.md`：R1 必须先形成真实发布证据，之后才能把多节点训练、企业配额、异构转换和高可用标记为 R2 能力。

## 1. 交付目标

本执行包交付两个连续但不可混淆的结果：

1. **Gate 0 / v0.1.0**：完成 R1 真实环境验收，消除生产路径的内存权威和 best-effort 写穿，发布可恢复的视觉 MVP。
2. **R2 / v0.2.0**：完成多租户生产训练，包括 NVIDIA 多节点 DDP、检查点恢复、配额审批、公平调度、转换矩阵、企业审计、控制面 HA 和灾备。

R2 的训练主链路为：

```text
TrainingJobSpec → admission + quota reservation + approval
→ weighted queue → Volcano gang → torchrun DDP
→ sharded checkpoint → failure recovery → usage settlement
→ model artifact → conversion/evaluation → promotion approval
```

## 2. 章节与依赖

| 波次 | 章节 | 交付结果 | 前置条件 |
|---|---|---|---|
| G0 | [01 R1 出口](01_r1_release_exit_gate.md) | `v0.1.0` 与完整证据 | 003 已实现项 |
| R2-A | [02 契约与迁移](02_contracts_adrs_and_migrations.md) | 向后兼容的 R2 契约 | G0；ADR 可提前评审 |
| R2-B | [03 配额与审批](03_quota_usage_and_approval.md) | 无超卖的企业资源治理 | R2-A |
| R2-C | [04 队列与异构调度](04_queue_volcano_and_resource_classes.md) | 公平、可对账的集群调度 | R2-A、R2-B |
| R2-D | [05 多节点 DDP](05_multinode_ddp_and_elastic_recovery.md) | NVIDIA 多节点训练 | R2-C |
| R2-D | [06 分布式检查点](06_distributed_checkpointing.md) | 一致、可恢复的 checkpoint | R2-A、R2-D 协议 |
| R2-E | [07 异构 Worker](07_heterogeneous_workers.md) | 分层硬件支持矩阵 | R2-C |
| R2-F | [08 转换与晋级](08_conversion_evaluation_and_promotion.md) | 可追溯的转换矩阵 | R2-A、R2-E |
| R2-G | [09 HA 与灾备](09_control_plane_ha_and_disaster_recovery.md) | 99.9% 生产 profile | R2-A–R2-D |
| 全程 | [10 企业安全与可观测](10_enterprise_security_audit_observability.md) | 可审计、可诊断、安全闭环 | 随各波次实施 |
| R2-H | [11 Web 管理面](11_web_operations_console.md) | 配额、审批、rank、HA 运维 UI | 对应后端契约冻结 |
| R2-I | [12 验收与发布](12_performance_chaos_and_release.md) | `v0.2.0` release gate | 全部章节 |

Gate 0 未通过时，可以评审 ADR 和契约草案，但不得合并依赖未验收 R1 行为的生产实现，也不得将 R2 能力标记为 `integrated` 或 `accepted`。

## 3. 全局执行契约

- 本目录所有 `[ ]` 均为开放任务；只在实现、测试、证据和文档同时完成后改为 `[x]`。
- PostgreSQL 是状态权威，S3/MinIO 是大文件权威；内存实现仅用于测试或显式 demo profile。
- 领域层不依赖 Kubernetes、Volcano、HAMi、PyTorch、厂商 SDK、SQLx 或 transport。
- 所有异步操作必须有 Operation、deadline、取消、幂等、租约、fencing、重试和恢复。
- 所有硬件和转换支持等级由真实证据推进；mock、交叉编译和模拟器不能升级支持等级。
- 数据库迁移从 `0018` 起只追加；先 expand、再双读写、最后在后续 minor 收缩。
- R1 REST/Proto/JSON Schema reader 至少兼容两个 minor；未知能力不得静默降级执行。
- 生产依赖和镜像固定版本/digest，并附 SBOM、provenance、许可证和签名。

## 4. 任务完成证据

```text
任务：R2-<DOMAIN>-NNN
提交：<commit / PR>
环境：<集群、节点、硬件、驱动、runtime>
命令：<可重复执行命令>
结果：<通过数、指标、耗时、故障注入结论>
证据：<CI artifact / report / digest>
支持等级：<supported / preview / compile-only / mock>
限制：<未关闭 blocker 与影响>
```

## 5. R2 发布硬门禁

- R1 Gate 0 和 `v0.1.0` 发布完成。
- 至少两个 Kubernetes 节点、每节点至少一张 NVIDIA 数据中心 GPU 完成 DDP、rank/node 故障和 checkpoint 恢复。
- 配额预留、用量结算、审批、队列公平性和抢占不发生跨租户读取或资源超卖。
- 控制面多副本、scheduler leader 切换和 Agent 重连不丢失命令或重复创建 workload。
- PostgreSQL/对象存储恢复满足 RPO ≤ 15 分钟、RTO ≤ 60 分钟。
- 跨租户渗透、供应链、72 小时 burn、N→N+1 升级和代码回滚通过。

AMD/Ascend 没有真实 runner 时保持 `compile-only`，单独阻止对应 `preview` 声明，但不阻止其他已满足门禁的 R2 能力发布。

## 6. 范围边界

R2 不实现 R3 的多集群推理、推理 canary、独立推理控制面，也不实现 R4 的 HPO、Notebook、Partner SDK 和桌面客户端。Onebox 保持单机开发/演示形态，R2 HA 承诺只适用于 Kubernetes production profile。
