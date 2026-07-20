# 18. 模型转换、评测与晋级

## 1. 转换任务

ConversionJob 输入为已验证模型版本与目标 profile，输出派生 artifact。目标包括 TensorRT engine、OpenVINO IR、RKNN、Sophon bmodel、Ascend OM；每种目标使用独立镜像、SDK 版本和真实设备验证。

转换缓存键覆盖输入 digest、工具链 digest、参数、目标芯片和精度。不同 GPU compute capability 或芯片型号不得错误复用。

## 2. 评测

EvaluationRun 固定模型 artifact、数据版本、指标实现、阈值和硬件 profile。晋级 policy 同时检查精度、性能、兼容、安全扫描和审批。

## 3. 任务

- [x] `CONVERT-001` 定义 `ConversionTarget` 与 `ConversionProfile`（target、SDK、toolchain image、chip、precision、dynamic shape、capabilities）。
- [x] `CONVERT-002` 实现 `ConversionJob` 状态机；`complete` 校验输出 artifact scan；`cache_key` 覆盖 source digest、toolchain、chip、precision、参数。
- [x] `EVAL-001` 实现 `EvaluationRun` 与 `EvaluationMetric`（值+容差），支持 seed、硬件 profile、参考输出。
- [x] `EVAL-002` `EvaluationRun` 固定 `dataset_version_id`、`preprocess_version`、`postprocess_version` 与 seed；metric 带 tolerance。
- [x] `PROMOTE-001` 实现 `PromotionPolicy` 的 policy-as-data、required metrics 阈值、approval 与 security scan 校验。
- [x] `PROMOTE-002` 单元测试覆盖转换完成/脏 artifact 拦截与晋级策略；回滚/芯片不匹配集成测试后续补充。

## 18. 完成证据

- 提交：新增 `crates/domain/src/conversion.rs`；扩展 `moqentra-types` ID 与 `crates/domain/src/lib.rs`。
- `ConversionJob` 状态机：`Pending → Running → Succeeded/Failed/Cancelled`。
- `ConversionProfile` 记录目标后端、SDK 版本、工具链镜像 digest、目标芯片、精度、动态 shape 与 capabilities。
- `cache_key` 由 source model version、profile 和参数确定，避免跨 chip/precision 复用。
- `EvaluationRun` 固定 model version、dataset version、seed、pre/postprocess 版本与硬件 profile。
- `PromotionPolicy` 检查 required metric 阈值、人工审批和安全扫描。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：workspace tests 通过；crate graph 合规。

完成条件：转换成功不自动等于可发布；每种 supported 产物都有真实目标硬件加载与推理证据。
