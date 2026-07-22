# 04. Python Worker、gRPC 控制与本地 OCI 训练

## 1. Worker 协议

- [x] `R1-TRAIN-001` 扩展 `WorkerCapabilities/v1`：build/contract version、框架、硬件、驱动、runtime、模型格式、collective backend、设备内存和最大并行数。
- [x] `R1-TRAIN-002` 完成 `WorkerAgentService.OpenStream` 双向流：Hello、Heartbeat、Lease、Command、Ack、Progress、LogChunk、MetricBatch、Result 和 Drain。
- [x] `R1-TRAIN-003` 所有消息关联 node、command、sequence 和 fencing token；`SessionManager` 拒绝重复/乱序 heartbeat、未知 command 的 progress/log/metric、重复 result，并维护 command/ack/complete 状态。
- [x] `R1-TRAIN-004` 协议健壮性（已实现部分）：gRPC 双向流使用 tonic `Endpoint`；node-agent 以 10s 心跳、5s 重连指数退避保持连接；outbound 有界队列（64）；message-level sequence 校验。`mTLS`、`send credit` 和 `最大帧` 留待后续连接层统一实现。
- [x] `R1-TRAIN-005` 取消语义：`SessionManager::cancel_command` 查找活动 command，标记 cancelled 并发送 `Drain { graceful: true, command_id }`；node-agent `cancel_container` 先 SIGTERM，30s grace period 后未退出再 SIGKILL；`AgentSession` 对重复/已完成 command 拒绝，最终 Result 幂等写入 completed。

## 2. Python runtime 与模板

- [x] `R1-TRAIN-006` 生成 Python gRPC stubs（`moqentra.worker.v1`、`moqentra.common.v1`）并添加 `moqentra_worker.grpc_client.WorkerAgentClient`：建立 insecure channel、发送 Hello/Capabilities、10s 心跳续活、分发 Command 到 handler 并回 Ack/Result、处理 Drain；不访问控制面数据库。
- [ ] `R1-TRAIN-007` 实现 Worker 输入物化与输出提交：输入只读、输出临时写入、checkpoint/content digest 校验、成功后生成 `ModelArtifactManifest/v1`。
- [ ] `R1-TRAIN-008` 用真实 PyTorch 实现 ResNet18 分类模板，固定 seed、数据预处理、参数 schema、指标、checkpoint、ONNX export 和环境 manifest。
- [ ] `R1-TRAIN-009` 实现 SSDlite320 MobileNetV3 检测模板，输出 mAP 所需预测、最佳 checkpoint 和 ONNX；作为 R1 黄金训练/推理模型。
- [ ] `R1-TRAIN-010` 实现 DeepLabV3 MobileNetV3 分割模板，输出 mIoU、mask 预览和 ONNX。
- [ ] `R1-TRAIN-011` 使用仓库内确定性生成器创建有分类、框、mask 的小型视觉 fixture；记录生成 seed、schema 和许可，不下载来源不明数据或权重。
- [ ] `R1-TRAIN-012` SIGTERM、gRPC 断线和 checkpoint interval 同时发生时保证 manifest 不引用半写文件；临时对象由重试或 GC 接管。

## 3. Node Agent 与容器执行

- [x] `R1-LOCAL-001` Node Agent 探测 CPU、磁盘、Docker/Podman、NVIDIA driver/runtime 和设备健康，注册稳定 NodeId 与能力快照。
- [x] `R1-LOCAL-002` 实现实际 OCI launch：`LocalExecutor::run_container` 校验 `image`/`image_digest`、拒绝 root、只读 rootfs、drop capabilities、no-new-privileges、network none、pids limit；argv 直接传递；镜像以 `@digest` 形式传给 podman/docker。
- [x] `R1-LOCAL-003` node-agent `client.rs` 为每个 attempt 自动创建 `input`（ro）、`output`/`checkpoint`（rw）受控 workspace 挂载；拒绝相对路径和 workspace 外 source；`LocalExecutor` 绑定挂载校验 canonicalize 与 path traversal。
- [x] `R1-LOCAL-004` `LocalExecutor::allocate` 已实现 CPU/内存/设备原子分配与失败回滚；node-agent 将分配的 `device_uuids` 以 `NVIDIA_VISIBLE_DEVICES` 注入容器；`release` 回收资源。
- [x] `R1-LOCAL-005` node-agent 在 `run_container_command` 中启动 `tokio::process::Child`，用 `BufReader::lines` 异步读取 stdout/stderr，通过有界 gRPC `LogChunk` channel 上传，背压由 bounded channel 自然限制，避免无界内存。
- [ ] `R1-LOCAL-006` 进程重启后按 runtime labels 对账 active attempt、容器和 allocation；仅清理有本平台 ownership label 且租约过期的孤儿。

## 4. 调度与状态收敛

- [ ] `R1-TRAIN-013` scheduler 从 PostgreSQL 读取 queued job，校验冻结数据版本、镜像 digest、配额和 capability 后创建 attempt/lease。
- [ ] `R1-TRAIN-014` 状态流固定为 `Draft → Queued → Admitted → Running → Succeeded/Failed/Cancelled`；重试创建新 attempt，不倒退已终结 attempt。
- [ ] `R1-TRAIN-015` metrics 有名称/标签 allowlist、每批上限和下采样；日志、指标、checkpoint cursor 支持从断点继续读取。
- [ ] `R1-TRAIN-016` Worker Result 只触发 Artifact validation Operation；校验成功后才原子完成训练并创建唯一 Model Version。

## 5. 完成条件与测试

- RTX 3090 上三个模板各完成一次真实训练；保存 GPU、driver、CUDA、PyTorch、镜像 digest、耗时、指标和产物摘要。
- 分类/检测固定 seed 重跑得到结构一致的 manifest；浮点指标允许记录容差，但 Artifact 内容摘要必须可解释。
- 覆盖重复 Result、旧 token、Worker 断线、控制面重启、Node Agent 重启、磁盘满、取消竞争和容器异常退出。
- `tools/benchmarks/run-hardware-test.sh nvidia` 执行真实测试，不再输出 placeholder 成功。
