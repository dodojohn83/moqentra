#!/usr/bin/env bash
set -euo pipefail

BACKUP_DIR="${1:-./backup}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
DEST="${BACKUP_DIR}/${TIMESTAMP}"
mkdir -p "$DEST"

docker compose -f "$(dirname "$0")/docker-compose.yml" exec -T postgres \
  pg_dumpall -U "${POSTGRES_USER:-moqentra}" > "${DEST}/pg_dump.sql"

docker run --rm --network host --entrypoint sh \
  minio/mc -c "mc alias set local http://localhost:${MINIO_API_PORT:-9000} ${MINIO_ROOT_USER:-minioadmin} ${MINIO_ROOT_PASSWORD:-minioadmin} && mc mirror local/moqentra ${DEST}/minio" \
  || true

tar czf "${DEST}.tar.gz" -C "$BACKUP_DIR" "$TIMESTAMP"
rm -rf "$DEST"
echo "Backup created at ${DEST}.tar.gz"
