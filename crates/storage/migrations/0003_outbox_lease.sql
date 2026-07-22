-- R1 Task 02: outbox lease/retry and idempotency fingerprint columns.
-- Migrations are append-only; once published they must not be modified.

-- Lease and retry state for outbox delivery.
ALTER TABLE outbox_events
    ADD COLUMN IF NOT EXISTS lease_owner TEXT,
    ADD COLUMN IF NOT EXISTS lease_expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS next_retry_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS max_attempts INT NOT NULL DEFAULT 3;

CREATE INDEX IF NOT EXISTS idx_outbox_poll
    ON outbox_events(status, next_retry_at, created_at)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_outbox_lease
    ON outbox_events(lease_owner, lease_expires_at)
    WHERE status = 'processing';

-- Fingerprint for idempotency request identity checks.
ALTER TABLE idempotency_keys
    ADD COLUMN IF NOT EXISTS fingerprint TEXT;

CREATE INDEX IF NOT EXISTS idx_idempotency_fingerprint
    ON idempotency_keys(tenant_id, operation_type, key);
