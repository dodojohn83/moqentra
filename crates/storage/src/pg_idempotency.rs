//! PostgreSQL-backed idempotency store.

use crate::idempotency::{
    IdempotencyEntry, IdempotencyResult, IdempotencyScope, IdempotencyStatus, IdempotencyStore,
};
use moqentra_types::{Error, TenantId, UtcTimestamp};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
struct IdempotencyRow {
    tenant_id: Uuid,
    operation_type: String,
    key: String,
    fingerprint: Option<String>,
    response: Option<Value>,
    status: String,
    expires_at: OffsetDateTime,
    created_at: OffsetDateTime,
    id: Uuid,
}

impl TryFrom<IdempotencyRow> for IdempotencyEntry {
    type Error = Error;

    fn try_from(row: IdempotencyRow) -> Result<Self, Self::Error> {
        Ok(IdempotencyEntry {
            id: row.id,
            scope: IdempotencyScope {
                tenant_id: TenantId::from_uuid(row.tenant_id),
                key: row.key,
                operation_type: row.operation_type,
                fingerprint: row.fingerprint.unwrap_or_default(),
            },
            status: parse_status(&row.status)?,
            response: row.response,
            expires_at: UtcTimestamp::new(row.expires_at),
            created_at: UtcTimestamp::new(row.created_at),
        })
    }
}

fn parse_status(s: &str) -> Result<IdempotencyStatus, Error> {
    match s {
        "in_progress" => Ok(IdempotencyStatus::InProgress),
        "completed" => Ok(IdempotencyStatus::Completed),
        _ => Err(Error::internal(format!("unknown idempotency status: {s}"))),
    }
}

/// PostgreSQL idempotency store with request-fingerprint conflict detection,
/// in-progress locking, response replay, TTL and safe cleanup.
#[derive(Debug, Clone)]
pub struct PgIdempotencyStore {
    pool: PgPool,
}

impl PgIdempotencyStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn evaluate_existing(
        row: IdempotencyRow,
        scope: &IdempotencyScope,
    ) -> Result<Option<IdempotencyResult>, Error> {
        if row.expires_at > OffsetDateTime::now_utc() {
            let entry: IdempotencyEntry = row.try_into()?;
            if entry.scope.fingerprint != scope.fingerprint {
                return Err(Error::conflict(
                    "idempotency key reused with different request fingerprint",
                ));
            }
            if entry.status == IdempotencyStatus::Completed {
                return Ok(Some(IdempotencyResult::Completed(entry)));
            }
            return Err(Error::conflict("idempotency key already in progress"));
        }
        Ok(None)
    }

    /// Begin an idempotency scope on an existing connection/transaction.
    pub async fn begin_with_conn(
        &self,
        conn: &mut sqlx::PgConnection,
        scope: IdempotencyScope,
        ttl: std::time::Duration,
    ) -> Result<IdempotencyResult, Error> {
        let tenant_id = scope.tenant_id;
        let tenant_str = tenant_id.to_string();
        let operation_type = &scope.operation_type;
        let key = &scope.key;
        let fingerprint = &scope.fingerprint;

        // Set tenant for RLS and avoid cross-tenant leakage.
        sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(&tenant_str)
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {e}")))?;

        let now = UtcTimestamp::now();
        let expires_at = now
            .add_std_duration(ttl)
            .ok_or_else(|| Error::invalid_argument("idempotency ttl is too large"))?;

        // Lock the row if it exists so concurrent requests for the same key are
        // serialized. After deleting an expired row we still rely on the primary
        // key, so we handle a unique-constraint race by re-reading.
        let existing: Option<IdempotencyRow> = sqlx::query_as(
            "SELECT * FROM idempotency_keys \
             WHERE tenant_id = $1 AND operation_type = $2 AND key = $3 \
             FOR UPDATE",
        )
        .bind(tenant_id.as_uuid())
        .bind(operation_type)
        .bind(key)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| Error::internal(format!("idempotency lookup failed: {e}")))?;

        if let Some(row) = existing {
            if let Some(result) = Self::evaluate_existing(row, &scope)? {
                return Ok(result);
            }
            // Expired entry: delete so it can be replaced.
            sqlx::query(
                "DELETE FROM idempotency_keys \
                 WHERE tenant_id = $1 AND operation_type = $2 AND key = $3",
            )
            .bind(tenant_id.as_uuid())
            .bind(operation_type)
            .bind(key)
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::internal(format!("idempotency delete failed: {e}")))?;
        }

        let id = Uuid::new_v4();
        let insert = sqlx::query(
            "INSERT INTO idempotency_keys \
             (tenant_id, operation_type, key, fingerprint, response, status, expires_at, created_at, id) \
             VALUES ($1, $2, $3, $4, NULL, 'in_progress', $5, $6, $7)",
        )
        .bind(tenant_id.as_uuid())
        .bind(operation_type)
        .bind(key)
        .bind(fingerprint)
        .bind(expires_at.as_offset())
        .bind(now.as_offset())
        .bind(id)
        .execute(&mut *conn)
        .await;

        if let Err(sqlx::Error::Database(db_err)) = &insert {
            if db_err.constraint() == Some("idempotency_keys_pkey") {
                // Another transaction inserted first. Re-read the committed row.
                let row: Option<IdempotencyRow> = sqlx::query_as(
                    "SELECT * FROM idempotency_keys \
                     WHERE tenant_id = $1 AND operation_type = $2 AND key = $3 \
                     FOR UPDATE",
                )
                .bind(tenant_id.as_uuid())
                .bind(operation_type)
                .bind(key)
                .fetch_optional(&mut *conn)
                .await
                .map_err(|e| Error::internal(format!("idempotency re-read failed: {e}")))?;
                if let Some(row) = row {
                    if let Some(result) = Self::evaluate_existing(row, &scope)? {
                        return Ok(result);
                    }
                    // The row was inserted by another transaction but has since
                    // expired before we could lock it; delete and retry once.
                    sqlx::query(
                        "DELETE FROM idempotency_keys \
                         WHERE tenant_id = $1 AND operation_type = $2 AND key = $3",
                    )
                    .bind(tenant_id.as_uuid())
                    .bind(operation_type)
                    .bind(key)
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| Error::internal(format!("idempotency delete failed: {e}")))?;

                    let id2 = Uuid::new_v4();
                    let _ = sqlx::query(
                        "INSERT INTO idempotency_keys \
                         (tenant_id, operation_type, key, fingerprint, response, status, expires_at, created_at, id) \
                         VALUES ($1, $2, $3, $4, NULL, 'in_progress', $5, $6, $7)",
                    )
                    .bind(tenant_id.as_uuid())
                    .bind(operation_type)
                    .bind(key)
                    .bind(fingerprint)
                    .bind(expires_at.as_offset())
                    .bind(now.as_offset())
                    .bind(id2)
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| Error::internal(format!("idempotency insert failed: {e}")))?;
                    let entry = IdempotencyEntry {
                        id: id2,
                        scope,
                        status: IdempotencyStatus::InProgress,
                        response: None,
                        expires_at,
                        created_at: now,
                    };
                    return Ok(IdempotencyResult::New(entry));
                }
            }
        }

        let _ = insert.map_err(|e| Error::internal(format!("idempotency insert failed: {e}")))?;

        let entry = IdempotencyEntry {
            id,
            scope,
            status: IdempotencyStatus::InProgress,
            response: None,
            expires_at,
            created_at: now,
        };
        Ok(IdempotencyResult::New(entry))
    }

    /// Complete an in-progress idempotency entry on an existing connection/transaction.
    pub async fn complete_with_conn(
        &self,
        conn: &mut sqlx::PgConnection,
        tenant_id: TenantId,
        id: Uuid,
        response: Value,
    ) -> Result<(), Error> {
        let tenant_str = tenant_id.to_string();
        let _ = sqlx::query("SELECT set_config('app.current_tenant', $1, true)")
            .bind(&tenant_str)
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::internal(format!("failed to set tenant context: {e}")))?;
        let result = sqlx::query(
            "UPDATE idempotency_keys \
             SET status = 'completed', response = $2 \
             WHERE id = $1",
        )
        .bind(id)
        .bind(response)
        .execute(&mut *conn)
        .await
        .map_err(|e| Error::internal(format!("idempotency complete failed: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(Error::not_found("idempotency entry"));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl IdempotencyStore for PgIdempotencyStore {
    async fn begin(
        &self,
        scope: IdempotencyScope,
        ttl: std::time::Duration,
    ) -> Result<IdempotencyResult, Error> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::internal(format!("idempotency begin failed: {e}")))?;
        let result = self.begin_with_conn(&mut *tx, scope, ttl).await?;
        tx.commit()
            .await
            .map_err(|e| Error::internal(format!("idempotency commit failed: {e}")))?;
        Ok(result)
    }

    async fn complete(&self, tenant_id: TenantId, id: Uuid, response: Value) -> Result<(), Error> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| Error::internal(format!("idempotency acquire failed: {e}")))?;
        self.complete_with_conn(&mut *conn, tenant_id, id, response).await
    }

    async fn cleanup(&self, before: UtcTimestamp) -> Result<u64, Error> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| Error::internal(format!("idempotency acquire failed: {e}")))?;
        let _ = sqlx::query("SELECT set_config('app.is_admin', 'true', true)")
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::internal(format!("failed to set admin context: {e}")))?;
        let result = sqlx::query("DELETE FROM idempotency_keys WHERE expires_at <= $1")
            .bind(before.as_offset())
            .execute(&mut *conn)
            .await
            .map_err(|e| Error::internal(format!("idempotency cleanup failed: {e}")))?;
        Ok(result.rows_affected())
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

    fn scope(key: &str, fingerprint: &str) -> IdempotencyScope {
        let gen = RandomIdGenerator;
        IdempotencyScope {
            tenant_id: TenantId::new_v7(&gen),
            key: key.to_string(),
            operation_type: "CreateDataset".to_string(),
            fingerprint: fingerprint.to_string(),
        }
    }

    #[tokio::test]
    async fn pg_idempotency_begin_complete_and_replay() {
        let Some(pool) = pool().await else { return };
        sqlx::query("DELETE FROM idempotency_keys").execute(&pool).await.unwrap();
        let store = PgIdempotencyStore::new(pool);
        let scope = scope("key-a", "fp-1");
        let result = store.begin(scope.clone(), std::time::Duration::from_secs(60)).await.unwrap();
        let IdempotencyResult::New(entry) = result else {
            panic!("expected new entry");
        };

        store
            .complete(
                entry.scope.tenant_id,
                entry.id,
                serde_json::json!({"id": "dataset-1"}),
            )
            .await
            .unwrap();

        let replay = store.begin(scope.clone(), std::time::Duration::from_secs(60)).await.unwrap();
        assert!(matches!(replay, IdempotencyResult::Completed(_)));
    }

    #[tokio::test]
    async fn pg_idempotency_fingerprint_conflict() {
        let Some(pool) = pool().await else { return };
        sqlx::query("DELETE FROM idempotency_keys").execute(&pool).await.unwrap();
        let store = PgIdempotencyStore::new(pool);
        let mut scope = scope("key-b", "fp-1");
        store.begin(scope.clone(), std::time::Duration::from_secs(60)).await.unwrap();
        scope.fingerprint = "fp-2".to_string();
        assert!(store.begin(scope, std::time::Duration::from_secs(60)).await.is_err());
    }

    #[tokio::test]
    async fn pg_idempotency_in_progress_conflict() {
        let Some(pool) = pool().await else { return };
        sqlx::query("DELETE FROM idempotency_keys").execute(&pool).await.unwrap();
        let store = PgIdempotencyStore::new(pool);
        let scope = scope("key-c", "fp-1");
        store.begin(scope.clone(), std::time::Duration::from_secs(60)).await.unwrap();
        assert!(store.begin(scope, std::time::Duration::from_secs(60)).await.is_err());
    }
}
