# 20. 算法应用编排与编译器

## 1. ApplicationSpec

平台图节点含稳定 type/version、具名输入输出端口、参数 schema、资源需求和 capability constraints。边必须类型兼容；图必须无环；secret、模型和流地址使用资源引用，不能内嵌明文。

UI 使用 React Flow 编辑纯数据 spec。服务端是唯一权威编译器：normalize → schema/type validate → capability resolve → graph lowering → canonicalize → sign。

## 2. 任务

- [ ] `APP-001` 定义节点目录、端口类型、参数 schema、版本和弃用策略。
- [ ] `APP-002` 实现 Application/ApplicationVersion，发布后不可变。
- [ ] `APP-003` 实现确定性编译器与字段级 diagnostics；相同输入必须产生相同 digest。
- [ ] `APP-004` 把模型、流、设备和 secret 引用解析为部署期 binding。
- [ ] `APP-005` 实现草稿 diff、版本比较、模板、复制和导入导出。
- [ ] `APP-006` 建立 compiler golden、属性测试、恶意大图和循环/端口错误测试。

完成条件：浏览器不能直接生成生产 dyun YAML；编译失败不得创建可部署版本。
