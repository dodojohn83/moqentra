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

check_arch() {
  local arch
  arch=$(uname -m)
  if [[ "$arch" != "x86_64" && "$arch" != "aarch64" ]]; then
    echo "ERROR: architecture $arch is not supported; use x86_64 or aarch64." >&2
    return 1
  fi
}

check_memory() {
  local min_mb=${1:-4096}
  local avail_mb
  avail_mb=$(awk '/MemAvailable:/ {print int($2/1024)}' /proc/meminfo 2>/dev/null || echo 0)
  if [[ "$avail_mb" -lt "$min_mb" ]]; then
    echo "WARNING: less than ${min_mb}MB available memory (${avail_mb}MB)." >&2
  fi
}

check_disk() {
  local min_gb=${1:-20}
  local avail_gb
  avail_gb=$(df -BG . 2>/dev/null | awk 'NR==2 {print int($4)}' || echo 0)
  if [[ "$avail_gb" -lt "$min_gb" ]]; then
    echo "WARNING: less than ${min_gb}GB available disk space (${avail_gb}GB)." >&2
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
check_command ss
check_command awk

check_arch
check_memory 4096
check_disk 20

check_port "${POSTGRES_PORT:-5432}"
check_port "${MINIO_API_PORT:-9000}"
check_port "${MINIO_CONSOLE_PORT:-9001}"
check_port "${OIDC_PORT:-5556}"
check_port "${CONTROL_PLANE_PORT:-8080}"
check_port "${WEB_PORT:-3000}"
check_port "${NODE_AGENT_PORT:-8081}"
check_port "${SCHEDULER_PORT:-8082}"
check_port "${DYUN_AGENT_PORT:-8083}"

if ! docker info &>/dev/null; then
  echo "ERROR: docker daemon is not running." >&2
  exit 1
fi

# RTSP/RTMP and media validation tooling used by end-to-end evidence tasks.
for tool in ffmpeg ffprobe; do
  if command -v "$tool" &>/dev/null; then
    echo "INFO: $tool found."
  else
    echo "WARNING: $tool not found; RTSP/RTMP and media validation tests will be skipped." >&2
  fi
done

check_gpu

echo "Preflight checks passed."
