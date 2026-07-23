-- R2 contract tables: quota, approval, queue, resource class, checkpoint manifest,
-- agent sessions, HA leases, conversion profiles, and audit partitioning.

CREATE TABLE IF NOT EXISTS quota_policies (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID,
    scope TEXT NOT NULL,
    revision BIGINT NOT NULL,
    effective_from TIMESTAMPTZ NOT NULL,
    effective_until TIMESTAMPTZ,
    limits JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_quota_policies_tenant_project
    ON quota_policies (tenant_id, project_id, effective_from DESC);

CREATE TABLE IF NOT EXISTS quota_reservations (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID NOT NULL,
    policy_id UUID NOT NULL,
    policy_revision BIGINT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_dimension TEXT NOT NULL,
    requested BIGINT NOT NULL,
    state TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_quota_reservations_tenant_state
    ON quota_reservations (tenant_id, state, expires_at);

CREATE TABLE IF NOT EXISTS usage_ledger (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID NOT NULL,
    reservation_id UUID NOT NULL,
    source_event_id TEXT NOT NULL,
    resource_dimension TEXT NOT NULL,
    amount BIGINT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_usage_ledger_tenant_recorded
    ON usage_ledger (tenant_id, project_id, resource_dimension, recorded_at DESC);

CREATE TABLE IF NOT EXISTS usage_rollups (
    tenant_id UUID NOT NULL,
    project_id UUID,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    resource_dimension TEXT NOT NULL,
    total_amount BIGINT NOT NULL,
    entry_count BIGINT NOT NULL,
    PRIMARY KEY (tenant_id, project_id, resource_dimension, period_start)
);

CREATE TABLE IF NOT EXISTS approval_requests (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID NOT NULL,
    requester_id UUID NOT NULL,
    kind TEXT NOT NULL,
    reason TEXT NOT NULL,
    requested_limits JSONB NOT NULL DEFAULT '{}'::jsonb,
    policy_revision BIGINT NOT NULL,
    state TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_at TIMESTAMPTZ,
    decision JSONB
);

CREATE INDEX IF NOT EXISTS idx_approval_requests_tenant_state
    ON approval_requests (tenant_id, state, created_at DESC);

CREATE TABLE IF NOT EXISTS approval_decisions (
    id UUID PRIMARY KEY,
    request_id UUID NOT NULL,
    approver_id UUID NOT NULL,
    outcome TEXT NOT NULL,
    reason TEXT NOT NULL,
    valid_until TIMESTAMPTZ,
    limit_values JSONB NOT NULL DEFAULT '{}'::jsonb,
    decision_revision BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_approval_decisions_request
    ON approval_decisions (request_id, decision_revision DESC);

CREATE TABLE IF NOT EXISTS queues (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID,
    name TEXT NOT NULL,
    weight INTEGER NOT NULL,
    capacity INTEGER NOT NULL,
    max_running INTEGER NOT NULL,
    revision BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_queues_tenant_name
    ON queues (tenant_id, name);

CREATE TABLE IF NOT EXISTS priority_classes (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID,
    name TEXT NOT NULL,
    priority INTEGER NOT NULL,
    preemptible BOOLEAN NOT NULL,
    revision BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_priority_classes_tenant_name
    ON priority_classes (tenant_id, name);

CREATE TABLE IF NOT EXISTS resource_classes (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    vendor TEXT NOT NULL,
    family TEXT NOT NULL,
    memory_mib BIGINT NOT NULL,
    driver_version TEXT NOT NULL,
    runtime TEXT NOT NULL,
    collective_backend TEXT NOT NULL,
    topology TEXT NOT NULL,
    sharing_mode TEXT NOT NULL,
    support_tier TEXT NOT NULL,
    evidence_date TIMESTAMPTZ,
    revision BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_resource_classes_support_tier
    ON resource_classes (support_tier, vendor, family);

CREATE TABLE IF NOT EXISTS cluster_resources (
    cluster_id UUID NOT NULL,
    resource_class_id UUID NOT NULL,
    node_name TEXT NOT NULL,
    total INTEGER NOT NULL,
    available INTEGER NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (cluster_id, resource_class_id, node_name)
);

CREATE TABLE IF NOT EXISTS workload_bindings (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID NOT NULL,
    workload_id UUID NOT NULL,
    workload_kind TEXT NOT NULL,
    resource_class_id UUID NOT NULL,
    node_name TEXT,
    rank_index INTEGER,
    state TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_workload_bindings_workload
    ON workload_bindings (tenant_id, workload_id, workload_kind);

CREATE TABLE IF NOT EXISTS ranks (
    id UUID PRIMARY KEY,
    attempt_id UUID NOT NULL,
    rank_index INTEGER NOT NULL,
    world_size INTEGER NOT NULL,
    node_id UUID,
    state TEXT NOT NULL,
    last_heartbeat TIMESTAMPTZ,
    exit_code INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_ranks_attempt
    ON ranks (attempt_id, rank_index);

CREATE TABLE IF NOT EXISTS rendezvous_members (
    id UUID PRIMARY KEY,
    rendezvous_id UUID NOT NULL,
    rank_index INTEGER NOT NULL,
    world_size INTEGER NOT NULL,
    endpoint TEXT NOT NULL,
    joined_at TIMESTAMPTZ,
    state TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (rendezvous_id, rank_index)
);

CREATE INDEX IF NOT EXISTS idx_rendezvous_members_rendezvous
    ON rendezvous_members (rendezvous_id, state);

CREATE TABLE IF NOT EXISTS checkpoints (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID NOT NULL,
    training_job_id UUID NOT NULL,
    attempt_id TEXT NOT NULL,
    step BIGINT NOT NULL,
    epoch BIGINT NOT NULL,
    world_size INTEGER NOT NULL,
    framework TEXT NOT NULL,
    template TEXT NOT NULL,
    code_digest TEXT NOT NULL,
    image_digest TEXT NOT NULL,
    dataset_version_id UUID NOT NULL,
    model_id UUID,
    shards JSONB NOT NULL DEFAULT '[]'::jsonb,
    compatibility JSONB NOT NULL DEFAULT '{}'::jsonb,
    state TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_job_step
    ON checkpoints (tenant_id, training_job_id, step DESC);

CREATE TABLE IF NOT EXISTS checkpoint_shards (
    checkpoint_id UUID NOT NULL,
    rank_index INTEGER NOT NULL,
    object_key TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    digest TEXT NOT NULL,
    tensor_layout JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (checkpoint_id, rank_index)
);

CREATE TABLE IF NOT EXISTS checkpoint_holds (
    manifest_id UUID NOT NULL,
    holder TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    PRIMARY KEY (manifest_id, holder)
);

CREATE TABLE IF NOT EXISTS agent_sessions (
    id UUID PRIMARY KEY,
    cluster_id UUID,
    node_name TEXT NOT NULL,
    tenant_id UUID NOT NULL,
    agent_version TEXT NOT NULL,
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_agent_sessions_last_seen
    ON agent_sessions (last_seen_at, state);

ALTER TABLE IF EXISTS agent_sessions ADD COLUMN IF NOT EXISTS state TEXT NOT NULL DEFAULT 'active';

CREATE TABLE IF NOT EXISTS agent_commands (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL,
    command_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    state TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    delivered_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_agent_commands_session_state
    ON agent_commands (session_id, state, created_at);

CREATE TABLE IF NOT EXISTS command_acks (
    command_id UUID PRIMARY KEY,
    agent_session_id UUID NOT NULL,
    outcome TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS scheduler_leases (
    subsystem TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS reconciler_cursors (
    subsystem TEXT PRIMARY KEY,
    resource_version TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS conversion_profiles (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID,
    name TEXT NOT NULL,
    target TEXT NOT NULL,
    sdk_version TEXT NOT NULL,
    toolchain_image_digest TEXT NOT NULL,
    target_chip TEXT NOT NULL,
    precision TEXT NOT NULL,
    dynamic_shapes BOOLEAN NOT NULL,
    capabilities JSONB NOT NULL DEFAULT '[]'::jsonb,
    postprocess JSONB,
    parameter_schema JSONB NOT NULL DEFAULT '{}'::jsonb,
    support_tier TEXT NOT NULL,
    revision BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_conversion_profiles_tenant_name
    ON conversion_profiles (tenant_id, name, revision DESC);

CREATE TABLE IF NOT EXISTS conversion_reports (
    id UUID PRIMARY KEY,
    conversion_job_id UUID NOT NULL,
    target TEXT NOT NULL,
    metrics JSONB NOT NULL DEFAULT '{}'::jsonb,
    artifacts JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_conversion_reports_job
    ON conversion_reports (conversion_job_id);

CREATE TABLE IF NOT EXISTS model_promotion_requests (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    project_id UUID NOT NULL,
    model_version_id UUID NOT NULL,
    requested_by UUID NOT NULL,
    state TEXT NOT NULL,
    reason TEXT NOT NULL,
    evaluation_report_id UUID,
    approval_request_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_model_promotion_requests_tenant_state
    ON model_promotion_requests (tenant_id, state, created_at DESC);

-- Replace audit_log with a partitioned table. Data is preserved by renaming the
-- old table; new partitions will be populated going forward. The old table can
-- be migrated offline.
ALTER TABLE IF EXISTS audit_log RENAME TO audit_log_pre_r2;

CREATE TABLE audit_log (
    id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    actor_id TEXT NOT NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    payload JSONB,
    PRIMARY KEY (id, recorded_at)
) PARTITION BY RANGE (recorded_at);

CREATE TABLE audit_log_default PARTITION OF audit_log DEFAULT;

CREATE TABLE audit_log_2026_07 PARTITION OF audit_log
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');

CREATE TABLE audit_log_2026_08 PARTITION OF audit_log
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
