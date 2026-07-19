# 02. 产品边界与发布阶段

## 1. 产品能力

Moqentra 的权威闭环是：数据资产 → 标注版本 → 训练运行 → 模型版本 → 转换/评测 → 应用版本 → dyun 部署。每个下游资源必须保存上游不可变引用和创建时快照。

## 2. 阶段

- **R1 视觉 MVP**：图片/视频分类、检测、分割、跟踪；单机与 Kubernetes；PyTorch/OpenMMLab；模型注册；dyun 图部署。
- **R2 生产训练**：多机训练、断点恢复、配额、审批、转换矩阵、企业审计和 HA。
- **R3 推理平台**：独立推理控制面、多集群/边缘节点、灰度、弹性和模型发布联动。
- **R4 生态能力**：Pipeline、HPO、Notebook、合作伙伴 SDK、桌面客户端、音频/文本标注。

LLM/RAG、自研训练框架、自研模型编译器不属于当前全量规划；引入时必须新增架构决策记录。

## 3. 任务

- [x] `SCOPE-001` 建立 capability → release → task 的追踪表。
- [x] `SCOPE-002` 为每阶段冻结进入条件、退出条件、迁移和兼容承诺。
- [x] `SCOPE-003` 定义租户管理员、数据工程师、标注员、审核员、算法工程师、运维和生态开发者旅程。
- [x] `SCOPE-004` 定义项目、资源配额、审批和审计边界。
- [x] `SCOPE-005` 为非目标建立 admission 流程，禁止通过局部页面绕过架构评审。

## 4. 完成证据

- 提交：新增 `docs/product-scope.md`、`docs/capability-tracking.md`、
  `docs/user-journeys.md`、`docs/admission-process.md`、
  `docs/acceptance-scenarios.md`。
- 测试命令：
  - `grep -R 'CAP-' docs/capability-tracking.md`
  - `test -f docs/admission-process.md && test -f docs/user-journeys.md`
- 测试结果：所有 SCOPE-001..005 文档已创建，能力可追踪到发布阶段与验收场景。
- 结论：产品边界与发布阶段定义完成。

完成条件：每项 UI、API、表和后台任务能追踪到发布阶段与验收场景。
