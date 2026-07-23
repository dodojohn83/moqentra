# 10. 企业安全、审计与可观测性

## 1. 身份、mTLS 与 Secret

- [ ] `R2-SEC-001` 为 control-plane、scheduler、Node Agent、Worker、dyun-agent 和 migration 定义独立 SPIFFE-like service identity 或等价 URI SAN。
- [ ] `R2-SEC-002` 证书短期签发、自动轮换、吊销和 overlap；过期/撤销实例保持可诊断但不能接受新命令。
- [ ] `R2-SEC-003` gRPC 双向验证 service identity 与 tenant/project command scope，不能只验证 CA 签名。
- [ ] `R2-SEC-004` SecretRef 只在目标执行节点解析；spec、command、Pod env dump、日志、metrics、checkpoint 和 support bundle 不包含明文。
- [ ] `R2-SEC-005` production 禁止静态服务 token、HMAC 开发 JWT、未签名 bundle 和明文内部 HTTP。

## 2. 企业审计

- [ ] `R2-AUDIT-001` 审计覆盖登录/拒绝、配额策略、reservation、审批、抢占、恢复、转换、晋级、跨租户管理员访问和 secret resolution 结果。
- [ ] `R2-AUDIT-002` 记录 actor/service、tenant/project、action、resource、policy revision、request/trace、outcome、reason 和时间。
- [ ] `R2-AUDIT-003` 使用 hash chain/签名或等价完整性机制跨分区衔接；验证工具能发现删除、插入和重排。
- [ ] `R2-AUDIT-004` 审计按时间分区、热存储与归档，总保留 365 天；归档 manifest 保存摘要和签名。
- [ ] `R2-AUDIT-005` 普通租户用户无 update/delete 权限；企业管理员查询跨租户记录必须带 reason 并产生二次审计。
- [ ] `R2-AUDIT-006` 导出执行字段 allowlist、脱敏和大小限制；导出文件短期下载且访问可审计。

## 3. 可观测性

- [ ] `R2-OBS-001` HTTP/gRPC/Operation/outbox/scheduler/queue/quota/approval/Agent/session/rank/checkpoint/conversion 全链路传播 trace context。
- [ ] `R2-OBS-002` metrics 覆盖 queue wait、fair share、reservation/usage、admission reject、gang startup、rank heartbeat、checkpoint、recovery 和 leader epoch。
- [ ] `R2-OBS-003` 训练指标按 tenant/job/rank 查询但不将无界资源 ID 放入 Prometheus labels；高基数字段进入 trace/log 或专用存储。
- [ ] `R2-OBS-004` NVIDIA 使用 DCGM/NVML，AMD/Ascend 使用相应 exporter；缺少 exporter 时对应 preview/supported 门禁失败。
- [ ] `R2-OBS-005` dashboard 覆盖 SLO、队列公平性、GPU 利用率、训练失败、checkpoint、conversion、outbox、Agent session 和 DR readiness。
- [ ] `R2-OBS-006` alert 覆盖 error budget burn、quota ledger 差异、queue starvation、leader flap、rank loss、checkpoint failure、backup lag 和审计链失败。

## 4. 供应链与隔离

- [ ] `R2-SUPPLY-001` Rust/NPM/Python/container/model 扫描自动执行，high/critical 默认失败；例外有 owner、理由、补偿控制和到期日。
- [ ] `R2-SUPPLY-002` 所有 Worker/converter image、模型 Artifact、CheckpointManifest 和 release bundle 具有可验证签名。
- [ ] `R2-SUPPLY-003` admission 验证 image digest、signature、SBOM/provenance policy 和 allowed registry，拒绝 floating tag。
- [ ] `R2-SUPPLY-004` Pod Security restricted、NetworkPolicy、非 root、只读 rootfs、drop capabilities、seccomp 和受控 volume 适用于所有训练/转换 Pod。
- [ ] `R2-SUPPLY-005` 恶意训练参数、env、config、模型和 checkpoint 测试不能造成 shell 注入、路径逃逸、反序列化执行或 secret 泄漏。

## 5. 完成条件

- TAS-028 跨租户渗透覆盖 API、RLS、quota、approval、queue、usage、audit、object、logs 和 metrics。
- 审计链、归档和查询在 365 天策略下可验证，不影响在线事务表性能。
- support bundle 自动扫描无 secret；所有拒绝和管理员越权操作有完整审计。
- 供应链门禁不是人工布尔值，而是验证真实报告、签名和 digest。
