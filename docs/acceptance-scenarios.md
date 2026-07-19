# Acceptance Scenarios

This file lists the high-level acceptance scenarios for each release phase.
Detailed test cases are added to the test suites as the implementation
progresses.

## R0 — Planning and Baseline

- **TAS-001** — A developer can reconstruct the full toolchain from
  `baseline/version-matrix.toml` without using floating tags.

## R1 — Visual MVP

- **TAS-002** — Workspace compiles and passes `cargo fmt/clippy/nextest`;
  web packages pass lint/typecheck/unit tests; Python passes lint/typecheck/unit
  tests.
- **TAS-003** — Foundation types implement `TenantId` scoping, error taxonomy,
  and configuration validation.
- **TAS-004** — Protobuf and OpenAPI code generation produce deterministic output.
- **TAS-005** — Tenant A cannot read, modify, or infer the existence of Tenant B
  resources through API, database, object storage, logs, or metrics.
- **TAS-006** — Repository contract tests pass against a real PostgreSQL
  database; outbox and idempotency tests survive process restart.
- **TAS-007** — Dataset upload, S3 import, version freeze, and checksum
  manifest round-trip correctly using MinIO.
- **TAS-008** — LabelU-Kit embedded in the web console can annotate and export
  image and video tasks.
- **TAS-009** — Data quality checks and automatic pre-labeling run in isolated
  worker processes.
- **TAS-010** — Classification and detection training jobs complete on a single
  machine and produce a reproducible model version.
- **TAS-011** — Node agent can execute a local OCI worker, stream logs, and
  return artifacts.
- **TAS-012** — Volcano job scheduler dispatches a training job to a Kubernetes
  node and reports status.
- **TAS-013** — Multi-node DDP training completes and checkpoint recovery
  resumes from the latest checkpoint.
- **TAS-014** — Model registry stores lineage (dataset version, training run,
  template, code commit) and artifact checksums.
- **TAS-015** — ONNX conversion produces a valid model; TensorRT / OpenVINO
  conversion reaches preview.
- **TAS-016** — Pipeline/HPO/Notebook services are documented as R4 scope.
- **TAS-017** — Application DAG compiles to a `dg/v1 Graph` bundle with
  deterministic checksum.
- **TAS-018** — dyun-gu agent deploys, runs, stops, and reports status for an
  RTSP → detect → track → OSD → RTMP pipeline.
- **TAS-019** — Northbound REST API, events, webhooks, and partner SDK follow
  versioned contracts.
- **TAS-020** — Web frontend passes CSP/CSRF/XSS/upload isolation security tests.
- **TAS-021** — All-in-one local deployment installs with one command and runs
  the R1 end-to-end flow.
- **TAS-022** — Cluster HA reconciliation and multicluster are documented as R3.
- **TAS-023** — Desktop clients and offline workflows are documented as R4.
- **TAS-024** — Security, secrets, and supply-chain scans pass with no
  high-severity findings.
- **TAS-025** — Observability stack exports traces, metrics, and structured
  logs.
- **TAS-026** — Hardware simulators and performance benchmarks produce
  reproducible baselines.
- **TAS-027** — Packaging, migration, and rollback procedures are tested from
  N to N+1 and back.

## R2 — Production Training

- **TAS-028** — Multi-tenant training with quotas, approvals, and enterprise
  audit passes a penetration test.
- **TAS-029** — Conversion matrix supports ONNX, TensorRT, OpenVINO, Ascend OM,
  RKNN, and Sophon with target hardware tags.
- **TAS-030** — PostgreSQL and object storage backup/restore completes within the
  RTO/RPO window.

## R3 — Inference Platform

- **TAS-031** — Inference graph canary publish and rollback complete without
  interrupting active streams.
- **TAS-032** — Multi-cluster inference federation routes tasks based on
  hardware capacity and latency.

## R4 — Ecosystem

- **TAS-033** — Partner SDK and desktop client reach preview quality.
- **TAS-034** — Pipeline/HPO and notebook services run isolated from the
  production control plane.
