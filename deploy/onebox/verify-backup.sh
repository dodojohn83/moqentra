#!/usr/bin/env bash
# R1-REC-003 partial: after restore, verify table counts, object listing, RLS presence.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if [[ -f .env ]]; then
  # shellcheck source=/dev/null
  set -a
  source .env
  set +a
fi

POSTGRES_USER="${POSTGRES_USER:-moqentra}"
POSTGRES_DB="${POSTGRES_DB:-moqentra}"
CONTAINER="${POSTGRES_CONTAINER:-$(docker compose ps -q postgres)}"

if [[ -z "$CONTAINER" ]]; then
  echo "ERROR: postgres container not found" >&2
  exit 1
fi

echo "Checking core table counts..."
docker exec -i "$CONTAINER" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 <<'SQL'
SELECT 'tenants' AS rel, COUNT(*) FROM tenants
UNION ALL SELECT 'projects', COUNT(*) FROM projects
UNION ALL SELECT 'datasets', COUNT(*) FROM datasets
UNION ALL SELECT 'dataset_versions', COUNT(*) FROM dataset_versions
UNION ALL SELECT 'training_jobs', COUNT(*) FROM training_jobs
UNION ALL SELECT 'models', COUNT(*) FROM models
UNION ALL SELECT 'outbox_events', COUNT(*) FROM outbox_events
ORDER BY 1;
SQL

echo "Checking RLS enabled on business tables..."
docker exec -i "$CONTAINER" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 -c \
  "SELECT relname, relrowsecurity FROM pg_class WHERE relname IN ('datasets','training_jobs','models','outbox_events') ORDER BY 1;"

echo "MinIO health..."
curl -fs "http://127.0.0.1:${MINIO_API_PORT:-9000}/minio/health/live" >/dev/null
echo "OK MinIO live"

echo "Backup verification checks completed."
