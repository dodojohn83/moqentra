//! Unit-of-work and cursor-based pagination helpers.

use crate::{
    IdempotencyEntry, IdempotencyResult, IdempotencyScope, OutboxEvent, PgAuditLog,
    PgIdempotencyStore, PgOutboxStore,
};
use moqentra_auth::AuditEvent;
use moqentra_types::{Error, OperationId, PageRequest, ProjectId, TenantId, UtcTimestamp};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Opaque cursor for paginated repository queries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub value: Option<String>,
}

impl Cursor {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: Some(value.into()),
        }
    }

    pub fn empty() -> Self {
        Self { value: None }
    }
}

/// A generic paginated response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<Cursor>,
    pub total: u64,
}

/// Builds a pagination SQL clause from a `PageRequest`.
///
/// The returned tuple is `(limit, offset)` suitable for `LIMIT $n OFFSET $n`.
pub fn pagination_clause(req: PageRequest) -> (u64, u64) {
    let limit = req.bounded_limit();
    (u64::from(limit), u64::from(req.offset))
}

/// Trait for a transaction-scoped unit of work.
#[async_trait::async_trait]
pub trait UnitOfWork: Send + Sync {
    async fn commit(self) -> Result<(), Error>;
    async fn rollback(self) -> Result<(), Error>;
}

/// A long-running operation recorded inside a unit of work.
#[derive(Debug, Clone)]
pub struct OperationRecord {
    pub id: OperationId,
    pub project_id: ProjectId,
    pub operation_type: String,
    pub status: String,
    pub progress: f64,
    pub error: Option<serde_json::Value>,
    pub deadline: Option<UtcTimestamp>,
    pub cancelled: bool,
    pub retry_count: i32,
    pub event_sequence: i64,
    pub sse_cursor: Option<String>,
}

/// PostgreSQL-backed unit of work. Aggregate changes, outbox events, audit
/// records, idempotency responses and operation state are committed in the
/// same transaction.
pub struct PgUnitOfWork {
    tx: sqlx::Transaction<'static, sqlx::Postgres>,
    tenant_id: TenantId,
    owner: String,
    outbox: PgOutboxStore,
    audit: PgAuditLog,
    idempotency: PgIdempotencyStore,
}

impl PgUnitOfWork {
    /// Begin a new transaction and set the tenant context for RLS.
    pub async fn begin(
        pool: &sqlx::PgPool,
        tenant_id: TenantId,
        owner: impl Into<String>,
    ) -> Result<Self, Error> {
        let owner = owner.into();
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| Error::internal(format!("failed to begin unit of work: {e}")))?;
        let tenant_str = tenant_id.to_string();
        sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(&tenant_str)
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {e}")))?;

        let outbox = PgOutboxStore::new(pool.clone(), tenant_id, owner.clone());
        let audit = PgAuditLog::new(pool.clone());
        let idempotency = PgIdempotencyStore::new(pool.clone());

        Ok(Self {
            tx,
            tenant_id,
            owner,
            outbox,
            audit,
            idempotency,
        })
    }

    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Access the underlying transaction connection for aggregate changes.
    pub fn connection(&mut self) -> &mut sqlx::PgConnection {
        &mut *self.tx
    }

    /// Append an outbox event within the transaction.
    pub async fn append_outbox(&mut self, event: OutboxEvent) -> Result<OutboxEvent, Error> {
        self.outbox
            .append_with_conn(&mut *self.tx, event)
            .await
            .map_err(|e| Error::internal(format!("unit-of-work outbox append failed: {e}")))
    }

    /// Write an audit record within the transaction.
    pub async fn record_audit(&mut self, event: AuditEvent) -> Result<(), Error> {
        self.audit
            .record_with_conn(&mut *self.tx, event)
            .await
            .map_err(|e| Error::internal(format!("unit-of-work audit record failed: {e}")))
    }

    /// Begin an idempotency scope within the transaction.
    pub async fn idempotency_begin(
        &mut self,
        scope: IdempotencyScope,
        ttl: Duration,
    ) -> Result<IdempotencyResult, Error> {
        self.idempotency
            .begin_with_conn(&mut *self.tx, scope, ttl)
            .await
            .map_err(|e| Error::internal(format!("unit-of-work idempotency begin failed: {e}")))
    }

    /// Complete an idempotency scope within the transaction.
    pub async fn idempotency_complete(
        &mut self,
        entry: &IdempotencyEntry,
        response: serde_json::Value,
    ) -> Result<(), Error> {
        self.idempotency
            .complete_with_conn(&mut *self.tx, entry.scope.tenant_id, entry.id, response)
            .await
            .map_err(|e| Error::internal(format!("unit-of-work idempotency complete failed: {e}")))
    }

    /// Record a long-running operation within the transaction.
    pub async fn record_operation(&mut self, op: OperationRecord) -> Result<(), Error> {
        let tenant_str = self.tenant_id.to_string();
        let _ = sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(&tenant_str)
            .execute(&mut *self.tx)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {e}")))?;
        let _ = sqlx::query(
            "INSERT INTO operations \
             (id, tenant_id, project_id, operation_type, status, progress, error, deadline, \
              cancelled, retry_count, event_sequence, sse_cursor, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now(), now())",
        )
        .bind(op.id.as_uuid())
        .bind(self.tenant_id.as_uuid())
        .bind(op.project_id.as_uuid())
        .bind(&op.operation_type)
        .bind(&op.status)
        .bind(op.progress)
        .bind(&op.error)
        .bind(op.deadline.map(|d| d.as_offset()))
        .bind(op.cancelled)
        .bind(op.retry_count)
        .bind(op.event_sequence)
        .bind(&op.sse_cursor)
        .execute(&mut *self.tx)
        .await
        .map_err(|e| Error::internal(format!("failed to record operation: {e}")))?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl UnitOfWork for PgUnitOfWork {
    async fn commit(self) -> Result<(), Error> {
        self.tx
            .commit()
            .await
            .map_err(|e| Error::internal(format!("unit-of-work commit failed: {e}")))
    }

    async fn rollback(self) -> Result<(), Error> {
        self.tx
            .rollback()
            .await
            .map_err(|e| Error::internal(format!("unit-of-work rollback failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_auth::{AuditCategory, AuditEvent, AuditOutcome};
    use moqentra_types::{Principal, ProjectId, RandomIdGenerator, TenantId, UserId};
    use serde_json::json;
    use std::time::Duration;
    use uuid::Uuid;

    async fn pool() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::PgPool::connect(&url).await.ok()
    }

    async fn ensure_tenant_project(
        pool: &sqlx::PgPool,
        tenant_id: TenantId,
        project_id: ProjectId,
    ) {
        let mut tx = match pool.begin().await {
            Ok(t) => t,
            Err(_) => return,
        };
        let _ = sqlx::query("SELECT set_config('app.is_admin', 'true', true)")
            .execute(&mut *tx)
            .await;
        let _ = sqlx::query("INSERT INTO tenants (id, name, created_at, updated_at) VALUES ($1, 'uow-test', now(), now()) ON CONFLICT (id) DO NOTHING")
            .bind(tenant_id.as_uuid())
            .execute(&mut *tx)
            .await;
        let _ = sqlx::query("INSERT INTO projects (id, tenant_id, name, created_at, updated_at) VALUES ($1, $2, 'uow-test', now(), now()) ON CONFLICT (id) DO NOTHING")
            .bind(project_id.as_uuid())
            .bind(tenant_id.as_uuid())
            .execute(&mut *tx)
            .await;
        let _ = tx.commit().await;
    }

    #[tokio::test]
    async fn pg_unit_of_work_commit_persists_all() {
        let Some(pool) = pool().await else { return };
        let gen = RandomIdGenerator;
        let tenant_id = TenantId::new_v7(&gen);
        let project_id = ProjectId::new_v7(&gen);
        ensure_tenant_project(&pool, tenant_id, project_id).await;

        let mut uow = PgUnitOfWork::begin(&pool, tenant_id, "uow-owner").await.unwrap();

        // Aggregate change: insert an operation directly via the connection.
        let op_id = OperationId::new_v7(&gen);
        let op = OperationRecord {
            id: op_id,
            project_id,
            operation_type: "CreateDataset".to_string(),
            status: "Pending".to_string(),
            progress: 0.0,
            error: None,
            deadline: None,
            cancelled: false,
            retry_count: 0,
            event_sequence: 0,
            sse_cursor: None,
        };
        uow.record_operation(op).await.unwrap();

        // Outbox event.
        let outbox = uow
            .append_outbox(OutboxEvent {
                id: Uuid::new_v4(),
                tenant_id,
                aggregate_type: "dataset".to_string(),
                aggregate_id: Uuid::new_v4().to_string(),
                event_type: "DatasetCreated".to_string(),
                payload: json!({"id": "dataset-1"}),
                status: crate::OutboxStatus::Pending,
                retry_count: 0,
                failure_reason: None,
                created_at: UtcTimestamp::now(),
            })
            .await
            .unwrap();

        // Audit record.
        let audit = AuditEvent {
            event_id: Uuid::new_v4().to_string(),
            category: AuditCategory::Write,
            actor: Principal::User {
                id: UserId::new_v7(&gen),
            },
            tenant_id,
            project_id: Some(project_id),
            action: "create".to_string(),
            resource: "dataset/uow-test".to_string(),
            outcome: AuditOutcome::Success,
            reason: None,
            policy_revision: 1,
            correlation_id: Uuid::new_v4().to_string(),
            request_id: None,
            trace_id: None,
            occurred_at: UtcTimestamp::now(),
            integrity_hash: String::new(),
        };
        uow.record_audit(audit).await.unwrap();

        // Idempotency scope.
        let scope = IdempotencyScope {
            tenant_id,
            key: "uow-key".to_string(),
            operation_type: "CreateDataset".to_string(),
            fingerprint: "fp-1".to_string(),
        };
        let result = uow.idempotency_begin(scope.clone(), Duration::from_secs(60)).await.unwrap();
        assert!(matches!(result, IdempotencyResult::New(_)));

        uow.commit().await.unwrap();

        // Verify outbox event is visible.
        let row: (String,) = sqlx::query_as(
            "SELECT aggregate_type FROM outbox_events WHERE id = $1 AND tenant_id = $2",
        )
        .bind(outbox.id)
        .bind(tenant_id.as_uuid())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "dataset");

        // Verify operation is visible.
        let row: (String,) =
            sqlx::query_as("SELECT status FROM operations WHERE id = $1 AND tenant_id = $2")
                .bind(op_id.as_uuid())
                .bind(tenant_id.as_uuid())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "Pending");

        // Verify idempotency scope persisted.
        let row: (String,) =
            sqlx::query_as("SELECT status FROM idempotency_keys WHERE tenant_id = $1 AND key = $2")
                .bind(tenant_id.as_uuid())
                .bind(&scope.key)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "in_progress");
    }

    #[tokio::test]
    async fn pg_unit_of_work_rollback_discards_all() {
        let Some(pool) = pool().await else { return };
        let gen = RandomIdGenerator;
        let tenant_id = TenantId::new_v7(&gen);
        let project_id = ProjectId::new_v7(&gen);
        ensure_tenant_project(&pool, tenant_id, project_id).await;

        let mut uow = PgUnitOfWork::begin(&pool, tenant_id, "uow-owner").await.unwrap();

        let op_id = OperationId::new_v7(&gen);
        let op = OperationRecord {
            id: op_id,
            project_id,
            operation_type: "CreateDataset".to_string(),
            status: "Pending".to_string(),
            progress: 0.0,
            error: None,
            deadline: None,
            cancelled: false,
            retry_count: 0,
            event_sequence: 0,
            sse_cursor: None,
        };
        uow.record_operation(op).await.unwrap();

        let event_id = Uuid::new_v4();
        uow.append_outbox(OutboxEvent {
            id: event_id,
            tenant_id,
            aggregate_type: "dataset".to_string(),
            aggregate_id: Uuid::new_v4().to_string(),
            event_type: "DatasetCreated".to_string(),
            payload: json!({}),
            status: crate::OutboxStatus::Pending,
            retry_count: 0,
            failure_reason: None,
            created_at: UtcTimestamp::now(),
        })
        .await
        .unwrap();

        uow.rollback().await.unwrap();

        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM operations WHERE id = $1 AND tenant_id = $2")
                .bind(op_id.as_uuid())
                .bind(tenant_id.as_uuid())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count.0, 0);

        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM outbox_events WHERE id = $1 AND tenant_id = $2")
                .bind(event_id)
                .bind(tenant_id.as_uuid())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count.0, 0);
    }
}
