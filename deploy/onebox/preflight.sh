#!/usr/bin/env bash
set -euo pipefail

check_command() {
  if ! command -v "$1" &>/dev/null; then
    echo "ERROR: $1 is required but not installed." >&2
    return 1
  fi
}

check_port() {
  local port=$1
  if ss -ltn "sport = :$port" 2>/dev/null | grep -q "LISTEN"; then
    echo "ERROR: port $port is already in use." >&2
    return 1
  fi
}

check_gpu() {
  if command -v nvidia-smi &>/dev/null; then
    echo "INFO: NVIDIA GPU detected."
  elif command -v npu-smi &>/dev/null; then
    echo "INFO: Ascend NPU detected."
  else
    echo "INFO: No GPU/NPU runtime detected. Training and inference will use CPU."
  fi
}

check_command docker

check_port "${POSTGRES_PORT:-5432}"
check_port "${MINIO_API_PORT:-9000}"
check_port "${MINIO_CONSOLE_PORT:-9001}"
check_port "${OIDC_PORT:-5556}"
check_port "${CONTROL_PLANE_PORT:-8080}"
check_port "${WEB_PORT:-3000}"

if ! docker info &>/dev/null; then
  echo "ERROR: docker daemon is not running." >&2
  exit 1
fi

check_gpu

echo "Preflight checks passed."
