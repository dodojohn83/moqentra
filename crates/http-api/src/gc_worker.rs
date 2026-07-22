//! Bounded garbage-collection worker for temporary objects.
//!
//! Collects object keys referenced by dataset versions, active upload sessions,
//! and import jobs, then asks the object store to delete unreferenced objects
//! that are older than the configured TTL and not under legal hold.

use crate::control_plane::AppState;
use moqentra_object_store::InMemoryObjectStore;
use std::collections::BTreeSet;
use std::time::Duration;
use tracing;

/// Spawn a periodic GC worker. No-op for non-in-memory object stores.
pub fn spawn_gc_worker(state: AppState) {
    let min_age_seconds: u64 = std::env::var("MOQENTRA_GC_MIN_AGE_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600);
    let max_delete_per_run: usize = std::env::var("MOQENTRA_GC_MAX_DELETE_PER_RUN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let interval_seconds: u64 = std::env::var("MOQENTRA_GC_INTERVAL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(interval_seconds)).await;
            let mut referenced = BTreeSet::new();

            // Dataset version assets.
            {
                let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
                referenced.extend(reg.referenced_object_keys());
            }

            // Active upload sessions target keys.
            if let Ok(sessions) = state.upload_sessions.list().await {
                for s in sessions {
                    if s.state != moqentra_object_store::upload_session::UploadSessionState::Aborted
                    {
                        referenced.insert(s.target_key.as_str().to_string());
                    }
                }
            }

            // Import job target keys.
            if let Ok(jobs) = state.import_jobs.list().await {
                for j in jobs {
                    if j.state != moqentra_domain::import::ImportJobState::Failed
                        && j.state != moqentra_domain::import::ImportJobState::Cancelled
                    {
                        referenced.insert(j.target_key);
                    }
                }
            }

            // Run GC only for the in-memory backend; S3 lifecycle is configured
            // outside the control plane.
            let any = state.object_store.as_any();
            if let Some(store) = any.downcast_ref::<InMemoryObjectStore>() {
                let removed = store.gc(
                    &referenced,
                    Duration::from_secs(min_age_seconds),
                    max_delete_per_run,
                );
                if removed > 0 {
                    tracing::info!(removed, "gc pass completed");
                }
            }
        }
    });
}
