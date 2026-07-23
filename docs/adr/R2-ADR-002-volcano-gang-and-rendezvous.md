# R2-ADR-002: Volcano Gang Scheduling, c10d Rendezvous and Fencing

## Status

Accepted

## Context

Distributed training in R2 must launch a deterministic number of pods, guarantee that all ranks meet before training starts, and cleanly abort the whole gang if any rank fails or if a stale fencing token is presented.

## Options Considered

| Option | Pros | Cons |
|---|---|---|
| Plain Kubernetes Jobs | Simple | No gang guarantee |
| Volcano `PodGroup` and `Job` | Gang scheduling, queue-aware | Requires cluster add-on |
| Custom gang controller | Full control | High operational cost |

## Decision

Use Volcano `Job` and `PodGroup` for gang scheduling. Each attempt creates a `GangGroup` with `min_available == total_members == world_size`. Ranks discover each other via a control-plane rendezvous table (`rendezvous_members`) keyed by attempt id. `Attempt` carries a monotonic fencing token; ranks and agents reject commands with stale tokens.

## Consequences

- Positive: Gang all-or-nothing scheduling and deterministic rendezvous.
- Negative: Volcano must be installed; `PodGroup` TTL tuning is required.
- Risks: Network partition between ranks and rendezvous can cause hangs; heartbeat timeouts must cover worst-case GC.

## Compliance

- Affected crates: `crates/scheduler`, `crates/k8s-executor`, `crates/domain`, `crates/storage`.
- New migrations: `ranks`, `rendezvous_members`, `scheduler_leases`.

## References

- `crates/scheduler/src/scheduler.rs`
- `crates/k8s-executor/src/lib.rs`
