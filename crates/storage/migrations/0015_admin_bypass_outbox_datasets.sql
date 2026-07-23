-- Allow multi-tenant outbox dispatch and control-plane recovery under app.is_admin.
-- Append-only migration; does not alter column shapes.

DROP POLICY IF EXISTS tenant_isolation ON outbox_events;
CREATE POLICY tenant_isolation ON outbox_events
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());

DROP POLICY IF EXISTS tenant_isolation ON datasets;
CREATE POLICY tenant_isolation ON datasets
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());

DROP POLICY IF EXISTS tenant_isolation ON dataset_versions;
CREATE POLICY tenant_isolation ON dataset_versions
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());
