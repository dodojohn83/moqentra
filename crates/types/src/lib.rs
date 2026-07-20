//! Moqentra shared foundation types.
//!
//! This crate contains identifiers, timestamps, pagination, request context,
//! configuration, and the platform-wide error taxonomy. It must not depend on
//! transport, storage, or runtime crates.

#![allow(missing_docs)]

pub mod config;
pub mod error;
pub mod id;
pub mod pagination;
pub mod request;
pub mod time;

pub use config::Configuration;
pub use error::{Error, ErrorKind, FieldViolation};
pub use id::{
    AnnotationId, AnnotationProjectId, AnnotationTaskId, ApplicationId, ApplicationVersionId,
    ArtifactDigest, AssetId, AttemptId, AutoLabelJobId, CheckpointId, ClusterId, ConversionJobId,
    DatasetId, DatasetVersionId, DeploymentId, EndpointId, EvaluationRunId, ExperimentId, HpoRunId,
    IdGenerator, ModelId, ModelVersionId, NodeId, NotebookId, OperationId, PipelineId,
    PipelineRunId, ProjectId, QualityRunId, RandomIdGenerator, RankId, ReplicaId,
    StaticIdGenerator, TenantId, TrainingJobId, UserId,
};
pub use pagination::{Page, PageRequest};
pub use request::{Principal, RequestContext, ResourceQuantity, ResourceRef};
pub use time::{Clock, Deadline, FencingToken, Revision, StaticClock, SystemClock, UtcTimestamp};

/// Crate version placeholder.
pub mod placeholder {
    /// Returns the crate version.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_sanity() {
        assert_eq!(placeholder::VERSION, "0.1.0");
    }
}
