# Onebox Runbook

## Install

```bash
cd deploy/onebox
./init.sh                 # idempotent secrets + TLS
./preflight.sh
docker compose up -d --build
# or one-shot:
./smoke.sh
```

Uninstall keeps data volumes by default:

```bash
docker compose down       # volumes retained
# destructive:
docker compose down -v
```

## Status / logs

```bash
docker compose ps
docker compose logs -f control-plane scheduler web
curl -fs localhost:8080/healthz
curl -fs localhost:8080/readyz
```

## Upgrade

1. Pull/build new images (`docker compose build`).
2. Migration job runs before control-plane (`depends_on: completed_successfully`).
3. `docker compose up -d` performs expand-first compatible restart.

## Backup / restore

```bash
./backup.sh ./backups
./restore.sh ./backups/<timestamp>.tar.gz
```

Backup captures PostgreSQL dump and MinIO data volume snapshot consistency window
described in the script. After restore, re-run `./smoke.sh` with `SMOKE_START=0`
if stacks are already up.

## Stop / start

```bash
docker compose stop
docker compose start
```
