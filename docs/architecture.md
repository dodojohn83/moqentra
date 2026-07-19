# Moqentra Architecture

## Overview

Moqentra uses a **modular monolith** for the control plane. The same domain and
application crates run inside multiple process personalities:

- `moqentra-control-plane` — HTTP/gRPC API and northbound entry.
- `moqentra-scheduler-agent` — background reconcilers, job dispatcher, outbox relay.
- `moqentra-node-agent` — local executor and worker supervisor.
- `moqentra-dyun-agent` — dyun-gu graph runtime supervisor.

All processes share `moqentra-domain`, `moqentra-application`, and the same
versioned contracts.

## Crate Layers

```text
┌─────────────────────────────────────────────────────────────┐
│  apps          control-plane  scheduler  node-agent  dyun  │
├─────────────────────────────────────────────────────────────┤
│  adapters      http-api  storage  object-store  auth         │
│                scheduler  worker-control  dyun-adapter       │
│                observability                                 │
├─────────────────────────────────────────────────────────────┤
│  application   use cases, ports, transaction boundaries      │
├─────────────────────────────────────────────────────────────┤
│  domain        state machines, invariants, pure logic         │
├─────────────────────────────────────────────────────────────┤
│  contracts     versioned specs and generated stubs           │
├─────────────────────────────────────────────────────────────┤
│  types         identifiers, value objects, shared errors     │
└─────────────────────────────────────────────────────────────┘
```

## Dependency Rules

- `moqentra-types` has no internal dependencies.
- `moqentra-contracts` depends only on `moqentra-types` and generated runtime stubs.
- `moqentra-domain` does not depend on transport, storage, or runtime crates
  (`axum`, `sqlx`, `kube`, `s3`, `tonic-transport`, `tokio`, etc.).
- `moqentra-application` declares repository, clock, identity, event, executor,
  artifact, and policy ports. It does not depend on adapter implementations.
- Adapters depend on `application` ports, not on each other.
- Web frontend accesses the platform only through the public HTTP/gRPC API and
  never directly touches databases, object storage, workers, or dyun-gu.

## Synchronous vs Asynchronous Operations

| Pattern | Use | Example |
|---|---|---|
| Sync API command | Fast, idempotent, returns resource or conflict. | `CreateDataset`, `UpdateProject` |
| Async Operation | Long running, cancellable, resumable. | `StartTraining`, `ConvertModel`, `DeployApplication` |
| Event | Change notification, outbox-ordered. | `TrainingRunUpdated`, `ModelVersionPublished` |

- Sync commands use explicit transaction boundaries and outbox tables for side effects.
- Async operations are modelled as state machines in the domain layer.
- Background reconcilers poll or subscribe to operations; they are bounded,
  paginated, cancelable, and idempotent.

## Background Execution Units

- `OutboxRelay` — forwards committed outbox events to event handlers.
- `JobDispatcher` — matches pending training jobs to available workers.
- `TrainingReconciler` — drives training run state machines.
- `ArtifactReconciler` — validates and registers model artifacts.
- `DeploymentReconciler` — publishes and rolls back application deployments.
- `GarbageCollector` — reclaims orphaned temporary objects and old versions.

Each unit:
- processes work in bounded batches;
- carries a `revision` or fencing token to tolerate restarts;
- uses idempotency keys for every external mutation;
- records progress in the database before emitting the next event.

## External System Ports

Every external system is accessed through an adapter implementing an
application-defined port:

| External System | Port Responsibility | Adapter Crate |
|---|---|---|
| PostgreSQL | Relational metadata and outbox | `moqentra-storage` |
| MinIO/S3/Ceph | Object storage and signed URLs | `moqentra-object-store` |
| OIDC / LDAP | Identity and token validation | `moqentra-auth` |
| Kubernetes / Volcano | Batch job scheduling | `moqentra-scheduler` |
| Python gRPC worker | Training and conversion | `moqentra-worker-control` |
| dyun-gu | Graph deployment and runtime | `moqentra-dyun-adapter` |
| Prometheus / OTLP | Metrics, traces, logs | `moqentra-observability` |

All adapters define explicit timeouts, retries, circuit breakers, and error
mapping to domain errors. Adapters never call each other directly.

## Testing Strategy

- Domain tests are pure, require no network, database, Tokio runtime, or
  vendor SDK, and run with `cargo test`.
- Application tests use in-memory repository and clock implementations.
- Adapter tests require real PostgreSQL, MinIO, or hardware runners and are
  marked with `#[cfg(feature = "integration")]`.
- End-to-end tests run against the local all-in-one deployment.

## Architecture Decision Records

Every cross-boundary change must be documented in `docs/adr/` using the
`0000-template.md` format. See `docs/adr/README.md` for the index.
