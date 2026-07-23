//! Observability context propagated across HTTP/gRPC, logs and agents.
//!
//! Resource IDs live in traces and logs; metric labels must remain low
//! cardinality and are validated separately by `MetricsRegistry`.

use moqentra_types::RequestContext;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Immutable request-scoped observability context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityContext {
    pub request_id: String,
    pub correlation_id: Option<String>,
    pub trace_id: Option<String>,
    pub tenant_id: String,
    pub project_id: Option<String>,
    pub operation: Option<String>,
    pub attempt_id: Option<String>,
    pub job_id: Option<String>,
    pub replica_id: Option<String>,
    pub deployment_id: Option<String>,
}

impl ObservabilityContext {
    pub fn new(request_id: impl Into<String>, tenant_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            correlation_id: None,
            trace_id: None,
            tenant_id: tenant_id.into(),
            project_id: None,
            operation: None,
            attempt_id: None,
            job_id: None,
            replica_id: None,
            deployment_id: None,
        }
    }

    pub fn from_request_context(
        ctx: &RequestContext,
        correlation_id: Option<String>,
        trace_id: Option<String>,
    ) -> Self {
        Self {
            request_id: ctx.request_id.clone(),
            correlation_id,
            trace_id,
            tenant_id: ctx.tenant_id.to_string(),
            project_id: ctx.project_id.as_ref().map(|p| p.to_string()),
            operation: None,
            attempt_id: None,
            job_id: None,
            replica_id: None,
            deployment_id: None,
        }
    }

    pub fn with_trace(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub fn with_project(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    pub fn with_attempt(mut self, attempt_id: impl Into<String>) -> Self {
        self.attempt_id = Some(attempt_id.into());
        self
    }

    pub fn with_job(mut self, job_id: impl Into<String>) -> Self {
        self.job_id = Some(job_id.into());
        self
    }

    pub fn with_replica(mut self, replica_id: impl Into<String>) -> Self {
        self.replica_id = Some(replica_id.into());
        self
    }

    pub fn with_deployment(mut self, deployment_id: impl Into<String>) -> Self {
        self.deployment_id = Some(deployment_id.into());
        self
    }

    /// Convert to a `BTreeMap` suitable for structured JSON logging.
    pub fn to_log_fields(&self) -> BTreeMap<String, serde_json::Value> {
        let mut fields = BTreeMap::new();
        fields.insert(
            "request_id".into(),
            serde_json::Value::String(self.request_id.clone()),
        );
        if let Some(c) = &self.correlation_id {
            fields.insert(
                "correlation_id".into(),
                serde_json::Value::String(c.clone()),
            );
        }
        if let Some(t) = &self.trace_id {
            fields.insert("trace_id".into(), serde_json::Value::String(t.clone()));
        }
        fields.insert(
            "tenant_id".into(),
            serde_json::Value::String(self.tenant_id.clone()),
        );
        if let Some(p) = &self.project_id {
            fields.insert("project_id".into(), serde_json::Value::String(p.clone()));
        }
        if let Some(o) = &self.operation {
            fields.insert("operation".into(), serde_json::Value::String(o.clone()));
        }
        fields
    }

    /// Create a tracing span with this context.
    pub fn make_span(&self) -> tracing::Span {
        tracing::info_span!(
            "request",
            request_id = %self.request_id,
            correlation_id = %self.correlation_id.as_deref().unwrap_or(""),
            trace_id = %self.trace_id.as_deref().unwrap_or(""),
            tenant_id = %self.tenant_id,
            project_id = %self.project_id.as_deref().unwrap_or(""),
            operation = %self.operation.as_deref().unwrap_or(""),
        )
    }
}
