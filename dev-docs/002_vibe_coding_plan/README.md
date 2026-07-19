# Moqentra 全量开发执行计划

本目录把 [系统设计](../../docs/system-design.md) 转换为可按顺序执行、测试和评审的工程任务。Moqentra 面向行业视觉场景，覆盖数据集、标注、训练、模型、应用编排和推理发布；同一版本支持单机一体化与 Kubernetes 集群部署。

## 1. 执行契约

- 严格按依赖顺序实施。契约、租户隔离和持久化先于页面与适配器。
- 每个 `[ ]` 是可独立提交、测试和评审的交付项；完成后改为 `[x]`，并附 commit、测试命令和结果。
- 禁止 `todo!()`、`unimplemented!()`、空成功响应、无界队列和 mock 冒充真实硬件验收。
- 所有资源必须携带 `TenantId`；所有异步操作必须有幂等键、deadline、取消和恢复语义。
- 外部格式和内部协议先定义版本化 schema，再生成代码和实现 mapper。
- 上游源码不直接复制。依赖固定到 tag/commit，缺口登记到 `../upstream/`。

规范用语：**必须**是验收门槛，**应该**需要记录偏离理由，**可以**是可选增强。

## 2. 仓库目标布局

```text
apps/
  control-plane/       Rust HTTP/gRPC 控制面
  scheduler/           对账与集群调度进程
  node-agent/          单机训练执行代理
  dyun-agent/          推理节点代理与 runner supervisor
  web/                 React 企业控制台
  desktop/             Tauri 桌面壳
crates/
  types/ contracts/ domain/ application/
  storage/ object-store/ auth/ scheduler/
  worker-control/ dyun-adapter/ http-api/ observability/
proto/moqentra/{common,worker,dyun,cluster}/v1/
python/moqentra_worker/
deploy/{compose,helm,kubernetes}/
tools/{simulators,fixtures,benchmarks}/
```

依赖方向固定为：`types/contracts → domain → application ports → adapters → apps`。领域层不得依赖 Axum、SQLx、Kubernetes、S3、tonic transport、厂商 SDK 或前端类型。

## 3. 执行阶段

| 阶段 | 章节 | 可交付结果 |
|---|---|---|
| P0 基线与契约 | 01–08 | 可编译工作区、版本化契约、租户安全与数据库骨架 |
| P1 视觉 MVP | 09–18 | 数据导入、LabelU-Kit 标注、单机/集群训练、模型注册与转换 |
| P2 应用闭环 | 20–25 | 图编排、dyun 部署、REST/SDK、企业前端、单机安装 |
| P3 平台化 | 19、22、26–29 | HPO/Notebook、推理分层、多集群、桌面、安全与运维 |
| P4 发布准入 | 30–31 | 全量测试、硬件证据、迁移、回滚和发布包 |

章节 12–14 可在 09–11 完成后并行；15–16 依赖 12–14；20 与 17 可并行；21 依赖 20 和 dyun 兼容基线；22 依赖 17、18、21。

## 4. 全局完成标准

- `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo nextest run --workspace` 通过。
- `buf format/lint/breaking`、OpenAPI breaking、JSON Schema golden test 通过。
- Web lint、typecheck、unit、E2E 和依赖审计通过；Python lint、typecheck、unit、integration 通过。
- PostgreSQL repository contract tests 使用真实数据库；S3 测试使用真实 MinIO。
- NVIDIA、AMD、Ascend 仅在对应自托管 runner 的真实训练、恢复和产物验证通过后标记支持。
- 无跨租户读取、对象存储凭据泄漏、日志泄密、任意 URL SSRF、任意容器参数或 shell 注入。
- 单机包与集群包运行相同 API、状态机和 manifest；切换执行适配器不改变业务语义。
- 数据库和对象存储备份恢复演练、N→N+1 升级与回滚、72 小时耐久测试有发布证据。

## 5. 需求追踪

| 需求 | 权威章节 |
|---|---|
| 数据集、标注、质检 | 09–11 |
| 单机与分布式训练 | 12–16 |
| NVIDIA/AMD/华为硬件 | 15–16、30 |
| 模型管理与转换 | 17–18 |
| 应用编排与 dyun-gu | 20–22 |
| 多租户与前端安全 | 07、24、28 |
| 单机/集群同版本 | 14、25–26 |
| 生态集成与客户端 | 23、27 |
| 商用与供应链 | 01、10、28、31 |

最终验收以本 README、各章完成条件和 `31_packaging_migration_release_and_rollback.md` 的发布证据清单共同裁决。
