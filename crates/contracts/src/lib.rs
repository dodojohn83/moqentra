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
#[allow(clippy::as_conversions)]
mod tests {
    use crate::moqentra::common::v1::{
        Error as ProtoError, ErrorKind, EventEnvelope, EventStatus, Operation, OperationStatus,
        Pagination, RequestContext, ResourceRef,
    };
    use crate::moqentra::worker::v1::{Framework, ModelFormat, WorkerCapabilities};
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

    #[test]
    fn operation_roundtrip() {
        let original = Operation {
            id: "op-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            operation_type: "CreateDataset".to_string(),
            status: OperationStatus::Pending as i32,
            progress: 0.0,
            resource_refs: vec![ResourceRef {
                resource_type: "dataset".to_string(),
                id: "dataset-1".to_string(),
            }],
            error: None,
            deadline: None,
            cancelled: false,
            retry_count: 0,
            event_sequence: 0,
            sse_cursor: "".to_string(),
            created_at: None,
            updated_at: None,
        };
        let mut buf = Vec::new();
        original.encode(&mut buf).unwrap();
        let decoded = Operation::decode(buf.as_slice()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn event_envelope_roundtrip() {
        let original = EventEnvelope {
            event_id: "evt-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            operation_id: "op-1".to_string(),
            aggregate_type: "dataset".to_string(),
            aggregate_id: "dataset-1".to_string(),
            sequence: 1,
            event_type: "DatasetCreated".to_string(),
            payload: br#"{"id":"dataset-1"}"#.to_vec(),
            occurred_at: None,
            correlation_id: "corr-1".to_string(),
            causation_id: "cause-1".to_string(),
            sse_cursor: "c1".to_string(),
            status: EventStatus::Pending as i32,
            attempt: 1,
        };
        let mut buf = Vec::new();
        original.encode(&mut buf).unwrap();
        let decoded = EventEnvelope::decode(buf.as_slice()).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn worker_capabilities_roundtrip() {
        let original = WorkerCapabilities {
            agent_build_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            frameworks: vec![Framework {
                name: "PyTorch".to_string(),
                version: "2.6.0".to_string(),
            }],
            hardware_label: "NVIDIA-RTX3090".to_string(),
            device_labels: vec!["GPU-0".to_string()],
            driver_version: "560.35.03".to_string(),
            runtime_version: "cuda12.6".to_string(),
            runtimes: vec!["cuda".to_string(), "cudnn".to_string()],
            model_formats: vec![ModelFormat {
                name: "onnx".to_string(),
                version: vec!["1.17".to_string()],
            }],
            collective_backend: "nccl".to_string(),
            device_memory_bytes: 24 * 1024 * 1024 * 1024,
            max_parallelism: 4,
            supports_gpu: true,
            supports_npu: false,
        };
        let mut buf = Vec::new();
        original.encode(&mut buf).unwrap();
        let decoded = WorkerCapabilities::decode(buf.as_slice()).unwrap();
        assert_eq!(original, decoded);
    }
}
