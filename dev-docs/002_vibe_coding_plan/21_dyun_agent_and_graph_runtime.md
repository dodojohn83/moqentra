# 21. dyun-agent 与图运行时

## 1. 边界

dyun-gu 保持独立版本。`dyun-agent` 是节点守护进程，负责能力、artifact、desired state、runner 隔离和 mTLS gRPC；每个 deployment replica 由独立 runner 进程/容器链接固定 commit 的 `dg-*` crates，避免厂商 SDK 崩溃影响代理。

`DyunGraphBundle/v1` 包含平台 application digest、规范化 `dg/v1 GraphSpec`、artifact bindings、runtime profile、资源限制、签名和兼容版本。agent 先验签、校验、预拉 artifact，再原子启动。

## 2. 状态与操作

Replica：Pending → Preparing → Starting → Running → Draining → Stopped；可 Failed。更新采用 generation 与 fencing token；旧 generation 的状态和日志不得覆盖新部署。

## 3. 任务

- [ ] `DYUN-001` 实现 agent Connect、能力清单、heartbeat、credit、drain 和 fencing。
- [ ] `DYUN-002` 实现 bundle 验签、dg schema/element preflight、路径安全和 artifact cache。
- [ ] `DYUN-003` 实现 runner supervisor、进程隔离、限额、健康、停止和孤儿清理。
- [ ] `DYUN-004` 映射 `RunningGraph` status、metrics、reload 和 failure diagnostics。
- [ ] `DYUN-005` 热更新先 validate/diff；失败保留上一 generation，破坏性更新走 drain/restart。
- [ ] `DYUN-006` 验证 RTSP→decode→detect→track→OSD→encode→RTMP 真实链路及 copy report。
- [ ] `DYUN-007` 测试 agent/runner 强杀、磁盘满、坏模型、签名错误、断网和重复 deploy。

完成条件：控制面不执行 dg CLI shell；基本集成无需修改 dyun-gu 上游源码。
