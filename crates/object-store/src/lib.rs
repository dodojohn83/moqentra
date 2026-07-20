//! Moqentra object storage adapter.

#![allow(missing_docs)]

pub mod memory;
pub mod s3;

pub use memory::InMemoryObjectStore;
pub use s3::S3ObjectStore;

use bytes::Bytes;
use moqentra_types::Error;
use std::time::Duration;

/// Metadata returned with an object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMetadata {
    pub key: String,
    pub size: u64,
    pub media_type: Option<String>,
    pub etag: Option<String>,
    pub digest: Option<String>,
}

/// Port for object storage backends.
#[async_trait::async_trait]
pub trait ObjectStorage: Send + Sync {
    /// Put an object under the given key.
    async fn put_object(
        &self,
        key: &str,
        data: Bytes,
        media_type: Option<&str>,
    ) -> Result<ObjectMetadata, Error>;

    /// Get an object by key.
    async fn get_object(&self, key: &str) -> Result<(Bytes, ObjectMetadata), Error>;

    /// Delete an object by key.
    async fn delete_object(&self, key: &str) -> Result<(), Error>;

    /// Generate a short-lived pre-signed URL for downloading an object.
    async fn presigned_get_url(&self, key: &str, ttl: Duration) -> Result<String, Error>;

    /// Start a multipart upload and return an upload ID.
    async fn start_multipart(&self, key: &str, media_type: Option<&str>) -> Result<String, Error>;

    /// Upload a single part.
    async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
    ) -> Result<String, Error>;

    /// Complete a multipart upload.
    async fn complete_multipart(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> Result<ObjectMetadata, Error>;

    /// Abort a multipart upload.
    async fn abort_multipart(&self, key: &str, upload_id: &str) -> Result<(), Error>;
}

pub mod placeholder {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
}
