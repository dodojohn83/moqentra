# Moqentra R1 真实闭环开发执行包

本目录承接 `dev-docs/002_vibe_coding_plan`。002 已完成架构、领域模型、契约和适配器骨架；本执行包不重复这些设计任务，而是把它们集成为可持久化、可部署、可恢复、可由真实硬件验证的 R1 视觉 MVP。

## 1. R1 交付目标

全新环境必须能够完成以下黄金路径，并在控制面重启后保持权威状态：

```text
OIDC 登录 → 创建项目 → 导入图片/视频 → 标注与审核 → 冻结数据版本
→ 本地或 Kubernetes 训练 → 注册模型与 ONNX → 编译 ApplicationSpec
→ 签名 DyunGraphBundle → dyun-gu 运行 RTSP→检测→跟踪→OSD→RTMP
```

同一份 `TrainingJobSpec/v1` 在本地 OCI 和单节点 Kubernetes 上具有相同状态、日志、指标、检查点和产物语义。RTX 3090 产生真实 NVIDIA 功能证据，但按支持矩阵保持 `preview`，不得据此声明数据中心硬件 `supported`。

## 2. 章节和执行顺序

| 波次 | 章节 | 结果 | 依赖 |
|---|---|---|---|
| G0 | [01 现状与执行契约](01_current_state_and_execution_contract.md) | 建立真实状态与证据口径 | 无 |
| G1 | [02 API、身份与持久化](02_api_identity_persistence.md) | 可重启的租户安全控制面 | G0 |
| G2 | [03 数据与标注](03_dataset_ingestion_and_annotation.md) | 可冻结的已标注数据版本 | G1 |
| G3a | [04 Worker 与本地训练](04_training_worker_and_local_execution.md) | RTX 3090 本地真实训练 | G2 |
| G3b | [05 Kubernetes 执行](05_kubernetes_execution.md) | 同一 JobSpec 的集群执行 | G1、G3a 契约 |
| G4a | [06 模型与转换](06_model_registry_and_conversion.md) | 可追溯模型和 ONNX | G3a |
| G4b | [07 应用与 dyun-gu](07_application_compiler_and_dyun.md) | 真实视频推理闭环 | G4a |
| G2–G5 | [08 Web 纵向旅程](08_web_console_vertical_journey.md) | 用户可操作的统一门户 | 对应后端接口 |
| G5 | [09 单机与集群交付](09_onebox_and_cluster_packaging.md) | 可安装、可升级的交付包 | G1–G4 |
| 全程 | [10 安全、可观测与恢复](10_security_observability_and_recovery.md) | 可诊断、可恢复、安全边界闭合 | 随各波次实施 |
| G6 | [11 验收与发布证据](11_acceptance_and_release_evidence.md) | R1 release candidate 门禁 | 全部章节 |

G1 完成后，G2 与 G3b 的基础适配可以并行；G2 冻结数据版本后才允许开始真实训练；G4a 和 G4b 必须串行。Web 按已冻结的 OpenAPI 增量接入，不得先发明临时接口。

## 3. 执行规则

- 本目录的 `[ ]` 是尚未完成的 R1 任务。只有真实实现、测试和证据齐全后才能改为 `[x]`。
- 每项任务单独提交或形成可独立评审的提交组；提交信息包含任务 ID。
- PostgreSQL 是元数据与状态的唯一权威来源，S3/MinIO 是大文件的唯一权威来源。内存 Registry、fake、simulator 只用于测试。
- 数据库迁移从 `0002` 起追加，禁止修改已发布的 `0001_init.sql`；回滚依靠代码兼容扩展后 schema，不依赖向下迁移。
- 北向 HTTP 统一为 `/v1`；内部控制使用版本化 gRPC。Agent 的 HTTP 只保留 health/readiness/metrics。
- 所有写请求幂等；所有异步任务有 deadline、取消、重试、租约、fencing token 和恢复语义。
- 禁止 `todo!()`、`unimplemented!()`、空成功响应、无界队列、shell 拼接、浮动镜像标签以及 mock 冒充真实验收。
- 任何跨层或契约变更先新增 ADR，再更新 OpenAPI/Proto/JSON Schema 和追踪矩阵。

## 4. 单任务完成证据

每个任务勾选时在所属章节追加：

```text
提交：<commit SHA / PR>
变更：<关键实现与兼容说明>
测试：<可复现命令>
结果：<通过数、耗时、环境>
证据：<日志、报告、对象摘要或 CI URL>
限制：<仍保持 preview/compile-only 的边界>
```

仅“代码可编译”不能关闭集成任务；仅单元测试不能关闭真实 PostgreSQL、MinIO、OCI、Kubernetes、硬件或 dyun-gu 验收任务。

## 5. 全局完成标准

- 黄金路径在 Onebox 和单节点 Kubernetes 各通过一次，且使用相同版本化契约。
- 控制面、scheduler、Worker、Node Agent、dyun-agent 分别被中止后，系统可恢复且无重复模型版本或永久孤儿资源。
- `cargo fmt/clippy/nextest`、Buf、OpenAPI breaking、Web、Python、依赖与镜像扫描全部通过。
- 跨租户 API、数据库、对象存储、日志和指标隔离测试通过。
- 完成 72 小时耐久运行、PostgreSQL/MinIO 备份恢复和一次应用版本回滚。
- 所有生产镜像使用 digest，发布物包含 SBOM、provenance、签名、版本矩阵和已知限制。

## 6. R1 范围边界

R1 不实现多节点 DDP、AMD/Ascend 真实支持、HPO、Notebook、多集群、桌面客户端和零停机 canary。002 中已有的相关领域骨架可以保留，但不得进入 R1 支持声明或阻塞 R1 交付。
