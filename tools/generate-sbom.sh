#!/usr/bin/env bash
# Generate SBOM / cargo-deny / license reports for release gates (R1-PKG-004 / R1-SEC-006).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/artifacts/r1-evidence/sbom}"
mkdir -p "$OUT"

echo "Writing reports to $OUT"
(
  cd "$ROOT"
  if command -v cargo-deny >/dev/null 2>&1; then
    cargo deny check 2>&1 | tee "$OUT/cargo-deny.txt" || true
  else
    echo "cargo-deny not installed; skip" | tee "$OUT/cargo-deny.txt"
  fi
  cargo tree -e normal 2>&1 | tee "$OUT/cargo-tree.txt" || true
  if command -v syft >/dev/null 2>&1; then
    syft dir:. -o spdx-json >"$OUT/sbom.spdx.json" || true
  else
    echo '{"note":"install syft to generate SPDX SBOM"}' >"$OUT/sbom.spdx.json"
  fi
)
echo "Done. Wire signatures/provenance in CI release workflow."
