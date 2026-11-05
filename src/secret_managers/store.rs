//! Database access for secret manager connections, path imports and the per-owner
//! failure policy / last resolved values (migration 135).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::providers::{
    build_provider, ConnectionConfig, ConnectionCredentials, ProviderError, SecretProvider,
};
use crate::crypto;
use crate::DbPool;

/// Owner kinds that can hold imports and a failure policy.
pub const OWNER_TYPES: &[&str] = &["app", "service", "project"];

/// Failure policies.
pub const POLICY_FAIL: &str = "fail";
pub const POLICY_LAST_KNOWN: &str = "last_known";

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SecretConnectionRow {
    pub id: String,
    pub team_id: Option<String>,
    pub name: String,
    pub provider: String,
    pub config: String,
    pub credentials: String,
    pub last_tested_at: Option<String>,
    pub last_test_ok: Option<i64>,
    pub last_test_message: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl SecretConnectionRow {
    pub fn config(&self) -> ConnectionConfig {
        serde_json::from_str(&self.config).unwrap_or_default()
    }

    pub fn credentials(&self, key: Option<&[u8; 32]>) -> Result<ConnectionCredentials> {
        if self.credentials.is_empty() {
            return Ok(ConnectionCredentials::default());
        }
        let plain = crypto::decrypt_if_encrypted(&self.credentials, key)
            .context("could not decrypt the connection credentials")?;
        serde_json::from_str(&plain).context("stored connection credentials are not valid JSON")
    }

    pub fn provider(
        &self,
        key: Option<&[u8; 32]>,
        timeout: Duration,
    ) -> Result<Arc<dyn SecretProvider>, ProviderError> {
        let creds = self
            .credentials(key)
            .map_err(|e| ProviderError::Config(format!("{:#}", e)))?;
        build_provider(&self.provider, &self.config(), &creds, timeout)
    }
}

/// Encrypt credentials for storage. Requires the instance encryption key so tokens are
/// never stored in plaintext.
pub fn seal_credentials(creds: &ConnectionCredentials, key: Option<&[u8; 32]>) -> Result<String> {
    let key = key.context(
        "an encryption key is required to store secret manager credentials \
         (set auth.encryption_key or let Rivetr generate <data_dir>/.encryption_key)",
    )?;
    let json = serde_json::to_string(creds)?;
    crypto::encrypt(&json, key)
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretImportRow {
    pub id: String,
    pub owner_type: String,
    pub owner_id: String,
    pub connection_id: String,
    pub path: String,
    pub prefix: String,
    pub created_at: String,
}

/// The connection named `name` that a resource in `team_id` may use: the team's own
/// connection, or an instance-wide one (no team). Team connections win on a name clash.
pub async fn find_connection_by_name(
    db: &DbPool,
    team_id: Option<&str>,
    name: &str,
) -> Result<Option<SecretConnectionRow>> {
    Ok(sqlx::query_as(
        "SELECT * FROM secret_connections \
         WHERE name = ?1 AND (team_id IS ?2 OR team_id IS NULL) \
         ORDER BY (team_id IS NULL) ASC LIMIT 1",
    )
    .bind(name)
    .bind(team_id)
    .fetch_optional(db)
    .await?)
}

pub async fn get_connection(db: &DbPool, id: &str) -> Result<Option<SecretConnectionRow>> {
    Ok(
        sqlx::query_as("SELECT * FROM secret_connections WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?,
    )
}

/// May a resource in `resource_team` use a connection owned by `connection_team`?
pub fn connection_usable_by(connection_team: Option<&str>, resource_team: Option<&str>) -> bool {
    match connection_team {
        None => true,
        Some(t) => resource_team == Some(t),
    }
}

pub async fn imports_for(
    db: &DbPool,
    owner_type: &str,
    owner_id: &str,
) -> Result<Vec<SecretImportRow>> {
    Ok(sqlx::query_as(
        "SELECT * FROM secret_imports WHERE owner_type = ? AND owner_id = ? ORDER BY created_at, id",
    )
    .bind(owner_type)
    .bind(owner_id)
    .fetch_all(db)
    .await?)
}

pub async fn get_policy(db: &DbPool, owner_type: &str, owner_id: &str) -> String {
    sqlx::query_scalar::<_, String>(
        "SELECT failure_policy FROM secret_resolution_state WHERE owner_type = ? AND owner_id = ?",
    )
    .bind(owner_type)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| POLICY_FAIL.to_string())
}

pub async fn set_policy(db: &DbPool, owner_type: &str, owner_id: &str, policy: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO secret_resolution_state (owner_type, owner_id, failure_policy) VALUES (?, ?, ?) \
         ON CONFLICT(owner_type, owner_id) DO UPDATE SET failure_policy = excluded.failure_policy",
    )
    .bind(owner_type)
    .bind(owner_id)
    .bind(policy)
    .execute(db)
    .await?;
    Ok(())
}

/// Values a successful resolution produced, kept (encrypted) for the `last_known` policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LastValues {
    /// Env key -> value for variables whose value was a reference.
    pub refs: BTreeMap<String, String>,
    /// Env key -> value for keys added by path imports.
    pub imports: BTreeMap<String, String>,
}

/// When the last values were stored, if any.
pub async fn last_resolved_at(db: &DbPool, owner_type: &str, owner_id: &str) -> Option<String> {
    sqlx::query_scalar::<_, Option<String>>(
        "SELECT last_resolved_at FROM secret_resolution_state WHERE owner_type = ? AND owner_id = ?",
    )
    .bind(owner_type)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .flatten()
}

pub async fn load_last_values(
    db: &DbPool,
    key: Option<&[u8; 32]>,
    owner_type: &str,
    owner_id: &str,
) -> Option<LastValues> {
    let stored: Option<String> = sqlx::query_scalar(
        "SELECT last_values FROM secret_resolution_state WHERE owner_type = ? AND owner_id = ?",
    )
    .bind(owner_type)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .flatten();
    let stored = stored?;
    // Only encrypted payloads are ever written; refuse anything else.
    if !crypto::is_encrypted(&stored) {
        return None;
    }
    let plain = crypto::decrypt_if_encrypted(&stored, key).ok()?;
    serde_json::from_str(&plain).ok()
}

/// Store the last resolved values encrypted. Without an encryption key nothing is stored
/// (values are never written in plaintext); returns whether they were stored.
pub async fn save_last_values(
    db: &DbPool,
    key: Option<&[u8; 32]>,
    owner_type: &str,
    owner_id: &str,
    values: &LastValues,
) -> Result<bool> {
    let Some(key) = key else {
        return Ok(false);
    };
    let sealed = crypto::encrypt(&serde_json::to_string(values)?, key)?;
    sqlx::query(
        "INSERT INTO secret_resolution_state (owner_type, owner_id, last_values, last_resolved_at) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(owner_type, owner_id) DO UPDATE SET \
         last_values = excluded.last_values, last_resolved_at = excluded.last_resolved_at",
    )
    .bind(owner_type)
    .bind(owner_id)
    .bind(&sealed)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(db)
    .await?;
    Ok(true)
}

#[cfg(test)]
pub(crate) mod test_db {
    use crate::DbPool;

    /// In-memory database with the migration 135 tables (plus a stub `teams` table).
    pub async fn pool() -> DbPool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE teams (id TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        let sql = include_str!("../../migrations/135_secret_managers.sql");
        let stripped: String = sql
            .lines()
            .filter(|l| !l.trim().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            sqlx::query(sqlx::AssertSqlSafe(stmt))
                .execute(&pool)
                .await
                .unwrap();
        }
        pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> [u8; 32] {
        crypto::derive_key("test-key")
    }

    async fn insert_conn(db: &DbPool, id: &str, team: Option<&str>, name: &str) {
        if let Some(t) = team {
            sqlx::query("INSERT OR IGNORE INTO teams (id) VALUES (?)")
                .bind(t)
                .execute(db)
                .await
                .unwrap();
        }
        sqlx::query(
            "INSERT INTO secret_connections (id, team_id, name, provider) VALUES (?, ?, ?, 'doppler')",
        )
        .bind(id)
        .bind(team)
        .bind(name)
        .execute(db)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn migration_is_idempotent() {
        let db = test_db::pool().await;
        let sql = include_str!("../../migrations/135_secret_managers.sql");
        let stripped: String = sql
            .lines()
            .filter(|l| !l.trim().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            sqlx::query(sqlx::AssertSqlSafe(stmt))
                .execute(&db)
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn names_are_unique_per_team_and_team_wins() {
        let db = test_db::pool().await;
        insert_conn(&db, "g", None, "vault").await;
        insert_conn(&db, "a", Some("team-a"), "vault").await;
        insert_conn(&db, "b", Some("team-b"), "vault").await;
        let dup = sqlx::query(
            "INSERT INTO secret_connections (id, team_id, name, provider) VALUES ('x', NULL, 'vault', 'doppler')",
        )
        .execute(&db)
        .await;
        assert!(dup.is_err());

        let f = |t: Option<&'static str>| {
            let db = db.clone();
            async move {
                find_connection_by_name(&db, t, "vault")
                    .await
                    .unwrap()
                    .unwrap()
                    .id
            }
        };
        assert_eq!(f(Some("team-a")).await, "a");
        assert_eq!(f(Some("team-b")).await, "b");
        assert_eq!(f(Some("team-c")).await, "g");
        assert_eq!(f(None).await, "g");
        assert!(find_connection_by_name(&db, None, "other")
            .await
            .unwrap()
            .is_none());
    }

    #[test]
    fn usable_by_rules() {
        assert!(connection_usable_by(None, Some("t")));
        assert!(connection_usable_by(None, None));
        assert!(connection_usable_by(Some("t"), Some("t")));
        assert!(!connection_usable_by(Some("t"), Some("u")));
        assert!(!connection_usable_by(Some("t"), None));
    }

    #[tokio::test]
    async fn policy_and_last_values_round_trip_encrypted() {
        let db = test_db::pool().await;
        assert_eq!(get_policy(&db, "app", "a1").await, POLICY_FAIL);
        set_policy(&db, "app", "a1", POLICY_LAST_KNOWN)
            .await
            .unwrap();
        assert_eq!(get_policy(&db, "app", "a1").await, POLICY_LAST_KNOWN);

        let k = key();
        let mut v = LastValues::default();
        v.refs.insert("DB".into(), "postgres://secret".into());
        assert!(save_last_values(&db, Some(&k), "app", "a1", &v)
            .await
            .unwrap());
        let raw: String = sqlx::query_scalar(
            "SELECT last_values FROM secret_resolution_state WHERE owner_id = 'a1'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert!(!raw.contains("postgres://secret"));
        assert_eq!(
            load_last_values(&db, Some(&k), "app", "a1").await,
            Some(v.clone())
        );
        // Saving values keeps the policy.
        assert_eq!(get_policy(&db, "app", "a1").await, POLICY_LAST_KNOWN);
        // No key: nothing stored.
        assert!(!save_last_values(&db, None, "app", "a2", &v).await.unwrap());
        assert!(load_last_values(&db, None, "app", "a2").await.is_none());
    }

    #[test]
    fn credentials_require_a_key_and_are_encrypted() {
        let creds = ConnectionCredentials {
            token: Some("dp.st.secret".into()),
            ..Default::default()
        };
        assert!(seal_credentials(&creds, None).is_err());
        let k = key();
        let sealed = seal_credentials(&creds, Some(&k)).unwrap();
        assert!(!sealed.contains("dp.st.secret"));
        let row = SecretConnectionRow {
            id: "i".into(),
            team_id: None,
            name: "n".into(),
            provider: "doppler".into(),
            config: "{}".into(),
            credentials: sealed,
            last_tested_at: None,
            last_test_ok: None,
            last_test_message: None,
            created_by: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        assert_eq!(row.credentials(Some(&k)).unwrap(), creds);
    }
}
