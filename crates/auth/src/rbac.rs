//! Role-based access control and tenant isolation.

use moqentra_types::{ProjectId, RequestContext, TenantId, UserId};
use std::collections::HashSet;

/// Core roles in the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Viewer,
    Labeler,
    Reviewer,
    MlEngineer,
    Operator,
    ProjectAdmin,
    TenantAdmin,
    SystemAdmin,
}

impl std::str::FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "viewer" => Ok(Self::Viewer),
            "labeler" => Ok(Self::Labeler),
            "reviewer" => Ok(Self::Reviewer),
            "ml_engineer" => Ok(Self::MlEngineer),
            "operator" => Ok(Self::Operator),
            "project_admin" => Ok(Self::ProjectAdmin),
            "tenant_admin" => Ok(Self::TenantAdmin),
            "system_admin" => Ok(Self::SystemAdmin),
            _ => Err(()),
        }
    }
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Viewer => "viewer",
            Role::Labeler => "labeler",
            Role::Reviewer => "reviewer",
            Role::MlEngineer => "ml_engineer",
            Role::Operator => "operator",
            Role::ProjectAdmin => "project_admin",
            Role::TenantAdmin => "tenant_admin",
            Role::SystemAdmin => "system_admin",
        }
    }
}

/// Action that can be performed on a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Read,
    List,
    Create,
    Update,
    Delete,
    Execute,
    Export,
    Admin,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Read => "read",
            Action::List => "list",
            Action::Create => "create",
            Action::Update => "update",
            Action::Delete => "delete",
            Action::Execute => "execute",
            Action::Export => "export",
            Action::Admin => "admin",
        }
    }
}

/// Resource type being accessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    Tenant,
    Project,
    Dataset,
    DatasetVersion,
    AnnotationTask,
    TrainingJob,
    ModelVersion,
    ApplicationVersion,
    Deployment,
    Node,
    Secret,
    AuditLog,
}

/// A permission is a resource/action pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Permission {
    pub resource: Resource,
    pub action: Action,
}

impl Permission {
    pub const fn new(resource: Resource, action: Action) -> Self {
        Self { resource, action }
    }
}

/// Scope of an authorization request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub tenant_id: TenantId,
    pub project_id: Option<ProjectId>,
}

impl Scope {
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            project_id: None,
        }
    }

    pub fn with_project(mut self, project_id: ProjectId) -> Self {
        self.project_id = Some(project_id);
        self
    }
}

/// Authorization decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

/// Authorization error.
#[derive(Debug, thiserror::Error)]
#[error("Permission denied: {} on {resource:?} in tenant {tenant}", action.as_str())]
pub struct AuthorizationError {
    pub action: Action,
    pub resource: Resource,
    pub tenant: TenantId,
}

/// Returns the set of permissions granted to a role.
fn role_permissions(role: Role) -> HashSet<Permission> {
    let mut perms = HashSet::new();
    match role {
        Role::Viewer => {
            perms.insert(Permission::new(Resource::Project, Action::Read));
            perms.insert(Permission::new(Resource::Dataset, Action::Read));
            perms.insert(Permission::new(Resource::Dataset, Action::List));
            perms.insert(Permission::new(Resource::DatasetVersion, Action::Read));
            perms.insert(Permission::new(Resource::ModelVersion, Action::Read));
            perms.insert(Permission::new(Resource::ApplicationVersion, Action::Read));
            perms.insert(Permission::new(Resource::Deployment, Action::Read));
        }
        Role::Labeler => {
            perms.extend(role_permissions(Role::Viewer));
            perms.insert(Permission::new(Resource::AnnotationTask, Action::Read));
            perms.insert(Permission::new(Resource::AnnotationTask, Action::Update));
        }
        Role::Reviewer => {
            perms.extend(role_permissions(Role::Labeler));
            perms.insert(Permission::new(Resource::AnnotationTask, Action::Execute));
        }
        Role::MlEngineer => {
            perms.extend(role_permissions(Role::Reviewer));
            perms.insert(Permission::new(Resource::Dataset, Action::Create));
            perms.insert(Permission::new(Resource::DatasetVersion, Action::Create));
            perms.insert(Permission::new(Resource::DatasetVersion, Action::Update));
            perms.insert(Permission::new(Resource::AnnotationTask, Action::Create));
            perms.insert(Permission::new(Resource::AnnotationTask, Action::List));
            perms.insert(Permission::new(Resource::TrainingJob, Action::Create));
            perms.insert(Permission::new(Resource::TrainingJob, Action::Read));
            perms.insert(Permission::new(Resource::TrainingJob, Action::List));
            perms.insert(Permission::new(Resource::TrainingJob, Action::Execute));
            perms.insert(Permission::new(Resource::ModelVersion, Action::Create));
            perms.insert(Permission::new(Resource::ModelVersion, Action::List));
            perms.insert(Permission::new(Resource::ModelVersion, Action::Update));
            perms.insert(Permission::new(Resource::AuditLog, Action::Read));
        }
        Role::Operator => {
            perms.extend(role_permissions(Role::Viewer));
            perms.insert(Permission::new(
                Resource::ApplicationVersion,
                Action::Create,
            ));
            perms.insert(Permission::new(Resource::Deployment, Action::Create));
            perms.insert(Permission::new(Resource::Deployment, Action::Update));
            perms.insert(Permission::new(Resource::Deployment, Action::Execute));
            perms.insert(Permission::new(Resource::Node, Action::Read));
        }
        Role::ProjectAdmin => {
            perms.extend(role_permissions(Role::MlEngineer));
            perms.extend(role_permissions(Role::Operator));
            perms.insert(Permission::new(Resource::Project, Action::Update));
            perms.insert(Permission::new(Resource::Project, Action::Delete));
            perms.insert(Permission::new(Resource::Secret, Action::Read));
            perms.insert(Permission::new(Resource::Secret, Action::Update));
        }
        Role::TenantAdmin => {
            perms.extend(role_permissions(Role::ProjectAdmin));
            perms.insert(Permission::new(Resource::Tenant, Action::Admin));
            perms.insert(Permission::new(Resource::AuditLog, Action::Read));
            perms.insert(Permission::new(Resource::AuditLog, Action::List));
        }
        Role::SystemAdmin => {
            for r in [
                Resource::Tenant,
                Resource::Project,
                Resource::Dataset,
                Resource::DatasetVersion,
                Resource::AnnotationTask,
                Resource::TrainingJob,
                Resource::ModelVersion,
                Resource::ApplicationVersion,
                Resource::Deployment,
                Resource::Node,
                Resource::Secret,
                Resource::AuditLog,
            ] {
                for a in [
                    Action::Read,
                    Action::List,
                    Action::Create,
                    Action::Update,
                    Action::Delete,
                    Action::Execute,
                    Action::Export,
                    Action::Admin,
                ] {
                    perms.insert(Permission::new(r, a));
                }
            }
        }
    }
    perms
}

/// Authorizer with deny-by-default policy.
#[derive(Debug, Clone)]
pub struct Authorizer {
    role_assignments: HashSet<(UserId, TenantId, Role)>,
    /// Service-account name → (tenant, role) grants.
    service_assignments: HashSet<(String, TenantId, Role)>,
    project_members: HashSet<(UserId, TenantId, ProjectId)>,
}

impl Default for Authorizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Authorizer {
    pub fn new() -> Self {
        Self {
            role_assignments: HashSet::new(),
            service_assignments: HashSet::new(),
            project_members: HashSet::new(),
        }
    }

    pub fn assign_role(&mut self, user: UserId, tenant: TenantId, role: Role) {
        self.role_assignments.insert((user, tenant, role));
    }

    /// Grant a role to a service account within a tenant (workload identity).
    pub fn assign_service_role(
        &mut self,
        service_name: impl Into<String>,
        tenant: TenantId,
        role: Role,
    ) {
        self.service_assignments.insert((service_name.into(), tenant, role));
    }

    pub fn add_project_member(&mut self, user: UserId, tenant: TenantId, project: ProjectId) {
        self.project_members.insert((user, tenant, project));
    }

    fn roles_for_principal(
        &self,
        principal: &moqentra_types::Principal,
        tenant: TenantId,
    ) -> Vec<Role> {
        match principal {
            moqentra_types::Principal::User { id } => self
                .role_assignments
                .iter()
                .filter(|(u, t, _)| *u == *id && *t == tenant)
                .map(|(_, _, r)| *r)
                .collect(),
            moqentra_types::Principal::Service { name } => self
                .service_assignments
                .iter()
                .filter(|(n, t, _)| n == name && *t == tenant)
                .map(|(_, _, r)| *r)
                .collect(),
            moqentra_types::Principal::Anonymous => Vec::new(),
        }
    }

    /// Returns `Ok(())` if the caller is allowed to perform `action` on `resource`
    /// within `scope`. Denies if there is no tenant context, the principal has no
    /// role, or the role does not grant the required permission. Project-scoped
    /// resources require project membership for user principals (service
    /// principals with a matching role skip membership checks — they act as
    /// workload identities).
    pub fn authorize(
        &self,
        ctx: &RequestContext,
        action: Action,
        resource: Resource,
        scope: &Scope,
    ) -> Result<(), AuthorizationError> {
        let deny = || AuthorizationError {
            action,
            resource,
            tenant: scope.tenant_id,
        };

        if matches!(ctx.principal, moqentra_types::Principal::Anonymous) {
            return Err(deny());
        }

        // Tenant isolation: the scope must match the request context.
        if ctx.tenant_id != scope.tenant_id {
            return Err(deny());
        }

        let roles = self.roles_for_principal(&ctx.principal, scope.tenant_id);
        let allowed = roles
            .iter()
            .any(|role| role_permissions(*role).contains(&Permission::new(resource, action)));
        if !allowed {
            return Err(deny());
        }

        // Project-level resources further require project membership unless the
        // principal is a tenant/system admin or a service account.
        if let Some(project_id) = scope.project_id {
            match &ctx.principal {
                moqentra_types::Principal::Service { .. } => {
                    // Workload identities are not project members; role is enough.
                }
                moqentra_types::Principal::User { id } => {
                    let is_admin =
                        roles.iter().any(|r| matches!(r, Role::TenantAdmin | Role::SystemAdmin));
                    if !is_admin
                        && !self.project_members.contains(&(*id, scope.tenant_id, project_id))
                    {
                        return Err(deny());
                    }
                }
                moqentra_types::Principal::Anonymous => return Err(deny()),
            }
        }

        Ok(())
    }

    /// Authorizes a system admin to act across tenants. This still requires an
    /// audit record (auditing is the caller's responsibility).
    pub fn authorize_system_admin(
        &self,
        ctx: &RequestContext,
        action: Action,
        resource: Resource,
        target_tenant: TenantId,
    ) -> Result<(), AuthorizationError> {
        let user = match &ctx.principal {
            moqentra_types::Principal::User { id } => *id,
            _ => {
                return Err(AuthorizationError {
                    action,
                    resource,
                    tenant: target_tenant,
                })
            }
        };

        let has_system_admin = self
            .role_assignments
            .iter()
            .any(|(u, _, r)| *u == user && matches!(r, Role::SystemAdmin));

        if !has_system_admin {
            return Err(AuthorizationError {
                action,
                resource,
                tenant: target_tenant,
            });
        }

        if !role_permissions(Role::SystemAdmin).contains(&Permission::new(resource, action)) {
            return Err(AuthorizationError {
                action,
                resource,
                tenant: target_tenant,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{Principal, RandomIdGenerator, RequestContext};

    fn setup() -> (Authorizer, UserId, TenantId, ProjectId) {
        let mut authz = Authorizer::new();
        let gen = RandomIdGenerator;
        let user = UserId::new_v7(&gen);
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        authz.assign_role(user, tenant, Role::MlEngineer);
        authz.add_project_member(user, tenant, project);
        (authz, user, tenant, project)
    }

    fn ctx(tenant: TenantId, user: UserId) -> RequestContext {
        RequestContext::new(tenant, Principal::user(user), "req-1")
    }

    #[test]
    fn allowed_for_role() {
        let (authz, user, tenant, project) = setup();
        let scope = Scope::new(tenant).with_project(project);
        authz
            .authorize(
                &ctx(tenant, user),
                Action::Create,
                Resource::TrainingJob,
                &scope,
            )
            .unwrap();
    }

    #[test]
    fn denied_without_tenant() {
        let (authz, user, tenant, project) = setup();
        let other_tenant = TenantId::new_v7(&RandomIdGenerator);
        let scope = Scope::new(other_tenant).with_project(project);
        assert!(authz
            .authorize(
                &ctx(tenant, user),
                Action::Create,
                Resource::TrainingJob,
                &scope
            )
            .is_err());
    }

    #[test]
    fn denied_by_default() {
        let (authz, user, tenant, _project) = setup();
        let scope = Scope::new(tenant);
        // ML engineer does not have Tenant admin.
        assert!(authz
            .authorize(&ctx(tenant, user), Action::Admin, Resource::Tenant, &scope)
            .is_err());
    }

    #[test]
    fn project_member_required() {
        let mut authz = Authorizer::new();
        let gen = RandomIdGenerator;
        let user = UserId::new_v7(&gen);
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        authz.assign_role(user, tenant, Role::MlEngineer);
        // Not a project member.
        let scope = Scope::new(tenant).with_project(project);
        assert!(authz
            .authorize(
                &ctx(tenant, user),
                Action::Create,
                Resource::TrainingJob,
                &scope
            )
            .is_err());
    }

    #[test]
    fn system_admin_cross_tenant_allowed() {
        let mut authz = Authorizer::new();
        let gen = RandomIdGenerator;
        let user = UserId::new_v7(&gen);
        let tenant = TenantId::new_v7(&gen);
        authz.assign_role(user, tenant, Role::SystemAdmin);
        let other_tenant = TenantId::new_v7(&gen);
        authz
            .authorize_system_admin(
                &ctx(tenant, user),
                Action::Read,
                Resource::Tenant,
                other_tenant,
            )
            .unwrap();
    }

    #[test]
    fn service_account_authorized_with_role() {
        let mut authz = Authorizer::new();
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        authz.assign_service_role("node-agent", tenant, Role::Operator);
        let ctx = RequestContext::new(tenant, Principal::service("node-agent"), "req-svc");
        let scope = Scope::new(tenant).with_project(project);
        authz.authorize(&ctx, Action::Read, Resource::Node, &scope).unwrap();
        // No grant → deny
        assert!(authz
            .authorize(
                &RequestContext::new(tenant, Principal::service("other"), "r2"),
                Action::Read,
                Resource::Node,
                &scope
            )
            .is_err());
    }
}
