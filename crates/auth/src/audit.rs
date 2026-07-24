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
    pub policy_revision: u64,
    pub correlation_id: String,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub occurred_at: UtcTimestamp,
    /// SHA-256 integrity hash linking this event to the previous one.  The
    /// first event uses an all-zero previous hash.
    pub integrity_hash: String,
}

impl AuditEvent {
    /// Compute a SHA-256 integrity hash over the canonical fields and the
    /// previous hash.  This forms a hash chain across events so that deletion,
    /// insertion or reordering can be detected by `AuditChain::verify`.
    pub fn compute_integrity_hash(&self, previous_hash: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(previous_hash.as_bytes());
        hasher.update(self.event_id.as_bytes());
        hasher.update(self.category.as_str().as_bytes());
        hasher.update(self.actor.actor_id().as_bytes());
        hasher.update(self.tenant_id.to_string().as_bytes());
        if let Some(pid) = self.project_id {
            hasher.update(pid.to_string().as_bytes());
        }
        hasher.update(self.action.as_bytes());
        hasher.update(self.resource.as_bytes());
        hasher.update(self.outcome_as_str().as_bytes());
        if let Some(r) = &self.reason {
            hasher.update(r.as_bytes());
        }
        hasher.update(self.policy_revision.to_string().as_bytes());
        hasher.update(self.correlation_id.as_bytes());
        if let Some(rid) = &self.request_id {
            hasher.update(rid.as_bytes());
        }
        if let Some(tid) = &self.trace_id {
            hasher.update(tid.as_bytes());
        }
        hasher.update(self.occurred_at.to_string().as_bytes());
        format!("sha256:{:x}", hasher.finalize())
    }

    fn outcome_as_str(&self) -> &'static str {
        match self.outcome {
            AuditOutcome::Success => "success",
            AuditOutcome::Denied => "denied",
            AuditOutcome::Failure => "failure",
        }
    }
}

/// In-memory hash-chained audit log.
///
/// Records events in order and recomputes the integrity hash on append so each
/// event is linked to the previous one.  `verify` walks the chain and returns
/// false if any event has been tampered with, deleted, or reordered.
#[derive(Debug, Clone, Default)]
pub struct AuditChain {
    events: Vec<AuditEvent>,
}

impl AuditChain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, mut event: AuditEvent) {
        let previous =
            self.events.last().map(|e| e.integrity_hash.as_str()).unwrap_or(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            );
        event.integrity_hash = event.compute_integrity_hash(previous);
        self.events.push(event);
    }

    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    pub fn verify(&self) -> bool {
        let mut previous =
            "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        for event in &self.events {
            let expected = event.compute_integrity_hash(previous);
            if event.integrity_hash != expected {
                return false;
            }
            previous = &event.integrity_hash;
        }
        true
    }
}

/// Export a subset of an audit log with field allowlisting, redaction and size
/// budget.  This supports R2-AUDIT-006 while preventing secret leakage in
/// exported support bundles.
#[derive(Debug, Clone, Default)]
pub struct AuditExporter {
    pub allowed_fields: std::collections::BTreeSet<String>,
    pub max_events: usize,
    pub max_bytes: usize,
}

impl AuditExporter {
    pub fn new(max_events: usize, max_bytes: usize) -> Self {
        Self {
            allowed_fields: std::collections::BTreeSet::new(),
            max_events,
            max_bytes,
        }
    }

    pub fn allow_field(mut self, name: impl Into<String>) -> Self {
        self.allowed_fields.insert(name.into());
        self
    }

    /// Export a slice of audit events.  Returns redacted JSON objects and
    /// `true` when the export fit within the budget.
    pub fn export(
        &self,
        events: &[AuditEvent],
    ) -> Result<(Vec<serde_json::Value>, bool), moqentra_types::Error> {
        use serde::Serialize;

        #[derive(Serialize)]
        struct EventView {
            event_id: String,
            category: String,
            actor: String,
            tenant_id: String,
            project_id: Option<String>,
            action: String,
            resource: String,
            outcome: String,
            reason: Option<String>,
            policy_revision: u64,
            correlation_id: String,
            request_id: Option<String>,
            trace_id: Option<String>,
            occurred_at: String,
            integrity_hash: String,
        }

        let mut output = Vec::new();
        let mut total = 0usize;
        let mut truncated = false;

        for event in events.iter().take(self.max_events) {
            let view = EventView {
                event_id: redact_or_keep("event_id", &event.event_id, &self.allowed_fields),
                category: event.category.as_str().to_string(),
                actor: redact_or_keep("actor", &event.actor.actor_id(), &self.allowed_fields),
                tenant_id: redact_or_keep(
                    "tenant_id",
                    &event.tenant_id.to_string(),
                    &self.allowed_fields,
                ),
                project_id: event
                    .project_id
                    .map(|p| redact_or_keep("project_id", &p.to_string(), &self.allowed_fields)),
                action: redact_or_keep("action", &event.action, &self.allowed_fields),
                resource: redact_or_keep("resource", &event.resource, &self.allowed_fields),
                outcome: event.outcome_as_str().to_string(),
                reason: event
                    .reason
                    .as_ref()
                    .map(|r| redact_or_keep("reason", r, &self.allowed_fields)),
                policy_revision: event.policy_revision,
                correlation_id: redact_or_keep(
                    "correlation_id",
                    &event.correlation_id,
                    &self.allowed_fields,
                ),
                request_id: event
                    .request_id
                    .as_ref()
                    .map(|r| redact_or_keep("request_id", r, &self.allowed_fields)),
                trace_id: event
                    .trace_id
                    .as_ref()
                    .map(|t| redact_or_keep("trace_id", t, &self.allowed_fields)),
                occurred_at: event.occurred_at.to_string(),
                integrity_hash: redact_or_keep(
                    "integrity_hash",
                    &event.integrity_hash,
                    &self.allowed_fields,
                ),
            };
            let value = serde_json::to_value(view)
                .map_err(|e| moqentra_types::Error::internal(format!("export json: {e}")))?;
            let size = serde_json::to_string(&value)
                .map_err(|e| moqentra_types::Error::internal(format!("export json: {e}")))?
                .len();
            if total + size > self.max_bytes && !output.is_empty() {
                truncated = true;
                break;
            }
            total += size;
            output.push(value);
        }

        Ok((output, !truncated))
    }
}

fn redact_or_keep(
    field: &str,
    value: &str,
    allowed: &std::collections::BTreeSet<String>,
) -> String {
    if allowed.is_empty() || allowed.contains(field) {
        value.to_string()
    } else {
        "[REDACTED]".to_string()
    }
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
            policy_revision: 1,
            correlation_id: ctx.correlation_id.clone().unwrap_or_default(),
            request_id: Some("req-1".to_string()),
            trace_id: None,
            occurred_at: UtcTimestamp::now(),
            integrity_hash: String::new(),
        };
        log.record(event.clone()).await.unwrap();
        assert_eq!(log.events().len(), 1);
    }

    #[test]
    fn audit_chain_links_events() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let actor = Principal::user(UserId::new_v7(&gen));
        let mut chain = AuditChain::new();
        let e1 = AuditEvent {
            event_id: "evt-1".to_string(),
            category: AuditCategory::Authentication,
            actor: actor.clone(),
            tenant_id: tenant,
            project_id: None,
            action: "login".to_string(),
            resource: "user".to_string(),
            outcome: AuditOutcome::Success,
            reason: None,
            policy_revision: 1,
            correlation_id: "c1".to_string(),
            request_id: None,
            trace_id: None,
            occurred_at: UtcTimestamp::now(),
            integrity_hash: String::new(),
        };
        chain.append(e1);
        let e2 = AuditEvent {
            event_id: "evt-2".to_string(),
            category: AuditCategory::Write,
            actor,
            tenant_id: tenant,
            project_id: None,
            action: "create".to_string(),
            resource: "dataset".to_string(),
            outcome: AuditOutcome::Success,
            reason: None,
            policy_revision: 1,
            correlation_id: "c1".to_string(),
            request_id: None,
            trace_id: None,
            occurred_at: UtcTimestamp::now(),
            integrity_hash: String::new(),
        };
        chain.append(e2);
        assert!(chain.verify());
        assert_ne!(
            chain.events()[0].integrity_hash,
            chain.events()[1].integrity_hash
        );
    }

    #[test]
    fn audit_chain_detects_tampering() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let actor = Principal::user(UserId::new_v7(&gen));
        let mut chain = AuditChain::new();
        let e1 = AuditEvent {
            event_id: "evt-1".to_string(),
            category: AuditCategory::Authentication,
            actor,
            tenant_id: tenant,
            project_id: None,
            action: "login".to_string(),
            resource: "user".to_string(),
            outcome: AuditOutcome::Success,
            reason: None,
            policy_revision: 1,
            correlation_id: "c1".to_string(),
            request_id: None,
            trace_id: None,
            occurred_at: UtcTimestamp::now(),
            integrity_hash: String::new(),
        };
        chain.append(e1);
        let mut events = chain.events().to_vec();
        events[0].outcome = AuditOutcome::Failure;
        let tampered = AuditChain { events };
        assert!(!tampered.verify());
    }

    #[test]
    fn audit_exporter_honors_allowlist_and_budget() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let actor = Principal::user(UserId::new_v7(&gen));
        let event = AuditEvent {
            event_id: "evt-1".to_string(),
            category: AuditCategory::Authentication,
            actor,
            tenant_id: tenant,
            project_id: None,
            action: "login".to_string(),
            resource: "user".to_string(),
            outcome: AuditOutcome::Success,
            reason: None,
            policy_revision: 1,
            correlation_id: "c1".to_string(),
            request_id: None,
            trace_id: None,
            occurred_at: UtcTimestamp::now(),
            integrity_hash: String::new(),
        };
        let exporter = AuditExporter::new(10, 1024 * 1024).allow_field("action");
        let (out, complete) = exporter.export(&[event]).unwrap();
        assert!(complete);
        assert_eq!(out[0]["action"], "login");
        assert_eq!(out[0]["tenant_id"], "[REDACTED]");
    }
}
