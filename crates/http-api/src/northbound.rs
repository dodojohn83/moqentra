//! Northbound API primitives: problem details, idempotency, cursors and webhooks.

use moqentra_types::{Error, RequestContext, UtcTimestamp};
use serde::{Deserialize, Serialize};

/// RFC 9457 Problem Details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub title: String,
    pub status: u16,
    pub code: String,
    pub detail: String,
    pub request_id: String,
    pub timestamp: UtcTimestamp,
}

impl ProblemDetails {
    pub fn from_error(err: &Error, request_id: impl Into<String>) -> Self {
        Self {
            problem_type: "about:blank".to_string(),
            title: err.to_string(),
            status: err.kind.http_status().as_u16(),
            code: err.kind.as_str().to_string(),
            detail: err.safe_message().to_string(),
            request_id: request_id.into(),
            timestamp: UtcTimestamp::now(),
        }
    }
}

/// Idempotency key with stored response digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyRecord {
    pub key: String,
    pub fingerprint: String,
    pub response_digest: String,
    pub completed_at: UtcTimestamp,
}

impl IdempotencyRecord {
    pub fn new(
        key: impl Into<String>,
        fingerprint: impl Into<String>,
        response_digest: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            fingerprint: fingerprint.into(),
            response_digest: response_digest.into(),
            completed_at: UtcTimestamp::now(),
        }
    }

    pub fn matches(&self, key: &str, fingerprint: &str) -> bool {
        self.key == key && self.fingerprint == fingerprint
    }
}

/// Opaque cursor for list pagination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub value: String,
    pub revision: u64,
}

/// Long-running operation reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationRef {
    pub operation_id: String,
    pub status_url: String,
    pub retry_after_seconds: u32,
}

/// Webhook subscription and delivery metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id: String,
    pub tenant_id: moqentra_types::TenantId,
    pub url: String,
    pub event_types: Vec<String>,
    pub secret_hmac: String,
    pub active: bool,
    pub max_retries: u32,
    pub circuit_open: bool,
}

impl WebhookSubscription {
    pub fn sign_payload(&self, body: &[u8], delivery_id: &str, timestamp: i64) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.secret_hmac.hash(&mut hasher);
        delivery_id.hash(&mut hasher);
        timestamp.hash(&mut hasher);
        body.hash(&mut hasher);
        format!("sha256={:x}", hasher.finish())
    }

    pub fn validate_url(&self) -> Result<(), Error> {
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err(Error::invalid_argument("invalid url"));
        }
        let after_scheme = self.url.split_once("://").map(|(_, rest)| rest).unwrap_or("");
        let host = after_scheme.split('/').next().unwrap_or("").split(':').next().unwrap_or("");
        if host.is_empty()
            || host == "localhost"
            || host == "127.0.0.1"
            || host.starts_with("10.")
            || host.starts_with("192.168.")
        {
            return Err(Error::invalid_argument(
                "SSRF: internal address not allowed",
            ));
        }
        Ok(())
    }
}

/// SSE event envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SseEvent {
    pub cursor: String,
    pub event_type: String,
    pub tenant_id: String,
    pub payload: serde_json::Value,
}

/// Request authorization summary for middleware.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedRequest {
    pub context: RequestContext,
    pub idempotency_key: Option<String>,
    pub if_match: Option<u64>,
}

/// Minimal rate-limit window (placeholder for a token-bucket implementation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitWindow {
    pub tenant_id: moqentra_types::TenantId,
    pub remaining: u32,
    pub reset_at: UtcTimestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{RandomIdGenerator, TenantId};

    #[test]
    fn problem_details_hides_stack() {
        let err = Error::invalid_argument("visible message")
            .with_source(std::io::Error::other("internal stack trace"));
        let pd = ProblemDetails::from_error(&err, "req-1");
        assert_eq!(pd.status, 400);
        assert!(pd.detail.contains("visible message"));
        assert!(!pd.detail.contains("internal stack trace"));
    }

    #[test]
    fn idempotency_record_matches_fingerprint() {
        let rec = IdempotencyRecord::new("key-1", "fp-1", "resp-abc");
        assert!(rec.matches("key-1", "fp-1"));
        assert!(!rec.matches("key-1", "fp-2"));
    }

    #[test]
    fn webhook_rejects_internal_addresses() {
        let sub = WebhookSubscription {
            id: "w1".to_string(),
            tenant_id: TenantId::new_v7(&RandomIdGenerator),
            url: "http://localhost:8080/hook".to_string(),
            event_types: vec![],
            secret_hmac: "secret".to_string(),
            active: true,
            max_retries: 3,
            circuit_open: false,
        };
        assert!(sub.validate_url().is_err());
    }

    #[test]
    fn webhook_accepts_external_addresses() {
        let sub = WebhookSubscription {
            id: "w2".to_string(),
            tenant_id: TenantId::new_v7(&RandomIdGenerator),
            url: "https://example.com/hook".to_string(),
            event_types: vec![],
            secret_hmac: "secret".to_string(),
            active: true,
            max_retries: 3,
            circuit_open: false,
        };
        assert!(sub.validate_url().is_ok());
    }

    #[test]
    fn sse_event_includes_cursor() {
        let ev = SseEvent {
            cursor: "c1".to_string(),
            event_type: "dataset.created".to_string(),
            tenant_id: "tenant-1".to_string(),
            payload: serde_json::json!({}),
        };
        assert_eq!(ev.cursor, "c1");
    }
}
