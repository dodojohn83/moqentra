# 11. R1 验收、CI 与发布证据

## 1. 自动质量门禁

- [ ] `R1-QA-001` Rust：`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo nextest run --workspace`。
- [ ] `R1-QA-002` 契约：Buf format/lint/breaking、JSON Schema validation/golden、OpenAPI lint/breaking 和生成代码无差异。
- [ ] `R1-QA-003` Web：lint、typecheck、Vitest、生产 build、Playwright E2E、依赖和 bundle secret scan。
- [ ] `R1-QA-004` Python：Ruff、mypy strict、pytest unit/integration、wheel build、protobuf 生成差异和依赖审计。
- [ ] `R1-QA-005` 真实适配器：PostgreSQL、MinIO、Dex、Docker/Podman、k3s/Volcano 和 dyun-gu contract suites。
- [ ] `R1-QA-006` CI 失败时保存日志和测试报告；禁止 `continue-on-error`、空硬件脚本或用 simulator 关闭真实环境门禁。

## 2. 黄金验收 TAS-R1-E2E-001

- [ ] `R1-E2E-001` 在干净 Onebox 主机完成安装，所有 migration、health、readiness 和 capability 检查通过。
- [ ] `R1-E2E-002` 通过 Dex/OIDC 登录，创建 tenant/project 并配置四类业务角色。
- [ ] `R1-E2E-003` 上传图片和视频、模拟一次中断续传，完成媒体探测并冻结 Dataset Version。
- [ ] `R1-E2E-004` 标注检测任务，经历提交、审核退回、修订、通过和 COCO round-trip。
- [ ] `R1-E2E-005` RTX 3090 完成 SSDlite 检测训练；Web 展示日志、指标和 checkpoint，生成可追溯 Model Version。
- [ ] `R1-E2E-006` 转换并校验 ONNX，完成模型发布审批；Artifact digest、signature 和 lineage 齐全。
- [ ] `R1-E2E-007` React Flow 创建 RTSP→检测→跟踪→OSD→RTMP 应用，编译出签名且摘要稳定的 bundle。
- [ ] `R1-E2E-008` dyun-gu 运行合成视频流，保存输出证据；停止和相同版本重新发布成功。
- [ ] `R1-E2E-009` 重启全部控制面组件，资源、Operation、训练、模型和 deployment 状态恢复且无重复资产。
- [ ] `R1-E2E-010` 同一检测 JobSpec 在单节点 k3s/Volcano 再运行一次，输出契约与本地执行一致。

## 3. 故障、安全与运维场景

- [ ] `R1-E2E-011` 分别在上传、冻结、训练、Artifact 校验和部署中止相关进程，验证幂等恢复和 fencing。
- [ ] `R1-E2E-012` 完成跨租户 API/RLS/S3/log/metric 测试以及 CSRF、XSS、SSRF、路径、命令和恶意文件测试。
- [ ] `R1-E2E-013` 完成 PostgreSQL/MinIO 备份、删除测试环境、恢复和摘要/引用校验。
- [ ] `R1-E2E-014` 执行 N→N+1 expand-first 升级、旧应用回滚和 ApplicationVersion 回滚。
- [ ] `R1-E2E-015` 连续运行 72 小时，期间注入 Worker/agent/stream 短暂中断；无无界增长、永久孤儿或不可解释状态。

## 4. 证据清单

每个候选版本必须归档：

- Git commit、dirty 状态、release/version/platform/hardware matrix；
- Rust/Proto/Web/Python/安全/许可证测试报告；
- OCI image digest、SBOM、provenance、签名和扫描报告；
- PostgreSQL migration 状态、MinIO bucket/version 配置；
- RTX 型号、driver、CUDA、PyTorch、训练命令、指标、checkpoint 和 Artifact digest；
- Kubernetes、Volcano、device plugin、Job/Pod 诊断和输出 manifest；
- ApplicationSpec、GraphSpec、DyunGraphBundle、签名、dg capability 和媒体输出；
- 故障注入、72 小时耐久、备份恢复、升级与回滚报告；
- 未关闭 blocker、已知限制和支持等级。

## 5. 发布裁决

`ReleaseGate::is_ready` 必须读取或验证真实报告引用，而不是由人工设置布尔值。任一高危安全问题、缺失 R1 黄金步骤、未验证迁移、无法恢复的权威状态或无解释的 Artifact 摘要差异都阻止 release candidate。

RTX 3090 验收只支持把 NVIDIA GeForce R1 功能标记为 `preview`。数据中心 NVIDIA、AMD、Ascend、多节点训练和推理零停机能力不得出现在 R1 GA 声明中。
