# 09. Onebox、Kubernetes 与发布打包

## 1. OCI 镜像

- [ ] `R1-PKG-001` 为 control-plane、scheduler、node-agent、dyun-agent、web、migration 和三个 PyTorch 模板提供多阶段 Dockerfile。
- [ ] `R1-PKG-002` 基础镜像和生产依赖固定 digest；运行用户非 root、rootfs 最小化、无编译器/包管理器，暴露 health 和版本信息。
- [ ] `R1-PKG-003` Worker 镜像固定 PyTorch 2.7/CUDA 12.8 兼容组，启动时输出驱动/运行时诊断；不在同一镜像混装 CUDA、ROCm 和 CANN。
- [ ] `R1-PKG-004` 构建生成 SBOM、provenance、漏洞/许可证报告和签名；ReleaseManifest 引用真实报告而非布尔占位。

## 2. Onebox

- [ ] `R1-ONEBOX-001` Compose 完整包含 PostgreSQL、MinIO、Dex、migration、control-plane、scheduler、web、node-agent 和 dyun-agent；移除 `latest` 和一个变量复用多个服务镜像的配置。
- [ ] `R1-ONEBOX-002` `init.sh` 幂等生成 secret、管理员凭据和 TLS，文件权限最小；重复执行不覆盖已有数据或凭据。
- [ ] `R1-ONEBOX-003` 启动顺序使用 dependency health 和 migration completion；control-plane readiness 实际检查数据库、对象存储和必要后台单元。
- [ ] `R1-ONEBOX-004` preflight 检查端口、磁盘、内存、Docker、NVIDIA driver/runtime、RTSP/RTMP 测试依赖和架构；失败给出明确修复方法。
- [ ] `R1-ONEBOX-005` 提供安装、状态、日志、停止、升级、备份、恢复和卸载 runbook；卸载默认保留数据卷。
- [ ] `R1-ONEBOX-006` 一条命令启动后自动执行 smoke：OIDC、迁移、MinIO、控制面和 agent capability 均通过。

## 3. Helm/Kubernetes

- [ ] `R1-HELM-001` Chart 包含所有控制面组件、migration Job、ServiceAccount/RBAC、NetworkPolicy、PDB、resources、probes 和 topology 设置。
- [ ] `R1-HELM-002` production values 强制外部 PostgreSQL/S3/OIDC、TLS、secret refs、资源限制和 NetworkPolicy；缺项 `helm lint/template` 失败。
- [ ] `R1-HELM-003` node-agent/dyun-agent 使用明确 DaemonSet/Deployment 策略和最小 host 权限；禁止默认挂载 Docker socket 到不需要的组件。
- [ ] `R1-HELM-004` 提供 NVIDIA device plugin、Volcano 和 storage 的前置检查与兼容说明，不把第三方组件静默安装到现有集群。
- [ ] `R1-HELM-005` 支持 N/N+1 expand-first 升级：先 migration，再兼容代码；回滚旧代码可读取扩展 schema。

## 4. 完成条件与测试

- 全新 x86_64 Linux 主机按 Onebox runbook 完成黄金路径，不能依赖源码目录或开发工具链。
- k3s 使用 Helm 完成安装、真实训练和 dyun 部署；chart 重装/升级不丢失外部数据。
- 所有镜像通过 signature、SBOM、license 和高危漏洞门禁，无 floating tag。
- 非开发人员仅按 runbook 能完成备份恢复、状态诊断和一次应用回滚。
