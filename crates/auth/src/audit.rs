//! Audit event model and logging port.

use moqentra_types::{Principal, ProjectId, TenantId, UtcTimestamp};

/// Category of an auditable action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditCategory {
    Authentication,
    Authorization,
    Write,
    DataExport,
    Training,
    ModelPublish,
    Deployment,
    SecretAccess,
    SystemAdmin,
}

impl AuditCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditCategory::Authentication => "authentication",
            AuditCategory::Authorization => "authorization",
            AuditCategory::Write => "write",
            AuditCategory::DataExport => "data_export",
            AuditCategory::Training => "training",
            AuditCategory::ModelPublish => "model_publish",
            AuditCategory::Deployment => "deployment",
            AuditCategory::SecretAccess => "secret_access",
            AuditCategory::SystemAdmin => "system_admin",
        }
    }
}

/// Outcome of an audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    Success,
    Denied,
    Failure,
}

/// An audit event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub event_id: String,
    pub category: AuditCategory,
    pub actor: Principal,
    pub tenant_id: TenantId,
    pub project_id: Option<ProjectId>,
    pub action: String,
    pub resource: String,
    pub outcome: AuditOutcome,
    pub reason: Option<String>,
    pub correlation_id: String,
    pub occurred_at: UtcTimestamp,
}

/// Sink for audit events.
#[async_trait::async_trait]
pub trait AuditLog: Send + Sync {
    async fn record(&self, event: AuditEvent) -> Result<(), moqentra_types::Error>;
}

/// In-memory audit log for tests.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAuditLog {
    events: std::sync::Arc<std::sync::Mutex<Vec<AuditEvent>>>,
}

impl InMemoryAuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[async_trait::async_trait]
impl AuditLog for InMemoryAuditLog {
    async fn record(&self, event: AuditEvent) -> Result<(), moqentra_types::Error> {
        self.events.lock().unwrap_or_else(|e| e.into_inner()).push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{Principal, RandomIdGenerator, RequestContext, UserId};

    #[tokio::test]
    async fn in_memory_audit_log_records() {
        let gen = RandomIdGenerator;
        let log = InMemoryAuditLog::new();
        let ctx = RequestContext::new(
            TenantId::new_v7(&gen),
            Principal::user(UserId::new_v7(&gen)),
            "req-1",
        );
        let event = AuditEvent {
            event_id: "evt-1".to_string(),
            category: AuditCategory::Authorization,
            actor: ctx.principal,
            tenant_id: ctx.tenant_id,
            project_id: None,
            action: "read".to_string(),
            resource: "dataset".to_string(),
            outcome: AuditOutcome::Denied,
            reason: Some("tenant isolation".to_string()),
            correlation_id: ctx.correlation_id.clone().unwrap_or_default(),
            occurred_at: UtcTimestamp::now(),
        };
        log.record(event.clone()).await.unwrap();
        assert_eq!(log.events().len(), 1);
    }
}
