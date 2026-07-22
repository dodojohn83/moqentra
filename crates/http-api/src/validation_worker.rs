//! Background media validation worker.
//!
//! Scans dataset versions for pending asset validations, downloads each object,
//! and applies `MediaValidator` results so that failed assets cannot be published.

use crate::control_plane::AppState;
use std::time::Duration;

/// Spawn a worker that periodically validates pending dataset assets.
pub fn spawn_media_validation_worker(state: AppState) {
    tokio::spawn(async move {
        loop {
            let pending = {
                let reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
                reg.pending_validations()
            };

            for (tenant_id, version_id, asset_id, _asset_name, object_key, media_type) in pending {
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

                let mut reg = state.datasets.lock().unwrap_or_else(|e| e.into_inner());
                let _ = match result {
                    Ok(_) => reg.mark_asset_valid(tenant_id, version_id, &asset_id),
                    Err(reason) => reg.mark_asset_failed(tenant_id, version_id, &asset_id, reason),
                };
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}
