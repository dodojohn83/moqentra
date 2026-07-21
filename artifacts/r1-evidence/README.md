# R1 Evidence Artifacts

This directory holds build-specific evidence for the R1 vertical slice. Each run
produces a subdirectory named `<build-id>/` (e.g., `20250721-120000-utc/`).

## Directory layout per build

```text
artifacts/r1-evidence/<build-id>/
  version.json              # commit SHA, dirty flag, version, platform matrix
  commands.txt              # exact commands used to produce evidence
  junit/                    # JUnit XML from Rust/Web/Python tests
  logs/                     # service logs and test output
  reports/                  # SBOM, license, vulnerability, provenance
  media/                    # screenshots, short videos, model outputs
  summaries/                # manifest digests, metrics, summary JSON
  failures/                 # fault injection results and RTO/data loss notes
```

## Rules

- Do not commit large binary files to Git. Subdirectories under `artifacts/r1-evidence/`
  are ignored by `.gitignore`.
- Keep this `README.md` and `.gitkeep` so the directory exists in a fresh clone.
- Evidence is produced by local runs, CI artifacts, or `./tools/benchmarks/run-hardware-test.sh`.
- The release gate reads the `version.json` and report references; it does not trust
  a single boolean flag.
