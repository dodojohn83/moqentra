# 03. 单仓工作区与 CI

## 1. 工作区

Rust、Web、Python worker、Proto、部署清单和测试工具同仓。Rust crate 使用 `moqentra-*` 前缀；生成代码放独立 crate/package，不允许手改。

## 2. CI 门禁

- Rust：fmt、clippy、nextest、doc、deny、最小版本策略、关键 feature build。
- Contracts：Buf format/lint/breaking、OpenAPI breaking、Schema golden。
- Web：pnpm frozen install、ESLint、TypeScript、Vitest、Playwright、bundle budget。
- Python：uv lock、Ruff、mypy/pyright、pytest、wheel build。
- Supply chain：secret scan、SAST、SBOM、许可证、镜像和 IaC 扫描。

## 3. 任务

- [ ] `BOOT-001` 创建目录、workspace manifest、统一格式和 lint 配置。
- [ ] `BOOT-002` 建立变更路径过滤但保留契约变更的全链路验证。
- [ ] `BOOT-003` 配置 PostgreSQL、MinIO、OIDC 测试服务和 Testcontainers。
- [ ] `BOOT-004` 配置无硬件 CI 与 NVIDIA/AMD/Ascend 自托管 runner 分组。
- [ ] `BOOT-005` 缓存仅加速，不允许缓存命中影响正确性。
- [ ] `BOOT-006` 产出版本化 OpenAPI、descriptor set、JSON Schema 和 TypeScript/Python SDK。
- [ ] `BOOT-007` 建立 required checks、CODEOWNERS 和架构边界检查。

完成条件：空骨架在 Linux x86_64/aarch64 通过门禁；硬件任务缺少 runner 时明确 pending，不伪造成功。
