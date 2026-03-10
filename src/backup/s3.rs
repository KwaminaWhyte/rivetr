//! S3 client for remote backup storage.
//!
//! Supports AWS S3, MinIO, Cloudflare R2, and any S3-compatible storage.

use anyhow::{Context, Result};
use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::{config::Builder as S3ConfigBuilder, primitives::ByteStream, Client};
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

    /// List objects in S3 under a prefix.
    pub async fn list_backups(&self, prefix: &str) -> Result<Vec<S3Object>> {
        let full_prefix = self.full_key(prefix);

        let output = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&full_prefix)
            .send()
            .await
            .context("Failed to list S3 objects")?;

        let objects = output
            .contents()
            .iter()
            .map(|obj| S3Object {
                key: obj.key().unwrap_or_default().to_string(),
                size: obj.size().unwrap_or(0),
                last_modified: obj.last_modified().map(|dt| {
                    dt.fmt(aws_sdk_s3::primitives::DateTimeFormat::DateTime)
                        .unwrap_or_default()
                }),
            })
            .collect();

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
