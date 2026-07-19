# 11. 数据质检、预标注与多模态扩展

## 1. 质检

规则包括缺标、越界框、自交多边形、非法类别、重复对象、帧范围、类别分布和抽样复核。QualityRun 固定 dataset/annotation version、规则版本与 seed，输出不可变报告。

审核采用 Submit → InReview → Approved/Rejected；Reject 必须有结构化原因并生成返工项。共识标注保存多个独立结果和裁决结果，不覆盖原始证据。

## 2. 预标注

AutoLabelJob 引用已发布 ModelVersion，输出 suggestion layer；人工接受或修改后才进入权威 annotation。置信度、模型版本、阈值和推理参数必须保留。

## 3. 任务

- [ ] `QUALITY-001` 实现可版本化质量规则、采样和报告。
- [ ] `QUALITY-002` 实现审核队列、返工、争议裁决和审核一致率。
- [ ] `QUALITY-003` 实现预标注任务、批量推理、建议接受率和来源血缘。
- [ ] `QUALITY-004` 实现数据分布、类别不平衡、重复和漂移摘要。
- [ ] `QUALITY-005` 定义音频时间段、文本分类/实体的扩展 schema；R4 前不进入生产 UI。
- [ ] `QUALITY-006` 测试规则重放、模型更换、部分失败、审核越权和大规模分页。

完成条件：任何最终 annotation 可追踪到人工/模型来源、审核记录和使用的 ontology。
