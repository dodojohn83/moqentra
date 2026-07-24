//! PostgreSQL implementation of the deployment repository port.

use async_trait::async_trait;
use moqentra_application::ports::{DeploymentRepository, ResourceListFilter, Versioned};
use moqentra_domain::inference::{Deployment, DeploymentState};
use moqentra_types::{
    DeploymentId, Error, Page, PageRequest, ProjectId, RequestContext, Revision, TenantId,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Row};
use time::OffsetDateTime;

/// PostgreSQL-backed deployment repository.
#[derive(Debug, Clone)]
pub struct PgDeploymentRepository {
    pool: PgPool,
}

impl PgDeploymentRepository {
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
struct DeploymentRow {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    project_id: uuid::Uuid,
    application_version_id: uuid::Uuid,
    target_cluster_id: Option<uuid::Uuid>,
    name: String,
    state: String,
    revision: i64,
    spec: Value,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeploymentSpec {
    release_bundle: Value,
    desired_replicas: u32,
    observed_generation: u64,
    replicas: Value,
}

fn deployment_spec(deployment: &Deployment) -> Result<Value, Error> {
    serde_json::to_value(DeploymentSpec {
        release_bundle: serde_json::to_value(&deployment.release_bundle)
            .map_err(|e| Error::internal(e.to_string()))?,
        desired_replicas: deployment.desired_replicas,
        observed_generation: deployment.observed_generation,
        replicas: serde_json::to_value(&deployment.replicas)
            .map_err(|e| Error::internal(e.to_string()))?,
    })
    .map_err(|e| Error::internal(e.to_string()))
}

fn row_to_domain(row: &DeploymentRow) -> Result<Deployment, Error> {
    let state: DeploymentState = row
        .state
        .parse()
        .map_err(|e| Error::internal(format!("invalid stored deployment state: {e}")))?;
    let spec: DeploymentSpec = serde_json::from_value(row.spec.clone())
        .map_err(|e| Error::internal(format!("failed to deserialize deployment spec: {e}")))?;
    let release_bundle = serde_json::from_value(spec.release_bundle)
        .map_err(|e| Error::internal(format!("failed to deserialize release bundle: {e}")))?;
    let replicas = serde_json::from_value(spec.replicas)
        .map_err(|e| Error::internal(format!("failed to deserialize replicas: {e}")))?;

    Ok(Deployment {
        id: DeploymentId::from_uuid(row.id),
        tenant_id: TenantId::from_uuid(row.tenant_id),
        project_id: ProjectId::from_uuid(row.project_id),
        release_bundle,
        desired_replicas: spec.desired_replicas,
        observed_generation: spec.observed_generation,
        desired_state: state,
        replicas,
    })
}

#[async_trait]
impl DeploymentRepository for PgDeploymentRepository {
    async fn create(
        &self,
        ctx: &RequestContext,
        deployment: Deployment,
    ) -> Result<Versioned<Deployment>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        deployment
            .release_bundle
            .policy
            .validate()
            .map_err(|e| Error::invalid_argument(e.to_string()))?;
        let revision = 0i64;
        let spec = deployment_spec(&deployment)?;
        let now = UtcTimestamp::now();

        sqlx::query(
            "INSERT INTO deployments
             (id, tenant_id, project_id, application_version_id, target_cluster_id, name, state, revision, spec, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(deployment.id.as_uuid())
        .bind(deployment.tenant_id.as_uuid())
        .bind(deployment.project_id.as_uuid())
        .bind(deployment.release_bundle.application_version_id.as_uuid())
        .bind(Option::<uuid::Uuid>::None)
        .bind(deployment.id.to_string())
        .bind(format!("{:?}", deployment.desired_state))
        .bind(revision)
        .bind(spec)
        .bind(now.as_offset())
        .bind(now.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to create deployment: {e}")))?;

        Ok(Versioned::new(
            deployment,
            Self::revision_from_i64(revision)?,
        ))
    }

    async fn get(
        &self,
        ctx: &RequestContext,
        id: DeploymentId,
    ) -> Result<Versioned<Deployment>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let row: DeploymentRow = sqlx::query_as(
            "SELECT id, tenant_id, project_id, application_version_id, target_cluster_id, name,
                    state, revision, spec, created_at, updated_at
             FROM deployments
             WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to get deployment: {e}")))?
        .ok_or_else(|| Error::not_found("deployment"))?;

        let deployment = row_to_domain(&row)?;
        Ok(Versioned::new(
            deployment,
            Self::revision_from_i64(row.revision)?,
        ))
    }

    async fn list(
        &self,
        ctx: &RequestContext,
        project_id: ProjectId,
        filter: ResourceListFilter,
        page: PageRequest,
    ) -> Result<Page<Versioned<Deployment>>, Error> {
        self.set_tenant(ctx.tenant_id).await?;

        let limit = page.limit.clamp(1, 1000) as i64;
        let offset = page.offset as i64;

        let mut conditions = vec!["tenant_id = $1".to_string(), "project_id = $2".to_string()];
        if filter.state.is_some() {
            conditions.push("state = $3".to_string());
        }

        let where_clause = conditions.join(" AND ");
        let base = format!("FROM deployments WHERE {where_clause}");

        let count_sql = format!("SELECT COUNT(*) {base}");
        let mut count_query =
            sqlx::query(&count_sql).bind(ctx.tenant_id.as_uuid()).bind(project_id.as_uuid());
        if let Some(state) = &filter.state {
            count_query = count_query.bind(state);
        }

        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to count deployments: {e}")))?
            .try_get(0)
            .map_err(|e| Error::internal(format!("failed to read count: {e}")))?;

        let items_sql = format!(
            "SELECT id, tenant_id, project_id, application_version_id, target_cluster_id, name,
                    state, revision, spec, created_at, updated_at
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

        let rows: Vec<DeploymentRow> = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to list deployments: {e}")))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let deployment = row_to_domain(&row)?;
            items.push(Versioned::new(
                deployment,
                Self::revision_from_i64(row.revision)?,
            ));
        }

        Ok(Page::new(items, total as u64, page))
    }

    async fn update(
        &self,
        ctx: &RequestContext,
        id: DeploymentId,
        expected: Revision,
        deployment: Deployment,
    ) -> Result<Versioned<Deployment>, Error> {
        self.set_tenant(ctx.tenant_id).await?;
        deployment
            .release_bundle
            .policy
            .validate()
            .map_err(|e| Error::invalid_argument(e.to_string()))?;
        let expected_rev = expected.as_i64()?;
        let spec = deployment_spec(&deployment)?;

        let result = sqlx::query(
            "UPDATE deployments
             SET application_version_id = $3, state = $4, spec = $5, updated_at = $6, revision = revision + 1
             WHERE id = $1 AND tenant_id = $2 AND revision = $7",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(deployment.release_bundle.application_version_id.as_uuid())
        .bind(format!("{:?}", deployment.desired_state))
        .bind(spec)
        .bind(UtcTimestamp::now().as_offset())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to update deployment: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("deployment revision mismatch"));
        }

        Ok(Versioned::new(deployment, expected.next()))
    }

    async fn delete(
        &self,
        ctx: &RequestContext,
        id: DeploymentId,
        expected: Revision,
    ) -> Result<(), Error> {
        self.set_tenant(ctx.tenant_id).await?;
        let expected_rev = expected.as_i64()?;

        let result = sqlx::query(
            "DELETE FROM deployments WHERE id = $1 AND tenant_id = $2 AND revision = $3",
        )
        .bind(id.as_uuid())
        .bind(ctx.tenant_id.as_uuid())
        .bind(expected_rev)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to delete deployment: {e}")))?;

        if result.rows_affected() != 1 {
            return Err(Error::conflict("deployment revision mismatch"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PgApplicationRepository;
    use moqentra_application::ports::ApplicationRepository;
    use moqentra_domain::application::{Application, ApplicationSpec, ApplicationVersion};
    use moqentra_domain::inference::{ReleaseBundle, RolloutPolicy, RolloutStrategy};
    use moqentra_types::{
        ApplicationVersionId, ModelVersionId, Principal, RandomIdGenerator, UserId,
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

    fn make_deployment(
        tenant_id: TenantId,
        project_id: ProjectId,
        application_version_id: ApplicationVersionId,
    ) -> Deployment {
        let gen = RandomIdGenerator;
        Deployment {
            id: DeploymentId::new_v7(&gen),
            tenant_id,
            project_id,
            release_bundle: ReleaseBundle {
                version: "v1".into(),
                model_version_id: ModelVersionId::new_v7(&gen),
                application_version_id,
                dyun_bundle_digest:
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
                policy: RolloutPolicy {
                    strategy: RolloutStrategy::RollingUpdate,
                    auto_rollback: false,
                    slo_error_rate: 0.01,
                    min_available_percent: 50,
                    max_surge_percent: 25,
                },
                signature:
                    "sha256:0000000000000000000000000000000000000000000000000000000000000001".into(),
            },
            desired_replicas: 1,
            observed_generation: 1,
            desired_state: DeploymentState::Pending,
            replicas: Default::default(),
        }
    }

    #[tokio::test]
    async fn deployment_crud() {
        let Some(pool) = pool().await else { return };
        let (tenant_id, project_id, user_id) = setup_tenant_project_user(&pool).await;
        let c = ctx(tenant_id, project_id, user_id);

        sqlx::query("DELETE FROM deployments WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .execute(&pool)
            .await
            .unwrap();

        let app_repo = PgApplicationRepository::new(pool.clone());
        let app = Application::new(
            moqentra_types::ApplicationId::new_v7(&RandomIdGenerator),
            tenant_id,
            project_id,
            "deployment-app",
        )
        .unwrap();
        let _created_app = app_repo.create(&c, app.clone()).await.unwrap();
        let version = ApplicationVersion::new(
            ApplicationVersionId::new_v7(&RandomIdGenerator),
            app.id,
            tenant_id,
            project_id,
            "v1",
            ApplicationSpec {
                nodes: Default::default(),
                edges: Vec::new(),
            },
        )
        .unwrap();
        let created_version = app_repo.create_version(&c, app.id, version).await.unwrap();

        let repo = PgDeploymentRepository::new(pool.clone());
        let deployment = make_deployment(tenant_id, project_id, created_version.entity.id);
        let created = repo.create(&c, deployment.clone()).await.unwrap();
        assert_eq!(created.entity.id, deployment.id);

        let fetched = repo.get(&c, deployment.id).await.unwrap();
        assert_eq!(fetched.entity.desired_replicas, deployment.desired_replicas);

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
        updated.desired_replicas = 3;
        let versioned =
            repo.update(&c, updated.id, created.revision, updated.clone()).await.unwrap();
        assert_eq!(versioned.revision.as_u64(), 1);

        repo.delete(&c, updated.id, versioned.revision).await.unwrap();
        assert!(repo.get(&c, updated.id).await.is_err());
    }
}
