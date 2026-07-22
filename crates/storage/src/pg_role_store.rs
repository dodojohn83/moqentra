//! PostgreSQL-backed role resolution.

use moqentra_auth::rbac::Role;
use moqentra_auth::RoleStore;
use moqentra_types::{Error, ProjectId, TenantId, UserId};
use sqlx::PgPool;

/// Resolves roles from the `project_members` table under row-level security.
#[derive(Debug, Clone)]
pub struct PgRoleStore {
    pool: PgPool,
}

impl PgRoleStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn roles_for(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        project_id: Option<ProjectId>,
    ) -> Result<Vec<Role>, Error> {
        let tenant_str = tenant_id.to_string();

        let rows: Vec<(String,)> = if let Some(pid) = project_id {
            sqlx::query_as(
                "SELECT set_config('app.current_tenant', $1, true); \
                 SELECT role FROM project_members \
                 WHERE tenant_id = $2 AND project_id = $3 AND user_id = $4",
            )
            .bind(&tenant_str)
            .bind(tenant_id.as_uuid())
            .bind(pid.as_uuid())
            .bind(user_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to query project roles: {e}")))?
        } else {
            sqlx::query_as(
                "SELECT set_config('app.current_tenant', $1, true); \
                 SELECT DISTINCT role FROM project_members \
                 WHERE tenant_id = $2 AND user_id = $3",
            )
            .bind(&tenant_str)
            .bind(tenant_id.as_uuid())
            .bind(user_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to query tenant roles: {e}")))?
        };

        let mut roles = Vec::new();
        for (role_str,) in rows {
            if let Ok(role) = role_str.parse::<Role>() {
                roles.push(role);
            }
        }
        Ok(roles)
    }
}

#[async_trait::async_trait]
impl RoleStore for PgRoleStore {
    async fn tenant_roles(&self, user_id: UserId, tenant_id: TenantId) -> Result<Vec<Role>, Error> {
        self.roles_for(user_id, tenant_id, None).await
    }

    async fn project_roles(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        project_id: ProjectId,
    ) -> Result<Vec<Role>, Error> {
        self.roles_for(user_id, tenant_id, Some(project_id)).await
    }
}
