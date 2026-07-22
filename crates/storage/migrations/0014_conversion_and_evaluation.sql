-- Conversion jobs and evaluation runs for model registry.

CREATE TABLE IF NOT EXISTS conversion_jobs (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    project_id UUID NOT NULL,
    source_model_version_id UUID NOT NULL,
    target TEXT NOT NULL,
    state TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    profile JSONB NOT NULL,
    parameters JSONB NOT NULL,
    output_artifacts JSONB NOT NULL DEFAULT '[]'::jsonb,
    cache_key TEXT NOT NULL,
    log_digest TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS evaluation_runs (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    project_id UUID NOT NULL,
    model_version_id UUID NOT NULL,
    dataset_version_id UUID NOT NULL,
    seed BIGINT NOT NULL,
    state TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    metrics JSONB NOT NULL DEFAULT '[]'::jsonb,
    hardware_profile TEXT NOT NULL,
    preprocess_version TEXT NOT NULL,
    postprocess_version TEXT NOT NULL,
    reference_outputs JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_conversion_jobs_tenant_project ON conversion_jobs (tenant_id, project_id, updated_at DESC, id);
CREATE INDEX IF NOT EXISTS idx_conversion_jobs_source ON conversion_jobs (source_model_version_id);
CREATE INDEX IF NOT EXISTS idx_evaluation_runs_tenant_project ON evaluation_runs (tenant_id, project_id, updated_at DESC, id);
CREATE INDEX IF NOT EXISTS idx_evaluation_runs_model ON evaluation_runs (model_version_id);

ALTER TABLE conversion_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE evaluation_runs ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_conversion_jobs ON conversion_jobs
    USING (tenant_matches(tenant_id));

CREATE POLICY tenant_isolation_evaluation_runs ON evaluation_runs
    USING (tenant_matches(tenant_id));
