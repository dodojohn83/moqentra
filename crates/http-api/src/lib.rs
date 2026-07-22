//! Moqentra `moqentra-http-api` crate.
//!
//! Northbound API concerns: problem details, idempotency, pagination, SSE and
//! webhook primitives.

#![allow(missing_docs)]

pub mod control_plane;
pub mod import;
pub mod northbound;

pub use control_plane::{app_router, spawn_outbox_dispatcher, AppState};
pub use import::{ImportJobStore, InMemoryImportJobStore};
