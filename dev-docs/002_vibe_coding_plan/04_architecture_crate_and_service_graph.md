# 04. 架构、Crate 与服务边界

## 1. 控制面

首版采用 Rust 模块化单体。HTTP 与 gRPC 入口调用 application service；domain 保存状态机；repository、S3、Kubernetes、worker 和 dyun 都是端口适配器。scheduler 可独立进程运行相同 application crate。

## 2. 依赖规则

- `types` 无项目内部依赖；`contracts` 只依赖 types 与生成运行库。
- `domain` 不依赖 transport/storage/runtime。
- `application` 只声明 repository、clock、id、event、executor、artifact 和 policy ports。
- adapter 之间禁止互相调用；跨能力通过 application service 或持久化事件。
- Web 不直接访问 S3、数据库、Kubernetes、worker 或 dyun。

## 3. 后台执行单元

`OutboxRelay`、`JobDispatcher`、`TrainingReconciler`、`ArtifactReconciler`、`DeploymentReconciler`、`GarbageCollector` 均必须分页、有界、可取消、幂等，并使用 revision/fencing。

## 4. 任务

- [ ] `ARCH-001` 创建 crate graph 检查并禁止反向依赖。
- [ ] `ARCH-002` 定义 control-plane、scheduler、node-agent、dyun-agent 的进程职责。
- [ ] `ARCH-003` 定义同步 API、异步 Operation 和事件的使用边界。
- [ ] `ARCH-004` 为每个外部系统定义 port、超时、重试和错误映射。
- [ ] `ARCH-005` 建立 ADR 模板；跨边界变化必须先更新 ADR。

完成条件：领域测试无需网络、数据库、Tokio handle 或厂商 SDK即可执行。
