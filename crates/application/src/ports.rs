//! Application-layer repository ports.
//!
//! Each port is an asynchronous trait whose methods explicitly accept a
//! [`moqentra_types::RequestContext`]. Optimistic concurrency is enforced via
//! an expected [`Revision`] on updates and deletes; successful writes return a
//! [`Versioned`] value that carries the new revision and a stable ETag.

#![allow(missing_docs)]

use async_trait::async_trait;
use moqentra_domain::{
    annotation::{AnnotationProject, AnnotationTask},
    application::{Application, ApplicationVersion},
    approval::ApprovalRequest,
    conversion::{ConversionJob, EvaluationRun},
    dataset::{Dataset, DatasetVersion},
    inference::Deployment,
    model_registry::{Model, ModelVersion},
    quota::{QuotaPolicy, QuotaReservation},
    training::TrainingJob,
};
use moqentra_types::{
    AnnotationProjectId, AnnotationTaskId, ApplicationId, ApplicationVersionId, ApprovalRequestId,
    ConversionJobId, DatasetId, DatasetVersionId, DeploymentId, Error, EvaluationRunId, ModelId,
    ModelVersionId, Page, PageRequest, ProjectId, QuotaPolicyId, QuotaReservationId,
    RequestContext, Revision, TrainingJobId,
};

/// A value paired with its optimistic-concurrency metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Versioned<T> {
    pub entity: T,
    pub revision: Revision,
    pub etag: String,
}

impl<T> Versioned<T> {
    pub fn new(entity: T, revision: Revision) -> Self {
        let etag = format!("W/\"{}\"", revision.as_u64());
        Self {
            entity,
            revision,
            etag,
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Versioned<U> {
        Versioned {
            entity: f(self.entity),
            revision: self.revision,
            etag: self.etag,
        }
    }
}

/// Generic filter for a resource list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceListFilter {
    pub state: Option<String>,
    pub name_prefix: Option<String>,
}

/// Repository for datasets.
#[async_trait]
pub trait DatasetRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &RequestContext,
        dataset: Dataset,
    ) -> Result<Versioned<Dataset>, Error>;

    async fn get(&self, ctx: &RequestContext, id: DatasetId) -> Result<Versioned<Dataset>, Error>;

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<Dataset>>, Error>;

    async fn update(
        &self,
        ctx: &RequestContext,
        id: DatasetId,
        expected: Revision,
        dataset: Dataset,
    ) -> Result<Versioned<Dataset>, Error>;

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: DatasetId,
        expected: Revision,
    ) -> Result<(), Error>;

    async fn create_version(
        &self,
        ctx: &RequestContext,
        dataset_id: DatasetId,
        version: DatasetVersion,
    ) -> Result<Versioned<DatasetVersion>, Error>;

    async fn get_version(
        &self,
        ctx: &RequestContext,
        dataset_id: DatasetId,
        version_id: DatasetVersionId,
    ) -> Result<Versioned<DatasetVersion>, Error>;
}

/// Repository for annotation projects and tasks.
#[async_trait]
pub trait AnnotationRepository: Send + Sync {
    async fn create_project(
        &self,
        ctx: &RequestContext,
        project: AnnotationProject,
    ) -> Result<Versioned<AnnotationProject>, Error>;

    async fn get_project(
        &self,
        ctx: &RequestContext,
        id: AnnotationProjectId,
    ) -> Result<Versioned<AnnotationProject>, Error>;

    async fn list_projects(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<AnnotationProject>>, Error>;

    async fn update_project(
        &self,
        ctx: &RequestContext,
        id: AnnotationProjectId,
        expected: Revision,
        project: AnnotationProject,
    ) -> Result<Versioned<AnnotationProject>, Error>;

    async fn delete_project(
        &self,
        ctx: &RequestContext,
        id: AnnotationProjectId,
        expected: Revision,
    ) -> Result<(), Error>;

    async fn create_task(
        &self,
        ctx: &RequestContext,
        project_id: AnnotationProjectId,
        task: AnnotationTask,
    ) -> Result<Versioned<AnnotationTask>, Error>;

    async fn get_task(
        &self,
        ctx: &RequestContext,
        id: AnnotationTaskId,
    ) -> Result<Versioned<AnnotationTask>, Error>;

    async fn list_tasks(
        &self,
        ctx: &RequestContext,
        project_id: AnnotationProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<AnnotationTask>>, Error>;

    async fn update_task(
        &self,
        ctx: &RequestContext,
        id: AnnotationTaskId,
        expected: Revision,
        task: AnnotationTask,
    ) -> Result<Versioned<AnnotationTask>, Error>;
}

/// Repository for training jobs.
#[async_trait]
pub trait TrainingJobRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &RequestContext,
        job: TrainingJob,
    ) -> Result<Versioned<TrainingJob>, Error>;

    async fn get(
        &self,
        ctx: &RequestContext,
        id: TrainingJobId,
    ) -> Result<Versioned<TrainingJob>, Error>;

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<TrainingJob>>, Error>;

    async fn update(
        &self,
        ctx: &RequestContext,
        id: TrainingJobId,
        expected: Revision,
        job: TrainingJob,
    ) -> Result<Versioned<TrainingJob>, Error>;

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: TrainingJobId,
        expected: Revision,
    ) -> Result<(), Error>;
}

/// Repository for models and model versions.
#[async_trait]
pub trait ModelRepository: Send + Sync {
    async fn create(&self, ctx: &RequestContext, model: Model) -> Result<Versioned<Model>, Error>;

    async fn get(&self, ctx: &RequestContext, id: ModelId) -> Result<Versioned<Model>, Error>;

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<Model>>, Error>;

    async fn update(
        &self,
        ctx: &RequestContext,
        id: ModelId,
        expected: Revision,
        model: Model,
    ) -> Result<Versioned<Model>, Error>;

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: ModelId,
        expected: Revision,
    ) -> Result<(), Error>;

    async fn create_version(
        &self,
        ctx: &RequestContext,
        model_id: ModelId,
        version: ModelVersion,
    ) -> Result<Versioned<ModelVersion>, Error>;

    async fn get_version(
        &self,
        ctx: &RequestContext,
        model_id: ModelId,
        version_id: ModelVersionId,
    ) -> Result<Versioned<ModelVersion>, Error>;
}

/// Repository for applications and application versions.
#[async_trait]
pub trait ApplicationRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &RequestContext,
        application: Application,
    ) -> Result<Versioned<Application>, Error>;

    async fn get(
        &self,
        ctx: &RequestContext,
        id: ApplicationId,
    ) -> Result<Versioned<Application>, Error>;

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<Application>>, Error>;

    async fn update(
        &self,
        ctx: &RequestContext,
        id: ApplicationId,
        expected: Revision,
        application: Application,
    ) -> Result<Versioned<Application>, Error>;

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: ApplicationId,
        expected: Revision,
    ) -> Result<(), Error>;

    async fn create_version(
        &self,
        ctx: &RequestContext,
        application_id: ApplicationId,
        version: ApplicationVersion,
    ) -> Result<Versioned<ApplicationVersion>, Error>;

    async fn get_version(
        &self,
        ctx: &RequestContext,
        application_id: ApplicationId,
        version_id: ApplicationVersionId,
    ) -> Result<Versioned<ApplicationVersion>, Error>;
}

/// Repository for deployments.
#[async_trait]
pub trait DeploymentRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &RequestContext,
        deployment: Deployment,
    ) -> Result<Versioned<Deployment>, Error>;

    async fn get(
        &self,
        ctx: &RequestContext,
        id: DeploymentId,
    ) -> Result<Versioned<Deployment>, Error>;

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<Deployment>>, Error>;

    async fn update(
        &self,
        ctx: &RequestContext,
        id: DeploymentId,
        expected: Revision,
        deployment: Deployment,
    ) -> Result<Versioned<Deployment>, Error>;

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: DeploymentId,
        expected: Revision,
    ) -> Result<(), Error>;
}

/// Repository for model conversion jobs.
#[async_trait]
pub trait ConversionRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &RequestContext,
        job: ConversionJob,
    ) -> Result<Versioned<ConversionJob>, Error>;

    async fn get(
        &self,
        ctx: &RequestContext,
        id: ConversionJobId,
    ) -> Result<Versioned<ConversionJob>, Error>;

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<ConversionJob>>, Error>;

    async fn update(
        &self,
        ctx: &RequestContext,
        id: ConversionJobId,
        expected: Revision,
        job: ConversionJob,
    ) -> Result<Versioned<ConversionJob>, Error>;

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: ConversionJobId,
        expected: Revision,
    ) -> Result<(), Error>;
}

/// Repository for model evaluation runs.
#[async_trait]
pub trait EvaluationRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &RequestContext,
        run: EvaluationRun,
    ) -> Result<Versioned<EvaluationRun>, Error>;

    async fn get(
        &self,
        ctx: &RequestContext,
        id: EvaluationRunId,
    ) -> Result<Versioned<EvaluationRun>, Error>;

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<EvaluationRun>>, Error>;

    async fn update(
        &self,
        ctx: &RequestContext,
        id: EvaluationRunId,
        expected: Revision,
        run: EvaluationRun,
    ) -> Result<Versioned<EvaluationRun>, Error>;

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: EvaluationRunId,
        expected: Revision,
    ) -> Result<(), Error>;
}

/// Repository for quota policies and reservations.
#[async_trait]
pub trait QuotaRepository: Send + Sync {
    async fn create_policy(
        &self,
        ctx: &RequestContext,
        policy: QuotaPolicy,
    ) -> Result<Versioned<QuotaPolicy>, Error>;

    async fn get_policy(
        &self,
        ctx: &RequestContext,
        id: QuotaPolicyId,
    ) -> Result<Versioned<QuotaPolicy>, Error>;

    async fn list_policies(
        &self,
        ctx: &RequestContext,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<QuotaPolicy>>, Error>;

    async fn create_reservation(
        &self,
        ctx: &RequestContext,
        reservation: QuotaReservation,
    ) -> Result<Versioned<QuotaReservation>, Error>;

    async fn get_reservation(
        &self,
        ctx: &RequestContext,
        id: QuotaReservationId,
    ) -> Result<Versioned<QuotaReservation>, Error>;

    /// Return active (non-expired and non-terminal) reservations for a dimension.
    async fn active_reservations(
        &self,
        ctx: &RequestContext,
        dimension: moqentra_domain::quota::ResourceDimension,
    ) -> Result<Vec<QuotaReservation>, Error>;

    async fn update_reservation(
        &self,
        ctx: &RequestContext,
        id: QuotaReservationId,
        reservation: QuotaReservation,
        expected: Revision,
    ) -> Result<Versioned<QuotaReservation>, Error>;
}

/// Repository for approval requests and decisions.
#[async_trait]
pub trait ApprovalRepository: Send + Sync {
    async fn create_request(
        &self,
        ctx: &RequestContext,
        request: ApprovalRequest,
    ) -> Result<Versioned<ApprovalRequest>, Error>;

    async fn get_request(
        &self,
        ctx: &RequestContext,
        id: ApprovalRequestId,
    ) -> Result<Versioned<ApprovalRequest>, Error>;

    async fn list_requests(
        &self,
        ctx: &RequestContext,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<ApprovalRequest>>, Error>;

    async fn update_request(
        &self,
        ctx: &RequestContext,
        id: ApprovalRequestId,
        request: ApprovalRequest,
        expected: Revision,
    ) -> Result<Versioned<ApprovalRequest>, Error>;
}
