//! `rivetr db encrypt-secrets`: encrypt secrets that were stored in plaintext before
//! an encryption key existed.
//!
//! Enabling a key only protects values written afterwards (legacy plaintext keeps
//! decrypting, so nothing breaks). This command encrypts the rows already in the
//! database. Only columns whose write path encrypts and whose read path decrypts are
//! listed, so the application reads every value back exactly as before.

use anyhow::{bail, Context, Result};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

use super::Cli;
use crate::config::Config;
use crate::crypto::{self, key_file};

/// `(table, column)` pairs the application encrypts on write and decrypts on read.
const SECRET_COLUMNS: &[(&str, &str)] = &[
    ("env_vars", "value"),
    ("environment_env_vars", "value"),
    ("project_env_vars", "value"),
    ("team_env_vars", "value"),
    ("servers", "ssh_private_key"),
    ("servers", "ssh_password"),
    ("build_servers", "ssh_private_key"),
    ("build_servers", "ssh_password"),
    ("s3_storage_configs", "access_key"),
    ("s3_storage_configs", "secret_key"),
    ("users", "totp_secret"),
    ("github_apps", "client_secret"),
    ("github_apps", "private_key"),
    ("github_apps", "webhook_secret"),
];

/// Plaintext rows found in one column (encrypted when `execute` is true).
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ColumnReport {
    pub table: &'static str,
    pub column: &'static str,
    pub plaintext_rows: usize,
}

/// Encrypt every plaintext value in [`SECRET_COLUMNS`] inside one transaction.
///
/// Rows that are NULL, empty or already `ENC:`/`ENC2:` are left alone, so the
/// command is safe to run again. Each value is decrypted back and compared before it
/// is written; any mismatch aborts and rolls everything back. With `execute` false
/// nothing is written.
pub(crate) async fn encrypt_secret_columns(
    pool: &SqlitePool,
    key: &[u8; 32],
    execute: bool,
) -> Result<Vec<ColumnReport>> {
    let mut tx = pool.begin().await?;
    let mut reports = Vec::new();

    for &(table, column) in SECRET_COLUMNS {
        let present: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pragma_table_info(?) WHERE name = ?")
                .bind(table)
                .bind(column)
                .fetch_one(&mut *tx)
                .await?;
        if present == 0 {
            continue;
        }

        // SAFETY: `table` and `column` come from the `SECRET_COLUMNS` constant above.
        let select = format!(
            "SELECT rowid, {column} FROM {table} WHERE {column} IS NOT NULL AND {column} != '' \
             AND {column} NOT LIKE 'ENC:%' AND {column} NOT LIKE 'ENC2:%'"
        );
        let rows: Vec<(i64, String)> = sqlx::query_as(sqlx::AssertSqlSafe(select.as_str()))
            .fetch_all(&mut *tx)
            .await
            .with_context(|| format!("Failed to read {table}.{column}"))?;

        if execute {
            // SAFETY: as above, only constant table and column names.
            let update = format!("UPDATE {table} SET {column} = ? WHERE rowid = ?");
            for (rowid, plain) in &rows {
                let encrypted = crypto::encrypt(plain, key)?;
                let back = crypto::decrypt(&encrypted, key)?;
                if back != *plain {
                    bail!(
                        "{table}.{column} row {rowid}: the encrypted value did not decrypt back \
                         to the original. Nothing was changed."
                    );
                }
                sqlx::query(sqlx::AssertSqlSafe(update.as_str()))
                    .bind(&encrypted)
                    .bind(rowid)
                    .execute(&mut *tx)
                    .await
                    .with_context(|| format!("Failed to update {table}.{column}"))?;
            }
        }

        reports.push(ColumnReport {
            table,
            column,
            plaintext_rows: rows.len(),
        });
    }

    if execute {
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }
    Ok(reports)
}

/// Entry point for `rivetr db encrypt-secrets`.
pub async fn cmd_encrypt_secrets(cli: &Cli, execute: bool) -> Result<()> {
    let config = Config::load(&cli.config)?;
    let data_dir = &config.server.data_dir;

    let db_path = data_dir.join("rivetr.db");
    if !db_path.exists() {
        bail!(
            "Database not found at {}. Is Rivetr initialized?",
            db_path.display()
        );
    }
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite:{}?mode=rw", db_path.display()))
        .await
        .context("Failed to connect to database")?;

    println!();
    println!("=== Rivetr Secret Encryption ===");
    println!();
    if !execute {
        println!("[DRY RUN] Nothing is changed. Use --execute to encrypt.");
        println!();
    }

    // Key: config > key file > (only with --execute) create the key file.
    let key_path = key_file::key_file_path(data_dir);
    let secret = match config.auth.encryption_key.clone() {
        Some(key) => {
            println!(
                "Key: from [auth] encryption_key in {}",
                cli.config.display()
            );
            Some(key)
        }
        None => match key_file::load_key_file(&key_path)? {
            Some(key) => {
                println!("Key: from {}", key_path.display());
                Some(key)
            }
            None if execute => {
                let key = key_file::create_key_file(&key_path)?;
                println!("Key: created {} (mode 600)", key_path.display());
                Some(key)
            }
            None => {
                println!(
                    "Key: none yet; --execute will create {} (mode 600)",
                    key_path.display()
                );
                None
            }
        },
    };

    if execute {
        // Per-install salt for the v2 ciphertext format, as the server does at startup.
        match key_file::resolve_install_salt(true, data_dir, &pool).await? {
            Some(salt) => {
                crypto::set_install_salt(salt);
            }
            None => bail!(
                "The salt file {} is missing but the database holds ENC2 values. Restore it \
                 from your backup before running this command.",
                key_file::salt_file_path(data_dir).display()
            ),
        }
    }

    // A dry run never encrypts, so it does not need a real key.
    let key = secret
        .as_deref()
        .map(crypto::derive_key)
        .unwrap_or([0u8; 32]);
    let reports = encrypt_secret_columns(&pool, &key, execute).await?;

    println!();
    println!("{:<34} {:>10}", "table.column", "plaintext");
    for r in &reports {
        if r.plaintext_rows > 0 {
            println!(
                "{:<34} {:>10}",
                format!("{}.{}", r.table, r.column),
                r.plaintext_rows
            );
        }
    }
    let total: usize = reports.iter().map(|r| r.plaintext_rows).sum();
    println!();
    if total == 0 {
        println!("No plaintext secrets found. Nothing to do.");
        return Ok(());
    }
    if execute {
        println!("Encrypted {total} value(s).");
        println!();
        println!("Next steps:");
        println!(
            "  1. Back up {} and {} together with the database.",
            key_path.display(),
            key_file::salt_file_path(data_dir).display()
        );
        println!("     Without them the encrypted values cannot be recovered.");
        println!("  2. Restart Rivetr so it loads the key.");
    } else {
        println!("{total} plaintext value(s) would be encrypted.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for ddl in [
            "CREATE TABLE env_vars (id TEXT PRIMARY KEY, key TEXT, value TEXT NOT NULL)",
            "CREATE TABLE servers (id TEXT PRIMARY KEY, ssh_private_key TEXT, ssh_password TEXT)",
            "CREATE TABLE users (id TEXT PRIMARY KEY, totp_secret TEXT)",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        pool
    }

    async fn value(pool: &SqlitePool, sql: &str) -> Option<String> {
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_string()))
            .fetch_one(pool)
            .await
            .unwrap()
    }

    fn key() -> [u8; 32] {
        crypto::derive_key("test-secret-for-encrypt-secrets")
    }

    async fn seed(pool: &SqlitePool) -> String {
        let already = crypto::encrypt("already", &key()).unwrap();
        sqlx::query(
            "INSERT INTO env_vars VALUES ('a', 'K1', 'plain-1'), ('b', 'K2', ?), ('c', 'K3', '')",
        )
        .bind(&already)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO servers VALUES ('s1', 'PRIVATE-KEY-TEXT', NULL)")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO users VALUES ('u1', 'TOTPSECRET')")
            .execute(pool)
            .await
            .unwrap();
        already
    }

    #[tokio::test]
    async fn dry_run_reports_plaintext_and_changes_nothing() {
        let pool = pool().await;
        seed(&pool).await;

        let reports = encrypt_secret_columns(&pool, &key(), false).await.unwrap();

        let count = |t: &str, c: &str| {
            reports
                .iter()
                .find(|r| r.table == t && r.column == c)
                .map(|r| r.plaintext_rows)
        };
        assert_eq!(count("env_vars", "value"), Some(1));
        assert_eq!(count("servers", "ssh_private_key"), Some(1));
        assert_eq!(count("servers", "ssh_password"), Some(0));
        assert_eq!(count("users", "totp_secret"), Some(1));
        assert_eq!(
            count("github_apps", "private_key"),
            None,
            "absent tables are skipped"
        );
        assert_eq!(
            value(&pool, "SELECT value FROM env_vars WHERE id = 'a'")
                .await
                .unwrap(),
            "plain-1"
        );
    }

    #[tokio::test]
    async fn execute_encrypts_plaintext_and_values_read_back() {
        let pool = pool().await;
        let already = seed(&pool).await;

        encrypt_secret_columns(&pool, &key(), true).await.unwrap();

        let stored = value(&pool, "SELECT value FROM env_vars WHERE id = 'a'")
            .await
            .unwrap();
        assert!(crypto::is_encrypted(&stored));
        assert_ne!(stored, "plain-1");
        assert_eq!(
            crypto::decrypt_if_encrypted(&stored, Some(&key())).unwrap(),
            "plain-1"
        );

        let key_text = value(&pool, "SELECT ssh_private_key FROM servers")
            .await
            .unwrap();
        assert_eq!(
            crypto::decrypt_if_encrypted(&key_text, Some(&key())).unwrap(),
            "PRIVATE-KEY-TEXT"
        );
        let totp = value(&pool, "SELECT totp_secret FROM users").await.unwrap();
        assert_eq!(
            crypto::decrypt_if_encrypted(&totp, Some(&key())).unwrap(),
            "TOTPSECRET"
        );

        // Untouched: already encrypted, empty string and NULL.
        assert_eq!(
            value(&pool, "SELECT value FROM env_vars WHERE id = 'b'")
                .await
                .unwrap(),
            already
        );
        assert_eq!(
            value(&pool, "SELECT value FROM env_vars WHERE id = 'c'")
                .await
                .unwrap(),
            ""
        );
        assert_eq!(value(&pool, "SELECT ssh_password FROM servers").await, None);
    }

    #[tokio::test]
    async fn running_twice_changes_nothing_the_second_time() {
        let pool = pool().await;
        seed(&pool).await;
        encrypt_secret_columns(&pool, &key(), true).await.unwrap();
        let first = value(&pool, "SELECT value FROM env_vars WHERE id = 'a'")
            .await
            .unwrap();

        let reports = encrypt_secret_columns(&pool, &key(), true).await.unwrap();

        assert!(reports.iter().all(|r| r.plaintext_rows == 0));
        assert_eq!(
            value(&pool, "SELECT value FROM env_vars WHERE id = 'a'")
                .await
                .unwrap(),
            first,
            "an encrypted value must not be encrypted again"
        );
    }
}
