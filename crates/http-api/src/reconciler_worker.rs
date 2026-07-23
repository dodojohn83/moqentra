//! Desired/observed reconcilers for training jobs and model artifacts (R1-REC-002).
//!
//! Recover state from registries (hydrated from PostgreSQL on restart) rather than
//! in-process futures.

use crate::control_plane::{persist_model_version, persist_training_job, AppState};
use crate::worker_runtime::{run_loop, ShutdownFlag, WorkerCursor, WorkerLimits};
use moqentra_application::ArtifactReconciler;
use moqentra_domain::model_registry::ModelVersionState;
use moqentra_domain::training::TrainingJobState;
use std::time::Duration;

/// Spawn training + artifact reconcilers.
pub fn spawn_reconciler_worker(state: AppState) {
    spawn_reconciler_worker_with_shutdown(state, ShutdownFlag::new());
}

/// Same as [`spawn_reconciler_worker`] with shutdown control.
pub fn spawn_reconciler_worker_with_shutdown(state: AppState, shutdown: ShutdownFlag) {
    let limits = WorkerLimits::from_env(
        "reconciler",
        "MOQENTRA_RECONCILER",
        WorkerLimits {
            name: "reconciler",
            batch_size: 32,
            max_concurrency: 1,
            cycle_deadline: Duration::from_secs(45),
            retry_budget: 1,
            idle_sleep: Duration::from_secs(10),
        },
    );

    tokio::spawn(async move {
        let batch = limits.batch_size;
        run_loop(shutdown, limits, || {
            let state = state.clone();
            async move {
                let mut cursor = WorkerCursor::default();
                reconcile_artifacts(&state, batch, &mut cursor).await;
                rehydrate_training_desired_state(&state, batch, &mut cursor).await;
            }
        })
        .await;
    });
}

async fn reconcile_artifacts(state: &AppState, batch: usize, cursor: &mut WorkerCursor) {
    let drafts: Vec<_> = {
        let reg = state.models.lock().unwrap_or_else(|e| e.into_inner());
        reg.draft_versions(batch)
    };

    for version in drafts {
        cursor.advance(version.id.to_string());
        let mut v = version;
        match ArtifactReconciler::reconcile(&mut v) {
            Ok(()) => {
                {
                    let mut reg = state.models.lock().unwrap_or_else(|e| e.into_inner());
                    let reconciled = v.clone();
                    let _ = reg.with_version_mut(v.tenant_id, v.id, |slot| {
                        *slot = reconciled;
                        Ok(())
                    });
                }
                persist_model_version(state, &v).await;
                tracing::debug!(version_id = %v.id, state = ?v.state, "artifact reconciled");
            }
            Err(e) => {
                if matches!(v.state, ModelVersionState::Draft) {
                    tracing::debug!(error = %e, version_id = %v.id, "artifact reconcile pending");
                }
            }
        }
    }
}

/// Re-persist Queued/Admitted training jobs so observed PostgreSQL state matches
/// in-memory desired state after restarts (does not depend on process-local futures).
async fn rehydrate_training_desired_state(
    state: &AppState,
    batch: usize,
    cursor: &mut WorkerCursor,
) {
    let jobs: Vec<_> = {
        let reg = state.training.lock().unwrap_or_else(|e| e.into_inner());
        reg.jobs_in_states(
            &[TrainingJobState::Admitted, TrainingJobState::Queued],
            batch,
        )
    };
    for job in jobs {
        cursor.advance(job.id.to_string());
        persist_training_job(state, &job).await;
    }
}
