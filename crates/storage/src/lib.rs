//! Moqentra PostgreSQL storage adapter.

#![allow(missing_docs)]

pub mod idempotency;
pub mod outbox;
pub mod pg_audit;
pub mod pg_idempotency;
pub mod pg_outbox;
pub mod pool;
pub mod unit_of_work;

pub use idempotency::{
    IdempotencyEntry, IdempotencyResult, IdempotencyScope, IdempotencyStatus, IdempotencyStore,
    InMemoryIdempotencyStore,
};
pub use outbox::{InMemoryOutbox, OutboxEvent, OutboxStatus, OutboxStore};
pub use pg_audit::PgAuditLog;
pub use pg_idempotency::PgIdempotencyStore;
pub use pg_outbox::PgOutboxStore;
pub use pool::{ConnectionPool, ScopedConnection};
pub use unit_of_work::{pagination_clause, Cursor, Paginated, UnitOfWork};

pub mod placeholder {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}
