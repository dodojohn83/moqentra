//! Moqentra domain aggregates and state machines.

#![allow(missing_docs)]

pub mod annotation;
pub mod conversion;
pub mod dataset;
pub mod export;
pub mod import;
pub mod model_registry;
pub mod pipeline;
pub mod quality;
pub mod training;

pub use annotation::{
    Annotation, AnnotationLog, AnnotationProject, AnnotationProjectState, AnnotationTask,
    AnnotationTaskState, AutosaveResult, ExportFormat, Label, Ontology, TaskLease, TaskType,
};
pub use conversion::{
    ConversionJob, ConversionJobState, ConversionProfile, ConversionTarget, EvaluationMetric,
    EvaluationRun, EvaluationRunState, PromotionPolicy,
};
pub use dataset::{
    compute_manifest_digest, AssetRef, Dataset, DatasetState, DatasetVersion, DatasetVersionState,
};
pub use import::{ImportJob, ImportJobFailure, ImportJobState};
pub use model_registry::{
    Artifact, Attachment, Model, ModelArtifactManifest, ModelLineage, ModelSignature, ModelVersion,
    ModelVersionState, TensorSpec,
};
pub use pipeline::{
    HpoRun, HpoRunState, NodeRunState, Notebook, NotebookState, PipelineNode, PipelineRun,
    PipelineRunState, PipelineSpec, SearchParam, Trial, TrialState,
};
pub use quality::{
    AutoLabelJob, AutoLabelJobState, AutoLabelSuggestion, MultimodalAnnotation, MultimodalMeta,
    QualityReport, QualityRule, QualityRun, QualityRunState, QualityViolation, ReviewDecision,
    ReviewItem, Severity,
};
pub use training::{
    Attempt, Checkpoint, DistributedConfig, Experiment, MetricPoint, OutputManifest, Rank,
    RankState, ResourceRequest, TrainingJob, TrainingJobSpec, TrainingJobState,
};

pub mod placeholder {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}
