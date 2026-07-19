//! Unit-of-work and cursor-based pagination helpers.

use moqentra_types::{Error, PageRequest};
use serde::{Deserialize, Serialize};

/// Opaque cursor for paginated repository queries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub value: Option<String>,
}

impl Cursor {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: Some(value.into()),
        }
    }

    pub fn empty() -> Self {
        Self { value: None }
    }
}

/// A generic paginated response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<Cursor>,
    pub total: u64,
}

/// Builds a pagination SQL clause from a `PageRequest`.
///
/// The returned tuple is `(limit, offset)` suitable for `LIMIT $n OFFSET $n`.
pub fn pagination_clause(req: PageRequest) -> (u64, u64) {
    let limit = req.bounded_limit();
    (limit as u64, req.offset as u64)
}

/// Trait for a transaction-scoped unit of work.
#[async_trait::async_trait]
pub trait UnitOfWork: Send + Sync {
    async fn commit(self) -> Result<(), Error>;
    async fn rollback(self) -> Result<(), Error>;
}
