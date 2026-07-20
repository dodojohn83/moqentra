# Moqentra One-Box Deployment

This directory contains an OCI Compose based single-machine deployment of
Moqentra, including the control plane, web console, PostgreSQL, MinIO, OIDC,
node-agent and dyun-agent.

## Quick start

```bash
./init.sh          # generates .env and random secrets
./preflight.sh     # checks ports, docker and GPU/NPU runtime
source .env
docker compose up -d
```

## Backup and restore

```bash
./backup.sh ./backups
./restore.sh ./backups/20250720-120000.tar.gz
```

## Data retention

Compose volumes `pgdata` and `miniodata` persist across `docker compose down`.
Use `docker compose down -v` with care; external directories are never removed.
