//! Pagination request and result types.

use serde::{Deserialize, Serialize};

const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 1000;

const fn default_limit() -> u32 {
    DEFAULT_LIMIT
}

/// A page request from a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageRequest {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

impl PageRequest {
    pub const fn new(limit: u32, offset: u32) -> Self {
        Self { limit, offset }
    }

    /// Returns a bounded limit, capping at [`MAX_LIMIT`].
    pub fn bounded_limit(&self) -> u32 {
        self.limit.min(MAX_LIMIT)
    }
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            limit: DEFAULT_LIMIT,
            offset: 0,
        }
    }
}

/// A paginated result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u32,
    pub limit: u32,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: u64, req: PageRequest) -> Self {
        Self {
            items,
            total,
            offset: req.offset,
            limit: req.bounded_limit(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn has_more(&self) -> bool {
        u64::from(self.offset).saturating_add(u64::from(self.limit)) < self.total
    }
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            offset: 0,
            limit: DEFAULT_LIMIT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_request_caps_limit() {
        let req = PageRequest::new(2000, 0);
        assert_eq!(req.bounded_limit(), MAX_LIMIT);
    }

    #[test]
    fn page_has_more() {
        let req = PageRequest::new(10, 0);
        let page = Page::new(vec![0; 10], 25, req);
        assert!(page.has_more());
    }

    #[test]
    fn page_no_more() {
        let req = PageRequest::new(10, 0);
        let page = Page::new(vec![0; 10], 10, req);
        assert!(!page.has_more());
    }
}
