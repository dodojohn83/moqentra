//! Import job state machine.

use serde::{Deserialize, Serialize};

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
    pub state: ImportJobState,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
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
            state: ImportJobState::Pending,
            total_bytes: 0,
            transferred_bytes: 0,
            failure: None,
            retry_count: 0,
        }
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
