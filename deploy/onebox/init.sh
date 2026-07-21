#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${SCRIPT_DIR}/.env"

for cmd in openssl python3; do
  if ! command -v "$cmd" &>/dev/null; then
    echo "ERROR: $cmd is required but not installed." >&2
    exit 1
  fi
done

if [[ -f "$ENV_FILE" ]]; then
  # shellcheck source=/dev/null
  source "$ENV_FILE"
fi

mkdir -p "${SCRIPT_DIR}/secrets" "${SCRIPT_DIR}/data" "${SCRIPT_DIR}/certs"

# Generate self-signed certificate if none exists.
if [[ ! -f "${SCRIPT_DIR}/certs/tls.crt" ]]; then
  openssl req -x509 -newkey rsa:2048 -keyout "${SCRIPT_DIR}/certs/tls.key" \
    -out "${SCRIPT_DIR}/certs/tls.crt" -days 365 -nodes \
    -subj "/CN=localhost" 2>/dev/null
  chmod 600 "${SCRIPT_DIR}/certs/tls.key"
fi

# Generate random secrets when defaults are still in place.
if [[ "${POSTGRES_PASSWORD:-moqentra}" == "moqentra" ]]; then
  POSTGRES_PASSWORD=$(openssl rand -hex 24)
  MINIO_ROOT_PASSWORD=$(openssl rand -hex 24)
  OIDC_CLIENT_SECRET=$(openssl rand -base64 32)
  ADMIN_PASSWORD=$(openssl rand -hex 16)
  # bcrypt cost factor 10 (2^10 rounds); Dex accepts $2a/$2b/$2y hashes.
  ADMIN_PASSWORD_HASH=$(python3 - "$ADMIN_PASSWORD" <<'PY'
import crypt, sys
password = sys.argv[1]
salt = crypt.mksalt(crypt.METHOD_BLOWFISH, rounds=10)
print(crypt.crypt(password, salt))
PY
)
  # Docker Compose .env files interpolate $VAR; escape literal $ as $$.
  ADMIN_PASSWORD_HASH_ESCAPED=$(printf '%s' "$ADMIN_PASSWORD_HASH" | sed 's/\$/$$/g')
  {
    echo "POSTGRES_USER=${POSTGRES_USER:-moqentra}"
    echo "POSTGRES_PASSWORD=$POSTGRES_PASSWORD"
    echo "POSTGRES_DB=${POSTGRES_DB:-moqentra}"
    echo "POSTGRES_PORT=${POSTGRES_PORT:-5432}"
    echo "MINIO_ROOT_USER=${MINIO_ROOT_USER:-minioadmin}"
    echo "MINIO_ROOT_PASSWORD=$MINIO_ROOT_PASSWORD"
    echo "MINIO_API_PORT=${MINIO_API_PORT:-9000}"
    echo "MINIO_CONSOLE_PORT=${MINIO_CONSOLE_PORT:-9001}"
    echo "OIDC_PORT=${OIDC_PORT:-5556}"
    echo "CONTROL_PLANE_PORT=${CONTROL_PLANE_PORT:-8080}"
    echo "WEB_PORT=${WEB_PORT:-3000}"
    echo "OIDC_CLIENT_SECRET=$OIDC_CLIENT_SECRET"
    echo "MOQENTRA_ADMIN_PASSWORD=$ADMIN_PASSWORD"
    echo "MOQENTRA_ADMIN_PASSWORD_HASH=$ADMIN_PASSWORD_HASH_ESCAPED"
  } > "$ENV_FILE"
  chmod 600 "$ENV_FILE"
  echo "Generated $ENV_FILE with random passwords."
fi

echo "One-box initialization complete. Run './preflight.sh' then 'docker compose up -d'."
