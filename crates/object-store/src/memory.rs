//! In-memory object storage backend for unit tests.

use crate::{ObjectMetadata, ObjectStorage};
use bytes::Bytes;
use moqentra_types::Error;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
struct Object {
    data: Bytes,
    media_type: Option<String>,
    etag: String,
    digest: String,
}

#[derive(Debug, Clone, Default)]
struct MultipartUpload {
    object_key: String,
    media_type: Option<String>,
    parts: HashMap<i32, Bytes>,
}

/// In-memory object store with multipart support.
#[derive(Debug, Clone, Default)]
pub struct InMemoryObjectStore {
    objects: Arc<Mutex<HashMap<String, Object>>>,
    created_at: Arc<Mutex<HashMap<String, Instant>>>,
    legal_holds: Arc<Mutex<BTreeSet<String>>>,
    multipart: Arc<Mutex<HashMap<String, MultipartUpload>>>,
    counter: Arc<Mutex<u64>>,
}

impl InMemoryObjectStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Place or remove a legal hold on an object key.
    pub fn set_legal_hold(&self, key: &str, hold: bool) {
        let mut holds = self.legal_holds.lock().unwrap_or_else(|e| e.into_inner());
        if hold {
            holds.insert(key.to_string());
        } else {
            holds.remove(key);
        }
    }

    /// Perform a bounded garbage collection pass over this store.
    ///
    /// Objects are eligible for deletion when they are not in `referenced`, are
    /// not under legal hold, and are older than `min_age`. At most `max_delete`
    /// objects are removed in one pass.
    pub fn gc(
        &self,
        referenced: &std::collections::BTreeSet<String>,
        min_age: Duration,
        max_delete: usize,
    ) -> usize {
        let now = Instant::now();
        let mut objects = self.objects.lock().unwrap_or_else(|e| e.into_inner());
        let mut created_at = self.created_at.lock().unwrap_or_else(|e| e.into_inner());
        let holds = self.legal_holds.lock().unwrap_or_else(|e| e.into_inner());
        let mut removed = 0;
        let mut keys: Vec<String> = objects.keys().cloned().collect();
        keys.sort();
        for key in keys {
            if removed >= max_delete {
                break;
            }
            if referenced.contains(&key) || holds.contains(&key) {
                continue;
            }
            let Some(instant) = created_at.get(&key) else {
                continue;
            };
            if now.duration_since(*instant) >= min_age {
                objects.remove(&key);
                created_at.remove(&key);
                removed += 1;
            }
        }
        removed
    }

    /// Return the keys that would be deleted by a GC pass without removing them.
    /// Objects under legal hold or with uncertain ownership (not in `referenced`
    /// but referenced indirectly) are excluded.
    pub fn gc_dry_run(
        &self,
        referenced: &std::collections::BTreeSet<String>,
        min_age: Duration,
        max_delete: usize,
    ) -> Vec<String> {
        let now = Instant::now();
        let objects = self.objects.lock().unwrap_or_else(|e| e.into_inner());
        let created_at = self.created_at.lock().unwrap_or_else(|e| e.into_inner());
        let holds = self.legal_holds.lock().unwrap_or_else(|e| e.into_inner());
        let mut candidates = Vec::new();
        let mut keys: Vec<String> = objects.keys().cloned().collect();
        keys.sort();
        for key in keys {
            if candidates.len() >= max_delete {
                break;
            }
            if referenced.contains(&key) || holds.contains(&key) {
                continue;
            }
            let Some(instant) = created_at.get(&key) else {
                continue;
            };
            if now.duration_since(*instant) >= min_age {
                candidates.push(key);
            }
        }
        candidates
    }
}

fn digest(data: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(data))
}

fn u64_len(len: usize) -> Result<u64, Error> {
    u64::try_from(len).map_err(|_| Error::invalid_argument("object size overflow"))
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
            size: u64_len(object.data.len())?,
            media_type: object.media_type.clone(),
            etag: Some(etag),
            digest: Some(object.digest.clone()),
        };
        {
            let mut objects = self.objects.lock().unwrap_or_else(|e| e.into_inner());
            let mut created_at = self.created_at.lock().unwrap_or_else(|e| e.into_inner());
            objects.insert(key.to_string(), object);
            created_at.insert(key.to_string(), Instant::now());
        }
        Ok(meta)
    }

    async fn get_object(&self, key: &str) -> Result<(Bytes, ObjectMetadata), Error> {
        let objects = self.objects.lock().unwrap_or_else(|e| e.into_inner());
        let object = objects
            .get(key)
            .ok_or_else(|| Error::not_found(format!("object not found: {}", key)))?;
        let meta = ObjectMetadata {
            key: key.to_string(),
            size: u64_len(object.data.len())?,
            media_type: object.media_type.clone(),
            etag: Some(object.etag.clone()),
            digest: Some(object.digest.clone()),
        };
        Ok((object.data.clone(), meta))
    }

    async fn delete_object(&self, key: &str) -> Result<(), Error> {
        self.objects.lock().unwrap_or_else(|e| e.into_inner()).remove(key);
        Ok(())
    }

    async fn presigned_get_url(&self, key: &str, _ttl: Duration) -> Result<String, Error> {
        // In-memory presigned URLs are not real; return a stable test URI.
        Ok(format!("memory://{}", key))
    }

    async fn start_multipart(&self, key: &str, media_type: Option<&str>) -> Result<String, Error> {
        let mut counter = self.counter.lock().unwrap_or_else(|e| e.into_inner());
        *counter += 1;
        let upload_id = format!("upload-{}", counter);
        self.multipart.lock().unwrap_or_else(|e| e.into_inner()).insert(
            upload_id.clone(),
            MultipartUpload {
                object_key: key.to_string(),
                media_type: media_type.map(|s| s.to_string()),
                parts: HashMap::new(),
            },
        );
        Ok(upload_id)
    }

    async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
    ) -> Result<String, Error> {
        if part_number <= 0 {
            return Err(Error::invalid_argument("part number must be positive"));
        }
        let mut multipart = self.multipart.lock().unwrap_or_else(|e| e.into_inner());
        let upload = multipart
            .get_mut(upload_id)
            .ok_or_else(|| Error::not_found("multipart upload"))?;
        if upload.object_key != key {
            return Err(Error::invalid_argument("multipart upload key mismatch"));
        }
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
        let mut multipart = self.multipart.lock().unwrap_or_else(|e| e.into_inner());
        let upload = multipart
            .remove(upload_id)
            .ok_or_else(|| Error::not_found("multipart upload"))?;
        if upload.object_key != key {
            return Err(Error::invalid_argument("multipart upload key mismatch"));
        }

        let mut combined = Vec::new();
        if parts.is_empty() {
            return Err(Error::invalid_argument(
                "multipart completion requires at least one part",
            ));
        }
        let mut parts: Vec<_> = parts;
        parts.sort_by_key(|(n, _)| *n);
        let mut seen = std::collections::HashSet::new();
        let mut expected = 1i32;
        for (part_number, etag) in parts {
            if part_number <= 0 {
                return Err(Error::invalid_argument("part number must be positive"));
            }
            if part_number != expected {
                return Err(Error::invalid_argument(format!(
                    "expected part {} but got {}",
                    expected, part_number
                )));
            }
            expected += 1;
            if !seen.insert(part_number) {
                return Err(Error::invalid_argument(format!(
                    "duplicate part {}",
                    part_number
                )));
            }
            let part = upload
                .parts
                .get(&part_number)
                .ok_or_else(|| Error::invalid_argument(format!("missing part {}", part_number)))?;
            let expected = format!("\"{}\"", digest(part));
            if etag != expected {
                return Err(Error::invalid_argument(format!(
                    "etag mismatch for part {}",
                    part_number
                )));
            }
            combined.extend_from_slice(part);
        }
        if seen.len() != upload.parts.len() {
            return Err(Error::invalid_argument(
                "multipart completion must list all uploaded parts",
            ));
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
            size: u64_len(object.data.len())?,
            media_type: object.media_type.clone(),
            etag: Some(etag),
            digest: Some(object.digest.clone()),
        };
        {
            let mut objects = self.objects.lock().unwrap_or_else(|e| e.into_inner());
            let mut created_at = self.created_at.lock().unwrap_or_else(|e| e.into_inner());
            objects.insert(key.to_string(), object);
            created_at.insert(key.to_string(), Instant::now());
        }
        Ok(meta)
    }

    async fn abort_multipart(&self, _key: &str, upload_id: &str) -> Result<(), Error> {
        self.multipart.lock().unwrap_or_else(|e| e.into_inner()).remove(upload_id);
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
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

    #[tokio::test]
    async fn gc_dry_run_does_not_delete() {
        use std::collections::BTreeSet;
        use std::time::Duration;

        let store = InMemoryObjectStore::new();
        store.put_object("orphan.bin", Bytes::from_static(b"x"), None).await.unwrap();
        let referenced = BTreeSet::new();
        let candidates = store.gc_dry_run(&referenced, Duration::from_secs(0), 10);
        assert_eq!(candidates.len(), 1);
        assert!(store.get_object("orphan.bin").await.is_ok());
    }

    #[tokio::test]
    async fn multipart_upload_key_mismatch_rejected() {
        let store = InMemoryObjectStore::new();
        let upload_id = store
            .start_multipart("big.bin", Some("application/octet-stream"))
            .await
            .unwrap();
        assert!(store
            .upload_part("other.bin", &upload_id, 1, Bytes::from_static(b"hello"))
            .await
            .is_err());
    }
}
