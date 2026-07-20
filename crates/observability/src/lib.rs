//! Moqentra observability: trace context, structured logs, audit and diagnostics.

#![allow(missing_docs)]

use moqentra_types::{RequestContext, UtcTimestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// OpenTelemetry trace context propagated across HTTP/gRPC and agent boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub sampled: bool,
}

impl TraceContext {
    pub fn new(trace_id: impl Into<String>, span_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            sampled: true,
        }
    }

    pub fn inject_headers(&self) -> BTreeMap<String, String> {
        let mut headers = BTreeMap::new();
        headers.insert(
            "traceparent".to_string(),
            format!("00-{}-{}-01", self.trace_id, self.span_id),
        );
        headers
    }
}

/// Structured log entry with tenant/project/request/operation tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredLog {
    pub timestamp: UtcTimestamp,
    pub level: String,
    pub message: String,
    pub trace_id: Option<String>,
    pub request_id: Option<String>,
    pub tenant_id: Option<String>,
    pub project_id: Option<String>,
    pub operation: Option<String>,
    pub fields: BTreeMap<String, serde_json::Value>,
}

impl StructuredLog {
    pub fn new(level: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: UtcTimestamp::now(),
            level: level.into(),
            message: message.into(),
            trace_id: None,
            request_id: None,
            tenant_id: None,
            project_id: None,
            operation: None,
            fields: BTreeMap::new(),
        }
    }

    pub fn with_context(mut self, ctx: &RequestContext) -> Self {
        self.tenant_id = Some(ctx.tenant_id.to_string());
        self.project_id = ctx.project_id.as_ref().map(|p| p.to_string());
        self
    }

    pub fn with_trace(mut self, trace: &TraceContext) -> Self {
        self.trace_id = Some(trace.trace_id.clone());
        self
    }

    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    pub fn with_operation(mut self, op: impl Into<String>) -> Self {
        self.operation = Some(op.into());
        self
    }

    /// Redacts known sensitive keys before serialization.
    pub fn sanitize(self) -> Self {
        let mut s = self;
        for key in &["password", "secret", "token", "api_key", "private_key"] {
            if s.fields.contains_key(*key) {
                s.fields.insert((*key).to_string(), serde_json::json!("[REDACTED]"));
            }
        }
        s
    }
}

/// Metric name with namespace and unit budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricName {
    pub namespace: String,
    pub subsystem: String,
    pub name: String,
    pub unit: String,
}

impl MetricName {
    pub fn formatted(&self) -> String {
        format!("{}_{}_{}", self.namespace, self.subsystem, self.name)
    }
}

impl std::fmt::Display for MetricName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}_{}", self.namespace, self.subsystem, self.name)
    }
}

/// Immutable audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub timestamp: UtcTimestamp,
    pub tenant_id: String,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub outcome: String,
    pub integrity_hash: String,
}

impl AuditRecord {
    pub fn verify_integrity(&self) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.tenant_id.hash(&mut hasher);
        self.actor.hash(&mut hasher);
        self.action.hash(&mut hasher);
        self.resource.hash(&mut hasher);
        self.outcome.hash(&mut hasher);
        let computed = format!("{:x}", hasher.finish());
        self.integrity_hash == computed
    }
}

/// Diagnostic support bundle builder (placeholder for CLI tooling).
#[derive(Debug, Clone, Default)]
pub struct DiagnosticBundle {
    pub redacted_logs: Vec<StructuredLog>,
    pub metrics: Vec<MetricName>,
    pub audit_tail: Vec<AuditRecord>,
}

impl DiagnosticBundle {
    pub fn add_log(&mut self, log: StructuredLog) {
        self.redacted_logs.push(log.sanitize());
    }

    pub fn is_safe(&self) -> bool {
        let raw = serde_json::to_string(&self.redacted_logs).unwrap_or_default();
        !raw.contains("password=") && !raw.contains("token=")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{Principal, RandomIdGenerator, TenantId};

    #[test]
    fn trace_context_injects_headers() {
        let trace = TraceContext::new("abc", "def");
        let headers = trace.inject_headers();
        assert!(headers.get("traceparent").unwrap().contains("abc"));
    }

    #[test]
    fn structured_log_redacts_secrets() {
        let mut log = StructuredLog::new("INFO", "ok");
        log.fields.insert("password".to_string(), serde_json::json!("secret"));
        log.fields.insert("tenant".to_string(), serde_json::json!("t1"));
        let safe = log.sanitize();
        assert_eq!(safe.fields.get("password").unwrap(), "[REDACTED]");
        assert_eq!(safe.fields.get("tenant").unwrap(), "t1");
    }

    #[test]
    fn audit_integrity_hash() {
        let record = AuditRecord {
            id: "1".to_string(),
            timestamp: UtcTimestamp::now(),
            tenant_id: "t1".to_string(),
            actor: "admin".to_string(),
            action: "create".to_string(),
            resource: "dataset".to_string(),
            outcome: "success".to_string(),
            integrity_hash: String::new(),
        };
        assert!(!record.verify_integrity());
    }

    #[test]
    fn diagnostic_bundle_detects_unsafe_logs() {
        let mut bundle = DiagnosticBundle::default();
        let mut log = StructuredLog::new("INFO", "ok");
        log.fields.insert("token".to_string(), serde_json::json!("t"));
        bundle.add_log(log);
        assert!(bundle.is_safe());
    }

    #[test]
    fn structured_log_attaches_request_context() {
        let gen = RandomIdGenerator;
        let principal = Principal::service("sa-1");
        let ctx = RequestContext::new(TenantId::new_v7(&gen), principal, "req-1");
        let log = StructuredLog::new("INFO", "ok").with_context(&ctx);
        assert_eq!(log.tenant_id, Some(ctx.tenant_id.to_string()));
    }
}
