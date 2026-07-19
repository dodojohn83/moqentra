# Release and Branch Policy

## Versioning

Moqentra follows [Semantic Versioning 2.0.0](https://semver.org/):

- `MAJOR` — breaking changes to public REST/gRPC APIs, storage migrations that
  cannot roll back, or incompatible `ApplicationSpec` schema changes.
- `MINOR` — new features, new adapters, new supported hardware, backward-compatible
  API additions.
- `PATCH` — bug fixes, security patches, documentation corrections.

## Branches

| Branch | Purpose | Lifespan |
|---|---|---|
| `main` | Current development. All feature PRs target `main`. | permanent |
| `release/x.y` | Stabilization branch for the `x.y` minor line. | until EOL |
| `hotfix/x.y.z` | Emergency patches cut from `release/x.y`. | until merged |

## Tags

- Release tags: `vX.Y.Z` (e.g. `v0.1.0`).
- Pre-release tags: `vX.Y.Z-rc.N`, `vX.Y.Z-beta.N`.
- Docker/OCI image tags include the same version and a content digest suffix:
  `moqentra/control-plane:v0.1.0@sha256:<digest>`.

## Changelog

- Each repository keeps `CHANGELOG.md` at the workspace root.
- Follow the [Keep a Changelog](https://keepachangelog.com/) format.
- Every PR must update `CHANGELOG.md` under `Unreleased` or the relevant release section.
- Categories: Added, Changed, Deprecated, Removed, Fixed, Security.

## Compatibility Window

- Public REST and gRPC APIs are supported for at least one `MINOR` release.
- Database migrations must remain reversible within the same `MAJOR` line.
- `ApplicationSpec` and `TrainingJobSpec` schema versions are supported for
  two `MINOR` releases before deprecation.
- Worker image tags are pinned by default; a compatibility matrix is kept in
  `baseline/version-matrix.toml`.

## Commit Messages

- Use conventional commits:
  - `feat(scope): summary`
  - `fix(scope): summary`
  - `docs(scope): summary`
  - `refactor(scope): summary`
  - `test(scope): summary`
  - `chore(scope): summary`
- Each commit must be signed and pushed through the PR workflow.
- Squash merges are preferred for feature PRs.
