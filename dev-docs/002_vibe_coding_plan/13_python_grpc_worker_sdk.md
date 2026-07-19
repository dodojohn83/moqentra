# 13. Python gRPC Worker 与训练 SDK

## 1. 进程边界

训练代码只运行在独立 Python worker/容器中，通过 gRPC 和对象存储与 Rust 控制面交互；禁止 PyO3 将 PyTorch 或厂商运行时嵌入 Rust 主进程。

Worker 启动后上报版本、框架、accelerator、device、driver/runtime、collective backend、任务类型和限制。控制面按 credit/lease 派发；worker 定期 heartbeat 并使用 fencing token 回报。

## 2. SDK

SDK 提供 `prepare/run/report_metric/save_checkpoint/finalize/cancel` 生命周期，框架 adapter 首批支持纯 PyTorch、MMEngine/MMDetection/MMSegmentation。业务训练包通过稳定 plugin interface 加载，不获得控制面凭据。

## 3. 任务

- [ ] `WORKER-001` 生成 Python Proto client、错误 mapper 和连接状态机。
- [ ] `WORKER-002` 实现 outbound mTLS、重连、credit、lease renew、drain 和 server fencing。
- [ ] `WORKER-003` 实现 workspace sandbox、只读输入、输出目录、信号和进程组清理。
- [ ] `WORKER-004` 实现结构化日志、metric batch、checkpoint 和 artifact manifest。
- [ ] `WORKER-005` 实现 PyTorch/MMEngine adapters 与最小分类、检测、分割样例。
- [ ] `WORKER-006` 限制日志、指标、文件数、产物大小和上传并发。
- [ ] `WORKER-007` 测试断连、重复命令、控制面重启、磁盘满、OOM、SIGTERM 和恶意参数。

完成条件：worker 无控制面数据库访问；凭据短期化；断连恢复不会出现两个有效 attempt。
