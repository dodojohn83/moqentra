# Moqentra Product Scope and Release Phases

Moqentra is an open industry-vision AI platform. Its authoritative value chain is:

```text
Dataset Asset → Annotated Version → Training Run → Model Version
        → Conversion/Evaluation → Application Version → dyun-gu Deployment
```

Every downstream resource stores an immutable reference to its upstream source
and a snapshot taken at creation time.

## Out of Scope

- LLM fine-tuning, RAG, or model marketplace.
- Proprietary training frameworks or model compilers.
- General text/audio/3D annotation in the first releases.
- Micro-frontends and third-party frontend plugin runtimes.

Any future request to add an out-of-scope item MUST go through the Architecture
Decision Record (ADR) process described in `docs/admission-process.md`.

## Release Phases

| Phase | Codename | Goal | Enter Condition | Exit Condition |
|---|---|---|---|---|
| R1 | Visual MVP | Close the visual data loop on a single machine and on Kubernetes. | Task 01–08 complete. | End-to-end flow (import → annotate → train → model → deploy) passes on NVIDIA and single-node Kubernetes with real training evidence. |
| R2 | Production Training | Add multi-machine training, checkpoint recovery, quotas, approvals, conversion matrix, enterprise audit, and HA. | R1 shipped. | Multi-node DDP, Volcano/HAMi scheduling, conversion for TensorRT/OpenVINO/Ascend, and cross-tenant security tests pass. |
| R3 | Inference Platform | Standalone inference control plane, multi-cluster/edge nodes, canary, elasticity, and model-publish linkage. | R2 shipped. | RTSP → decode → detect → track → OSD → encode → RTMP pipeline runs and can be re-published without downtime. |
| R4 | Ecosystem | Pipeline, HPO, Notebook, partner SDK, desktop client, and audio/text annotation. | R3 shipped. | Partner SDK and desktop client reach preview. |

## Capabilities

| Capability | R1 | R2 | R3 | R4 |
|---|---|---|---|---|
| Dataset upload / S3 import | MVP | GA | GA | GA |
| Dataset versioning & manifest | MVP | GA | GA | GA |
| LabelU-Kit image/video annotation | MVP | GA | GA | GA |
| COCO / LabelU import & export | MVP | GA | GA | GA |
| Automatic pre-labeling | preview | GA | GA | GA |
| Classification training | MVP | GA | GA | GA |
| Detection / segmentation training | MVP | GA | GA | GA |
| Multi-node distributed training | - | GA | GA | GA |
| NVIDIA training | supported | supported | supported | supported |
| AMD training | compile-only | preview | supported | supported |
| Ascend training | compile-only | preview | preview | supported |
| Model registry & versioning | MVP | GA | GA | GA |
| ONNX / TensorRT / OpenVINO conversion | - | preview | GA | GA |
| Ascend OM / RKNN / Sophon conversion | - | compile-only | preview | GA |
| dyun-gu graph deployment | MVP | GA | GA | GA |
| Multi-cluster inference | - | - | preview | GA |
| HPO / Notebook | - | - | - | preview |
| Partner SDK | - | - | - | preview |
| Desktop client (Tauri) | - | - | preview | GA |

## Personas

| Persona | Concerns | Primary Workflows |
|---|---|---|
| Tenant Administrator | Tenant creation, user/role management, quotas, billing visibility, audit. | RBAC, quota, cost dashboard. |
| Data Engineer | Dataset ingestion, versioning, storage layout, schema, quality. | Upload, import, manifest, format conversion. |
| Annotator | Task assignment, drawing tools, drafts, submit/rework. | LabelU-Kit projects, tasks, shortcuts. |
| Reviewer | Quality control, approve/reject, inter-annotator agreement. | Review queue, metrics, export. |
| Algorithm Engineer | Training templates, experiments, hyperparameters, model versions. | Training run, metrics, checkpoints, model registry. |
| DevOps / Platform Operator | Deployment, upgrades, observability, security, HA. | Helm/compose install, monitors, backups. |
| Ecosystem Developer | Partner SDK, plugin runtime, API integration. | SDK, webhooks, API keys. |

## Project, Resource Quota and Approval Boundaries

- A **Tenant** is the top-level isolation boundary. All resources carry a `TenantId`.
- A **Project** groups datasets, annotation tasks, training runs, models, and applications.
- **Resource Quota** limits per-project GPU/NPU hours, storage, and number of concurrent training/inference jobs.
- **Approvals** are required for:
  - Publishing a model to the model registry.
  - Deploying an application to production namespace.
  - Exceeding a project quota.
- **Audit** records every create/update/delete and every access to cross-tenant resources.

## Traceability Rule

Every UI page, API endpoint, database table, and background job MUST be traceable
to one release phase and one acceptance scenario in `docs/acceptance-scenarios.md`.
