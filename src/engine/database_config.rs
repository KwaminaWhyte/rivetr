//! Database type configurations for managed database deployments

use crate::db::{DatabaseCredentials, DatabaseType};
use rand::Rng;

/// Configuration for a database type
#[derive(Debug, Clone)]
pub struct DatabaseTypeConfig {
    /// Docker image name
    pub image: &'static str,
    /// Default version/tag
    pub default_version: &'static str,
    /// Available versions
    pub versions: &'static [&'static str],
    /// Default internal port
    pub port: u16,
    /// Environment variables for credentials
    pub env_vars: DatabaseEnvVars,
    /// Data directory path inside container
    pub data_path: &'static str,
}

/// Environment variable mappings for database credentials
#[derive(Debug, Clone)]
pub struct DatabaseEnvVars {
    /// Username env var (None for Redis)
    pub username: Option<&'static str>,
    /// Password env var
    pub password: &'static str,
    /// Database name env var (None for Redis)
    pub database: Option<&'static str>,
    /// Root password env var (MySQL only)
    pub root_password: Option<&'static str>,
}

/// Get configuration for PostgreSQL
pub fn postgres_config() -> DatabaseTypeConfig {
    DatabaseTypeConfig {
        image: "postgres",
        default_version: "16",
        versions: &["16", "15", "14", "13", "12"],
        port: 5432,
        env_vars: DatabaseEnvVars {
            username: Some("POSTGRES_USER"),
            password: "POSTGRES_PASSWORD",
            database: Some("POSTGRES_DB"),
            root_password: None,
        },
        data_path: "/var/lib/postgresql/data",
    }
}

/// Get configuration for MySQL
pub fn mysql_config() -> DatabaseTypeConfig {
    DatabaseTypeConfig {
        image: "mysql",
        default_version: "8.0",
        versions: &["8.0", "8.4", "5.7"],
        port: 3306,
        env_vars: DatabaseEnvVars {
            username: Some("MYSQL_USER"),
            password: "MYSQL_PASSWORD",
            database: Some("MYSQL_DATABASE"),
            root_password: Some("MYSQL_ROOT_PASSWORD"),
        },
        data_path: "/var/lib/mysql",
    }
}

/// Get configuration for MongoDB
pub fn mongodb_config() -> DatabaseTypeConfig {
    DatabaseTypeConfig {
        image: "mongo",
        default_version: "7",
        versions: &["7", "6", "5", "4.4"],
        port: 27017,
        env_vars: DatabaseEnvVars {
            username: Some("MONGO_INITDB_ROOT_USERNAME"),
            password: "MONGO_INITDB_ROOT_PASSWORD",
            database: Some("MONGO_INITDB_DATABASE"),
            root_password: None,
        },
        data_path: "/data/db",
    }
}

/// Get configuration for Redis
pub fn redis_config() -> DatabaseTypeConfig {
    DatabaseTypeConfig {
        image: "redis",
        default_version: "7",
        versions: &["7", "7.2", "6", "6.2"],
        port: 6379,
        env_vars: DatabaseEnvVars {
            username: None,
            password: "REDIS_PASSWORD", // Note: Redis uses --requirepass flag
            database: None,
            root_password: None,
        },
        data_path: "/data",
    }
}

/// Get configuration for a database type
pub fn get_config(db_type: &DatabaseType) -> DatabaseTypeConfig {
    match db_type {
        DatabaseType::Postgres => postgres_config(),
        DatabaseType::Mysql => mysql_config(),
        DatabaseType::Mongodb => mongodb_config(),
        DatabaseType::Redis => redis_config(),
    }
}

/// Generate environment variables for container deployment
pub fn generate_env_vars(
    db_type: &DatabaseType,
    credentials: &DatabaseCredentials,
) -> Vec<(String, String)> {
    let config = get_config(db_type);
    let mut env = Vec::new();

    // Add username env var if applicable
    if let Some(user_var) = config.env_vars.username {
        env.push((user_var.to_string(), credentials.username.clone()));
    }

    // Add password env var
    env.push((
        config.env_vars.password.to_string(),
        credentials.password.clone(),
    ));

    // Add database name env var if applicable
    if let Some(db_var) = config.env_vars.database {
        if let Some(ref db_name) = credentials.database {
            env.push((db_var.to_string(), db_name.clone()));
        }
    }

    // Add root password env var if applicable (MySQL)
    if let Some(root_var) = config.env_vars.root_password {
        if let Some(ref root_pass) = credentials.root_password {
            env.push((root_var.to_string(), root_pass.clone()));
        }
    }

    env
}

/// Generate random secure password
pub fn generate_password(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Generate random username
pub fn generate_username() -> String {
    let mut rng = rand::rng();
    format!("user_{}", rng.random_range(10000..99999))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_password() {
        let password = generate_password(24);
        assert_eq!(password.len(), 24);
        assert!(password.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_generate_username() {
        let username = generate_username();
        assert!(username.starts_with("user_"));
        assert!(username.len() > 5);
    }

    #[test]
    fn test_postgres_config() {
        let config = postgres_config();
        assert_eq!(config.image, "postgres");
        assert_eq!(config.port, 5432);
        assert_eq!(config.default_version, "16");
    }

    #[test]
    fn test_mysql_config() {
        let config = mysql_config();
        assert_eq!(config.image, "mysql");
        assert_eq!(config.port, 3306);
        assert!(config.env_vars.root_password.is_some());
    }

    #[test]
    fn test_mongodb_config() {
        let config = mongodb_config();
        assert_eq!(config.image, "mongo");
        assert_eq!(config.port, 27017);
    }

    #[test]
    fn test_redis_config() {
        let config = redis_config();
        assert_eq!(config.image, "redis");
        assert_eq!(config.port, 6379);
        assert!(config.env_vars.username.is_none());
    }

    #[test]
    fn test_generate_env_vars_postgres() {
        let creds = DatabaseCredentials {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            database: Some("testdb".to_string()),
            root_password: None,
        };
        let env = generate_env_vars(&DatabaseType::Postgres, &creds);
        assert!(env.contains(&("POSTGRES_USER".to_string(), "testuser".to_string())));
        assert!(env.contains(&("POSTGRES_PASSWORD".to_string(), "testpass".to_string())));
        assert!(env.contains(&("POSTGRES_DB".to_string(), "testdb".to_string())));
    }
}
