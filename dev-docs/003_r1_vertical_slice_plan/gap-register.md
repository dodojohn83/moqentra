# R1 垂直切片差距清单

> **2026-07-21 自动代码审查循环最终记录**
>
> 已完成多轮修复（PR #48–#50）：middleware 顺序、S3 digest、内存对象存储索引泄漏、指标非有限值过滤、分页越界、SSRF、PostgreSQL idempotency/outbox/UnitOfWork、reqwest 超时、启动期配置校验、调度器重入队、health probe 固定 key、RBAC 角色解析、审计失败阻断、项目 scope 竞态。
>
> 本地 `cargo fmt`、`cargo clippy -D warnings`、`cargo test --workspace` 全绿；CI `rust`/`contracts`/`openapi` 通过。
>
> 本轮循环未能自动修复的**关键风险**与**无法验证的环境风险**见下表末尾 "剩余风险" 部分。

本文件承接 `01_current_state_and_execution_contract.md` 的 `R1-GOV-002`，把 OpenAPI、Proto、JSON Schema、运行路由和数据库表核对后的主要差距绑定到 `003_r1_vertical_slice_plan` 的唯一任务 ID。

| ID | 差距描述 | 当前状态 | 003 任务 | 是否阻塞 R1 交付 |
|---|---|---|---|---|
| GOV-001 | 能力矩阵缺少 `designed/implemented/integrated/accepted` 四级状态和证据链接 | `docs/capability-tracking.md` 仅有阶段与章节 | R1-GOV-001 | 否 |
| GOV-002 | 缺少面向 R1 的差距、风险和固定环境清单 | 无统一登记 | R1-GOV-002 ~ R1-GOV-004 | 是 |
| GOV-003 | CI artifact 目录和命名约定未固化 | 无 `artifacts/r1-evidence/` 约定 | R1-GOV-005 | 否 |
| GOV-004 | `ci-staged.yml` 仍手工触发，未按变更路径自动执行 | `on: workflow_dispatch` | R1-GOV-006 | 是 |
| GOV-005 | 控制面 handler 的 DTO 和路由仍分散在应用入口 | `apps/control-plane/src/main.rs` 直接组装 | R1-API-001 | 否 |
| API-001 | 应用层 repositories 未持久化到 PostgreSQL | `crates/storage` 仅内存实现 | R1-API-002, R1-DB-001 ~ R1-DB-006 | 是 |
| API-002 | `UnitOfWork` 未聚合聚合根、Operation、outbox、audit 和幂等 | 无统一事务边界 | R1-API-003 | 是 |
| API-003 | 分页、过滤、稳定排序、revision 和 ETag 未统一 | 各模块自行处理 | R1-API-004 | 否 |
| API-004 | `Operation/v1` 和 `EventEnvelope/v1` schema 未覆盖 deadline/取消/重试 | 无 Operation/Event schema | R1-API-005 | 是 |
| API-005 | TypeScript/Python client 生成未纳入 CI 差异检查 | 生成脚本缺失 | R1-API-006 | 否 |
| DB-001 | 核心资源表迁移仅到 `0001_init.sql` | `crates/storage/migrations/` 仅初始 | R1-DB-001 | 是 |
| DB-002 | PostgreSQL repositories 未实现 | 仅有 trait/内存适配 | R1-DB-002 | 是 |
| DB-003 | PostgreSQL outbox 未实现 `FOR UPDATE SKIP LOCKED`/lease/死信 | 内存实现 | R1-DB-003 | 是 |
| DB-004 | PostgreSQL idempotency 未实现 TTL/GC/响应回放 | 内存实现 | R1-DB-004 | 是 |
| DB-005 | RLS 非 fail-closed，缺少 `app.current_tenant` 时行为未定义 | 初始策略待审 | R1-DB-005 | 是 |
| IAM-001 | OIDC 生产路径依赖 HMAC 开发 token | `crates/auth/src/jwt.rs` 含 HMAC 路径 | R1-IAM-001 | 是 |
| IAM-002 | principal 的 tenant/project role 从数据库成员关系解析未实现 | 内存角色 | R1-IAM-002 | 是 |
| IAM-003 | deny-by-default 权限矩阵未落地 | 基础 RBAC 测试 | R1-IAM-003 | 是 |
| IAM-004 | 结构化审计未持久化且不可被租户用户修改 | 内存日志 | R1-IAM-004 | 是 |
| DATA-001 | 对象 key builder 未强制租户/项目/资源/版本分层 | 调用者可传任意 key | R1-DATA-001 | 是 |
| DATA-002 | multipart upload session API 未实现 | 无 session 管理 | R1-DATA-002 | 是 |
| DATA-003 | 浏览器断点续传、取消和短期签名 URL 未实现 | 无上传管理器端到端 | R1-DATA-003 | 否 |
| DATA-004 | S3/MinIO 导入 Operation 未实现 | 无导入 Operation | R1-DATA-004 | 是 |
| DATA-005 | 媒体探测/解码验证/恶意文件扫描 Worker 未实现 | 无隔离 Worker | R1-DATA-005 | 是 |
| DATA-006 | 临时对象 GC 与 legal hold 未实现 | 无引用保护 | R1-DATA-006 | 否 |
| DATA-007 | Dataset/Asset/Version/Manifest 持久化未实现 | 仅内存领域模型 | R1-DATA-007 | 是 |
| DATA-008 | train/val/test split 规则和确定性摘要未进入 manifest | 无 split 实现 | R1-DATA-008 | 否 |
| DATA-009 | 冻结后版本可变性保护未实现 | 无状态机 | R1-DATA-009 | 是 |
| DATA-010 | COCO/LabelU native 导入导出 round-trip 未实现 | 仅有 LabelUAdapter 骨架 | R1-DATA-010 | 否 |
| LABEL-001 | LabelU-Kit v5.11.0 未固定引入 | `apps/web/src/annotation/LabelUAdapter.ts` 为适配器 | R1-LABEL-001 | 否 |
| LABEL-002 | 标注项目/ontology/task 切分 API 未实现 | 仅 `annotation_svc.rs` 基本测试 | R1-LABEL-002 | 否 |
| LABEL-003 | task claim/renew/release 与 fencing 未实现 | 无任务领取 | R1-LABEL-003 | 否 |
| LABEL-004 | 草稿自动保存冲突 diff 未实现 | 无草稿自动保存 | R1-LABEL-004 | 否 |
| LABEL-005 | submit → review → approve/reject → rework 状态机未实现 | 无审核状态机 | R1-LABEL-005 | 是 |
| LABEL-006 | 媒体 URL 授权与秘密泄露防护未实现 | 无签名 URL | R1-LABEL-006 | 是 |
| TRAIN-001 | `WorkerCapabilities/v1` 字段不完整 | 基础 capability 结构 | R1-TRAIN-001 | 否 |
| TRAIN-002 | `WorkerAgentService.Connect` 双向流未实现 | 无 gRPC service | R1-TRAIN-002 | 是 |
| TRAIN-003 | 消息 fencing/sequence/过期租约校验未实现 | 无序列号 | R1-TRAIN-003 | 是 |
| TRAIN-004 | mTLS、版本协商、keepalive、credit、退避未实现 | 基础 gRPC stub 缺失 | R1-TRAIN-004 | 是 |
| TRAIN-005 | 取消语义（Drain/SIGTERM/SIGKILL）未实现 | 无状态区分 | R1-TRAIN-005 | 是 |
| TRAIN-006 | Python gRPC client/stubs 未生成 | `python/moqentra_worker` 仅 SDK 骨架 | R1-TRAIN-006 | 是 |
| TRAIN-007 | Worker 输入物化与输出提交 manifest 未实现 | 无物化/提交 | R1-TRAIN-007 | 是 |
| TRAIN-008 | ResNet18 分类训练模板未实现 | 无模板 | R1-TRAIN-008 | 是 |
| TRAIN-009 | SSDlite320 MobileNetV3 检测模板未实现 | 无模板 | R1-TRAIN-009 | 是 |
| TRAIN-010 | DeepLabV3 MobileNetV3 分割模板未实现 | 无模板 | R1-TRAIN-010 | 是 |
| TRAIN-011 | 合成视觉 fixture 生成器未实现 | 无 fixture | R1-TRAIN-011 | 是 |
| LOCAL-001 | Node Agent 真实硬件/runtime 探测未实现 | 硬编码 capability | R1-LOCAL-001 | 否 |
| LOCAL-002 | 真实 OCI launch 安全策略未实现 | 仅分配模型 | R1-LOCAL-002 | 是 |
| LOCAL-003 | workspace 挂载/symlink escape 控制未实现 | 无挂载实现 | R1-LOCAL-003 | 是 |
| LOCAL-004 | 设备原子分配和并发配额未实现 | 无设备分配 | R1-LOCAL-004 | 是 |
| LOCAL-005 | 日志流背压切块上传未实现 | 无日志流 | R1-LOCAL-005 | 否 |
| LOCAL-006 | 重启后对 active attempt/容器/allocation 对账未实现 | 无对账 | R1-LOCAL-006 | 是 |
| K8S-001 | Kubernetes client adapter 未实现 | 无 client | R1-K8S-001 | 是 |
| K8S-002 | Job/VolcanoJob 编译未实现 | 无编译 | R1-K8S-002 | 是 |
| K8S-003 | tenant namespace/RBAC/NetworkPolicy 未实现 | 无 K8s 集成 | R1-K8S-003 | 是 |
| K8S-004 | short-term credentials 传入 Job/Pod 未实现 | 无凭据传递 | R1-K8S-004 | 是 |
| K8S-005 | watch resourceVersion 恢复与分页 list 未实现 | 无 watch | R1-K8S-005 | 否 |
| K8S-006 | Pod 状态归一化未实现 | 无归一化 | R1-K8S-006 | 是 |
| K8S-007 | 取消/delete 语义未实现 | 无删除处理 | R1-K8S-007 | 是 |
| K8S-008 | orphan workload 回收未实现 | 无回收 | R1-K8S-008 | 是 |
| K8S-009 | k3s/Volcano smoke test 未实现 | 无真实测试 | R1-K8S-009 | 是 |
| MODEL-001 | Model/Version/Artifact/Lineage 持久化未实现 | 仅内存领域 | R1-MODEL-001 | 是 |
| MODEL-002 | Artifact reconciler 校验未实现 | 无 reconciler | R1-MODEL-002 | 是 |
| MODEL-003 | lineage 字段强制记录未实现 | 无持久 lineage | R1-MODEL-003 | 是 |
| MODEL-004 | ModelVersion 去重未实现 | 无去重 | R1-MODEL-004 | 是 |
| MODEL-005 | 模型生命周期与审批未实现 | 无状态机 | R1-MODEL-005 | 是 |
| CONVERT-001 | 隔离 Conversion Operation 未实现 | 无转换 Operation | R1-CONVERT-001 | 是 |
| CONVERT-002 | ONNX 导出（三个模板）未实现 | 无导出 | R1-CONVERT-002 | 是 |
| CONVERT-003 | ONNX Runtime 数值校验未实现 | 无校验 | R1-CONVERT-003 | 是 |
| APP-001 | Application/Version/Deployment 持久化未实现 | 仅内存领域 | R1-APP-001 | 是 |
| APP-002 | ComponentCatalog 未建立 | 无组件目录 | R1-APP-002 | 是 |
| APP-003 | 编译前校验未实现 | 无校验 | R1-APP-003 | 是 |
| APP-004 | binding snapshot 未实现 | 无 binding | R1-APP-004 | 是 |
| APP-005 | canonical `dg/v1 Graph` 生成未实现 | 无 canonical | R1-APP-005 | 是 |
| APP-006 | 签名 `DyunGraphBundle/v1` 未实现 | 无签名 | R1-APP-006 | 是 |
| DYUN-001 | `DyunAgentService` proto 未定义 | 无 proto | R1-DYUN-001 | 是 |
| DYUN-002 | dyun-agent 动态能力探测未实现 | 硬编码 | R1-DYUN-002 | 否 |
| DYUN-003 | bundle 签名验证未实现 | 无签名 | R1-DYUN-003 | 是 |
| DYUN-004 | 模型下载和 SecretRef 解析未实现 | 无下载 | R1-DYUN-004 | 是 |
| DYUN-005 | 直接调用 dyun-gu Rust API 未实现 | 无 runner 集成 | R1-DYUN-005 | 是 |
| DYUN-006 | desired/observed generation 持久化未实现 | 无持久状态 | R1-DYUN-006 | 是 |
| DYUN-007 | drain/异常/失联收敛未实现 | 无收敛 | R1-DYUN-007 | 是 |
| DYUN-008 | 合成 RTSP 输入未实现 | 无输入 | R1-DYUN-008 | 是 |
| DYUN-009 | RTSP→检测→跟踪→OSD→RTMP 真实链路未实现 | 无链路 | R1-DYUN-009 | 是 |
| DYUN-010 | 断流/模型下载失败/runner crash 等故障注入未实现 | 无测试 | R1-DYUN-010 | 是 |
| WEB-001 | React Router/OIDC PKCE shell 未实现 | 仅 `Shell.tsx` | R1-WEB-001 | 否 |
| WEB-002 | OpenAPI-generated TypeScript client 未使用 | 手写 `apiClient.ts` | R1-WEB-002 | 否 |
| WEB-003 | tenant cache 隔离未实现 | 无租户缓存层 | R1-WEB-003 | 否 |
| WEB-004 | Problem Details/202 Operation 交互未统一 | 基础错误处理 | R1-WEB-004 | 否 |
| WEB-005 | SSE cursor/reconnect 未实现 | 无 SSE | R1-WEB-005 | 否 |
| WEB-006 ~ WEB-012 | 业务页面未实现 | 仅有核心组件 | R1-WEB-006 ~ R1-WEB-012 | 否 |
| PKG-001 ~ PKG-004 | OCI 镜像、SBOM、签名未实现 | 无 Dockerfile | R1-PKG-001 ~ R1-PKG-004 | 是 |
| ONEBOX-001 ~ ONEBOX-006 | Onebox Compose/初始化/smoke 未实现 | 仅 README | R1-ONEBOX-001 ~ R1-ONEBOX-006 | 是 |
| HELM-001 ~ HELM-005 | Helm chart 和升级/回滚策略未实现 | 仅有目录骨架 | R1-HELM-001 ~ R1-HELM-005 | 否 |
| SEC-001 ~ SEC-006 | threat model/证书/SecretRef/输入限制/扫描未落地 | 部分安全测试 | R1-SEC-001 ~ R1-SEC-006 | 是 |
| OBS-001 ~ OBS-005 | 全链路 trace/metrics/dashboard 未落地 | 部分日志测试 | R1-OBS-001 ~ R1-OBS-005 | 否 |
| REC-001 ~ REC-005 | 对账/备份/混沌/GC 未落地 | 部分 scheduler 对账测试 | R1-REC-001 ~ R1-REC-005 | 否 |
| QA-001 ~ QA-006 | nextest/Buf/Web/Python/真实适配器 CI 未完善 | 基础 CI | R1-QA-001 ~ R1-QA-006 | 是 |
| E2E-001 ~ E2E-015 | 黄金路径与故障/安全/备份/72h 测试未执行 | 无 | R1-E2E-001 ~ R1-E2E-015 | 是 |


## 自动代码审查循环剩余风险（2026-07-21）

本轮审查在 PR #48–#50 修复后仍发现以下问题，按严重程度排序。它们无法在一次自动修复轮次内完成，已作为已知风险记录。

### 1. 高：控制面状态未真正持久化（阻塞循环终止）

- **文件位置**
  - `apps/control-plane/src/main.rs` 146–160 行：`AppState` 仍实例化 `InMemoryDatasetRegistry`、`InMemoryTrainingRegistry`、`InMemoryModelRegistry`、`InMemoryConversionRegistry`、`InMemoryEvaluationRegistry`、`InMemoryAnnotationRegistry`、`InMemoryOutbox`、`InMemoryUploadSessionStore`、`InMemoryImportJobStore`。
  - `crates/http-api/src/control_plane.rs` `AppState` 定义与所有写 handler：handler 直接通过 `state.datasets.lock()` 等调用内存 registry 方法，未使用 `crates/storage/src/repositories/*.rs` 中的 `Pg*` 实现。
- **原因**
  - `crates/application/src/ports.rs` 已定义 `DatasetRepository`、`TrainingJobRepository`、`ModelRepository`、`AnnotationProjectRepository`、`ConversionRepository`、`EvaluationRepository` 等 trait；`crates/storage` 已实现对应的 PostgreSQL repository 与 `PgUnitOfWork`。
  - 但 `AppState` 字段类型是 `Arc<Mutex<InMemory*Registry>>`，`build_state_from_env` 即使检测到 `DATABASE_URL` 也忽略 `db_pool` 的 repository 实现。
  - `DatasetRepository` 等 trait 的方法集与 handler 中使用的内存 registry 方法不完全对齐（缺少 `add_asset`、`generate_splits`、`find_asset`、`pending_validations`、`referenced_object_keys` 等），导致无法直接替换。
- **影响**
  - 配置 `DATABASE_URL` 后，控制面重启或横向扩容仍会丢失/不一致 datasets、training jobs、models、annotations、conversions、evaluations、outbox、upload sessions、import jobs。
  - `R1-API-003`（UnitOfWork 事务边界）和 `R1-DB-002` 未在控制面落地，幂等、审计、outbox 无法保证原子性。
- **建议修复方式**
  1. 扩展 repository trait（或在 domain service 中实现 `add_asset`/`generate_splits`/`find_asset` 等操作），使其覆盖 handler 所需的全部操作。
  2. 将 `AppState` 的 registry 字段改为 `Arc<dyn Repository + Send + Sync>`（或用 enum wrapper）。
  3. 在 `build_state_from_env` 中根据 `db_pool` 选择 `Pg*` repository 或内存 repository adapter。
  4. 把所有 handler 改为 async trait 调用，并通过 `PgUnitOfWork` 聚合 `repository + outbox + audit + idempotency` 的提交。
  5. 实现 `PgUploadSessionStore`、`PgImportJobStore`、多租户 `PgOutbox` 及其 dispatcher。

### 2. 中：`upload_sessions` 与 `import_jobs` 无 PostgreSQL 实现

- **文件位置**：`crates/object-store/src/upload_session.rs`、`crates/http-api/src/import.rs`、`apps/control-plane/src/main.rs`。
- **原因**：仅有 `UploadSessionStore`/`ImportJobStore` trait 和内存实现，无 PG adapter。
- **影响**：上传会话和导入任务在进程重启后丢失。
- **建议修复方式**：为这两个 trait 新增 `PgUploadSessionStore` 和 `PgImportJobStore`，并在 `build_state_from_env` 中按 `db_pool` 选择。

### 3. 中：多租户 PG outbox dispatcher 未实现

- **文件位置**：`crates/storage/src/pg_outbox.rs`、`crates/http-api/src/control_plane.rs` `emit_event`/`spawn_outbox_dispatcher`。
- **原因**：`PgOutboxStore` 按 tenant 实例化并依赖 `app.current_tenant` RLS；dispatcher 需要跨所有租户轮询，但当前没有超级用户/函数绕过机制。
- **影响**：配置 DB 后 outbox 事件仍无法持久化、恢复与 lease 回收。
- **建议修复方式**：实现一个多租户 `PgOutbox`（dispatcher 为每个活跃租户 `set_config` 后 `FOR UPDATE SKIP LOCKED` 领取），或创建 `SECURITY DEFINER` 轮询函数并跳过 RLS。

### 4. 低：scheduler 内存 fallback 无退避/死信

- **文件位置**：`apps/scheduler/src/main.rs` `process_in_memory_job`。
- **原因**：控制面 `admit` 持续失败时，任务会立即重新入队并再次消费。
- **影响**：可能形成高频重试循环，消耗 CPU/网络。
- **建议修复方式**：在 `QueueEntry` 增加 `retry_count` 与 `next_attempt_at` 时间戳；超过最大重试或 deadline 后转入死信或丢弃。

### 5. 低：`MOQENTRA_UPLOAD_SIG_SECRET` 默认硬编码

- **文件位置**：`apps/control-plane/src/main.rs`。
- **原因**：未设置环境变量时使用固定字符串 `"moqentra-upload-sig-secret"`。
- **影响**：上传签名 URL 可被预测/伪造。
- **建议修复方式**：生产环境未配置时启动失败；文档要求随机高熵 secret。

### 6. 低：`InMemoryOutbox` 无 processing 事件 lease 过期回收

- **文件位置**：`crates/http-api/src/control_plane.rs` `spawn_outbox_dispatcher`。
- **原因**：内存 dispatcher 在 `processing` 状态下 panic 后不会重新派发。
- **影响**：内存模式时部分 outbox 消息可能 stuck。
- **建议修复方式**：为内存 dispatcher 增加 TTL/lease 清理，或优先迁移到 PG outbox。

### 无法验证的环境风险

- 真实 PostgreSQL/S3/MinIO/Kubernetes/Volcano/ONNX Runtime/dyun-gu runner 环境不可用；相关 contract/integration 测试只能跑内存或 mock。
- PR #50 之后 CI 中 `python`/`web`/`supply-chain` 仍被跳过，未执行前端类型检查、Python template 测试与依赖供应链扫描。

---

**结论**：本轮自动代码审查循环在修复 PR #48–#50 后结束；剩余关键风险主要是控制面 PostgreSQL 持久化接入，需在后续专门 PR/里程碑中处理。
