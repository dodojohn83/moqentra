# 01. 现状校准与执行契约

## 1. 已确认现状

- 002 的 189 项任务已勾选，Rust workspace 单元测试可通过。
- 控制面业务资源主要保存在进程内 Registry，重启后丢失。
- PostgreSQL 仅有初始少量表，outbox/idempotency 仍主要使用内存实现。
- Python Worker 是生命周期 SDK，尚无真实 gRPC client 和训练模板。
- Node Agent 具备资源分配模型，但未真正启动受控 OCI 训练任务。
- Web 只有 Shell、tenant context、安全工具和 LabelU JSON adapter，尚无业务旅程。
- dyun-agent 管理内存状态，尚未直接驱动 dyun-gu runner。
- 硬件 CI 脚本仍为 placeholder，staged CI 仍是手工触发。

因此 002 的状态解释为“设计/原型完成”，不能解释为 TAS 或 release gate 已通过。

## 2. 任务

- [ ] `R1-GOV-001` 在 `docs/capability-tracking.md` 为能力增加 `designed / implemented / integrated / accepted` 四级状态和证据链接；初始状态按仓库真实实现填写。
- [ ] `R1-GOV-002` 核对 OpenAPI、Proto、JSON Schema、运行路由和数据库表，建立差距清单；每个差距绑定本执行包唯一任务 ID。
- [ ] `R1-GOV-003` 建立 R1 风险登记，至少覆盖 LabelU v5.11.1 不可用、dyun-gu 未发布 tag、k3s kubeconfig 权限、RTX 3090 仅 preview 和真实 RTSP/RTMP 环境。
- [ ] `R1-GOV-004` 为集成测试定义固定环境清单：PostgreSQL、MinIO、Dex、Docker、k3s、Volcano、NVIDIA device plugin、RTX 3090、dyun-gu pinned commit。
- [ ] `R1-GOV-005` 建立 `artifacts/r1-evidence/<build-id>/` 或等价 CI artifact 约定，统一保存版本、命令、JUnit、日志、摘要、媒体输出和故障注入结果；仓库不提交大体积证据。
- [ ] `R1-GOV-006` 把 `.github/workflows/ci-staged.yml` 从“空骨架手工任务”改为按变更路径自动触发，并建立 required checks 清单。

## 3. 状态推进规则

| 状态 | 最低证据 |
|---|---|
| designed | 权威契约、领域不变量和 ADR 已评审 |
| implemented | 生产代码存在，单元测试通过，无 placeholder 成功路径 |
| integrated | 与真实相邻系统完成契约/集成测试 |
| accepted | 对应 TAS、故障和安全场景通过，证据进入发布包 |

状态只允许逐级推进。已接受能力发生契约破坏、依赖升级或测试失效时必须降级并重新验证。

## 4. 完成条件

- 任何开发者能从追踪矩阵定位能力、任务、代码、测试和证据。
- R1 阻塞项具有 owner、解除条件和失败门禁，不使用“后续补充”关闭任务。
- 002 文档保持历史事实，不回写或伪造其完成证据。
