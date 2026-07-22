-- R1 Task 02: allow administrative cleanup across tenants for idempotency keys.
-- Migrations are append-only; once published they must not be modified.

DROP POLICY IF EXISTS tenant_isolation ON idempotency_keys;
CREATE POLICY tenant_isolation ON idempotency_keys
    FOR ALL
    USING (tenant_matches(tenant_id) OR current_admin());
