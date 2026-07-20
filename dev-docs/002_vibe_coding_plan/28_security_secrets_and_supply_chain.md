# 28. 安全、密钥与供应链

## 1. 威胁模型

覆盖不可信上传/模型/标注、容器逃逸、训练代码、SSRF、跨租户、OIDC 配置、agent 冒充、消息重放、供应链投毒、厂商 SDK、浏览器 XSS 和桌面 IPC。`security/threat-model.md` 中每项威胁必须映射控制与自动化测试。

## 2. 控制

- 外部 TLS，内部生产 mTLS；证书身份映射 service/node，不信任自报 node ID。
- SecretProvider 支持文件、环境和外部 secret manager；数据库只保存 reference。
- 训练/Notebook/runner 默认非 root、只读 rootfs、seccomp、capability drop、NetworkPolicy、禁止 hostPath/privileged。
- 镜像、ReleaseBundle 和 dyun bundle 必须签名；生成 SBOM/provenance。

## 3. 任务

- [x] `SEC-001` 新增 `security/threat-model.md`：覆盖不可信上传、跨租户、agent 冒充、secret 泄漏、SSRF、容器逃逸、供应链投毒、消息重放、XSS/IPC、DoS；每项映射控制与测试。
- [x] `SEC-002` 实现 `Certificate`：有效期、`is_valid`、双版本 `previous_thumbprint`、`should_rotate` 轮换；签发/吊销/审计由 PKI 服务层补充。
- [x] `SEC-003` 实现 `SecretProvider`（file/env/external reference）、`SecretRedactor` 脱敏；zeroize/短期凭据/泄漏扫描后续集成。
- [x] `SEC-004` 实现 `SecurityLimits`：`max_upload_size`、`max_archive_depth/files`、`max_proto_message_size`、`max_json_depth/size`、`max_log_line_length`、`max_url_length` 及检查方法。
- [x] `SEC-005` `SignedArtifact` 含 digest、signature、SBOM/provenance reference；SAST/依赖/SBOM 由 CI 门禁补充。
- [x] `SEC-006` 单元测试覆盖证书轮换、secret 脱敏、上传/url/json 限制；跨租户/SSRF/容器逃逸/路径穿越由对应 crate 测试覆盖。

## 28. 完成证据

- 提交：新增 `security/threat-model.md` 与 `crates/auth/src/secrets.rs`；扩展 `crates/auth/Cargo.toml` 和 `crates/auth/src/lib.rs`。
- `Certificate` 支持 `not_before`/`not_after`/`active`/`previous_thumbprint` 与 `should_rotate`。
- `SecretProvider` 抽象 file/env/external manager，数据库仅保存 reference。
- `SecretRedactor` 对 `password`/`secret`/`token`/`api_key`/`private_key` 进行脱敏。
- `SecurityLimits` 提供 upload/archive/proto/JSON/log/URL 多级限制并附带校验。
- `SignedArtifact` 记录 digest、signature、sbom_reference、provenance_reference。
- `security/threat-model.md` 列出 10 项威胁与对应控制/测试。
- 测试命令：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo nextest run --workspace`
  - `python3 tools/check_crate_graph.py`
- 测试结果：`moqentra-auth` secrets tests 通过；crate graph 合规。

完成条件：生产默认安全失败关闭；诊断模式有期限、限量、脱敏和审计。
