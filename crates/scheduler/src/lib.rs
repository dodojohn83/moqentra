//! Moqentra `moqentra-scheduler` crate.
//!
//! This crate is part of the Moqentra workspace. Domain logic and public APIs
//! are documented in the `dev-docs/002_vibe_coding_plan` chapters.

#![allow(missing_docs)]

pub mod distributed;
pub mod reconciler;
pub mod scheduler;

pub use reconciler::{ClusterAgent, DesiredObserved, LeaderElection, Lease, Reconciler};
pub use scheduler::{
    AcceleratorCapability, ExecutionPlan, GangGroup, NetworkPolicySpec, QueueEntry,
    SchedulingQueue, VolumeSpec,
};

/// Placeholder module until domain types are added in subsequent tasks.
pub mod placeholder {
    /// Returns the crate version.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}
