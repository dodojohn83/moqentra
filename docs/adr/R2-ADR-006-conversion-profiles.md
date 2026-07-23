# R2-ADR-006: Conversion Profiles and Staged Release Gate

## Status

Accepted

## Context

R1 conversion used an inline `ConversionProfile` with no identity or support tier. R2 needs profile versioning, hardware-specific toolchains, and a gate from evaluation results to model promotion.

## Options Considered

| Option | Pros | Cons |
|---|---|---|
| Inline profile per conversion job | Self-contained | Duplication |
| Named `ConversionProfile` with revision and `support_tier` | Auditable, reusable | Extra CRUD surface |

## Decision

Promote `ConversionProfile` to a first-class resource with `id`, `name`, `revision`, `support_tier` and `parameter_schema`. The `support_tier` enum (`Verified` / `Preview` / `CompileOnly` / `Unsupported`) gates whether evaluation and promotion are attempted. Promotion requests can go through `ApprovalRequest` when policy requires it.

## Consequences

- Positive: Toolchain images and target chips are versioned and auditable.
- Negative: Profile change requires a new revision, increasing storage.
- Risks: A profile marked `CompileOnly` that becomes `Verified` needs a new revision.

## Compliance

- Affected crates: `crates/domain`, `crates/storage`, `crates/application`.
- Changed contract: `ConversionProfile/v1`.

## References

- `crates/domain/src/conversion.rs`
