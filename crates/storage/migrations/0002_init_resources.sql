-- R1 Task 02: core resource tables, versions, operations, leases and object references.
-- Migrations are append-only; once published they must not be modified.

-- Helper to read an admin flag used for cross-tenant management operations.
CREATE OR REPLACE FUNCTION current_admin() RETURNS BOOLEAN AS $$
BEGIN
    RETURN current_setting('app.is_admin', true)::boolean;
EXCEPTION WHEN OTHERS THEN
    RETURN FALSE;
END;
$$ LANGUAGE plpgsql;

-- Immutable dataset versions.
CREATE TABLE IF NOT EXISTS dataset_versions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    dataset_id UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    version_number BIGINT NOT NULL,
    manifest JSONB NOT NULL DEFAULT '{}',
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, dataset_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_dataset_versions_tenant_project ON dataset_versions(tenant_id, project_id);
CREATE INDEX IF NOT EXISTS idx_dataset_versions_dataset ON dataset_versions(tenant_id, dataset_id, version_number DESC);

-- Annotation projects within a dataset.
CREATE TABLE IF NOT EXISTS annotation_projects (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    dataset_id UUID NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    state TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, dataset_id, name)
);

CREATE INDEX IF NOT EXISTS idx_annotation_projects_tenant_project ON annotation_projects(tenant_id, project_id);

-- Annotation tasks belonging to an annotation project.
CREATE TABLE IF NOT EXISTS annotation_tasks (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    annotation_project_id UUID NOT NULL REFERENCES annotation_projects(id) ON DELETE CASCADE,
    dataset_version_id UUID REFERENCES dataset_versions(id) ON DELETE SET NULL,
    assignee UUID,
    state TEXT NOT NULL,
    result JSONB NOT NULL DEFAULT '{}',
    revision BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_annotation_tasks_project ON annotation_tasks(tenant_id, project_id, annotation_project_id);
CREATE INDEX IF NOT EXISTS idx_annotation_tasks_assignee ON annotation_tasks(tenant_id, assignee, state);

-- Training jobs.
CREATE TABLE IF NOT EXISTS training_jobs (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    dataset_version_id UUID REFERENCES dataset_versions(id) ON DELETE SET NULL,
    model_id UUID,
    name TEXT NOT NULL,
    spec JSONB NOT NULL DEFAULT '{}',
    state TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, project_id, name)
);

CREATE INDEX IF NOT EXISTS idx_training_jobs_tenant_project ON training_jobs(tenant_id, project_id);
CREATE INDEX IF NOT EXISTS idx_training_jobs_dataset ON training_jobs(tenant_id, dataset_version_id);
CREATE INDEX IF NOT EXISTS idx_training_jobs_state ON training_jobs(tenant_id, state);

-- Models.
CREATE TABLE IF NOT EXISTS models (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    state TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, project_id, name)
);

CREATE INDEX IF NOT EXISTS idx_models_tenant_project ON models(tenant_id, project_id);

-- Immutable model versions.
CREATE TABLE IF NOT EXISTS model_versions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    model_id UUID NOT NULL REFERENCES models(id) ON DELETE CASCADE,
    version_number BIGINT NOT NULL,
    artifact_digest TEXT,
    manifest JSONB NOT NULL DEFAULT '{}',
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, model_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_model_versions_tenant_project ON model_versions(tenant_id, project_id);
CREATE INDEX IF NOT EXISTS idx_model_versions_model ON model_versions(tenant_id, model_id, version_number DESC);

-- Applications (dyun-gu graphs).
CREATE TABLE IF NOT EXISTS applications (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    state TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    graph JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, project_id, name)
);

CREATE INDEX IF NOT EXISTS idx_applications_tenant_project ON applications(tenant_id, project_id);

-- Immutable application versions.
CREATE TABLE IF NOT EXISTS application_versions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    application_id UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    version_number BIGINT NOT NULL,
    bundle_digest TEXT,
    manifest JSONB NOT NULL DEFAULT '{}',
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, application_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_application_versions_tenant_project ON application_versions(tenant_id, project_id);
CREATE INDEX IF NOT EXISTS idx_application_versions_app ON application_versions(tenant_id, application_id, version_number DESC);

-- Deployments of application versions.
CREATE TABLE IF NOT EXISTS deployments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    application_version_id UUID NOT NULL REFERENCES application_versions(id) ON DELETE CASCADE,
    target_cluster_id UUID,
    name TEXT NOT NULL,
    state TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    spec JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, project_id, name)
);

CREATE INDEX IF NOT EXISTS idx_deployments_tenant_project ON deployments(tenant_id, project_id);
CREATE INDEX IF NOT EXISTS idx_deployments_app_version ON deployments(tenant_id, application_version_id);
CREATE INDEX IF NOT EXISTS idx_deployments_state ON deployments(tenant_id, state);

-- Long-running operations.
CREATE TABLE IF NOT EXISTS operations (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    operation_type TEXT NOT NULL,
    status TEXT NOT NULL,
    progress DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    error JSONB,
    deadline TIMESTAMPTZ,
    cancelled BOOLEAN NOT NULL DEFAULT FALSE,
    retry_count INT NOT NULL DEFAULT 0,
    event_sequence BIGINT NOT NULL DEFAULT 0,
    sse_cursor TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_operations_tenant_project ON operations(tenant_id, project_id);
CREATE INDEX IF NOT EXISTS idx_operations_status ON operations(tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_operations_cursor ON operations(tenant_id, sse_cursor);

-- Operation event stream (ordered, persistent log).
CREATE TABLE IF NOT EXISTS operation_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    operation_id UUID NOT NULL REFERENCES operations(id) ON DELETE CASCADE,
    sequence BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, operation_id, sequence)
);

CREATE INDEX IF NOT EXISTS idx_operation_events_operation ON operation_events(tenant_id, operation_id, sequence);

-- Distributed leases for workers and reconcilers.
CREATE TABLE IF NOT EXISTS leases (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    owner TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, resource_type, resource_id)
);

CREATE INDEX IF NOT EXISTS idx_leases_expires ON leases(tenant_id, expires_at);
CREATE INDEX IF NOT EXISTS idx_leases_owner ON leases(owner);

-- Object references for artifacts stored in object storage.
CREATE TABLE IF NOT EXISTS object_references (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    object_type TEXT NOT NULL,
    storage_backend TEXT NOT NULL,
    bucket TEXT NOT NULL,
    key TEXT NOT NULL,
    etag TEXT,
    size_bytes BIGINT,
    digest TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_object_refs_tenant_project ON object_references(tenant_id, project_id);
CREATE INDEX IF NOT EXISTS idx_object_refs_digest ON object_references(tenant_id, digest);

-- Generic resource state history for audit and rollback support.
CREATE TABLE IF NOT EXISTS resource_state_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    revision BIGINT NOT NULL,
    state TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    actor TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_state_history_aggregate ON resource_state_history(tenant_id, aggregate_type, aggregate_id, revision DESC);
CREATE INDEX IF NOT EXISTS idx_state_history_time ON resource_state_history(tenant_id, occurred_at DESC);

-- Row-level security: enable on all new tables.
ALTER TABLE dataset_versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE annotation_projects ENABLE ROW LEVEL SECURITY;
ALTER TABLE annotation_tasks ENABLE ROW LEVEL SECURITY;
ALTER TABLE training_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE models ENABLE ROW LEVEL SECURITY;
ALTER TABLE model_versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE applications ENABLE ROW LEVEL SECURITY;
ALTER TABLE application_versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE deployments ENABLE ROW LEVEL SECURITY;
ALTER TABLE operations ENABLE ROW LEVEL SECURITY;
ALTER TABLE operation_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE leases ENABLE ROW LEVEL SECURITY;
ALTER TABLE object_references ENABLE ROW LEVEL SECURITY;
ALTER TABLE resource_state_history ENABLE ROW LEVEL SECURITY;

-- Tenant-matching helper used by all RLS policies.
CREATE OR REPLACE FUNCTION tenant_matches(tenant_id UUID) RETURNS BOOLEAN AS $$
BEGIN
    RETURN tenant_id::text = current_tenant_id();
END;
$$ LANGUAGE plpgsql;

-- Fail-closed tenant isolation for all new resource tables.
CREATE POLICY tenant_isolation ON dataset_versions FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON annotation_projects FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON annotation_tasks FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON training_jobs FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON models FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON model_versions FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON applications FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON application_versions FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON deployments FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON operations FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON operation_events FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON leases FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON object_references FOR ALL USING (tenant_matches(tenant_id));
CREATE POLICY tenant_isolation ON resource_state_history FOR ALL USING (tenant_matches(tenant_id));

-- RLS fix: tenants table should be fail-closed. Drop the permissive NULL fallback from
-- the initial migration and replace it with a tenant-scoped policy plus an admin bypass.
DROP POLICY IF EXISTS tenant_isolation ON tenants;
CREATE POLICY tenant_isolation ON tenants FOR ALL USING (
    id::text = current_tenant_id() OR current_admin()
);
