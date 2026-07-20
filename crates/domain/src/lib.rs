//! Moqentra domain aggregates and state machines.

#![allow(missing_docs)]

pub mod annotation;
pub mod dataset;
pub mod export;
pub mod import;

pub use annotation::{
    Annotation, AnnotationLog, AnnotationProject, AnnotationProjectState, AnnotationTask,
    AnnotationTaskState, AutosaveResult, ExportFormat, Label, Ontology, TaskLease, TaskType,
};
pub use dataset::{
    compute_manifest_digest, AssetRef, Dataset, DatasetState, DatasetVersion, DatasetVersionState,
};
pub use import::{ImportJob, ImportJobFailure, ImportJobState};

pub mod placeholder {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}
