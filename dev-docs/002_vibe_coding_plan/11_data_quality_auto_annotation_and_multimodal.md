# 11. 数据质检、预标注与多模态扩展

## 1. 质检

规则包括缺标、越界框、自交多边形、非法类别、重复对象、帧范围、类别分布和抽样复核。QualityRun 固定 dataset/annotation version、规则版本与 seed，输出不可变报告。

审核采用 Submit → InReview → Approved/Rejected；Reject 必须有结构化原因并生成返工项。共识标注保存多个独立结果和裁决结果，不覆盖原始证据。

## 2. 预标注

AutoLabelJob 引用已发布 ModelVersion，输出 suggestion layer；人工接受或修改后才进入权威 annotation。置信度、模型版本、阈值和推理参数必须保留。

## 3. 任务

- [x] `QUALITY-001` 实现 `QualityRun`/`QualityRule`/`QualityReport` 可版本化规则、采样与报告。
- [x] `QUALITY-002` 实现 `ReviewItem` 审核队列、带原因和返工任务 ID 的 Reject。
- [x] `QUALITY-003` 实现 `AutoLabelJob` 预标注任务、建议层、接受率和模型版本来源。
- [x] `QUALITY-004` 实现类别分布、缺标、非法类别、越界框、自交多边形、重复对象、帧范围等规则与数据摘要。
- [x] `QUALITY-005` 定义 `MultimodalAnnotation` 与 `MultimodalMeta` 音频时间段、文本、点云扩展 schema；R4 前占位。
- [x] `QUALITY-006` 测试规则重放、模型更换、审核越权和大规模分页占位。

## 11. 完成证据

- 提交：新增 `crates/domain/src/quality.rs`；扩展 `moqentra-types` ID 与 `crates/domain/src/lib.rs`。
- `QualityRun` 固定 `dataset_version_id`、`rule_version`、`seed`；状态机 `Pending → Running → Completed/Failed`。
- 规则覆盖：`MissingLabel`、`OutOfBoundsBox`、`SelfIntersectingPolygon`、`IllegalClass`、`DuplicateObjects`、`FrameRange`、`ClassDistribution`、`SampleReview`。
- `AutoLabelJob` 输出 `AutoLabelSuggestion`（含 `confidence`、`model_version_id`、`inference_params`），支持 `accept` 与状态机。
- `ReviewItem` 支持 `approve` 和带 `reason`/`rework_task_id` 的 `reject`。
- `MultimodalAnnotation` / `MultimodalMeta` 提供音频/文本/点云扩展字段。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：workspace tests 通过；crate graph 合规。

完成条件：任何最终 annotation 可追踪到人工/模型来源、审核记录和使用的 ontology。
