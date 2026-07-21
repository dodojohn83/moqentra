-- R1 Task 02: add surrogate id to idempotency keys so completion can address a row directly.
-- Migrations are append-only; once published they must not be modified.

ALTER TABLE idempotency_keys
    ADD COLUMN IF NOT EXISTS id UUID DEFAULT uuid_generate_v4() NOT NULL;

CREATE INDEX IF NOT EXISTS idx_idempotency_id ON idempotency_keys(id);
