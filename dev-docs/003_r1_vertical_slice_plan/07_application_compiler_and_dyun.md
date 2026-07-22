# 07. Application 编译与 dyun-gu 真实运行

## 1. Application API 与编译器

- [x] `R1-APP-001` 持久化 Application、不可变 ApplicationVersion、ComponentCatalog、Binding、Deployment 和状态历史。
- [x] `R1-APP-002` 建立版本化组件目录，首批包含 RTSP source、decode、preprocess、inference、postprocess、tracker、OSD、encode 和 RTMP sink；端口、参数、能力和资源 schema 明确。
- [~] `R1-APP-003` 编译前校验 DAG、端口类型、参数、deprecated component、ModelRef 状态、Artifact runtime、StreamRef 和 SecretRef 权限。
- [ ] `R1-APP-004` 编译期解析具体模型 Artifact、后处理、runtime profile 和目标 agent capability；解析结果进入不可变 binding snapshot。
- [ ] `R1-APP-005` 生成完整 canonical `dg/v1 Graph`，排序 map/节点/边、规范化数字与默认值；同一 spec、catalog 和 binding 必须得到相同摘要。
- [ ] `R1-APP-006` `DyunGraphBundle/v1` 包含 GraphSpec、application/catalog/binding digest、Artifact、资源限制、兼容 dg 版本和签名；替换仅记录 runtime profile 字符串的占位实现。

## 2. Agent 协议与 runner

- [ ] `R1-DYUN-001` 在 Proto 增加 DyunAgentService：capability/heartbeat、prepare、start、status、metrics、drain、stop 和 result；命令携带 generation、fencing token、deadline 与幂等键。
- [ ] `R1-DYUN-002` dyun-agent 启动时从固定 commit 探测 schema、elements、codecs、backends 和 build features；不把静态 CPU capability 当真实探测结果。
- [ ] `R1-DYUN-003` 使用可信公钥验证 bundle 签名和全部内容摘要；开发签名与生产签名分离，生产不接受前缀匹配式伪签名。
- [ ] `R1-DYUN-004` 将模型下载到 digest 命名、只读、本租户 runner 可见的受控目录；SecretRef 在启动时解析且不写入 bundle、命令行或日志。
- [ ] `R1-DYUN-005` 每个 replica 使用独立 runner 进程和故障域，直接调用 dyun-gu Rust API 完成 validate/start/status/metrics/shutdown，禁止生产 shell 拼接 `dg-cli`。
- [ ] `R1-DYUN-006` 持久化 desired/observed generation、runner identity、heartbeat 和错误；agent/control-plane 重启后按 fencing 对账，旧状态不能覆盖新部署。
- [ ] `R1-DYUN-007` drain 等待有界时间后停止；异常 runner、失联 agent、部分下载和重复 start 都收敛到明确状态并可重试。

## 3. 真实媒体链路

- [ ] `R1-DYUN-008` 用许可明确的合成视频生成 RTSP 输入，避免依赖外部不稳定或无授权流。
- [ ] `R1-DYUN-009` 使用 R1 检测 ONNX 完成 decode → preprocess → inference → postprocess → tracker → OSD → encode → RTMP。
- [ ] `R1-DYUN-010` 验证输入断流重连、输出端拒绝、模型下载失败、runner crash、停止和相同版本重新发布。
- [ ] `R1-DYUN-011` 保存 GraphSpec、bundle digest、运行日志、metrics、截图/短视频输出和 dg build capability 作为证据。

## 4. 完成条件与测试

- 所有 upstream `UP-DYUN-001` 至 `UP-DYUN-007` 契约测试有结论；未通过项保持 blocker，不能降级为静默 fallback。
- 非法 DAG、越权 SecretRef、未发布模型、不兼容后端、无效签名和摘要不符在 runner 启动前拒绝。
- 控制面和 dyun-agent 各重启一次，运行中 deployment 的 observed state 最终与真实 runner 一致。
- Agent HTTP 只保留 `/healthz`、`/readyz`、`/metrics`；状态变更全部通过认证的内部 gRPC。
