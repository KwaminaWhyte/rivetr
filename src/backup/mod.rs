//! Instance Backup & Restore module.
//!
//! Provides functionality to backup and restore the entire Rivetr instance:
//! - SQLite database (with WAL checkpoint)
//! - Configuration file (rivetr.toml)
//! - SSL/ACME certificates
//! - S3 remote backup integration
//! - Service container database dumps (postgres, mysql, redis, mongo)

pub mod s3;

use anyhow::{Context, Result};
use chrono::Utc;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Serialize;
use sqlx::SqlitePool;
use std::fs;
use std::io::{Read as IoRead, Write as IoWrite};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};
use tokio::process::Command as TokioCommand;
use tracing::{info, warn};

/// Information about a backup file
#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    /// Filename of the backup
    pub name: String,
    /// Size in bytes
    pub size: u64,
    /// ISO 8601 timestamp when the backup was created
    pub created_at: String,
}

/// Result of a backup operation
#[derive(Debug, Clone, Serialize)]
pub struct BackupResult {
    /// Full path to the backup file
    pub path: PathBuf,
    /// Filename of the backup
    pub name: String,
    /// Size in bytes
    pub size: u64,
}

/// Result of a restore operation
#[derive(Debug, Clone, Serialize)]
pub struct RestoreResult {
    /// Whether the database was restored
    pub database_restored: bool,
    /// Whether the config was restored
    pub config_restored: bool,
    /// Whether SSL certificates were restored
    pub certs_restored: bool,
    /// Warning messages
    pub warnings: Vec<String>,
}

/// Create a backup of the Rivetr instance.
///
/// This will:
/// 1. Checkpoint the SQLite WAL to flush all pending writes
/// 2. Copy the database file
/// 3. Copy the config file
/// 4. Copy SSL/ACME certificates (if they exist)
/// 5. Dump database containers from running Docker Compose services
/// 6. Bundle everything into a .tar.gz archive
pub async fn create_backup(
    db: &SqlitePool,
    data_dir: &Path,
    config_path: &Path,
    acme_cache_dir: &Path,
    output_path: Option<&Path>,
) -> Result<BackupResult> {
    // 1. Checkpoint the SQLite WAL to ensure all data is written to the main DB file
    info!("Checkpointing SQLite WAL...");
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(db)
        .await
        .context("Failed to checkpoint SQLite WAL")?;

    // Determine output path
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let backup_filename = format!("rivetr-backup-{}.tar.gz", timestamp);

    let backup_path = if let Some(out) = output_path {
        out.to_path_buf()
    } else {
        let backups_dir = data_dir.join("backups");
        fs::create_dir_all(&backups_dir).context("Failed to create backups directory")?;
        backups_dir.join(&backup_filename)
    };

    // Create the tar.gz archive
    info!("Creating backup archive at {}...", backup_path.display());
    let file = fs::File::create(&backup_path).context("Failed to create backup file")?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut archive = Builder::new(encoder);

    // 2. Add the SQLite database file
    let db_path = data_dir.join("rivetr.db");
    if db_path.exists() {
        info!("Adding database to backup...");
        archive
            .append_path_with_name(&db_path, "rivetr.db")
            .context("Failed to add database to backup")?;
    } else {
        warn!("Database file not found at {}", db_path.display());
    }

    // 3. Add the config file
    if config_path.exists() {
        info!("Adding config file to backup...");
        archive
            .append_path_with_name(config_path, "rivetr.toml")
            .context("Failed to add config to backup")?;
    } else {
        warn!("Config file not found at {}", config_path.display());
    }

    // 4. Add SSL/ACME certificates directory (if it exists)
    if acme_cache_dir.exists() && acme_cache_dir.is_dir() {
        info!("Adding ACME/SSL certificates to backup...");
        add_directory_to_archive(&mut archive, acme_cache_dir, Path::new("acme"))
            .context("Failed to add ACME certificates to backup")?;
    }

    // 5. Dump database containers from running Docker Compose services
    info!("Backing up service container databases...");
    backup_service_databases(db, data_dir, &mut archive).await;

    // Finish the archive
    let encoder = archive
        .into_inner()
        .context("Failed to finalize tar archive")?;
    encoder
        .finish()
        .context("Failed to finish gzip compression")?;

    // Get the file size
    let metadata = fs::metadata(&backup_path).context("Failed to read backup file metadata")?;

    let result = BackupResult {
        path: backup_path,
        name: backup_filename,
        size: metadata.len(),
    };

    info!(
        "Backup created successfully: {} ({} bytes)",
        result.name, result.size
    );

    Ok(result)
}

/// List all backups in the data/backups/ directory
pub fn list_backups(data_dir: &Path) -> Result<Vec<BackupInfo>> {
    let backups_dir = data_dir.join("backups");
    if !backups_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&backups_dir).context("Failed to read backups directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Only include .tar.gz files that start with "rivetr-backup-"
            if name.starts_with("rivetr-backup-") && name.ends_with(".tar.gz") {
                let metadata = fs::metadata(&path)?;
                let created_at = metadata
                    .created()
                    .or_else(|_| metadata.modified())
                    .map(|t| {
                        let datetime: chrono::DateTime<Utc> = t.into();
                        datetime.to_rfc3339()
                    })
                    .unwrap_or_else(|_| "unknown".to_string());

                backups.push(BackupInfo {
                    name,
                    size: metadata.len(),
                    created_at,
                });
            }
        }
    }

    // Sort by name descending (newest first, since names contain timestamps)
    backups.sort_by(|a, b| b.name.cmp(&a.name));

    Ok(backups)
}

/// Delete a specific backup file
pub fn delete_backup(data_dir: &Path, name: &str) -> Result<()> {
    // Validate the backup name to prevent path traversal
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        anyhow::bail!("Invalid backup name");
    }

    if !name.starts_with("rivetr-backup-") || !name.ends_with(".tar.gz") {
        anyhow::bail!("Invalid backup name format");
    }

    let backup_path = data_dir.join("backups").join(name);
    if !backup_path.exists() {
        anyhow::bail!("Backup not found: {}", name);
    }

    fs::remove_file(&backup_path).context("Failed to delete backup file")?;
    info!("Deleted backup: {}", name);
    Ok(())
}

/// Restore from a backup archive.
///
/// This will:
/// 1. Extract and validate the tar.gz archive
/// 2. Replace the database file
/// 3. Replace the config file (if included)
/// 4. Replace SSL certificates (if included)
pub async fn restore_from_backup(
    backup_data: &[u8],
    data_dir: &Path,
    config_path: &Path,
    acme_cache_dir: &Path,
) -> Result<RestoreResult> {
    let mut result = RestoreResult {
        database_restored: false,
        config_restored: false,
        certs_restored: false,
        warnings: Vec::new(),
    };

    // Extract the tar.gz archive
    info!("Extracting backup archive...");
    let decoder = GzDecoder::new(backup_data);
    let mut archive = Archive::new(decoder);

    // Create a temporary directory for extraction
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;

    archive
        .unpack(temp_dir.path())
        .context("Failed to extract backup archive")?;

    // Validate: must contain rivetr.db at minimum
    let extracted_db = temp_dir.path().join("rivetr.db");
    if !extracted_db.exists() {
        anyhow::bail!("Invalid backup: missing rivetr.db database file");
    }

    // Restore database
    info!("Restoring database...");
    let target_db = data_dir.join("rivetr.db");

    // Also remove WAL and SHM files if they exist
    let wal_path = data_dir.join("rivetr.db-wal");
    let shm_path = data_dir.join("rivetr.db-shm");

    fs::copy(&extracted_db, &target_db).context("Failed to restore database file")?;
    result.database_restored = true;

    // Remove WAL/SHM files so SQLite starts fresh
    if wal_path.exists() {
        let _ = fs::remove_file(&wal_path);
    }
    if shm_path.exists() {
        let _ = fs::remove_file(&shm_path);
    }

    // Restore config file (if included)
    let extracted_config = temp_dir.path().join("rivetr.toml");
    if extracted_config.exists() {
        info!("Restoring config file...");
        fs::copy(&extracted_config, config_path).context("Failed to restore config file")?;
        result.config_restored = true;
    } else {
        result
            .warnings
            .push("Backup did not contain a config file".to_string());
    }

    // Restore ACME/SSL certificates (if included)
    let extracted_acme = temp_dir.path().join("acme");
    if extracted_acme.exists() && extracted_acme.is_dir() {
        info!("Restoring ACME/SSL certificates...");
        // Remove existing acme dir and replace with backup
        if acme_cache_dir.exists() {
            fs::remove_dir_all(acme_cache_dir)
                .context("Failed to remove existing ACME directory")?;
        }
        copy_dir_all(&extracted_acme, acme_cache_dir)
            .context("Failed to restore ACME certificates")?;
        result.certs_restored = true;
    } else {
        result
            .warnings
            .push("Backup did not contain SSL certificates".to_string());
    }

    result
        .warnings
        .push("Server restart recommended after restore".to_string());

    info!("Restore completed successfully");
    Ok(result)
}

// ---------------------------------------------------------------------------
// Service container database backup helpers
// ---------------------------------------------------------------------------

/// A discovered container belonging to a Docker Compose service.
struct ServiceContainer {
    /// The full container name (e.g. `rivetr-svc-myservice-postgres-1`)
    name: String,
    /// The image name in lowercase (used to detect DB type)
    image: String,
}

/// Classify a container by image name and return the DB engine, or `None` if
/// the image does not look like a known database.
fn detect_db_engine(image: &str) -> Option<&'static str> {
    let img = image.to_lowercase();
    if img.contains("postgres") || img.contains("postgresql") {
        Some("postgres")
    } else if img.contains("mariadb") {
        // Check mariadb before mysql so mariadb images aren't misidentified
        Some("mariadb")
    } else if img.contains("mysql") {
        Some("mysql")
    } else if img.contains("redis") || img.contains("keydb") || img.contains("dragonfly") {
        Some("redis")
    } else if img.contains("mongo") {
        Some("mongo")
    } else {
        None
    }
}

/// Use `docker compose ps` to list containers for a service's compose project.
///
/// Returns a list of running containers (name + image).
async fn list_service_containers(compose_file: &Path, project_name: &str) -> Vec<ServiceContainer> {
    // docker compose -f <file> -p <project> ps --format "{{.Name}}\t{{.Image}}\t{{.State}}"
    let output = TokioCommand::new("docker")
        .args([
            "compose",
            "-f",
            compose_file.to_str().unwrap_or(""),
            "-p",
            project_name,
            "ps",
            "--format",
            "{{.Name}}\t{{.Image}}\t{{.State}}",
        ])
        .output()
        .await;

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            warn!(compose_file=%compose_file.display(), error=%e, "Failed to run docker compose ps");
            return Vec::new();
        }
    };

    if !output.status.success() {
        warn!(
            compose_file = %compose_file.display(),
            stderr = %String::from_utf8_lossy(&output.stderr),
            "docker compose ps returned non-zero exit code"
        );
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut containers = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let container_name = parts[0].trim();
        let image = parts[1].trim();
        let state = parts[2].trim();

        if state != "running" {
            continue;
        }

        containers.push(ServiceContainer {
            name: container_name.to_string(),
            image: image.to_string(),
        });
    }

    containers
}

/// Run `pg_dumpall -U postgres` inside a container and return the SQL bytes.
async fn dump_postgres(container: &str) -> Result<Vec<u8>> {
    let out = TokioCommand::new("docker")
        .args(["exec", container, "pg_dumpall", "-U", "postgres"])
        .output()
        .await
        .context("Failed to exec pg_dumpall")?;

    if out.status.success() {
        Ok(out.stdout)
    } else {
        anyhow::bail!(
            "pg_dumpall failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    }
}

/// Run `mysqldump --all-databases` inside a container.
///
/// Tries without a password first (empty root password), then falls back to
/// reading `MYSQL_ROOT_PASSWORD` from the container environment.
async fn dump_mysql(container: &str) -> Result<Vec<u8>> {
    // First attempt: no password (works when MYSQL_ALLOW_EMPTY_PASSWORD is set)
    let out = TokioCommand::new("docker")
        .args([
            "exec",
            container,
            "mysqldump",
            "--all-databases",
            "-u",
            "root",
            "--password=",
        ])
        .output()
        .await
        .context("Failed to exec mysqldump")?;

    if out.status.success() && !out.stdout.is_empty() {
        return Ok(out.stdout);
    }

    // Second attempt: read MYSQL_ROOT_PASSWORD from container env
    let env_out = TokioCommand::new("docker")
        .args(["exec", container, "printenv", "MYSQL_ROOT_PASSWORD"])
        .output()
        .await;

    if let Ok(env_result) = env_out {
        if env_result.status.success() {
            let password = String::from_utf8_lossy(&env_result.stdout)
                .trim()
                .to_string();
            if !password.is_empty() {
                let pass_flag = format!("--password={}", password);
                let dump = TokioCommand::new("docker")
                    .args([
                        "exec",
                        container,
                        "mysqldump",
                        "--all-databases",
                        "-u",
                        "root",
                        &pass_flag,
                    ])
                    .output()
                    .await
                    .context("Failed to exec mysqldump with password")?;

                if dump.status.success() {
                    return Ok(dump.stdout);
                }
            }
        }
    }

    anyhow::bail!("mysqldump failed: {}", String::from_utf8_lossy(&out.stderr))
}

/// Trigger a Redis `BGSAVE` then read the dump file from the container.
async fn dump_redis(container: &str) -> Result<Vec<u8>> {
    // Trigger background save
    let _ = TokioCommand::new("docker")
        .args(["exec", container, "redis-cli", "BGSAVE"])
        .output()
        .await;

    // Give Redis a moment to finish writing the RDB
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Try the default dump location; some images use /data/dump.rdb
    for path in &["/data/dump.rdb", "/var/lib/redis/dump.rdb"] {
        let out = TokioCommand::new("docker")
            .args(["exec", container, "cat", path])
            .output()
            .await;

        if let Ok(result) = out {
            if result.status.success() && !result.stdout.is_empty() {
                return Ok(result.stdout);
            }
        }
    }

    anyhow::bail!("redis dump.rdb not found in container {}", container)
}

/// Run `mongodump --archive` inside a container and return the BSON archive bytes.
async fn dump_mongo(container: &str) -> Result<Vec<u8>> {
    let out = TokioCommand::new("docker")
        .args(["exec", container, "mongodump", "--archive"])
        .output()
        .await
        .context("Failed to exec mongodump")?;

    if out.status.success() {
        Ok(out.stdout)
    } else {
        anyhow::bail!("mongodump failed: {}", String::from_utf8_lossy(&out.stderr))
    }
}

/// Query all running services from the database and, for each one, attempt to
/// dump any database containers belonging to its Docker Compose stack.
///
/// All failures are non-fatal: a warning is logged and the backup continues.
/// The compose file for each service is also added to the archive.
async fn backup_service_databases<W: IoWrite>(
    db: &SqlitePool,
    data_dir: &Path,
    archive: &mut Builder<W>,
) {
    // Load all services from the database.  A query failure means we simply skip.
    let services: Vec<(String, String, String)> = match sqlx::query_as(
        "SELECT id, name, status FROM services ORDER BY name",
    )
    .fetch_all(db)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            warn!(error=%e, "Failed to query services for backup — skipping service database dumps");
            return;
        }
    };

    if services.is_empty() {
        info!("No services found — skipping service database dumps");
        return;
    }

    for (service_id, service_name, service_status) in &services {
        // Compose project name mirrors Service::compose_project_name()
        let project_name = format!("rivetr-svc-{}", service_name);

        // The compose file lives at data_dir/services/{name}/docker-compose.yml
        let compose_dir = data_dir.join("services").join(service_name);
        let compose_file = compose_dir.join("docker-compose.yml");

        // --- Always include the compose file itself (if it exists) ---
        if compose_file.exists() {
            let archive_path = format!("services/{}/docker-compose.yml", service_name);
            match archive.append_path_with_name(&compose_file, &archive_path) {
                Ok(()) => {
                    info!(service=%service_name, "Added compose file to backup");
                }
                Err(e) => {
                    warn!(service=%service_name, error=%e, "Failed to add compose file to backup");
                }
            }
        } else {
            info!(
                service = %service_name,
                "No compose file found on disk — skipping compose file for this service"
            );
        }

        // --- Only dump running services ---
        if service_status != "running" {
            info!(service=%service_name, status=%service_status, "Service not running — skipping DB dump");
            continue;
        }

        if !compose_file.exists() {
            // Nothing to enumerate containers from
            continue;
        }

        info!(service=%service_name, "Discovering database containers...");
        let containers = list_service_containers(&compose_file, &project_name).await;

        if containers.is_empty() {
            info!(service=%service_name, "No running containers found for service");
            continue;
        }

        for container in &containers {
            let engine = match detect_db_engine(&container.image) {
                Some(e) => e,
                None => {
                    // Not a recognised database image — skip silently
                    continue;
                }
            };

            info!(
                service = %service_name,
                container = %container.name,
                engine = %engine,
                "Dumping database container"
            );

            let dump_result = match engine {
                "postgres" => dump_postgres(&container.name).await,
                "mysql" | "mariadb" => dump_mysql(&container.name).await,
                "redis" => dump_redis(&container.name).await,
                "mongo" => dump_mongo(&container.name).await,
                _ => continue,
            };

            match dump_result {
                Ok(dump_data) => {
                    let ext = match engine {
                        "redis" => "rdb",
                        _ => "sql",
                    };
                    // Use the full container name (may contain slashes on some Docker
                    // versions) — strip any leading slash for safety.
                    let safe_container = container.name.trim_start_matches('/');
                    let archive_path =
                        format!("services/{}/{}/dump.{}", service_name, safe_container, ext);

                    // Build a tar header for the in-memory bytes
                    let mut header = tar::Header::new_gnu();
                    header.set_size(dump_data.len() as u64);
                    header.set_mode(0o644);
                    header.set_cksum();

                    match archive.append_data(&mut header, &archive_path, dump_data.as_slice()) {
                        Ok(()) => {
                            info!(
                                service = %service_name,
                                container = %container.name,
                                size = dump_data.len(),
                                "DB dump added to backup archive"
                            );
                        }
                        Err(e) => {
                            warn!(
                                service = %service_name,
                                container = %container.name,
                                error = %e,
                                "Failed to add DB dump to archive"
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        service = %service_name,
                        container = %container.name,
                        engine = %engine,
                        error = %e,
                        "Service DB dump failed — skipping (backup will continue)"
                    );
                }
            }
        }

        let _ = service_id; // suppress unused-variable warning
    }
}

/// Recursively add a directory to a tar archive
fn add_directory_to_archive<W: IoWrite>(
    archive: &mut Builder<W>,
    source_dir: &Path,
    archive_prefix: &Path,
) -> Result<()> {
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let path = entry.path();
        let relative = archive_prefix.join(entry.file_name());

        if path.is_file() {
            archive.append_path_with_name(&path, &relative)?;
        } else if path.is_dir() {
            add_directory_to_archive(archive, &path, &relative)?;
        }
    }
    Ok(())
}

/// Recursively copy a directory
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Read a backup file from the backups directory (for download)
pub fn read_backup_file(data_dir: &Path, name: &str) -> Result<Vec<u8>> {
    // Validate the backup name to prevent path traversal
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        anyhow::bail!("Invalid backup name");
    }

    if !name.starts_with("rivetr-backup-") || !name.ends_with(".tar.gz") {
        anyhow::bail!("Invalid backup name format");
    }

    let backup_path = data_dir.join("backups").join(name);
    if !backup_path.exists() {
        anyhow::bail!("Backup not found: {}", name);
    }

    let mut file = fs::File::open(&backup_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    Ok(data)
}
