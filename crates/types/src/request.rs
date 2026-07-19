//! Request context, principal, and resource primitives.

use crate::id::{ProjectId, TenantId, UserId};
use crate::time::{Deadline, UtcTimestamp};
use serde::{Deserialize, Serialize};
use std::fmt;

/// An actor making a request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Principal {
    User {
        id: UserId,
    },
    Service {
        name: String,
    },
    #[default]
    Anonymous,
}

impl Principal {
    pub fn user(id: UserId) -> Self {
        Self::User { id }
    }

    pub fn service(name: impl Into<String>) -> Self {
        Self::Service { name: name.into() }
    }

    pub fn is_authenticated(&self) -> bool {
        !matches!(self, Principal::Anonymous)
    }
}

/// Scoped request context carried through every service call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestContext {
    pub tenant_id: TenantId,
    pub project_id: Option<ProjectId>,
    pub principal: Principal,
    pub request_id: String,
    pub correlation_id: Option<String>,
    pub deadline: Option<Deadline>,
}

impl RequestContext {
    pub fn new(tenant_id: TenantId, principal: Principal, request_id: impl Into<String>) -> Self {
        Self {
            tenant_id,
            project_id: None,
            principal,
            request_id: request_id.into(),
            correlation_id: None,
            deadline: None,
        }
    }

    pub fn with_project(mut self, project_id: ProjectId) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn with_deadline(mut self, deadline: Deadline) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn is_expired_at(&self, now: UtcTimestamp) -> bool {
        self.deadline.is_some_and(|d| d.is_expired_at(now))
    }
}

/// A typed resource reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceRef {
    pub resource_type: String,
    pub id: String,
}

impl ResourceRef {
    pub fn new(resource_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            id: id.into(),
        }
    }
}

/// Quantifiable resource request (CPU, memory, GPU, NPU).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResourceQuantity {
    pub cpu_cores: u32,
    pub memory_gib: u32,
    pub gpu_count: u32,
    pub npu_count: u32,
}

impl ResourceQuantity {
    pub const fn zero() -> Self {
        Self {
            cpu_cores: 0,
            memory_gib: 0,
            gpu_count: 0,
            npu_count: 0,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.cpu_cores == 0 && self.memory_gib == 0 && self.gpu_count == 0 && self.npu_count == 0
    }
}

impl Default for ResourceQuantity {
    fn default() -> Self {
        Self::zero()
    }
}

impl fmt::Display for ResourceQuantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cpu={} mem={}Gi gpu={} npu={}",
            self.cpu_cores, self.memory_gib, self.gpu_count, self.npu_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{RandomIdGenerator, UserId};

    #[test]
    fn principal_authenticated() {
        let gen = RandomIdGenerator;
        let user = Principal::user(UserId::new_v7(&gen));
        assert!(user.is_authenticated());
        assert!(!Principal::Anonymous.is_authenticated());
    }

    #[test]
    fn request_context_project() {
        let gen = RandomIdGenerator;
        let ctx = RequestContext::new(TenantId::new_v7(&gen), Principal::Anonymous, "req-1")
            .with_project(ProjectId::new_v7(&gen));
        assert!(ctx.project_id.is_some());
    }
}
