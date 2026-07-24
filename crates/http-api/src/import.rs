//! Import job orchestration for external object sources.
//!
//! Provides an in-memory store and a background worker that downloads objects
//! from a source URL and writes them into the controlled object store with
//! deadline, retry, and digest-based deduplication.

use crate::control_plane::AppState;
use bytes::BytesMut;
use moqentra_domain::import::{ImportJob, ImportJobFailure, ImportJobState};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
    use crate::worker_runtime::{run_loop, ShutdownFlag, WorkerCursor, WorkerLimits};

    let limits = WorkerLimits::from_env(
        "import",
        "MOQENTRA_IMPORT",
        WorkerLimits {
            name: "import",
            batch_size: 8,
            max_concurrency: 2,
            cycle_deadline: Duration::from_secs(120),
            retry_budget: 3,
            idle_sleep: Duration::from_millis(250),
        },
    );
    let shutdown = ShutdownFlag::new();
    let batch = limits.batch_size;
    let retry_budget = limits.retry_budget;

    tokio::spawn(async move {
        run_loop(shutdown, limits, || {
            let state = state.clone();
            async move {
                let mut cursor = WorkerCursor::default();
                let mut ids = match state.import_jobs.pending_ids().await {
                    Ok(ids) => ids,
                    Err(e) => {
                        tracing::warn!(error = %e, "import job poll failed");
                        return;
                    }
                };
                ids.truncate(batch);
                for id in ids {
                    // Cursor before external transfer side effects.
                    cursor.advance(id.clone());
                    if let Ok(Some(mut job)) = state.import_jobs.get(&id).await {
                        if !matches!(job.state, ImportJobState::Pending | ImportJobState::Failed) {
                            continue;
                        }
                        let result = execute_import(&state, &mut job).await;
                        match result {
                            Ok(()) => {
                                if matches!(job.state, ImportJobState::Validating) {
                                    let _ = job.complete();
                                }
                            }
                            Err((reason, retryable)) => {
                                let _ = job.fail(reason);
                                if retryable && job.retry_count < retry_budget {
                                    let _ = job.retry();
                                }
                            }
                        }
                        let _ = state.import_jobs.save(&job).await;
                    }
                }
            }
        })
        .await;
    });
}

async fn execute_import(
    state: &AppState,
    job: &mut ImportJob,
) -> Result<(), (ImportJobFailure, bool)> {
    if !matches!(job.state, ImportJobState::Pending | ImportJobState::Failed) {
        return Ok(());
    }
    if job.state == ImportJobState::Failed {
        if let Err(_e) = job.retry() {
            return Err((ImportJobFailure::InvalidSource, false));
        }
    }
    if let Err(_e) = job.start_inspection(job.total_bytes) {
        return Err((ImportJobFailure::InvalidSource, false));
    }
    if let Err(_e) = job.start_transfer() {
        return Err((ImportJobFailure::InvalidSource, false));
    }

    match download_and_store(state, job).await {
        Ok(digest) => {
            if let Err(_e) = job.progress_transfer(job.total_bytes) {
                return Err((ImportJobFailure::Oversized, false));
            }
            job.digest = Some(digest);
            if let Err(_e) = job.start_validation() {
                return Err((ImportJobFailure::ValidationFailed, false));
            }
            Ok(())
        }
        Err(reason) => {
            let retryable = matches!(reason, ImportJobFailure::Network);
            Err((reason, retryable))
        }
    }
}

async fn download_and_store(state: &AppState, job: &ImportJob) -> Result<String, ImportJobFailure> {
    // Prevent SSRF through user-supplied import URLs.
    if let Err(e) = crate::northbound::validate_url(&job.source_url) {
        tracing::warn!(job_id = %job.id, source_url = %job.source_url, error = %e, "import source URL rejected");
        return Err(ImportJobFailure::InvalidSource);
    }

    // Check for existing object with same digest (deduplication).
    if let Ok((_, meta)) = state.object_store.get_object(&job.target_key).await {
        if let Some(existing_digest) = meta.digest {
            tracing::info!(
                job_id = %job.id,
                target_key = %job.target_key,
                "skipping transfer: object already exists"
            );
            return Ok(existing_digest);
        }
    }

    let timeout = Duration::from_secs(u64::from(job.deadline_seconds.clamp(30, 300)));
    let mut resp = state
        .http
        .get(&job.source_url)
        .timeout(timeout)
        .send()
        .await
        .map_err(|_| ImportJobFailure::Network)?;

    if !resp.status().is_success() {
        return Err(ImportJobFailure::InvalidSource);
    }

    if resp.content_length().is_some_and(|len| len > job.total_bytes) {
        return Err(ImportJobFailure::Oversized);
    }

    // Stream the body so a missing/malicious Content-Length cannot cause OOM.
    let max_bytes = usize::try_from(job.total_bytes).unwrap_or(usize::MAX);
    let mut buf = BytesMut::new();
    while let Some(chunk) = resp.chunk().await.map_err(|_| ImportJobFailure::Network)? {
        if buf.len().saturating_add(chunk.len()) > max_bytes {
            return Err(ImportJobFailure::Oversized);
        }
        buf.extend_from_slice(&chunk);
    }

    if u64::try_from(buf.len()).map_err(|_| ImportJobFailure::ValidationFailed)? != job.total_bytes
    {
        return Err(ImportJobFailure::ValidationFailed);
    }

    let bytes = buf.freeze();
    let digest = format!("sha256:{:x}", Sha256::digest(&bytes));

    state
        .object_store
        .put_object(&job.target_key, bytes, Some(&job.media_type))
        .await
        .map_err(|_| ImportJobFailure::Network)?;

    Ok(digest)
}
