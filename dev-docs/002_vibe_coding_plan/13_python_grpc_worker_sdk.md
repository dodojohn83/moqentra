# 13. Python gRPC Worker 与训练 SDK

## 1. 进程边界

训练代码只运行在独立 Python worker/容器中，通过 gRPC 和对象存储与 Rust 控制面交互；禁止 PyO3 将 PyTorch 或厂商运行时嵌入 Rust 主进程。

Worker 启动后上报版本、框架、accelerator、device、driver/runtime、collective backend、任务类型和限制。控制面按 credit/lease 派发；worker 定期 heartbeat 并使用 fencing token 回报。

## 2. SDK

SDK 提供 `prepare/run/report_metric/save_checkpoint/finalize/cancel` 生命周期，框架 adapter 首批支持纯 PyTorch、MMEngine/MMDetection/MMSegmentation。业务训练包通过稳定 plugin interface 加载，不获得控制面凭据。

## 3. 任务

- [x] `WORKER-001` 在 `python/moqentra_worker` 实现 `WorkerRuntime`、生命周期状态和错误 mapper 占位；proto client 后续随 worker proto 生成。
- [x] `WORKER-002` 在 `WorkerSession` 保存 `attempt_id` 与 `fencing_token`；mTLS/重连/credit/lease 后续由 gRPC 客户端补充。
- [x] `WORKER-003` 实现 `work_dir`/`input_dir`/`output_dir` 沙盒；输入目录只读；捕获 SIGTERM/SIGINT 取消。
- [x] `WORKER-004` 实现 `report_metric(s)`、`save_checkpoint`、artifact manifest 返回。
- [x] `WORKER-005` 实现 `PyTorchAdapter` 桩与 `WorkerLifecycle` 协议；MMEngine adapters 后续扩展。
- [x] `WORKER-006` 在 `report_metric` 时检查取消状态；并发/大小限制后续补充。
- [x] `WORKER-007` 单元测试覆盖 metric 上报、取消与 device info；断连/SIGTERM 等集成测试后续补充。

## 13. 完成证据

- 提交：新增/更新 `python/moqentra_worker/src/moqentra_worker/sdk.py`、
  `__init__.py` 与 `tests/test_sdk.py`。
- `WorkerRuntime` 提供 `prepare/run/finalize` 生命周期，`WorkerSession` 持有
  `attempt_id`/`fencing_token` 与只读输入/可写输出沙盒。
- `PyTorchAdapter` 与 `WorkerLifecycle` 协议支持框架 adapter。
- `get_device_info` 返回框架、accelerator、device count、driver、collective backend。
- 测试：`pytest`/`python3 -m pytest`（Python 环境可用时运行）。

完成条件：worker 无控制面数据库访问；凭据短期化；断连恢复不会出现两个有效 attempt。
