-- Add version lifecycle columns to dataset_versions.
ALTER TABLE dataset_versions
    ADD COLUMN IF NOT EXISTS state TEXT NOT NULL DEFAULT 'Draft',
    ADD COLUMN IF NOT EXISTS published_at TIMESTAMPTZ;

-- Index for scheduler/publisher queries by state.
CREATE INDEX IF NOT EXISTS idx_dataset_versions_state
    ON dataset_versions(tenant_id, project_id, state);
