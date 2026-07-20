# Moqentra Threat Model

## Scope

This document maps threats to controls and automated tests for the Moqentra
platform.

## Threats and Controls

| ID | Threat | Control | Test |
|---|---|---|---|
| T01 | Untrusted uploads execute malicious code | `SecurityLimits` max size / depth; container seccomp/cap-drop; sandbox path validation | `moqentra-desktop::validate_path` rejects `..` and symlinks |
| T02 | Cross-tenant data access | Tenant-scoped IDs; RBAC in `moqentra-auth`; request context validation | `moqentra-auth::rbac` tenant/project tests |
| T03 | Agent impersonation | mTLS; node certificates with thumbprint; lease fencing | `moqentra-scheduler::Lease` epoch/fencing tests |
| T04 | Secret leakage in logs | `SecretRedactor`; typed `SecretProvider` references | `moqentra-auth::secrets` redaction tests |
| T05 | SSRF from webhooks | URL validation rejects localhost/127.0.0.1/10.x/192.168.x | `moqentra-http-api::webhook_rejects_internal_addresses` |
| T06 | Container escape | Read-only rootfs; dropped capabilities; no privileged/hostPath | `moqentra-worker-control::root_container_rejected`; `moqentra-domain::notebook` rejects privileged |
| T07 | Supply-chain poisoning | Signed artifacts (`SignedArtifact`); SBOM/provenance references | Generated on release; verified before deployment |
| T08 | Message replay / stale events | `Revision` CAS; generation/fencing in replicas | `moqentra-scheduler::DesiredObserved` reconcile tests |
| T09 | XSS / desktop IPC abuse | HTML escaping; `IpcAllowlist` command/path/scheme allowlist | `moqentra-desktop` tests |
| T10 | Denial of service via oversized payloads | `SecurityLimits` on JSON/proto/upload/log/url | `moqentra-auth::security_limits_enforced` |

## Secret Handling

- Secrets are never stored in the database as plaintext.
- `SecretProvider` resolves from file, environment or external manager at runtime.
- Certificates rotate before expiry with overlapping active/previous thumbprints.
- The platform defaults to secure-fail: missing mTLS or invalid signatures reject requests.
