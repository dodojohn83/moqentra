//! Background media validation worker.
//!
//! Scans dataset versions for pending asset validations, downloads each object,
//! and applies `MediaValidator` results so that failed assets cannot be published.
//! Results are written through to PostgreSQL when configured.

use crate::control_plane::{persist_dataset_version, AppState};
use crate::worker_runtime::{run_loop, ShutdownFlag, WorkerCursor, WorkerLimits};
use moqentra_types::{Principal, RequestContext};
use std::time::Duration;

/// Spawn a worker that periodically validates pending dataset assets.
pub fn spawn_media_validation_worker(state: AppState) {
    spawn_media_validation_worker_with_shutdown(state, ShutdownFlag::new());
}

/// Same as [`spawn_media_validation_worker`] with an explicit shutdown flag.
pub fn spawn_media_validation_worker_with_shutdown(state: AppState, shutdown: ShutdownFlag) {
    let limits = WorkerLimits::from_env(
        "media-validation",
        "MOQENTRA_VALIDATION",
        WorkerLimits {
            name: "media-validation",
            batch_size: 16,
            max_concurrency: 4,
            cycle_deadline: Duration::from_secs(60),
            retry_budget: 2,
            idle_sleep: Duration::from_secs(5),
        },
    );

    tokio::spawn(async move {
        let batch = limits.batch_size;
        run_loop(shutdown, limits, || {
            let state = state.clone();
            async move {
                let mut cursor = WorkerCursor::default();
                let pending = {
                    let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
                    let mut all = reg.pending_validations();
                    all.truncate(batch);
                    all
                };

                for (tenant_id, version_id, asset_id, _asset_name, object_key, media_type) in
                    pending
                {
                    // Record cursor before external object-store side effect.
                    cursor.advance(format!("{tenant_id}:{version_id}:{asset_id}"));

                    let result = async {
                        let (bytes, _) = state
                            .object_store
                            .get_object(&object_key)
                            .await
                            .map_err(|e| e.to_string())?;
                        state
                            .media_validator
                            .validate(&bytes, &media_type)
                            .await
                            .map_err(|e| e.to_string())
                    }
                    .await;

                    let version = {
                        let mut reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
                        let _ = match &result {
                            Ok(_) => reg.mark_asset_valid(tenant_id, version_id, &asset_id),
                            Err(reason) => {
                                reg.mark_asset_failed(tenant_id, version_id, &asset_id, reason)
                            }
                        };
                        reg.get_version(tenant_id, version_id).ok().cloned()
                    };

                    if let Some(v) = version {
                        let ctx = RequestContext::new(
                            tenant_id,
                            Principal::Anonymous,
                            format!("validation-worker-{}", cursor.processed),
                        )
                        .with_project(v.project_id);
                        persist_dataset_version(&state, &ctx, &v).await;
                    }
                }
            }
        })
        .await;
    });
}
