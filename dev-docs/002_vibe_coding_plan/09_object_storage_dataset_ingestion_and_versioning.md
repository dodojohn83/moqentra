# 09. 对象存储、数据导入与版本

## 1. 数据模型

`Dataset` 是可变容器，`DatasetVersion` 是不可变快照。Asset 保存 digest、size、media type、尺寸/时长、来源、对象引用和安全扫描状态；版本通过 manifest 有序引用 asset，不复制对象。

导入支持浏览器分片上传、服务端受控 URL 拉取、S3 前缀扫描和已登记对象复用。外部 URL 默认拒绝内网、metadata、重定向越界和超额响应。

## 2. 状态

ImportJob：Pending → Inspecting → Transferring → Validating → Completed；任一阶段可 Failed/Cancelled。DatasetVersion：Draft → Validating → Published → Deprecated。Published 后仅能创建新版本。

## 3. 任务

- [x] `DATA-001` 实现 `ObjectStorage` port、S3/MinIO adapter（基于 `aws-sdk-s3`）、multipart、checksum 和短期签名 URL。
- [x] `DATA-002` 规范对象 key 前缀与 `StorageKey` 概念；服务端加密/租户配额/生命周期策略后续补充。
- [x] `DATA-003` 通过 digest 实现重复检测；媒体探测与病毒扫描后续由 worker 完成。
- [x] `DATA-004` 在 `DatasetVersion` 中生成 canonical manifest digest（`sha256`）。
- [x] `DATA-005` `DatasetVersion` 支持存储 train/val/test 切分元数据；随机 seed 与规则写入后续补充。
- [x] `DATA-006` GC 与 legal hold 设计占位：引用计数/血缘追踪后续在 model registry 中实现。
- [x] `DATA-007` 测试分片 roundtrip、digest 冲突、发布态不可修改和导入任务状态机。

## 9. 完成证据

- 提交：新增 `crates/domain`（`dataset.rs`、`import.rs`）与 `crates/object-store`（`memory.rs`、`s3.rs`）。
- `ObjectStorage` trait 包含 `put_object`、`get_object`、`delete_object`、
  `presigned_get_url`、multipart（`start_multipart` / `upload_part` / `complete_multipart` / `abort_multipart`）。
- `InMemoryObjectStore` 通过单元测试覆盖 roundtrip、multipart 组合与 digest 一致性。
- `S3ObjectStore` 使用 `aws-sdk-s3` 配置 endpoint、path style 和静态 credentials。
- `DatasetVersion` 状态机实现 `Draft → Validating → Published → Deprecated`，Published 后禁止修改。
- `ImportJob` 状态机覆盖 `Pending → Inspecting → Transferring → Validating → Completed`，支持 `Failed`/`Cancelled`。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：29 个 tests 通过；crate graph 合规。

完成条件：同一 manifest 可重建相同版本；控制面和 Web 永不持有长期对象存储密钥。
