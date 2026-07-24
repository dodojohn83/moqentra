# 06. 分布式检查点一致性与恢复

## 1. CheckpointManifest

- [x] `R2-CKPT-001` manifest 记录 tenant/project/job/attempt、step、epoch、world size、framework、template、code/image digest 和创建时间。`crates/domain/src/checkpoint_manifest.rs` `CheckpointManifest` 已包含全部字段。
- [x] `R2-CKPT-002` 每个 shard 记录 rank、对象 key、size、SHA-256、tensor/layout 元数据；key 只能由受控 builder 生成。`CheckpointShard` 与 `CheckpointService::shard_key` 受控生成。
- [x] `R2-CKPT-003` compatibility 保存 model、optimizer、AMP scaler、RNG、sampler 和 dataset manifest signature。`CheckpointManifest.compatibility` 为 `BTreeMap<String, String>`，可在 finalize 前填入；`select_for_recovery` 已校验 code/image/dataset 签名。
- [x] `R2-CKPT-004` checkpoint 状态固定为 Uploading → Validating → Complete/Failed；只有 Complete 可用于恢复、下载或模型晋级。`CheckpointState` 枚举与 `select_for_recovery` 的 `Complete` 过滤保证。

## 2. 两阶段完成协议

- [x] `R2-CKPT-005` 所有 rank 先写 attempt/step 临时前缀并上报 shard digest，任何 rank 不直接发布 complete marker。`CheckpointService::report_shard` 仅收集 shard，`finalize` 才写 marker。
- [x] `R2-CKPT-006` coordinator 等待 barrier 和全部预期 shard，校验数量、摘要、size、world size 与 fencing。`finalize` 校验 expected ranks、shard digests/sizes、`world_size`；attempt id 在 `report_shard` 中校验。
- [x] `R2-CKPT-007` validator 成功后在事务中写 manifest/outbox，再写内容寻址 complete marker；重复 finalize 幂等。`finalize` 先检查 marker，再写 manifest 与 marker；第二次调用返回相同 digest。
- [ ] `R2-CKPT-008` 数据库提交或 marker 写入任一失败时保持可重试中间状态，reconciler 根据 digest 恢复，不生成第二份 checkpoint。对象存储操作已幂等；PostgreSQL 事务恢复由后续 storage/reconciler 任务补齐。
- [x] `R2-CKPT-009` 旧 attempt、旧 generation、重复 shard、摘要冲突和缺失 rank 必须拒绝并产生安全诊断。`report_shard` 拒绝重复 rank、越界 rank、非法 digest、非受控 key；`finalize` 拒绝缺失 rank；marker digest 冲突返回 `Error::conflict`。

## 3. PyTorch 状态

- [ ] `R2-CKPT-010` 使用 `torch.distributed.checkpoint` 保存 sharded model/optimizer；单机模板保持兼容的 state_dict 导入路径。待 Python SDK `moqentra_worker` 训练循环实现。
- [ ] `R2-CKPT-011` 保存 AMP scaler、scheduler、RNG、sampler epoch/offset、global step 和 template 自定义状态。待 Python SDK 训练循环实现；compatibility map 已预留键值空间。
- [ ] `R2-CKPT-012` 恢复后用固定 fixture 验证模型输出，并确认 step/epoch/optimizer 不倒退或重复训练已完成 batch。待训练 runtime 与 fixture 任务实现。
- [ ] `R2-CKPT-013` preserve_optimizer=false 的恢复被记录为显式 warm restart，不能伪装成等价续训。待 recovery planner 扩展实现。

## 4. Retention 与 GC

- [ ] `R2-CKPT-014` 默认保留最近 3 个、最佳 1 个和最终 1 个；策略 snapshot 随 job 固定。待后续 retention policy 任务实现。
- [ ] `R2-CKPT-015` active recovery、模型血缘、人工 hold 和审计调查引用阻止删除。`InMemoryObjectStore` 已支持 legal hold，checkpoint 层 hold 语义待后续实现。
- [ ] `R2-CKPT-016` GC 先生成 dry-run，经过 grace period 后删除对象，再更新 tombstone；不确定 ownership 时只告警。`InMemoryObjectStore::gc` 已支持 dry-run/min_age/max_delete，checkpoint GC orchestration 待后续实现。
- [ ] `R2-CKPT-017` 清理 Failed/Uploading 临时 shard 前确认 attempt 已终结且 lease 过期。待后续 lease-aware GC 任务实现。

## 5. 完成条件与测试

- 在 upload、barrier、manifest transaction、complete marker 和 restore 各阶段注入故障，系统只选择完整 checkpoint。
- 相同 checkpoint 被并发恢复或重复 finalize 不造成对象冲突、双重用量或状态倒退。
- node/rank 故障后恢复到最新兼容 step，并通过数值、optimizer 和 sampler 一致性检查。
- 备份恢复后 manifest、shard 和 complete marker 引用完整，摘要全部可验证。
