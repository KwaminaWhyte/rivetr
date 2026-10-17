//! Encryption key bootstrap (SEC-M10).
//!
//! When `[auth] encryption_key` is not configured, secrets (env vars, SSH keys,
//! OAuth/TOTP secrets) would be stored in plaintext. This module resolves a key at
//! startup:
//!
//! 1. A configured key always wins.
//! 2. Otherwise a key file at `<data_dir>/.encryption_key` is loaded if present.
//! 3. Otherwise, on a fresh install (no users and no stored secrets), a random
//!    256-bit key is generated and written to that file (mode 0600, created
//!    atomically, never overwritten).
//! 4. Otherwise (existing install without a key) behaviour is NOT changed
//!    silently: the server keeps running without encryption and logs a warning.
//!
//! Decryption of legacy plaintext values stays tolerant (`decrypt_if_encrypted`
//! passes through values without the `ENC:` prefix), so enabling a key later never
//! breaks reads of old rows.

use anyhow::{Context, Result};
use rand::RngCore;
use sqlx::SqlitePool;
use std::io::Write;
use std::path::{Path, PathBuf};

/// File name of the auto-generated key, stored next to the database.
pub const KEY_FILE_NAME: &str = ".encryption_key";

/// How the encryption key was resolved at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyResolution {
    /// Key came from `[auth] encryption_key` in the config.
    Configured,
    /// Key was loaded from the key file.
    LoadedFromFile,
    /// A new key was generated and written to the key file (fresh install).
    Generated,
    /// No key anywhere and the install already holds data: left unencrypted.
    MissingExistingInstall,
}

/// Path of the key file for a given data directory.
pub fn key_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join(KEY_FILE_NAME)
}

/// Read the key file. Returns `Ok(None)` if it does not exist or is empty.
pub fn load_key_file(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(s) => {
            let key = s.trim().to_string();
            Ok(if key.is_empty() { None } else { Some(key) })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("Failed to read {}", path.display())),
    }
}

/// Create the key file with a fresh random key and return the key.
///
/// The file is created with `create_new` (O_EXCL) so an existing file is never
/// overwritten; on unix the mode is 0600 from creation. If the file already
/// exists (including a concurrent creator winning the race), its key is returned.
pub fn create_key_file(path: &Path) -> Result<String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let key = hex::encode(bytes);

    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    match opts.open(path) {
        Ok(mut f) => {
            f.write_all(key.as_bytes())
                .and_then(|_| f.write_all(b"\n"))
                .and_then(|_| f.sync_all())
                .with_context(|| format!("Failed to write {}", path.display()))?;
            Ok(key)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            load_key_file(path)?.with_context(|| format!("{} exists but is empty", path.display()))
        }
        Err(e) => Err(e).with_context(|| format!("Failed to create {}", path.display())),
    }
}

/// Warn if the key file is readable by group/others (unix only).
#[cfg(unix)]
fn warn_if_loose_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.permissions().mode() & 0o077 != 0 {
            tracing::warn!(
                path = %path.display(),
                "Encryption key file is accessible by other users; run: chmod 600 {}",
                path.display()
            );
        }
    }
}

#[cfg(not(unix))]
fn warn_if_loose_permissions(_path: &Path) {}

/// True when the database holds no users and no secret-bearing rows, i.e. nothing
/// that would have been stored in plaintext. Any query error counts as "not fresh"
/// so the safe (no behaviour change) path is taken.
async fn is_fresh_install(db: &SqlitePool) -> bool {
    for table in [
        "users",
        "env_vars",
        "servers",
        "build_servers",
        "git_providers",
    ] {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        match sqlx::query_scalar::<_, i64>(&sql).fetch_one(db).await {
            Ok(0) => {}
            _ => return false,
        }
    }
    true
}

/// Count env vars already stored encrypted (used to detect a lost key).
async fn count_encrypted_env_vars(db: &SqlitePool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM env_vars WHERE value LIKE 'ENC:%'")
        .fetch_one(db)
        .await
        .unwrap_or(0)
}

/// Resolve the encryption key and store it in `configured` when one is found or
/// generated. See the module docs for the decision order.
pub async fn resolve_encryption_key(
    configured: &mut Option<String>,
    data_dir: &Path,
    db: &SqlitePool,
) -> Result<KeyResolution> {
    let path = key_file_path(data_dir);

    if configured.is_some() {
        if path.exists() {
            tracing::info!(
                path = %path.display(),
                "Both [auth] encryption_key and a key file exist; using the configured key"
            );
        }
        return Ok(KeyResolution::Configured);
    }

    if let Some(key) = load_key_file(&path)? {
        warn_if_loose_permissions(&path);
        tracing::info!(path = %path.display(), "Loaded encryption key from key file");
        *configured = Some(key);
        return Ok(KeyResolution::LoadedFromFile);
    }

    if is_fresh_install(db).await {
        let key = create_key_file(&path)?;
        tracing::warn!(
            path = %path.display(),
            "No encryption_key configured: generated a new one for this fresh install. \
             Secrets are now encrypted at rest. BACK UP {} together with the database: \
             without it encrypted values cannot be recovered.",
            path.display()
        );
        *configured = Some(key);
        return Ok(KeyResolution::Generated);
    }

    let encrypted = count_encrypted_env_vars(db).await;
    if encrypted > 0 {
        tracing::error!(
            encrypted_env_vars = encrypted,
            "Encrypted values exist in the database but no encryption key is configured and \
             {} is missing. Restore the original key to read them: add `encryption_key = \"...\"` \
             under [auth] in rivetr.toml, or restore the key file.",
            path.display()
        );
    }
    tracing::warn!(
        "SECURITY: no encryption key is configured; env vars, SSH keys, OAuth and TOTP secrets \
         are stored in PLAINTEXT. To enable encryption add this to rivetr.toml:\n\
         [auth]\n\
         encryption_key = \"<output of: openssl rand -hex 32>\"\n\
         (or create {} containing the key, mode 600). Existing plaintext values stay readable; \
         new and updated values will be encrypted. Keep the key backed up.",
        path.display()
    );
    Ok(KeyResolution::MissingExistingInstall)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool_with(rows: &[&str]) -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for t in ["users", "servers", "build_servers", "git_providers"] {
            sqlx::query(&format!("CREATE TABLE {t} (id TEXT)"))
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query("CREATE TABLE env_vars (id TEXT, value TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        for t in rows {
            sqlx::query(&format!("INSERT INTO {t} (id) VALUES ('x')"))
                .execute(&pool)
                .await
                .unwrap();
        }
        pool
    }

    #[test]
    fn create_key_file_writes_hex_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = key_file_path(dir.path());
        let key = create_key_file(&path).unwrap();
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(load_key_file(&path).unwrap().as_deref(), Some(key.as_str()));
    }

    #[test]
    fn create_key_file_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = key_file_path(dir.path());
        let first = create_key_file(&path).unwrap();
        let second = create_key_file(&path).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            load_key_file(&path).unwrap().as_deref(),
            Some(first.as_str())
        );
    }

    #[cfg(unix)]
    #[test]
    fn create_key_file_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = key_file_path(dir.path());
        create_key_file(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn load_missing_or_empty_key_file_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = key_file_path(dir.path());
        assert_eq!(load_key_file(&path).unwrap(), None);
        std::fs::write(&path, "  \n").unwrap();
        assert_eq!(load_key_file(&path).unwrap(), None);
    }

    #[tokio::test]
    async fn fresh_install_generates_then_reuses_key() {
        let dir = tempfile::tempdir().unwrap();
        let db = pool_with(&[]).await;

        let mut key = None;
        let r = resolve_encryption_key(&mut key, dir.path(), &db)
            .await
            .unwrap();
        assert_eq!(r, KeyResolution::Generated);
        let generated = key.clone().unwrap();

        // Next boot: same key from the file, even though the db is still empty.
        let mut key2 = None;
        let r = resolve_encryption_key(&mut key2, dir.path(), &db)
            .await
            .unwrap();
        assert_eq!(r, KeyResolution::LoadedFromFile);
        assert_eq!(key2.unwrap(), generated);
    }

    #[tokio::test]
    async fn existing_install_is_left_unencrypted() {
        let dir = tempfile::tempdir().unwrap();
        let db = pool_with(&["users"]).await;
        let mut key = None;
        let r = resolve_encryption_key(&mut key, dir.path(), &db)
            .await
            .unwrap();
        assert_eq!(r, KeyResolution::MissingExistingInstall);
        assert!(key.is_none());
        assert!(!key_file_path(dir.path()).exists());
    }

    #[tokio::test]
    async fn existing_install_loads_key_file_if_present() {
        let dir = tempfile::tempdir().unwrap();
        let db = pool_with(&["users"]).await;
        std::fs::write(key_file_path(dir.path()), "abc123\n").unwrap();
        let mut key = None;
        let r = resolve_encryption_key(&mut key, dir.path(), &db)
            .await
            .unwrap();
        assert_eq!(r, KeyResolution::LoadedFromFile);
        assert_eq!(key.as_deref(), Some("abc123"));
    }

    #[tokio::test]
    async fn configured_key_wins_over_key_file() {
        let dir = tempfile::tempdir().unwrap();
        let db = pool_with(&[]).await;
        std::fs::write(key_file_path(dir.path()), "from-file").unwrap();
        let mut key = Some("from-config".to_string());
        let r = resolve_encryption_key(&mut key, dir.path(), &db)
            .await
            .unwrap();
        assert_eq!(r, KeyResolution::Configured);
        assert_eq!(key.as_deref(), Some("from-config"));
    }
}
