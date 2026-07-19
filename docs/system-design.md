# Moqentra AI 开放平台调研报告与 MVP 系统设计

## 一、调研结论

调研数据截至 2026-07-19。GitHub 上有足够多的组件级参考项目，但没有一个项目完整覆盖：

- 数据集、标注、分布式训练、模型管理和行业应用编排全流程。
- 单机一体化与 Kubernetes 集群双部署形态。
- NVIDIA、AMD、华为昇腾等异构训练。
- 视频流处理、边缘推理以及 `dyun-gu` 执行图生成。

[CubeStudio](https://github.com/data-infra/cube-studio) 最接近“一站式 AI 平台”，Kubeflow 生态在训练、流水线和推理方面最成熟；但二者都不能直接满足 `dyun-gu` 视频编排和 Rust 控制面的要求。因此建议采用：

> Rust 自研统一控制面 + LabelU-Kit 标注组件 + Python gRPC 训练 Worker + Kubernetes/Volcano/HAMi 调度 + dyun-gu 推理执行面。

海康训练平台和编排平台页面依赖 JavaScript，无法可靠提取内部实现；微信文章也无法正常抓取。因此这里只采用已有产品边界，不推测其未公开架构。

## 二、相似项目列表

Stars 和“最近更新”取自 GitHub API；“最近更新”指仓库最近代码推送日期，Stars 会持续变化。

| 项目 | Stars / 最近更新 | 技术栈及许可 | 解决的问题与核心功能 |
|---|---:|---|---|
| [CubeStudio](https://github.com/data-infra/cube-studio) | 2,360 / 2026-07-15 | Python 56.9%、TypeScript 16.4%、Kubernetes；MIT | 最接近目标的一站式平台。包含多租户、数据集、标注、Notebook、拖拽 Pipeline、分布式训练、模型管理、推理服务、异构算力和云边协同。源码按 `myapp/models`、`views`、`job-template`、`install` 组织。 |
| [Kubeflow](https://github.com/kubeflow/kubeflow) / [Trainer](https://github.com/kubeflow/trainer) / [Pipelines](https://github.com/kubeflow/pipelines) | 15,782 / 2,152 / 4,168；2026-07-10～19 | Go、Python、Kubernetes CRD、Helm；Apache-2.0 | Kubeflow 是可组合 AI 平台生态而非单体应用。Trainer 提供 TrainJob/Runtime 分布式训练；Pipelines 提供可复用 DAG、实验和组件编排。其 `api/controller/runtime/statusserver` 分层值得借鉴。 |
| [Determined](https://github.com/Determined-AI/determined) | 3,224 / 2025-03-20 | Go 44.6%、Python 27.9%、TypeScript 24.4%；Apache-2.0 | 分布式训练、超参搜索、实验跟踪、资源管理和模型注册。采用 Master、Agent、Python SDK、Web UI 分离；但项目已一年多没有代码推送，不宜作为新平台底座。 |
| [ClearML](https://github.com/clearml/clearml) | 6,782 / 2026-07-17 | Python SDK/Agent；Apache-2.0 | 解决训练代码低侵入接入、实验可复现、数据版本、远程任务、流水线和服务化问题。值得借鉴“控制服务 + Agent + SDK”模式，但完整 Server/UI 分散在其他仓库，直接集成会形成第二套平台。 |
| [dstack](https://github.com/dstackai/dstack) | 2,185 / 2026-07-17 | Python 75.6%、TypeScript、Go；MPL-2.0 | 面向云、Kubernetes、裸机统一编排开发环境、分布式任务和推理服务，支持 NVIDIA、AMD、TPU、Tenstorrent。硬件/后端 Profile 很有参考价值，但没有华为昇腾，MPL-2.0 也要求修改文件继续开放源码。 |
| [MLflow](https://github.com/mlflow/mlflow) | 27,100 / 2026-07-18 | Python 59.6%、TypeScript 31.4%；Apache-2.0 | 解决实验指标、模型血缘、评估、注册和部署生命周期。适合作为领域模型参考或可选后端，不适合直接承担平台多租户、训练调度和数据标注。 |
| [LabelU](https://github.com/opendatalab/labelU) / [LabelU-Kit](https://github.com/opendatalab/labelU-Kit) | 1,628 / 158；2026-07-15～16 | FastAPI/Python、TypeScript 组件库；Apache-2.0 | 支持图像、视频、音频、多种标注工具、S3 导入和 AI 预标注。Kit 采用 `apps + packages` 前端 Monorepo，适合嵌入统一门户，由 Rust 后端管理租户、任务和数据。 |
| [Label Studio](https://github.com/HumanSignal/label-studio) | 27,873 / 2026-07-17 | Django/Python、React/TypeScript；Apache-2.0 | 通用多模态标注、模板配置、数据导入导出、云存储和机器学习后端。功能成熟，但独立组织、用户、项目和任务模型会与主平台重复。 |
| [CVAT](https://github.com/cvat-ai/cvat) | 16,330 / 2026-07-19 | Django/Python、TypeScript、React、RQ；MIT | 专注视觉数据，图像、视频、3D、跟踪、质检和团队协作能力成熟。适合作为未来可选外部标注服务；`serverless` 模型资产和 FFmpeg 构建仍需逐项核查许可证。 |
| [LabelBee Client](https://github.com/open-mmlab/labelbee-client) | 395 / 2024-04-19 | JavaScript 55.5%、TypeScript 40.7%、Electron；Apache-2.0 | 提供检测、分类、分割、文本、轮廓、关键点等桌面标注工具。但维护活跃度低、导入格式有限，不建议作为新平台首选。 |
| [Volcano](https://github.com/volcano-sh/volcano) | 5,785 / 2026-07-17 | Go、Kubernetes CRD；Apache-2.0 | 面向 AI/HPC 的批任务调度，提供队列、公平调度、Gang Scheduling、拓扑感知、抢占和多种分布式框架集成；新版本包含 Ascend vNPU 支持。 |
| [HAMi](https://github.com/Project-HAMi/HAMi) | 3,960 / 2026-07-17 | Go、Device Plugin、Webhook；Apache-2.0 | 提供 NVIDIA、昇腾、海光、寒武纪、摩尔线程等异构设备发现、共享、隔离和调度。目录明确分为 `device-plugin`、`device`、`scheduler`、`monitor/metrics`。 |
| [KServe](https://github.com/kserve/kserve) | 5,710 / 2026-07-18 | Go 64.1%、Python 30.6%、Kubernetes CRD；Apache-2.0 | 标准化中心云模型服务，包含多框架 Serving Runtime、自动扩缩容、流量管理和推理图。适合未来独立推理平台，不适合替代 `dyun-gu` 的视频流图执行。 |
| [dyun-gu-dev](https://github.com/ChungTak/dyun-gu-dev) | 0 / 2026-07-19 | Rust 98%、C；Cargo 声明 Apache-2.0 | 已具备 OpenVINO、TensorRT、RKNN2、Sophon 后端，以及 Graph、Scheduler、Media、Stream、Elements、C API。`dg/v1 Graph` 已能表达流源、编解码、预处理、推理、后处理、跟踪、OSD 和输出，是本项目应用执行面的直接上游。 |

补充判断：

- [OpenPAI](https://github.com/microsoft/pai) 曾覆盖 AI 集群、作业和资源管理，但已于 2024-06 归档，只适合研究历史设计。
- BentoML、TensorFlow Serving 等主要解决模型服务，不覆盖数据、标注和训练控制面，因此没有列为主要底座。
- `dyun-gu` 的 Cargo 工作区声明 Apache-2.0，但仓库根目录没有标准 `LICENSE` 文件。正式分发前应补齐 LICENSE/NOTICE 和第三方依赖清单。

## 三、目录结构和关键模块的共性

这些项目虽然语言不同，但核心结构高度一致：

| 共同层次 | 典型目录 | 作用 |
|---|---|---|
| 契约层 | `api`、`proto`、`schemas`、CRD | 定义作业、运行时、模型、状态和外部接口，避免 UI 或 Worker 直接依赖数据库结构。 |
| 控制面 | `server`、`master`、`controller`、`pkg` | 鉴权、元数据、任务状态机、资源策略、调度适配和声明式协调。 |
| 执行面 | `agent`、`runner`、`worker`、`runtime`、`device-plugin` | 在计算节点执行训练或推理，采集日志、指标、心跳和硬件能力。 |
| 前端 | `frontend`、`webui`、`apps/packages` | 管理门户、可视化编排、标注组件和监控界面。 |
| 部署层 | `charts`、`helm`、`install`、`manifests`、`docker` | 将同一产品部署到本地、Kubernetes 或云环境。 |
| 扩展层 | `plugins`、`runtime`、`job-template`、`examples` | 把不同框架、硬件、数据格式和推理引擎放在核心系统之外。 |
| 质量层 | `tests`、`e2e_tests`、`benchmark`、`security` | 覆盖控制器、API、部署、性能和安全边界。 |

关键设计共性：

- 控制面不直接运行用户训练代码，训练通过独立 Agent/Worker 或 Kubernetes 作业执行。
- 元数据与大文件分离：关系数据库保存索引和状态，对象存储保存数据集、日志、检查点和模型。
- 训练、流水线、推理都采用版本化声明式规格，而不是把运行命令散落在业务代码中。
- 硬件差异封装在 Runtime、Device Plugin、Worker Image 和 Capability 中。
- 所有长任务都有明确状态机、心跳、取消、重试、日志和产物回传。
- 前端、API、调度器和执行节点通过稳定契约协作，可以独立部署和升级。

## 四、值得借鉴与不适合直接采用的设计

### 值得借鉴

- 借鉴 CubeStudio 的功能域划分、多租户项目、资源组、训练模板和数据到模型的完整链路。
- 借鉴 Kubeflow 的声明式 Job/Runtime 和控制器协调模式，但隐藏在自有执行适配器后面。
- 借鉴 Determined/ClearML 的 Agent 模式：训练环境、用户代码和主控制进程相互隔离。
- 借鉴 MLflow 的实验、Run、参数、指标、Artifact、Registered Model 和模型版本血缘。
- 采用 LabelU-Kit 作为可嵌入前端组件，统一平台登录、租户、数据授权和审计。
- 采用 Volcano 的队列、Gang Scheduling、优先级、拓扑感知和分布式任务调度。
- 采用 HAMi 和各厂商 Device Plugin 描述异构设备，平台自身不实现 GPU/NPU 虚拟化。
- `dyun-gu` 保持为推理与流媒体执行面；平台只负责设计、校验、编译、发布和状态管理。
- 为未来中心推理平台预留 KServe 风格的 Deployment/Runtime 接口，不让训练平台直接绑定某个 Serving 实现。

### 不适合

- 不直接 Fork CubeStudio：功能过宽、Python 单体业务层较重，难以形成清晰的 Rust 领域边界。
- 不部署完整 Kubeflow 作为 MVP：依赖、运维和升级复杂度过高，单机部署也不合适。
- 不自研 Kubernetes 调度器、GPU 共享或厂商驱动层。
- 不设计一个“所有硬件通用”的训练镜像。CUDA、ROCm、CANN/MindSpore 应使用独立认证镜像和能力矩阵。
- 不让 Python 通过 PyO3 嵌入 Rust 主进程；训练始终运行在独立 Worker/容器中。
- 不直接把 Label Studio/LabelU 后端暴露成第二套用户与权限系统。
- 不把 `dg/v1` YAML 直接作为平台业务模型。平台应维护更高层、平台无关的 ApplicationSpec，再确定性编译成 dyun-gu 配置。
- 不在 MVP 采用微前端。当前只有一个团队和统一产品边界，微前端会增加依赖共享、路由、CSP、供应链和跨租户安全复杂度。
- 不在 MVP 集成 `cheetah-signaling`。只有出现 GB/T 28181、ONVIF 设备管理、级联或会话协商需求时再通过适配器接入。
- 不直接复用来源不明的模型权重、CVAT serverless 模型或 GPL 编解码构建；代码许可、模型许可、数据许可和 Codec 许可分别审计。

## 五、MVP 范围

MVP 已按以下选择锁定：

- 行业视觉优先。
- 单机与集群双形态同版。
- 嵌入 LabelU-Kit，不部署独立标注后台。

### 1. 纳入 MVP

#### 多租户基础

- 租户、项目、用户、角色和项目成员。
- RBAC、资源配额、审计日志、OIDC/企业身份源。
- PostgreSQL 行级隔离作为应用鉴权之外的第二道防线。
- 对象存储按租户和项目隔离，浏览器只获取短期签名 URL。

#### 数据集管理

- 图片和视频上传、S3/MinIO 导入、媒体元数据提取。
- 不可变 Dataset Version，使用 Manifest、文件校验和、标签 Schema 和来源信息描述。
- 训练只能引用冻结的数据集版本，避免训练过程中数据漂移。
- 首版支持 COCO、LabelU 原生格式和平台统一中间格式。

#### 数据标注

- 在统一 React 门户嵌入 LabelU-Kit。
- 支持图像分类、矩形检测、语义/实例分割，以及视频目标/轨迹标注。
- 标注项目、任务分配、草稿、提交、审核、退回和导出。
- 自动标注通过独立推理 Worker 或 dyun-gu 作业完成，不在 API 进程加载模型。

#### 训练与实验

- 模板化视觉训练：分类、目标检测、分割。
- Python gRPC Worker 封装 PyTorch/OpenMMLab；昇腾使用独立 `torch-npu` 或 MindSpore Worker。
- 同一 TrainingJobSpec 可编译为单机 OCI 任务或 Kubernetes/VolcanoJob。
- 支持单机、多卡和多节点 DDP，日志、指标、检查点、取消和失败重试。
- NVIDIA CUDA、AMD ROCm、华为 Ascend 分别维护 Worker 镜像与验证矩阵，不能仅以“能调度到设备”宣称训练兼容。

#### 模型管理

- 模型、模型版本、来源 Run、数据集版本、训练模板和指标血缘。
- 原始权重与 ONNX 作为基准产物；TensorRT、OpenVINO IR、RKNN、Sophon、Ascend OM 等作为派生 Artifact。
- 模型状态包括草稿、已验证、已发布、已停用。
- 每个 Artifact 保存格式、目标硬件、运行时版本、校验和和兼容约束。

#### 算法应用编排

- React Flow 构建视觉 DAG，节点来自版本化组件目录。
- 平台 ApplicationSpec 表达模型引用、流引用、逻辑节点、端口类型、参数和部署约束。
- 编译器解析模型版本和硬件能力，生成不可变、带校验和的 `dg/v1 Graph` 发布包。
- dyun-gu Adapter 提供能力查询、图校验、部署、停止、状态订阅和日志查询。
- 编排平台不直接保存 RTSP 密码、对象存储密钥等明文；配置中只使用 SecretRef。

#### 两种部署形态

- 单机版：一条安装命令部署 Rust 控制面、静态前端、Node Agent、Python Worker、PostgreSQL 和 MinIO；训练仍在隔离容器内执行。
- 集群版：Kubernetes 部署控制面，Volcano 管理批任务，HAMi/厂商 Device Plugin 管理异构设备。
- 单机和集群共用领域模型、数据库迁移、API 和 JobSpec，仅执行适配器不同。

### 2. 不纳入 MVP

- LLM 微调、RAG、Agent 和模型市场。
- Notebook、在线 IDE、自动超参搜索、计量计费。
- 多集群联邦调度和跨地域数据复制。
- 完整 KServe 中心推理平台。
- GB/T 28181、ONVIF、级联和 `cheetah-signaling`。
- 原生桌面客户端、AI 一站通客户端和 LogicAgent 客户端。
- 微前端及第三方前端插件运行时。
- 3D 点云、音频和通用文本标注。
- 复杂主动学习和全自动数据闭环。

## 六、技术实现建议

### Rust 控制面

- Rust Workspace，采用模块化单体起步，控制面与 Node Agent 分别构建和部署。
- `axum + tokio` 提供 REST/OpenAPI，`tonic` 提供内部 gRPC。
- `sqlx + PostgreSQL` 管理显式 SQL、迁移、事务和行级安全策略。
- `kube-rs` 实现 Kubernetes、VolcanoJob 和设备资源适配。
- S3 API 统一访问 MinIO、Ceph RGW 或公有云对象存储。
- OpenTelemetry 输出 Trace、Metric 和结构化日志。
- 长任务使用数据库状态机与 Outbox；MVP 不引入 Kafka。Worker 通过带心跳的 gRPC 租约获取任务，所有状态更新要求幂等。

### Python Worker

- 独立 Python 进程或容器，以 `grpcio` 实现 Worker 协议。
- Worker 注册硬件、框架、驱动、模型格式和分布式能力。
- CUDA、ROCm、CANN/MindSpore 分别构建镜像，不动态混装厂商 SDK。
- 用户代码、训练数据和输出目录采用明确挂载；控制面不会执行任意 Shell。
- 训练产物先写临时位置，成功校验后再原子注册为模型 Artifact。

### 前端与安全

- React、TypeScript、Vite、React Flow、LabelU-Kit，采用普通 Monorepo。
- MVP 不使用微前端；后续若生态伙伴需要扩展，提供受控组件协议或沙箱 iframe，而不是执行不可信 Module Federation 代码。
- OIDC Authorization Code + PKCE；Web 会话使用 Secure、HttpOnly、SameSite Cookie，不在 LocalStorage 保存长期 Token。
- 强制 CSP、CSRF 防护、上传类型/大小验证、文件名归一化和 SVG/HTML 隔离。
- 前端永远不直接持有数据库、对象存储或 Worker 凭据。
- 所有 API 在资源查询层强制 tenant/project 条件，同时由 PostgreSQL RLS 兜底。

### 调度和异构硬件

- 单机执行器负责 OCI 容器、设备探测、资源限制和日志采集。
- 集群执行器默认单副本任务使用 Kubernetes Job，分布式训练使用 VolcanoJob。
- HAMi 负责可共享设备；需要整卡或厂商原生隔离时使用对应 Device Plugin。
- 硬件能力以标准化 ResourceClass 表达，例如 vendor、device_family、memory、driver、runtime、network_fabric 和 sharing_mode。
- 平台调度条件只引用 ResourceClass，不在业务层硬编码 `nvidia.com/gpu`、Ascend 或 ROCm 细节。
- Kubeflow Trainer 后续作为可插拔执行适配器接入，避免 MVP 绑定其仍在演进的 API。

### 与 dyun-gu 的分层

```text
Moqentra 控制面
  ├─ 数据集、标注、训练、模型、租户与策略
  ├─ ApplicationSpec 与可视化设计器
  └─ Deployment API
             │
             ▼
Inference Adapter / 未来独立推理平台
  ├─ 能力协商
  ├─ 模型变体选择
  ├─ ApplicationSpec 编译
  └─ 发布、回滚、状态与告警
             │
             ▼
dyun-gu Runtime
  ├─ Graph / Scheduler / Elements
  ├─ OpenVINO / TensorRT / RKNN / Sophon
  └─ Decode / Stream / Tracking / OSD / Output
```

`cheetah-signaling` 以后只作为设备与信令适配器接入，不进入训练、模型或图执行核心。

## 七、重要接口和类型

需要先稳定以下版本化契约：

- `DatasetManifest/v1`：数据集版本、对象列表、校验和、媒体信息、标签 Schema 和来源。
- `AnnotationProjectSpec/v1`：标注类型、任务切分、人员角色、审核流程和导出格式。
- `TrainingJobSpec/v1`：数据版本、训练模板、参数、镜像、资源、分布式策略、输出和重试策略。
- `WorkerCapabilities/v1`：硬件厂商、设备、驱动、框架、模型格式和最大并行能力。
- `ModelArtifactManifest/v1`：模型血缘、格式、目标运行时、目标硬件、校验和与依赖。
- `ApplicationSpec/v1`：平台无关应用 DAG、类型化端口、ModelRef、StreamRef、SecretRef 和部署策略。
- `DeploymentSpec/Status/v1`：目标运行时、发布版本、期望副本、状态、错误和回滚点。
- `DyunGraphBundle/v1`：编译后的 `dg/v1 Graph`、解析后的 Artifact 引用、校验和和签名。

对外接口使用 REST/OpenAPI；控制面、Worker、Node Agent 和 dyun Adapter 使用 Protobuf/gRPC。所有创建和重试接口支持 Idempotency Key。

## 八、实施顺序

### 1. 契约与基础控制面

- 冻结上述 Manifest、JobSpec、ApplicationSpec 和状态机。
- 完成租户、项目、RBAC、审计、PostgreSQL RLS、对象存储和 SecretRef。

### 2. 数据与标注闭环

- 实现数据集版本、上传/导入和 LabelU-Kit 嵌入。
- 打通任务分配、审核、COCO/LabelU 导入导出和冻结版本。

### 3. 双形态训练

- 完成 Node Agent、Python Worker、单机 OCI 执行和 Kubernetes/Volcano 执行。
- 打通 NVIDIA、AMD、Ascend 的分类/检测最小训练样例和分布式验证。

### 4. 实验与模型管理

- 建立 Run、指标、日志、检查点、模型版本和派生 Artifact 血缘。
- 完成 ONNX 与至少一个 dyun-gu 目标后端的模型转换验证。

### 5. 应用编排与 dyun-gu

- 实现类型化 DAG、静态校验、能力协商和确定性编译。
- 打通 RTSP 输入、检测、跟踪、OSD、编码和 RTMP 输出的发布闭环。

### 6. 企业化加固

- 完成跨租户安全测试、故障恢复、镜像签名、SBOM、依赖许可、性能和升级测试。

## 九、测试与验收场景

- 租户 A 无法通过 API、签名 URL、日志、任务 ID 或模型 ID 访问租户 B 数据。
- 数据集冻结后不能原地修改；相同文件生成稳定校验和，训练 Run 能追溯到准确版本。
- LabelU-Kit 标注可完整导出并重新导入，审核退回不会覆盖历史版本。
- 同一个 TrainingJobSpec 能在单机和 Kubernetes 执行，输出结构和状态语义一致。
- Worker 断连、重复上报、控制面重启、任务取消和重试不会生成重复模型版本。
- NVIDIA、AMD、Ascend 各完成一个真实训练任务；分布式任务验证 Gang Scheduling、Rank 环境和检查点恢复。
- 模型 Artifact 能追溯训练模板、代码版本、数据版本、框架和硬件环境。
- 非法 DAG、端口类型不匹配、缺少能力、循环依赖和无权限 SecretRef 必须在发布前拒绝。
- 相同 ApplicationSpec 和依赖版本应编译出相同 `DyunGraphBundle` 校验和。
- dyun-gu 完成 RTSP → 解码 → 检测 → 跟踪 → OSD → 编码 → RTMP 的端到端运行、停止和重新发布。
- 验证 CSP、CSRF、XSS、越权、恶意文件上传、日志泄密和训练镜像逃逸边界。
- 全新单机环境可一条命令安装并完成“导入数据—标注—训练—注册—编排—推理”闭环。

## 十、默认假设

- MVP 面向行业视觉用户，优先图像和视频，不追求通用 AI 功能全集。
- Rust 负责控制面、节点代理、数据传输、应用编译和性能敏感推理集成；Python 负责训练生态。
- 单机“一体化”指一个安装包/一条命令和统一 UI，不表示把 Python 训练引擎链接进 Rust 进程。
- LabelU-Kit、训练框架、模型权重、数据集和编解码器分别进行许可证审计。
- `dyun-gu` 是首选视频推理执行面；未来独立推理平台通过 Adapter 层替换或并存，不改变上层资产和 ApplicationSpec。
