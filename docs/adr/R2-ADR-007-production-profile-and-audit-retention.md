# R2-ADR-007: Production Profile, RPO/RTO and Audit Retention

## Status

Accepted

## Context

R2 production training requires a clear boundary between HA Kubernetes deployments and the onebox development stack, plus long-term audit retention for compliance.

## Options Considered

| Option | Pros | Cons |
|---|---|---|
| Single table audit log | Simple | Performance degrades with size |
| Time-based partitioning | Query pruning, easier archival | DDL complexity |
| External audit sink | Unlimited retention | Extra infrastructure |

## Decision

Partition `audit_log` by month on `recorded_at` using PostgreSQL declarative partitioning. A default partition catches out-of-range timestamps. Production Kubernetes profile targets 99.9% availability with documented RPO/RTO; the onebox compose stack is explicitly non-HA and only for development.

## Consequences

- Positive: Recent audits stay fast; old partitions can be detached and archived.
- Negative: Partitions must be created ahead of time by a scheduled job.
- Risks: Incorrect partition bounds can reject valid audit rows.

## Compliance

- Affected crates: `crates/storage`, `crates/auth`.
- New migrations: `0018_r2_contracts.sql`.

## References

- `crates/storage/migrations/0018_r2_contracts.sql`
