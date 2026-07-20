# 17. 模型注册、产物与血缘

## 1. 模型层级

Model 是业务名称；ModelVersion 是不可变发布单元；Artifact 是内容寻址文件。版本保存训练 job、数据/标注版本、代码/镜像、超参、指标、输入输出签名、许可证和创建者。

状态：Draft → Validating → Ready → Approved → Deprecated/Rejected。部署只能引用 Approved，开发租户可由策略允许 Ready。

## 2. 任务

- [x] `MODEL-001` 实现 `Model`、`ModelVersion`、`Artifact`、`ModelSignature`、`TensorSpec` 和 `ModelLineage`。
- [x] `MODEL-002` `validate` 校验 artifact `scan_status` 为 clean、lineage digest 与 dataset 版本非空；恶意内容/反序列化策略后续由 object-store/adapter 层补充。
- [x] `MODEL-003` `Artifact` 支持任意媒体类型；ONNX opset/dynamic shape 以 signature metadata/artifact 形式记录。
- [x] `MODEL-004` 实现 `validate → mark_ready → approve` / `reject` / `deprecate` 状态机；引用保护后续在 deployment 层实现。
- [x] `MODEL-005` 实现 `ModelArtifactManifest` 包含版本、artifact 列表、signature、lineage；SBOM/license `Attachment` 已定义。
- [x] `MODEL-006` 单元测试覆盖生命周期、脏 artifact 拦截与未就绪审批拒绝。

## 17. 完成证据

- 提交：新增 `crates/domain/src/model_registry.rs`；扩展 `moqentra-types` ID 与 `crates/domain/src/lib.rs`。
- `Model` 聚合 `ModelVersionId` 与 `latest_approved`。
- `ModelVersion` 状态机：`Draft → Validating → Ready → Approved → Deprecated/Rejected`。
- `validate` 要求所有 artifact 已扫描为 clean 且 lineage 完整。
- `approve(user_id)` 记录审批人。
- `ModelLineage` 关联 training job、experiment、dataset version、annotation project 与 base model version。
- `ModelArtifactManifest` 包含所有可复现字段与 signature。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：workspace tests 通过；crate graph 合规。

完成条件：任何部署 artifact 可回溯到模型、转换、训练、数据和审核人；对象缺失会阻断发布。
