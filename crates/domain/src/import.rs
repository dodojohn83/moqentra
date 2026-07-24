//! Import job state machine.

use moqentra_types::{ProjectId, TenantId};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// State of an import job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportJobState {
    Pending,
    Inspecting,
    Transferring,
    Validating,
    Completed,
    Failed,
    Cancelled,
}

/// Reason for an import failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportJobFailure {
    Network,
    InvalidSource,
    Oversized,
    MalwareDetected,
    DigestConflict,
    ValidationFailed,
    Cancelled,
}

/// Import job aggregate root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportJob {
    pub id: String,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub source_url: String,
    pub target_key: String,
    pub media_type: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub concurrency: u32,
    pub deadline_seconds: u32,
    pub digest: Option<String>,
    pub state: ImportJobState,
    pub failure: Option<ImportJobFailure>,
    pub retry_count: u32,
}

impl Default for ImportJob {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportJob {
    pub fn new() -> Self {
        Self {
            id: String::new(),
            tenant_id: TenantId::from_str("00000000-0000-0000-0000-000000000000")
                .unwrap_or_else(|_| unreachable!("nil UUID parses")),
            project_id: ProjectId::from_str("00000000-0000-0000-0000-000000000000")
                .unwrap_or_else(|_| unreachable!("nil UUID parses")),
            source_url: String::new(),
            target_key: String::new(),
            media_type: String::new(),
            total_bytes: 0,
            transferred_bytes: 0,
            concurrency: 1,
            deadline_seconds: 300,
            digest: None,
            state: ImportJobState::Pending,
            failure: None,
            retry_count: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_v1(
        id: String,
        tenant_id: TenantId,
        project_id: ProjectId,
        source_url: String,
        target_key: String,
        media_type: String,
        total_bytes: u64,
        concurrency: u32,
        deadline_seconds: u32,
    ) -> Result<Self, moqentra_types::Error> {
        if source_url.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "source_url is required",
            ));
        }
        if target_key.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "target_key is required",
            ));
        }
        if total_bytes == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "total_bytes must be greater than zero",
            ));
        }
        if concurrency == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "concurrency must be greater than zero",
            ));
        }
        if deadline_seconds == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "deadline_seconds must be greater than zero",
            ));
        }
        Ok(Self {
            id,
            tenant_id,
            project_id,
            source_url,
            target_key,
            media_type,
            total_bytes,
            transferred_bytes: 0,
            concurrency,
            deadline_seconds,
            digest: None,
            state: ImportJobState::Pending,
            failure: None,
            retry_count: 0,
        })
    }

    pub fn start_inspection(&mut self, total_bytes: u64) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ImportJobState::Pending) {
            return Err(moqentra_types::Error::conflict("job is not pending"));
        }
        if total_bytes == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "total_bytes must be greater than zero",
            ));
        }
        self.total_bytes = total_bytes;
        self.state = ImportJobState::Inspecting;
        Ok(())
    }

    pub fn start_transfer(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ImportJobState::Inspecting) {
            return Err(moqentra_types::Error::conflict("job is not inspecting"));
        }
        self.state = ImportJobState::Transferring;
        Ok(())
    }

    pub fn progress_transfer(&mut self, bytes: u64) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ImportJobState::Transferring) {
            return Err(moqentra_types::Error::conflict("job is not transferring"));
        }
        self.transferred_bytes = self
            .transferred_bytes
            .checked_add(bytes)
            .ok_or_else(|| moqentra_types::Error::invalid_argument("transferred bytes overflow"))?;
        if self.transferred_bytes > self.total_bytes {
            return Err(moqentra_types::Error::invalid_argument(
                "transferred bytes exceed total",
            ));
        }
        Ok(())
    }

    pub fn start_validation(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ImportJobState::Transferring) {
            return Err(moqentra_types::Error::conflict("job is not transferring"));
        }
        if self.transferred_bytes != self.total_bytes {
            return Err(moqentra_types::Error::conflict(
                "transfer not complete before validation",
            ));
        }
        self.state = ImportJobState::Validating;
        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ImportJobState::Validating) {
            return Err(moqentra_types::Error::conflict("job is not validating"));
        }
        self.state = ImportJobState::Completed;
        Ok(())
    }

    pub fn fail(&mut self, reason: ImportJobFailure) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            ImportJobState::Completed | ImportJobState::Cancelled
        ) {
            return Err(moqentra_types::Error::conflict("terminal state reached"));
        }
        self.state = ImportJobState::Failed;
        self.failure = Some(reason);
        self.retry_count += 1;
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            ImportJobState::Completed | ImportJobState::Failed
        ) {
            return Err(moqentra_types::Error::conflict(
                "cannot cancel terminal job",
            ));
        }
        self.state = ImportJobState::Cancelled;
        self.failure = Some(ImportJobFailure::Cancelled);
        Ok(())
    }

    /// Retry a failed import from the beginning.
    pub fn retry(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ImportJobState::Failed) {
            return Err(moqentra_types::Error::conflict(
                "only failed jobs can be retried",
            ));
        }
        self.state = ImportJobState::Pending;
        self.failure = None;
        self.transferred_bytes = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_job_happy_path() {
        let mut job = ImportJob::new();
        job.start_inspection(1000).unwrap();
        job.start_transfer().unwrap();
        job.progress_transfer(500).unwrap();
        job.progress_transfer(500).unwrap();
        job.start_validation().unwrap();
        job.complete().unwrap();
        assert_eq!(job.state, ImportJobState::Completed);
    }

    #[test]
    fn import_job_fails_and_recovers() {
        let mut job = ImportJob::new();
        job.start_inspection(1000).unwrap();
        job.fail(ImportJobFailure::Network).unwrap();
        assert_eq!(job.state, ImportJobState::Failed);
        assert_eq!(job.retry_count, 1);
        job.retry().unwrap();
        assert_eq!(job.state, ImportJobState::Pending);
        assert!(job.failure.is_none());
        assert_eq!(job.transferred_bytes, 0);
    }

    #[test]
    fn import_job_over_transfer_rejected() {
        let mut job = ImportJob::new();
        job.start_inspection(100).unwrap();
        job.start_transfer().unwrap();
        assert!(job.progress_transfer(101).is_err());
    }

    #[test]
    fn completed_job_cannot_be_cancelled() {
        let mut job = ImportJob::new();
        job.start_inspection(100).unwrap();
        job.start_transfer().unwrap();
        job.progress_transfer(100).unwrap();
        job.start_validation().unwrap();
        job.complete().unwrap();
        assert!(job.cancel().is_err());
    }
}
