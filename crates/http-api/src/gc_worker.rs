//! Bounded garbage-collection worker for temporary objects.
//!
//! Collects object keys referenced by dataset versions, active upload sessions,
//! and import jobs, then asks the object store to delete unreferenced objects
//! that are older than the configured TTL and not under legal hold.

use crate::control_plane::AppState;
use crate::worker_runtime::{run_loop, ShutdownFlag, WorkerCursor, WorkerLimits};
use moqentra_object_store::InMemoryObjectStore;
use std::collections::BTreeSet;
use std::time::Duration;
use tracing;

/// Spawn a periodic GC worker. No-op for non-in-memory object stores.
pub fn spawn_gc_worker(state: AppState) {
    spawn_gc_worker_with_shutdown(state, ShutdownFlag::new());
}

/// Same as [`spawn_gc_worker`] with an explicit shutdown flag.
pub fn spawn_gc_worker_with_shutdown(state: AppState, shutdown: ShutdownFlag) {
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
    let dry_run = std::env::var("MOQENTRA_GC_DRY_RUN")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let limits = WorkerLimits {
        name: "gc",
        batch_size: max_delete_per_run,
        max_concurrency: 1,
        cycle_deadline: Duration::from_secs(interval_seconds.max(30)),
        retry_budget: 1,
        idle_sleep: Duration::from_secs(interval_seconds),
    };

    tokio::spawn(async move {
        run_loop(shutdown, limits, || {
            let state = state.clone();
            async move {
                let mut cursor = WorkerCursor::default();
                let mut referenced = BTreeSet::new();

                {
                    let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
                    referenced.extend(reg.referenced_object_keys());
                }
                {
                    let reg = state.models.lock().unwrap_or_else(|e| e.into_inner());
                    referenced.extend(reg.referenced_object_keys());
                }

                if let Ok(sessions) = state.upload_sessions.list().await {
                    for s in sessions {
                        if s.state
                            != moqentra_object_store::upload_session::UploadSessionState::Aborted
                        {
                            referenced.insert(s.target_key.as_str().to_string());
                        }
                    }
                }

                if let Ok(jobs) = state.import_jobs.list().await {
                    for j in jobs {
                        if j.state != moqentra_domain::import::ImportJobState::Failed
                            && j.state != moqentra_domain::import::ImportJobState::Cancelled
                        {
                            referenced.insert(j.target_key);
                        }
                    }
                }

                // Cursor/lease recorded before delete side effects.
                cursor.advance(format!("gc-ref-{}", referenced.len()));

                let any = state.object_store.as_any();
                if let Some(store) = any.downcast_ref::<InMemoryObjectStore>() {
                    let min_age = Duration::from_secs(min_age_seconds);
                    if dry_run {
                        let candidates = store.gc_dry_run(&referenced, min_age, max_delete_per_run);
                        if !candidates.is_empty() {
                            tracing::info!(
                                count = candidates.len(),
                                ?candidates,
                                "gc dry-run would delete objects"
                            );
                        }
                    } else {
                        let removed = store.gc(&referenced, min_age, max_delete_per_run);
                        if removed > 0 {
                            tracing::info!(removed, "gc pass completed");
                        }
                    }
                }
            }
        })
        .await;
    });
}
