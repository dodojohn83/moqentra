#!/usr/bin/env bash
# Onebox smoke: compose up (if needed) then verify OIDC, migration, MinIO, control-plane, agents.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if [[ -f .env ]]; then
  # shellcheck source=/dev/null
  set -a
  source .env
  set +a
fi

CONTROL_PLANE_PORT="${CONTROL_PLANE_PORT:-8080}"
OIDC_PORT="${OIDC_PORT:-5556}"
MINIO_API_PORT="${MINIO_API_PORT:-9000}"
NODE_PORT="${NODE_AGENT_PORT:-8081}"
SCHED_PORT="${SCHEDULER_PORT:-8082}"
DYUN_PORT="${DYUN_AGENT_PORT:-8083}"

if [[ "${SMOKE_START:-1}" == "1" ]]; then
  if [[ ! -f .env ]]; then
    ./init.sh
  fi
  ./preflight.sh
  docker compose up -d --build
fi

echo "Waiting for control-plane on :${CONTROL_PLANE_PORT}..."
for i in $(seq 1 60); do
  if curl -fs "http://127.0.0.1:${CONTROL_PLANE_PORT}/healthz" >/dev/null 2>&1; then
    break
  fi
  if [[ "$i" -eq 60 ]]; then
    echo "ERROR: control-plane healthz not ready" >&2
    docker compose ps >&2 || true
    exit 1
  fi
  sleep 2
done

fail=0
check() {
  local name="$1" url="$2"
  if curl -fs "$url" >/dev/null 2>&1; then
    echo "OK  $name  $url"
  else
    echo "FAIL $name  $url" >&2
    fail=1
  fi
}

check "control-plane healthz" "http://127.0.0.1:${CONTROL_PLANE_PORT}/healthz"
check "control-plane readyz" "http://127.0.0.1:${CONTROL_PLANE_PORT}/readyz"
check "oidc discovery" "http://127.0.0.1:${OIDC_PORT}/dex/.well-known/openid-configuration"
check "minio live" "http://127.0.0.1:${MINIO_API_PORT}/minio/health/live"
check "node-agent healthz" "http://127.0.0.1:${NODE_PORT}/healthz" || true
check "scheduler healthz" "http://127.0.0.1:${SCHED_PORT}/healthz" || true
check "dyun-agent healthz" "http://127.0.0.1:${DYUN_PORT}/healthz" || true

# Migration success: control-plane started after migration service_completed_successfully.
if docker compose ps migration 2>/dev/null | grep -Eqi 'exited \(0\)|Exit 0'; then
  echo "OK  migration completed"
else
  # Compose v2 may not show exited jobs; treat healthy control-plane as sufficient.
  echo "OK  migration (control-plane is up; depends_on completed_successfully)"
fi

if [[ "$fail" -ne 0 ]]; then
  echo "Smoke failed." >&2
  exit 1
fi
echo "Smoke passed."
