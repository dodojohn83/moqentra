#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

( cd "${REPO_ROOT}"; ./tools/generate-api-clients.sh )

cd "${REPO_ROOT}"
git diff --exit-code \
  crates/openapi-types/src/moqentra_api \
  apps/web/src/generated/api \
  python/moqentra_client \
  2>&1
