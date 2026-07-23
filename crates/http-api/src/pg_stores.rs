//! PostgreSQL adapters for upload sessions and import jobs.

use crate::import::ImportJobStore;
use moqentra_domain::import::ImportJob;
use moqentra_object_store::upload_session::{PartUpload, UploadSessionState};
use moqentra_object_store::{ObjectKey, UploadSession, UploadSessionStore};
use moqentra_types::{Error, ProjectId, TenantId, UtcTimestamp};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::BTreeMap;

/// PostgreSQL-backed upload session store.
#[derive(Debug, Clone)]
pub struct PgUploadSessionStore {
    pool: PgPool,
}

impl PgUploadSessionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn set_admin(&self) -> Result<(), Error> {
        sqlx::query("SELECT set_config('app.is_admin', 'true', true)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to set admin context: {e}")))?;
        Ok(())
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
}

#[derive(Debug, Serialize, Deserialize)]
struct UploadSessionPayload {
    id: String,
    tenant_id: TenantId,
    project_id: ProjectId,
    target_key: String,
    media_type: String,
    part_size: u64,
    total_size: u64,
    parts: BTreeMap<i32, PartUploadDto>,
    state: String,
    expires_at: UtcTimestamp,
    created_at: UtcTimestamp,
    backend_upload_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PartUploadDto {
    part_number: i32,
    size: u64,
    etag: Option<String>,
    digest: Option<String>,
    completed: bool,
    uploaded_at: Option<UtcTimestamp>,
}

fn session_to_payload(session: &UploadSession) -> Result<UploadSessionPayload, Error> {
    let mut parts = BTreeMap::new();
    for (k, p) in &session.parts {
        parts.insert(
            *k,
            PartUploadDto {
                part_number: p.part_number,
                size: p.size,
                etag: p.etag.clone(),
                digest: p.digest.clone(),
                completed: p.completed,
                uploaded_at: p.uploaded_at,
            },
        );
    }
    Ok(UploadSessionPayload {
        id: session.id.clone(),
        tenant_id: session.tenant_id,
        project_id: session.project_id,
        target_key: session.target_key.as_str().to_string(),
        media_type: session.media_type.clone(),
        part_size: session.part_size,
        total_size: session.total_size,
        parts,
        state: format!("{:?}", session.state),
        expires_at: session.expires_at,
        created_at: session.created_at,
        backend_upload_id: session.backend_upload_id.clone(),
    })
}

fn payload_to_session(payload: UploadSessionPayload) -> Result<UploadSession, Error> {
    let state = match payload.state.as_str() {
        "Pending" => UploadSessionState::Pending,
        "Completed" => UploadSessionState::Completed,
        "Aborted" => UploadSessionState::Aborted,
        "Expired" => UploadSessionState::Expired,
        other => {
            return Err(Error::internal(format!(
                "unknown upload session state: {other}"
            )))
        }
    };
    let target_key =
        ObjectKey::from_str(payload.tenant_id, payload.project_id, &payload.target_key)
            .map_err(|e| Error::internal(format!("invalid stored object key: {e}")))?;
    let mut parts = BTreeMap::new();
    for (k, p) in payload.parts {
        parts.insert(
            k,
            PartUpload {
                part_number: p.part_number,
                size: p.size,
                etag: p.etag,
                digest: p.digest,
                completed: p.completed,
                uploaded_at: p.uploaded_at,
            },
        );
    }
    Ok(UploadSession {
        id: payload.id,
        tenant_id: payload.tenant_id,
        project_id: payload.project_id,
        target_key,
        media_type: payload.media_type,
        part_size: payload.part_size,
        total_size: payload.total_size,
        parts,
        state,
        expires_at: payload.expires_at,
        created_at: payload.created_at,
        backend_upload_id: payload.backend_upload_id,
    })
}

#[async_trait::async_trait]
impl UploadSessionStore for PgUploadSessionStore {
    async fn save(&self, session: &UploadSession) -> Result<(), Error> {
        self.set_tenant(session.tenant_id).await?;
        let payload = session_to_payload(session)?;
        let payload_json = serde_json::to_value(&payload)
            .map_err(|e| Error::internal(format!("serialize upload session: {e}")))?;
        sqlx::query(
            "INSERT INTO upload_sessions (id, tenant_id, project_id, state, expires_at, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, now())
             ON CONFLICT (id) DO UPDATE SET
                state = EXCLUDED.state,
                expires_at = EXCLUDED.expires_at,
                payload = EXCLUDED.payload,
                updated_at = now()",
        )
        .bind(&session.id)
        .bind(session.tenant_id.as_uuid())
        .bind(session.project_id.as_uuid())
        .bind(format!("{:?}", session.state))
        .bind(session.expires_at.as_offset())
        .bind(payload_json)
        .bind(session.created_at.as_offset())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to save upload session: {e}")))?;
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<UploadSession>, Error> {
        self.set_admin().await?;
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT payload FROM upload_sessions WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Error::internal(format!("failed to get upload session: {e}")))?;
        match row {
            Some((payload,)) => {
                let dto: UploadSessionPayload = serde_json::from_value(payload)
                    .map_err(|e| Error::internal(format!("deserialize upload session: {e}")))?;
                Ok(Some(payload_to_session(dto)?))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, id: &str) -> Result<(), Error> {
        self.set_admin().await?;
        sqlx::query("DELETE FROM upload_sessions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to delete upload session: {e}")))?;
        Ok(())
    }

    async fn list_expired(&self, before: UtcTimestamp) -> Result<Vec<String>, Error> {
        self.set_admin().await?;
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM upload_sessions
             WHERE expires_at < $1 AND state = 'Pending'",
        )
        .bind(before.as_offset())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to list expired upload sessions: {e}")))?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    async fn list(&self) -> Result<Vec<UploadSession>, Error> {
        self.set_admin().await?;
        let rows: Vec<(serde_json::Value,)> =
            sqlx::query_as("SELECT payload FROM upload_sessions ORDER BY created_at")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| Error::internal(format!("failed to list upload sessions: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for (payload,) in rows {
            let dto: UploadSessionPayload = serde_json::from_value(payload)
                .map_err(|e| Error::internal(format!("deserialize upload session: {e}")))?;
            out.push(payload_to_session(dto)?);
        }
        Ok(out)
    }
}

/// PostgreSQL-backed import job store.
#[derive(Debug, Clone)]
pub struct PgImportJobStore {
    pool: PgPool,
}

impl PgImportJobStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn set_admin(&self) -> Result<(), Error> {
        sqlx::query("SELECT set_config('app.is_admin', 'true', true)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::internal(format!("failed to set admin context: {e}")))?;
        Ok(())
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
}

#[async_trait::async_trait]
impl ImportJobStore for PgImportJobStore {
    async fn save(&self, job: &ImportJob) -> Result<(), Error> {
        self.set_tenant(job.tenant_id).await?;
        let payload = serde_json::to_value(job)
            .map_err(|e| Error::internal(format!("serialize import job: {e}")))?;
        sqlx::query(
            "INSERT INTO import_jobs (id, tenant_id, project_id, state, payload, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, now(), now())
             ON CONFLICT (id) DO UPDATE SET
                state = EXCLUDED.state,
                payload = EXCLUDED.payload,
                updated_at = now()",
        )
        .bind(&job.id)
        .bind(job.tenant_id.as_uuid())
        .bind(job.project_id.as_uuid())
        .bind(format!("{:?}", job.state))
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to save import job: {e}")))?;
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<ImportJob>, Error> {
        self.set_admin().await?;
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT payload FROM import_jobs WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Error::internal(format!("failed to get import job: {e}")))?;
        match row {
            Some((payload,)) => {
                let job: ImportJob = serde_json::from_value(payload)
                    .map_err(|e| Error::internal(format!("deserialize import job: {e}")))?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    async fn pending_ids(&self) -> Result<Vec<String>, Error> {
        self.set_admin().await?;
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM import_jobs
             WHERE state NOT IN ('Completed', 'Failed', 'Cancelled')
             ORDER BY updated_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("failed to list pending import jobs: {e}")))?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    async fn list(&self) -> Result<Vec<ImportJob>, Error> {
        self.set_admin().await?;
        let rows: Vec<(serde_json::Value,)> =
            sqlx::query_as("SELECT payload FROM import_jobs ORDER BY updated_at")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| Error::internal(format!("failed to list import jobs: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for (payload,) in rows {
            let job: ImportJob = serde_json::from_value(payload)
                .map_err(|e| Error::internal(format!("deserialize import job: {e}")))?;
            out.push(job);
        }
        Ok(out)
    }
}
