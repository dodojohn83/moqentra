# R1 风险登记

| ID | 风险描述 | 触发条件 | 影响 | 缓解措施 | Owner | 003 任务 |
|---|---|---|---|---|---|---|
| RISK-001 | LabelU-Kit v5.11.1 无法按预期发布或存在许可证不兼容 | 上游发布延迟/许可证变更 | 标注平台 UI 集成延迟或需回退 | 固定 v5.11.0 并记录许可证；维护 LabelU JSON adapter 作为回退 | R1-LABEL-001 负责人 | R1-LABEL-001 |
| RISK-002 | `dyun-gu` runner 无发布 tag，API 不稳定 | 上游未打 tag | 推理链路无法稳定编译或运行 | 固定到上游已知 good commit；通过 `DyunAgentService` 版本协商隔离；保留 contract test 占位 | R1-DYUN-001/005 负责人 | R1-DYUN-001, R1-DYUN-005 |
| RISK-003 | k3s kubeconfig 权限不足或 Volcano 未安装 | 目标集群缺少管理员权限 | Kubernetes 执行测试无法运行 | preflight 检查权限和 CRD；readiness 明确失败；本地 OCI 作为 R1 主路径 | R1-K8S-001 负责人 | R1-K8S-001, R1-ONEBOX-004 |
| RISK-004 | RTX 3090 为消费级 GPU，不能作为数据中心支持证据 | 仅在 RTX 3090 上跑通训练 | R1 错误声明数据中心 NVIDIA 支持 | 支持矩阵保持 `preview`；测试报告明确记录 RTX 型号、driver、CUDA | R1-TRAIN-008/009 负责人 | R1-TRAIN-008 ~ R1-TRAIN-010, R1-E2E-005 |
| RISK-005 | 真实 RTSP/RTMP 源不稳定或无授权 | 外部流不可用 | dyun 真实媒体链路验收失败 | 使用许可明确的合成视频生成 RTSP 输入；不依赖外部不稳定流 | R1-DYUN-008 负责人 | R1-DYUN-008 |
| RISK-006 | PostgreSQL/MinIO/Dex 在 CI 中不可用或配置漂移 | 容器启动失败 | 集成测试、持久化验收失败 | Onebox Compose 固定版本；health/readiness 显式检查；contract tests 使用 testcontainers/fakes | R1-DB-001 负责人 | R1-DB-001, R1-ONEBOX-001 |
| RISK-007 | 容器运行时（Docker/Podman）权限或 NVIDIA Container Toolkit 未就绪 | 本地无 root/NVIDIA 环境 | 本地 OCI 训练无法启动 | preflight 检查并给出修复方法；Node Agent 探测后降级为 CPU-only capability | R1-LOCAL-001 负责人 | R1-LOCAL-001, R1-ONEBOX-004 |
| RISK-008 | 合成视觉 fixture 的 license 或来源不明 | 使用外部数据集 | 发布包侵权风险 | 使用仓库内确定性生成器创建；记录 seed/schema/license | R1-TRAIN-011 负责人 | R1-TRAIN-011 |
| RISK-009 | OIDC issuer JWKS 轮换或网络不可达 | 生产 issuer 变更 | 登录/认证失败 | JWKS 缓存与轮换实现；失败降级到明确 401，不回到 HMAC 开发 token | R1-IAM-001 负责人 | R1-IAM-001 |
| RISK-010 | RLS 配置错误导致跨租户读取 | 迁移或连接池复用 | 数据隔离破坏 | fail-closed RLS；连接池 reset；contract tests 验证连接复用不串租户 | R1-DB-005 负责人 | R1-DB-005, R1-DB-006 |
