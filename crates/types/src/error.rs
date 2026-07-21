//! Stable error taxonomy for the Moqentra platform.

use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::fmt;
use thiserror::Error;

/// Stable error kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ErrorKind {
    InvalidArgument,
    Unauthenticated,
    PermissionDenied,
    NotFound,
    AlreadyExists,
    Conflict,
    StaleRevision,
    StaleFence,
    Busy,
    QuotaExceeded,
    RateLimited,
    Timeout,
    Cancelled,
    Unavailable,
    Unsupported,
    VersionMismatch,
    ExternalFailed,
    Internal,
}

impl ErrorKind {
    /// Returns the stable string code used in logs and telemetry.
    pub const fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::InvalidArgument => "INVALID_ARGUMENT",
            ErrorKind::Unauthenticated => "UNAUTHENTICATED",
            ErrorKind::PermissionDenied => "PERMISSION_DENIED",
            ErrorKind::NotFound => "NOT_FOUND",
            ErrorKind::AlreadyExists => "ALREADY_EXISTS",
            ErrorKind::Conflict => "CONFLICT",
            ErrorKind::StaleRevision => "STALE_REVISION",
            ErrorKind::StaleFence => "STALE_FENCE",
            ErrorKind::Busy => "BUSY",
            ErrorKind::QuotaExceeded => "QUOTA_EXCEEDED",
            ErrorKind::RateLimited => "RATE_LIMITED",
            ErrorKind::Timeout => "TIMEOUT",
            ErrorKind::Cancelled => "CANCELLED",
            ErrorKind::Unavailable => "UNAVAILABLE",
            ErrorKind::Unsupported => "UNSUPPORTED",
            ErrorKind::VersionMismatch => "VERSION_MISMATCH",
            ErrorKind::ExternalFailed => "EXTERNAL_FAILED",
            ErrorKind::Internal => "INTERNAL",
        }
    }

    /// Maps the error kind to an HTTP Problem Details status code.
    pub const fn http_status(&self) -> StatusCode {
        match self {
            ErrorKind::InvalidArgument => StatusCode::BAD_REQUEST,
            ErrorKind::Unauthenticated => StatusCode::UNAUTHORIZED,
            ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::AlreadyExists => StatusCode::CONFLICT,
            ErrorKind::Conflict => StatusCode::CONFLICT,
            ErrorKind::StaleRevision => StatusCode::CONFLICT,
            ErrorKind::StaleFence => StatusCode::CONFLICT,
            ErrorKind::Busy => StatusCode::SERVICE_UNAVAILABLE,
            ErrorKind::QuotaExceeded => StatusCode::TOO_MANY_REQUESTS,
            ErrorKind::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ErrorKind::Timeout => StatusCode::REQUEST_TIMEOUT,
            ErrorKind::Cancelled => StatusCode::REQUEST_TIMEOUT,
            ErrorKind::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorKind::Unsupported => StatusCode::NOT_IMPLEMENTED,
            ErrorKind::VersionMismatch => StatusCode::BAD_REQUEST,
            ErrorKind::ExternalFailed => StatusCode::BAD_GATEWAY,
            ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Whether the operation may be retried with the same idempotency key.
    pub const fn retryable(&self) -> bool {
        matches!(
            self,
            ErrorKind::Busy
                | ErrorKind::Unavailable
                | ErrorKind::Timeout
                | ErrorKind::ExternalFailed
                | ErrorKind::RateLimited
        )
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A field-level validation violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldViolation {
    pub field: String,
    pub description: String,
}

impl FieldViolation {
    pub fn new(field: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            description: description.into(),
        }
    }
}

/// Platform error.
///
/// `source` is boxed and not exposed through `Display` to avoid leaking
/// internal details to callers.
#[derive(Error, Debug)]
#[error("{kind}: {safe_message}")]
pub struct Error {
    pub kind: ErrorKind,
    safe_message: String,
    retryable: bool,
    field_violations: Vec<FieldViolation>,
    request_id: Option<String>,
    correlation_id: Option<String>,
    #[source]
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl Error {
    fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            kind,
            retryable: kind.retryable(),
            safe_message: message,
            field_violations: Vec::new(),
            request_id: None,
            correlation_id: None,
            source: None,
        }
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidArgument, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    pub fn already_exists(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::AlreadyExists, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Conflict, message)
    }

    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::PermissionDenied, message)
    }

    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unauthenticated, message)
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::RateLimited, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unavailable, message)
    }

    pub fn external_failed(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ExternalFailed, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    pub fn with_source(mut self, source: impl StdError + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn with_field_violation(
        mut self,
        field: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.field_violations.push(FieldViolation::new(field, description));
        self
    }

    /// Returns the safe, caller-facing message.
    pub fn safe_message(&self) -> &str {
        &self.safe_message
    }

    pub fn is_retryable(&self) -> bool {
        self.retryable
    }

    pub fn field_violations(&self) -> &[FieldViolation] {
        &self.field_violations
    }

    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }

    /// Returns a problem-details-style JSON object for HTTP responses.
    pub fn problem_details(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::new();
        map.insert(
            "type".to_string(),
            serde_json::Value::String(format!("/errors/{}", self.kind.as_str())),
        );
        map.insert(
            "title".to_string(),
            serde_json::Value::String(self.kind.to_string()),
        );
        map.insert(
            "status".to_string(),
            serde_json::Value::Number(serde_json::Number::from(self.kind.http_status().as_u16())),
        );
        map.insert(
            "detail".to_string(),
            serde_json::Value::String(self.safe_message.clone()),
        );
        map.insert(
            "code".to_string(),
            serde_json::Value::String(self.kind.as_str().to_string()),
        );
        if !self.field_violations.is_empty() {
            let violations: Vec<serde_json::Value> = self
                .field_violations
                .iter()
                .map(|v| {
                    serde_json::json!({
                        "field": v.field,
                        "description": v.description,
                    })
                })
                .collect();
            map.insert(
                "violations".to_string(),
                serde_json::Value::Array(violations),
            );
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_kind_to_http_status() {
        assert_eq!(ErrorKind::NotFound.http_status(), StatusCode::NOT_FOUND);
        assert_eq!(
            ErrorKind::Internal.http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn error_safe_message_no_source_leak() {
        let err =
            Error::internal("something went wrong").with_source(std::io::Error::other("secret"));
        let display = err.to_string();
        assert!(!display.contains("secret"));
        assert!(display.contains("something went wrong"));
    }

    #[test]
    fn problem_details_contains_code() {
        let err = Error::invalid_argument("bad input").with_field_violation("name", "too long");
        let details = err.problem_details();
        assert_eq!(details["status"], 400);
        assert_eq!(details["code"], "INVALID_ARGUMENT");
        assert!(details["violations"].is_array());
    }

    #[test]
    fn retryable_kinds() {
        assert!(Error::unavailable("busy").is_retryable());
        assert!(!Error::invalid_argument("bad").is_retryable());
    }
}
