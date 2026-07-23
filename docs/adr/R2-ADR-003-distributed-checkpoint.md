# R2-ADR-003: Distributed Checkpoint Two-Phase Completion, Compatibility and GC

## Status

Accepted

## Context

R2 distributed training checkpoints one shard per rank. Resuming from a checkpoint may use a different world size or resource class, so the checkpoint must carry enough metadata to detect mismatches and support resharding.

## Options Considered

| Option | Pros | Cons |
|---|---|---|
| One object per rank, no manifest | Simple | No atomicity, hard to list and GC |
| Manifest + per-rank shards | Atomic, listable, GC-safe | Larger single object |

## Decision

Use a single `CheckpointManifest` aggregate containing `CheckpointShard` records for every rank. A two-phase completion protocol sets state to `Complete` only after all shard digests are validated. `CheckpointHold` records protect manifests from garbage collection. The manifest stores `framework`, `template`, `code_digest`, `image_digest`, `world_size` and a `compatibility` map for resume validation.

## Consequences

- Positive: Resumes can validate code/image compatibility before accepting a checkpoint.
- Negative: Large manifests may approach JSONB limits for huge tensor layouts.
- Risks: A missing or duplicate rank shard must be rejected at validation time.

## Compliance

- Affected crates: `crates/domain`, `crates/storage`.
- New contract: `CheckpointManifest/v1`.

## References

- `crates/domain/src/checkpoint_manifest.rs`
