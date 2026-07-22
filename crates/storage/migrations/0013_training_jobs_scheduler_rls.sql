-- Allow scheduler/admin role to read all training_jobs across tenants.
DROP POLICY IF EXISTS tenant_isolation ON training_jobs;
CREATE POLICY tenant_isolation ON training_jobs
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());
