//! S3 storage configuration and backup models.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// S3 storage configuration stored in the database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct S3StorageConfig {
    pub id: String,
    pub name: String,
    pub endpoint: Option<String>,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub path_prefix: Option<String>,
    pub is_default: i32,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO that masks secret key values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3StorageConfigResponse {
    pub id: String,
    pub name: String,
    pub endpoint: Option<String>,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub path_prefix: Option<String>,
    pub is_default: bool,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl S3StorageConfig {
    pub fn to_response(&self, reveal_secrets: bool) -> S3StorageConfigResponse {
        S3StorageConfigResponse {
            id: self.id.clone(),
            name: self.name.clone(),
            endpoint: self.endpoint.clone(),
            bucket: self.bucket.clone(),
            region: self.region.clone(),
            access_key: if reveal_secrets {
                self.access_key.clone()
            } else {
                "********".to_string()
            },
            secret_key: if reveal_secrets {
                self.secret_key.clone()
            } else {
                "********".to_string()
            },
            path_prefix: self.path_prefix.clone(),
            is_default: self.is_default != 0,
            team_id: self.team_id.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

/// Request to create an S3 storage configuration
#[derive(Debug, Deserialize)]
pub struct CreateS3StorageConfigRequest {
    pub name: String,
    pub endpoint: Option<String>,
    pub bucket: String,
    #[serde(default = "default_region")]
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    #[serde(default)]
    pub path_prefix: Option<String>,
    #[serde(default)]
    pub is_default: bool,
    pub team_id: Option<String>,
}

fn default_region() -> String {
    "us-east-1".to_string()
}

/// Request to update an S3 storage configuration
#[derive(Debug, Deserialize)]
pub struct UpdateS3StorageConfigRequest {
    pub name: Option<String>,
    pub endpoint: Option<String>,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub path_prefix: Option<String>,
    pub is_default: Option<bool>,
}

/// S3 backup record stored in the database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct S3Backup {
    pub id: String,
    pub storage_config_id: String,
    pub backup_type: String,
    pub source_id: Option<String>,
    pub s3_key: String,
    pub size_bytes: Option<i64>,
    pub status: String,
    pub error_message: Option<String>,
    pub team_id: Option<String>,
    pub created_at: String,
}

/// Response DTO for S3 backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3BackupResponse {
    pub id: String,
    pub storage_config_id: String,
    pub storage_config_name: Option<String>,
    pub backup_type: String,
    pub source_id: Option<String>,
    pub s3_key: String,
    pub size_bytes: Option<i64>,
    pub size_human: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub team_id: Option<String>,
    pub created_at: String,
}

impl S3Backup {
    pub fn to_response(&self, config_name: Option<String>) -> S3BackupResponse {
        S3BackupResponse {
            id: self.id.clone(),
            storage_config_id: self.storage_config_id.clone(),
            storage_config_name: config_name,
            backup_type: self.backup_type.clone(),
            source_id: self.source_id.clone(),
            s3_key: self.s3_key.clone(),
            size_bytes: self.size_bytes,
            size_human: self.size_bytes.map(format_bytes),
            status: self.status.clone(),
            error_message: self.error_message.clone(),
            team_id: self.team_id.clone(),
            created_at: self.created_at.clone(),
        }
    }
}

/// Request to trigger an S3 backup
#[derive(Debug, Deserialize)]
pub struct TriggerS3BackupRequest {
    pub storage_config_id: String,
    pub backup_type: String,
    pub source_id: Option<String>,
}

/// Request to restore from an S3 backup
#[derive(Debug, Deserialize)]
pub struct RestoreS3BackupRequest {
    /// Optional: override the storage config for the restore
    pub storage_config_id: Option<String>,
}

/// S3 object metadata from list operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Object {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<String>,
}

fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
