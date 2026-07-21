# 05. 单节点 Kubernetes 与 Volcano 执行

## 1. 固定部署基线

- Kubernetes 使用基线矩阵允许版本；当前开发验证为 k3s v1.33.x。
- NVIDIA Container Toolkit 与 device plugin 暴露真实 GPU；平台不实现 GPU 虚拟化。
- 单副本训练可编译为 Kubernetes Job；需要 gang 语义或显式 Volcano profile 时编译为 VolcanoJob。
- 领域层只使用 `ResourceClass`，`nvidia.com/gpu`、node selector、taint/toleration 和 runtimeClass 映射留在 scheduler adapter。

## 2. 任务

- [ ] `R1-K8S-001` 实现 Kubernetes client adapter 和版本/capability discovery；启动时验证 CRD、权限、namespace、storage 和 device resource。
- [ ] `R1-K8S-002` 把同一 `TrainingJobSpec/v1` 确定性编译为 Job/VolcanoJob，包含 image digest、argv、资源、deadline、restart policy、labels 和 owner reference。
- [ ] `R1-K8S-003` 为每个 tenant/project/attempt 使用受控 namespace 或标签/策略边界；创建最小 ServiceAccount、RBAC、NetworkPolicy 和 Pod Security 设置。
- [ ] `R1-K8S-004` 数据与产物通过短期凭据或 init/sidecar 物化，不把长期 S3 密钥写入 JobSpec、Pod env dump 或日志。
- [ ] `R1-K8S-005` watch 使用 resourceVersion 恢复、超时重连和分页 list；410 Gone 时安全 relist，重复事件保持幂等。
- [ ] `R1-K8S-006` 将 Pod pending/running/succeeded/failed/evicted 和 termination reason 归一化为 attempt 状态；保留安全的原始诊断供 operator 查看。
- [ ] `R1-K8S-007` 取消先更新 desired state，再删除/终止 workload；处理 API timeout、重复 delete 和控制面重启。
- [ ] `R1-K8S-008` 对账器使用 generation、lease 和 ownership labels 回收孤儿；禁止删除不属于本平台或仍有有效租约的 workload。
- [ ] `R1-K8S-009` 在单节点 k3s 上分别完成 Kubernetes Job smoke test 和 VolcanoJob NVIDIA 检测训练。

## 3. 一致性要求

本地 OCI 与 Kubernetes 执行必须产生相同的：

- Job/Attempt 状态和错误分类；
- 日志、指标和 checkpoint cursor；
- 输入 DatasetManifest 与输出 ModelArtifactManifest；
- 取消、deadline、重试和 fencing 语义；
- 审计事件和模型血缘字段。

执行器特有的 Pod、namespace、container ID 等只能进入 execution environment snapshot，不得改变领域资源语义。

## 4. 完成条件与测试

- 相同检测 JobSpec 在本地 OCI 和 k3s/Volcano 均成功，输出 manifest schema 与 lineage 等价。
- 测试 unschedulable、image pull、GPU 不可用、Pod eviction、watch 断线、重复事件和删除超时。
- 无 kubeconfig 权限时 readiness 明确失败并给出操作诊断，不能回退为假成功或静默使用本地执行器。
- R1 只声明单节点 Kubernetes；多节点 rendezvous、NCCL 和 checkpoint recovery 保留为 R2。
