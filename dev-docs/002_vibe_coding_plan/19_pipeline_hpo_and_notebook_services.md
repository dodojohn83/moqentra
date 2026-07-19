# 19. Pipeline、HPO 与 Notebook

## 1. Pipeline

PipelineSpec 是有向无环任务图，节点引用已注册 task template，边传递 artifact reference 而非本地路径。运行快照固定 spec、参数、镜像和输入；节点级缓存键必须内容寻址。

## 2. HPO 与 Notebook

HPO controller 只生成普通 TrainingJob，搜索算法不得绕过配额。Notebook 是有期限、受网络策略和资源配额约束的开发环境；凭据为短期 token，不挂载控制面服务账号。

## 3. 任务

- [ ] `PIPE-001` 定义 PipelineSpec、DAG 校验、条件、重试、缓存和取消传播。
- [ ] `PIPE-002` 实现 pipeline/node run 状态机和失败恢复。
- [ ] `HPO-001` 实现 search space、trial budget、early stop 和最佳模型选择。
- [ ] `NOTE-001` 实现模板、空闲回收、持久卷、数据只读挂载和受控镜像。
- [ ] `NOTE-002` 限制出站网络、特权、hostPath、镜像来源和资源上限。
- [ ] `PIPE-003` 测试缓存污染、并发取消、配额耗尽、Notebook 过期与逃逸防护。

完成条件：高级服务复用训练、artifact、审计和租户模型，不形成第二套调度系统。
