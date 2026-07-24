//! Multipart upload sessions.
//!
//! A session tracks a multi-part object upload: it records target object key,
//! part size, expected total size, per-part ETag / digest, and expiration.

use crate::ObjectKey;
use bytes::Bytes;
use moqentra_types::{ProjectId, TenantId, UtcTimestamp};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// State of an upload session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadSessionState {
    Pending,
    Completed,
    Aborted,
    Expired,
}

/// Information about a single part upload.
#[derive(Debug, Clone)]
pub struct PartUpload {
    pub part_number: i32,
    pub size: u64,
    pub etag: Option<String>,
    pub digest: Option<String>,
    pub completed: bool,
    pub uploaded_at: Option<UtcTimestamp>,
}

/// A multipart upload session.
#[derive(Debug, Clone)]
pub struct UploadSession {
    pub id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub target_key: ObjectKey,
    pub media_type: String,
    pub part_size: u64,
    pub total_size: u64,
    pub parts: BTreeMap<i32, PartUpload>,
    pub state: UploadSessionState,
    pub expires_at: UtcTimestamp,
    pub created_at: UtcTimestamp,
    /// Backend-specific multipart upload id (e.g. S3 UploadId). Optional for
    /// single-process backends.
    pub backend_upload_id: Option<String>,
}

impl UploadSession {
    /// Maximum number of parts allowed in one upload session.
    pub const MAX_PARTS: i32 = 10_000;

    /// Create a new pending upload session.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        tenant_id: TenantId,
        project_id: ProjectId,
        target_key: ObjectKey,
        media_type: String,
        part_size: u64,
        total_size: u64,
        ttl_seconds: u64,
    ) -> Result<Self, moqentra_types::Error> {
        if part_size == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "part_size must be greater than zero",
            ));
        }
        if total_size == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "total_size must be greater than zero",
            ));
        }
        let now = UtcTimestamp::now();
        let max_parts = total_size.div_ceil(part_size).max(1);
        let max_parts_limit = u64::try_from(Self::MAX_PARTS)
            .map_err(|_| moqentra_types::Error::internal("MAX_PARTS exceeds u64 range"))?;
        if max_parts > max_parts_limit {
            return Err(moqentra_types::Error::invalid_argument(format!(
                "upload would require more than {} parts",
                Self::MAX_PARTS
            )));
        }
        let expires_at = now
            .add_std_duration(std::time::Duration::from_secs(ttl_seconds))
            .ok_or_else(|| moqentra_types::Error::invalid_argument("invalid session ttl"))?;
        let expected_parts = i32::try_from(max_parts).map_err(|_| {
            moqentra_types::Error::invalid_argument("upload part count exceeds i32 range")
        })?;
        let mut parts = BTreeMap::new();
        for n in 1..=expected_parts {
            let size = if n == expected_parts && !total_size.is_multiple_of(part_size) {
                total_size % part_size
            } else {
                part_size
            };
            parts.insert(
                n,
                PartUpload {
                    part_number: n,
                    size,
                    etag: None,
                    digest: None,
                    completed: false,
                    uploaded_at: None,
                },
            );
        }
        Ok(Self {
            id: id.into(),
            tenant_id,
            project_id,
            target_key,
            media_type,
            part_size,
            total_size,
            parts,
            state: UploadSessionState::Pending,
            expires_at,
            created_at: now,
            backend_upload_id: None,
        })
    }

    /// Mark a part as uploaded with its ETag and content digest.
    pub fn mark_part_uploaded(
        &mut self,
        part_number: i32,
        etag: String,
        digest: String,
    ) -> Result<(), moqentra_types::Error> {
        if self.state != UploadSessionState::Pending {
            return Err(moqentra_types::Error::conflict(
                "upload session is not pending",
            ));
        }
        if UtcTimestamp::now() > self.expires_at {
            self.state = UploadSessionState::Expired;
            return Err(moqentra_types::Error::conflict(
                "upload session has expired",
            ));
        }
        let part = self.parts.get_mut(&part_number).ok_or_else(|| {
            moqentra_types::Error::invalid_argument(format!("invalid part number {}", part_number))
        })?;
        if part.completed {
            return Err(moqentra_types::Error::conflict(format!(
                "part {} already uploaded",
                part_number
            )));
        }
        part.etag = Some(etag);
        part.digest = Some(digest);
        part.completed = true;
        part.uploaded_at = Some(UtcTimestamp::now());
        Ok(())
    }

    /// Return the list of parts that have been uploaded so far.
    pub fn uploaded_parts(&self) -> Vec<&PartUpload> {
        self.parts.values().filter(|p| p.completed).collect()
    }

    /// Return true when all expected parts have been uploaded.
    pub fn is_complete(&self) -> bool {
        !self.parts.is_empty() && self.parts.values().all(|p| p.completed)
    }

    /// Validate all expected parts are present and the total size matches.
    pub fn validate_for_completion(&self) -> Result<(), moqentra_types::Error> {
        if self.state != UploadSessionState::Pending {
            return Err(moqentra_types::Error::conflict(
                "upload session is not pending",
            ));
        }
        if UtcTimestamp::now() > self.expires_at {
            return Err(moqentra_types::Error::conflict(
                "upload session has expired",
            ));
        }
        if !self.is_complete() {
            return Err(moqentra_types::Error::invalid_argument(
                "not all parts have been uploaded",
            ));
        }
        let total: u64 = self.parts.values().map(|p| p.size).sum();
        if total != self.total_size {
            return Err(moqentra_types::Error::invalid_argument(
                "uploaded part sizes do not match expected total size",
            ));
        }
        Ok(())
    }
}

/// Port for upload session persistence.
#[async_trait::async_trait]
pub trait UploadSessionStore: Send + Sync {
    /// Save a session.
    async fn save(&self, session: &UploadSession) -> Result<(), moqentra_types::Error>;

    /// Get a session by id.
    async fn get(&self, id: &str) -> Result<Option<UploadSession>, moqentra_types::Error>;

    /// Delete a session.
    async fn delete(&self, id: &str) -> Result<(), moqentra_types::Error>;

    /// List ids of sessions that expired before `before`.
    async fn list_expired(
        &self,
        before: UtcTimestamp,
    ) -> Result<Vec<String>, moqentra_types::Error>;

    /// List all upload sessions.
    async fn list(&self) -> Result<Vec<UploadSession>, moqentra_types::Error>;
}

/// In-memory upload session store for tests and single-process deployments.
#[derive(Debug, Default, Clone)]
pub struct InMemoryUploadSessionStore {
    sessions: Arc<Mutex<BTreeMap<String, UploadSession>>>,
}

impl InMemoryUploadSessionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl UploadSessionStore for InMemoryUploadSessionStore {
    async fn save(&self, session: &UploadSession) -> Result<(), moqentra_types::Error> {
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.insert(session.id.clone(), session.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<UploadSession>, moqentra_types::Error> {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        Ok(sessions.get(id).cloned())
    }

    async fn delete(&self, id: &str) -> Result<(), moqentra_types::Error> {
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.remove(id);
        Ok(())
    }

    async fn list_expired(
        &self,
        before: UtcTimestamp,
    ) -> Result<Vec<String>, moqentra_types::Error> {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        Ok(sessions
            .values()
            .filter(|s| s.expires_at < before)
            .map(|s| s.id.clone())
            .collect())
    }

    async fn list(&self) -> Result<Vec<UploadSession>, moqentra_types::Error> {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        Ok(sessions.values().cloned().collect())
    }
}

/// Compute a content digest for uploaded bytes.
pub fn part_digest(data: &Bytes) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{ProjectId, RandomIdGenerator, TenantId};

    #[test]
    fn session_lifecycle() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let key = ObjectKey::asset(tenant, project, "datasets", "ds-1", "v-1", "data.bin").unwrap();
        let mut session = UploadSession::new(
            "u-1",
            tenant,
            project,
            key,
            "application/octet-stream".into(),
            5,
            12,
            3600,
        )
        .unwrap();
        assert_eq!(session.parts.len(), 3);
        session
            .mark_part_uploaded(
                1,
                "etag-1".into(),
                part_digest(&Bytes::from_static(b"12345")),
            )
            .unwrap();
        session
            .mark_part_uploaded(
                2,
                "etag-2".into(),
                part_digest(&Bytes::from_static(b"67890")),
            )
            .unwrap();
        session
            .mark_part_uploaded(3, "etag-3".into(), part_digest(&Bytes::from_static(b"ab")))
            .unwrap();
        assert!(session.is_complete());
        assert!(session.validate_for_completion().is_ok());
    }

    #[test]
    fn rejects_duplicate_part_upload() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let key = ObjectKey::asset(tenant, project, "datasets", "ds-1", "v-1", "data.bin").unwrap();
        let mut session = UploadSession::new(
            "u-1",
            tenant,
            project,
            key,
            "application/octet-stream".into(),
            5,
            5,
            3600,
        )
        .unwrap();
        session.mark_part_uploaded(1, "a".into(), "d1".into()).unwrap();
        assert!(session.mark_part_uploaded(1, "b".into(), "d2".into()).is_err());
    }
}
