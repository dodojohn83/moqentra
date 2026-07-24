//! PostgreSQL implementation of the application repository port.

use async_trait::async_trait;
use moqentra_application::ports::{ApplicationRepository, ResourceListFilter, Versioned};
use moqentra_domain::application::{
    Application, ApplicationSpec, ApplicationVersion, ApplicationVersionState,
};
use moqentra_types::{
    ApplicationId, ApplicationVersionId, Error, Page, PageRequest, ProjectId, RequestContext,
    Revision, TenantId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// PostgreSQL-backed application repository.
#[derive(Debug, Clone)]
pub struct PgApplicationRepository {
    pool: PgPool,
}

impl PgApplicationRepository {
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
struct ApplicationRow {
    id: Uuid,
    tenant_id: Uuid,
    project_id: Uuid,
    name: String,
    state: String,
    revision: i64,
    graph: Value,
    metadata: Value,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct ApplicationVersionRow {
    id: Uuid,
    tenant_id: Uuid,
    project_id: Uuid,
    application_id: Uuid,
    version_number: i64,
    bundle_digest: String,
    manifest: Value,
    created_by: Uuid,
    state: String,
    updated_at: OffsetDateTime,
    created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApplicationMetadata {
    version_ids: Vec<ApplicationVersionId>,
    latest_published: Option<ApplicationVersionId>,
}

fn app_metadata(app: &Application) -> ApplicationMetadata {
    ApplicationMetadata {
        version_ids: app.version_ids.clone(),
        latest_published: app.latest_published,
    }
}

fn app_row_to_domain(row: &ApplicationRow) -> Result<Application, Error> {
    let _spec: ApplicationSpec = serde_json::from_value(row.graph.clone())
        .map_err(|e| Error::internal(format!("failed to deserialize application graph: {e}")))?;
    let meta: ApplicationMetadata = serde_json::from_value(row.metadata.clone())
        .map_err(|e| Error::internal(format!("failed to deserialize application metadata: {e}")))?;

    Ok(Application {
        id: ApplicationId::from_uuid(row.id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        name: row.name.clone(),
        version_ids: meta.version_ids,
        latest_published: meta.latest_published,
        created_at: UtcTimestamp::new(row.created_at),
        updated_at: UtcTimestamp::new(row.updated_at),
    })
}

fn version_manifest(version: &ApplicationVersion) -> Result<Value, Error> {
    #[derive(Serialize)]
    struct Manifest {
        version: String,
        spec: ApplicationSpec,
        published_at: Option<UtcTimestamp>,
    }
    serde_json::to_value(Manifest {
        version: version.version.clone(),
        spec: version.spec.clone(),
        published_at: version.published_at,
    })
    .map_err(|e| Error::internal(e.to_string()))
}

fn version_row_to_domain(row: &ApplicationVersionRow) -> Result<ApplicationVersion, Error> {
    #[derive(Deserialize)]
    struct Manifest {
        version: String,
        spec: ApplicationSpec,
        published_at: Option<UtcTimestamp>,
    }
    let manifest: Manifest = serde_json::from_value(row.manifest.clone()).map_err(|e| {
        Error::internal(format!(
            "failed to deserialize application version manifest: {e}"
        ))
    })?;
    let state: ApplicationVersionState = row
        .state
        .parse()
        .map_err(|e| Error::internal(format!("invalid stored application version state: {e}")))?;

    Ok(ApplicationVersion {
        id: ApplicationVersionId::from_uuid(row.id),
        application_id: ApplicationId::from_uuid(row.application_id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        version: manifest.version,
        spec: manifest.spec,
        digest: row.bundle_digest.clone(),
        state,
        created_at: UtcTimestamp::new(row.created_at),
        published_at: manifest.published_at,
    })
}

#[async_trait]
impl ApplicationRepository for PgApplicationRepository {
    async fn create(
        &self,
        ctx: &RequestContext,
        application: Application,
    ) -> Result<Versioned<Application>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let revision = 0i64;
        let graph = serde_json::to_value(ApplicationSpec {
            nodes: Default::default(),
            edges: Vec::new(),
        })
        .map_err(|e| Error::internal(e.to_string()))?;
        let metadata = serde_json::to_value(app_metadata(&application))
            .map_err(|e| Error::internal(e.to_string()))?;

        sqlx::query(
            "INSERT INTO applications (id, tenant_id, project_id, name, state, revision, graph, metadata, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(application.id.as_uuid())
        .bind(application.tenant_id.as_uuid())
        .bind(application.project_id.as_uuid())
        .bind(&application.name)
        .bind("Active")
        .bind(revision)
        .bind(graph)
        .bind(metadata)
        .bind(application.created_at.as_offset())
        .bind(application.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create application: {e}")))?;

        Ok(Versioned::new(
            application,
            Self::revision_from_i64(revision)?,
        ))
    }

    async fn get(
        &self,
        ctx: &RequestContext,
        id: ApplicationId,
    ) -> Result<Versioned<Application>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: ApplicationRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, name, state, revision, graph, metadata, created_at, updated_at
             FROM applications
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get application: {e}")))?
        .ok_or_else(|| Error::not_found("application"))?;

        let app = app_row_to_domain(&row)?;
        Ok(Versioned::new(app, Self::revision_from_i64(row.revision)?))
    }

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<Application>>, Error> {
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
        let base = format!("FROM applications WHERE {where_clause}");

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
            .map_err(|e| Error::internal(format!("failed to count applications: {e}")))?
            .try_get(0)
            .map_err(|e| Error::internal(format!("failed to read count: {e}")))?;

        let items_sql = format!(
            "SELECT id, tenant_id, project_id, name, state, revision, graph, metadata, created_at, updated_at
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

        let rows: Vec<ApplicationRow> = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to list applications: {e}")))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let app = app_row_to_domain(&row)?;
            items.push(Versioned::new(app, Self::revision_from_i64(row.revision)?));
        }

        Ok(Page::new(items, total as u64, page))
    }

    async fn update(
        &self,
        ctx: &RequestContext,
        id: ApplicationId,
        expected: Revision,
        application: Application,
    ) -> Result<Versioned<Application>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let expected_rev = expected.as_i64()?;
        let metadata = serde_json::to_value(app_metadata(&application))
            .map_err(|e| Error::internal(e.to_string()))?;

        let result = sqlx::query(
            "UPDATE applications
             SET name = $3, metadata = $4, updated_at = $5, revision = revision + 1
             WHERE id = $1 AND tenant_id = $2 AND revision = $6",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(&application.name)
        .bind(metadata)
        .bind(UtcTimestamp::now().as_offset())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to update application: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("application revision mismatch"));
        }

        Ok(Versioned::new(application, expected.next()))
    }

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: ApplicationId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let expected_rev = expected.as_i64()?;

        let result = sqlx::query(
            "DELETE FROM applications WHERE id = $1 AND tenant_id = $2 AND revision = $3",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to delete application: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("application revision mismatch"));
        }

        Ok(())
    }

    async fn create_version(
        &self,
        ctx: &RequestContext,
        application_id: ApplicationId,
        version: ApplicationVersion,
    ) -> Result<Versioned<ApplicationVersion>, Error> {
        if version.application_id != application_id {
            return Err(Error::invalid_argument("version application_id mismatch"));
        }
        self.set_tenant(ctx.tenant_id).await?;

        let next_version_number: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_number), 0) + 1 FROM application_versions
             WHERE application_id = $1 AND tenant_id = $2",
        )
        .bind(application_id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            Error::internal(format!("failed to compute application version number: {e}"))
        })?;

        let manifest = version_manifest(&version)?;
        let created_by = Uuid::nil();
        let revision = 0i64;

        sqlx::query(
            "INSERT INTO application_versions
             (id, tenant_id, project_id, application_id, version_number, bundle_digest, manifest, created_by, state, updated_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(version.id.as_uuid())
        .bind(version.tenant_id.as_uuid())
        .bind(version.project_id.as_uuid())
        .bind(application_id.as_uuid())
        .bind(next_version_number)
        .bind(&version.digest)
        .bind(manifest)
        .bind(created_by)
        .bind(format!("{:?}", version.state))
        .bind(UtcTimestamp::now().as_offset())
        .bind(version.created_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create application version: {e}")))?;

        Ok(Versioned::new(version, Self::revision_from_i64(revision)?))
    }

    async fn get_version(
        &self,
        ctx: &RequestContext,
        application_id: ApplicationId,
        version_id: ApplicationVersionId,
    ) -> Result<Versioned<ApplicationVersion>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: ApplicationVersionRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, application_id, version_number, bundle_digest,
                    manifest, created_by, state, updated_at, created_at
             FROM application_versions
             WHERE id = $1 AND application_id = $2 AND tenant_id = $3",
        )
        .bind(version_id.as_uuid())
        .bind(application_id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get application version: {e}")))?
        .ok_or_else(|| Error::not_found("application version"))?;

        let version = version_row_to_domain(&row)?;
        Ok(Versioned::new(version, Self::revision_from_i64(0)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{Principal, RandomIdGenerator, UserId};
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

    fn sample_spec() -> ApplicationSpec {
        ApplicationSpec {
            nodes: Default::default(),
            edges: Vec::new(),
        }
    }

    #[tokio::test]
    async fn application_crud_and_version() {
        let Some(pool) = pool().await else { return };
        let (tenant_id, project_id, user_id) = setup_tenant_project_user(&pool).await;
        let c = ctx(tenant_id, project_id, user_id);

        sqlx::query("DELETE FROM application_versions WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM applications WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();

        let repo = PgApplicationRepository::new(pool.clone());
        let app = Application::new(
            ApplicationId::new_v7(&RandomIdGenerator),
            tenant_id,
            project_id,
            "test-application",
        )
        .unwrap();
        let created = repo.create(&c, app.clone()).await.unwrap();
        assert_eq!(created.entity.name, "test-application");

        let fetched = repo.get(&c, created.entity.id).await.unwrap();
        assert_eq!(fetched.entity.id, created.entity.id);

        let version = ApplicationVersion::new(
            ApplicationVersionId::new_v7(&RandomIdGenerator),
            app.id,
            tenant_id,
            project_id,
            "v1",
            sample_spec(),
        )
        .unwrap();
        let v = repo.create_version(&c, app.id, version).await.unwrap();
        assert_eq!(v.entity.state, ApplicationVersionState::Draft);

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
        updated.name = "updated-application".into();
        let versioned =
            repo.update(&c, updated.id, created.revision, updated.clone()).await.unwrap();
        assert_eq!(versioned.revision.as_u64(), 1);

        repo.delete(&c, updated.id, versioned.revision).await.unwrap();
        assert!(repo.get(&c, updated.id).await.is_err());
    }
}
