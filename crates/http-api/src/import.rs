//! Import job orchestration for external object sources.
//!
//! Provides an in-memory store and a background worker that downloads objects
//! from a source URL and writes them into the controlled object store with
//! deadline, retry, and digest-based deduplication.

use crate::control_plane::AppState;
use moqentra_domain::import::{ImportJob, ImportJobFailure, ImportJobState};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing;

/// Port for import job persistence.
#[async_trait::async_trait]
pub trait ImportJobStore: Send + Sync {
    /// Save or update a job.
    async fn save(&self, job: &ImportJob) -> Result<(), moqentra_types::Error>;
    /// Get a job by id.
    async fn get(&self, id: &str) -> Result<Option<ImportJob>, moqentra_types::Error>;
    /// List ids of jobs that are not in a terminal state.
    async fn pending_ids(&self) -> Result<Vec<String>, moqentra_types::Error>;
    /// List all jobs.
    async fn list(&self) -> Result<Vec<ImportJob>, moqentra_types::Error>;
}

/// In-memory import job store for single-process deployments.
#[derive(Debug, Default, Clone)]
pub struct InMemoryImportJobStore {
    jobs: Arc<Mutex<BTreeMap<String, ImportJob>>>,
}

impl InMemoryImportJobStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl ImportJobStore for InMemoryImportJobStore {
    async fn save(&self, job: &ImportJob) -> Result<(), moqentra_types::Error> {
        let mut jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        jobs.insert(job.id.clone(), job.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<ImportJob>, moqentra_types::Error> {
        let jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        Ok(jobs.get(id).cloned())
    }

    async fn pending_ids(&self) -> Result<Vec<String>, moqentra_types::Error> {
        let jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        Ok(jobs
            .values()
            .filter(|j| {
                !matches!(
                    j.state,
                    ImportJobState::Completed | ImportJobState::Failed | ImportJobState::Cancelled
                )
            })
            .map(|j| j.id.clone())
            .collect())
    }

    async fn list(&self) -> Result<Vec<ImportJob>, moqentra_types::Error> {
        let jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        Ok(jobs.values().cloned().collect())
    }
}

/// Background import worker. Polls pending jobs and executes server-side
/// transfer with bounded retries.
pub fn spawn_import_worker(state: AppState) {
    tokio::spawn(async move {
        loop {
            let ids = match state.import_jobs.pending_ids().await {
                Ok(ids) => ids,
                Err(e) => {
                    tracing::warn!(error = %e, "import job poll failed");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
            for id in ids {
                if let Ok(Some(job)) = state.import_jobs.get(&id).await {
                    let result = execute_import(&state, &job).await;
                    let mut job = job;
                    if let Err((reason, retryable)) = result {
                        let _ = job.fail(reason);
                        if retryable && job.retry_count < 3 {
                            let _ = job.retry();
                        }
                    } else {
                        let _ = job.complete();
                    }
                    let _ = state.import_jobs.save(&job).await;
                }
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    });
}

async fn execute_import(state: &AppState, job: &ImportJob) -> Result<(), (ImportJobFailure, bool)> {
    if !matches!(job.state, ImportJobState::Pending | ImportJobState::Failed) {
        return Ok(());
    }

    // Transition to inspecting.
    {
        let mut current = state
            .import_jobs
            .get(&job.id)
            .await
            .ok()
            .flatten()
            .ok_or((ImportJobFailure::InvalidSource, false))?;
        if let Err(_e) = current.start_inspection(job.total_bytes) {
            return Err((ImportJobFailure::InvalidSource, false));
        }
        let _ = state.import_jobs.save(&current).await;
    }

    let deadline = Instant::now() + Duration::from_secs(job.deadline_seconds as u64);
    let max_retries = 3;
    let mut attempt = 0;
    let mut last_error = ImportJobFailure::Network;

    while attempt <= max_retries && Instant::now() < deadline {
        attempt += 1;
        match download_and_store(state, job).await {
            Ok(digest) => {
                let mut current = state
                    .import_jobs
                    .get(&job.id)
                    .await
                    .ok()
                    .flatten()
                    .ok_or((ImportJobFailure::InvalidSource, false))?;
                if let Err(_e) = current.start_validation() {
                    return Err((ImportJobFailure::ValidationFailed, false));
                }
                current.digest = Some(digest);
                let _ = state.import_jobs.save(&current).await;
                return Ok(());
            }
            Err(reason) => {
                last_error = reason;
                if Instant::now() >= deadline {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
            }
        }
    }

    Err((last_error, false))
}

async fn download_and_store(state: &AppState, job: &ImportJob) -> Result<String, ImportJobFailure> {
    let mut current = state
        .import_jobs
        .get(&job.id)
        .await
        .ok()
        .flatten()
        .ok_or(ImportJobFailure::InvalidSource)?;
    if current.start_transfer().is_err() {
        return Err(ImportJobFailure::InvalidSource);
    }
    let _ = state.import_jobs.save(&current).await;

    // Check for existing object with same digest (deduplication).
    if let Ok((_, meta)) = state.object_store.get_object(&job.target_key).await {
        if let Some(existing_digest) = meta.digest {
            tracing::info!(
                job_id = %job.id,
                target_key = %job.target_key,
                "skipping transfer: object already exists"
            );
            let _ = current.progress_transfer(job.total_bytes);
            let _ = state.import_jobs.save(&current).await;
            return Ok(existing_digest);
        }
    }

    let timeout = Duration::from_secs((job.deadline_seconds as u64).clamp(30, 300));
    let resp = state
        .http
        .get(&job.source_url)
        .timeout(timeout)
        .send()
        .await
        .map_err(|_| ImportJobFailure::Network)?;

    if !resp.status().is_success() {
        return Err(ImportJobFailure::InvalidSource);
    }

    let content_length = resp.content_length().unwrap_or(0);
    if content_length > job.total_bytes {
        return Err(ImportJobFailure::Oversized);
    }

    let bytes = resp.bytes().await.map_err(|_| ImportJobFailure::Network)?;

    if bytes.len() as u64 != job.total_bytes {
        return Err(ImportJobFailure::ValidationFailed);
    }

    let digest = format!("sha256:{:x}", Sha256::digest(&bytes));

    let mut current = state
        .import_jobs
        .get(&job.id)
        .await
        .ok()
        .flatten()
        .ok_or(ImportJobFailure::InvalidSource)?;
    if current.progress_transfer(bytes.len() as u64).is_err() {
        return Err(ImportJobFailure::Oversized);
    }
    let _ = state.import_jobs.save(&current).await;

    state
        .object_store
        .put_object(&job.target_key, bytes, Some(&job.media_type))
        .await
        .map_err(|_| ImportJobFailure::Network)?;

    Ok(digest)
}
