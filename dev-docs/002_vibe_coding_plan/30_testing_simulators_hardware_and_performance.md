# 30. 测试、模拟器、硬件与性能

## 1. 分层

1. 单元/属性：类型、状态机、编译器、解析器。
2. crate/package 集成：真实 PostgreSQL、MinIO、OIDC。
3. 契约：REST、Proto、agent、repository、local/Kubernetes executor。
4. 系统：完整单机和三节点集群。
5. 硬件/性能/耐久/故障注入：固定实验环境。

注入 Clock、ID、random seed、fault policy；测试不得依赖公网、公共端口或执行顺序。失败输出 seed 和环境 manifest。

## 2. 工具

建立 fake worker、fake dyun-agent、Kubernetes event simulator、S3 fault proxy、OIDC test issuer 和媒体流 fixture。真实样本必须脱敏；模型与数据大文件放受控测试存储，不提交 Git。

## 3. 核心场景

- [ ] `TST-001` 多租户相同名称/外部 ID 的全链路隔离与 RLS。
- [ ] `TST-002` 数据上传→标注→审核→训练→模型→编排→dyun 部署闭环。
- [ ] `TST-003` local 与 Kubernetes executor 对同一 spec 的状态/产物一致。
- [ ] `TST-004` worker/agent/control-plane 强杀、消息重复乱序、DB/S3/K8s 短时故障。
- [ ] `TST-005` NVIDIA CUDA/NCCL、AMD ROCm/RCCL、Ascend CANN/HCCL 单机与双节点。
- [ ] `TST-006` 编排 compiler deterministic、GraphSpec hot reload 和真实 RTSP/RTMP。
- [ ] `TST-007` fuzz 配置、manifest、Proto、cursor、archive、模型元数据和桌面 IPC。

## 4. 性能门槛

报告记录 commit、工具链、硬件、驱动、配置、数据规模、预热和持续时间。测量上传吞吐、标注首屏、API P95/P99、调度时延、训练启动、metric 写入、artifact 下载、部署收敛、视频端到端时延和资源使用。开发运行 24 小时、发布候选运行 72 小时；所有队列和缓存必须有稳定上限。

完成条件：性能退化超过冻结阈值阻止发布；真实硬件结果不得由 mock 或 compile-only 替代。
