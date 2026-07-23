//! PostgreSQL implementation of the model registry repository port.

use async_trait::async_trait;
use moqentra_application::ports::{ModelRepository, ResourceListFilter, Versioned};
use moqentra_domain::model_registry::{Model, ModelLineage, ModelVersion, ModelVersionState};
use moqentra_types::{
    Error, ModelId, ModelVersionId, Page, PageRequest, ProjectId, RequestContext, Revision,
    TenantId, UserId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// PostgreSQL-backed model repository.
#[derive(Debug, Clone)]
pub struct PgModelRepository {
    pool: PgPool,
}

impl PgModelRepository {
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

    fn revision_from_i64(value: i64) -> Revision {
        let mut r = Revision::initial();
        for _ in 0..value {
            r = r.next();
        }
        r
    }
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct ModelRow {
    id: Uuid,
    tenant_id: Uuid,
    project_id: Uuid,
    name: String,
    state: String,
    revision: i64,
    metadata: Value,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct ModelVersionRow {
    id: Uuid,
    tenant_id: Uuid,
    project_id: Uuid,
    model_id: Uuid,
    version_number: i64,
    artifact_digest: Option<String>,
    manifest: Value,
    created_by: Uuid,
    state: String,
    updated_at: OffsetDateTime,
    created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelMetadata {
    version_ids: Vec<ModelVersionId>,
    latest_approved: Option<ModelVersionId>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelVersionManifest {
    version: String,
    signature: Value,
    artifacts: Value,
    lineage: ModelLineage,
    metrics: serde_json::Map<String, serde_json::Value>,
    attachments: Value,
    approved_by: Option<UserId>,
}

fn model_metadata(model: &Model) -> ModelMetadata {
    ModelMetadata {
        version_ids: model.version_ids.clone(),
        latest_approved: model.latest_approved,
    }
}

fn version_manifest(version: &ModelVersion) -> Result<Value, Error> {
    let mut metrics = serde_json::Map::new();
    for (k, v) in &version.metrics {
        metrics.insert(
            k.clone(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(*v).unwrap_or_else(|| serde_json::Number::from(0)),
            ),
        );
    }
    serde_json::to_value(ModelVersionManifest {
        version: version.version.clone(),
        signature: serde_json::to_value(&version.signature)
            .map_err(|e| Error::internal(e.to_string()))?,
        artifacts: serde_json::to_value(&version.artifacts)
            .map_err(|e| Error::internal(e.to_string()))?,
        lineage: version.lineage.clone(),
        metrics,
        attachments: serde_json::to_value(&version.attachments)
            .map_err(|e| Error::internal(e.to_string()))?,
        approved_by: version.approved_by,
    })
    .map_err(|e| Error::internal(e.to_string()))
}

fn model_row_to_domain(row: &ModelRow) -> Result<Model, Error> {
    let meta: ModelMetadata = serde_json::from_value(row.metadata.clone())
        .map_err(|e| Error::internal(format!("failed to deserialize model metadata: {e}")))?;

    Ok(Model {
        id: ModelId::from_uuid(row.id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        name: row.name.clone(),
        version_ids: meta.version_ids,
        latest_approved: meta.latest_approved,
        created_at: UtcTimestamp::new(row.created_at),
        updated_at: UtcTimestamp::new(row.updated_at),
    })
}

fn version_row_to_domain(row: &ModelVersionRow) -> Result<ModelVersion, Error> {
    let state: ModelVersionState = row
        .state
        .parse()
        .map_err(|e| Error::internal(format!("invalid stored model version state: {e}")))?;
    let manifest: ModelVersionManifest =
        serde_json::from_value(row.manifest.clone()).map_err(|e| {
            Error::internal(format!("failed to deserialize model version manifest: {e}"))
        })?;

    let signature = serde_json::from_value(manifest.signature)
        .map_err(|e| Error::internal(format!("failed to deserialize model signature: {e}")))?;
    let artifacts = serde_json::from_value(manifest.artifacts)
        .map_err(|e| Error::internal(format!("failed to deserialize model artifacts: {e}")))?;
    let attachments = serde_json::from_value(manifest.attachments)
        .map_err(|e| Error::internal(format!("failed to deserialize model attachments: {e}")))?;

    let metrics = manifest
        .metrics
        .into_iter()
        .filter_map(|(k, v)| v.as_f64().map(|f| (k, f)))
        .collect();

    let version = ModelVersion {
        id: ModelVersionId::from_uuid(row.id),
        model_id: ModelId::from_uuid(row.model_id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        version: manifest.version,
        state,
        signature,
        artifacts,
        lineage: manifest.lineage,
        metrics,
        attachments,
        approved_by: manifest.approved_by,
        created_at: UtcTimestamp::new(row.created_at),
        updated_at: UtcTimestamp::new(row.updated_at),
    };

    // Re-validate artifacts on load: stored state must reflect cleaned artifacts.
    if version.artifacts.iter().any(|a| a.scan_status != "clean") {
        return Err(Error::internal(
            "stored model version contains non-clean artifact",
        ));
    }

    Ok(version)
}

#[async_trait]
impl ModelRepository for PgModelRepository {
    async fn create(&self, ctx: &RequestContext, model: Model) -> Result<Versioned<Model>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let revision = 0i64;
        let metadata = serde_json::to_value(model_metadata(&model))
            .map_err(|e| Error::internal(e.to_string()))?;

        sqlx::query(
            "INSERT INTO models (id, tenant_id, project_id, name, state, revision, metadata, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(model.id.as_uuid())
        .bind(model.tenant_id.as_uuid())
        .bind(model.project_id.as_uuid())
        .bind(&model.name)
        .bind("Active")
        .bind(revision)
        .bind(metadata)
        .bind(model.created_at.as_offset())
        .bind(model.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create model: {e}")))?;

        Ok(Versioned::new(model, Self::revision_from_i64(revision)))
    }

    async fn get(&self, ctx: &RequestContext, id: ModelId) -> Result<Versioned<Model>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: ModelRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, name, state, revision, metadata, created_at, updated_at
             FROM models
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get model: {e}")))?
        .ok_or_else(|| Error::not_found("model"))?;

        let model = model_row_to_domain(&row)?;
        Ok(Versioned::new(model, Self::revision_from_i64(row.revision)))
    }

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<Model>>, Error> {
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
        let base = format!("FROM models WHERE {where_clause}");

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
            .map_err(|e| Error::internal(format!("failed to count models: {e}")))?
            .try_get(0)
            .map_err(|e| Error::internal(format!("failed to read count: {e}")))?;

        let items_sql = format!(
            "SELECT id, tenant_id, project_id, name, state, revision, metadata, created_at, updated_at
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

        let rows: Vec<ModelRow> = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to list models: {e}")))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let model = model_row_to_domain(&row)?;
            items.push(Versioned::new(model, Self::revision_from_i64(row.revision)));
        }

        Ok(Page::new(items, total as u64, page))
    }

    async fn update(
        &self,
        ctx: &RequestContext,
        id: ModelId,
        expected: Revision,
        model: Model,
    ) -> Result<Versioned<Model>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let expected_rev = expected.as_u64() as i64;
        let metadata = serde_json::to_value(model_metadata(&model))
            .map_err(|e| Error::internal(e.to_string()))?;

        let result = sqlx::query(
            "UPDATE models
             SET name = $3, metadata = $4, updated_at = $5, revision = revision + 1
             WHERE id = $1 AND tenant_id = $2 AND revision = $6",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(&model.name)
        .bind(metadata)
        .bind(UtcTimestamp::now().as_offset())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to update model: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("model revision mismatch"));
        }

        Ok(Versioned::new(model, expected.next()))
    }

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: ModelId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let expected_rev = expected.as_u64() as i64;

        let result =
            sqlx::query("DELETE FROM models WHERE id = $1 AND tenant_id = $2 AND revision = $3")
                .bind(id.as_uuid())
                .bind(ctx.tenant_id.as_uuid())
                .bind(expected_rev)
                .execute(&self.pool)
                .await
                .map_err(|e| Error::internal(format!("failed to delete model: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("model revision mismatch"));
        }

        Ok(())
    }

    async fn create_version(
        &self,
        ctx: &RequestContext,
        model_id: ModelId,
        version: ModelVersion,
    ) -> Result<Versioned<ModelVersion>, Error> {
        if version.model_id != model_id {
            return Err(Error::invalid_argument("version model_id mismatch"));
        }
        self.set_tenant(ctx.tenant_id).await?;

        let next_version_number: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_number), 0) + 1 FROM model_versions
             WHERE model_id = $1 AND tenant_id = $2",
        )
        .bind(model_id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to compute model version number: {e}")))?;

        let manifest = version_manifest(&version)?;
        let artifact_digest =
            version.artifacts.first().map(|a| a.digest.clone()).unwrap_or_default();
        let created_by = version.approved_by.map(|u| u.as_uuid()).unwrap_or_else(Uuid::nil);
        let revision = 0i64;

        sqlx::query(
            "INSERT INTO model_versions
             (id, tenant_id, project_id, model_id, version_number, artifact_digest, manifest, created_by, state, updated_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(version.id.as_uuid())
        .bind(version.tenant_id.as_uuid())
        .bind(version.project_id.as_uuid())
        .bind(model_id.as_uuid())
        .bind(next_version_number)
        .bind(artifact_digest)
        .bind(manifest)
        .bind(created_by)
        .bind(format!("{:?}", version.state))
        .bind(version.updated_at.as_offset())
        .bind(version.created_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create model version: {e}")))?;

        Ok(Versioned::new(version, Self::revision_from_i64(revision)))
    }

    async fn get_version(
        &self,
        ctx: &RequestContext,
        model_id: ModelId,
        version_id: ModelVersionId,
    ) -> Result<Versioned<ModelVersion>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: ModelVersionRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, model_id, version_number, artifact_digest,
                    manifest, created_by, state, updated_at, created_at
             FROM model_versions
             WHERE id = $1 AND model_id = $2 AND tenant_id = $3",
        )
        .bind(version_id.as_uuid())
        .bind(model_id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get model version: {e}")))?
        .ok_or_else(|| Error::not_found("model version"))?;

        let version = version_row_to_domain(&row)?;
        Ok(Versioned::new(version, Self::revision_from_i64(0)))
    }
}

impl PgModelRepository {
    async fn set_admin(&self) -> Result<(), Error> {
        sqlx::query("SELECT set_config('app.is_admin', 'true', true)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to set admin context: {e}")))?;
        Ok(())
    }

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

    /// Best-effort upsert of a model family.
    pub async fn upsert_model(&self, model: &Model) -> Result<(), Error> {
        self.set_tenant(model.tenant_id).await?;
        let metadata = serde_json::to_value(model_metadata(model))
            .map_err(|e| Error::internal(e.to_string()))?;
        sqlx::query(
            "INSERT INTO models (id, tenant_id, project_id, name, state, revision, metadata, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, 0, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                metadata = EXCLUDED.metadata,
                updated_at = EXCLUDED.updated_at,
                revision = models.revision + 1",
        )
        .bind(model.id.as_uuid())
        .bind(model.tenant_id.as_uuid())
        .bind(model.project_id.as_uuid())
        .bind(&model.name)
        .bind("Active")
        .bind(metadata)
        .bind(model.created_at.as_offset())
        .bind(model.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to upsert model: {e}")))?;
        Ok(())
    }

    /// Best-effort upsert of a model version (manifest + state).
    pub async fn upsert_version(&self, version: &ModelVersion) -> Result<(), Error> {
        self.set_tenant(version.tenant_id).await?;
        let manifest = version_manifest(version)?;
        let artifact_digest =
            version.artifacts.first().map(|a| a.digest.clone()).unwrap_or_default();
        let created_by = version.approved_by.map(|u| u.as_uuid()).unwrap_or_else(Uuid::nil);

        let updated = sqlx::query(
            "UPDATE model_versions
             SET artifact_digest = $3, manifest = $4, state = $5, updated_at = $6
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(version.id.as_uuid())
        .bind(version.tenant_id.as_uuid())
        .bind(&artifact_digest)
        .bind(&manifest)
        .bind(format!("{:?}", version.state))
        .bind(version.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to update model version: {e}")))?;

        if updated.rows_affected() == 0 {
            let next_version_number: i64 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(version_number), 0) + 1 FROM model_versions
                 WHERE model_id = $1 AND tenant_id = $2",
            )
            .bind(version.model_id.as_uuid())
            .bind(version.tenant_id.as_uuid())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to compute model version number: {e}")))?;

            sqlx::query(
                "INSERT INTO model_versions
                 (id, tenant_id, project_id, model_id, version_number, artifact_digest, manifest, created_by, state, updated_at, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 ON CONFLICT (id) DO UPDATE SET
                    artifact_digest = EXCLUDED.artifact_digest,
                    manifest = EXCLUDED.manifest,
                    state = EXCLUDED.state,
                    updated_at = EXCLUDED.updated_at",
            )
            .bind(version.id.as_uuid())
            .bind(version.tenant_id.as_uuid())
            .bind(version.project_id.as_uuid())
            .bind(version.model_id.as_uuid())
            .bind(next_version_number)
            .bind(artifact_digest)
            .bind(manifest)
            .bind(created_by)
            .bind(format!("{:?}", version.state))
            .bind(version.updated_at.as_offset())
            .bind(version.created_at.as_offset())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to insert model version: {e}")))?;
        }
        Ok(())
    }

    /// Load all models and versions for process restart recovery (admin).
    pub async fn load_all_for_recovery(&self) -> Result<(Vec<Model>, Vec<ModelVersion>), Error> {
        self.set_admin().await?;
        let model_rows: Vec<ModelRow> = sqlx::query_as(
            "SELECT id, tenant_id, project_id, name, state, revision, metadata, created_at, updated_at
             FROM models
             ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to load models: {e}")))?;

        let version_rows: Vec<ModelVersionRow> = sqlx::query_as(
            "SELECT id, tenant_id, project_id, model_id, version_number, artifact_digest,
                    manifest, created_by, state, updated_at, created_at
             FROM model_versions
             ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to load model versions: {e}")))?;

        let mut versions = Vec::with_capacity(version_rows.len());
        for row in version_rows {
            // Skip corrupt versions rather than failing full hydrate.
            match version_row_to_domain(&row) {
                Ok(v) => versions.push(v),
                Err(e) => tracing::warn!(error = %e, "skipping corrupt model version on hydrate"),
            }
        }

        let mut models = Vec::with_capacity(model_rows.len());
        for row in model_rows {
            models.push(model_row_to_domain(&row)?);
        }
        Ok((models, versions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::model_registry::{
        Artifact, Attachment, ModelLineage, ModelSignature, TensorSpec,
    };
    use moqentra_types::{
        AssetId, AttemptId, DatasetVersionId, ExperimentId, Principal, RandomIdGenerator,
        TrainingJobId, UserId,
    };
    use sqlx::PgPool;

    async fn pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPool::connect(&url).await.ok()
    }

    async fn setup_tenant_project_user(pool: &PgPool) -> (TenantId, ProjectId, UserId) {
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

    fn make_lineage(dataset_version_id: DatasetVersionId) -> ModelLineage {
        let gen = RandomIdGenerator;
        ModelLineage {
            training_job_id: Some(TrainingJobId::new_v7(&gen)),
            experiment_id: Some(ExperimentId::new_v7(&gen)),
            dataset_version_id,
            attempt_id: Some(AttemptId::new_v7(&gen)),
            annotation_project_id: None,
            base_model_version_id: None,
            code_digest: "sha256:5694d08a2e53ffcae0c3103e5ad6f6076abd960eb1f8a56577040bc1028f702b"
                .into(),
            image_digest: "sha256:6105d6cc76af400325e94d588ce511be5bfdbb73b437dc51eca43917d7a43e3d"
                .into(),
            hyperparameter_digest:
                "sha256:a8aa236e33e65ccc368827e0af1497b5f655cd460b9db8ebd82ad415d59ad0f2".into(),
            dataset_manifest_digest: None,
            framework: None,
            template: None,
            template_version: None,
            parameters_digest: None,
            metrics_digest: None,
            checkpoint_digests: Vec::new(),
            hardware_environment: None,
        }
    }

    fn make_version(
        model_id: ModelId,
        tenant_id: TenantId,
        project_id: ProjectId,
        dataset_version_id: DatasetVersionId,
    ) -> ModelVersion {
        let gen = RandomIdGenerator;
        let mut version = ModelVersion::new(
            ModelVersionId::new_v7(&gen),
            model_id,
            tenant_id,
            project_id,
            "v1",
            make_lineage(dataset_version_id),
        )
        .unwrap();
        version.artifacts.push(Artifact {
            asset_id: AssetId::new_v7(&gen),
            digest: "sha256:9372c470eeadd5ecd9c3c74c2b3cb633f8e2f2fad799250a0f70d652b6b825e4"
                .into(),
            size_bytes: 1024,
            media_type: "application/octet-stream".into(),
            scan_status: "clean".into(),
            object_key: None,
        });
        version.signature = ModelSignature {
            inputs: vec![TensorSpec {
                name: "input".into(),
                dtype: "float32".into(),
                shape: vec!["?".into(), "3".into()],
            }],
            outputs: vec![TensorSpec {
                name: "output".into(),
                dtype: "float32".into(),
                shape: vec!["?".into(), "10".into()],
            }],
        };
        version.attachments.push(Attachment {
            kind: "license".into(),
            asset_id: AssetId::new_v7(&gen),
        });
        version.metrics.insert("accuracy".into(), 0.95);
        version.validate().unwrap();
        version.mark_ready().unwrap();
        version.approve(UserId::new_v7(&gen)).unwrap();
        version
    }

    #[tokio::test]
    async fn model_crud_and_version() {
        let Some(pool) = pool().await else { return };
        let (tenant_id, project_id, user_id) = setup_tenant_project_user(&pool).await;
        let c = ctx(tenant_id, project_id, user_id);

        sqlx::query("DELETE FROM model_versions WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM models WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();

        let repo = PgModelRepository::new(pool.clone());
        let model = Model::new(
            ModelId::new_v7(&RandomIdGenerator),
            tenant_id,
            project_id,
            "test-model",
        )
        .unwrap();
        let created = repo.create(&c, model.clone()).await.unwrap();
        assert_eq!(created.entity.name, "test-model");

        let fetched = repo.get(&c, created.entity.id).await.unwrap();
        assert_eq!(fetched.entity.id, created.entity.id);

        let version = make_version(
            model.id,
            tenant_id,
            project_id,
            DatasetVersionId::new_v7(&RandomIdGenerator),
        );
        let v = repo.create_version(&c, model.id, version).await.unwrap();
        assert_eq!(v.entity.state, ModelVersionState::Approved);

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

        let mut updated = created.entity.clone();
        updated.name = "updated-model".into();
        let versioned =
            repo.update(&c, updated.id, created.revision, updated.clone()).await.unwrap();
        assert_eq!(versioned.revision.as_u64(), 1);

        repo.delete(&c, updated.id, versioned.revision).await.unwrap();
        assert!(repo.get(&c, updated.id).await.is_err());
    }
}
