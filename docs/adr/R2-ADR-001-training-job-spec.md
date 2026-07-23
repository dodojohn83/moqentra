# R2-ADR-001: Training Job Spec — Replicas, Processes and World Size

## Status

Accepted

## Context

R1 `TrainingJobSpec` used `DistributedConfig::Single` or `Ddp { world_size }` with one process per pod. R2 must support multiple processes per replica, checkpoint policies, queue bindings and preemption. R1 clients must continue to work against the R2 control plane.

## Options Considered

| Option | Pros | Cons |
|---|---|---|
| Replace `Ddp { world_size }` with `replicas` and `processes_per_replica` only | Clean | Breaks stored R1 specs |
| Keep `Ddp { world_size }` and add `processes_per_replica` | Backward compatible | Requires validating `world_size == replicas * processes_per_replica` |

## Decision

Keep `DistributedConfig::Ddp { world_size }` and add `processes_per_replica`, `checkpoint_policy`, `queue_ref`, `priority_class_ref`, `preemption_policy` and `resource_class_ref` to `TrainingJobSpec`. `world_size()` returns the canonical world size; `canonicalize()` normalises R1-missing fields to single-replica, single-process, no preemption. Validation rejects inconsistent `world_size`.

## Consequences

- Positive: R1 specs deserialize unchanged; R2 specs can express multi-process replicas.
- Negative: Scale is configured in two places (`resources.replicas` and `distributed.world_size`).
- Risks: Callers must call `canonicalize()` before persisting; otherwise validation fails.

## Compliance

- Affected crates: `crates/domain`, `crates/scheduler`, `crates/k8s-executor`, `crates/http-api`.
- Changed contract: `TrainingJobSpec/v1`.

## References

- `crates/domain/src/training.rs`
- `dev-docs/004_r1_release_r2_production_training_plan/02_contracts_adrs_and_migrations.md`
