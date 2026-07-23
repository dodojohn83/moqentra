# 10. 安全、可观测性与故障恢复

## 1. 安全边界

- [x] `R1-SEC-001` 更新 threat model，覆盖浏览器、控制面、数据库、对象存储、Worker、用户镜像、Node Agent、Kubernetes、dyun runner 和媒体输入信任边界。
- [ ] `R1-SEC-002` 为每种服务身份签发短期证书，支持轮换、吊销和过期诊断；内部 gRPC 强制 mTLS 和服务级授权。
- [x] `R1-SEC-003` SecretRef 只在目标执行节点按授权解析，secret 不进入 spec、bundle、事件、日志、metrics、命令行、诊断包或数据库普通 JSON 字段。
- [~] `R1-SEC-004` 上传和媒体输入限制大小、类型、解码资源、URL scheme、DNS/IP、重定向和超时，防止 SSRF、压缩炸弹与解析器 DoS。
- [x] `R1-SEC-005` Worker/runner 容器无特权、无 host network/PID、最小 mount、只读 rootfs；Kubernetes admission 拒绝越权 spec。
- [ ] `R1-SEC-006` 建立 Rust/NPM/Python/container/model license 与漏洞扫描，发现 high/critical 时 release gate 默认失败并要求有期限的风险接受记录。

## 2. 可观测性

- [~] `R1-OBS-001` 全链路传播 request、correlation、trace、tenant、project、operation、job、attempt、deployment 和 replica ID；日志输出前统一脱敏。
- [~] `R1-OBS-002` 导出 HTTP/gRPC latency/error、DB pool/query、outbox backlog、Operation age、scheduler queue、Worker heartbeat、GPU、upload、training、deployment 和 stream 指标。
- [~] `R1-OBS-003` 控制标签基数：资源 ID 进入 trace/log，不进入无界 metric labels；训练用户自定义指标使用 allowlist 和配额。
- [x] `R1-OBS-004` readiness 区分必需依赖和可选能力；健康接口不返回 secret、DSN、内部堆栈或跨租户统计。
- [x] `R1-OBS-005` 提供 dashboard 与 alert rules：服务不可用、outbox 堆积、租约失联、训练失败率、对象错误、GPU 异常、deployment 不收敛和媒体断流。

## 3. 恢复与对账

- [x] `R1-REC-001` 每个后台单元限定 batch、并发、deadline、retry budget 和 shutdown drain；记录 cursor/lease 后再执行下一外部副作用。
- [x] `R1-REC-002` Training、Artifact 和 Deployment reconciler 从 desired/observed state 恢复，不依赖进程内 future 或本地队列。
- [x] `R1-REC-003` PostgreSQL 与 MinIO 备份使用一致性窗口和 manifest；恢复后逐项验证表数量、对象摘要、引用完整性和 RLS。
- [ ] `R1-REC-004` 注入控制面、scheduler、Worker、Node Agent、dyun-agent、数据库和 MinIO 中断，记录 RTO、数据丢失和孤儿资源。
- [x] `R1-REC-005` GC 使用引用扫描、grace period 和 dry-run；任何不确定 ownership 的对象只告警不删除。

## 4. 完成条件

- 租户 A 无法通过 API、数据库、对象、签名 URL、日志、metrics、错误或时间差推断租户 B 数据。
- 诊断包通过自动 secret scanner；审计记录可验证完整性且普通用户不可篡改。
- 任一进程中止后，系统最终收敛到 PostgreSQL 权威状态，无重复发布、永久 running 或无主容器/Pod/runner。
- 备份恢复达到 `baseline/release-policy.md` 规定的 RTO/RPO；没有明确基线时先补 ADR/策略，不由实现者自行猜测。
