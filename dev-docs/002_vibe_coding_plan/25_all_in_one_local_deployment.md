# 25. 单机一体化部署

## 1. 交付形态

单机包使用 OCI Compose 为默认路径：control-plane、scheduler、web、PostgreSQL、MinIO、OIDC、node-agent 和 dyun-agent。训练/推理 worker 仍运行独立容器。可选单节点 Kubernetes 用于与生产环境完全同构的验证。

## 2. 任务

- [ ] `ONEBOX-001` 生成配置向导、端口/磁盘/设备/驱动 preflight。
- [ ] `ONEBOX-002` 实现首次初始化、管理员创建、证书和 secret 生成。
- [ ] `ONEBOX-003` 提供在线/离线镜像包、checksum、SBOM 和签名验证。
- [ ] `ONEBOX-004` 实现备份、恢复、升级、回滚和数据目录迁移命令。
- [ ] `ONEBOX-005` GPU/NPU runtime profile 按能力显式启用，缺失时给出诊断。
- [ ] `ONEBOX-006` 在干净 x86_64/aarch64 主机执行一键安装和完整视觉闭环。

完成条件：单机 API/spec 与集群完全一致；卸载默认保留数据且不会删除外部目录。
