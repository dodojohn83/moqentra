# Architecture Admission Process

Any request to add a capability that is currently **out of scope** MUST go
through this process. Local feature pages or proof-of-concepts cannot bypass it.

## Out-of-Scope List

- LLM fine-tuning, RAG, or model marketplace.
- Proprietary training frameworks or model compilers.
- General text/audio/3D annotation in R1/R2/R3.
- Micro-frontends and third-party frontend plugin runtimes.
- `cheetah-signaling` integration unless GB/T 28181 / ONVIF / cascade is explicitly required.

## Admission Steps

1. **Request** — Open an issue or ADR with the proposed capability, user,
   release target, and why it cannot be achieved with existing building blocks.
2. **Impact** — A maintainer assesses domain changes, API surface, storage
   schema, worker image, security boundary, and supply-chain impact.
3. **Prototype** — A throw-away branch demonstrates the integration contract
   without modifying the domain layer.
4. **Review** — Architecture and security review; must pass a threat-model
   checklist.
5. **Decision** — Approved capabilities are added to the capability matrix,
   release plan, and acceptance scenarios.
6. **Implementation** — Only then is code merged into `main`.

## Criteria for Approval

- The capability fits the industry-vision focus and release phase.
- It can be expressed through an existing versioned contract (`ApplicationSpec`,
  `TrainingJobSpec`, `DatasetManifest`, etc.) or a backward-compatible extension.
- It does not require the domain layer to depend on Axum, SQLx, Kubernetes, S3,
  Tonic transport, vendor SDKs, or frontend types.
- It includes tests, security analysis, and documentation updates.
- It does not introduce a floating dependency or unlicensed artifact.
