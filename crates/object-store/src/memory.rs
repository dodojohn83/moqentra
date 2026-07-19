//! In-memory object storage backend for unit tests.

use crate::{ObjectMetadata, ObjectStorage};
use bytes::Bytes;
use moqentra_types::Error;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
struct Object {
    data: Bytes,
    media_type: Option<String>,
    etag: String,
    digest: String,
}

#[derive(Debug, Clone, Default)]
struct MultipartUpload {
    media_type: Option<String>,
    parts: HashMap<i32, Bytes>,
}

/// In-memory object store with multipart support.
#[derive(Debug, Clone, Default)]
pub struct InMemoryObjectStore {
    objects: Arc<Mutex<HashMap<String, Object>>>,
    multipart: Arc<Mutex<HashMap<String, MultipartUpload>>>,
    counter: Arc<Mutex<u64>>,
}

impl InMemoryObjectStore {
    pub fn new() -> Self {
        Self::default()
    }
}

fn digest(data: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(data))
}

#[async_trait::async_trait]
impl ObjectStorage for InMemoryObjectStore {
    async fn put_object(
        &self,
        key: &str,
        data: Bytes,
        media_type: Option<&str>,
    ) -> Result<ObjectMetadata, Error> {
        let digest_value = digest(&data);
        let etag = format!("\"{}\"", digest_value);
        let object = Object {
            data,
            media_type: media_type.map(|s| s.to_string()),
            etag: etag.clone(),
            digest: digest_value,
        };
        let meta = ObjectMetadata {
            key: key.to_string(),
            size: object.data.len() as u64,
            media_type: object.media_type.clone(),
            etag: Some(etag),
            digest: Some(object.digest.clone()),
        };
        self.objects.lock().expect("lock").insert(key.to_string(), object);
        Ok(meta)
    }

    async fn get_object(&self, key: &str) -> Result<(Bytes, ObjectMetadata), Error> {
        let objects = self.objects.lock().expect("lock");
        let object = objects
            .get(key)
            .ok_or_else(|| Error::not_found(format!("object not found: {}", key)))?;
        let meta = ObjectMetadata {
            key: key.to_string(),
            size: object.data.len() as u64,
            media_type: object.media_type.clone(),
            etag: Some(object.etag.clone()),
            digest: Some(object.digest.clone()),
        };
        Ok((object.data.clone(), meta))
    }

    async fn delete_object(&self, key: &str) -> Result<(), Error> {
        self.objects.lock().expect("lock").remove(key);
        Ok(())
    }

    async fn presigned_get_url(&self, key: &str, _ttl: Duration) -> Result<String, Error> {
        // In-memory presigned URLs are not real; return a stable test URI.
        Ok(format!("memory://{}", key))
    }

    async fn start_multipart(&self, _key: &str, media_type: Option<&str>) -> Result<String, Error> {
        let mut counter = self.counter.lock().expect("lock");
        *counter += 1;
        let upload_id = format!("upload-{}", counter);
        self.multipart.lock().expect("lock").insert(
            upload_id.clone(),
            MultipartUpload {
                media_type: media_type.map(|s| s.to_string()),
                parts: HashMap::new(),
            },
        );
        Ok(upload_id)
    }

    async fn upload_part(
        &self,
        _key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
    ) -> Result<String, Error> {
        let mut multipart = self.multipart.lock().expect("lock");
        let upload = multipart
            .get_mut(upload_id)
            .ok_or_else(|| Error::not_found("multipart upload"))?;
        let etag = format!("\"{}\"", digest(&data));
        upload.parts.insert(part_number, data);
        Ok(etag)
    }

    async fn complete_multipart(
        &self,
        key: &str,
        upload_id: &str,
        parts: Vec<(i32, String)>,
    ) -> Result<ObjectMetadata, Error> {
        let mut multipart = self.multipart.lock().expect("lock");
        let upload = multipart
            .remove(upload_id)
            .ok_or_else(|| Error::not_found("multipart upload"))?;

        let mut combined = Vec::new();
        for (part_number, _etag) in parts {
            let part = upload
                .parts
                .get(&part_number)
                .ok_or_else(|| Error::invalid_argument(format!("missing part {}", part_number)))?;
            combined.extend_from_slice(part);
        }
        let data = Bytes::from(combined);
        let etag = format!("\"{}\"", digest(&data));
        let digest_value = digest(&data);
        let object = Object {
            data,
            media_type: upload.media_type.clone(),
            etag: etag.clone(),
            digest: digest_value,
        };
        let meta = ObjectMetadata {
            key: key.to_string(),
            size: object.data.len() as u64,
            media_type: object.media_type.clone(),
            etag: Some(etag),
            digest: Some(object.digest.clone()),
        };
        self.objects.lock().expect("lock").insert(key.to_string(), object);
        Ok(meta)
    }

    async fn abort_multipart(&self, _key: &str, upload_id: &str) -> Result<(), Error> {
        self.multipart.lock().expect("lock").remove(upload_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip() {
        let store = InMemoryObjectStore::new();
        let meta = store
            .put_object(
                "hello.txt",
                Bytes::from_static(b"hello"),
                Some("text/plain"),
            )
            .await
            .unwrap();
        assert_eq!(meta.size, 5);

        let (data, meta2) = store.get_object("hello.txt").await.unwrap();
        assert_eq!(data, Bytes::from_static(b"hello"));
        assert_eq!(meta2.digest, meta.digest);
    }

    #[tokio::test]
    async fn multipart_roundtrip() {
        let store = InMemoryObjectStore::new();
        let upload_id = store
            .start_multipart("big.bin", Some("application/octet-stream"))
            .await
            .unwrap();
        let etag1 = store
            .upload_part("big.bin", &upload_id, 1, Bytes::from_static(b"hello"))
            .await
            .unwrap();
        let etag2 = store
            .upload_part("big.bin", &upload_id, 2, Bytes::from_static(b"world"))
            .await
            .unwrap();
        let meta = store
            .complete_multipart("big.bin", &upload_id, vec![(1, etag1), (2, etag2)])
            .await
            .unwrap();
        let (data, _) = store.get_object("big.bin").await.unwrap();
        assert_eq!(data, Bytes::from_static(b"helloworld"));
        assert_eq!(meta.size, 10);
    }

    #[tokio::test]
    async fn digest_conflict_detected() {
        let store = InMemoryObjectStore::new();
        let meta1 = store.put_object("a.bin", Bytes::from_static(b"data"), None).await.unwrap();
        let meta2 = store.put_object("a.bin", Bytes::from_static(b"data"), None).await.unwrap();
        // Same data results in the same digest.
        assert_eq!(meta1.digest, meta2.digest);
    }
}
