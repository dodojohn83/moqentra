//! Idempotency-key storage.

use moqentra_types::{Error, TenantId, UtcTimestamp};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Scope for an idempotency key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdempotencyScope {
    pub tenant_id: TenantId,
    pub key: String,
    pub operation_type: String,
}

/// Stored idempotency entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyEntry {
    pub id: Uuid,
    pub scope: IdempotencyScope,
    pub status: IdempotencyStatus,
    pub response: Option<Value>,
    pub expires_at: UtcTimestamp,
    pub created_at: UtcTimestamp,
}

/// Status of an idempotency entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdempotencyStatus {
    InProgress,
    Completed,
}

/// Result of beginning an idempotent operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyResult {
    /// A new in-progress entry was created.
    New(IdempotencyEntry),
    /// The key already completed; caller should replay the response.
    Completed(IdempotencyEntry),
}

/// Idempotency store port.
#[async_trait::async_trait]
pub trait IdempotencyStore: Send + Sync {
    /// Starts an operation under the key. Returns `Completed` if the key was
    /// already used successfully, `New` if no prior entry exists, or an error
    /// if the key is already in progress.
    async fn begin(
        &self,
        scope: IdempotencyScope,
        ttl: std::time::Duration,
    ) -> Result<IdempotencyResult, Error>;
    /// Marks an in-progress idempotency entry as completed.
    async fn complete(&self, id: Uuid, response: Value) -> Result<(), Error>;
    /// Cleans up expired entries.
    async fn cleanup(&self, before: UtcTimestamp) -> Result<u64, Error>;
}

/// In-memory idempotency store for unit tests.
#[derive(Debug, Clone, Default)]
pub struct InMemoryIdempotencyStore {
    entries: std::sync::Arc<std::sync::Mutex<HashMap<IdempotencyScope, IdempotencyEntry>>>,
}

impl InMemoryIdempotencyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl IdempotencyStore for InMemoryIdempotencyStore {
    async fn begin(
        &self,
        scope: IdempotencyScope,
        ttl: std::time::Duration,
    ) -> Result<IdempotencyResult, Error> {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let now = UtcTimestamp::now();
        if let Some(entry) = entries.get(&scope) {
            if entry.expires_at >= now {
                if entry.status == IdempotencyStatus::Completed {
                    return Ok(IdempotencyResult::Completed(entry.clone()));
                }
                return Err(Error::conflict("idempotency key already in progress"));
            }
        }
        let expires_at = now
            .add_std_duration(ttl)
            .ok_or_else(|| Error::invalid_argument("idempotency ttl is too large"))?;
        let entry = IdempotencyEntry {
            id: Uuid::new_v4(),
            scope: scope.clone(),
            status: IdempotencyStatus::InProgress,
            response: None,
            expires_at,
            created_at: now,
        };
        entries.insert(scope, entry.clone());
        Ok(IdempotencyResult::New(entry))
    }

    async fn complete(&self, id: Uuid, response: Value) -> Result<(), Error> {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        for entry in entries.values_mut() {
            if entry.id == id {
                entry.status = IdempotencyStatus::Completed;
                entry.response = Some(response);
                return Ok(());
            }
        }
        Err(Error::not_found("idempotency entry"))
    }

    async fn cleanup(&self, before: UtcTimestamp) -> Result<u64, Error> {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let before_count = entries.len();
        entries.retain(|_, entry| entry.expires_at > before);
        let removed = before_count - entries.len();
        u64::try_from(removed).map_err(|_| Error::internal("cleanup removed count overflow"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    #[tokio::test]
    async fn idempotency_begin_and_complete() {
        let gen = RandomIdGenerator;
        let store = InMemoryIdempotencyStore::new();
        let scope = IdempotencyScope {
            tenant_id: TenantId::new_v7(&gen),
            key: "key-1".to_string(),
            operation_type: "CreateDataset".to_string(),
        };
        let new = store.begin(scope.clone(), std::time::Duration::from_secs(60)).await.unwrap();
        let IdempotencyResult::New(entry) = new else {
            panic!("expected a new in-progress entry");
        };

        store.complete(entry.id, serde_json::json!({"id": "dataset-1"})).await.unwrap();

        let completed =
            store.begin(scope.clone(), std::time::Duration::from_secs(60)).await.unwrap();
        assert!(matches!(completed, IdempotencyResult::Completed(_)));

        // Different key is independent.
        let mut other = scope.clone();
        other.key = "key-2".to_string();
        let other_result = store.begin(other, std::time::Duration::from_secs(60)).await.unwrap();
        assert!(matches!(other_result, IdempotencyResult::New(_)));
    }

    #[tokio::test]
    async fn idempotency_in_progress_conflict() {
        let gen = RandomIdGenerator;
        let store = InMemoryIdempotencyStore::new();
        let scope = IdempotencyScope {
            tenant_id: TenantId::new_v7(&gen),
            key: "key-3".to_string(),
            operation_type: "CreateDataset".to_string(),
        };
        store.begin(scope.clone(), std::time::Duration::from_secs(60)).await.unwrap();
        assert!(store.begin(scope, std::time::Duration::from_secs(60)).await.is_err());
    }
}
