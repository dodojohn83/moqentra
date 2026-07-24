//! PostgreSQL implementation of the training job repository port.

use async_trait::async_trait;
use moqentra_application::ports::{ResourceListFilter, TrainingJobRepository, Versioned};
use moqentra_domain::training::{
    Attempt, Experiment, MetricPoint, OutputManifest, TrainingJob, TrainingJobSpec,
    TrainingJobState,
};
use moqentra_types::{
    AttemptId, Error, ExperimentId, Page, PageRequest, ProjectId, RequestContext, Revision,
    TenantId, TrainingJobId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// PostgreSQL-backed training job repository.
#[derive(Debug, Clone)]
pub struct PgTrainingJobRepository {
    pool: PgPool,
}

impl PgTrainingJobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn set_tenant(&self, tenant_id: TenantId) -> Result<(), Error> {
        let tenant = tenant_id.to_string();
        sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(&tenant)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {e}")))?;
        Ok(())
    }

    async fn set_admin(&self) -> Result<(), Error> {
        sqlx::query("SELECT set_config('app.is_admin', 'true', true)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to set admin context: {e}")))?;
        Ok(())
    }

    /// List all queued training jobs across tenants for the scheduler.
    pub async fn list_queued_for_scheduler(
        &self,
        limit: u32,
    ) -> Result<Vec<Versioned<TrainingJob>>, Error> {
        self.set_admin().await?;
        let rows: Vec<TrainingJobRow> = sqlx::query_as(
            "SELECT id, tenant_id, project_id, experiment_id, dataset_version_id, model_id, name,
                    spec, state, revision, metadata, created_at, updated_at
             FROM training_jobs
             WHERE state = 'Queued'
             ORDER BY updated_at ASC, id
             LIMIT $1",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to list queued training jobs: {e}")))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let job = row_to_domain(&row)?;
            items.push(Versioned::new(job, Self::revision_from_i64(row.revision)?));
        }
        Ok(items)
    }

    fn revision_from_i64(value: i64) -> Result<Revision, Error> {
        let value =
            u64::try_from(value).map_err(|_| Error::internal("negative revision in database"))?;
        Ok(Revision::from_u64(value))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TrainingJobMetadata {
    experiment_id: ExperimentId,
    attempts: Vec<AttemptId>,
    current_attempt: Option<Attempt>,
    metrics: Vec<MetricPoint>,
    output_manifest: Option<OutputManifest>,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct TrainingJobRow {
    id: Uuid,
    tenant_id: Uuid,
    project_id: Uuid,
    experiment_id: Option<Uuid>,
    dataset_version_id: Option<Uuid>,
    model_id: Option<Uuid>,
    name: String,
    spec: Value,
    state: String,
    revision: i64,
    metadata: Value,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn job_to_metadata(job: &TrainingJob) -> TrainingJobMetadata {
    TrainingJobMetadata {
        experiment_id: job.experiment_id,
        attempts: job.attempts.clone(),
        current_attempt: job.current_attempt.clone(),
        metrics: job.metrics.clone(),
        output_manifest: job.output_manifest.clone(),
    }
}

fn row_to_domain(row: &TrainingJobRow) -> Result<TrainingJob, Error> {
    let spec: TrainingJobSpec = serde_json::from_value(row.spec.clone())
        .map_err(|e| Error::internal(format!("failed to deserialize training job spec: {e}")))?;
    spec.validate()
        .map_err(|e| Error::internal(format!("invalid stored training job spec: {e}")))?;

    let meta: TrainingJobMetadata = serde_json::from_value(row.metadata.clone()).map_err(|e| {
        Error::internal(format!("failed to deserialize training job metadata: {e}"))
    })?;

    let state: TrainingJobState = row
        .state
        .parse()
        .map_err(|e: Error| Error::internal(format!("unknown training job state: {e}")))?;

    Ok(TrainingJob {
        id: TrainingJobId::from_uuid(row.id),
        experiment_id: meta.experiment_id,
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        spec,
        state,
        current_attempt: meta.current_attempt,
        attempts: meta.attempts,
        metrics: meta.metrics,
        output_manifest: meta.output_manifest,
        created_at: UtcTimestamp::new(row.created_at),
        updated_at: UtcTimestamp::new(row.updated_at),
    })
}

#[async_trait]
impl TrainingJobRepository for PgTrainingJobRepository {
    async fn create(
        &self,
        ctx: &RequestContext,
        job: TrainingJob,
    ) -> Result<Versioned<TrainingJob>, Error> {
        job.spec.validate().map_err(|e| Error::invalid_argument(e.to_string()))?;
        self.set_tenant(ctx.tenant_id).await?;

        let metadata = serde_json::to_value(job_to_metadata(&job)).map_err(|e| {
            Error::internal(format!("failed to serialize training job metadata: {e}"))
        })?;
        let spec = serde_json::to_value(&job.spec)
            .map_err(|e| Error::internal(format!("failed to serialize training job spec: {e}")))?;
        let revision = 0i64;

        sqlx::query(
            "INSERT INTO training_jobs
             (id, tenant_id, project_id, experiment_id, dataset_version_id, model_id, name, spec, state, revision, metadata, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(job.id.as_uuid())
        .bind(job.tenant_id.as_uuid())
        .bind(job.project_id.as_uuid())
        .bind(job.experiment_id.as_uuid())
        .bind(job.spec.dataset_version_id.as_uuid())
        .bind(Option::<Uuid>::None)
        .bind(job.id.to_string())
        .bind(spec)
        .bind(format!("{:?}", job.state))
        .bind(revision)
        .bind(metadata)
        .bind(job.created_at.as_offset())
        .bind(job.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create training job: {e}")))?;

        Ok(Versioned::new(job, Self::revision_from_i64(revision)?))
    }

    async fn get(
        &self,
        ctx: &RequestContext,
        id: TrainingJobId,
    ) -> Result<Versioned<TrainingJob>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: TrainingJobRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, experiment_id, dataset_version_id, model_id, name,
                    spec, state, revision, metadata, created_at, updated_at
             FROM training_jobs
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get training job: {e}")))?
        .ok_or_else(|| Error::not_found("training job"))?;

        let job = row_to_domain(&row)?;
        Ok(Versioned::new(job, Self::revision_from_i64(row.revision)?))
    }

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<TrainingJob>>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let limit = page.limit.clamp(1, 1000) as i64;
        let offset = page.offset as i64;

        let mut conditions = vec!["tenant_id = $1".to_string(), "project_id = $2".to_string()];
        if filter.state.is_some() {
            conditions.push("state = $3".to_string());
        }
        if filter.name_prefix.is_some() {
            conditions.push(format!("name LIKE ${}", conditions.len() + 1));
        }

        let where_clause = conditions.join(" AND ");
        let base = format!("FROM training_jobs WHERE {where_clause}");

        let count_sql = format!("SELECT COUNT(*) {base}");
        let mut count_query =
            sqlx::query(&count_sql).bind(ctx.tenant_id.as_uuid()).bind(project_id.as_uuid());
        if let Some(state) = &filter.state {
            count_query = count_query.bind(state);
        }
        if let Some(prefix) = &filter.name_prefix {
            count_query = count_query.bind(format!("{prefix}%"));
        }

        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to count training jobs: {e}")))?
            .try_get(0)
            .map_err(|e| Error::internal(format!("failed to read count: {e}")))?;

        let items_sql = format!(
            "SELECT id, tenant_id, project_id, experiment_id, dataset_version_id, model_id, name,
                    spec, state, revision, metadata, created_at, updated_at
             {base}
             ORDER BY updated_at DESC, id
             LIMIT ${} OFFSET ${}",
            conditions.len() + 1,
            conditions.len() + 2
        );

        let mut query = sqlx::query_as(&items_sql)
            .bind(ctx.tenant_id.as_uuid())
            .bind(project_id.as_uuid());
        if let Some(state) = &filter.state {
            query = query.bind(state);
        }
        if let Some(prefix) = &filter.name_prefix {
            query = query.bind(format!("{prefix}%"));
        }
        query = query.bind(limit).bind(offset);

        let rows: Vec<TrainingJobRow> = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to list training jobs: {e}")))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let job = row_to_domain(&row)?;
            items.push(Versioned::new(job, Self::revision_from_i64(row.revision)?));
        }

        Ok(Page::new(items, total as u64, page))
    }

    async fn update(
        &self,
        ctx: &RequestContext,
        id: TrainingJobId,
        expected: Revision,
        job: TrainingJob,
    ) -> Result<Versioned<TrainingJob>, Error> {
        job.spec.validate().map_err(|e| Error::invalid_argument(e.to_string()))?;
        self.set_tenant(ctx.tenant_id).await?;

        let metadata = serde_json::to_value(job_to_metadata(&job)).map_err(|e| {
            Error::internal(format!("failed to serialize training job metadata: {e}"))
        })?;
        let spec = serde_json::to_value(&job.spec)
            .map_err(|e| Error::internal(format!("failed to serialize training job spec: {e}")))?;
        let expected_rev = expected.as_u64() as i64;

        let result = sqlx::query(
            "UPDATE training_jobs
             SET experiment_id = $3, dataset_version_id = $4, model_id = $5, spec = $6, state = $7,
                 metadata = $8, updated_at = $9, revision = revision + 1
             WHERE id = $1 AND tenant_id = $2 AND revision = $10",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(job.experiment_id.as_uuid())
        .bind(job.spec.dataset_version_id.as_uuid())
        .bind(Option::<Uuid>::None)
        .bind(spec)
        .bind(format!("{:?}", job.state))
        .bind(metadata)
        .bind(UtcTimestamp::now().as_offset())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to update training job: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("training job revision mismatch"));
        }

        Ok(Versioned::new(job, expected.next()))
    }

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: TrainingJobId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let expected_rev = expected.as_u64() as i64;
        let result = sqlx::query(
            "DELETE FROM training_jobs WHERE id = $1 AND tenant_id = $2 AND revision = $3",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to delete training job: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("training job revision mismatch"));
        }

        Ok(())
    }
}

impl PgTrainingJobRepository {
    /// Ensure tenant/project FK rows exist for write-through inserts.
    pub async fn ensure_tenant_project(
        &self,
        tenant_id: TenantId,
        project_id: ProjectId,
    ) -> Result<(), Error> {
        self.set_admin().await?;
        let tenant_str = tenant_id.to_string();
        sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING")
            .bind(tenant_id.as_uuid())
            .bind(format!("tenant-{tenant_str}"))
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to ensure tenant: {e}")))?;
        sqlx::query(
            "INSERT INTO projects (id, tenant_id, name) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        )
        .bind(project_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .bind(format!("project-{project_id}"))
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to ensure project: {e}")))?;
        Ok(())
    }

    /// Best-effort upsert of a training job (write-through from control-plane registry).
    pub async fn upsert_job(&self, job: &TrainingJob) -> Result<(), Error> {
        job.spec.validate().map_err(|e| Error::invalid_argument(e.to_string()))?;
        self.set_tenant(job.tenant_id).await?;
        let metadata = serde_json::to_value(job_to_metadata(job)).map_err(|e| {
            Error::internal(format!("failed to serialize training job metadata: {e}"))
        })?;
        let spec = serde_json::to_value(&job.spec)
            .map_err(|e| Error::internal(format!("failed to serialize training job spec: {e}")))?;

        sqlx::query(
            "INSERT INTO training_jobs
             (id, tenant_id, project_id, experiment_id, dataset_version_id, model_id, name, spec, state, revision, metadata, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0, $10, $11, $12)
             ON CONFLICT (id) DO UPDATE SET
                experiment_id = EXCLUDED.experiment_id,
                dataset_version_id = EXCLUDED.dataset_version_id,
                spec = EXCLUDED.spec,
                state = EXCLUDED.state,
                metadata = EXCLUDED.metadata,
                updated_at = EXCLUDED.updated_at,
                revision = training_jobs.revision + 1",
        )
        .bind(job.id.as_uuid())
        .bind(job.tenant_id.as_uuid())
        .bind(job.project_id.as_uuid())
        .bind(job.experiment_id.as_uuid())
        .bind(job.spec.dataset_version_id.as_uuid())
        .bind(Option::<Uuid>::None)
        .bind(job.id.to_string())
        .bind(spec)
        .bind(format!("{:?}", job.state))
        .bind(metadata)
        .bind(job.created_at.as_offset())
        .bind(job.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to upsert training job: {e}")))?;
        Ok(())
    }

    /// Persist experiment aggregate for restart recovery.
    pub async fn upsert_experiment(&self, experiment: &Experiment) -> Result<(), Error> {
        self.set_tenant(experiment.tenant_id).await?;
        let payload = serde_json::to_value(experiment)
            .map_err(|e| Error::internal(format!("failed to serialize experiment: {e}")))?;
        sqlx::query(
            "INSERT INTO experiments (id, tenant_id, project_id, name, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                payload = EXCLUDED.payload,
                updated_at = EXCLUDED.updated_at",
        )
        .bind(experiment.id.as_uuid())
        .bind(experiment.tenant_id.as_uuid())
        .bind(experiment.project_id.as_uuid())
        .bind(&experiment.name)
        .bind(payload)
        .bind(experiment.created_at.as_offset())
        .bind(experiment.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to upsert experiment: {e}")))?;
        Ok(())
    }

    /// Load all experiments and jobs for process restart recovery (admin).
    pub async fn load_all_for_recovery(
        &self,
    ) -> Result<(Vec<Experiment>, Vec<TrainingJob>), Error> {
        self.set_admin().await?;
        let exp_rows: Vec<(Uuid, Value)> =
            sqlx::query_as("SELECT id, payload FROM experiments ORDER BY created_at")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| Error::internal(format!("failed to load experiments: {e}")))?;

        let mut experiments = Vec::with_capacity(exp_rows.len());
        for (_id, payload) in exp_rows {
            let exp: Experiment = serde_json::from_value(payload)
                .map_err(|e| Error::internal(format!("failed to deserialize experiment: {e}")))?;
            experiments.push(exp);
        }

        let job_rows: Vec<TrainingJobRow> = sqlx::query_as(
            "SELECT id, tenant_id, project_id, experiment_id, dataset_version_id, model_id, name,
                    spec, state, revision, metadata, created_at, updated_at
             FROM training_jobs
             ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to load training jobs: {e}")))?;

        let mut jobs = Vec::with_capacity(job_rows.len());
        for row in job_rows {
            jobs.push(row_to_domain(&row)?);
        }
        Ok((experiments, jobs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PgDatasetRepository;
    use moqentra_application::ports::DatasetRepository;
    use moqentra_domain::dataset::{Dataset, DatasetVersion};
    use moqentra_domain::training::{
        DistributedConfig, ParameterSchema, ResourceRequest, TrainingJobSpec,
    };
    use moqentra_types::{Principal, RandomIdGenerator, UserId};
    use sqlx::PgPool;

    async fn pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPool::connect(&url).await.ok()
    }

    async fn setup_tenant_project(pool: &PgPool) -> (TenantId, ProjectId, UserId) {
        let gen = RandomIdGenerator;
        let tenant_id = TenantId::new_v7(&gen);
        let project_id = ProjectId::new_v7(&gen);
        let user_id = UserId::new_v7(&gen);
        let tenant_str = tenant_id.to_string();

        sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(&tenant_str)
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING")
            .bind(tenant_id.as_uuid())
            .bind(format!("tenant-{tenant_str}"))
            .execute(pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO projects (id, tenant_id, name) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        )
        .bind(project_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .bind(format!("project-{project_id}"))
        .execute(pool)
        .await
        .unwrap();

        (tenant_id, project_id, user_id)
    }

    fn ctx(tenant_id: TenantId, project_id: ProjectId, user_id: UserId) -> RequestContext {
        RequestContext::new(
            tenant_id,
            Principal::User { id: user_id },
            "test-req".to_string(),
        )
        .with_project(project_id)
    }

    fn valid_digest() -> String {
        "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string()
    }

    fn sample_spec(dataset_version_id: moqentra_types::DatasetVersionId) -> TrainingJobSpec {
        TrainingJobSpec {
            code_digest: valid_digest(),
            image_digest: valid_digest(),
            dataset_version_id,
            hyperparameters: ParameterSchema {
                argv: vec!["train".into()],
                env: Default::default(),
                config_files: Default::default(),
            },
            seed: 1,
            resources: ResourceRequest {
                replicas: 1,
                cpu_milli: 1000,
                memory_mib: 2048,
                ephemeral_storage_mib: 1024,
                accelerator_kind: None,
                accelerator_count: 0,
                topology: None,
            },
            distributed: DistributedConfig::Single,
            processes_per_replica: 1,
            checkpoint_policy: Default::default(),
            queue_ref: None,
            priority_class_ref: None,
            preemption_policy: Default::default(),
            resource_class_ref: None,
            max_attempts: 1,
            deadline_seconds: 60,
        }
    }

    #[tokio::test]
    async fn training_job_crud() {
        let Some(pool) = pool().await else { return };
        let (tenant_id, project_id, user_id) = setup_tenant_project(&pool).await;

        let c = ctx(tenant_id, project_id, user_id);
        let dataset_repo = PgDatasetRepository::new(pool.clone());

        sqlx::query("DELETE FROM training_jobs WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM dataset_versions WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM datasets WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();

        let dataset = Dataset::new(
            moqentra_types::DatasetId::new_v7(&RandomIdGenerator),
            tenant_id,
            project_id,
            "training-dataset",
        )
        .unwrap();
        let ds = dataset_repo.create(&c, dataset).await.unwrap();
        let version = DatasetVersion::new(
            moqentra_types::DatasetVersionId::new_v7(&RandomIdGenerator),
            ds.entity.id,
            tenant_id,
            project_id,
        );
        let v = dataset_repo.create_version(&c, ds.entity.id, version).await.unwrap();

        let repo = PgTrainingJobRepository::new(pool.clone());
        let exp_id = moqentra_types::ExperimentId::new_v7(&RandomIdGenerator);
        let job = TrainingJob::new(
            TrainingJobId::new_v7(&RandomIdGenerator),
            exp_id,
            tenant_id,
            project_id,
            sample_spec(v.entity.id),
        )
        .unwrap();

        let created = repo.create(&c, job.clone()).await.unwrap();
        assert_eq!(created.entity.id, job.id);
        assert_eq!(created.revision.as_u64(), 0);

        let fetched = repo.get(&c, job.id).await.unwrap();
        assert_eq!(fetched.entity.spec.dataset_version_id, v.entity.id);

        let listed = repo
            .list(
                &c,
                project_id,
                ResourceListFilter::default(),
                PageRequest::default(),
            )
            .await
            .unwrap();
        assert_eq!(listed.items.len(), 1);
        assert_eq!(listed.total, 1);

        repo.delete(&c, job.id, created.revision).await.unwrap();
        assert!(repo.get(&c, job.id).await.is_err());
    }

    #[tokio::test]
    async fn pg_scheduler_lists_queued_jobs() {
        let Some(pool) = pool().await else { return };
        let (tenant_id, project_id, user_id) = setup_tenant_project(&pool).await;
        let c = ctx(tenant_id, project_id, user_id);
        let dataset_repo = PgDatasetRepository::new(pool.clone());

        sqlx::query("DELETE FROM training_jobs WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM dataset_versions WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM datasets WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();

        let dataset = Dataset::new(
            moqentra_types::DatasetId::new_v7(&RandomIdGenerator),
            tenant_id,
            project_id,
            "training-dataset",
        )
        .unwrap();
        let ds = dataset_repo.create(&c, dataset).await.unwrap();
        let version = DatasetVersion::new(
            moqentra_types::DatasetVersionId::new_v7(&RandomIdGenerator),
            ds.entity.id,
            tenant_id,
            project_id,
        );
        let v = dataset_repo.create_version(&c, ds.entity.id, version).await.unwrap();

        let repo = PgTrainingJobRepository::new(pool.clone());
        let exp_id = moqentra_types::ExperimentId::new_v7(&RandomIdGenerator);
        let job = TrainingJob::new(
            TrainingJobId::new_v7(&RandomIdGenerator),
            exp_id,
            tenant_id,
            project_id,
            sample_spec(v.entity.id),
        )
        .unwrap();

        repo.create(&c, job.clone()).await.unwrap();

        let queued = repo.list_queued_for_scheduler(10).await.unwrap();
        assert!(queued.iter().any(|j| j.entity.id == job.id));

        repo.delete(&c, job.id, Revision::initial()).await.unwrap();
    }
}
