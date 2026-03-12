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
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::crypto;
use crate::db::{App, CreateServerRequest, Server, UpdateServerRequest};
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

    sqlx::query(
        r#"
        INSERT INTO servers (id, name, host, port, username, ssh_private_key, ssh_password, status, team_id, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 'unknown', ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.host)
    .bind(port)
    .bind(&username)
    .bind(&encrypted_key)
    .bind(&encrypted_password)
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
    let _existing = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
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

    sqlx::query(
        r#"
        UPDATE servers SET
            name = COALESCE(?, name),
            host = COALESCE(?, host),
            port = COALESCE(?, port),
            username = COALESCE(?, username),
            ssh_private_key = COALESCE(?, ssh_private_key),
            ssh_password = COALESCE(?, ssh_password),
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
) -> Result<Json<Server>, StatusCode> {
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

    Ok(Json(updated))
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
        echo "DOCKER:$(docker version --format '{{.Server.Version}}' 2>/dev/null || echo 'not installed')"
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
            let val = val.trim();
            if !val.is_empty() {
                docker_version = Some(val.to_string());
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
    })
}

// ---------------------------------------------------------------------------
// App–Server Assignment Endpoints
// ---------------------------------------------------------------------------

/// List all apps assigned to this server (`apps.server_id = :id`).
pub async fn list_server_apps(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<App>>, StatusCode> {
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

    Ok(Json(apps))
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
            match crate::crypto::decrypt_if_encrypted(encrypted_pwd, enc_key.as_ref()) {
                Ok(p) => Some(p),
                Err(_) => None,
            }
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
