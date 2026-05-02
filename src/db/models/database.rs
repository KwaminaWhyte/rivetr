//! Managed database models.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Supported database types for managed databases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    Postgres,
    Mysql,
    Mariadb,
    Mongodb,
    Redis,
    Dragonfly,
    Keydb,
    ClickHouse,
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "postgres"),
            Self::Mysql => write!(f, "mysql"),
            Self::Mariadb => write!(f, "mariadb"),
            Self::Mongodb => write!(f, "mongodb"),
            Self::Redis => write!(f, "redis"),
            Self::Dragonfly => write!(f, "dragonfly"),
            Self::Keydb => write!(f, "keydb"),
            Self::ClickHouse => write!(f, "clickhouse"),
        }
    }
}

impl std::str::FromStr for DatabaseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" => Ok(Self::Postgres),
            "mysql" => Ok(Self::Mysql),
            "mariadb" => Ok(Self::Mariadb),
            "mongodb" | "mongo" => Ok(Self::Mongodb),
            "redis" => Ok(Self::Redis),
            "dragonfly" | "dragonflydb" => Ok(Self::Dragonfly),
            "keydb" => Ok(Self::Keydb),
            "clickhouse" => Ok(Self::ClickHouse),
            _ => Err(format!("Unknown database type: {}", s)),
        }
    }
}

impl From<String> for DatabaseType {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::Postgres)
    }
}

/// Database deployment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseStatus {
    Pending,
    Pulling,
    Starting,
    Running,
    Stopped,
    Failed,
}

impl std::fmt::Display for DatabaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Pulling => write!(f, "pulling"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for DatabaseStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "pulling" => Ok(Self::Pulling),
            "starting" => Ok(Self::Starting),
            "running" => Ok(Self::Running),
            "stopped" => Ok(Self::Stopped),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

impl From<String> for DatabaseStatus {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::Pending)
    }
}

/// Database credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseCredentials {
    pub username: String,
    pub password: String,
    /// Database name (not applicable for Redis)
    pub database: Option<String>,
    /// Root password for MySQL
    pub root_password: Option<String>,
}

/// Managed database entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ManagedDatabase {
    pub id: String,
    pub name: String,
    pub db_type: String,
    pub version: String,
    pub container_id: Option<String>,
    pub status: String,
    pub internal_port: i32,
    pub external_port: i32,
    pub public_access: i32,
    /// JSON-encoded DatabaseCredentials
    pub credentials: String,
    pub volume_name: Option<String>,
    pub volume_path: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub error_message: Option<String>,
    /// Associated project ID
    pub project_id: Option<String>,
    /// Associated team ID for multi-tenancy
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Stable, globally-unique container hostname (set at creation time).
    /// Format: `rivetr-db-{id[:8]}` for new databases.
    /// Legacy databases fall back to `rivetr-db-{name}` if NULL.
    pub container_slug: Option<String>,
    /// Whether SSL/TLS is enabled for this database
    pub ssl_enabled: i32,
    /// SSL mode (e.g. "require", "verify-ca", "verify-full" for Postgres;
    /// "required", "verify-ca", "verify-identity" for MySQL/MariaDB)
    pub ssl_mode: Option<String>,
    /// Custom Docker image override (e.g. "timescaledb/timescaledb-ha:pg16-latest").
    /// When set, this image is used instead of the default image for the db_type.
    pub custom_image: Option<String>,
    /// JSON array of SQL command strings to execute after the database first starts.
    /// Example: ["CREATE EXTENSION IF NOT EXISTS postgis;", "CREATE SCHEMA app;"]
    pub init_commands: Option<String>,
}

impl ManagedDatabase {
    /// Get the database type as enum
    pub fn get_db_type(&self) -> DatabaseType {
        DatabaseType::from(self.db_type.clone())
    }

    /// Get the status as enum
    pub fn get_status(&self) -> DatabaseStatus {
        DatabaseStatus::from(self.status.clone())
    }

    /// Parse credentials from JSON
    pub fn get_credentials(&self) -> Option<DatabaseCredentials> {
        serde_json::from_str(&self.credentials).ok()
    }

    /// Check if public access is enabled
    pub fn is_public(&self) -> bool {
        self.public_access != 0
    }

    /// Check if SSL is enabled
    pub fn is_ssl_enabled(&self) -> bool {
        self.ssl_enabled != 0
    }

    /// Get container name — also used as the Docker network hostname.
    ///
    /// New databases use a slug based on the ID prefix for global uniqueness.
    /// Existing databases fall back to the legacy name-based format so existing
    /// containers and connection strings continue to work without disruption.
    pub fn container_name(&self) -> String {
        self.container_slug
            .clone()
            .unwrap_or_else(|| format!("rivetr-db-{}", self.name))
    }

    /// Build the slug that new databases should be assigned at creation time.
    pub fn build_slug(id: &str) -> String {
        format!("rivetr-db-{}", &id[..8.min(id.len())])
    }

    /// Generate internal connection string (for apps on same Docker network)
    pub fn internal_connection_string(&self) -> Option<String> {
        let creds = self.get_credentials()?;
        let container_name = self.container_name();

        match self.get_db_type() {
            DatabaseType::Postgres => Some(format!(
                "postgresql://{}:{}@{}:{}/{}",
                creds.username,
                creds.password,
                container_name,
                self.internal_port,
                creds.database.unwrap_or_else(|| creds.username.clone())
            )),
            DatabaseType::Mysql | DatabaseType::Mariadb => {
                // MySQL 8 enables SSL by default with a self-signed cert that clients
                // can't validate, so connections fail with `TLS/SSL error: self-signed
                // certificate in certificate chain`.  Append `?ssl-mode=DISABLED` so the
                // connection string works out of the box for managed DBs on the same
                // private Docker network.  MariaDB ignores the parameter, which is fine.
                Some(format!(
                    "mysql://{}:{}@{}:{}/{}?ssl-mode=DISABLED",
                    creds.username,
                    creds.password,
                    container_name,
                    self.internal_port,
                    creds.database.unwrap_or_else(|| creds.username.clone())
                ))
            }
            DatabaseType::Mongodb => Some(format!(
                "mongodb://{}:{}@{}:{}/{}?authSource=admin",
                creds.username,
                creds.password,
                container_name,
                self.internal_port,
                creds.database.unwrap_or_else(|| "admin".to_string())
            )),
            DatabaseType::Redis | DatabaseType::Dragonfly | DatabaseType::Keydb => {
                if creds.password.is_empty() {
                    Some(format!("redis://{}:{}", container_name, self.internal_port))
                } else {
                    Some(format!(
                        "redis://:{}@{}:{}",
                        creds.password, container_name, self.internal_port
                    ))
                }
            }
            DatabaseType::ClickHouse => Some(format!(
                "clickhouse://{}:{}@{}:{}/{}",
                creds.username,
                creds.password,
                container_name,
                self.internal_port,
                creds.database.unwrap_or_else(|| creds.username.clone())
            )),
        }
    }

    /// Generate external connection string (for public access)
    pub fn external_connection_string(&self, host: &str) -> Option<String> {
        if !self.is_public() || self.external_port == 0 {
            return None;
        }

        let creds = self.get_credentials()?;

        match self.get_db_type() {
            DatabaseType::Postgres => Some(format!(
                "postgresql://{}:{}@{}:{}/{}",
                creds.username,
                creds.password,
                host,
                self.external_port,
                creds.database.unwrap_or_else(|| creds.username.clone())
            )),
            DatabaseType::Mysql | DatabaseType::Mariadb => Some(format!(
                "mysql://{}:{}@{}:{}/{}?ssl-mode=DISABLED",
                creds.username,
                creds.password,
                host,
                self.external_port,
                creds.database.unwrap_or_else(|| creds.username.clone())
            )),
            DatabaseType::Mongodb => Some(format!(
                "mongodb://{}:{}@{}:{}/{}?authSource=admin",
                creds.username,
                creds.password,
                host,
                self.external_port,
                creds.database.unwrap_or_else(|| "admin".to_string())
            )),
            DatabaseType::Redis | DatabaseType::Dragonfly | DatabaseType::Keydb => {
                if creds.password.is_empty() {
                    Some(format!("redis://{}:{}", host, self.external_port))
                } else {
                    Some(format!(
                        "redis://:{}@{}:{}",
                        creds.password, host, self.external_port
                    ))
                }
            }
            DatabaseType::ClickHouse => Some(format!(
                "clickhouse://{}:{}@{}:{}/{}",
                creds.username,
                creds.password,
                host,
                self.external_port,
                creds.database.unwrap_or_else(|| creds.username.clone())
            )),
        }
    }
}

/// Response DTO for ManagedDatabase (masks password in credentials)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedDatabaseResponse {
    pub id: String,
    pub name: String,
    pub db_type: String,
    pub version: String,
    pub container_id: Option<String>,
    pub status: String,
    pub internal_port: i32,
    pub external_port: i32,
    pub public_access: bool,
    /// Credentials with password masked unless reveal=true
    pub credentials: DatabaseCredentials,
    pub volume_name: Option<String>,
    pub volume_path: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub internal_connection_string: Option<String>,
    pub external_connection_string: Option<String>,
    pub error_message: Option<String>,
    pub project_id: Option<String>,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Whether SSL/TLS is enabled
    pub ssl_enabled: bool,
    /// SSL mode (e.g. "require", "verify-ca", "verify-full")
    pub ssl_mode: Option<String>,
    /// Custom Docker image override (e.g. "timescaledb/timescaledb-ha:pg16-latest")
    pub custom_image: Option<String>,
    /// JSON array of SQL init commands to run after first start
    pub init_commands: Option<String>,
}

impl ManagedDatabase {
    pub fn to_response(
        &self,
        reveal_password: bool,
        external_host: Option<&str>,
    ) -> ManagedDatabaseResponse {
        let mut creds = self.get_credentials().unwrap_or(DatabaseCredentials {
            username: String::new(),
            password: String::new(),
            database: None,
            root_password: None,
        });

        if !reveal_password {
            creds.password = "********".to_string();
            if creds.root_password.is_some() {
                creds.root_password = Some("********".to_string());
            }
        }

        ManagedDatabaseResponse {
            id: self.id.clone(),
            name: self.name.clone(),
            db_type: self.db_type.clone(),
            version: self.version.clone(),
            container_id: self.container_id.clone(),
            status: self.status.clone(),
            internal_port: self.internal_port,
            external_port: self.external_port,
            public_access: self.is_public(),
            credentials: creds,
            volume_name: self.volume_name.clone(),
            volume_path: self.volume_path.clone(),
            memory_limit: self.memory_limit.clone(),
            cpu_limit: self.cpu_limit.clone(),
            internal_connection_string: self.internal_connection_string(),
            external_connection_string: external_host
                .and_then(|h| self.external_connection_string(h)),
            error_message: self.error_message.clone(),
            project_id: self.project_id.clone(),
            team_id: self.team_id.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            ssl_enabled: self.is_ssl_enabled(),
            ssl_mode: self.ssl_mode.clone(),
            custom_image: self.custom_image.clone(),
            init_commands: self.init_commands.clone(),
        }
    }
}

/// Request to create a managed database
#[derive(Debug, Deserialize)]
pub struct CreateManagedDatabaseRequest {
    pub name: String,
    pub db_type: DatabaseType,
    /// Version/tag (e.g., "16", "8.0", "7", "7.2")
    #[serde(default = "default_db_version")]
    pub version: String,
    /// Enable public port exposure
    #[serde(default)]
    pub public_access: bool,
    /// Custom username (optional, auto-generated if not provided)
    pub username: Option<String>,
    /// Custom password (optional, auto-generated if not provided)
    pub password: Option<String>,
    /// Custom database name (optional)
    pub database: Option<String>,
    /// Custom root password for MySQL (optional, auto-generated if not provided)
    pub root_password: Option<String>,
    /// Memory limit
    pub memory_limit: Option<String>,
    /// CPU limit
    pub cpu_limit: Option<String>,
    /// Associated project ID
    pub project_id: Option<String>,
    /// Associated team ID for multi-tenancy
    pub team_id: Option<String>,
    /// Custom Docker image override (e.g. "timescaledb/timescaledb-ha:pg16-latest")
    pub custom_image: Option<String>,
    /// JSON array of SQL command strings to execute after the database first starts
    pub init_commands: Option<String>,
}

fn default_db_version() -> String {
    "latest".to_string()
}

/// Request to update a managed database
#[derive(Debug, Deserialize)]
pub struct UpdateManagedDatabaseRequest {
    /// Enable/disable public access
    pub public_access: Option<bool>,
    /// Custom external port (0 = auto-assign, 1024-65535 = specific port)
    /// Only used when public_access is true
    pub external_port: Option<i32>,
    /// Memory limit
    pub memory_limit: Option<String>,
    /// CPU limit
    pub cpu_limit: Option<String>,
    /// Enable/disable SSL/TLS
    pub ssl_enabled: Option<bool>,
    /// SSL mode string
    pub ssl_mode: Option<String>,
    /// Custom Docker image override (e.g. "timescaledb/timescaledb-ha:pg16-latest")
    pub custom_image: Option<String>,
    /// JSON array of SQL init command strings to run after first start
    pub init_commands: Option<String>,
}
