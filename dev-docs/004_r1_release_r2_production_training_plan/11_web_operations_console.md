# 11. R2 Web 管理与运维界面

## 1. 配额与审批

- [ ] `R2-WEB-001` 租户/项目配额页展示策略 revision、有效期、hard limits、reservation、实际用量和剩余额度。
- [ ] `R2-WEB-002` 训练提交前展示设备×replica×max duration 的预估 reservation，并提示需要审批的超限项。
- [ ] `R2-WEB-003` 审批队列支持申请、详情、approve/reject、理由和有效期；隐藏用户无权查看的跨项目字段。
- [ ] `R2-WEB-004` 审批人不能在 UI 或直接 API 审批自己提交的请求；冲突/过期 revision 要求刷新。

## 2. 队列、资源和分布式运行

- [ ] `R2-WEB-005` queue 页面展示 weight、capacity、running/pending、等待时间、priority 和 fair-share 解释。
- [ ] `R2-WEB-006` ResourceClass/节点页展示 vendor、设备、driver/runtime、topology、sharing、health、support tier 和证据日期。
- [ ] `R2-WEB-007` 训练创建页支持 replicas、processesPerReplica、backend、queue、priority、checkpoint、resume 和 preemption，实时显示 world size。
- [ ] `R2-WEB-008` 运行详情按 attempt/replica/rank 展示节点、设备、状态、heartbeat、exit code、日志和指标。
- [ ] `R2-WEB-009` checkpoint 页面显示 shards、complete 状态、兼容性、hold/retention 和恢复历史；Uploading checkpoint 不提供恢复操作。
- [ ] `R2-WEB-010` 抢占、取消、retry、resume 均显示影响、审批需求和 Operation 进度，失败不显示虚假成功。

## 3. 转换、审计与 HA

- [ ] `R2-WEB-011` ConversionProfile 页面展示源/目标格式、converter、硬件、参数、支持等级和 blocker。
- [ ] `R2-WEB-012` 转换报告比较 size、精度、延迟和基线，模型晋级走审批界面。
- [ ] `R2-WEB-013` 审计检索支持 actor/action/resource/outcome/time，跨租户模式仅对企业管理员开放并要求 reason。
- [ ] `R2-WEB-014` 运维页展示 scheduler leader/epoch、Agent session、outbox/dead-letter、backup age、RPO lag、SLO 和 alert。
- [ ] `R2-WEB-015` dead-letter retry、Agent drain 和 DR 操作必须使用高风险确认、重新鉴权和服务端审批/审计。

## 4. 安全与性能

- [ ] `R2-WEB-016` 所有数据 cache key 包含 tenant/project；切换租户取消 SSE、轮询、下载和 mutation，并清空前租户缓存。
- [ ] `R2-WEB-017` rank/log/metric 大列表分页或虚拟化；SSE 按 cursor 恢复并去重，不形成无界浏览器内存。
- [ ] `R2-WEB-018` support tier、blocked 和 compile-only 使用明确文案，不能用绿色“可用”混淆支持等级。
- [ ] `R2-WEB-019` Playwright 覆盖 tenant admin、approver、algorithm engineer、operator 和无权用户的完整 R2 旅程。

## 5. 完成条件

- 所有可见动作由服务端返回 capability/policy 决定，前端隐藏按钮不替代授权。
- 配额、审批、rank、checkpoint、转换和 HA 状态刷新后可从服务端恢复。
- CSP、XSS、CSRF、tenant cache isolation、过期审批和恶意日志内容测试通过。
