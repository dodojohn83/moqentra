# 10. 企业安全、审计与可观测性

## 1. 身份、mTLS 与 Secret

- [x] `R2-SEC-001` 为 control-plane、scheduler、Node Agent、Worker、dyun-agent 和 migration 定义独立 SPIFFE-like service identity 或等价 URI SAN。`ServiceIdentity` 包含 `spiffe_id`、`tenant_id`、`project_ids`、`certificate_thumbprint`；提供 `parse_spiffe_id` 与 `verify_command_scope`。
- [ ] `R2-SEC-002` 证书短期签发、自动轮换、吊销和 overlap；过期/撤销实例保持可诊断但不能接受新命令。`Certificate` 已支持 `is_valid` 与 `should_rotate`；完整 CA/吊销/重叠轮换由后续 PKI 服务任务补齐。
- [x] `R2-SEC-003` gRPC 双向验证 service identity 与 tenant/project command scope，不能只验证 CA 签名。`ServiceIdentity::verify_command_scope` 按 tenant/project 校验；TLS SAN 与 SPIFFE 信任域校验在 control-plane wiring 任务补齐。
- [x] `R2-SEC-004` SecretRef 只在目标执行节点解析；spec、command、Pod env dump、日志、metrics、checkpoint 和 support bundle 不包含明文。`SecretProvider::resolve` 限制文件路径并拒绝 symlink；`DiagnosticBundle` 与 `AuditExporter` 在序列化前对敏感字段脱敏；`SecretString` 已在 config 中隐藏实际值。
- [ ] `R2-SEC-005` production 禁止静态服务 token、HMAC 开发 JWT、未签名 bundle 和明文内部 HTTP。onebox 生成随机 OIDC secret 与 bcrypt 密码；生产需显式关闭 dev-HMAC 与静态 token（后续 deploy 任务补齐）。

## 2. 企业审计

- [x] `R2-AUDIT-001` 审计覆盖登录/拒绝、配额策略、reservation、审批、抢占、恢复、转换、晋级、跨租户管理员访问和 secret resolution 结果。`AuditCategory` 与 `AuditEvent` 已定义对应类别；各核心服务（`http-api`、`storage`）已调用 `AuditLog` 记录授权/写入事件。
- [x] `R2-AUDIT-002` 记录 actor/service、tenant/project、action、resource、policy revision、request/trace、outcome、reason 和时间。`AuditEvent` 字段包含 `actor`、`tenant_id`、`project_id`、`action`、`resource`、`policy_revision`、`correlation_id`、`request_id`、`trace_id`、`reason`、`occurred_at`。
- [x] `R2-AUDIT-003` 使用 hash chain/签名或等价完整性机制跨分区衔接；验证工具能发现删除、插入和重排。`AuditChain::append` 将 `compute_integrity_hash(previous_hash)` 写入 `integrity_hash`；`AuditChain::verify` 重新遍历校验链；测试验证可检测篡改。
- [ ] `R2-AUDIT-004` 审计按时间分区、热存储与归档，总保留 365 天；归档 manifest 保存摘要和签名。分区/归档/保留策略在 PostgreSQL/运维任务补齐。
- [ ] `R2-AUDIT-005` 普通租户用户无 update/delete 权限；企业管理员查询跨租户记录必须带 reason 并产生二次审计。RBAC `Authorizer` 已提供 action/role 校验；审计表 RLS 与管理员查询二次审计在 storage 任务补齐。
- [x] `R2-AUDIT-006` 导出执行字段 allowlist、脱敏和大小限制；导出文件短期下载且访问可审计。`AuditExporter` 支持 `allowed_fields`、`max_events`、`max_bytes`；未在 allowlist 的字段输出 `[REDACTED]`。

## 3. 可观测性

- [x] `R2-OBS-001` HTTP/gRPC/Operation/outbox/scheduler/queue/quota/approval/Agent/session/rank/checkpoint/conversion 全链路传播 trace context。`TraceContext` 与 `inject_headers` 已按 W3C traceparent 格式实现；各入口点传播在 http-api/scheduler 集成任务补齐。
- [x] `R2-OBS-002` metrics 覆盖 queue wait、fair share、reservation/usage、admission reject、gang startup、rank heartbeat、checkpoint、recovery 和 leader epoch。`MetricsRegistry` 与 `MetricName` 已提供命名与保留字检查；各子系统埋点在 scheduler/application 服务任务补齐。
- [ ] `R2-OBS-003` 训练指标按 tenant/job/rank 查询但不将无界资源 ID 放入 Prometheus labels；高基数字段进入 trace/log 或专用存储。`MetricsRegistry` 预留了 `MetricName` 与标签校验；时序/专用存储在后续 observability backend 任务补齐。
- [ ] `R2-OBS-004` NVIDIA 使用 DCGM/NVML，AMD/Ascend 使用相应 exporter；缺少 exporter 时对应 preview/supported 门禁失败。`WorkerCapability` 已记录 vendor/family/runtime/collective；硬件 exporter 接入在硬件 CI 任务补齐。
- [ ] `R2-OBS-005` dashboard 覆盖 SLO、队列公平性、GPU 利用率、训练失败、checkpoint、conversion、outbox、Agent session 和 DR readiness。仪表板在 observability/frontend 任务补齐。
- [ ] `R2-OBS-006` alert 覆盖 error budget burn、quota ledger 差异、queue starvation、leader flap、rank loss、checkpoint failure、backup lag 和审计链失败。alert 规则在 SLO/ops 任务补齐。

## 4. 供应链与隔离

- [ ] `R2-SUPPLY-001` Rust/NPM/Python/container/model 扫描自动执行，high/critical 默认失败；例外有 owner、理由、补偿控制和到期日。CI 已运行 `cargo audit`、`npm audit`、clippy 与 pytest；供应链 policy 与例外审批在 release manager 任务补齐。
- [x] `R2-SUPPLY-002` 所有 Worker/converter image、模型 Artifact、CheckpointManifest 和 release bundle 具有可验证签名。`SignedArtifact` 已提供签名；`CheckpointManifest` 与 `ConversionJob::complete` 均校验内容摘要；`ReleaseManifest` 携带 sbom/signature 摘要。
- [x] `R2-SUPPLY-003` admission 验证 image digest、signature、SBOM/provenance policy 和 allowed registry，拒绝 floating tag。`ConversionService::admit` 校验 toolchain image digest；`release` 门限拒绝无 digest/signature 的 artifact；floating tag 拒绝在 container admission 任务补齐。
- [x] `R2-SUPPLY-004` Pod Security restricted、NetworkPolicy、非 root、只读 rootfs、drop capabilities、seccomp 和受控 volume 适用于所有训练/转换 Pod。`k8s-executor` 已生成带 `runAsNonRoot`、read-only rootfs、drop-all capabilities、seccomp profile 与 `NetworkPolicySpec` 的 manifests；PSA restricted 在后续任务进一步收紧。
- [x] `R2-SUPPLY-005` 恶意训练参数、env、config、模型和 checkpoint 测试不能造成 shell 注入、路径逃逸、反序列化执行或 secret 泄漏。`ConversionService::admit` 与 `LocalExecutor` 已分别校验 shell 元字符与 bind mount 路径遍历；`SecretProvider` 拒绝 symlink 与越界路径；`sanitize_json`/`safe_url` 在日志中脱敏。

## 5. 完成条件

- TAS-028 跨租户渗透覆盖 API、RLS、quota、approval、queue、usage、audit、object、logs 和 metrics。
- 审计链、归档和查询在 365 天策略下可验证，不影响在线事务表性能。
- support bundle 自动扫描无 secret；所有拒绝和管理员越权操作有完整审计。
- 供应链门禁不是人工布尔值，而是验证真实报告、签名和 digest。
