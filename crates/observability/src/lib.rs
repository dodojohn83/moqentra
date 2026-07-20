//! Moqentra observability: trace context, structured logs, audit and diagnostics.

#![allow(missing_docs)]

use moqentra_types::{RequestContext, UtcTimestamp};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
        let flags = if self.sampled { "01" } else { "00" };
        headers.insert(
            "traceparent".to_string(),
            format!("00-{}-{}-{}", self.trace_id, self.span_id, flags),
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
        s.fields = Self::redact_fields(&s.fields);
        s
    }

    fn redact_fields(
        fields: &BTreeMap<String, serde_json::Value>,
    ) -> BTreeMap<String, serde_json::Value> {
        const SECRET_PATTERNS: &[&str] = &["password", "secret", "token", "api_key", "private_key"];
        fields
            .iter()
            .map(|(k, v)| {
                let redacted = SECRET_PATTERNS.iter().any(|p| k.to_lowercase().contains(p));
                let value = if redacted {
                    serde_json::json!("[REDACTED]")
                } else {
                    Self::redact_value(v)
                };
                (k.clone(), value)
            })
            .collect()
    }

    fn redact_value(value: &serde_json::Value) -> serde_json::Value {
        const SECRET_PATTERNS: &[&str] = &["password", "secret", "token", "api_key", "private_key"];
        match value {
            serde_json::Value::Object(map) => serde_json::Value::Object(
                map.iter()
                    .map(|(k, v)| {
                        let redacted = SECRET_PATTERNS.iter().any(|p| k.to_lowercase().contains(p));
                        if redacted {
                            (k.clone(), serde_json::json!("[REDACTED]"))
                        } else {
                            (k.clone(), Self::redact_value(v))
                        }
                    })
                    .collect(),
            ),
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(Self::redact_value).collect())
            }
            other => other.clone(),
        }
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
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.tenant_id.as_bytes());
        hasher.update(self.actor.as_bytes());
        hasher.update(self.action.as_bytes());
        hasher.update(self.resource.as_bytes());
        hasher.update(self.outcome.as_bytes());
        hasher.update(self.timestamp.to_string().as_bytes());
        let computed = format!("{:x}", hasher.finalize());
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
        let Ok(raw) = serde_json::to_string(&self.redacted_logs) else {
            return false;
        };
        !raw.contains("password=") && !raw.contains("token=") && !raw.contains("secret=")
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
