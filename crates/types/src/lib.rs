//! Moqentra `moqentra-types` crate.
//!
//! This crate is part of the Moqentra workspace. Domain logic and public APIs
//! are documented in the `dev-docs/002_vibe_coding_plan` chapters.

#![warn(missing_docs)]

/// Placeholder module until domain types are added in subsequent tasks.
pub mod placeholder {
    /// Returns the crate version.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}

#[cfg(test)]
mod tests {
    use super::placeholder;

    #[test]
    fn workspace_sanity() {
        assert_eq!(placeholder::VERSION, "0.1.0");
    }
}
