# 01. Gate 0：R1 发布出口

## 1. 进入依据

003 当前仍有真实环境与发布证据任务未完成，同时生产控制面仍存在内存热路径和 best-effort PostgreSQL 写穿。R2 的 HA、配额和多节点调度不能建立在该状态之上。

## 2. 权威状态收口

- [ ] `R2-G0-001` 将 Dataset、Annotation、Training、Conversion、Evaluation、Model、Application 和 Deployment 的生产访问统一切换为 application repository ports。
- [ ] `R2-G0-002` 所有 mutating use case 使用 `UnitOfWork` 原子提交聚合、Operation、outbox、audit 和 idempotency response；删除 best-effort 写穿。
- [ ] `R2-G0-003` 移除生产启动时依赖 `load_all_for_recovery` 构建内存权威的路径；cache 只保存可失效的只读副本。
- [ ] `R2-G0-004` 内存 repository、outbox、audit、object store 仅在测试或显式 `demo-in-memory` feature/profile 可用。
- [ ] `R2-G0-005` 生产缺少 PostgreSQL、S3、OIDC、mTLS 或 bundle/signing key 时 fail-fast，readiness 不允许伪健康。
- [ ] `R2-G0-006` 真实 PostgreSQL contract tests 覆盖事务失败、连接复用、RLS、并发 revision、outbox 重放和控制面多进程访问。

## 3. 完成 003 外部验收

- [ ] `R2-G0-007` 关闭 `R1-K8S-009`：单节点 k3s 的 Kubernetes Job 和 VolcanoJob 完成真实 NVIDIA 检测训练。
- [ ] `R2-G0-008` 关闭 `R1-SEC-002`：控制面、scheduler、Node Agent、Worker 和 dyun-agent 使用短期 mTLS 身份并完成轮换/吊销测试。
- [ ] `R2-G0-009` 关闭 `R1-SEC-006`：Rust/NPM/Python/container/model 扫描自动运行，高危和严重问题默认阻止发布。
- [ ] `R2-G0-010` 关闭 `R1-DYUN-005` 至 `R1-DYUN-011`：固定 commit 的 dyun-gu runner 完成合成 RTSP→检测→跟踪→OSD→RTMP、故障和证据归档。
- [ ] `R2-G0-011` 自动执行 Web、Python、真实 PostgreSQL/MinIO/Dex/OCI/k3s/dyun 合约与 E2E，取消仅手工触发的 staged 门禁。
- [ ] `R2-G0-012` 完成 `R1-E2E-001` 至 `R1-E2E-015`，包括跨租户安全、备份恢复、升级回滚和 72 小时耐久。

## 4. R1 发布裁决

- [ ] `R2-G0-013` 将 capability tracking 按真实证据更新为 designed/implemented/integrated/accepted；未验收能力不得标记 accepted。
- [ ] `R2-G0-014` 生成 `v0.1.0` ReleaseManifest、镜像 digest、SBOM、provenance、签名、支持矩阵和已知限制。
- [ ] `R2-G0-015` 创建 R1 release branch/tag，验证干净环境能只依赖发布物完成安装、黄金路径、备份恢复和应用回滚。
- [ ] `R2-G0-016` 冻结 R1 OpenAPI、Proto、JSON Schema 和迁移兼容基线，作为 R2 breaking tests 的比较源。

## 5. 完成条件

- PostgreSQL/对象存储成为生产唯一权威，控制面重启或切换实例不改变资源结果。
- 003 所有 R1 release blocker 已关闭或按明确支持等级进入已接受风险。
- `v0.1.0` 的证据包可以独立复核，不引用 mock 作为真实环境结论。
