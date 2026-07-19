# 22. 推理平台分层与联动

## 1. 分层

训练开放平台权威管理模型和应用版本；推理平台权威管理 runtime cluster、endpoint、deployment、replica、流 binding 和实时 SLO。两者通过版本化发布契约联动，禁止共享内部数据库表。

发布流程：Promotion → ReleaseBundle → inference admission → staged rollout → observation → complete/rollback。ReleaseBundle 固定模型、ApplicationVersion、dyun bundle、策略和签名。

## 2. 任务

- [ ] `INFER-001` 定义 inference northbound port、ReleaseBundle 和状态回传事件。
- [ ] `INFER-002` 定义 cluster/zone/node/capability/endpoint 模型和租约。
- [ ] `INFER-003` 实现滚动、蓝绿、金丝雀、暂停、继续和自动回滚。
- [ ] `INFER-004` 实现 replica desired/observed state、generation、fencing 和对账。
- [ ] `INFER-005` 实现多集群 placement、数据地域、容量、亲和性和故障域策略。
- [ ] `INFER-006` 定义训练平台不可用时推理平台的独立运行与缓存契约。
- [ ] `INFER-007` 测试事件重复/乱序、集群断连、部分发布、回滚和版本不兼容。

完成条件：两个平台可独立升级；推理持续运行不依赖训练控制面在线。
