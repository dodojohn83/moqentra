#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SPEC="docs/openapi/openapi.yaml"

# Rust server types (shared models only).
RUST_OUT="${REPO_ROOT}/crates/openapi-types"
TMP_RUST=$(mktemp -d)
trap 'rm -rf "${TMP_RUST}"' EXIT
( cd "${REPO_ROOT}"; openapi-to-rust generate "${SPEC}" --output-dir "${TMP_RUST}" --module-name moqentra_api --types-only )
rm -rf "${RUST_OUT}/src/moqentra_api"
mkdir -p "${RUST_OUT}/src/moqentra_api"
cp "${TMP_RUST}/mod.rs" "${RUST_OUT}/src/moqentra_api/mod.rs"
cp "${TMP_RUST}/types.rs" "${RUST_OUT}/src/moqentra_api/types.rs"

# TypeScript client for the web app.
TS_OUT="${REPO_ROOT}/apps/web/src/generated/api"
TMP_TS=$(mktemp -d)
trap 'rm -rf "${TMP_TS}"' EXIT
( cd "${REPO_ROOT}"; npx --yes @openapitools/openapi-generator-cli generate \
  -g typescript-fetch \
  -i "${SPEC}" \
  -o "${TMP_TS}" \
  --additional-properties=hideGenerationTimestamp=true,modelPropertyNaming=original )
rm -rf "${TS_OUT}"
mkdir -p "$(dirname "${TS_OUT}")"
cp -R "${TMP_TS}" "${TS_OUT}"

# Python client.
PY_OUT="${REPO_ROOT}/python/moqentra_client"
TMP_PY=$(mktemp -d)
trap 'rm -rf "${TMP_PY}"' EXIT
( cd "${REPO_ROOT}"; npx --yes @openapitools/openapi-generator-cli generate \
  -g python \
  -i "${SPEC}" \
  -o "${TMP_PY}" \
  --additional-properties=hideGenerationTimestamp=true,packageName=moqentra_client )
rm -rf "${PY_OUT}"
mkdir -p "$(dirname "${PY_OUT}")"
cp -R "${TMP_PY}" "${PY_OUT}"

# Normalize Rust formatting so the generated code matches what cargo fmt produces.
( cd "${REPO_ROOT}"; cargo fmt --all )
