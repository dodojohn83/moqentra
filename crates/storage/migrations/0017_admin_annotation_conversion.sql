-- Admin hydrate for annotation / conversion / evaluation tables.

DROP POLICY IF EXISTS tenant_isolation ON annotation_projects;
CREATE POLICY tenant_isolation ON annotation_projects
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());

DROP POLICY IF EXISTS tenant_isolation ON annotation_tasks;
CREATE POLICY tenant_isolation ON annotation_tasks
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());

DROP POLICY IF EXISTS tenant_isolation_conversion_jobs ON conversion_jobs;
CREATE POLICY tenant_isolation_conversion_jobs ON conversion_jobs
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());

DROP POLICY IF EXISTS tenant_isolation_evaluation_runs ON evaluation_runs;
CREATE POLICY tenant_isolation_evaluation_runs ON evaluation_runs
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());
