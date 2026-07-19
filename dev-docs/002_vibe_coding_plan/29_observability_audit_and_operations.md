# 29. 可观测性、审计与运维

## 1. 遥测

统一 OpenTelemetry。日志字段包括 tenant/project/request/operation/job/attempt/deployment/node；secret、签名 URL、原始图片帧和训练参数中的敏感值禁止输出。高基数业务 ID 不作为 Prometheus label。

指标至少覆盖 API、DB/S3、outbox、队列/配额、训练状态和时延、worker/agent lease、checkpoint、artifact、部署 replica、dyun element 队列/拷贝、reconciler backlog。

## 2. 任务

- [ ] `OBS-001` 实现 HTTP/gRPC/outbox/agent trace context 传播和结构化日志。
- [ ] `OBS-002` 建立指标命名、单位、基数预算、dashboard 和 recording rules。
- [ ] `OBS-003` 实现不可变审计查询、导出、保留、legal hold 和完整性校验。
- [ ] `OPS-001` 提供配置/DB/对象/节点/任务/部署诊断 CLI。
- [ ] `OPS-002` 编写数据库、对象存储、队列积压、节点故障、证书、磁盘和训练风暴 runbook。
- [ ] `OPS-003` 生成脱敏诊断包并自动检查凭据、URL 和用户数据。

完成条件：故障演练能仅依据指标、日志、trace、审计和 runbook 定位；遥测后端故障不阻塞业务。
