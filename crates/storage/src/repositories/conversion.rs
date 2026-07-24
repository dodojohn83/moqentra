//! PostgreSQL implementation of conversion and evaluation repositories.

use async_trait::async_trait;
use moqentra_application::ports::{
    ConversionRepository, EvaluationRepository, ResourceListFilter, Versioned,
};
use moqentra_domain::conversion::{
    ConversionJob, ConversionJobState, ConversionTarget, EvaluationMetric, EvaluationRun,
    EvaluationRunState,
};
use moqentra_types::{
    ConversionJobId, Error, EvaluationRunId, ModelVersionId, Page, PageRequest, ProjectId,
    RequestContext, Revision, TenantId,
};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

/// PostgreSQL-backed conversion repository.
#[derive(Debug, Clone)]
pub struct PgConversionRepository {
    pool: PgPool,
}

/// PostgreSQL-backed evaluation repository.
#[derive(Debug, Clone)]
pub struct PgEvaluationRepository {
    pool: PgPool,
}

impl PgConversionRepository {
    /// Create a new repository.
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

    fn revision_from_i64(value: i64) -> Result<Revision, Error> {
        let value =
            u64::try_from(value).map_err(|_| Error::internal("negative revision in database"))?;
        Ok(Revision::from_u64(value))
    }
}

impl PgEvaluationRepository {
    /// Create a new repository.
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

    fn revision_from_i64(value: i64) -> Result<Revision, Error> {
        let value =
            u64::try_from(value).map_err(|_| Error::internal("negative revision in database"))?;
        Ok(Revision::from_u64(value))
    }
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct ConversionJobRow {
    id: Uuid,
    tenant_id: Uuid,
    project_id: Uuid,
    source_model_version_id: Uuid,
    target: String,
    state: String,
    revision: i64,
    profile: Value,
    parameters: Value,
    output_artifacts: Value,
    cache_key: String,
    log_digest: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct EvaluationRunRow {
    id: Uuid,
    tenant_id: Uuid,
    project_id: Uuid,
    model_version_id: Uuid,
    dataset_version_id: Uuid,
    seed: i64,
    state: String,
    revision: i64,
    metrics: Value,
    hardware_profile: String,
    preprocess_version: String,
    postprocess_version: String,
    reference_outputs: Value,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn conversion_row_to_domain(row: &ConversionJobRow) -> Result<ConversionJob, Error> {
    let state: ConversionJobState = row
        .state
        .parse()
        .map_err(|e| Error::internal(format!("invalid stored conversion state: {e}")))?;
    let target: ConversionTarget = row
        .target
        .parse()
        .map_err(|e| Error::internal(format!("invalid stored conversion target: {e}")))?;
    let profile = serde_json::from_value(row.profile.clone())
        .map_err(|e| Error::internal(format!("failed to deserialize conversion profile: {e}")))?;
    let parameters = serde_json::from_value(row.parameters.clone()).map_err(|e| {
        Error::internal(format!("failed to deserialize conversion parameters: {e}"))
    })?;
    let output_artifacts = serde_json::from_value(row.output_artifacts.clone())
        .map_err(|e| Error::internal(format!("failed to deserialize output artifacts: {e}")))?;

    Ok(ConversionJob {
        id: ConversionJobId::from_uuid(row.id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        source_model_version_id: ModelVersionId::from_uuid(row.source_model_version_id),
        target,
        profile,
        parameters,
        state,
        output_artifacts,
        preview: false,
        cache_key: row.cache_key.clone(),
        log_digest: row.log_digest.clone(),
        created_at: moqentra_types::UtcTimestamp::new(row.created_at),
        updated_at: moqentra_types::UtcTimestamp::new(row.updated_at),
    })
}

fn conversion_job_to_values(job: &ConversionJob) -> Result<(Value, Value, Value, String), Error> {
    let profile = serde_json::to_value(&job.profile)
        .map_err(|e| Error::internal(format!("failed to serialize conversion profile: {e}")))?;
    let parameters = serde_json::to_value(&job.parameters)
        .map_err(|e| Error::internal(format!("failed to serialize conversion parameters: {e}")))?;
    let output_artifacts = serde_json::to_value(&job.output_artifacts)
        .map_err(|e| Error::internal(format!("failed to serialize output artifacts: {e}")))?;
    Ok((
        profile,
        parameters,
        output_artifacts,
        job.target.to_string(),
    ))
}

fn evaluation_row_to_domain(row: &EvaluationRunRow) -> Result<EvaluationRun, Error> {
    let state: EvaluationRunState = row
        .state
        .parse()
        .map_err(|e| Error::internal(format!("invalid stored evaluation state: {e}")))?;
    let metrics: Vec<EvaluationMetric> = serde_json::from_value(row.metrics.clone())
        .map_err(|e| Error::internal(format!("failed to deserialize evaluation metrics: {e}")))?;
    let reference_outputs = serde_json::from_value(row.reference_outputs.clone())
        .map_err(|e| Error::internal(format!("failed to deserialize reference outputs: {e}")))?;

    Ok(EvaluationRun {
        id: EvaluationRunId::from_uuid(row.id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        model_version_id: ModelVersionId::from_uuid(row.model_version_id),
        dataset_version_id: moqentra_types::DatasetVersionId::from_uuid(row.dataset_version_id),
        seed: u64::try_from(row.seed)
            .map_err(|_| Error::internal("negative seed in evaluation run row"))?,
        metrics,
        state,
        hardware_profile: row.hardware_profile.clone(),
        preprocess_version: row.preprocess_version.clone(),
        postprocess_version: row.postprocess_version.clone(),
        reference_outputs,
        created_at: moqentra_types::UtcTimestamp::new(row.created_at),
        updated_at: moqentra_types::UtcTimestamp::new(row.updated_at),
    })
}

fn evaluation_run_to_values(run: &EvaluationRun) -> Result<(Value, Value, String), Error> {
    let metrics = serde_json::to_value(&run.metrics)
        .map_err(|e| Error::internal(format!("failed to serialize evaluation metrics: {e}")))?;
    let reference_outputs = serde_json::to_value(&run.reference_outputs)
        .map_err(|e| Error::internal(format!("failed to serialize reference outputs: {e}")))?;
    Ok((metrics, reference_outputs, run.state.to_string()))
}

#[async_trait]
impl ConversionRepository for PgConversionRepository {
    async fn create(
        &self,
        ctx: &RequestContext,
        job: ConversionJob,
    ) -> Result<Versioned<ConversionJob>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let revision = 0i64;
        let (profile, parameters, output_artifacts, target) = conversion_job_to_values(&job)?;

        sqlx::query(
            "INSERT INTO conversion_jobs (id, tenant_id, project_id, source_model_version_id, target, state, revision, profile, parameters, output_artifacts, cache_key, log_digest, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(job.id.as_uuid())
        .bind(job.tenant_id.as_uuid())
        .bind(job.project_id.as_uuid())
        .bind(job.source_model_version_id.as_uuid())
        .bind(target)
        .bind(job.state.to_string())
        .bind(revision)
        .bind(profile)
        .bind(parameters)
        .bind(output_artifacts)
        .bind(&job.cache_key)
        .bind(job.log_digest.as_ref())
        .bind(job.created_at.as_offset())
        .bind(job.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create conversion job: {e}")))?;

        Ok(Versioned::new(job, Self::revision_from_i64(revision)?))
    }

    async fn get(
        &self,
        ctx: &RequestContext,
        id: ConversionJobId,
    ) -> Result<Versioned<ConversionJob>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: ConversionJobRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, source_model_version_id, target, state, revision, profile, parameters, output_artifacts, cache_key, log_digest, created_at, updated_at
             FROM conversion_jobs
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get conversion job: {e}")))?
        .ok_or_else(|| Error::not_found("conversion job"))?;

        let job = conversion_row_to_domain(&row)?;
        Ok(Versioned::new(job, Self::revision_from_i64(row.revision)?))
    }

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        _filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<ConversionJob>>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let (limit, offset) = crate::pagination_clause(page);

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM conversion_jobs WHERE tenant_id = $1 AND project_id = $2",
        )
        .bind(ctx.tenant_id.as_uuid())
        .bind(project_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to count conversion jobs: {e}")))?;

        let rows: Vec<ConversionJobRow> = sqlx::query_as(
            "SELECT id, tenant_id, project_id, source_model_version_id, target, state, revision, profile, parameters, output_artifacts, cache_key, log_digest, created_at, updated_at
             FROM conversion_jobs
             WHERE tenant_id = $1 AND project_id = $2
             ORDER BY updated_at DESC, id
             LIMIT $3 OFFSET $4",
        )
        .bind(ctx.tenant_id.as_uuid())
        .bind(project_id.as_uuid())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to list conversion jobs: {e}")))?;

        let items: Vec<Versioned<ConversionJob>> = rows
            .into_iter()
            .map(|row| {
                let revision = row.revision;
                let job = conversion_row_to_domain(&row)?;
                Ok(Versioned::new(job, Self::revision_from_i64(revision)?))
            })
            .collect::<Result<_, Error>>()?;

        let total = u64::try_from(total).map_err(|_| Error::internal("negative database count"))?;
        Ok(Page::new(items, total, page))
    }

    async fn update(
        &self,
        ctx: &RequestContext,
        id: ConversionJobId,
        expected: Revision,
        job: ConversionJob,
    ) -> Result<Versioned<ConversionJob>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let (profile, parameters, output_artifacts, target) = conversion_job_to_values(&job)?;
        let new_revision = expected.next();

        let result = sqlx::query(
            "UPDATE conversion_jobs
             SET target = $1, state = $2, revision = $3, profile = $4, parameters = $5, output_artifacts = $6, cache_key = $7, log_digest = $8, updated_at = $9
             WHERE id = $10 AND tenant_id = $11 AND revision = $12",
        )
        .bind(target)
        .bind(job.state.to_string())
        .bind(expected.as_i64()? + 1)
        .bind(profile)
        .bind(parameters)
        .bind(output_artifacts)
        .bind(&job.cache_key)
        .bind(job.log_digest.as_ref())
        .bind(job.updated_at.as_offset())
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(expected.as_i64()?)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to update conversion job: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(Error::conflict("conversion job revision mismatch"));
        }

        Ok(Versioned::new(job, new_revision))
    }

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: ConversionJobId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let result = sqlx::query(
            "DELETE FROM conversion_jobs WHERE id = $1 AND tenant_id = $2 AND revision = $3",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(expected.as_i64()?)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to delete conversion job: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(Error::conflict("conversion job revision mismatch"));
        }
        Ok(())
    }
}

#[async_trait]
impl EvaluationRepository for PgEvaluationRepository {
    async fn create(
        &self,
        ctx: &RequestContext,
        run: EvaluationRun,
    ) -> Result<Versioned<EvaluationRun>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let revision = 0i64;
        let (metrics, reference_outputs, state) = evaluation_run_to_values(&run)?;
        let seed_i64 = i64::try_from(run.seed)
            .map_err(|_| Error::invalid_argument(format!("seed {} exceeds i64 range", run.seed)))?;

        sqlx::query(
            "INSERT INTO evaluation_runs (id, tenant_id, project_id, model_version_id, dataset_version_id, seed, state, revision, metrics, hardware_profile, preprocess_version, postprocess_version, reference_outputs, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(run.id.as_uuid())
        .bind(run.tenant_id.as_uuid())
        .bind(run.project_id.as_uuid())
        .bind(run.model_version_id.as_uuid())
        .bind(run.dataset_version_id.as_uuid())
        .bind(seed_i64)
        .bind(state)
        .bind(revision)
        .bind(metrics)
        .bind(&run.hardware_profile)
        .bind(&run.preprocess_version)
        .bind(&run.postprocess_version)
        .bind(reference_outputs)
        .bind(run.created_at.as_offset())
        .bind(run.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create evaluation run: {e}")))?;

        Ok(Versioned::new(run, Self::revision_from_i64(revision)?))
    }

    async fn get(
        &self,
        ctx: &RequestContext,
        id: EvaluationRunId,
    ) -> Result<Versioned<EvaluationRun>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: EvaluationRunRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, model_version_id, dataset_version_id, seed, state, revision, metrics, hardware_profile, preprocess_version, postprocess_version, reference_outputs, created_at, updated_at
             FROM evaluation_runs
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get evaluation run: {e}")))?
        .ok_or_else(|| Error::not_found("evaluation run"))?;

        let run = evaluation_row_to_domain(&row)?;
        Ok(Versioned::new(run, Self::revision_from_i64(row.revision)?))
    }

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        _filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<EvaluationRun>>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let (limit, offset) = crate::pagination_clause(page);

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM evaluation_runs WHERE tenant_id = $1 AND project_id = $2",
        )
        .bind(ctx.tenant_id.as_uuid())
        .bind(project_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to count evaluation runs: {e}")))?;

        let rows: Vec<EvaluationRunRow> = sqlx::query_as(
            "SELECT id, tenant_id, project_id, model_version_id, dataset_version_id, seed, state, revision, metrics, hardware_profile, preprocess_version, postprocess_version, reference_outputs, created_at, updated_at
             FROM evaluation_runs
             WHERE tenant_id = $1 AND project_id = $2
             ORDER BY updated_at DESC, id
             LIMIT $3 OFFSET $4",
        )
        .bind(ctx.tenant_id.as_uuid())
        .bind(project_id.as_uuid())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to list evaluation runs: {e}")))?;

        let items: Vec<Versioned<EvaluationRun>> = rows
            .into_iter()
            .map(|row| {
                let revision = row.revision;
                let run = evaluation_row_to_domain(&row)?;
                Ok(Versioned::new(run, Self::revision_from_i64(revision)?))
            })
            .collect::<Result<_, Error>>()?;

        let total = u64::try_from(total).map_err(|_| Error::internal("negative database count"))?;
        Ok(Page::new(items, total, page))
    }

    async fn update(
        &self,
        ctx: &RequestContext,
        id: EvaluationRunId,
        expected: Revision,
        run: EvaluationRun,
    ) -> Result<Versioned<EvaluationRun>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let (metrics, reference_outputs, state) = evaluation_run_to_values(&run)?;
        let new_revision = expected.next();

        let result = sqlx::query(
            "UPDATE evaluation_runs
             SET state = $1, revision = $2, metrics = $3, hardware_profile = $4, preprocess_version = $5, postprocess_version = $6, reference_outputs = $7, updated_at = $8
             WHERE id = $9 AND tenant_id = $10 AND revision = $11",
        )
        .bind(state)
        .bind(expected.as_i64()? + 1)
        .bind(metrics)
        .bind(&run.hardware_profile)
        .bind(&run.preprocess_version)
        .bind(&run.postprocess_version)
        .bind(reference_outputs)
        .bind(run.updated_at.as_offset())
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(expected.as_i64()?)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to update evaluation run: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(Error::conflict("evaluation run revision mismatch"));
        }

        Ok(Versioned::new(run, new_revision))
    }

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: EvaluationRunId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let result = sqlx::query(
            "DELETE FROM evaluation_runs WHERE id = $1 AND tenant_id = $2 AND revision = $3",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(expected.as_i64()?)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to delete evaluation run: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(Error::conflict("evaluation run revision mismatch"));
        }
        Ok(())
    }
}

impl PgConversionRepository {
    async fn set_admin(&self) -> Result<(), Error> {
        sqlx::query("SELECT set_config('app.is_admin', 'true', true)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to set admin context: {e}")))?;
        Ok(())
    }

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

    pub async fn upsert_job(&self, job: &ConversionJob) -> Result<(), Error> {
        self.set_tenant(job.tenant_id).await?;
        let (profile, parameters, output_artifacts, target) = conversion_job_to_values(job)?;
        sqlx::query(
            "INSERT INTO conversion_jobs (id, tenant_id, project_id, source_model_version_id, target, state, revision, profile, parameters, output_artifacts, cache_key, log_digest, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (id) DO UPDATE SET
                target = EXCLUDED.target,
                state = EXCLUDED.state,
                profile = EXCLUDED.profile,
                parameters = EXCLUDED.parameters,
                output_artifacts = EXCLUDED.output_artifacts,
                cache_key = EXCLUDED.cache_key,
                log_digest = EXCLUDED.log_digest,
                updated_at = EXCLUDED.updated_at,
                revision = conversion_jobs.revision + 1",
        )
        .bind(job.id.as_uuid())
        .bind(job.tenant_id.as_uuid())
        .bind(job.project_id.as_uuid())
        .bind(job.source_model_version_id.as_uuid())
        .bind(target)
        .bind(job.state.to_string())
        .bind(profile)
        .bind(parameters)
        .bind(output_artifacts)
        .bind(&job.cache_key)
        .bind(job.log_digest.as_ref())
        .bind(job.created_at.as_offset())
        .bind(job.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to upsert conversion job: {e}")))?;
        Ok(())
    }

    pub async fn load_all_for_recovery(&self) -> Result<Vec<ConversionJob>, Error> {
        self.set_admin().await?;
        let rows: Vec<ConversionJobRow> = sqlx::query_as(
            "SELECT id, tenant_id, project_id, source_model_version_id, target, state, revision, profile, parameters, output_artifacts, cache_key, log_digest, created_at, updated_at
             FROM conversion_jobs
             ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to load conversion jobs: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            match conversion_row_to_domain(&row) {
                Ok(j) => out.push(j),
                Err(e) => tracing::warn!(error = %e, "skipping corrupt conversion job"),
            }
        }
        Ok(out)
    }
}

impl PgEvaluationRepository {
    async fn set_admin(&self) -> Result<(), Error> {
        sqlx::query("SELECT set_config('app.is_admin', 'true', true)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to set admin context: {e}")))?;
        Ok(())
    }

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

    pub async fn upsert_run(&self, run: &EvaluationRun) -> Result<(), Error> {
        self.set_tenant(run.tenant_id).await?;
        let (metrics, reference_outputs, state) = evaluation_run_to_values(run)?;
        let seed_i64 = i64::try_from(run.seed)
            .map_err(|_| Error::invalid_argument(format!("seed {} exceeds i64 range", run.seed)))?;
        sqlx::query(
            "INSERT INTO evaluation_runs (id, tenant_id, project_id, model_version_id, dataset_version_id, seed, state, revision, metrics, hardware_profile, preprocess_version, postprocess_version, reference_outputs, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, $9, $10, $11, $12, $13, $14)
             ON CONFLICT (id) DO UPDATE SET
                state = EXCLUDED.state,
                metrics = EXCLUDED.metrics,
                hardware_profile = EXCLUDED.hardware_profile,
                preprocess_version = EXCLUDED.preprocess_version,
                postprocess_version = EXCLUDED.postprocess_version,
                reference_outputs = EXCLUDED.reference_outputs,
                updated_at = EXCLUDED.updated_at,
                revision = evaluation_runs.revision + 1",
        )
        .bind(run.id.as_uuid())
        .bind(run.tenant_id.as_uuid())
        .bind(run.project_id.as_uuid())
        .bind(run.model_version_id.as_uuid())
        .bind(run.dataset_version_id.as_uuid())
        .bind(seed_i64)
        .bind(state)
        .bind(metrics)
        .bind(&run.hardware_profile)
        .bind(&run.preprocess_version)
        .bind(&run.postprocess_version)
        .bind(reference_outputs)
        .bind(run.created_at.as_offset())
        .bind(run.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to upsert evaluation run: {e}")))?;
        Ok(())
    }

    pub async fn load_all_for_recovery(&self) -> Result<Vec<EvaluationRun>, Error> {
        self.set_admin().await?;
        let rows: Vec<EvaluationRunRow> = sqlx::query_as(
            "SELECT id, tenant_id, project_id, model_version_id, dataset_version_id, seed, state, revision, metrics, hardware_profile, preprocess_version, postprocess_version, reference_outputs, created_at, updated_at
             FROM evaluation_runs
             ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to load evaluation runs: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            match evaluation_row_to_domain(&row) {
                Ok(r) => out.push(r),
                Err(e) => tracing::warn!(error = %e, "skipping corrupt evaluation run"),
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::conversion::{ConversionProfile, ConversionSupportTier, ConversionTarget};
    use moqentra_types::{
        ConversionJobId, ConversionProfileId, DatasetVersionId, EvaluationRunId, ModelVersionId,
        Principal, ProjectId, RandomIdGenerator, TenantId, UtcTimestamp,
    };

    async fn pg_pool() -> PgPool {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/moqentra".to_string());
        PgPool::connect(&url).await.unwrap()
    }

    fn ids() -> (
        TenantId,
        ProjectId,
        ModelVersionId,
        DatasetVersionId,
        ConversionJobId,
        EvaluationRunId,
    ) {
        let g = RandomIdGenerator;
        (
            TenantId::new_v7(&g),
            ProjectId::new_v7(&g),
            ModelVersionId::new_v7(&g),
            DatasetVersionId::new_v7(&g),
            ConversionJobId::new_v7(&g),
            EvaluationRunId::new_v7(&g),
        )
    }

    fn profile() -> ConversionProfile {
        ConversionProfile {
            id: ConversionProfileId::new_v7(&RandomIdGenerator),
            name: "onnx-default".to_string(),
            target: ConversionTarget::Onnx,
            sdk_version: "1.16.3".to_string(),
            toolchain_image_digest:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            target_chip: "generic".to_string(),
            precision: "fp32".to_string(),
            dynamic_shapes: false,
            capabilities: vec![],
            postprocess: None,
            parameter_schema: std::collections::BTreeMap::new(),
            support_tier: ConversionSupportTier::Verified,
            revision: 1,
            created_at: UtcTimestamp::now(),
        }
    }

    fn conversion_job(
        id: ConversionJobId,
        tenant_id: TenantId,
        project_id: ProjectId,
        source: ModelVersionId,
    ) -> ConversionJob {
        ConversionJob::new(
            id,
            tenant_id,
            project_id,
            source,
            ConversionTarget::Onnx,
            profile(),
            std::collections::BTreeMap::new(),
        )
        .unwrap()
    }

    fn evaluation_run(
        id: EvaluationRunId,
        tenant_id: TenantId,
        project_id: ProjectId,
        model_version_id: ModelVersionId,
        dataset_version_id: DatasetVersionId,
    ) -> EvaluationRun {
        EvaluationRun::new(
            id,
            tenant_id,
            project_id,
            model_version_id,
            dataset_version_id,
            42,
            "cpu",
            "v1",
            "v1",
        )
        .unwrap()
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn pg_conversion_job_crud() {
        let pool = pg_pool().await;
        let (t, p, m, _, cj_id, _) = ids();
        let repo = PgConversionRepository::new(pool);
        let ctx = RequestContext::new(t, Principal::Anonymous, "test");

        let job = conversion_job(cj_id, t, p, m);
        let created = repo.create(&ctx, job.clone()).await.unwrap();
        assert_eq!(created.entity.id, cj_id);

        let got = repo.get(&ctx, cj_id).await.unwrap();
        assert_eq!(got.entity.id, cj_id);
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL"]
    async fn pg_evaluation_run_crud() {
        let pool = pg_pool().await;
        let (t, p, m, dv, _, er_id) = ids();
        let repo = PgEvaluationRepository::new(pool);
        let ctx = RequestContext::new(t, Principal::Anonymous, "test");

        let run = evaluation_run(er_id, t, p, m, dv);
        let created = repo.create(&ctx, run.clone()).await.unwrap();
        assert_eq!(created.entity.id, er_id);

        let got = repo.get(&ctx, er_id).await.unwrap();
        assert_eq!(got.entity.id, er_id);
    }
}
