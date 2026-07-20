//! Moqentra `moqentra-scheduler` crate.
//!
//! This crate is part of the Moqentra workspace. Domain logic and public APIs
//! are documented in the `dev-docs/002_vibe_coding_plan` chapters.

#![allow(missing_docs)]

pub mod scheduler;

/// Placeholder module until domain types are added in subsequent tasks.
pub mod placeholder {
    /// Returns the crate version.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}
