# Architecture Decision Records

This directory records significant architectural decisions in Moqentra. Any
change that crosses a crate boundary, adds an external dependency, introduces a
new runtime, or modifies a versioned contract must be documented here before code
is merged.

## Index

| ADR | Title | Status |
|---|---|---|
| 0000 | Template | template |
| R2-001 | Training Job Spec — Replicas, Processes and World Size | accepted |
| R2-002 | Volcano Gang Scheduling, c10d Rendezvous and Fencing | accepted |
| R2-003 | Distributed Checkpoint Two-Phase Completion | accepted |
| R2-004 | Quota, Approval and Fair Queue Authority Boundaries | accepted |
| R2-005 | Scheduler Leader Election, Agent Session and Command/Ack HA | accepted |
| R2-006 | Conversion Profiles and Staged Release Gate | accepted |
| R2-007 | Production Profile, RPO/RTO and Audit Retention | accepted |

## Format

Use `0000-template.md` as the starting point. Number new ADRs sequentially and
update this index.
