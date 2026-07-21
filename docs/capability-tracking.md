# Capability Tracking Matrix

This matrix maps each Moqentra capability to a release phase, the plan chapters
that implement it, and the acceptance scenario that validates it.

The R1 status column uses the four-level model defined in
`dev-docs/003_r1_vertical_slice_plan/01_current_state_and_execution_contract.md`:
`designed` → `implemented` → `integrated` → `accepted`. Only capabilities with
real integration evidence or release gate evidence may be marked `integrated` or
`accepted`.

| Capability ID | Capability | Release | Plan Chapters (002) | R1 Slice (003) | Status | Evidence |
|---|---|---|---|---|---|---|
| CAP-001 | Workspace baseline & version matrix | R0 | 01 | 01 | accepted | `baseline/`, `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `clippy.toml` |
| CAP-002 | Product scope & release phases | R0 | 02 | 01 | accepted | `docs/product-scope.md`, `docs/architecture.md`, `baseline/release-policy.md` |
| CAP-003 | Monorepo bootstrap & CI | R1 | 03 | 01/11 | implemented | `.github/workflows/ci.yml`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` |
| CAP-004 | Architecture crate & service graph | R1 | 04 | 01 | integrated | `tools/check_crate_graph.py`, `crates/*/Cargo.toml`, acyclic dependency graph enforced by CI |
| CAP-005 | Foundation types, errors, config | R1 | 05 | 02 | implemented | `crates/types/src/` (25 unit tests passing), `crates/contracts/src/` |
| CAP-006 | API/proto specs & codegen | R1 | 06 | 02 | implemented | `crates/http-api/src/northbound.rs`, `proto/moqentra/`, `docs/openapi/` |
| CAP-007 | Multitenancy, identity, RBAC, audit | R1 | 07 | 02 | implemented | `crates/auth/src/` (15 unit tests passing); OIDC/RBAC in memory, DB integration in `R1-API-002` |
| CAP-008 | PostgreSQL storage, outbox, idempotency | R1 | 08 | 02 | implemented | `crates/storage/src/outbox.rs`, `crates/storage/src/idempotency.rs`, `crates/storage/src/pool.rs` (in-memory); PostgreSQL repos in `R1-DB-*` |
| CAP-009 | Object storage, dataset ingestion, versioning | R1 | 09 | 03 | implemented | `crates/object-store/src/memory.rs`, `crates/object-store/src/s3.rs` (4 tests), `crates/domain/src/dataset.rs` |
| CAP-010 | Annotation platform & LabelU-Kit | R1 | 10 | 03 | implemented | `crates/application/src/annotation_svc.rs`, `apps/web/src/annotation/LabelUAdapter.ts`; real LabelU integration pending `R1-LABEL-001` |
| CAP-011 | Data quality, auto annotation, multimodal | R1 | 11 | 03 | designed | `crates/domain/src/quality.rs`; full quality gate tasks in `R1-DATA-005`, `R1-LABEL-*` |
| CAP-012 | Experiment & training domain | R1 | 12 | 04 | implemented | `crates/domain/src/training.rs`, `crates/application/src/training_svc.rs` (tests passing) |
| CAP-013 | Python gRPC worker SDK | R1 | 13 | 04 | designed | `python/moqentra_worker/src/moqentra_worker/sdk.py`; real gRPC stubs and lifecycle in `R1-TRAIN-006` |
| CAP-014 | Local executor & node agent | R1 | 14 | 04 | implemented | `crates/worker-control/src/local_executor.rs` (4 tests), `apps/node-agent/src/main.rs` |
| CAP-015 | Kubernetes, Volcano, heterogeneous scheduler | R2 | 15 | 05 | designed | `crates/scheduler/src/scheduler.rs` (unit tests), real k3s/Volcano integration in `R1-K8S-*` |
| CAP-016 | Distributed training, checkpoint & recovery | R2 | 16 | 05 | designed | `crates/scheduler/src/distributed.rs` (unit tests), multi-node rendezvous is R2 |
| CAP-017 | Model registry, artifacts & lineage | R1/R2 | 17 | 06 | implemented | `crates/domain/src/model_registry.rs`, `crates/application/src/model_svc.rs` (tests passing); persistent registry in `R1-MODEL-001` |
| CAP-018 | Model conversion, evaluation & promotion | R2 | 18 | 06 | designed | `crates/domain/src/conversion.rs`; ONNX/runtime conversion in `R1-CONVERT-*` |
| CAP-019 | Pipeline, HPO & notebook services | R4 | 19 | — | designed | `crates/domain/src/pipeline.rs`; out of R1 scope |
| CAP-020 | Application orchestration & compiler | R1 | 20 | 07 | implemented | `crates/domain/src/application.rs`, `crates/application/src/tests` (`compile_and_diff_roundtrip`, `publish_flow_and_bindings`) |
| CAP-021 | dyun agent & graph runtime | R1/R2 | 21 | 07 | implemented | `crates/dyun-adapter/src/dyun.rs`, `apps/dyun-agent/src/main.rs`; real dyun-gu runner integration in `R1-DYUN-*` |
| CAP-022 | Inference platform layering & federation | R3 | 22 | — | designed | `crates/dyun-adapter/src/lib.rs`; out of R1 scope |
| CAP-023 | Northbound API, events, webhooks, partner SDK | R3/R4 | 23 | — | designed | `crates/http-api/src/northbound.rs` (9 tests); full event/webhook SDK is R3 |
| CAP-024 | Web frontend architecture & data security | R1 | 24 | 08 | implemented | `apps/web/src/core/` (TenantContext, apiClient, security, uploadManager tests) |
| CAP-025 | All-in-one local deployment | R1 | 25 | 09 | designed | `deploy/onebox/README.md`, `deploy/compose/`; runnable Onebox in `R1-ONEBOX-*` |
| CAP-026 | Cluster HA reconciliation & multicluster | R3 | 26 | — | designed | `crates/scheduler/src/reconciler.rs` (13 tests); HA/multicluster is R3 |
| CAP-027 | Desktop clients & offline workflows | R4 | 27 | — | designed | `crates/desktop/src/lib.rs`; out of R1 scope |
| CAP-028 | Security, secrets & supply chain | R2 | 28 | 10 | implemented | `crates/auth/src/secrets.rs` (tests), `crates/observability/src/lib.rs` (redaction/audit tests), `cargo deny check` |
| CAP-029 | Observability, audit & operations | R2 | 29 | 10 | implemented | `crates/observability/src/lib.rs` (5 tests); dashboard/alert rules in `R1-OBS-005` |
| CAP-030 | Testing, simulators, hardware & performance | R2/R3 | 30 | 11 | implemented | `crates/test-harness/src/lib.rs` (6 tests); real hardware evidence via `.github/workflows/hardware-ci.yml` |
| CAP-031 | Packaging, migration, release & rollback | R2/R3 | 31 | 11 | implemented | `crates/release-manager/src/lib.rs` (4 tests), `crates/storage/src/bin/migrate.rs` |

## Definition of Release Columns

- **R0** — planning and baseline deliverables; no runtime.
- **R1** — Visual MVP: complete data → annotation → training → model →
  application → dyun-gu deployment on a single machine and on Kubernetes.
- **R2** — Production training: multi-node, checkpoint recovery, conversion matrix,
  enterprise audit, and HA.
- **R3** — Inference platform: multi-cluster/edge, canary, elasticity, and
  model-publish linkage.
- **R4** — Ecosystem: pipeline/HPO, notebook, partner SDK, and desktop client.
