# Moqentra Operations Runbooks

This directory contains runbooks for operating Moqentra in production.

## Runbooks

### `database-outage.md`

Symptoms: API returns `UNAVAILABLE` or `TIMEOUT` for storage calls.
Steps:
1. Check PostgreSQL connection pool metrics (`moqentra_db_pool_active`).
2. Verify DNS and firewall connectivity to the DB endpoint.
3. Restart read-only control-plane replicas; they will resume from outbox.
4. If failover happened, run `moqentra-migrate` against the new leader.

### `object-store-latency.md`

Symptoms: S3 timeouts, multipart upload failures.
Steps:
1. Check `moqentra_object_store_latency_seconds` histogram.
2. Verify MinIO/S3 endpoint health and certificate validity.
3. Increase upload concurrency or reduce chunk size in `TrainingJobSpec`.
4. Alert if error rate exceeds `0.1%` for 5 minutes.

### `queue-backlog.md`

Symptoms: `moqentra_reconciler_backlog` keeps growing.
Steps:
1. Identify the backed-off reconciler from metric label `name`.
2. Scale the corresponding worker or reduce `max_events_per_cycle`.
3. Inspect dead-letter queue for repeated failures.

### `node-failure.md`

Symptoms: Agent heartbeats missing, `Lease` expired.
Steps:
1. Confirm node lease expiry and mark attempts as failed.
2. Reschedule affected `TrainingJob` attempts on a healthy node.
3. Restore from latest compatible checkpoint.

### `certificate-rotation.md`

Symptoms: `Certificate::should_rotate` returns true.
Steps:
1. Issue new certificate with overlapping validity window.
2. Update `previous_thumbprint` on old cert and `active=false` after grace period.
3. Verify mTLS handshakes succeed on both thumbprints.

### `disk-pressure.md`

Symptoms: `moqentra_node_disk_used_percent` > 85%.
Steps:
1. Evict old artifact cache and incomplete multipart uploads.
2. Lower local executor cache quota.
3. Add nodes or volumes.

### `training-storm.md`

Symptoms: Queue full, many jobs timing out, GPU utilization > 95%.
Steps:
1. Pause low-priority queues.
2. Increase preemption threshold for best-effort jobs.
3. Add reserved nodes with matching accelerator capability.
