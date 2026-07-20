# Capability Tracking Matrix

This matrix maps each Moqentra capability to a release phase, the plan chapters
that implement it, and the acceptance scenario that validates it.

| Capability ID | Capability | Release | Plan Chapters | Acceptance Scenario |
|---|---|---|---|---|
| CAP-001 | Workspace baseline & version matrix | R0 | 01 | TAS-001 |
| CAP-002 | Product scope & release phases | R0 | 02 | TAS-001 |
| CAP-003 | Monorepo bootstrap & CI | R1 | 03 | TAS-002 |
| CAP-004 | Architecture crate & service graph | R1 | 04 | TAS-002 |
| CAP-005 | Foundation types, errors, config | R1 | 05 | TAS-003 |
| CAP-006 | API/proto specs & codegen | R1 | 06 | TAS-004 |
| CAP-007 | Multitenancy, identity, RBAC, audit | R1 | 07 | TAS-005 |
| CAP-008 | PostgreSQL storage, outbox, idempotency | R1 | 08 | TAS-006 |
| CAP-009 | Object storage, dataset ingestion, versioning | R1 | 09 | TAS-007 |
| CAP-010 | Annotation platform & LabelU-Kit | R1 | 10 | TAS-008 |
| CAP-011 | Data quality, auto annotation, multimodal | R1 | 11 | TAS-009 |
| CAP-012 | Experiment & training domain | R1 | 12 | TAS-010 |
| CAP-013 | Python gRPC worker SDK | R1 | 13 | TAS-010 |
| CAP-014 | Local executor & node agent | R1 | 14 | TAS-011 |
| CAP-015 | Kubernetes, Volcano, heterogeneous scheduler | R2 | 15 | TAS-012 |
| CAP-016 | Distributed training, checkpoint & recovery | R2 | 16 | TAS-013 |
| CAP-017 | Model registry, artifacts & lineage | R1/R2 | 17 | TAS-014 |
| CAP-018 | Model conversion, evaluation & promotion | R2 | 18 | TAS-015 |
| CAP-019 | Pipeline, HPO & notebook services | R4 | 19 | TAS-016 |
| CAP-020 | Application orchestration & compiler | R1 | 20 | TAS-017 |
| CAP-021 | dyun agent & graph runtime | R1/R2 | 21 | TAS-018 |
| CAP-022 | Inference platform layering & federation | R3 | 22 | TAS-032 |
| CAP-023 | Northbound API, events, webhooks, partner SDK | R3/R4 | 23 | TAS-019 |
| CAP-024 | Web frontend architecture & data security | R1 | 24 | TAS-020 |
| CAP-025 | All-in-one local deployment | R1 | 25 | TAS-021 |
| CAP-026 | Cluster HA reconciliation & multicluster | R3 | 26 | TAS-022 |
| CAP-027 | Desktop clients & offline workflows | R4 | 27 | TAS-023 |
| CAP-028 | Security, secrets & supply chain | R2 | 28 | TAS-024 |
| CAP-029 | Observability, audit & operations | R2 | 29 | TAS-025 |
| CAP-030 | Testing, simulators, hardware & performance | R2/R3 | 30 | TAS-026 |
| CAP-031 | Packaging, migration, release & rollback | R2/R3 | 31 | TAS-027 |

## Definition of Release Columns

- **R0** — planning and baseline deliverables; no runtime.
- **R1** — Visual MVP: complete data → annotation → training → model →
  application → dyun-gu deployment on a single machine and on Kubernetes.
- **R2** — Production training: multi-node, checkpoint recovery, conversion matrix,
  enterprise audit, and HA.
- **R3** — Inference platform: multi-cluster/edge, canary, elasticity, and
  model-publish linkage.
- **R4** — Ecosystem: pipeline/HPO, notebook, partner SDK, and desktop client.
