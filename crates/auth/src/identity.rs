//! SPIFFE-like service identity and mTLS command scope verification.
//!
//! Every control-plane, scheduler, Node Agent, Worker and dyun-agent workload
//! receives a short-lived service identity.  The identity carries a URI SAN
//! (`spiffe://<trust-domain>/ns/<namespace>/sa/<service-account>`) and a
//! command scope that limits which tenant/project operations the bearer may
//! request or execute.

use moqentra_types::{ProjectId, TenantId};
use serde::{Deserialize, Serialize};

/// Service identity attached to an mTLS peer certificate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceIdentity {
    /// SPIFFE ID in URI form.
    pub spiffe_id: String,
    /// Allowed tenant for command routing.  `None` for global infrastructure
    /// services that are not scoped to a single tenant.
    pub tenant_id: Option<TenantId>,
    /// Allowed projects within the tenant.  Empty means all projects in the
    /// tenant are permitted.  `None` / empty only when `tenant_id` is set and
    /// the service is a project-wide workload.
    pub project_ids: Vec<ProjectId>,
    /// Short-lived certificate thumbprint used to detect rotation/revocation.
    pub certificate_thumbprint: String,
}

impl ServiceIdentity {
    /// Create a tenant-scoped service identity (e.g. a Worker or Node Agent).
    pub fn tenant_scoped(
        spiffe_id: impl Into<String>,
        tenant_id: TenantId,
        certificate_thumbprint: impl Into<String>,
    ) -> Self {
        Self {
            spiffe_id: spiffe_id.into(),
            tenant_id: Some(tenant_id),
            project_ids: Vec::new(),
            certificate_thumbprint: certificate_thumbprint.into(),
        }
    }

    /// Restrict the identity to a specific project.
    pub fn for_project(mut self, project_id: ProjectId) -> Self {
        self.project_ids.push(project_id);
        self
    }

    /// Verify that a command targets an allowed tenant and, when required, an
    /// allowed project.  Returns an error with a safe message so callers
    /// cannot enumerate valid scopes.
    pub fn verify_command_scope(
        &self,
        tenant_id: TenantId,
        project_id: Option<ProjectId>,
    ) -> Result<(), moqentra_types::Error> {
        let allowed = self.tenant_id.ok_or_else(|| {
            moqentra_types::Error::permission_denied("identity has no tenant scope")
        })?;
        if allowed != tenant_id {
            return Err(moqentra_types::Error::permission_denied(
                "identity not authorized for tenant",
            ));
        }
        if let Some(pid) = project_id {
            if !self.project_ids.is_empty() && !self.project_ids.contains(&pid) {
                return Err(moqentra_types::Error::permission_denied(
                    "identity not authorized for project",
                ));
            }
        }
        Ok(())
    }

    /// Parse a SPIFFE ID and extract namespace/service-account components.
    /// Performs only syntactic validation; trust-domain and signature checks
    /// are done by the TLS layer.
    pub fn parse_spiffe_id(id: &str) -> Result<(String, String, String), moqentra_types::Error> {
        let prefix = "spiffe://";
        if !id.starts_with(prefix) {
            return Err(moqentra_types::Error::invalid_argument(
                "SPIFFE ID must start with spiffe://",
            ));
        }
        let rest = &id[prefix.len()..];
        let parts: Vec<_> = rest.split('/').collect();
        if parts.get(1) != Some(&"ns") || parts.get(3) != Some(&"sa") {
            return Err(moqentra_types::Error::invalid_argument(
                "SPIFFE ID must follow spiffe://<trust>/ns/<namespace>/sa/<service-account>",
            ));
        }
        Ok((
            parts
                .first()
                .copied()
                .ok_or_else(|| {
                    moqentra_types::Error::invalid_argument("missing SPIFFE trust domain")
                })?
                .to_string(),
            parts
                .get(2)
                .copied()
                .ok_or_else(|| moqentra_types::Error::invalid_argument("missing SPIFFE namespace"))?
                .to_string(),
            parts
                .get(4)
                .copied()
                .ok_or_else(|| {
                    moqentra_types::Error::invalid_argument("missing SPIFFE service account")
                })?
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    #[test]
    fn tenant_scoped_identity_requires_tenant_match() {
        let g = RandomIdGenerator;
        let tenant = TenantId::new_v7(&g);
        let id =
            ServiceIdentity::tenant_scoped("spiffe://moqentra/ns/prod/sa/worker", tenant, "t1");
        assert!(id.verify_command_scope(tenant, None).is_ok());

        let other = TenantId::new_v7(&g);
        assert!(id.verify_command_scope(other, None).is_err());
    }

    #[test]
    fn project_restricted_identity_enforces_project() {
        let g = RandomIdGenerator;
        let tenant = TenantId::new_v7(&g);
        let allowed = ProjectId::new_v7(&g);
        let other = ProjectId::new_v7(&g);
        let id =
            ServiceIdentity::tenant_scoped("spiffe://moqentra/ns/prod/sa/worker", tenant, "t1")
                .for_project(allowed);
        assert!(id.verify_command_scope(tenant, Some(allowed)).is_ok());
        assert!(id.verify_command_scope(tenant, Some(other)).is_err());
    }

    #[test]
    fn parse_spiffe_id_extracts_components() {
        let (trust, ns, sa) =
            ServiceIdentity::parse_spiffe_id("spiffe://moqentra/ns/prod/sa/worker").unwrap();
        assert_eq!(trust, "moqentra");
        assert_eq!(ns, "prod");
        assert_eq!(sa, "worker");
    }

    #[test]
    fn parse_rejects_malformed_spiffe_id() {
        assert!(ServiceIdentity::parse_spiffe_id("http://example").is_err());
        assert!(ServiceIdentity::parse_spiffe_id("spiffe://moqentra/ns/prod").is_err());
    }
}
