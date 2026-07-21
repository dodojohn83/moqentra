//! S3/MinIO object storage adapter.

use crate::{ObjectMetadata, ObjectStorage};
use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use bytes::Bytes;
use moqentra_types::{config::SecretString, Error};
use std::time::Duration;

/// Configuration for an S3-compatible object store.
#[derive(Clone)]
pub struct S3Config {
    pub bucket: String,
    pub endpoint: String,
    pub region: String,
    pub access_key_id: SecretString,
    pub secret_access_key: SecretString,
    pub force_path_style: bool,
}

impl std::fmt::Debug for S3Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Config")
            .field("bucket", &self.bucket)
            .field("endpoint", &self.endpoint)
            .field("region", &self.region)
            .field("access_key_id", &"[REDACTED]")
            .field("secret_access_key", &"[REDACTED]")
            .field("force_path_style", &self.force_path_style)
            .finish()
    }
}

/// S3/MinIO backed object store.
#[derive(Clone)]
pub struct S3ObjectStore {
    client: aws_sdk_s3::Client,
    bucket: String,
}

impl std::fmt::Debug for S3ObjectStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3ObjectStore")
            .field("bucket", &self.bucket)
            .finish_non_exhaustive()
    }
}

impl S3ObjectStore {
    pub fn new(config: S3Config) -> Result<Self, Error> {
        let credentials = Credentials::new(
            config.access_key_id.expose_secret(),
            config.secret_access_key.expose_secret(),
            None,
            None,
            "static",
        );

        let s3_config = aws_sdk_s3::Config::builder()
            .credentials_provider(credentials)
            .region(aws_sdk_s3::config::Region::new(config.region))
            .endpoint_url(config.endpoint)
            .force_path_style(config.force_path_style)
            .build();

        let client = aws_sdk_s3::Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket: config.bucket,
        })
    }

    fn map_error<E: std::fmt::Display>(err: E) -> Error {
        Error::unavailable(format!("object store error: {}", err))
    }
}

#[async_trait]
impl ObjectStorage for S3ObjectStore {
    async fn put_object(
        &self,
        key: &str,
        data: Bytes,
        media_type: Option<&str>,
    ) -> Result<ObjectMetadata, Error> {
        let body = ByteStream::from(data.clone());
        let mut builder = self.client.put_object().bucket(&self.bucket).key(key).body(body);
        if let Some(mt) = media_type {
            builder = builder.content_type(mt.to_string());
        }
        let output = builder.send().await.map_err(S3ObjectStore::map_error)?;

        Ok(ObjectMetadata {
            key: key.to_string(),
            size: data.len() as u64,
            media_type: media_type.map(|s| s.to_string()),
            etag: output.e_tag,
            digest: None,
        })
    }

    async fn get_object(&self, key: &str) -> Result<(Bytes, ObjectMetadata), Error> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(S3ObjectStore::map_error)?;
        let size = u64::try_from(output.content_length.unwrap_or(0))
            .map_err(|_| Error::invalid_argument("negative content length"))?;
        let media_type = output.content_type.map(|s| s.to_string());
        let bytes = output.body.collect().await.map_err(S3ObjectStore::map_error)?.into_bytes();
        Ok((
            bytes,
            ObjectMetadata {
                key: key.to_string(),
                size,
                media_type,
                etag: output.e_tag,
                digest: None,
            },
        ))
    }

    async fn delete_object(&self, key: &str) -> Result<(), Error> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(S3ObjectStore::map_error)?;
        Ok(())
    }

    async fn presigned_get_url(&self, key: &str, ttl: Duration) -> Result<String, Error> {
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::builder()
            .expires_in(ttl)
            .build()
            .map_err(S3ObjectStore::map_error)?;
        let request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(S3ObjectStore::map_error)?;
        Ok(request.uri().to_string())
    }

    async fn start_multipart(&self, key: &str, media_type: Option<&str>) -> Result<String, Error> {
        let mut builder = self.client.create_multipart_upload().bucket(&self.bucket).key(key);
        if let Some(mt) = media_type {
            builder = builder.content_type(mt.to_string());
        }
        let output = builder.send().await.map_err(S3ObjectStore::map_error)?;
        output.upload_id.ok_or_else(|| Error::internal("missing upload id"))
    }

    async fn upload_part(
        &self,
        _key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
    ) -> Result<String, Error> {
        let body = ByteStream::from(data);
        let output = self
            .client
            .upload_part()
            .bucket(&self.bucket)
            .key(_key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(body)
            .send()
            .await
            .map_err(S3ObjectStore::map_error)?;
        output.e_tag.ok_or_else(|| Error::internal("missing etag"))
    }

    async fn complete_multipart(
        &self,
        key: &str,
        upload_id: &str,
        mut parts: Vec<(i32, String)>,
    ) -> Result<ObjectMetadata, Error> {
        use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
        parts.sort_by_key(|(n, _)| *n);
        let completed_parts: Vec<_> = parts
            .into_iter()
            .map(|(part_number, etag)| {
                CompletedPart::builder().part_number(part_number).e_tag(etag).build()
            })
            .collect();
        let completed =
            CompletedMultipartUpload::builder().set_parts(Some(completed_parts)).build();
        let output = self
            .client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(completed)
            .send()
            .await
            .map_err(S3ObjectStore::map_error)?;
        Ok(ObjectMetadata {
            key: key.to_string(),
            size: 0, // S3 response does not include size directly.
            media_type: None,
            etag: output.e_tag,
            digest: None,
        })
    }

    async fn abort_multipart(&self, key: &str, upload_id: &str) -> Result<(), Error> {
        self.client
            .abort_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(S3ObjectStore::map_error)?;
        Ok(())
    }
}
