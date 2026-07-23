# R2-ADR-005: Scheduler Leader Election, Agent Session and Command/Ack HA

## Status

Accepted

## Context

Node agents must continue running when the control plane is unavailable. The control plane must issue commands and agents must acknowledge them idempotently. Multiple control-plane replicas must not reconcile the same workload.

## Options Considered

| Option | Pros | Cons |
|---|---|---|
| gRPC bidirectional streams | Low latency | Hard to resume after partition |
| REST poll + command queue | Simple to resume, idempotent | Higher latency |
| REST poll + agent_sessions, agent_commands, command_acks + scheduler_leases | Durable, auditable, failover | Requires lease TTL tuning |

## Decision

Use polled command/ack model backed by `agent_sessions`, `agent_commands` and `command_acks`. Control-plane HA uses `scheduler_leases` and `reconciler_cursors` with lease TTLs. Only one active instance holds the lease per subsystem and drives reconciliation.

## Consequences

- Positive: Agent recovery and control-plane failover are deterministic.
- Negative: Command latency is bounded by the poll interval.
- Risks: Split-brain if lease TTL is shorter than clock skew or GC pause.

## Compliance

- Affected crates: `crates/storage`, `crates/scheduler`, `apps/scheduler`.
- New migrations: `agent_sessions`, `agent_commands`, `command_acks`, `scheduler_leases`, `reconciler_cursors`.

## References

- `crates/storage/migrations/0018_r2_contracts.sql`
