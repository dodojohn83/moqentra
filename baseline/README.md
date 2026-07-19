# Moqentra Baseline

This directory contains the frozen engineering baselines for the Moqentra
platform. These files are the source of truth for supported versions, licenses,
platforms, hardware support levels, and external blockers.

## Files

- `version-matrix.toml` — pinned toolchains, runtimes, frameworks, and platforms.
- `licenses.toml` — allowed and denied licenses per ecosystem.
- `platform-matrix.toml` — supported operating systems, CPU architectures,
  Kubernetes versions, object storage protocols, databases, and browsers.
- `hardware-support.toml` — supported / preview / compile-only / mock levels for
  NVIDIA, AMD, and Ascend hardware.
- `release-policy.md` — versioning, branching, tagging, changelog, and
  compatibility policy.
- `external-blockers.toml` — upstream SDK or hardware gaps that block a feature
  from reaching its target support level.

## Update Process

1. Any change to a baseline requires a PR and a reviewed impact analysis.
2. Version upgrades must reference a release note, commit, or digest.
3. License exceptions require legal review and an explicit `[[exceptions]]` entry.
4. Hardware support level promotions require real CI evidence and a linked
   `external-blockers` resolution.
