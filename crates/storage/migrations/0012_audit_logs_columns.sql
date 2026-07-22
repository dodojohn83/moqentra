-- Align audit_logs with the structured audit model introduced in 0006.
-- The initial schema already created an audit_logs table with actor/resource
-- text columns; this migration adds the structured fields and removes the
-- legacy columns that are no longer referenced.
ALTER TABLE audit_logs
    ADD COLUMN IF NOT EXISTS event_id TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS actor_type TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS actor_id TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS resource_type TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS resource_id TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS details JSONB NOT NULL DEFAULT '{}';

ALTER TABLE audit_logs
    ALTER COLUMN correlation_id SET NOT NULL;

ALTER TABLE audit_logs
    DROP COLUMN IF EXISTS actor,
    DROP COLUMN IF EXISTS resource;
