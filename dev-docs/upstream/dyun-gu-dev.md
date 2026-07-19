# dyun-gu-dev 兼容基线

## 1. 固定版本

- 本地仓库：`/dataset/datavol/workspace/media_server/dyun-gu-dev`
- Repository：`https://github.com/ChungTak/dyun-gu-dev`
- 核对 commit：`f6d6cb06e07b8dde332ed585a8250207501898dc`
- Rust：`1.94.1`
- License：Apache-2.0

升级时必须重跑本文件的契约测试，不能直接跟随 branch。

## 2. 已核对能力

- `dg-graph` 公开 `GraphSpec`、Builder、schema、注册 element、Graph、RunningGraph、GraphStatus。
- GraphSpec 使用 `apiVersion: dg/v1`、`kind: Graph`，支持 YAML/JSON/TOML、严格校验、具名节点/端口、资源限制和 includes/templates。
- RunningGraph 公开 start、status、metrics、shutdown 和热更新能力。
- `dg-cli` 提供 validate、run、watch、schema、list-elements 和 ops 健康/指标。
- `dg-capi` 提供生命周期、状态、metrics、build/backend capabilities。
- OpenVINO、TensorRT、RKNN、Sophon 与媒体/流能力通过 feature 隔离；真实 SDK 支持等级必须分别验证。

结论：Moqentra 首版可在本项目实现 `dyun-agent + runner` 并直接链接固定版本的 `dg-*` crates，不需要把远程服务代码加入 dyun-gu。

## 3. 平台适配约束

- 平台 `ApplicationSpec/v1` 不等同 GraphSpec；必须由服务端确定性编译。
- runner 接收签名的 `DyunGraphBundle/v1`，解析后调用 Rust API，禁止生产环境 shell 拼接 `dg-cli`。
- deployment replica 进程隔离；agent 不把多个不可信租户图放入同一故障域。
- 模型和 secret 在部署期解析为本地受控路径/引用；GraphSpec 不携带长期凭据。
- agent 归一化 dg 状态、metrics 和错误，但保留原始安全诊断供管理员查看。

## 4. 升级契约测试

- [ ] `UP-DYUN-001` 当前平台生成的所有 bundle 均通过新版本 `GraphSpec` 严格校验。
- [ ] `UP-DYUN-002` 连续两次编译 canonical GraphSpec digest 相同。
- [ ] `UP-DYUN-003` start/status/metrics/drain/shutdown 生命周期无语义变化。
- [ ] `UP-DYUN-004` 合法热更新成功；非法更新拒绝且旧图继续运行。
- [ ] `UP-DYUN-005` mock 图在无 SDK CI 运行；各产品 feature 在对应 runner 验证。
- [ ] `UP-DYUN-006` 真实 RTSP→推理→OSD→RTMP 链路、重连和 copy report 达标。
- [ ] `UP-DYUN-007` schema/element capability diff 被纳入发布审查。

## 5. 上游修改准入

只有公共 API 无法完成结构化 capability、隔离 runner 生命周期、热更新诊断或有界 metrics 导出，且 adapter 无法安全解决时，才新增 `UP-DYUN-GAP-*`。每项必须附最小复现、期望 Rust API、兼容测试和不修改上游时的受限 fallback。
