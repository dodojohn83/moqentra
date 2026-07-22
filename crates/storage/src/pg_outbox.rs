//! PostgreSQL-backed outbox event store.

use crate::outbox::{OutboxEvent, OutboxStatus, OutboxStore};
use moqentra_types::{Error, TenantId, UtcTimestamp};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
struct OutboxRow {
    id: Uuid,
    tenant_id: Uuid,
    aggregate_type: String,
    aggregate_id: String,
    event_type: String,
    payload: serde_json::Value,
    status: String,
    retry_count: i32,
    failure_reason: Option<String>,
    lease_owner: Option<String>,
    lease_expires_at: Option<OffsetDateTime>,
    next_retry_at: Option<OffsetDateTime>,
    max_attempts: i32,
    created_at: OffsetDateTime,
    updated_at: Option<OffsetDateTime>,
}

impl TryFrom<OutboxRow> for OutboxEvent {
    type Error = Error;

    fn try_from(row: OutboxRow) -> Result<Self, Self::Error> {
        Ok(OutboxEvent {
            id: row.id,
            tenant_id: TenantId::from_uuid(row.tenant_id),
            aggregate_type: row.aggregate_type,
            aggregate_id: row.aggregate_id,
            event_type: row.event_type,
            payload: row.payload,
            status: parse_status(&row.status)?,
            retry_count: row.retry_count as u32,
            failure_reason: row.failure_reason,
            created_at: UtcTimestamp::new(row.created_at),
        })
    }
}

fn status_str(status: OutboxStatus) -> &'static str {
    match status {
        OutboxStatus::Pending => "pending",
        OutboxStatus::Processing => "processing",
        OutboxStatus::Completed => "completed",
        OutboxStatus::Failed => "failed",
    }
}

fn parse_status(s: &str) -> Result<OutboxStatus, Error> {
    match s {
        "pending" => Ok(OutboxStatus::Pending),
        "processing" => Ok(OutboxStatus::Processing),
        "completed" => Ok(OutboxStatus::Completed),
        "failed" => Ok(OutboxStatus::Failed),
        _ => Err(Error::internal(format!("unknown outbox status: {s}"))),
    }
}

/// PostgreSQL outbox store with bounded `FOR UPDATE SKIP LOCKED` polling,
/// lease tracking, exponential retry backoff and dead-letter handling.
#[derive(Debug, Clone)]
pub struct PgOutboxStore {
    pool: PgPool,
    tenant_id: TenantId,
    owner: String,
}

impl PgOutboxStore {
    pub fn new(pool: PgPool, tenant_id: TenantId, owner: impl Into<String>) -> Self {
        Self {
            pool,
            tenant_id,
            owner: owner.into(),
        }
    }

    /// Append an outbox event using an existing connection/transaction.
    pub async fn append_with_conn(
        &self,
        conn: &mut sqlx::PgConnection,
        event: OutboxEvent,
    ) -> Result<OutboxEvent, Error> {
        if event.tenant_id != self.tenant_id {
            return Err(Error::invalid_argument(
                "event tenant does not match outbox store tenant",
            ));
        }
        let tenant_id = event.tenant_id;
        let tenant_str = tenant_id.to_string();
        let status = status_str(event.status);
        let created_at = event.created_at.as_offset();
        let payload = event.payload.clone();

        let _ = sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(&tenant_str)
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {e}")))?;
        let _ = sqlx::query(
            "INSERT INTO outbox_events \
             (id, tenant_id, aggregate_type, aggregate_id, event_type, payload, status, \
              retry_count, failure_reason, lease_owner, lease_expires_at, next_retry_at, \
              max_attempts, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL, NULL, $10, $11, $12)",
        )
        .bind(event.id)
        .bind(tenant_id.as_uuid())
        .bind(&event.aggregate_type)
        .bind(&event.aggregate_id)
        .bind(&event.event_type)
        .bind(payload)
        .bind(status)
        .bind(i32::try_from(event.retry_count).unwrap_or(0))
        .bind(event.failure_reason.as_ref())
        .bind(created_at)
        .bind(3i32)
        .bind(created_at)
        .execute(&mut *conn)
        .await
        .map_err(|e| Error::internal(format!("outbox append failed: {e}")))?;
        Ok(event)
    }
}

#[async_trait::async_trait]
impl OutboxStore for PgOutboxStore {
    async fn append(&self, event: OutboxEvent) -> Result<OutboxEvent, Error> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| Error::internal(format!("outbox acquire failed: {e}")))?;
        self.append_with_conn(&mut *conn, event).await
    }

    async fn poll_pending(&self, limit: u32) -> Result<Vec<OutboxEvent>, Error> {
        let tenant_id = self.tenant_id;
        let tenant_str = tenant_id.to_string();
        let limit = i64::from(limit);
        let owner = &self.owner;
        let lease_seconds: i32 = 30;

        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| Error::internal(format!("outbox acquire failed: {e}")))?;
        let _ = sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(&tenant_str)
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {e}")))?;
        let rows: Vec<OutboxRow> = sqlx::query_as(
            "WITH candidates AS (
                SELECT id
                FROM outbox_events
                WHERE status = 'pending'
                  AND (next_retry_at IS NULL OR next_retry_at <= now())
                  AND (lease_expires_at IS NULL OR lease_expires_at < now())
                ORDER BY tenant_id, created_at
                FOR UPDATE SKIP LOCKED
                LIMIT $1
            )
            UPDATE outbox_events
            SET status = 'processing',
                lease_owner = $2,
                lease_expires_at = now() + make_interval(secs => $3),
                retry_count = retry_count + 1,
                updated_at = now()
            FROM candidates
            WHERE outbox_events.id = candidates.id
            RETURNING outbox_events.*",
        )
        .bind(limit)
        .bind(owner)
        .bind(lease_seconds)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| Error::internal(format!("outbox poll failed: {e}")))?;
        rows.into_iter().map(OutboxEvent::try_from).collect::<Result<Vec<_>, _>>()
    }

    async fn mark_completed(&self, event_id: Uuid) -> Result<(), Error> {
        let tenant_str = self.tenant_id.to_string();
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| Error::internal(format!("outbox acquire failed: {e}")))?;
        let _ = sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(&tenant_str)
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {e}")))?;
        let result = sqlx::query(
            "UPDATE outbox_events \
             SET status = 'completed', lease_owner = NULL, lease_expires_at = NULL, \
                 next_retry_at = NULL, updated_at = now() \
             WHERE id = $1",
        )
        .bind(event_id)
        .execute(&mut *conn)
        .await
        .map_err(|e| Error::internal(format!("outbox mark completed failed: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(Error::not_found("outbox event"));
        }
        Ok(())
    }

    async fn mark_failed(&self, event_id: Uuid, reason: String) -> Result<(), Error> {
        let tenant_str = self.tenant_id.to_string();
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| Error::internal(format!("outbox acquire failed: {e}")))?;
        let _ = sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(&tenant_str)
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {e}")))?;
        let result = sqlx::query(
            "UPDATE outbox_events \
             SET failure_reason = $2, \
                 updated_at = now(), \
                 status = CASE \
                     WHEN retry_count >= max_attempts THEN 'failed' \
                     ELSE 'pending' \
                 END, \
                 next_retry_at = CASE \
                     WHEN retry_count >= max_attempts THEN NULL \
                     ELSE now() + make_interval(secs => GREATEST(1, LEAST(300, power(2, retry_count)::int))) \
                 END, \
                 lease_owner = NULL, lease_expires_at = NULL \
                 WHERE id = $1",
        )
        .bind(event_id)
        .bind(&reason)
        .execute(&mut *conn)
        .await
        .map_err(|e| Error::internal(format!("outbox mark failed failed: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(Error::not_found("outbox event"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{RandomIdGenerator, TenantId};

    async fn pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPool::connect(&url).await.ok()
    }

    fn sample_event() -> OutboxEvent {
        let gen = RandomIdGenerator;
        OutboxEvent {
            id: Uuid::new_v4(),
            tenant_id: TenantId::new_v7(&gen),
            aggregate_type: "dataset".to_string(),
            aggregate_id: "dataset-1".to_string(),
            event_type: "DatasetCreated".to_string(),
            payload: serde_json::json!({}),
            status: OutboxStatus::Pending,
            retry_count: 0,
            failure_reason: None,
            created_at: UtcTimestamp::now(),
        }
    }

    #[tokio::test]
    async fn pg_outbox_append_poll_and_complete() {
        let Some(pool) = pool().await else { return };
        sqlx::query("DELETE FROM outbox_events").execute(&pool).await.unwrap();
        let event = sample_event();
        let store = PgOutboxStore::new(pool, event.tenant_id, "test-owner");
        let appended = store.append(event).await.unwrap();
        let pending = store.poll_pending(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, appended.id);
        assert_eq!(pending[0].status, OutboxStatus::Processing);
        store.mark_completed(pending[0].id).await.unwrap();
        let next = store.poll_pending(10).await.unwrap();
        assert!(next.is_empty());
    }

    #[tokio::test]
    async fn pg_outbox_retry_then_dead_letter() {
        let Some(pool) = pool().await else { return };
        sqlx::query("DELETE FROM outbox_events").execute(&pool).await.unwrap();
        let event = sample_event();
        let tenant_id = event.tenant_id;
        let store = PgOutboxStore::new(pool, tenant_id, "test-owner");
        store.append(event).await.unwrap();
        for _ in 0..3 {
            let pending = store.poll_pending(10).await.unwrap();
            assert_eq!(pending.len(), 1);
            store.mark_failed(pending[0].id, "boom".to_string()).await.unwrap();
            // Reset retry time to allow the next loop to re-acquire immediately.
            sqlx::query("UPDATE outbox_events SET next_retry_at = now()")
                .execute(&store.pool)
                .await
                .unwrap();
        }
        let pending = store.poll_pending(10).await.unwrap();
        assert!(pending.is_empty());

        let rows: Vec<OutboxRow> =
            sqlx::query_as("SELECT * FROM outbox_events WHERE status = 'failed' LIMIT 1")
                .fetch_all(&store.pool)
                .await
                .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].failure_reason.as_deref(), Some("boom"));
    }
}
