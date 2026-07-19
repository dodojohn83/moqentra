# 上游兼容与修改管理

本目录只保存 Moqentra 与独立上游仓库之间的兼容基线、已验证缺口和修改任务，不保存第三方源码。

## 规则

- 每个上游固定 repository、commit/tag、license、核对日期和本地契约测试。
- 先以公开 API 和本项目 adapter 解决；只有源码核对和失败测试证明无法满足时，才提出上游修改。
- 每项缺口包含 `UP-<PROJECT>-NNN`、场景、当前行为、期望契约、最小改动、兼容影响、测试和 fallback。
- 上游 PR 合并前，本项目通过 pinned fork/patch 验证；合并后升级 commit 并删除临时 patch。
- 不复制未兼容许可证的代码，不以 README 声明替代源码与许可证核对。

## 当前登记

- [dyun-gu-dev](dyun-gu-dev.md)：推理图运行时，首版无需强制上游修改。
- LabelU-Kit 是外部 Apache-2.0 依赖；适配和必要的本地 vendor 规则位于主计划第 10 章，不作为自有上游修改。
- cheetah-signaling 当前不集成。dyun-gu 已提供流媒体能力；只有明确的设备信令用例和 fit-gap 证明后再新增文件。
