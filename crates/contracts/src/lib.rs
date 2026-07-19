//! Moqentra `moqentra-contracts` crate.
//!
//! This crate contains generated protobuf contracts and JSON schema definitions.

#![allow(missing_docs)]

/// Placeholder module until domain types are added in subsequent tasks.
pub mod placeholder {
    /// Returns the crate version.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}

include!(concat!(env!("OUT_DIR"), "/prost_generated.rs"));

#[cfg(test)]
mod tests {
    use crate::moqentra::common::v1::{Error as ProtoError, ErrorKind, Pagination, RequestContext};
    use prost::Message;

    #[test]
    fn pagination_roundtrip() {
        let original = Pagination {
            limit: 10,
            offset: 5,
        };
        let mut buf = Vec::new();
        original.encode(&mut buf).unwrap();
        let decoded = Pagination::decode(buf.as_slice()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn request_context_roundtrip() {
        let original = RequestContext {
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            principal: "user:alice".to_string(),
            request_id: "req-1".to_string(),
            correlation_id: "corr-1".to_string(),
            deadline: None,
        };
        let mut buf = Vec::new();
        original.encode(&mut buf).unwrap();
        let decoded = RequestContext::decode(buf.as_slice()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn error_enum_unknown_value() {
        // Unrecognized enum values should not panic when decoded.
        let encoded = ErrorKind::Internal as i32;
        let decoded = ErrorKind::try_from(encoded);
        assert!(matches!(decoded, Ok(ErrorKind::Internal)));
    }

    #[test]
    fn error_message_roundtrip() {
        let original = ProtoError {
            kind: ErrorKind::NotFound as i32,
            code: "NOT_FOUND".to_string(),
            message: "missing".to_string(),
            retryable: false,
            violations: vec![],
            request_id: "req-1".to_string(),
            correlation_id: "corr-1".to_string(),
        };
        let mut buf = Vec::new();
        original.encode(&mut buf).unwrap();
        let decoded = ProtoError::decode(buf.as_slice()).unwrap();
        assert_eq!(original, decoded);
    }
}
