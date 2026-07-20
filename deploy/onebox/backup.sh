#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "${SCRIPT_DIR}/.env" ]]; then
  # shellcheck source=/dev/null
  source "${SCRIPT_DIR}/.env"
fi

BACKUP_DIR="${1:-${SCRIPT_DIR}/backup}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
DEST="${BACKUP_DIR}/${TIMESTAMP}"
mkdir -p "$DEST"
BACKUP_DEST="${DEST}/minio"

ENV_FILE="$(mktemp)"
trap 'rm -rf "$DEST" "$ENV_FILE"' EXIT
chmod 600 "$ENV_FILE"

{
  echo "MINIO_API_PORT=${MINIO_API_PORT:-9000}"
  echo "MINIO_ROOT_USER=${MINIO_ROOT_USER:-minioadmin}"
  echo "MINIO_ROOT_PASSWORD=${MINIO_ROOT_PASSWORD:-minioadmin}"
  echo "BACKUP_DEST=${BACKUP_DEST}"
} > "$ENV_FILE"

docker compose -f "$(dirname "$0")/docker-compose.yml" exec -T -e "PGUSER=${POSTGRES_USER:-moqentra}" postgres \
  pg_dumpall > "${DEST}/pg_dump.sql"

docker run --rm --network host --env-file "$ENV_FILE" \
  -v "${DEST}:${DEST}" --entrypoint sh \
  minio/mc -c 'mc alias set local "http://localhost:${MINIO_API_PORT}" "${MINIO_ROOT_USER}" "${MINIO_ROOT_PASSWORD}" && mc mirror local/moqentra "${BACKUP_DEST}"' \
  || true

tar czf "${DEST}.tar.gz" -C "$BACKUP_DIR" "$TIMESTAMP"
rm -rf "$DEST"
echo "Backup created at ${DEST}.tar.gz"
