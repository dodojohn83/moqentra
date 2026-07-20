#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "${SCRIPT_DIR}/.env" ]]; then
  # shellcheck source=/dev/null
  source "${SCRIPT_DIR}/.env"
fi

export PGUSER="${POSTGRES_USER:-moqentra}"

BACKUP_ARCHIVE="$1"
BACKUP_DIR=$(mktemp -d)
trap 'rm -rf "$BACKUP_DIR"' EXIT

# Reject archives that contain absolute paths or parent-directory traversal.
if tar -tzf "$BACKUP_ARCHIVE" | grep -qE '^/|(^|/)\.\.(/|$)'; then
  echo "ERROR: backup archive contains absolute or traversal paths." >&2
  exit 1
fi

tar --no-same-owner -xzf "$BACKUP_ARCHIVE" -C "$BACKUP_DIR"

DUMP_FILE=$(find "$BACKUP_DIR" -name "pg_dump.sql" -print -quit)
if [[ -z "$DUMP_FILE" ]]; then
  echo "ERROR: pg_dump.sql not found in backup archive." >&2
  exit 1
fi

docker compose -f "$(dirname "$0")/docker-compose.yml" exec -T -e PGUSER postgres \
  psql < "$DUMP_FILE"

echo "Database restored from $BACKUP_ARCHIVE"
rm -rf "$BACKUP_DIR"
