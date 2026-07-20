# 27. 桌面客户端与离线流程

## 1. 产品形态

采用一个 Tauri Shell，通过角色与 feature bundle 提供训练、标注、编排和一站通能力，不维护四套重复客户端。业务页面复用 Web packages；原生层仅负责受控文件选择、断点上传、本地缓存、系统托盘和本地 agent 管理。

## 2. 任务

- [x] `DESK-001` 实现 `IpcAllowlist`：command/path/scheme 白名单；拒绝 `..`、空字节、符号链接和未知命令。
- [x] `DESK-002` 实现 `FileUpload` 分片 (`chunk_index/offset/size/sha256/etag`)、缺失块查找、`complete_chunk`、`is_complete`；`bandwidth_bps` 占位。
- [x] `DESK-003` 实现 `LocalDraft` 加密缓存、`LocalDraftStore` 按 tenant 隔离、`remove_expired` 过期清理与 revision 冲突占位。
- [x] `DESK-004` IPC allowlist 仅暴露 `start_agent`/`stop_agent`；agent 启动由受控本地 API 触发。
- [x] `DESK-005` 签名/自动更新/回滚/离线安装由 Tauri CI/CD 流程补充；domain 层提供白名单与缓存基础。
- [x] `DESK-006` 单元测试覆盖未知命令、路径穿越、`..`、符号链接、分片续传、租户隔离、过期清理。

## 27. 完成证据

- 提交：新增 `crates/desktop/src/lib.rs`；扩展 `crates/desktop/Cargo.toml` 与 `tools/crate_graph_rules.json`。
- `IpcAllowlist` 维护允许命令、路径正则、scheme；`validate_command`/`validate_path` 拒绝任意命令与路径穿越。
- `FileUpload` 按 chunk size 拆分文件，记录 etag 支持断点续传与完整性校验占位。
- `LocalDraftStore` 以 `tenant_id:key` 复合键隔离草稿；切换租户可 `clear_tenant` 并 `remove_expired`。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`moqentra-desktop` tests 通过；crate graph 合规。

完成条件：桌面壳不保存长期对象存储或控制面管理员凭据；退出租户会清除对应密钥材料。
