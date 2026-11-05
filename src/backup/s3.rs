//! S3 client for remote backup storage.
//!
//! Supports AWS S3, MinIO, Cloudflare R2, and any S3-compatible storage.

use std::path::Path;

use anyhow::{Context, Result};
use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::{config::Builder as S3ConfigBuilder, primitives::ByteStream, Client};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::db::S3Object;

/// S3 client wrapper for backup operations
pub struct S3Client {
    client: Client,
    bucket: String,
    path_prefix: String,
}

impl S3Client {
    /// Create a new S3 client from a storage config's decrypted credentials.
    ///
    /// # Arguments
    /// * `endpoint` - Optional custom endpoint URL (for MinIO, R2, etc.)
    /// * `bucket` - S3 bucket name
    /// * `region` - AWS region (e.g., "us-east-1")
    /// * `access_key` - Decrypted AWS access key ID
    /// * `secret_key` - Decrypted AWS secret access key
    /// * `path_prefix` - Optional prefix for all S3 keys
    pub fn new(
        endpoint: Option<&str>,
        bucket: &str,
        region: &str,
        access_key: &str,
        secret_key: &str,
        path_prefix: Option<&str>,
    ) -> Result<Self> {
        let credentials = Credentials::new(access_key, secret_key, None, None, "rivetr-s3");

        let mut config_builder = S3ConfigBuilder::new()
            .region(Region::new(region.to_string()))
            .credentials_provider(credentials)
            .behavior_version_latest();

        // For custom endpoints (MinIO, R2, etc.), force path-style addressing
        if let Some(ep) = endpoint {
            config_builder = config_builder.endpoint_url(ep).force_path_style(true);
        }

        let config = config_builder.build();
        let client = Client::from_conf(config);

        Ok(Self {
            client,
            bucket: bucket.to_string(),
            path_prefix: path_prefix.unwrap_or("").to_string(),
        })
    }

    /// The configured key prefix (without a trailing slash handling applied).
    pub fn path_prefix(&self) -> &str {
        &self.path_prefix
    }

    /// Build the full S3 key with path prefix
    fn full_key(&self, key: &str) -> String {
        if self.path_prefix.is_empty() {
            key.to_string()
        } else {
            format!(
                "{}/{}",
                self.path_prefix.trim_end_matches('/'),
                key.trim_start_matches('/')
            )
        }
    }

    /// Test the S3 connection by listing the bucket (HEAD bucket).
    pub async fn test_connection(&self) -> Result<()> {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .context(
                "Failed to connect to S3 bucket. Check credentials, bucket name, and endpoint.",
            )?;
        info!("S3 connection test successful for bucket: {}", self.bucket);
        Ok(())
    }

    /// Upload data to S3.
    pub async fn upload_backup(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let full_key = self.full_key(key);
        let body = ByteStream::from(data);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .body(body)
            .send()
            .await
            .context(format!("Failed to upload to S3: {}", full_key))?;

        info!("Uploaded backup to S3: s3://{}/{}", self.bucket, full_key);
        Ok(())
    }

    /// Upload a file from disk, streaming it instead of reading it into memory.
    pub async fn upload_file(&self, key: &str, path: &Path) -> Result<()> {
        let full_key = self.full_key(key);
        let body = ByteStream::from_path(path)
            .await
            .with_context(|| format!("Failed to open {} for upload", path.display()))?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .body(body)
            .send()
            .await
            .context(format!("Failed to upload to S3: {}", full_key))?;

        info!("Uploaded backup to S3: s3://{}/{}", self.bucket, full_key);
        Ok(())
    }

    /// Stream an object to `dest`, hashing it on the way. Stops (and removes the
    /// partial file) once more than `max_bytes` have arrived.
    pub async fn download_to_file(
        &self,
        key: &str,
        dest: &Path,
        max_bytes: u64,
    ) -> Result<DownloadedObject> {
        let full_key = self.full_key(key);

        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
            .context(format!("Failed to download from S3: {}", full_key))?;

        let content_length = output
            .content_length()
            .filter(|l| *l >= 0)
            .map(|l| l as u64);
        if let Some(len) = content_length {
            if len > max_bytes {
                anyhow::bail!(
                    "S3 object is {} bytes, more than the {} byte restore limit",
                    len,
                    max_bytes
                );
            }
        }

        let mut file = tokio::fs::File::create(dest)
            .await
            .with_context(|| format!("Failed to create {}", dest.display()))?;
        let mut body = output.body;
        let mut hasher = Sha256::new();
        let mut size: u64 = 0;
        let result: Result<()> = async {
            while let Some(chunk) = body
                .try_next()
                .await
                .context("Failed to read S3 object body")?
            {
                size += chunk.len() as u64;
                if size > max_bytes {
                    anyhow::bail!("S3 object exceeds the {} byte restore limit", max_bytes);
                }
                hasher.update(&chunk);
                file.write_all(&chunk).await?;
            }
            file.flush().await?;
            Ok(())
        }
        .await;
        if let Err(e) = result {
            drop(file);
            let _ = tokio::fs::remove_file(dest).await;
            return Err(e);
        }

        info!(
            "Downloaded backup from S3: s3://{}/{} ({} bytes)",
            self.bucket, full_key, size
        );
        Ok(DownloadedObject {
            size,
            content_length,
            sha256: hex::encode(hasher.finalize()),
        })
    }

    /// Download data from S3.
    pub async fn download_backup(&self, key: &str) -> Result<Vec<u8>> {
        let full_key = self.full_key(key);

        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
            .context(format!("Failed to download from S3: {}", full_key))?;

        let data = output
            .body
            .collect()
            .await
            .context("Failed to read S3 object body")?
            .into_bytes()
            .to_vec();

        info!(
            "Downloaded backup from S3: s3://{}/{} ({} bytes)",
            self.bucket,
            full_key,
            data.len()
        );
        Ok(data)
    }

    /// List objects in S3 under a prefix (all pages). Keys are returned in full,
    /// including the configured path prefix.
    pub async fn list_backups(&self, prefix: &str) -> Result<Vec<S3Object>> {
        let full_prefix = self.full_key(prefix);
        let mut objects = Vec::new();
        let mut token: Option<String> = None;

        // Hard cap so a huge bucket cannot stall a request.
        for _ in 0..50 {
            let output = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&full_prefix)
                .set_continuation_token(token.clone())
                .send()
                .await
                .context("Failed to list S3 objects")?;

            objects.extend(output.contents().iter().map(|obj| S3Object {
                key: obj.key().unwrap_or_default().to_string(),
                size: obj.size().unwrap_or(0),
                last_modified: obj.last_modified().map(|dt| {
                    dt.fmt(aws_sdk_s3::primitives::DateTimeFormat::DateTime)
                        .unwrap_or_default()
                }),
            }));

            match output.next_continuation_token() {
                Some(next) if output.is_truncated().unwrap_or(false) => {
                    token = Some(next.to_string())
                }
                _ => break,
            }
        }

        Ok(objects)
    }

    /// Delete an object from S3.
    pub async fn delete_backup(&self, key: &str) -> Result<()> {
        let full_key = self.full_key(key);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
            .context(format!("Failed to delete from S3: {}", full_key))?;

        info!("Deleted backup from S3: s3://{}/{}", self.bucket, full_key);
        Ok(())
    }
}

/// Result of [`S3Client::download_to_file`].
#[derive(Debug, Clone)]
pub struct DownloadedObject {
    /// Bytes written to disk
    pub size: u64,
    /// Content-Length S3 reported, when it did
    pub content_length: Option<u64>,
    /// Hex SHA-256 of what was written
    pub sha256: String,
}
