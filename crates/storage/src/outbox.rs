//! Outbox event store.

use moqentra_types::{Error, TenantId, UtcTimestamp};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

/// Status of an outbox event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl fmt::Display for OutboxStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutboxStatus::Pending => f.write_str("pending"),
            OutboxStatus::Processing => f.write_str("processing"),
            OutboxStatus::Completed => f.write_str("completed"),
            OutboxStatus::Failed => f.write_str("failed"),
        }
    }
}

/// An outbox event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: Value,
    pub status: OutboxStatus,
    pub retry_count: u32,
    pub failure_reason: Option<String>,
    pub created_at: UtcTimestamp,
}

/// Outbox store port.
#[async_trait::async_trait]
pub trait OutboxStore: Send + Sync {
    async fn append(&self, event: OutboxEvent) -> Result<OutboxEvent, Error>;
    /// Claim up to `limit` pending (or lease-expired) events for processing.
    async fn poll_pending(&self, limit: u32) -> Result<Vec<OutboxEvent>, Error>;
    /// Non-mutating view of pending/processing events for admin/list APIs.
    async fn list_events(&self, limit: u32) -> Result<Vec<OutboxEvent>, Error>;
    async fn mark_completed(&self, event_id: Uuid) -> Result<(), Error>;
    async fn mark_failed(&self, event_id: Uuid, reason: String) -> Result<(), Error>;
}

/// Lease duration for in-memory processing claims before they are reclaimed.
const IN_MEMORY_LEASE_SECS: i64 = 30;
const IN_MEMORY_MAX_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone)]
struct InMemoryEntry {
    event: OutboxEvent,
    /// When the current processing lease expires; `None` if not processing.
    lease_expires_at: Option<std::time::Instant>,
    next_retry_at: Option<std::time::Instant>,
}

/// In-memory outbox implementation for unit tests and local demos.
#[derive(Debug, Clone, Default)]
pub struct InMemoryOutbox {
    events: std::sync::Arc<std::sync::Mutex<Vec<InMemoryEntry>>>,
}

impl InMemoryOutbox {
    pub fn new() -> Self {
        Self::default()
    }

    fn reclaim_expired(entries: &mut [InMemoryEntry]) {
        let now = std::time::Instant::now();
        for entry in entries.iter_mut() {
            if entry.event.status == OutboxStatus::Processing
                && entry.lease_expires_at.is_none_or(|exp| exp <= now)
            {
                entry.event.status = OutboxStatus::Pending;
                entry.lease_expires_at = None;
                entry.next_retry_at = None;
            }
        }
    }
}

#[async_trait::async_trait]
impl OutboxStore for InMemoryOutbox {
    async fn append(&self, mut event: OutboxEvent) -> Result<OutboxEvent, Error> {
        if event.id == Uuid::nil() {
            event.id = Uuid::new_v4();
        }
        self.events.lock().unwrap_or_else(|e| e.into_inner()).push(InMemoryEntry {
            event: event.clone(),
            lease_expires_at: None,
            next_retry_at: None,
        });
        Ok(event)
    }

    async fn poll_pending(&self, limit: u32) -> Result<Vec<OutboxEvent>, Error> {
        let limit =
            usize::try_from(limit).map_err(|_| Error::invalid_argument("limit too large"))?;
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        Self::reclaim_expired(&mut events);
        let now = std::time::Instant::now();
        let mut claimed = Vec::new();
        for entry in events.iter_mut() {
            if claimed.len() >= limit {
                break;
            }
            let retry_ready = entry.next_retry_at.is_none_or(|t| t <= now);
            if entry.event.status == OutboxStatus::Pending && retry_ready {
                entry.event.status = OutboxStatus::Processing;
                entry.event.retry_count = entry.event.retry_count.saturating_add(1);
                entry.lease_expires_at =
                    Some(now + std::time::Duration::from_secs(IN_MEMORY_LEASE_SECS as u64));
                entry.next_retry_at = None;
                claimed.push(entry.event.clone());
            }
        }
        Ok(claimed)
    }

    async fn list_events(&self, limit: u32) -> Result<Vec<OutboxEvent>, Error> {
        let limit =
            usize::try_from(limit).map_err(|_| Error::invalid_argument("limit too large"))?;
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        Self::reclaim_expired(&mut events);
        Ok(events
            .iter()
            .filter(|e| {
                matches!(
                    e.event.status,
                    OutboxStatus::Pending | OutboxStatus::Processing | OutboxStatus::Failed
                )
            })
            .take(limit)
            .map(|e| e.event.clone())
            .collect())
    }

    async fn mark_completed(&self, event_id: Uuid) -> Result<(), Error> {
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = events.iter_mut().find(|e| e.event.id == event_id) {
            entry.event.status = OutboxStatus::Completed;
            entry.lease_expires_at = None;
            entry.next_retry_at = None;
        }
        Ok(())
    }

    async fn mark_failed(&self, event_id: Uuid, reason: String) -> Result<(), Error> {
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = events.iter_mut().find(|e| e.event.id == event_id) {
            entry.event.failure_reason = Some(reason);
            entry.lease_expires_at = None;
            if entry.event.retry_count >= IN_MEMORY_MAX_ATTEMPTS {
                entry.event.status = OutboxStatus::Failed;
                entry.next_retry_at = None;
            } else {
                entry.event.status = OutboxStatus::Pending;
                let backoff_secs = 1u64 << entry.event.retry_count.min(8);
                entry.next_retry_at =
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(backoff_secs));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{RandomIdGenerator, TenantId};

    #[tokio::test]
    async fn outbox_append_and_poll() {
        let gen = RandomIdGenerator;
        let store = InMemoryOutbox::new();
        let event = OutboxEvent {
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
        };
        store.append(event).await.unwrap();
        let listed = store.list_events(10).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, OutboxStatus::Pending);
        let pending = store.poll_pending(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, OutboxStatus::Processing);
        store.mark_completed(pending[0].id).await.unwrap();
        let next = store.poll_pending(10).await.unwrap();
        assert!(next.is_empty());
        assert!(store.list_events(10).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn outbox_processing_lease_is_reclaimed() {
        let gen = RandomIdGenerator;
        let store = InMemoryOutbox::new();
        let event = OutboxEvent {
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
        };
        store.append(event).await.unwrap();
        let claimed = store.poll_pending(10).await.unwrap();
        assert_eq!(claimed.len(), 1);
        // Force lease expiry.
        {
            let mut events = store.events.lock().unwrap();
            for e in events.iter_mut() {
                e.lease_expires_at =
                    Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
            }
        }
        let reclaimed = store.poll_pending(10).await.unwrap();
        assert_eq!(reclaimed.len(), 1);
        assert_eq!(reclaimed[0].id, claimed[0].id);
    }
}
