//! S3 side of database, volume and instance backups: object keys, client setup,
//! streaming upload with checksum, verified download, and bucket scanning.
//!
//! Key layout (relative to the storage config's path prefix):
//! - `backups/database/<database_id>/<file>`
//! - `backups/volume/<volume_id>/<file>`
//! - `backups/instance/<file>` (older releases also wrote `instance-backups/<file>`)

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::backup::s3::S3Client;
use crate::crypto;
use crate::db::{S3Backup, S3StorageConfig};
use crate::DbPool;

/// Backup kinds stored in S3 (matches `s3_backups.backup_type`).
pub const KINDS: [&str; 3] = ["database", "volume", "instance"];

/// Build an S3 client for a stored config, decrypting its credentials.
pub fn client_for(config: &S3StorageConfig, encryption_secret: Option<&str>) -> Result<S3Client> {
    let key = encryption_secret.map(crypto::derive_key);
    let access_key = crypto::decrypt_if_encrypted(&config.access_key, key.as_ref())
        .map_err(|e| anyhow!("Failed to decrypt S3 access key: {e}"))?;
    let secret_key = crypto::decrypt_if_encrypted(&config.secret_key, key.as_ref())
        .map_err(|e| anyhow!("Failed to decrypt S3 secret key: {e}"))?;
    S3Client::new(
        config.endpoint.as_deref(),
        &config.bucket,
        &config.region,
        &access_key,
        &secret_key,
        config.path_prefix.as_deref(),
    )
}

/// Load a storage config by id.
pub async fn load_config(db: &DbPool, id: &str) -> Result<S3StorageConfig> {
    sqlx::query_as::<_, S3StorageConfig>("SELECT * FROM s3_storage_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| anyhow!("S3 storage config {id} not found"))
}

/// Folder (relative to the path prefix) holding backups of one resource.
pub fn listing_prefix(kind: &str, source_id: Option<&str>) -> String {
    match (kind, source_id) {
        ("instance", _) => "backups/instance/".to_string(),
        (kind, Some(id)) => format!("backups/{kind}/{id}/"),
        (kind, None) => format!("backups/{kind}/"),
    }
}

/// Object key for a new backup. `file_name` is the local archive's name.
pub fn backup_key(kind: &str, source_id: Option<&str>, file_name: &str) -> String {
    format!("{}{}", listing_prefix(kind, source_id), file_name)
}

/// A backup object found in a bucket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBackupKey {
    pub kind: String,
    pub source_id: Option<String>,
    pub file_name: String,
    /// Key relative to the storage config's path prefix (what `s3_backups.s3_key` holds)
    pub relative_key: String,
    /// Timestamp encoded in the file name, when there is one
    pub timestamp: Option<DateTime<Utc>>,
}

/// Strip the config's path prefix from a full object key.
pub fn strip_prefix<'a>(full_key: &'a str, path_prefix: &str) -> Option<&'a str> {
    let prefix = path_prefix.trim_matches('/');
    if prefix.is_empty() {
        return Some(full_key.trim_start_matches('/'));
    }
    full_key
        .trim_start_matches('/')
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('/'))
}

/// Parse a full object key (as listed) into a backup entry, or `None` for keys that
/// are not Rivetr backups (folders, stray files, other prefixes).
pub fn parse_backup_key(full_key: &str, path_prefix: &str) -> Option<ParsedBackupKey> {
    let relative = strip_prefix(full_key, path_prefix)?;
    let parts: Vec<&str> = relative.split('/').collect();
    let (kind, source_id, file_name) = match parts.as_slice() {
        ["backups", "instance", file] => ("instance", None, *file),
        ["instance-backups", file] => ("instance", None, *file),
        ["backups", kind @ ("database" | "volume"), id, file] if !id.is_empty() => {
            (*kind, Some(id.to_string()), *file)
        }
        _ => return None,
    };
    if file_name.is_empty() || file_name.starts_with('.') {
        return None;
    }
    Some(ParsedBackupKey {
        kind: kind.to_string(),
        source_id,
        file_name: file_name.to_string(),
        relative_key: relative.to_string(),
        timestamp: timestamp_from_file_name(file_name),
    })
}

/// Find a `YYYYMMDD[-_]HHMMSS` timestamp in a backup file name.
pub fn timestamp_from_file_name(name: &str) -> Option<DateTime<Utc>> {
    let bytes = name.as_bytes();
    if bytes.len() < 15 {
        return None;
    }
    for start in 0..=bytes.len() - 15 {
        let window = &name[start..start + 15];
        let sep = window.as_bytes()[8];
        if (sep == b'-' || sep == b'_')
            && window[..8].bytes().all(|b| b.is_ascii_digit())
            && window[9..].bytes().all(|b| b.is_ascii_digit())
        {
            let normalized = format!("{}{}", &window[..8], &window[9..]);
            if let Ok(dt) = NaiveDateTime::parse_from_str(&normalized, "%Y%m%d%H%M%S") {
                return Some(dt.and_utc());
            }
        }
    }
    None
}

/// Check a finished download against what the backup record says. A size or checksum
/// mismatch means a truncated or altered object; restoring it would be unsafe.
pub fn verify_download(
    expected_size: Option<i64>,
    expected_sha256: Option<&str>,
    content_length: Option<u64>,
    actual_size: u64,
    actual_sha256: &str,
) -> Result<()> {
    if actual_size == 0 {
        return Err(anyhow!("Downloaded backup is empty"));
    }
    if let Some(len) = content_length {
        if len != actual_size {
            return Err(anyhow!(
                "Download is incomplete: got {actual_size} of {len} bytes"
            ));
        }
    }
    if let Some(expected) = expected_size.filter(|s| *s > 0) {
        if expected as u64 != actual_size {
            return Err(anyhow!(
                "Backup size mismatch: expected {expected} bytes, downloaded {actual_size}"
            ));
        }
    }
    if let Some(expected) = expected_sha256.filter(|s| !s.is_empty()) {
        if !expected.eq_ignore_ascii_case(actual_sha256) {
            return Err(anyhow!(
                "Backup checksum mismatch: expected sha256 {expected}, got {actual_sha256}"
            ));
        }
    }
    Ok(())
}

/// Hex SHA-256 and size of a file, read in chunks.
pub async fn sha256_file(path: &Path) -> Result<(String, u64)> {
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("Failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut size = 0u64;
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        size += n as u64;
        hasher.update(&buf[..n]);
    }
    Ok((hex::encode(hasher.finalize()), size))
}

/// Insert a pending `s3_backups` row (status `pending`) for an upload that starts
/// later. Returns its id.
pub async fn insert_pending(
    db: &DbPool,
    config: &S3StorageConfig,
    kind: &str,
    source_id: Option<&str>,
    key: &str,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO s3_backups (id, storage_config_id, backup_type, source_id, s3_key, status, team_id) \
         VALUES (?, ?, ?, ?, ?, 'pending', ?)",
    )
    .bind(&id)
    .bind(&config.id)
    .bind(kind)
    .bind(source_id)
    .bind(key)
    .bind(&config.team_id)
    .execute(db)
    .await?;
    Ok(id)
}

/// Mark a pending `s3_backups` row failed.
pub async fn mark_failed(db: &DbPool, id: &str, error: &str) {
    let _ = sqlx::query("UPDATE s3_backups SET status = 'failed', error_message = ? WHERE id = ?")
        .bind(error)
        .bind(id)
        .execute(db)
        .await;
}

/// Upload `local_path` for the existing row `row_id` (key derived from the file
/// name), storing size and checksum. The row ends `completed` or `failed`.
pub async fn upload_into_record(
    db: &DbPool,
    encryption_secret: Option<&str>,
    config: &S3StorageConfig,
    row_id: &str,
    kind: &str,
    source_id: Option<&str>,
    local_path: &Path,
) -> Result<S3Backup> {
    let result = async {
        let file_name = local_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Backup file has no usable name"))?;
        let key = backup_key(kind, source_id, file_name);
        let (sha256, size) = sha256_file(local_path).await?;
        sqlx::query("UPDATE s3_backups SET status = 'uploading', s3_key = ? WHERE id = ?")
            .bind(&key)
            .bind(row_id)
            .execute(db)
            .await?;
        let client = client_for(config, encryption_secret)?;
        client.upload_file(&key, local_path).await?;
        sqlx::query(
            "UPDATE s3_backups SET status = 'completed', size_bytes = ?, checksum_sha256 = ?, \
             error_message = NULL WHERE id = ?",
        )
        .bind(size as i64)
        .bind(&sha256)
        .bind(row_id)
        .execute(db)
        .await?;
        Ok::<_, anyhow::Error>(())
    }
    .await;
    if let Err(e) = result {
        mark_failed(db, row_id, &format!("{e:#}")).await;
        return Err(e);
    }
    Ok(
        sqlx::query_as::<_, S3Backup>("SELECT * FROM s3_backups WHERE id = ?")
            .bind(row_id)
            .fetch_one(db)
            .await?,
    )
}

/// Upload a local backup file and record it in `s3_backups`. Returns the new row.
pub async fn upload_and_record(
    db: &DbPool,
    encryption_secret: Option<&str>,
    config: &S3StorageConfig,
    kind: &str,
    source_id: Option<&str>,
    local_path: &Path,
) -> Result<S3Backup> {
    let file_name = local_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("Backup file has no usable name"))?;
    let id = insert_pending(
        db,
        config,
        kind,
        source_id,
        &backup_key(kind, source_id, file_name),
    )
    .await?;
    upload_into_record(
        db,
        encryption_secret,
        config,
        &id,
        kind,
        source_id,
        local_path,
    )
    .await
}

/// Download an S3 backup to `dest` and verify it against its record.
pub async fn download_verified(
    db: &DbPool,
    encryption_secret: Option<&str>,
    backup: &S3Backup,
    dest: &Path,
    max_bytes: u64,
) -> Result<u64> {
    let config = load_config(db, &backup.storage_config_id).await?;
    let client = client_for(&config, encryption_secret)?;
    let downloaded = client
        .download_to_file(&backup.s3_key, dest, max_bytes)
        .await?;
    if let Err(e) = verify_download(
        backup.size_bytes,
        backup.checksum_sha256.as_deref(),
        downloaded.content_length,
        downloaded.size,
        &downloaded.sha256,
    ) {
        let _ = tokio::fs::remove_file(dest).await;
        return Err(e);
    }
    Ok(downloaded.size)
}

/// List a resource's backups in every given bucket and add rows for objects that
/// `s3_backups` does not know yet (for example after a reinstall). Returns how many
/// rows were added. Configs that fail to list are skipped with a warning.
pub async fn import_bucket_backups(
    db: &DbPool,
    encryption_secret: Option<&str>,
    configs: &[S3StorageConfig],
    kind: &str,
    source_id: Option<&str>,
) -> usize {
    let mut added = 0;
    for config in configs {
        let listed = async {
            let client = client_for(config, encryption_secret)?;
            let mut objects = client
                .list_backups(&listing_prefix(kind, source_id))
                .await?;
            if kind == "instance" {
                objects.extend(client.list_backups("instance-backups/").await?);
            }
            Ok::<_, anyhow::Error>((objects, client.path_prefix().to_string()))
        }
        .await;
        let (objects, prefix) = match listed {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(config = %config.name, error = %e, "Failed to list S3 backups");
                continue;
            }
        };
        for object in objects {
            let Some(parsed) = parse_backup_key(&object.key, &prefix) else {
                continue;
            };
            if parsed.kind != kind || parsed.source_id.as_deref() != source_id {
                continue;
            }
            let known: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM s3_backups WHERE storage_config_id = ? AND s3_key = ?",
            )
            .bind(&config.id)
            .bind(&parsed.relative_key)
            .fetch_one(db)
            .await
            .unwrap_or(1);
            if known > 0 {
                continue;
            }
            let created_at = parsed
                .timestamp
                .or_else(|| {
                    object
                        .last_modified
                        .as_deref()
                        .and_then(super::missed::parse_timestamp)
                })
                .unwrap_or_else(Utc::now)
                .to_rfc3339();
            let inserted = sqlx::query(
                "INSERT INTO s3_backups (id, storage_config_id, backup_type, source_id, s3_key, \
                 size_bytes, status, team_id, created_at) VALUES (?, ?, ?, ?, ?, ?, 'completed', ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&config.id)
            .bind(kind)
            .bind(source_id)
            .bind(&parsed.relative_key)
            .bind(object.size)
            .bind(&config.team_id)
            .bind(&created_at)
            .execute(db)
            .await;
            if inserted.is_ok() {
                added += 1;
            }
        }
    }
    added
}

impl super::retention::Retained for S3Backup {
    fn created_at(&self) -> &str {
        &self.created_at
    }
    fn id(&self) -> &str {
        &self.id
    }
}

/// Keep the newest `keep` completed backups of one resource in one storage config;
/// delete older objects and their rows. Returns how many were removed.
pub async fn prune_s3_backups(
    db: &DbPool,
    encryption_secret: Option<&str>,
    config: &S3StorageConfig,
    kind: &str,
    source_id: Option<&str>,
    keep: usize,
) -> usize {
    let rows: Vec<S3Backup> = sqlx::query_as(
        "SELECT * FROM s3_backups WHERE storage_config_id = ? AND backup_type = ?          AND source_id IS ? AND status = 'completed'",
    )
    .bind(&config.id)
    .bind(kind)
    .bind(source_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let doomed = super::retention::select_for_pruning(&rows, keep);
    if doomed.is_empty() {
        return 0;
    }
    let client = match client_for(config, encryption_secret) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "S3 retention skipped: cannot build client");
            return 0;
        }
    };
    let mut removed = 0;
    for backup in doomed {
        if let Err(e) = client.delete_backup(&backup.s3_key).await {
            tracing::warn!(key = %backup.s3_key, error = %e, "S3 retention: delete failed");
            continue;
        }
        let _ = sqlx::query("DELETE FROM s3_backups WHERE id = ?")
            .bind(&backup.id)
            .execute(db)
            .await;
        // Volume history rows that pointed at this upload.
        let _ = sqlx::query("DELETE FROM volume_backups WHERE s3_backup_id = ?")
            .bind(&backup.id)
            .execute(db)
            .await;
        removed += 1;
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn keys_are_built_per_kind() {
        assert_eq!(
            backup_key("database", Some("db-1"), "orders_20261105_020000.sql"),
            "backups/database/db-1/orders_20261105_020000.sql"
        );
        assert_eq!(
            backup_key("volume", Some("v-1"), "data-20261105-020000.tar.gz"),
            "backups/volume/v-1/data-20261105-020000.tar.gz"
        );
        assert_eq!(
            backup_key("instance", None, "rivetr-backup-20261105-020000.tar.gz"),
            "backups/instance/rivetr-backup-20261105-020000.tar.gz"
        );
        assert_eq!(listing_prefix("volume", None), "backups/volume/");
    }

    #[test]
    fn listed_keys_parse_with_and_without_prefix() {
        let p = parse_backup_key("backups/database/db-1/orders_20261105_020000.sql", "").unwrap();
        assert_eq!(p.kind, "database");
        assert_eq!(p.source_id.as_deref(), Some("db-1"));
        assert_eq!(p.file_name, "orders_20261105_020000.sql");
        assert_eq!(
            p.relative_key,
            "backups/database/db-1/orders_20261105_020000.sql"
        );
        assert_eq!(
            p.timestamp,
            Some(Utc.with_ymd_and_hms(2026, 11, 5, 2, 0, 0).unwrap())
        );

        let p = parse_backup_key(
            "team-a/rivetr/backups/volume/v-9/uploads-20261104-231500.tar.gz",
            "/team-a/rivetr/",
        )
        .unwrap();
        assert_eq!(p.kind, "volume");
        assert_eq!(p.source_id.as_deref(), Some("v-9"));
        assert_eq!(
            p.relative_key,
            "backups/volume/v-9/uploads-20261104-231500.tar.gz"
        );

        let p =
            parse_backup_key("instance-backups/rivetr-backup-20261101-000000.tar.gz", "").unwrap();
        assert_eq!(p.kind, "instance");
        assert!(p.source_id.is_none());
    }

    #[test]
    fn foreign_keys_are_ignored() {
        assert!(parse_backup_key("other/backups/database/db-1/x.sql", "").is_none());
        assert!(parse_backup_key("backups/database/db-1/", "").is_none());
        assert!(parse_backup_key("backups/database/x.sql", "").is_none());
        assert!(parse_backup_key("backups/cache/db-1/x.sql", "").is_none());
        assert!(parse_backup_key("backups/volume/v/.keep", "").is_none());
        // Key outside the configured prefix.
        assert!(parse_backup_key("backups/volume/v/a.tar.gz", "prefix").is_none());
    }

    #[test]
    fn timestamps_are_found_in_file_names() {
        let ts = Utc.with_ymd_and_hms(2026, 11, 5, 2, 0, 0).unwrap();
        assert_eq!(timestamp_from_file_name("db_20261105_020000.sql"), Some(ts));
        assert_eq!(
            timestamp_from_file_name("rivetr-backup-20261105-020000.tar.gz"),
            Some(ts)
        );
        assert_eq!(timestamp_from_file_name("manual.tar.gz"), None);
        assert_eq!(timestamp_from_file_name("x_20261399_020000.sql"), None);
    }

    #[test]
    fn download_verification() {
        let sha = "ab".repeat(32);
        assert!(verify_download(Some(10), Some(&sha), Some(10), 10, &sha).is_ok());
        // Unknown size/checksum (rows imported from a bucket scan) still pass.
        assert!(verify_download(None, None, None, 10, &sha).is_ok());
        assert!(verify_download(Some(0), Some(""), None, 10, &sha).is_ok());
        // Case-insensitive checksum compare.
        assert!(verify_download(None, Some(&sha.to_uppercase()), None, 10, &sha).is_ok());

        let err = verify_download(Some(10), None, None, 9, &sha).unwrap_err();
        assert!(err.to_string().contains("size mismatch"));
        let err = verify_download(None, None, Some(12), 10, &sha).unwrap_err();
        assert!(err.to_string().contains("incomplete"));
        let err = verify_download(None, Some(&"cd".repeat(32)), None, 10, &sha).unwrap_err();
        assert!(err.to_string().contains("checksum mismatch"));
        assert!(verify_download(None, None, None, 0, &sha).is_err());
    }

    #[tokio::test]
    async fn file_checksum_matches_known_digest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        tokio::fs::write(&path, b"abc").await.unwrap();
        let (sha, size) = sha256_file(&path).await.unwrap();
        assert_eq!(size, 3);
        assert_eq!(
            sha,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
