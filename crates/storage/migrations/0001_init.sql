-- Initial PostgreSQL schema for Moqentra.
-- Migrations are append-only; once published they must not be modified.

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_projects_tenant_id ON projects(tenant_id);

CREATE TABLE IF NOT EXISTS project_members (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    role TEXT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, project_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_project_members_user ON project_members(tenant_id, user_id);

CREATE TABLE IF NOT EXISTS datasets (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    state TEXT NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_datasets_tenant_project ON datasets(tenant_id, project_id);
CREATE INDEX IF NOT EXISTS idx_datasets_state ON datasets(tenant_id, state);

CREATE TABLE IF NOT EXISTS outbox_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    retry_count INT NOT NULL DEFAULT 0,
    failure_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_outbox_status_created ON outbox_events(status, created_at);
CREATE INDEX IF NOT EXISTS idx_outbox_tenant ON outbox_events(tenant_id);

CREATE TABLE IF NOT EXISTS idempotency_keys (
    tenant_id UUID NOT NULL,
    operation_type TEXT NOT NULL,
    key TEXT NOT NULL,
    response JSONB,
    status TEXT NOT NULL DEFAULT 'in_progress',
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, operation_type, key)
);

CREATE INDEX IF NOT EXISTS idx_idempotency_expires ON idempotency_keys(expires_at);

CREATE TABLE IF NOT EXISTS processed_messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    message_id TEXT NOT NULL,
    handler TEXT NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, message_id, handler)
);

CREATE INDEX IF NOT EXISTS idx_processed_messages_lookup ON processed_messages(tenant_id, message_id, handler);

CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    project_id UUID,
    actor TEXT NOT NULL,
    category TEXT NOT NULL,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    outcome TEXT NOT NULL,
    reason TEXT,
    correlation_id TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_audit_tenant_time ON audit_logs(tenant_id, occurred_at DESC);

-- Row-level security policies for tenant isolation.
ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
ALTER TABLE projects ENABLE ROW LEVEL SECURITY;
ALTER TABLE project_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE datasets ENABLE ROW LEVEL SECURITY;
ALTER TABLE outbox_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE idempotency_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE processed_messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;

CREATE OR REPLACE FUNCTION current_tenant_id() RETURNS TEXT AS $$
BEGIN
    RETURN current_setting('app.current_tenant', true);
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE POLICY tenant_isolation ON tenants
    FOR ALL
    USING (id::text = current_tenant_id() OR current_tenant_id() IS NULL);

CREATE POLICY tenant_isolation ON projects
    FOR ALL
    USING (tenant_id::text = current_tenant_id());

CREATE POLICY tenant_isolation ON project_members
    FOR ALL
    USING (tenant_id::text = current_tenant_id());

CREATE POLICY tenant_isolation ON datasets
    FOR ALL
    USING (tenant_id::text = current_tenant_id());

CREATE POLICY tenant_isolation ON outbox_events
    FOR ALL
    USING (tenant_id::text = current_tenant_id());

CREATE POLICY tenant_isolation ON idempotency_keys
    FOR ALL
    USING (tenant_id::text = current_tenant_id());

CREATE POLICY tenant_isolation ON processed_messages
    FOR ALL
    USING (tenant_id::text = current_tenant_id());

CREATE POLICY tenant_isolation ON audit_logs
    FOR ALL
    USING (tenant_id::text = current_tenant_id());
