//! Moqentra `moqentra-http-api` crate.
//!
//! Northbound API concerns: problem details, idempotency, pagination, SSE and
//! webhook primitives.

#![allow(missing_docs)]

pub mod annotation;
pub mod control_plane;
pub mod gc_worker;
pub mod import;
pub mod northbound;
pub mod pg_stores;
pub mod reconciler_worker;
pub mod validation_worker;
pub mod worker_runtime;

pub use control_plane::{app_router, spawn_outbox_dispatcher, AppState};
pub use gc_worker::spawn_gc_worker;
pub use import::{ImportJobStore, InMemoryImportJobStore};
pub use pg_stores::{PgImportJobStore, PgUploadSessionStore};
pub use reconciler_worker::spawn_reconciler_worker;
pub use validation_worker::spawn_media_validation_worker;
pub use worker_runtime::{ShutdownFlag, WorkerLimits};
