#!/usr/bin/env bash
set -euo pipefail

BACKUP_ARCHIVE="$1"
BACKUP_DIR=$(mktemp -d)
tar xzf "$BACKUP_ARCHIVE" -C "$BACKUP_DIR"

DUMP_FILE=$(find "$BACKUP_DIR" -name "pg_dump.sql" -print -quit)
if [[ -z "$DUMP_FILE" ]]; then
  echo "ERROR: pg_dump.sql not found in backup archive." >&2
  exit 1
fi

docker compose -f "$(dirname "$0")/docker-compose.yml" exec -T postgres \
  psql -U "${POSTGRES_USER:-moqentra}" < "$DUMP_FILE"

echo "Database restored from $BACKUP_ARCHIVE"
rm -rf "$BACKUP_DIR"
