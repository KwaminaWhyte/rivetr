//! Multi-server management API endpoints.
//!
//! Provides CRUD operations for remote servers, SSH-based health checks,
//! app–server assignment management, and a WebSocket SSH terminal.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::crypto;
use crate::db::{App, AppResponse, CreateServerRequest, Server, UpdateServerRequest};
use crate::AppState;

/// Key length for AES-256 encryption
const KEY_LENGTH: usize = 32;

/// Get the derived encryption key from the config if configured
fn get_encryption_key(state: &AppState) -> Option<[u8; KEY_LENGTH]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crypto::derive_key(secret))
}

#[derive(Debug, Deserialize)]
pub struct ListServersQuery {
    pub team_id: Option<String>,
}

/// List all servers, optionally filtered by team_id
pub async fn list_servers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListServersQuery>,
) -> Result<Json<Vec<Server>>, StatusCode> {
    let servers = if let Some(team_id) = &query.team_id {
        sqlx::query_as::<_, Server>(
            "SELECT * FROM servers WHERE team_id = ? ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, Server>("SELECT * FROM servers ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
    }
    .map_err(|e| {
        tracing::error!("Failed to list servers: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Mask secrets in responses (never return raw credentials)
    let masked: Vec<Server> = servers
        .into_iter()
        .map(|mut s| {
            s.ssh_private_key = s.ssh_private_key.map(|_| "[encrypted]".to_string());
            s.ssh_password = s.ssh_password.map(|_| "[encrypted]".to_string());
            s
        })
        .collect();

    Ok(Json(masked))
}

/// Create a new server record
pub async fn create_server(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateServerRequest>,
) -> Result<(StatusCode, Json<Server>), StatusCode> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let port = req.port.unwrap_or(22);
    let username = req.username.unwrap_or_else(|| "root".to_string());

    let enc_key = get_encryption_key(&state);

    // Encrypt SSH private key if provided
    let encrypted_key = if let Some(ref key) = req.ssh_private_key {
        let encrypted = crypto::encrypt_if_key_available(key, enc_key.as_ref()).map_err(|e| {
            tracing::error!("Failed to encrypt SSH private key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Some(encrypted)
    } else {
        None
    };

    // Encrypt SSH password if provided
    let encrypted_password = if let Some(ref pwd) = req.ssh_password {
        let encrypted = crypto::encrypt_if_key_available(pwd, enc_key.as_ref()).map_err(|e| {
            tracing::error!("Failed to encrypt SSH password: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Some(encrypted)
    } else {
        None
    };

    let hourly_rate = req.hourly_rate.unwrap_or(0.036);

    sqlx::query(
        r#"
        INSERT INTO servers (id, name, host, port, username, ssh_private_key, ssh_password, hourly_rate, status, team_id, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'unknown', ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.host)
    .bind(port)
    .bind(&username)
    .bind(&encrypted_key)
    .bind(&encrypted_password)
    .bind(hourly_rate)
    .bind(&req.team_id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Mask secrets in response
    server.ssh_private_key = server.ssh_private_key.map(|_| "[encrypted]".to_string());
    server.ssh_password = server.ssh_password.map(|_| "[encrypted]".to_string());

    Ok((StatusCode::CREATED, Json(server)))
}

/// Get a single server by ID
pub async fn get_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Server>, StatusCode> {
    let mut server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get server: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Mask secrets in response
    server.ssh_private_key = server.ssh_private_key.map(|_| "[encrypted]".to_string());
    server.ssh_password = server.ssh_password.map(|_| "[encrypted]".to_string());

    Ok(Json(server))
}

/// Update a server record
pub async fn update_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateServerRequest>,
) -> Result<Json<Server>, StatusCode> {
    // Verify server exists
    let existing = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let now = chrono::Utc::now().to_rfc3339();

    let enc_key = get_encryption_key(&state);

    // Encrypt SSH private key if a new one is provided
    let encrypted_key = if let Some(ref key) = req.ssh_private_key {
        let encrypted = crypto::encrypt_if_key_available(key, enc_key.as_ref()).map_err(|e| {
            tracing::error!("Failed to encrypt SSH private key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Some(encrypted)
    } else {
        None
    };

    // Encrypt SSH password if a new one is provided
    let encrypted_password = if let Some(ref pwd) = req.ssh_password {
        let encrypted = crypto::encrypt_if_key_available(pwd, enc_key.as_ref()).map_err(|e| {
            tracing::error!("Failed to encrypt SSH password: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Some(encrypted)
    } else {
        None
    };

    let timezone = req
        .timezone
        .clone()
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| existing.timezone.clone());

    sqlx::query(
        r#"
        UPDATE servers SET
            name = COALESCE(?, name),
            host = COALESCE(?, host),
            port = COALESCE(?, port),
            username = COALESCE(?, username),
            ssh_private_key = COALESCE(?, ssh_private_key),
            ssh_password = COALESCE(?, ssh_password),
            hourly_rate = COALESCE(?, hourly_rate),
            timezone = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&req.name)
    .bind(&req.host)
    .bind(req.port)
    .bind(&req.username)
    .bind(&encrypted_key)
    .bind(&encrypted_password)
    .bind(req.hourly_rate)
    .bind(&timezone)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update server: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Mask secrets in response
    server.ssh_private_key = server.ssh_private_key.map(|_| "[encrypted]".to_string());
    server.ssh_password = server.ssh_password.map(|_| "[encrypted]".to_string());

    Ok(Json(server))
}

/// Delete a server
pub async fn delete_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM servers WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete server: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Trigger a health check on a remote server via SSH.
///
/// Runs a set of commands over SSH to gather CPU, memory, disk, OS, and
/// Docker version information, then updates the server record in the database.
pub async fn check_server_health(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ServerHealthResponse>, StatusCode> {
    // Fetch the server record (with the encrypted key)
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch server for health check: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let enc_key = get_encryption_key(&state);

    // Decrypt the SSH private key if available
    let private_key_content = if let Some(ref encrypted_key) = server.ssh_private_key {
        match crypto::decrypt_if_encrypted(encrypted_key, enc_key.as_ref()) {
            Ok(key) => Some(key),
            Err(e) => {
                tracing::warn!("Failed to decrypt SSH key for server {}: {}", id, e);
                None
            }
        }
    } else {
        None
    };

    // Decrypt the SSH password if available
    let password_content = if let Some(ref encrypted_pwd) = server.ssh_password {
        match crypto::decrypt_if_encrypted(encrypted_pwd, enc_key.as_ref()) {
            Ok(pwd) => Some(pwd),
            Err(e) => {
                tracing::warn!("Failed to decrypt SSH password for server {}: {}", id, e);
                None
            }
        }
    } else {
        None
    };

    // Run the health check
    let health_result = run_ssh_health_check(
        &server.host,
        server.port as u16,
        &server.username,
        private_key_content.as_deref(),
        password_content.as_deref(),
    )
    .await;

    let now = chrono::Utc::now().to_rfc3339();

    // Capture docker status fields before moving stats into the DB update
    let (docker_installed, docker_running, compose_installed, compose_version) =
        match &health_result {
            Ok(stats) => {
                let installed = stats
                    .docker_version
                    .as_deref()
                    .map(|v| v != "not installed" && !v.is_empty())
                    .unwrap_or(false);
                let running = stats.docker_running;
                let c_installed = stats
                    .compose_version
                    .as_deref()
                    .map(|v| v != "not installed" && !v.is_empty())
                    .unwrap_or(false);
                let c_version = stats.compose_version.clone();
                (installed, running, c_installed, c_version)
            }
            Err(_) => (false, false, false, None),
        };

    match health_result {
        Ok(stats) => {
            // Update server with gathered stats
            sqlx::query(
                r#"
                UPDATE servers SET
                    status = 'online',
                    last_seen_at = ?,
                    cpu_usage = ?,
                    memory_usage = ?,
                    disk_usage = ?,
                    memory_total = ?,
                    disk_total = ?,
                    os_info = ?,
                    docker_version = ?,
                    updated_at = ?
                WHERE id = ?
                "#,
            )
            .bind(&now)
            .bind(stats.cpu_usage)
            .bind(stats.memory_usage)
            .bind(stats.disk_usage)
            .bind(stats.memory_total)
            .bind(stats.disk_total)
            .bind(&stats.os_info)
            .bind(&stats.docker_version)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update server health stats: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }
        Err(e) => {
            tracing::warn!("Health check failed for server {}: {}", id, e);
            // Mark as offline but still update last_seen_at (check was attempted)
            sqlx::query(
                "UPDATE servers SET status = 'offline', last_seen_at = ?, updated_at = ? WHERE id = ?"
            )
            .bind(&now)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update server status to offline: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }
    }

    let mut updated = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Mask secrets in response
    updated.ssh_private_key = updated.ssh_private_key.map(|_| "[encrypted]".to_string());
    updated.ssh_password = updated.ssh_password.map(|_| "[encrypted]".to_string());

    Ok(Json(ServerHealthResponse {
        server: updated,
        docker_installed,
        docker_running,
        compose_installed,
        compose_version,
    }))
}

/// Gathered stats from a remote server SSH health check
struct ServerHealthStats {
    cpu_usage: Option<f64>,
    memory_usage: Option<f64>,
    disk_usage: Option<f64>,
    memory_total: Option<i64>,
    disk_total: Option<i64>,
    os_info: Option<String>,
    docker_version: Option<String>,
    /// Whether the Docker daemon is currently active (systemctl is-active docker)
    docker_running: bool,
    /// Docker Compose version string ("not installed" when absent)
    compose_version: Option<String>,
}

/// Extended health-check response: the updated Server row plus Docker status fields
/// that are not stored in the database.
#[derive(Debug, Serialize)]
pub struct ServerHealthResponse {
    #[serde(flatten)]
    pub server: Server,
    /// True when `docker --version` reports a real version (not "not installed")
    pub docker_installed: bool,
    /// True when the Docker daemon is active on the remote server
    pub docker_running: bool,
    /// True when `docker compose version` (or `docker-compose --version`) succeeds
    pub compose_installed: bool,
    /// Raw version string from `docker compose version`, if available
    pub compose_version: Option<String>,
}

/// Run SSH commands to gather server health statistics.
///
/// Writes the private key to a temporary file (with 0600 permissions on Unix)
/// then runs a combined SSH command to collect all stats in one connection.
async fn run_ssh_health_check(
    host: &str,
    port: u16,
    username: &str,
    private_key: Option<&str>,
    password: Option<&str>,
) -> anyhow::Result<ServerHealthStats> {
    use std::io::Write;
    use tokio::process::Command;

    // Write private key to a temp file if provided
    let key_file = if let Some(key_content) = private_key {
        let mut tmpfile = tempfile::Builder::new()
            .prefix("rivetr-ssh-key-")
            .suffix(".pem")
            .tempfile()?;

        tmpfile.write_all(key_content.as_bytes())?;

        // Set permissions to 600 on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(tmpfile.path(), std::fs::Permissions::from_mode(0o600))?;
        }

        Some(tmpfile)
    } else {
        None
    };

    // Build the combined command string to run remotely
    let remote_cmd = r#"
        echo "CPU:$(top -bn1 2>/dev/null | grep 'Cpu(s)' | awk '{print $2}' | tr -d '%us,' || echo 'N/A')";
        echo "MEM:$(free -b 2>/dev/null | awk 'NR==2{print $2, $3}' || echo 'N/A')";
        echo "DISK:$(df -b / 2>/dev/null | awk 'NR==2{print $2, $3}' || echo 'N/A')";
        echo "OS:$(uname -a 2>/dev/null || echo 'N/A')";
        echo "DOCKER:$(docker version --format '{{.Server.Version}}' 2>/dev/null || echo 'not installed')";
        echo "DOCKER_RUNNING:$(systemctl is-active docker 2>/dev/null || (docker info > /dev/null 2>&1 && echo 'active') || echo 'inactive')";
        echo "COMPOSE:$(docker compose version --short 2>/dev/null || docker-compose --version 2>/dev/null | awk '{print $3}' | tr -d ',' || echo 'not installed')"
    "#;

    // Use sshpass for password authentication if provided and no key is available
    let use_sshpass = password.is_some() && key_file.is_none();

    let mut cmd = if use_sshpass {
        let mut c = Command::new("sshpass");
        c.arg("-p").arg(password.unwrap());
        c.arg("ssh");
        c
    } else {
        Command::new("ssh")
    };

    cmd.arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("ConnectTimeout=5");

    // BatchMode=yes disables password prompts; skip it when using sshpass
    if !use_sshpass {
        cmd.arg("-o").arg("BatchMode=yes");
    }

    cmd.arg("-p").arg(port.to_string());

    if let Some(ref key_file) = key_file {
        cmd.arg("-i").arg(key_file.path());
    }

    cmd.arg(format!("{}@{}", username, host)).arg(remote_cmd);

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // If the key failed to load and a password is available, retry with password only.
        if let (Some(_), Some(pw)) = (&key_file, password) {
            tracing::warn!(
                "SSH key auth failed ({}), retrying with password",
                stderr.trim()
            );
            let mut fallback = Command::new("sshpass");
            fallback
                .arg("-p")
                .arg(pw)
                .arg("ssh")
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-o")
                .arg("ConnectTimeout=5")
                .arg("-p")
                .arg(port.to_string())
                .arg(format!("{}@{}", username, host))
                .arg(remote_cmd);

            let fallback_output = fallback.output().await?;
            if !fallback_output.status.success() {
                let fallback_stderr = String::from_utf8_lossy(&fallback_output.stderr);
                return Err(anyhow::anyhow!("SSH command failed: {}", fallback_stderr));
            }
            let stdout = String::from_utf8_lossy(&fallback_output.stdout);
            return parse_health_output(&stdout);
        }

        return Err(anyhow::anyhow!("SSH command failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_health_output(&stdout)
}

/// Parse the output from the SSH health check commands.
fn parse_health_output(output: &str) -> anyhow::Result<ServerHealthStats> {
    let mut cpu_usage: Option<f64> = None;
    let mut memory_usage: Option<f64> = None;
    let mut disk_usage: Option<f64> = None;
    let mut memory_total: Option<i64> = None;
    let mut disk_total: Option<i64> = None;
    let mut os_info: Option<String> = None;
    let mut docker_version: Option<String> = None;
    let mut docker_running = false;
    let mut compose_version: Option<String> = None;

    for line in output.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("CPU:") {
            let val = val.trim();
            if val != "N/A" {
                if let Ok(v) = val.parse::<f64>() {
                    cpu_usage = Some(v);
                }
            }
        } else if let Some(val) = line.strip_prefix("MEM:") {
            let val = val.trim();
            if val != "N/A" {
                let parts: Vec<&str> = val.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let (Ok(total), Ok(used)) =
                        (parts[0].parse::<i64>(), parts[1].parse::<i64>())
                    {
                        memory_total = Some(total);
                        if total > 0 {
                            memory_usage = Some((used as f64 / total as f64) * 100.0);
                        }
                    }
                }
            }
        } else if let Some(val) = line.strip_prefix("DISK:") {
            let val = val.trim();
            if val != "N/A" {
                let parts: Vec<&str> = val.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let (Ok(total), Ok(used)) =
                        (parts[0].parse::<i64>(), parts[1].parse::<i64>())
                    {
                        disk_total = Some(total);
                        if total > 0 {
                            disk_usage = Some((used as f64 / total as f64) * 100.0);
                        }
                    }
                }
            }
        } else if let Some(val) = line.strip_prefix("OS:") {
            let val = val.trim();
            if val != "N/A" && !val.is_empty() {
                os_info = Some(val.to_string());
            }
        } else if let Some(val) = line.strip_prefix("DOCKER:") {
            // This prefix must be checked before DOCKER_RUNNING to avoid mis-matching
            // Only match the plain "DOCKER:" line (not "DOCKER_RUNNING:")
            let val = val.trim();
            if !val.is_empty() {
                docker_version = Some(val.to_string());
            }
        } else if let Some(val) = line.strip_prefix("DOCKER_RUNNING:") {
            let val = val.trim();
            docker_running = val == "active";
        } else if let Some(val) = line.strip_prefix("COMPOSE:") {
            let val = val.trim();
            if !val.is_empty() {
                compose_version = Some(val.to_string());
            }
        }
    }

    Ok(ServerHealthStats {
        cpu_usage,
        memory_usage,
        disk_usage,
        memory_total,
        disk_total,
        os_info,
        docker_version,
        docker_running,
        compose_version,
    })
}

// ---------------------------------------------------------------------------
// Docker Installation Endpoint
// ---------------------------------------------------------------------------

/// POST /api/servers/:id/install-docker
///
/// SSHes into the server and installs Docker using the official get.docker.com
/// convenience script, then enables and starts the daemon.
pub async fn install_docker(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Fetch the server record
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch server for Docker install: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch server"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Server not found"})),
            )
        })?;

    let enc_key = get_encryption_key(&state);

    let private_key_content = if let Some(ref encrypted_key) = server.ssh_private_key {
        match crypto::decrypt_if_encrypted(encrypted_key, enc_key.as_ref()) {
            Ok(key) => Some(key),
            Err(e) => {
                tracing::warn!("Failed to decrypt SSH key for server {}: {}", id, e);
                None
            }
        }
    } else {
        None
    };

    let password_content = if let Some(ref encrypted_pwd) = server.ssh_password {
        match crypto::decrypt_if_encrypted(encrypted_pwd, enc_key.as_ref()) {
            Ok(pwd) => Some(pwd),
            Err(e) => {
                tracing::warn!("Failed to decrypt SSH password for server {}: {}", id, e);
                None
            }
        }
    } else {
        None
    };

    let result = run_ssh_install_docker(
        &server.host,
        server.port as u16,
        &server.username,
        private_key_content.as_deref(),
        password_content.as_deref(),
    )
    .await;

    match result {
        Ok(output) => {
            tracing::info!("Docker installed on server {}", id);
            Ok(Json(serde_json::json!({
                "success": true,
                "output": output,
            })))
        }
        Err(e) => {
            tracing::warn!("Docker install failed on server {}: {}", id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Docker installation failed: {}", e),
                })),
            ))
        }
    }
}

/// Run the Docker installation script on a remote server via SSH.
async fn run_ssh_install_docker(
    host: &str,
    port: u16,
    username: &str,
    private_key: Option<&str>,
    password: Option<&str>,
) -> anyhow::Result<String> {
    use std::io::Write;
    use tokio::process::Command;

    let key_file = if let Some(key_content) = private_key {
        let mut tmpfile = tempfile::Builder::new()
            .prefix("rivetr-ssh-key-")
            .suffix(".pem")
            .tempfile()?;

        tmpfile.write_all(key_content.as_bytes())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(tmpfile.path(), std::fs::Permissions::from_mode(0o600))?;
        }

        Some(tmpfile)
    } else {
        None
    };

    let remote_cmd = r#"
        set -e
        curl -fsSL https://get.docker.com -o /tmp/get-docker.sh
        sh /tmp/get-docker.sh
        systemctl enable docker
        systemctl start docker
        echo "Docker installed successfully: $(docker --version)"
    "#;

    let use_sshpass = password.is_some() && key_file.is_none();

    let mut cmd = if use_sshpass {
        let mut c = Command::new("sshpass");
        c.arg("-p").arg(password.unwrap());
        c.arg("ssh");
        c
    } else {
        Command::new("ssh")
    };

    cmd.arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("ConnectTimeout=30");

    if !use_sshpass {
        cmd.arg("-o").arg("BatchMode=yes");
    }

    cmd.arg("-p").arg(port.to_string());

    if let Some(ref key_file) = key_file {
        cmd.arg("-i").arg(key_file.path());
    }

    cmd.arg(format!("{}@{}", username, host)).arg(remote_cmd);

    let output = cmd.output().await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        // If the key failed and a password is available, retry with password only.
        if let (Some(_), Some(pw)) = (&key_file, password) {
            tracing::warn!("SSH key auth failed, retrying with password");
            let mut fallback = Command::new("sshpass");
            fallback
                .arg("-p")
                .arg(pw)
                .arg("ssh")
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-o")
                .arg("ConnectTimeout=30")
                .arg("-p")
                .arg(port.to_string())
                .arg(format!("{}@{}", username, host))
                .arg(remote_cmd);
            let fallback_output = fallback.output().await?;
            let fb_stdout = String::from_utf8_lossy(&fallback_output.stdout).to_string();
            let fb_stderr = String::from_utf8_lossy(&fallback_output.stderr).to_string();
            if !fallback_output.status.success() {
                return Err(anyhow::anyhow!(
                    "Installation failed. stdout: {} stderr: {}",
                    fb_stdout,
                    fb_stderr
                ));
            }
            return Ok(fb_stdout);
        }
        return Err(anyhow::anyhow!(
            "Installation failed. stdout: {} stderr: {}",
            stdout,
            stderr
        ));
    }

    Ok(format!("{}\n{}", stdout, stderr).trim().to_string())
}

// ---------------------------------------------------------------------------
// App–Server Assignment Endpoints
// ---------------------------------------------------------------------------

/// List all apps assigned to this server (`apps.server_id = :id`).
pub async fn list_server_apps(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<AppResponse>>, StatusCode> {
    // Verify server exists
    let _server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch server: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let apps =
        sqlx::query_as::<_, App>("SELECT * FROM apps WHERE server_id = ? ORDER BY created_at DESC")
            .bind(&id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list apps for server {}: {}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    Ok(Json(apps.into_iter().map(AppResponse::from).collect()))
}

/// Assign an app to a server by setting `apps.server_id`.
pub async fn assign_app_to_server(
    State(state): State<Arc<AppState>>,
    Path((id, app_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    // Verify server exists
    let _server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query("UPDATE apps SET server_id = ?, updated_at = ? WHERE id = ?")
        .bind(&id)
        .bind(&now)
        .bind(&app_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to assign app {} to server {}: {}", app_id, id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Unassign an app from a server by clearing `apps.server_id`.
pub async fn unassign_app_from_server(
    State(state): State<Arc<AppState>>,
    Path((_id, app_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query("UPDATE apps SET server_id = NULL, updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&app_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to unassign app {}: {}", app_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// WebSocket SSH Terminal
// ---------------------------------------------------------------------------

/// WebSocket endpoint that proxies an interactive SSH terminal session to a
/// remote server.
///
/// GET /api/servers/:id/terminal?token=<auth_token>
pub async fn server_terminal_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<crate::api::ws::WsAuthQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    if !crate::api::ws::validate_ws_token_pub(&state, &query).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(ws.on_upgrade(move |socket| handle_server_terminal(socket, state, server_id)))
}

async fn handle_server_terminal(mut socket: WebSocket, state: Arc<AppState>, server_id: String) {
    use std::io::Write;
    use tokio::process::Command;

    // Fetch server record
    let server = match sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&server_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            let msg = serde_json::json!({"type": "error", "message": "Server not found"});
            let _ = socket.send(Message::Text(msg.to_string())).await;
            return;
        }
        Err(e) => {
            let msg =
                serde_json::json!({"type": "error", "message": format!("Database error: {}", e)});
            let _ = socket.send(Message::Text(msg.to_string())).await;
            return;
        }
    };

    let enc_key = get_encryption_key(&state);

    // Decrypt SSH private key if present
    let private_key_content = if let Some(ref encrypted_key) = server.ssh_private_key {
        match crate::crypto::decrypt_if_encrypted(encrypted_key, enc_key.as_ref()) {
            Ok(k) => Some(k),
            Err(e) => {
                tracing::warn!(
                    "Failed to decrypt SSH key for server terminal {}: {}",
                    server_id,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    // Decrypt SSH password if present and no key available
    let password_content = if private_key_content.is_none() {
        if let Some(ref encrypted_pwd) = server.ssh_password {
            crate::crypto::decrypt_if_encrypted(encrypted_pwd, enc_key.as_ref()).ok()
        } else {
            None
        }
    } else {
        None
    };

    // Write key to temp file if available
    let key_file: Option<tempfile::NamedTempFile> =
        if let Some(ref key_content) = private_key_content {
            match tempfile::Builder::new()
                .prefix("rivetr-ssh-term-")
                .suffix(".pem")
                .tempfile()
            {
                Ok(mut f) => {
                    if f.write_all(key_content.as_bytes()).is_err() {
                        None
                    } else {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(
                                f.path(),
                                std::fs::Permissions::from_mode(0o600),
                            );
                        }
                        Some(f)
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        };

    // Build SSH args; key_path_str must outlive ssh_args
    let port_str = server.port.to_string();
    let target = format!("{}@{}", server.username, server.host);
    let key_path_str = key_file
        .as_ref()
        .map(|kf| kf.path().to_string_lossy().to_string())
        .unwrap_or_default();

    let use_sshpass = password_content.is_some() && key_path_str.is_empty();

    let mut ssh_args: Vec<&str> = vec![
        "-tt",
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "ConnectTimeout=10",
        "-p",
        &port_str,
    ];
    // BatchMode=yes disables password prompts; skip it when using sshpass
    if !use_sshpass {
        ssh_args.push("-o");
        ssh_args.push("BatchMode=yes");
    }
    if !key_path_str.is_empty() {
        ssh_args.push("-i");
        ssh_args.push(&key_path_str);
    }
    ssh_args.push(&target);

    // Spawn SSH process (with sshpass for password auth)
    let mut child = if use_sshpass {
        let pwd = password_content.as_deref().unwrap_or("");
        let mut cmd = Command::new("sshpass");
        cmd.arg("-p").arg(pwd).arg("ssh").args(&ssh_args);
        match cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let msg = serde_json::json!({"type": "error", "message": format!("Failed to spawn SSH: {}", e)});
                let _ = socket.send(Message::Text(msg.to_string())).await;
                return;
            }
        }
    } else {
        match Command::new("ssh")
            .args(&ssh_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let msg = serde_json::json!({"type": "error", "message": format!("Failed to spawn SSH: {}", e)});
                let _ = socket.send(Message::Text(msg.to_string())).await;
                return;
            }
        }
    };

    let connected_msg = serde_json::json!({
        "type": "connected",
        "server_id": server_id,
        "host": server.host,
    });
    if socket
        .send(Message::Text(connected_msg.to_string()))
        .await
        .is_err()
    {
        return;
    }

    let mut child_stdin = match child.stdin.take() {
        Some(s) => s,
        None => {
            let msg = serde_json::json!({"type": "error", "message": "Failed to open stdin"});
            let _ = socket.send(Message::Text(msg.to_string())).await;
            return;
        }
    };
    let mut child_stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            let msg = serde_json::json!({"type": "error", "message": "Failed to open stdout"});
            let _ = socket.send(Message::Text(msg.to_string())).await;
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Spawn a task to forward SSH stdout → WebSocket
    let stdout_task = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 4096];
        loop {
            match child_stdout.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let msg = serde_json::json!({"type": "data", "data": data});
                    if ws_sender
                        .send(Message::Text(msg.to_string()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
        let end_msg = serde_json::json!({"type": "end", "message": "SSH session ended"});
        let _ = ws_sender.send(Message::Text(end_msg.to_string())).await;
    });

    // Forward WebSocket messages → SSH stdin
    loop {
        match ws_receiver.next().await {
            Some(Ok(Message::Text(text))) => {
                #[derive(serde::Deserialize)]
                struct TermMsg {
                    #[serde(rename = "type")]
                    kind: String,
                    data: Option<String>,
                }
                if let Ok(msg) = serde_json::from_str::<TermMsg>(&text) {
                    if msg.kind == "data" {
                        if let Some(data) = msg.data {
                            use tokio::io::AsyncWriteExt;
                            if child_stdin.write_all(data.as_bytes()).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            Some(Ok(Message::Binary(bytes))) => {
                use tokio::io::AsyncWriteExt;
                if child_stdin.write_all(&bytes).await.is_err() {
                    break;
                }
            }
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {}
            Some(Ok(Message::Close(_))) | None => break,
            _ => {}
        }
    }

    stdout_task.abort();
    let _ = child.kill().await;
    drop(key_file); // Temp file deleted here
}

// ---------------------------------------------------------------------------
// OS Patch Notifications
// ---------------------------------------------------------------------------

/// Response for the GET /api/servers/:id/patches endpoint.
#[derive(Debug, Serialize)]
pub struct PatchesResponse {
    pub security_updates: u32,
    pub total_updates: u32,
    pub packages: Vec<String>,
    pub checked_at: String,
}

/// Check a remote server for pending OS/security updates via SSH.
///
/// GET /api/servers/:id/patches
pub async fn check_server_patches(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PatchesResponse>, StatusCode> {
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch server for patch check: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let enc_key = get_encryption_key(&state);

    let private_key_content = if let Some(ref encrypted_key) = server.ssh_private_key {
        match crypto::decrypt_if_encrypted(encrypted_key, enc_key.as_ref()) {
            Ok(key) => Some(key),
            Err(e) => {
                tracing::warn!("Failed to decrypt SSH key for patch check {}: {}", id, e);
                None
            }
        }
    } else {
        None
    };

    let password_content = if let Some(ref encrypted_pwd) = server.ssh_password {
        match crypto::decrypt_if_encrypted(encrypted_pwd, enc_key.as_ref()) {
            Ok(pwd) => Some(pwd),
            Err(e) => {
                tracing::warn!(
                    "Failed to decrypt SSH password for patch check {}: {}",
                    id,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    let result = run_ssh_patch_check(
        &server.host,
        server.port as u16,
        &server.username,
        private_key_content.as_deref(),
        password_content.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::warn!("Patch check SSH failed for server {}: {}", id, e);
        StatusCode::BAD_GATEWAY
    })?;

    Ok(Json(result))
}

/// SSH into a server and enumerate pending package upgrades.
async fn run_ssh_patch_check(
    host: &str,
    port: u16,
    username: &str,
    private_key: Option<&str>,
    password: Option<&str>,
) -> anyhow::Result<PatchesResponse> {
    use std::io::Write;
    use tokio::process::Command;

    let key_file = if let Some(key_content) = private_key {
        let mut tmpfile = tempfile::Builder::new()
            .prefix("rivetr-ssh-key-")
            .suffix(".pem")
            .tempfile()?;
        tmpfile.write_all(key_content.as_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(tmpfile.path(), std::fs::Permissions::from_mode(0o600))?;
        }
        Some(tmpfile)
    } else {
        None
    };

    // Detect package manager and list upgradable packages in one SSH call.
    let remote_cmd = r#"
        if command -v apt-get >/dev/null 2>&1; then
            apt-get -qq update >/dev/null 2>&1 || true;
            echo "PKG_MANAGER:apt";
            apt list --upgradable 2>/dev/null | grep -v "^Listing" || true;
        elif command -v dnf >/dev/null 2>&1; then
            echo "PKG_MANAGER:dnf";
            dnf check-update 2>/dev/null | grep -v "^$" | grep -v "^Last" || true;
        elif command -v yum >/dev/null 2>&1; then
            echo "PKG_MANAGER:yum";
            yum check-update 2>/dev/null | grep -v "^$" | grep -v "^Loaded" || true;
        else
            echo "PKG_MANAGER:unknown";
        fi
    "#;

    let use_sshpass = password.is_some() && key_file.is_none();

    let mut cmd = if use_sshpass {
        let mut c = Command::new("sshpass");
        c.arg("-p").arg(password.unwrap());
        c.arg("ssh");
        c
    } else {
        Command::new("ssh")
    };

    cmd.arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("ConnectTimeout=10");

    if !use_sshpass {
        cmd.arg("-o").arg("BatchMode=yes");
    }

    cmd.arg("-p").arg(port.to_string());

    if let Some(ref kf) = key_file {
        cmd.arg("-i").arg(kf.path());
    }

    cmd.arg(format!("{}@{}", username, host)).arg(remote_cmd);

    let output = cmd.output().await?;

    // yum check-update exits 100 when updates are available — treat as success
    let success = output.status.success() || output.status.code() == Some(100);

    if !success {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("SSH command failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let checked_at = chrono::Utc::now().to_rfc3339();

    let mut pkg_manager = "apt";
    let mut packages: Vec<String> = Vec::new();
    let mut security_updates: u32 = 0;

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(mgr) = line.strip_prefix("PKG_MANAGER:") {
            pkg_manager = if mgr == "dnf" || mgr == "yum" {
                "yum"
            } else {
                "apt"
            };
            continue;
        }
        if line.is_empty() {
            continue;
        }

        if pkg_manager == "apt" {
            // Lines look like: "package/focal-security 1.2.3 amd64 [upgradable from: 1.2.2]"
            if line.contains('/') {
                packages.push(line.to_string());
                if line.contains("security") {
                    security_updates += 1;
                }
            }
        } else {
            // yum/dnf: "package.arch  version  repo"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                packages.push(line.to_string());
                let repo = parts[2].to_lowercase();
                if repo.contains("security") || repo.contains("sec") {
                    security_updates += 1;
                }
            }
        }
    }

    let total_updates = packages.len() as u32;

    // Keep the package list manageable
    if packages.len() > 50 {
        packages.truncate(50);
    }

    Ok(PatchesResponse {
        security_updates,
        total_updates,
        packages,
        checked_at,
    })
}

// ---------------------------------------------------------------------------
// Server Security Checklist
// ---------------------------------------------------------------------------

/// A single security check result item.
#[derive(Debug, Serialize)]
pub struct SecurityCheckItem {
    pub id: String,
    pub name: String,
    pub description: String,
    /// One of: "pass", "fail", "warn", "unknown"
    pub status: String,
    pub details: Option<String>,
}

/// Response for GET /api/servers/:id/security-check.
#[derive(Debug, Serialize)]
pub struct SecurityCheckResponse {
    pub items: Vec<SecurityCheckItem>,
    pub checked_at: String,
}

/// Run a security checklist against a remote server via SSH.
///
/// GET /api/servers/:id/security-check
pub async fn check_server_security(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SecurityCheckResponse>, StatusCode> {
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch server for security check: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let enc_key = get_encryption_key(&state);

    let private_key_content = if let Some(ref encrypted_key) = server.ssh_private_key {
        match crypto::decrypt_if_encrypted(encrypted_key, enc_key.as_ref()) {
            Ok(key) => Some(key),
            Err(e) => {
                tracing::warn!("Failed to decrypt SSH key for security check {}: {}", id, e);
                None
            }
        }
    } else {
        None
    };

    let password_content = if let Some(ref encrypted_pwd) = server.ssh_password {
        match crypto::decrypt_if_encrypted(encrypted_pwd, enc_key.as_ref()) {
            Ok(pwd) => Some(pwd),
            Err(e) => {
                tracing::warn!(
                    "Failed to decrypt SSH password for security check {}: {}",
                    id,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    let result = run_ssh_security_check(
        &server.host,
        server.port as u16,
        &server.username,
        private_key_content.as_deref(),
        password_content.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::warn!("Security check SSH failed for server {}: {}", id, e);
        StatusCode::BAD_GATEWAY
    })?;

    Ok(Json(result))
}

/// Run multiple SSH commands to evaluate common security best practices.
async fn run_ssh_security_check(
    host: &str,
    port: u16,
    username: &str,
    private_key: Option<&str>,
    password: Option<&str>,
) -> anyhow::Result<SecurityCheckResponse> {
    use std::io::Write;
    use tokio::process::Command;

    let key_file = if let Some(key_content) = private_key {
        let mut tmpfile = tempfile::Builder::new()
            .prefix("rivetr-ssh-key-")
            .suffix(".pem")
            .tempfile()?;
        tmpfile.write_all(key_content.as_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(tmpfile.path(), std::fs::Permissions::from_mode(0o600))?;
        }
        Some(tmpfile)
    } else {
        None
    };

    // All checks run in a single SSH connection.
    // Each section is prefixed with a "CHECK:" sentinel for parsing.
    let remote_cmd = r#"
        echo "CHECK:ssh_root_login";
        grep -E "^PermitRootLogin" /etc/ssh/sshd_config 2>/dev/null || echo "MISSING";

        echo "CHECK:password_auth";
        grep -E "^PasswordAuthentication" /etc/ssh/sshd_config 2>/dev/null || echo "MISSING";

        echo "CHECK:firewall";
        if command -v ufw >/dev/null 2>&1; then
            ufw status 2>/dev/null | head -1;
        elif command -v firewall-cmd >/dev/null 2>&1; then
            firewall-cmd --state 2>/dev/null;
        else
            echo "NOT_FOUND";
        fi;

        echo "CHECK:ssh_port";
        grep -E "^Port " /etc/ssh/sshd_config 2>/dev/null || echo "MISSING";

        echo "CHECK:fail2ban";
        systemctl is-active fail2ban 2>/dev/null || echo "inactive";

        echo "CHECK:docker_exposed";
        (ss -tlnp 2>/dev/null | grep -E ":(2375|2376)\s" || true);
        (grep -rE "0\.0\.0\.0:2375|tcp://0\.0\.0\.0" /etc/docker/ 2>/dev/null || echo "NOT_EXPOSED");

        echo "CHECK:unattended_upgrades";
        dpkg -l unattended-upgrades 2>/dev/null | grep -E "^ii" || echo "NOT_INSTALLED";

        echo "CHECK:security_updates";
        if command -v apt-get >/dev/null 2>&1; then
            apt list --upgradable 2>/dev/null | grep -c security || echo "0";
        else
            echo "0";
        fi;
    "#;

    let use_sshpass = password.is_some() && key_file.is_none();

    let mut cmd = if use_sshpass {
        let mut c = Command::new("sshpass");
        c.arg("-p").arg(password.unwrap());
        c.arg("ssh");
        c
    } else {
        Command::new("ssh")
    };

    cmd.arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("ConnectTimeout=10");

    if !use_sshpass {
        cmd.arg("-o").arg("BatchMode=yes");
    }

    cmd.arg("-p").arg(port.to_string());

    if let Some(ref kf) = key_file {
        cmd.arg("-i").arg(kf.path());
    }

    cmd.arg(format!("{}@{}", username, host)).arg(remote_cmd);

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("SSH command failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let checked_at = chrono::Utc::now().to_rfc3339();

    // Parse into sections keyed by check id
    let mut sections: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut current_check: Option<String> = None;

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(check_id) = line.strip_prefix("CHECK:") {
            current_check = Some(check_id.to_string());
            sections.entry(check_id.to_string()).or_default();
        } else if let Some(ref check) = current_check {
            if !line.is_empty() {
                sections
                    .entry(check.clone())
                    .or_default()
                    .push(line.to_string());
            }
        }
    }

    let get_lines = |key: &str| -> Vec<String> { sections.get(key).cloned().unwrap_or_default() };

    let mut items: Vec<SecurityCheckItem> = Vec::new();

    // 1. SSH root login
    {
        let lines = get_lines("ssh_root_login");
        let raw = lines.first().map(|s| s.as_str()).unwrap_or("MISSING");
        let val = raw.to_lowercase();
        let (status, details) = if raw == "MISSING" {
            (
                "warn",
                Some("PermitRootLogin not explicitly set in sshd_config".to_string()),
            )
        } else if val.contains("yes") && !val.contains("without") && !val.contains("forced") {
            ("fail", Some(format!("Root login is permitted: {}", raw)))
        } else {
            ("pass", Some(format!("Root login restricted: {}", raw)))
        };
        items.push(SecurityCheckItem {
            id: "ssh_root_login".to_string(),
            name: "SSH Root Login Disabled".to_string(),
            description: "Direct root login over SSH should be prohibited.".to_string(),
            status: status.to_string(),
            details,
        });
    }

    // 2. Password authentication
    {
        let lines = get_lines("password_auth");
        let raw = lines.first().map(|s| s.as_str()).unwrap_or("MISSING");
        let val = raw.to_lowercase();
        let (status, details) = if raw == "MISSING" {
            (
                "warn",
                Some("PasswordAuthentication not explicitly set".to_string()),
            )
        } else if val.contains("yes") {
            (
                "warn",
                Some(format!("Password authentication is enabled: {}", raw)),
            )
        } else {
            (
                "pass",
                Some(format!("Password authentication disabled: {}", raw)),
            )
        };
        items.push(SecurityCheckItem {
            id: "password_auth".to_string(),
            name: "SSH Password Authentication Disabled".to_string(),
            description: "Key-based authentication is more secure than passwords.".to_string(),
            status: status.to_string(),
            details,
        });
    }

    // 3. Firewall active
    {
        let lines = get_lines("firewall");
        let raw = lines.first().map(|s| s.as_str()).unwrap_or("NOT_FOUND");
        let val = raw.to_lowercase();
        let (status, details) = if raw == "NOT_FOUND" {
            (
                "warn",
                Some("No recognised firewall (ufw/firewalld) found".to_string()),
            )
        } else if (val.contains("active") && !val.contains("inactive")) || val.contains("running") {
            ("pass", Some(format!("Firewall is active: {}", raw)))
        } else {
            ("fail", Some(format!("Firewall is not active: {}", raw)))
        };
        items.push(SecurityCheckItem {
            id: "firewall".to_string(),
            name: "Firewall Active".to_string(),
            description: "A host firewall (ufw or firewalld) should be enabled.".to_string(),
            status: status.to_string(),
            details,
        });
    }

    // 4. SSH on non-standard port
    {
        let lines = get_lines("ssh_port");
        let raw = lines.first().map(|s| s.as_str()).unwrap_or("MISSING");
        let (status, details) = if raw == "MISSING" {
            (
                "warn",
                Some(
                    "SSH port not explicitly configured — likely using default port 22".to_string(),
                ),
            )
        } else {
            let port_val = raw.trim_start_matches("Port ").trim();
            if port_val == "22" {
                (
                    "warn",
                    Some("SSH is running on the default port 22".to_string()),
                )
            } else {
                (
                    "pass",
                    Some(format!("SSH is on a non-standard port: {}", port_val)),
                )
            }
        };
        items.push(SecurityCheckItem {
            id: "ssh_port".to_string(),
            name: "SSH Non-Standard Port".to_string(),
            description: "Moving SSH to a non-standard port reduces automated scan noise."
                .to_string(),
            status: status.to_string(),
            details,
        });
    }

    // 5. Fail2ban
    {
        let lines = get_lines("fail2ban");
        let raw = lines.first().map(|s| s.as_str()).unwrap_or("inactive");
        let val = raw.trim().to_lowercase();
        let (status, details) = if val == "active" {
            ("pass", Some("fail2ban service is active".to_string()))
        } else {
            (
                "warn",
                Some(format!("fail2ban is not active (status: {})", raw)),
            )
        };
        items.push(SecurityCheckItem {
            id: "fail2ban".to_string(),
            name: "Fail2ban Active".to_string(),
            description: "fail2ban helps block brute-force SSH attacks.".to_string(),
            status: status.to_string(),
            details,
        });
    }

    // 6. Docker daemon not exposed on TCP
    {
        let lines = get_lines("docker_exposed");
        let is_not_exposed = lines.iter().all(|l| l.trim() == "NOT_EXPOSED");
        let has_exposure = !is_not_exposed
            && lines.iter().any(|l| {
                let lower = l.to_lowercase();
                lower.contains("2375") || lower.contains("2376") || lower.contains("tcp://0.0.0.0")
            });
        let (status, details) = if has_exposure {
            (
                "fail",
                Some("Docker daemon appears to be listening on a TCP port".to_string()),
            )
        } else {
            (
                "pass",
                Some("Docker daemon is not exposed on TCP".to_string()),
            )
        };
        items.push(SecurityCheckItem {
            id: "docker_exposed".to_string(),
            name: "Docker Daemon Not Exposed on TCP".to_string(),
            description: "Docker's TCP socket (port 2375/2376) should not be publicly reachable."
                .to_string(),
            status: status.to_string(),
            details,
        });
    }

    // 7. Unattended upgrades
    {
        let lines = get_lines("unattended_upgrades");
        let raw = lines.first().map(|s| s.as_str()).unwrap_or("NOT_INSTALLED");
        let (status, details) = if raw.starts_with("ii") {
            ("pass", Some("unattended-upgrades is installed".to_string()))
        } else {
            (
                "warn",
                Some(
                    "unattended-upgrades is not installed — auto security patches disabled"
                        .to_string(),
                ),
            )
        };
        items.push(SecurityCheckItem {
            id: "unattended_upgrades".to_string(),
            name: "Unattended Security Upgrades".to_string(),
            description: "Automatic security updates help keep the system patched.".to_string(),
            status: status.to_string(),
            details,
        });
    }

    // 8. Pending security updates count
    {
        let lines = get_lines("security_updates");
        let count_str = lines.first().map(|s| s.as_str()).unwrap_or("0");
        let count: u32 = count_str.trim().parse().unwrap_or(0);
        let (status, details) = if count == 0 {
            ("pass", Some("No pending security updates".to_string()))
        } else if count <= 5 {
            (
                "warn",
                Some(format!("{} pending security update(s)", count)),
            )
        } else {
            (
                "fail",
                Some(format!(
                    "{} pending security updates — apply immediately",
                    count
                )),
            )
        };
        items.push(SecurityCheckItem {
            id: "security_updates".to_string(),
            name: "Pending Security Updates".to_string(),
            description: "Outstanding security patches should be applied promptly.".to_string(),
            status: status.to_string(),
            details,
        });
    }

    Ok(SecurityCheckResponse { items, checked_at })
}
// ---------------------------------------------------------------------------
// Fetch Server Details Endpoint
// ---------------------------------------------------------------------------

/// Human-readable server detail fields returned by POST /api/servers/:id/fetch-details
#[derive(Debug, Serialize)]
pub struct ServerDetails {
    pub os_name: String,
    pub docker_version: String,
    pub disk_free: String,
    pub cpu_cores: String,
    pub total_ram: String,
}

/// POST /api/servers/:id/fetch-details
///
/// SSHes into the server, fetches OS info, Docker version, disk space, CPU cores,
/// and total RAM, then stores them on the server record.
pub async fn fetch_details(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ServerDetails>, (StatusCode, Json<serde_json::Value>)> {
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch server for details: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch server"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Server not found"})),
            )
        })?;

    let enc_key = get_encryption_key(&state);

    let private_key_content = if let Some(ref encrypted_key) = server.ssh_private_key {
        match crypto::decrypt_if_encrypted(encrypted_key, enc_key.as_ref()) {
            Ok(key) => Some(key),
            Err(e) => {
                tracing::warn!("Failed to decrypt SSH key for server {}: {}", id, e);
                None
            }
        }
    } else {
        None
    };

    let password_content = if let Some(ref encrypted_pwd) = server.ssh_password {
        match crypto::decrypt_if_encrypted(encrypted_pwd, enc_key.as_ref()) {
            Ok(pwd) => Some(pwd),
            Err(e) => {
                tracing::warn!("Failed to decrypt SSH password for server {}: {}", id, e);
                None
            }
        }
    } else {
        None
    };

    let details = run_fetch_server_details(
        &server.host,
        server.port as u16,
        &server.username,
        private_key_content.as_deref(),
        password_content.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::warn!("fetch-details failed for server {}: {}", id, e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("SSH command failed: {}", e)})),
        )
    })?;

    // Persist the gathered info into the server record
    let os_info_json = serde_json::json!({
        "os_name": details.os_name,
        "docker_version": details.docker_version,
        "disk_free": details.disk_free,
        "cpu_cores": details.cpu_cores,
        "total_ram": details.total_ram,
    })
    .to_string();

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE servers SET os_info = ?, docker_version = ?, updated_at = ? WHERE id = ?")
        .bind(&os_info_json)
        .bind(&details.docker_version)
        .bind(&now)
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update server details: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to save server details"})),
            )
        })?;

    Ok(Json(details))
}

/// Run SSH commands to gather detailed server info.
async fn run_fetch_server_details(
    host: &str,
    port: u16,
    username: &str,
    private_key: Option<&str>,
    password: Option<&str>,
) -> anyhow::Result<ServerDetails> {
    use std::io::Write;
    use tokio::process::Command;

    let key_file = if let Some(key_content) = private_key {
        let mut tmpfile = tempfile::Builder::new()
            .prefix("rivetr-ssh-key-")
            .suffix(".pem")
            .tempfile()?;
        tmpfile.write_all(key_content.as_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(tmpfile.path(), std::fs::Permissions::from_mode(0o600))?;
        }
        Some(tmpfile)
    } else {
        None
    };

    let remote_cmd = r#"
        echo "OS:$(grep PRETTY_NAME /etc/os-release 2>/dev/null | cut -d= -f2 | tr -d '"' || uname -s)";
        echo "DOCKER:$(docker version --format '{{.Server.Version}}' 2>/dev/null || echo 'not installed')";
        echo "DISK_FREE:$(df -h / 2>/dev/null | tail -1 | awk '{print $4}' || echo 'N/A')";
        echo "CPU_CORES:$(nproc 2>/dev/null || echo 'N/A')";
        echo "TOTAL_RAM:$(free -h 2>/dev/null | grep Mem | awk '{print $2}' || echo 'N/A')"
    "#;

    let use_sshpass = password.is_some() && key_file.is_none();

    let mut cmd = if use_sshpass {
        let mut c = Command::new("sshpass");
        c.arg("-p").arg(password.unwrap());
        c.arg("ssh");
        c
    } else {
        Command::new("ssh")
    };

    cmd.arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("ConnectTimeout=10");

    if !use_sshpass {
        cmd.arg("-o").arg("BatchMode=yes");
    }

    cmd.arg("-p").arg(port.to_string());

    if let Some(ref kf) = key_file {
        cmd.arg("-i").arg(kf.path());
    }

    cmd.arg(format!("{}@{}", username, host)).arg(remote_cmd);

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let (Some(_), Some(pw)) = (&key_file, password) {
            tracing::warn!(
                "SSH key auth failed ({}), retrying with password",
                stderr.trim()
            );
            let mut fallback = Command::new("sshpass");
            fallback
                .arg("-p")
                .arg(pw)
                .arg("ssh")
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-o")
                .arg("ConnectTimeout=10")
                .arg("-p")
                .arg(port.to_string())
                .arg(format!("{}@{}", username, host))
                .arg(remote_cmd);
            let fallback_output = fallback.output().await?;
            if !fallback_output.status.success() {
                let fb_err = String::from_utf8_lossy(&fallback_output.stderr);
                return Err(anyhow::anyhow!("SSH command failed: {}", fb_err));
            }
            let stdout = String::from_utf8_lossy(&fallback_output.stdout);
            return parse_server_details(&stdout);
        }
        return Err(anyhow::anyhow!("SSH command failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_server_details(&stdout)
}

/// Parse the output of `run_fetch_server_details`.
fn parse_server_details(output: &str) -> anyhow::Result<ServerDetails> {
    let mut os_name = "Unknown".to_string();
    let mut docker_version = "not installed".to_string();
    let mut disk_free = "N/A".to_string();
    let mut cpu_cores = "N/A".to_string();
    let mut total_ram = "N/A".to_string();

    for line in output.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("OS:") {
            let v = val.trim();
            if !v.is_empty() {
                os_name = v.to_string();
            }
        } else if let Some(val) = line.strip_prefix("DOCKER:") {
            let v = val.trim();
            if !v.is_empty() {
                docker_version = v.to_string();
            }
        } else if let Some(val) = line.strip_prefix("DISK_FREE:") {
            let v = val.trim();
            if !v.is_empty() {
                disk_free = v.to_string();
            }
        } else if let Some(val) = line.strip_prefix("CPU_CORES:") {
            let v = val.trim();
            if !v.is_empty() {
                cpu_cores = v.to_string();
            }
        } else if let Some(val) = line.strip_prefix("TOTAL_RAM:") {
            let v = val.trim();
            if !v.is_empty() {
                total_ram = v.to_string();
            }
        }
    }

    Ok(ServerDetails {
        os_name,
        docker_version,
        disk_free,
        cpu_cores,
        total_ram,
    })
}
