# R2-ADR-004: Quota, Approval and Fair Queue Authority Boundaries

## Status

Accepted

## Context

R2 introduces multi-tenant scheduling. Operators need tenant/project quota enforcement and human approval for exceptional cases, while the scheduler needs fair queue policies and priority classes that are authoritative in the control plane.

## Options Considered

| Option | Pros | Cons |
|---|---|---|
| Hard policy limits only | Simple | No exceptions |
| External approval ticket | Flexible | No audit trail |
| Quota policy + immutable reservation + approval request + queue policy | Versioned, auditable, replayable | More tables |

## Decision

Introduce `QuotaPolicy`, `QuotaReservation`, `UsageLedgerEntry`, `UsageRollup`, `ApprovalRequest`, `QueuePolicy` and `PriorityClass` as first-class domain aggregates. Approval decisions are immutable snapshots tied to the policy revision at request time. Scheduler uses `QueuePolicy` and `PriorityClass` to construct Volcano objects but the authoritative versions live in PostgreSQL.

## Consequences

- Positive: Exact audit trail of who approved what and against which policy revision.
- Negative: Reservation expiry, usage rollup and queue reconciliation require background jobs.
- Risks: Clock skew between policy effective dates and ledger timestamps can create off-by-one enforcement errors.

## Compliance

- Affected crates: `crates/domain`, `crates/storage`, `crates/application`, `crates/scheduler`.
- New contracts: `QuotaPolicy/v1`, `QuotaUsage/v1`, `ApprovalRequest/v1`, `QueuePolicy/v1`.

## References

- `crates/domain/src/quota.rs`
- `crates/domain/src/approval.rs`
- `crates/domain/src/queue.rs`
