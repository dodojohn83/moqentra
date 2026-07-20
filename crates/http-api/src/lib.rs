//! Moqentra `moqentra-http-api` crate.
//!
//! Northbound API concerns: problem details, idempotency, pagination, SSE and
//! webhook primitives.

#![allow(missing_docs)]

pub mod northbound;

/// Placeholder module until domain types are added in subsequent tasks.
pub mod placeholder {
    /// Returns the crate version.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}
