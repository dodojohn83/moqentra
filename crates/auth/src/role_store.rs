//! Port for resolving a principal's tenant/project roles from persistent membership.

use crate::rbac::Role;
use async_trait::async_trait;
use moqentra_types::{Error, ProjectId, TenantId, UserId};

/// Resolves tenant- and project-scoped roles from a membership store.
#[async_trait]
pub trait RoleStore: Send + Sync {
    /// Returns the distinct tenant-level roles for a user.
    async fn tenant_roles(&self, user_id: UserId, tenant_id: TenantId) -> Result<Vec<Role>, Error>;

    /// Returns the project-level roles for a user within a tenant/project.
    async fn project_roles(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        project_id: ProjectId,
    ) -> Result<Vec<Role>, Error>;
}
