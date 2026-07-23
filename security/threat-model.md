# Moqentra Threat Model

## Scope

This document maps threats to controls and automated tests for the Moqentra
platform (R1 vertical slice). Trust boundaries covered:

| Boundary | Components | Trust assumption |
|---|---|---|
| Browser | Web console (OIDC PKCE) | Untrusted; CSP + no secrets in localStorage |
| Control-plane HTTP | axum `/v1`, health | Authenticated principal + RBAC |
| PostgreSQL | metadata authority | RLS fail-closed; admin only for cross-tenant hydrate |
| Object storage | MinIO/S3 | Tenant-scoped object keys; short-lived signed URLs |
| Worker gRPC | Python workers | Session + fencing; no long-lived S3 keys in env dumps |
| User container images | training templates | Non-root, no privileged, digest-pinned |
| Node Agent | local OCI launch | Host isolation; no default Docker socket in Helm |
| Kubernetes | Job/VolcanoJob | Namespace/RBAC/NetworkPolicy |
| dyun-agent / runner | video graph | Signed bundle; SecretRef resolved only on node |
| Media inputs | RTSP/uploads | Size/type/URL/DNS limits; SSRF denylist |

## Threats and Controls

| ID | Threat | Control | Test |
|---|---|---|---|
| T01 | Untrusted uploads execute malicious code | `SecurityLimits` max size / depth; container seccomp/cap-drop; sandbox path validation | `moqentra-desktop::validate_path` rejects `..` and symlinks |
| T02 | Cross-tenant data access | Tenant-scoped IDs; RBAC in `moqentra-auth`; request context validation; RLS | `moqentra-auth::rbac` tenant/project tests |
| T03 | Agent impersonation | mTLS; node certificates with thumbprint; lease fencing | `moqentra-scheduler::Lease` epoch/fencing tests |
| T04 | Secret leakage in logs | `SecretRedactor`; typed `SecretProvider` / SecretRef | `moqentra-auth::secrets` redaction tests |
| T05 | SSRF from webhooks / import | URL validation rejects localhost/127.0.0.1/10.x/192.168.x | `moqentra-http-api::webhook_rejects_internal_addresses` |
| T06 | Container escape | Read-only rootfs; dropped capabilities; no privileged/hostPath | `moqentra-worker-control::root_container_rejected` |
| T07 | Supply-chain poisoning | Signed artifacts (`SignedArtifact`); SBOM/provenance references | Generated on release; verified before deployment |
| T08 | Message replay / stale events | `Revision` CAS; generation/fencing in replicas | `moqentra-scheduler::DesiredObserved` reconcile tests |
| T09 | XSS / desktop IPC abuse | HTML escaping; `IpcAllowlist` command/path/scheme allowlist | `moqentra-desktop` tests |
| T10 | Denial of service via oversized payloads | `SecurityLimits` on JSON/proto/upload/log/url | `moqentra-auth::security_limits_enforced` |
| T11 | Upload URL forgery | `MOQENTRA_UPLOAD_SIG_SECRET` required when auth enabled | control-plane startup validation |
| T12 | Outbox lease theft | `FOR UPDATE SKIP LOCKED` + lease owner/expiry | `pg_outbox` tests |
| T13 | Malicious media | Isolated validation worker before publish | media validation worker |

## Secret Handling (SecretRef)

- Secrets are never stored in the database as plaintext JSON fields.
- `SecretProvider` resolves from file, environment or external manager **only on the authorized execution node**.
- Specs, DyunGraphBundle, outbox events, metrics labels, CLI argv, and diagnostic packages must carry **SecretRef identifiers**, never raw secret material.
- Certificates rotate before expiry with overlapping active/previous thumbprints.
- The platform defaults to secure-fail: missing mTLS or invalid signatures reject requests.

## Service identity (R1)

| Identity | Issuance | Rotation |
|---|---|---|
| Browser user | OIDC access token | Issuer JWKS rotation |
| Service account | Static tokens (dev) / short-lived (prod) | Env re-deploy |
| Node / dyun agent | mTLS client cert (planned full PKI) | Thumbprint overlap |
| Worker session | gRPC session + fencing token | Per-attempt |

R1 production path requires OIDC for browser traffic; HMAC JWT is local-only.

