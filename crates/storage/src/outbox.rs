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
    pub created_at: UtcTimestamp,
}

/// Outbox store port.
#[async_trait::async_trait]
pub trait OutboxStore: Send + Sync {
    async fn append(&self, event: OutboxEvent) -> Result<OutboxEvent, Error>;
    async fn poll_pending(&self, limit: u32) -> Result<Vec<OutboxEvent>, Error>;
    async fn mark_completed(&self, event_id: Uuid) -> Result<(), Error>;
    async fn mark_failed(&self, event_id: Uuid, reason: String) -> Result<(), Error>;
}

/// In-memory outbox implementation for unit tests.
#[derive(Debug, Clone, Default)]
pub struct InMemoryOutbox {
    events: std::sync::Arc<std::sync::Mutex<Vec<OutboxEvent>>>,
}

impl InMemoryOutbox {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl OutboxStore for InMemoryOutbox {
    async fn append(&self, mut event: OutboxEvent) -> Result<OutboxEvent, Error> {
        if event.id == Uuid::nil() {
            event.id = Uuid::new_v4();
        }
        self.events.lock().expect("lock").push(event.clone());
        Ok(event)
    }

    async fn poll_pending(&self, limit: u32) -> Result<Vec<OutboxEvent>, Error> {
        let mut events = self.events.lock().expect("lock");
        let mut pending: Vec<OutboxEvent> = events
            .iter_mut()
            .filter(|e| e.status == OutboxStatus::Pending)
            .take(limit as usize)
            .map(|e| {
                e.status = OutboxStatus::Processing;
                e.clone()
            })
            .collect();
        for p in &mut pending {
            if let Some(e) = events.iter_mut().find(|e| e.id == p.id) {
                e.status = OutboxStatus::Processing;
            }
        }
        Ok(pending)
    }

    async fn mark_completed(&self, event_id: Uuid) -> Result<(), Error> {
        let mut events = self.events.lock().expect("lock");
        if let Some(e) = events.iter_mut().find(|e| e.id == event_id) {
            e.status = OutboxStatus::Completed;
        }
        Ok(())
    }

    async fn mark_failed(&self, event_id: Uuid, _reason: String) -> Result<(), Error> {
        let mut events = self.events.lock().expect("lock");
        if let Some(e) = events.iter_mut().find(|e| e.id == event_id) {
            e.status = OutboxStatus::Failed;
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
            created_at: UtcTimestamp::now(),
        };
        store.append(event).await.unwrap();
        let pending = store.poll_pending(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, OutboxStatus::Processing);
        store.mark_completed(pending[0].id).await.unwrap();
        let next = store.poll_pending(10).await.unwrap();
        assert!(next.is_empty());
    }
}
