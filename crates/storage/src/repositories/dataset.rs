//! PostgreSQL implementation of the dataset repository port.

use async_trait::async_trait;
use moqentra_application::ports::{DatasetRepository, ResourceListFilter, Versioned};
use moqentra_domain::dataset::{AssetRef, Dataset, DatasetVersion, DatasetVersionState};
use moqentra_types::{
    DatasetId, DatasetVersionId, Error, Page, PageRequest, ProjectId, RequestContext, Revision,
    TenantId, UtcTimestamp,
};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// PostgreSQL-backed dataset repository.
#[derive(Debug, Clone)]
pub struct PgDatasetRepository {
    pool: PgPool,
}

impl PgDatasetRepository {
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

    async fn load_versions(
        &self,
        tenant_id: TenantId,
        dataset_id: DatasetId,
    ) -> Result<Vec<DatasetVersion>, Error> {
        self.set_tenant(tenant_id).await?;
        let rows: Vec<DatasetVersionRow> = sqlx::query_as(
            "SELECT id, tenant_id, project_id, dataset_id, version_number, manifest, state,
                    published_at, created_by, created_at
             FROM dataset_versions
             WHERE tenant_id = $1 AND dataset_id = $2
             ORDER BY version_number DESC",
        )
        .bind(tenant_id.as_uuid())
        .bind(dataset_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to load dataset versions: {e}")))?;

        rows.into_iter().map(version_row_to_domain).collect()
    }
}

#[async_trait]
impl DatasetRepository for PgDatasetRepository {
    async fn create(
        &self,
        ctx: &RequestContext,
        dataset: Dataset,
    ) -> Result<Versioned<Dataset>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let metadata = serde_json::json!({ "labels": dataset.labels });
        let revision = 0i64;

        sqlx::query(
            "INSERT INTO datasets (id, tenant_id, project_id, name, state, revision, metadata, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (id, tenant_id) DO NOTHING",
        )
        .bind(dataset.id.as_uuid())
        .bind(dataset.tenant_id.as_uuid())
        .bind(dataset.project_id.as_uuid())
        .bind(&dataset.name)
        .bind(format!("{:?}", dataset.state))
        .bind(revision)
        .bind(metadata)
        .bind(dataset.created_at.as_offset())
        .bind(dataset.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create dataset: {e}")))?;

        Ok(Versioned::new(dataset, Self::revision_from_i64(revision)))
    }

    async fn get(&self, ctx: &RequestContext, id: DatasetId) -> Result<Versioned<Dataset>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: DatasetRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, name, state, revision, metadata, created_at, updated_at
             FROM datasets
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get dataset: {e}")))?
        .ok_or_else(|| Error::not_found("dataset"))?;

        let versions = self.load_versions(ctx.tenant_id, id).await?;
        let domain = dataset_row_to_domain(&row, versions)?;
        Ok(Versioned::new(
            domain,
            Self::revision_from_i64(row.revision),
        ))
    }

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<Dataset>>, Error> {
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
        let base = format!("FROM datasets WHERE {where_clause}");

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
            .map_err(|e| Error::internal(format!("failed to count datasets: {e}")))?
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

        let rows: Vec<DatasetRow> = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to list datasets: {e}")))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let versions = self.load_versions(ctx.tenant_id, DatasetId::from_uuid(row.id)).await?;
            let domain = dataset_row_to_domain(&row, versions)?;
            items.push(Versioned::new(
                domain,
                Self::revision_from_i64(row.revision),
            ));
        }

        Ok(Page::new(items, total as u64, page))
    }

    async fn update(
        &self,
        ctx: &RequestContext,
        id: DatasetId,
        expected: Revision,
        dataset: Dataset,
    ) -> Result<Versioned<Dataset>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let metadata = serde_json::json!({ "labels": dataset.labels });
        let expected_rev = expected.as_u64() as i64;

        let result = sqlx::query(
            "UPDATE datasets
             SET name = $3, state = $4, metadata = $5, updated_at = $6, revision = revision + 1
             WHERE id = $1 AND tenant_id = $2 AND revision = $7",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(&dataset.name)
        .bind(format!("{:?}", dataset.state))
        .bind(metadata)
        .bind(UtcTimestamp::now().as_offset())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to update dataset: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("dataset revision mismatch"));
        }

        Ok(Versioned::new(dataset, expected.next()))
    }

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: DatasetId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let expected_rev = expected.as_u64() as i64;
        let result =
            sqlx::query("DELETE FROM datasets WHERE id = $1 AND tenant_id = $2 AND revision = $3")
                .bind(id.as_uuid())
                .bind(ctx.tenant_id.as_uuid())
                .bind(expected_rev)
                .execute(&self.pool)
                .await
                .map_err(|e| Error::internal(format!("failed to delete dataset: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("dataset revision mismatch"));
        }

        Ok(())
    }

    async fn create_version(
        &self,
        ctx: &RequestContext,
        dataset_id: DatasetId,
        version: DatasetVersion,
    ) -> Result<Versioned<DatasetVersion>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let manifest = serde_json::json!({
            "assets": version.assets,
            "splits": version.splits,
            "manifest_digest": version.manifest_digest,
        });

        let next_version_number: i64 = sqlx::query(
            "SELECT COALESCE(MAX(version_number), 0) + 1
             FROM dataset_versions
             WHERE tenant_id = $1 AND dataset_id = $2",
        )
        .bind(ctx.tenant_id.as_uuid())
        .bind(dataset_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to compute version number: {e}")))?
        .try_get(0)
        .map_err(|e| Error::internal(format!("failed to read version number: {e}")))?;

        let created_by = match &ctx.principal {
            moqentra_types::Principal::User { id } => id.as_uuid(),
            _ => Uuid::nil(),
        };

        sqlx::query(
            "INSERT INTO dataset_versions
             (id, tenant_id, project_id, dataset_id, version_number, manifest, state, published_at, created_by, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (id, tenant_id) DO NOTHING",
        )
        .bind(version.id.as_uuid())
        .bind(version.tenant_id.as_uuid())
        .bind(version.project_id.as_uuid())
        .bind(dataset_id.as_uuid())
        .bind(next_version_number)
        .bind(manifest)
        .bind(format!("{:?}", version.state))
        .bind(version.published_at.map(|t: UtcTimestamp| t.as_offset()))
        .bind(created_by)
        .bind(version.created_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create dataset version: {e}")))?;

        // Dataset versions are immutable after creation; revision is always 0.
        Ok(Versioned::new(version, Revision::initial()))
    }

    async fn get_version(
        &self,
        ctx: &RequestContext,
        _dataset_id: DatasetId,
        version_id: DatasetVersionId,
    ) -> Result<Versioned<DatasetVersion>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: DatasetVersionRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, dataset_id, version_number, manifest, state,
                    published_at, created_by, created_at
             FROM dataset_versions
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(version_id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get dataset version: {e}")))?
        .ok_or_else(|| Error::not_found("dataset version"))?;

        let domain = version_row_to_domain(row)?;
        Ok(Versioned::new(domain, Revision::initial()))
    }
}

fn dataset_row_to_domain(
    row: &DatasetRow,
    versions: Vec<DatasetVersion>,
) -> Result<Dataset, Error> {
    let labels: std::collections::BTreeMap<String, String> = row
        .metadata
        .get("labels")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let version_ids: Vec<DatasetVersionId> = versions.iter().map(|v| v.id).collect();
    let latest_published_version = versions
        .iter()
        .filter(|v| v.state == DatasetVersionState::Published)
        .max_by_key(|v| v.created_at)
        .map(|v| v.id);

    Ok(Dataset {
        id: DatasetId::from_uuid(row.id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        name: row.name.clone(),
        state: row.state.parse()?,
        version_ids,
        latest_published_version,
        labels,
        created_at: UtcTimestamp::new(row.created_at),
        updated_at: UtcTimestamp::new(row.updated_at),
    })
}

fn version_row_to_domain(row: DatasetVersionRow) -> Result<DatasetVersion, Error> {
    let assets: Vec<AssetRef> = row
        .manifest
        .get("assets")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let splits = row.manifest.get("splits").cloned().unwrap_or(Value::Null);
    let manifest_digest =
        row.manifest.get("manifest_digest").and_then(|v| v.as_str().map(String::from));

    Ok(DatasetVersion {
        id: DatasetVersionId::from_uuid(row.id),
        dataset_id: DatasetId::from_uuid(row.dataset_id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        state: row.state.parse()?,
        assets,
        splits,
        manifest_digest,
        created_at: UtcTimestamp::new(row.created_at),
        published_at: row.published_at.map(UtcTimestamp::new),
    })
}

#[derive(Debug, FromRow)]
struct DatasetRow {
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
struct DatasetVersionRow {
    id: Uuid,
    tenant_id: Uuid,
    project_id: Uuid,
    dataset_id: Uuid,
    _version_number: i64,
    manifest: Value,
    state: String,
    published_at: Option<OffsetDateTime>,
    _created_by: Uuid,
    created_at: OffsetDateTime,
}
