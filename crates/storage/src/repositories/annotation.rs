//! PostgreSQL implementation of the annotation repository port.

use async_trait::async_trait;
use moqentra_application::ports::{AnnotationRepository, ResourceListFilter, Versioned};
use moqentra_domain::annotation::{
    AnnotationProject, AnnotationProjectState, AnnotationTask, AnnotationTaskState, TaskLease,
    TaskType,
};
use moqentra_types::{
    AnnotationProjectId, AnnotationTaskId, AssetId, Error, Page, PageRequest, ProjectId,
    RequestContext, Revision, TenantId, UserId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// PostgreSQL-backed annotation repository.
#[derive(Debug, Clone)]
pub struct PgAnnotationRepository {
    pool: PgPool,
}

impl PgAnnotationRepository {
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
struct AnnotationProjectRow {
    id: Uuid,
    tenant_id: Uuid,
    dataset_id: Option<Uuid>,
    project_id: Uuid,
    dataset_version_id: Option<Uuid>,
    name: String,
    task_type: Option<String>,
    ontology: Value,
    state: String,
    revision: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct AnnotationTaskRow {
    id: Uuid,
    tenant_id: Uuid,
    project_id: Uuid,
    annotation_project_id: Uuid,
    dataset_version_id: Option<Uuid>,
    assignee: Option<Uuid>,
    state: String,
    result: Value,
    revision: i64,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnnotationTaskResult {
    asset_ids: Vec<AssetId>,
    lease: Option<TaskLease>,
}

impl AnnotationTaskResult {
    fn from_task(task: &AnnotationTask) -> Self {
        Self {
            asset_ids: task.asset_ids.clone(),
            lease: task.lease.clone(),
        }
    }
}

fn project_row_to_domain(row: &AnnotationProjectRow) -> Result<AnnotationProject, Error> {
    let task_type = row
        .task_type
        .as_deref()
        .unwrap_or("ImageClassification")
        .parse::<TaskType>()
        .map_err(|e| Error::internal(format!("invalid stored task type: {e}")))?;
    let ontology: moqentra_domain::annotation::Ontology =
        serde_json::from_value(row.ontology.clone()).map_err(|e| {
            Error::internal(format!("failed to deserialize annotation ontology: {e}"))
        })?;
    let state: AnnotationProjectState = row
        .state
        .parse()
        .map_err(|e| Error::internal(format!("invalid stored project state: {e}")))?;

    Ok(AnnotationProject {
        id: AnnotationProjectId::from_uuid(row.id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        dataset_version_id: row
            .dataset_version_id
            .map(moqentra_types::DatasetVersionId::from_uuid)
            .ok_or_else(|| Error::internal("missing dataset_version_id"))?,
        name: row.name.clone(),
        task_type,
        ontology,
        state,
        created_at: UtcTimestamp::new(row.created_at),
        updated_at: UtcTimestamp::new(row.updated_at),
    })
}

fn task_row_to_domain(row: &AnnotationTaskRow) -> Result<AnnotationTask, Error> {
    let result: AnnotationTaskResult = serde_json::from_value(row.result.clone()).map_err(|e| {
        Error::internal(format!("failed to deserialize annotation task result: {e}"))
    })?;
    let state: AnnotationTaskState = row
        .state
        .parse()
        .map_err(|e| Error::internal(format!("invalid stored task state: {e}")))?;

    Ok(AnnotationTask {
        id: AnnotationTaskId::from_uuid(row.id),
        project_id: AnnotationProjectId::from_uuid(row.annotation_project_id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        state,
        assignee: row.assignee.map(UserId::from_uuid),
        asset_ids: result.asset_ids,
        lease: result.lease,
        created_at: UtcTimestamp::new(row.created_at),
        updated_at: UtcTimestamp::new(row.updated_at),
    })
}

#[async_trait]
impl AnnotationRepository for PgAnnotationRepository {
    async fn create_project(
        &self,
        ctx: &RequestContext,
        project: AnnotationProject,
    ) -> Result<Versioned<AnnotationProject>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let revision = 0i64;
        let ontology =
            serde_json::to_value(&project.ontology).map_err(|e| Error::internal(e.to_string()))?;

        sqlx::query(
            "INSERT INTO annotation_projects
             (id, tenant_id, project_id, dataset_version_id, name, task_type, ontology, state, revision, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(project.id.as_uuid())
        .bind(project.tenant_id.as_uuid())
        .bind(project.project_id.as_uuid())
        .bind(project.dataset_version_id.as_uuid())
        .bind(&project.name)
        .bind(format!("{:?}", project.task_type))
        .bind(ontology)
        .bind(format!("{:?}", project.state))
        .bind(revision)
        .bind(project.created_at.as_offset())
        .bind(project.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create annotation project: {e}")))?;

        Ok(Versioned::new(project, Self::revision_from_i64(revision)?))
    }

    async fn get_project(
        &self,
        ctx: &RequestContext,
        id: AnnotationProjectId,
    ) -> Result<Versioned<AnnotationProject>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: AnnotationProjectRow = sqlx::query_as(
            "SELECT id, tenant_id, dataset_id, project_id, dataset_version_id, name, task_type,
                    ontology, state, revision, created_at, updated_at
             FROM annotation_projects
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get annotation project: {e}")))?
        .ok_or_else(|| Error::not_found("annotation project"))?;

        let project = project_row_to_domain(&row)?;
        Ok(Versioned::new(
            project,
            Self::revision_from_i64(row.revision)?,
        ))
    }

    async fn list_projects(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<AnnotationProject>>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let limit = page.limit.clamp(1, 1000) as i64;
        let offset = page.offset as i64;

        let mut conditions = vec!["tenant_id = $1".to_string(), "project_id = $2".to_string()];
        if filter.state.is_some() {
            conditions.push("state = $3".to_string());
        }

        let where_clause = conditions.join(" AND ");
        let base = format!("FROM annotation_projects WHERE {where_clause}");

        let count_sql = format!("SELECT COUNT(*) {base}");
        let mut count_query =
            sqlx::query(&count_sql).bind(ctx.tenant_id.as_uuid()).bind(project_id.as_uuid());
        if let Some(state) = &filter.state {
            count_query = count_query.bind(state);
        }

        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to count annotation projects: {e}")))?
            .try_get(0)
            .map_err(|e| Error::internal(format!("failed to read count: {e}")))?;

        let items_sql = format!(
            "SELECT id, tenant_id, dataset_id, project_id, dataset_version_id, name, task_type,
                    ontology, state, revision, created_at, updated_at
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
        query = query.bind(limit).bind(offset);

        let rows: Vec<AnnotationProjectRow> = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to list annotation projects: {e}")))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let project = project_row_to_domain(&row)?;
            items.push(Versioned::new(
                project,
                Self::revision_from_i64(row.revision)?,
            ));
        }

        Ok(Page::new(items, total as u64, page))
    }

    async fn update_project(
        &self,
        ctx: &RequestContext,
        id: AnnotationProjectId,
        expected: Revision,
        project: AnnotationProject,
    ) -> Result<Versioned<AnnotationProject>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let expected_rev = expected.as_i64()?;
        let ontology =
            serde_json::to_value(&project.ontology).map_err(|e| Error::internal(e.to_string()))?;

        let result = sqlx::query(
            "UPDATE annotation_projects
             SET dataset_version_id = $3, name = $4, task_type = $5, ontology = $6, state = $7,
                 updated_at = $8, revision = revision + 1
             WHERE id = $1 AND tenant_id = $2 AND revision = $9",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(project.dataset_version_id.as_uuid())
        .bind(&project.name)
        .bind(format!("{:?}", project.task_type))
        .bind(ontology)
        .bind(format!("{:?}", project.state))
        .bind(UtcTimestamp::now().as_offset())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to update annotation project: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("annotation project revision mismatch"));
        }

        Ok(Versioned::new(project, expected.next()))
    }

    async fn delete_project(
        &self,
        ctx: &RequestContext,
        id: AnnotationProjectId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let expected_rev = expected.as_i64()?;

        let result = sqlx::query(
            "DELETE FROM annotation_projects WHERE id = $1 AND tenant_id = $2 AND revision = $3",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to delete annotation project: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("annotation project revision mismatch"));
        }

        Ok(())
    }

    async fn create_task(
        &self,
        ctx: &RequestContext,
        project_id: AnnotationProjectId,
        task: AnnotationTask,
    ) -> Result<Versioned<AnnotationTask>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let revision = 0i64;
        let result = serde_json::to_value(AnnotationTaskResult::from_task(&task))
            .map_err(|e| Error::internal(e.to_string()))?;
        let project_uuid = ctx.project_id.map(|p| p.as_uuid()).unwrap_or_else(Uuid::nil);

        sqlx::query(
            "INSERT INTO annotation_tasks
             (id, tenant_id, project_id, annotation_project_id, assignee, state, result, revision, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(task.id.as_uuid())
        .bind(task.tenant_id.as_uuid())
        .bind(project_uuid)
        .bind(project_id.as_uuid())
        .bind(task.assignee.map(|u| u.as_uuid()))
        .bind(format!("{:?}", task.state))
        .bind(result)
        .bind(revision)
        .bind(task.created_at.as_offset())
        .bind(task.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create annotation task: {e}")))?;

        Ok(Versioned::new(task, Self::revision_from_i64(revision)?))
    }

    async fn get_task(
        &self,
        ctx: &RequestContext,
        id: AnnotationTaskId,
    ) -> Result<Versioned<AnnotationTask>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: AnnotationTaskRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, annotation_project_id, dataset_version_id,
                    assignee, state, result, revision, created_at, updated_at
             FROM annotation_tasks
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get annotation task: {e}")))?
        .ok_or_else(|| Error::not_found("annotation task"))?;

        let task = task_row_to_domain(&row)?;
        Ok(Versioned::new(task, Self::revision_from_i64(row.revision)?))
    }

    async fn list_tasks(
        &self,
        ctx: &RequestContext,
        project_id: AnnotationProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<AnnotationTask>>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let limit = page.limit.clamp(1, 1000) as i64;
        let offset = page.offset as i64;

        let mut conditions = vec![
            "tenant_id = $1".to_string(),
            "annotation_project_id = $2".to_string(),
        ];
        if filter.state.is_some() {
            conditions.push("state = $3".to_string());
        }

        let where_clause = conditions.join(" AND ");
        let base = format!("FROM annotation_tasks WHERE {where_clause}");

        let count_sql = format!("SELECT COUNT(*) {base}");
        let mut count_query =
            sqlx::query(&count_sql).bind(ctx.tenant_id.as_uuid()).bind(project_id.as_uuid());
        if let Some(state) = &filter.state {
            count_query = count_query.bind(state);
        }

        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to count annotation tasks: {e}")))?
            .try_get(0)
            .map_err(|e| Error::internal(format!("failed to read count: {e}")))?;

        let items_sql = format!(
            "SELECT id, tenant_id, project_id, annotation_project_id, dataset_version_id,
                    assignee, state, result, revision, created_at, updated_at
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
        query = query.bind(limit).bind(offset);

        let rows: Vec<AnnotationTaskRow> = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to list annotation tasks: {e}")))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let task = task_row_to_domain(&row)?;
            items.push(Versioned::new(task, Self::revision_from_i64(row.revision)?));
        }

        Ok(Page::new(items, total as u64, page))
    }

    async fn update_task(
        &self,
        ctx: &RequestContext,
        id: AnnotationTaskId,
        expected: Revision,
        task: AnnotationTask,
    ) -> Result<Versioned<AnnotationTask>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let expected_rev = expected.as_i64()?;
        let result = serde_json::to_value(AnnotationTaskResult::from_task(&task))
            .map_err(|e| Error::internal(e.to_string()))?;

        let query_result = sqlx::query(
            "UPDATE annotation_tasks
             SET assignee = $3, state = $4, result = $5, updated_at = $6, revision = revision + 1
             WHERE id = $1 AND tenant_id = $2 AND revision = $7",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(task.assignee.map(|u| u.as_uuid()))
        .bind(format!("{:?}", task.state))
        .bind(result)
        .bind(UtcTimestamp::now().as_offset())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to update annotation task: {e}")))?;

        if query_result.rows_affected() != 1 {
            return Err(Error::conflict("annotation task revision mismatch"));
        }

        Ok(Versioned::new(task, expected.next()))
    }
}

impl PgAnnotationRepository {
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

    pub async fn upsert_project(&self, project: &AnnotationProject) -> Result<(), Error> {
        self.set_tenant(project.tenant_id).await?;
        let ontology =
            serde_json::to_value(&project.ontology).map_err(|e| Error::internal(e.to_string()))?;
        sqlx::query(
            "INSERT INTO annotation_projects
             (id, tenant_id, project_id, dataset_version_id, name, task_type, ontology, state, revision, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, $10)
             ON CONFLICT (id) DO UPDATE SET
                dataset_version_id = EXCLUDED.dataset_version_id,
                name = EXCLUDED.name,
                task_type = EXCLUDED.task_type,
                ontology = EXCLUDED.ontology,
                state = EXCLUDED.state,
                updated_at = EXCLUDED.updated_at,
                revision = annotation_projects.revision + 1",
        )
        .bind(project.id.as_uuid())
        .bind(project.tenant_id.as_uuid())
        .bind(project.project_id.as_uuid())
        .bind(project.dataset_version_id.as_uuid())
        .bind(&project.name)
        .bind(format!("{:?}", project.task_type))
        .bind(ontology)
        .bind(format!("{:?}", project.state))
        .bind(project.created_at.as_offset())
        .bind(project.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to upsert annotation project: {e}")))?;
        Ok(())
    }

    pub async fn upsert_task(&self, task: &AnnotationTask) -> Result<(), Error> {
        self.set_tenant(task.tenant_id).await?;
        let result = serde_json::to_value(AnnotationTaskResult::from_task(task))
            .map_err(|e| Error::internal(e.to_string()))?;
        // Prefer parent project's platform project_id when known; fall back to nil.
        let platform_project = sqlx::query_scalar::<_, Uuid>(
            "SELECT project_id FROM annotation_projects WHERE id = $1 AND tenant_id = $2",
        )
        .bind(task.project_id.as_uuid())
        .bind(task.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to resolve annotation project: {e}")))?
        .unwrap_or_else(Uuid::nil);

        sqlx::query(
            "INSERT INTO annotation_tasks
             (id, tenant_id, project_id, annotation_project_id, assignee, state, result, revision, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, $9)
             ON CONFLICT (id) DO UPDATE SET
                assignee = EXCLUDED.assignee,
                state = EXCLUDED.state,
                result = EXCLUDED.result,
                updated_at = EXCLUDED.updated_at,
                revision = annotation_tasks.revision + 1",
        )
        .bind(task.id.as_uuid())
        .bind(task.tenant_id.as_uuid())
        .bind(platform_project)
        .bind(task.project_id.as_uuid())
        .bind(task.assignee.map(|u| u.as_uuid()))
        .bind(format!("{:?}", task.state))
        .bind(result)
        .bind(task.created_at.as_offset())
        .bind(task.updated_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to upsert annotation task: {e}")))?;
        Ok(())
    }

    pub async fn load_all_for_recovery(
        &self,
    ) -> Result<(Vec<AnnotationProject>, Vec<AnnotationTask>), Error> {
        self.set_admin().await?;
        let project_rows: Vec<AnnotationProjectRow> = sqlx::query_as(
            "SELECT id, tenant_id, dataset_id, project_id, dataset_version_id, name, task_type,
                    ontology, state, revision, created_at, updated_at
             FROM annotation_projects
             ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to load annotation projects: {e}")))?;

        let task_rows: Vec<AnnotationTaskRow> = sqlx::query_as(
            "SELECT id, tenant_id, project_id, annotation_project_id, dataset_version_id,
                    assignee, state, result, revision, created_at, updated_at
             FROM annotation_tasks
             ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to load annotation tasks: {e}")))?;

        let mut projects = Vec::with_capacity(project_rows.len());
        for row in project_rows {
            match project_row_to_domain(&row) {
                Ok(p) => projects.push(p),
                Err(e) => tracing::warn!(error = %e, "skipping corrupt annotation project"),
            }
        }
        let mut tasks = Vec::with_capacity(task_rows.len());
        for row in task_rows {
            match task_row_to_domain(&row) {
                Ok(t) => tasks.push(t),
                Err(e) => tracing::warn!(error = %e, "skipping corrupt annotation task"),
            }
        }
        Ok((projects, tasks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PgDatasetRepository;
    use moqentra_application::ports::DatasetRepository;
    use moqentra_domain::annotation::{Label, Ontology};
    use moqentra_domain::dataset::{Dataset, DatasetVersion};
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

    async fn create_dataset_version(
        pool: &PgPool,
        c: &RequestContext,
        tenant_id: TenantId,
        project_id: ProjectId,
    ) -> moqentra_types::DatasetVersionId {
        let dataset_repo = PgDatasetRepository::new(pool.clone());
        sqlx::query("DELETE FROM dataset_versions WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM datasets WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(pool)
            .await
            .unwrap();
        let dataset = Dataset::new(
            moqentra_types::DatasetId::new_v7(&RandomIdGenerator),
            tenant_id,
            project_id,
            "annotation-dataset",
        )
        .unwrap();
        let ds = dataset_repo.create(c, dataset).await.unwrap();
        let version = DatasetVersion::new(
            moqentra_types::DatasetVersionId::new_v7(&RandomIdGenerator),
            ds.entity.id,
            tenant_id,
            project_id,
        );
        let v = dataset_repo.create_version(c, ds.entity.id, version).await.unwrap();
        v.entity.id
    }

    #[tokio::test]
    async fn annotation_project_and_task_crud() {
        let Some(pool) = pool().await else { return };
        let (tenant_id, project_id, user_id) = setup_tenant_project(&pool).await;
        let c = ctx(tenant_id, project_id, user_id);
        let version_id = create_dataset_version(&pool, &c, tenant_id, project_id).await;

        let repo = PgAnnotationRepository::new(pool.clone());
        sqlx::query("DELETE FROM annotation_tasks WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM annotation_projects WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();

        let ontology = Ontology::new(
            vec![Label {
                id: "label-1".into(),
                name: "object".into(),
                color: "#000000".into(),
                parent_id: None,
                metadata: serde_json::Value::Null,
            }],
            vec!["bbox".into()],
        );
        let project = AnnotationProject::new(
            AnnotationProjectId::new_v7(&RandomIdGenerator),
            tenant_id,
            project_id,
            version_id,
            "annotation-project",
            TaskType::BoundingBox,
            ontology,
        )
        .unwrap();
        let created = repo.create_project(&c, project.clone()).await.unwrap();
        assert_eq!(created.entity.name, "annotation-project");

        let fetched = repo.get_project(&c, created.entity.id).await.unwrap();
        assert_eq!(fetched.entity.id, created.entity.id);

        let listed = repo
            .list_projects(
                &c,
                project_id,
                ResourceListFilter::default(),
                PageRequest::default(),
            )
            .await
            .unwrap();
        assert_eq!(listed.items.len(), 1);

        let asset_id = moqentra_types::AssetId::new_v7(&RandomIdGenerator);
        let task = AnnotationTask::new(
            AnnotationTaskId::new_v7(&RandomIdGenerator),
            created.entity.id,
            tenant_id,
            vec![asset_id],
        )
        .unwrap();
        let created_task = repo.create_task(&c, created.entity.id, task).await.unwrap();
        assert_eq!(created_task.entity.state, AnnotationTaskState::Pending);

        let fetched_task = repo.get_task(&c, created_task.entity.id).await.unwrap();
        assert_eq!(fetched_task.entity.id, created_task.entity.id);

        let listed_tasks = repo
            .list_tasks(
                &c,
                created.entity.id,
                ResourceListFilter::default(),
                PageRequest::default(),
            )
            .await
            .unwrap();
        assert_eq!(listed_tasks.items.len(), 1);

        repo.delete_project(&c, created.entity.id, created.revision).await.unwrap();
        assert!(repo.get_project(&c, created.entity.id).await.is_err());
    }
}
