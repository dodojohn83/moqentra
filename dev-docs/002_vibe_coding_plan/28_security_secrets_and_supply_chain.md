# 28. 安全、密钥与供应链

## 1. 威胁模型

覆盖不可信上传/模型/标注、容器逃逸、训练代码、SSRF、跨租户、OIDC 配置、agent 冒充、消息重放、供应链投毒、厂商 SDK、浏览器 XSS 和桌面 IPC。`security/threat-model.md` 中每项威胁必须映射控制与自动化测试。

## 2. 控制

- 外部 TLS，内部生产 mTLS；证书身份映射 service/node，不信任自报 node ID。
- SecretProvider 支持文件、环境和外部 secret manager；数据库只保存 reference。
- 训练/Notebook/runner 默认非 root、只读 rootfs、seccomp、capability drop、NetworkPolicy、禁止 hostPath/privileged。
- 镜像、ReleaseBundle 和 dyun bundle 必须签名；生成 SBOM/provenance。

## 3. 任务

- [ ] `SEC-001` 完成威胁建模、数据分类和安全边界图。
- [ ] `SEC-002` 实现证书签发、轮换、吊销、双版本过渡和审计。
- [ ] `SEC-003` 实现 secret redaction、zeroize、短期凭据和泄漏扫描。
- [ ] `SEC-004` 对上传、archive、模型、URL、Proto、JSON 和日志设置层级/总量限制。
- [ ] `SEC-005` 建立 SAST、依赖漏洞、许可证、SBOM、镜像、IaC、签名发布门禁。
- [ ] `SEC-006` 执行租户越界、SSRF、XSS、命令注入、路径穿越和容器逃逸测试。

完成条件：生产默认安全失败关闭；诊断模式有期限、限量、脱敏和审计。
