#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "${SCRIPT_DIR}/.env" ]]; then
  # shellcheck source=/dev/null
  source "${SCRIPT_DIR}/.env"
fi

# Avoid leaking secrets in `ps` output by exporting them and passing only the
# variable names to docker.
export PGUSER="${POSTGRES_USER:-moqentra}"
export MINIO_API_PORT="${MINIO_API_PORT:-9000}"
export MINIO_ROOT_USER="${MINIO_ROOT_USER:-minioadmin}"
export MINIO_ROOT_PASSWORD="${MINIO_ROOT_PASSWORD:-minioadmin}"
export BACKUP_DEST

BACKUP_DIR="${1:-${SCRIPT_DIR}/backup}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
DEST="${BACKUP_DIR}/${TIMESTAMP}"
mkdir -p "$DEST"
BACKUP_DEST="${DEST}/minio"
trap 'rm -rf "$DEST"' EXIT

docker compose -f "$(dirname "$0")/docker-compose.yml" exec -T -e PGUSER postgres \
  pg_dumpall > "${DEST}/pg_dump.sql"

docker run --rm --network host --entrypoint sh \
  -e MINIO_API_PORT \
  -e MINIO_ROOT_USER \
  -e MINIO_ROOT_PASSWORD \
  -e BACKUP_DEST \
  minio/mc -c 'mc alias set local "http://localhost:${MINIO_API_PORT}" "${MINIO_ROOT_USER}" "${MINIO_ROOT_PASSWORD}" && mc mirror local/moqentra "${BACKUP_DEST}"' \
  || true

tar czf "${DEST}.tar.gz" -C "$BACKUP_DIR" "$TIMESTAMP"
rm -rf "$DEST"
echo "Backup created at ${DEST}.tar.gz"
