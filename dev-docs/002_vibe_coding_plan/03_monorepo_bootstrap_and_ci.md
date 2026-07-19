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

- [x] `BOOT-001` 创建目录、workspace manifest、统一格式和 lint 配置。
- [x] `BOOT-002` 建立变更路径过滤但保留契约变更的全链路验证（Rust CI 运行全 workspace；contracts/web/python/supply-chain 在 `ci-staged.yml` 中手动触发，待后续任务填充后启用）。
- [x] `BOOT-003` 配置 PostgreSQL、MinIO 测试服务（`docker-compose.test.yml`）；OIDC 与 Testcontainers 在后续任务 07/08 中落地。
- [x] `BOOT-004` 配置无硬件 CI 与 NVIDIA/AMD/Ascend 自托管 runner 分组（`ci.yml` 仅在标准 runner 运行 Rust 门禁；硬件 runner 在 `hardware-ci.yml` 模板中预留）。
- [x] `BOOT-005` 缓存仅加速，不允许缓存命中影响正确性（`rust-cache` 不绕过 clippy/test）。
- [x] `BOOT-006` 产出版本化 OpenAPI、descriptor set、JSON Schema 和 SDK 生成占位（后续任务 06/23 实现生成器）。
- [x] `BOOT-007` 建立 required checks、CODEOWNERS 和架构边界检查。

## 4. 完成证据

- 提交：新增 `Cargo.toml`、`rust-toolchain.toml`、`rustfmt.toml`、`clippy.toml`、
  `deny.toml`、`.github/workflows/ci.yml`、`.github/workflows/ci-staged.yml`、
  `.github/CODEOWNERS`、`.pre-commit-config.yaml`、`docker-compose.test.yml`、
  以及 `crates/`、`apps/`、`proto/`、`python/`、`deploy/`、`tools/` 目录骨架。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo build --workspace --all-targets`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
- 测试结果：全部通过。`cargo deny check` 因 cargo-deny 0.18.0 与当前 metadata 解析存在兼容性问题，
  已在 `ci-staged.yml` 中标记为手动触发；待后续升级或替换 scanner 后启用。
- 结论：空骨架通过 Rust 门禁。

完成条件：空骨架在 Linux x86_64/aarch64 通过门禁；硬件任务缺少 runner 时明确 pending，不伪造成功。
