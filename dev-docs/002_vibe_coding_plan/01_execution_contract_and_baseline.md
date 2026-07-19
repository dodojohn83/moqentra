# 01. 执行契约与基线冻结

## 1. 权威基线

- Rust 固定 `1.94.1`，edition 与 workspace lint 统一；依赖由 `Cargo.lock` 固定。
- dyun-gu 固定到已核对 commit `f6d6cb06e07b8dde332ed585a8250207501898dc`，升级必须运行 GraphSpec、热更新和媒体链路契约测试。
- LabelU-Kit 首个评估基线为 `v5.11.1`；实际引入时固定精确版本并保存许可证清单。
- Python、PyTorch、ROCm、CUDA、Ascend、Kubernetes、Volcano 和设备插件按厂商兼容矩阵成组冻结，禁止独立追逐最新版。
- 所有生产 OCI 镜像使用 digest；模型与数据集使用内容摘要。

## 2. 支持等级

`supported` 必须有真实硬件 CI、精度基线、故障恢复、监控和升级证据；`preview` 允许人工实验室验证；`compile-only` 仅表示可构建；`mock` 仅用于无硬件测试。

## 3. 任务

- [x] `BASE-001` 建立版本矩阵，记录核对日期、官方来源、EOL 和组合约束。
- [x] `BASE-002` 建立许可证 allow/deny 清单，覆盖 Rust、NPM、Python、容器和模型。
- [x] `BASE-003` 冻结支持的 OS、CPU 架构、Kubernetes 和对象存储协议版本。
- [x] `BASE-004` 为 NVIDIA、AMD、Ascend 定义 supported/preview/compile-only 状态。
- [x] `BASE-005` 定义分支、提交、变更日志、版本号和兼容窗口。
- [x] `BASE-006` 建立外部 SDK/硬件阻塞登记；不得用跳过测试关闭门禁。

## 4. 完成证据

- 提交：`baseline/` 目录下新增 `version-matrix.toml`、`licenses.toml`、`platform-matrix.toml`、
  `hardware-support.toml`、`release-policy.md`、`external-blockers.toml` 和 `README.md`。
- 测试命令：
  - `find baseline -name '*.toml' -exec python3 -c 'import tomllib,sys; tomllib.load(open(sys.argv[1],"rb"))' {} \;`
  - `grep -R 'latest' baseline/ || true`
- 测试结果：所有 TOML 文件通过语法解析；未使用未限定的 `latest` 标签或 floating git branch。
- 结论：基线冻结完成，满足任务 01 完成条件。

完成条件：任何开发者能从版本矩阵重建工具链；不存在 floating git branch、`latest` 镜像或未分类许可证。
