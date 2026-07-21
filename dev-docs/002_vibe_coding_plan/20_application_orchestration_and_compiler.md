# 20. 算法应用编排与编译器

## 1. ApplicationSpec

平台图节点含稳定 type/version、具名输入输出端口、参数 schema、资源需求和 capability constraints。边必须类型兼容；图必须无环；secret、模型和流地址使用资源引用，不能内嵌明文。

UI 使用 React Flow 编辑纯数据 spec。服务端是唯一权威编译器：normalize → schema/type validate → capability resolve → graph lowering → canonicalize → sign。

## 2. 任务

- [x] `APP-001` 定义 `ApplicationNode`、`Port`、参数 schema、node type/version、deprecated 标志与 capability constraints。
- [x] `APP-002` 实现 `Application`/`ApplicationVersion`；发布后 `ApplicationVersion` 不可变（spec 在 `new` 时校验并固定 digest）。
- [x] `APP-003` `ApplicationVersion::canonical_digest` 基于确定性 JSON 序列化生成 digest；相同输入产生相同 digest。
- [x] `APP-004` `ResourceRef` 支持 Model、Dataset、Stream、Secret、Device 引用；`Binding` 用于部署期绑定。
- [x] `APP-005` `moqentra-application` 实现 `ApplicationCompiler`（compile/diff）、`InMemoryApplicationRegistry`（create/publish/bindings）；模板/导入导出后续补充。
- [x] `APP-006` 单元测试覆盖图环、端口类型不匹配、缺失节点和发布冻结。

## 20. 完成证据

- 提交：新增 `crates/domain/src/application.rs`；扩展 `moqentra-types` ID 与 `crates/domain/src/lib.rs`。
- `ApplicationNode` 含 stable `node_type`/`version`、输入输出 `Port`、参数、资源请求、`capabilities`、`bindings` 与 `ResourceRef`。
- `ApplicationSpec` 校验：检测环、缺失节点、源/目标端口存在性与类型一致性。
- `ApplicationVersion` 在构造时验证 spec 并计算 digest；`publish` 后不可再次发布。
- `Application` 聚合版本并记录 `latest_published`。
- `Binding` 将节点 slot 解析为部署期 `ResourceRef`。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：workspace tests 通过；crate graph 合规。

完成条件：浏览器不能直接生成生产 dyun YAML；编译失败不得创建可部署版本。
