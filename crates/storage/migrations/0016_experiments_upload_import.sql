-- R1 control-plane recovery: experiments, upload sessions, import jobs,
-- and admin bypass for model / training tables used by multi-tenant hydrate.

CREATE TABLE IF NOT EXISTS experiments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_experiments_tenant_project
    ON experiments(tenant_id, project_id);

ALTER TABLE experiments ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS tenant_isolation ON experiments;
CREATE POLICY tenant_isolation ON experiments
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());

CREATE TABLE IF NOT EXISTS upload_sessions (
    id TEXT PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID NOT NULL,
    state TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_upload_sessions_expires
    ON upload_sessions(expires_at)
    WHERE state = 'Pending';

CREATE INDEX IF NOT EXISTS idx_upload_sessions_tenant
    ON upload_sessions(tenant_id, project_id);

ALTER TABLE upload_sessions ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS tenant_isolation ON upload_sessions;
CREATE POLICY tenant_isolation ON upload_sessions
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());

CREATE TABLE IF NOT EXISTS import_jobs (
    id TEXT PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID NOT NULL,
    state TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_import_jobs_state
    ON import_jobs(state)
    WHERE state NOT IN ('Completed', 'Failed', 'Cancelled');

CREATE INDEX IF NOT EXISTS idx_import_jobs_tenant
    ON import_jobs(tenant_id, project_id);

ALTER TABLE import_jobs ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS tenant_isolation ON import_jobs;
CREATE POLICY tenant_isolation ON import_jobs
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());

-- Admin hydrate/recovery for models and training jobs.
DROP POLICY IF EXISTS tenant_isolation ON models;
CREATE POLICY tenant_isolation ON models
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());

DROP POLICY IF EXISTS tenant_isolation ON model_versions;
CREATE POLICY tenant_isolation ON model_versions
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());

DROP POLICY IF EXISTS tenant_isolation ON training_jobs;
CREATE POLICY tenant_isolation ON training_jobs
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());
