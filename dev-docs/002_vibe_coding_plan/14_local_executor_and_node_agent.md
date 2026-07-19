# 14. 单机执行器与训练节点代理

## 1. 目标

单机模式使用与集群相同的 TrainingJobSpec、worker 镜像和状态机。`node-agent` 负责能力探测、资源锁、OCI 运行、日志转发、缓存和进程回收，不在宿主机直接执行用户 shell。

## 2. 调度

资源分配写入持久化 allocation，包含 device UUID、memory、CPU set、attempt 和 fencing token。NVIDIA/AMD/Ascend 通过独立 runtime profile 暴露设备；不允许多个互斥任务静默占用同一设备。

## 3. 任务

- [ ] `LOCAL-001` 探测 CPU、RAM、磁盘、设备、驱动、容器 runtime 和健康。
- [ ] `LOCAL-002` 实现有界本地队列、资源 admission、原子 allocation 和释放。
- [ ] `LOCAL-003` 以非 root、只读 rootfs、capability drop 和 seccomp 启动容器。
- [ ] `LOCAL-004` 实现镜像/model/dataset cache，按 digest 校验和 LRU 配额回收。
- [ ] `LOCAL-005` agent 重启后对账数据库、容器和 allocation，清理孤儿。
- [ ] `LOCAL-006` 建立 local executor 与 Kubernetes executor 的行为契约测试。

完成条件：宿主机重启后任务进入明确恢复或失败状态；单机结果可被集群模式无损读取。
