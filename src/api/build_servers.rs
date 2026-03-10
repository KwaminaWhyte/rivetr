//! Build server management API endpoints.
//!
//! Provides CRUD operations for dedicated remote build servers and SSH-based health checks.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::crypto;
use crate::db::{BuildServer, CreateBuildServerRequest, UpdateBuildServerRequest};
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
pub struct ListBuildServersQuery {
    pub team_id: Option<String>,
}

/// List all build servers, optionally filtered by team_id
pub async fn list_build_servers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListBuildServersQuery>,
) -> Result<Json<Vec<BuildServer>>, StatusCode> {
    let servers = if let Some(team_id) = &query.team_id {
        sqlx::query_as::<_, BuildServer>(
            "SELECT * FROM build_servers WHERE team_id = ? ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, BuildServer>("SELECT * FROM build_servers ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
    }
    .map_err(|e| {
        tracing::error!("Failed to list build servers: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Mask the SSH private key in responses (never return raw key)
    let masked: Vec<BuildServer> = servers
        .into_iter()
        .map(|mut s| {
            s.ssh_private_key = s.ssh_private_key.map(|_| "[encrypted]".to_string());
            s
        })
        .collect();

    Ok(Json(masked))
}

/// Create a new build server record
pub async fn create_build_server(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateBuildServerRequest>,
) -> Result<(StatusCode, Json<BuildServer>), StatusCode> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let port = req.port.unwrap_or(22);
    let username = req.username.unwrap_or_else(|| "root".to_string());
    let concurrent_builds = req.concurrent_builds.unwrap_or(2);

    // Encrypt SSH private key if provided
    let encrypted_key = if let Some(ref key) = req.ssh_private_key {
        let enc_key = get_encryption_key(&state);
        let encrypted = crypto::encrypt_if_key_available(key, enc_key.as_ref())
            .map_err(|e| {
                tracing::error!("Failed to encrypt SSH private key: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        Some(encrypted)
    } else {
        None
    };

    sqlx::query(
        r#"
        INSERT INTO build_servers (id, name, host, port, username, ssh_private_key, status, concurrent_builds, team_id, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, 'unknown', ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.host)
    .bind(port)
    .bind(&username)
    .bind(&encrypted_key)
    .bind(concurrent_builds)
    .bind(&req.team_id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create build server: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut server = sqlx::query_as::<_, BuildServer>("SELECT * FROM build_servers WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Mask SSH key in response
    server.ssh_private_key = server.ssh_private_key.map(|_| "[encrypted]".to_string());

    Ok((StatusCode::CREATED, Json(server)))
}

/// Get a single build server by ID
pub async fn get_build_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<BuildServer>, StatusCode> {
    let mut server =
        sqlx::query_as::<_, BuildServer>("SELECT * FROM build_servers WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get build server: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

    // Mask SSH key in response
    server.ssh_private_key = server.ssh_private_key.map(|_| "[encrypted]".to_string());

    Ok(Json(server))
}

/// Update a build server record
pub async fn update_build_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateBuildServerRequest>,
) -> Result<Json<BuildServer>, StatusCode> {
    // Verify server exists
    let _existing =
        sqlx::query_as::<_, BuildServer>("SELECT * FROM build_servers WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

    let now = chrono::Utc::now().to_rfc3339();

    // Encrypt SSH private key if a new one is provided
    let encrypted_key = if let Some(ref key) = req.ssh_private_key {
        let enc_key = get_encryption_key(&state);
        let encrypted = crypto::encrypt_if_key_available(key, enc_key.as_ref())
            .map_err(|e| {
                tracing::error!("Failed to encrypt SSH private key: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        Some(encrypted)
    } else {
        None
    };

    sqlx::query(
        r#"
        UPDATE build_servers SET
            name = COALESCE(?, name),
            host = COALESCE(?, host),
            port = COALESCE(?, port),
            username = COALESCE(?, username),
            ssh_private_key = COALESCE(?, ssh_private_key),
            concurrent_builds = COALESCE(?, concurrent_builds),
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&req.name)
    .bind(&req.host)
    .bind(req.port)
    .bind(&req.username)
    .bind(&encrypted_key)
    .bind(req.concurrent_builds)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update build server: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut server = sqlx::query_as::<_, BuildServer>("SELECT * FROM build_servers WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Mask SSH key in response
    server.ssh_private_key = server.ssh_private_key.map(|_| "[encrypted]".to_string());

    Ok(Json(server))
}

/// Delete a build server
pub async fn delete_build_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM build_servers WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete build server: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Trigger a health check on a remote build server via SSH.
///
/// Runs SSH commands to gather Docker version, CPU count, and memory information,
/// then updates the build server record in the database.
pub async fn check_build_server_health(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<BuildServer>, StatusCode> {
    // Fetch the build server record (with the encrypted key)
    let server = sqlx::query_as::<_, BuildServer>("SELECT * FROM build_servers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch build server for health check: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Decrypt the SSH private key if available
    let private_key_content = if let Some(ref encrypted_key) = server.ssh_private_key {
        let enc_key = get_encryption_key(&state);
        match crypto::decrypt_if_encrypted(encrypted_key, enc_key.as_ref()) {
            Ok(key) => Some(key),
            Err(e) => {
                tracing::warn!("Failed to decrypt SSH key for build server {}: {}", id, e);
                None
            }
        }
    } else {
        None
    };

    // Run the health check
    let health_result = run_build_server_health_check(
        &server.host,
        server.port as u16,
        &server.username,
        private_key_content.as_deref(),
    )
    .await;

    let now = chrono::Utc::now().to_rfc3339();

    match health_result {
        Ok(stats) => {
            sqlx::query(
                r#"
                UPDATE build_servers SET
                    status = 'online',
                    last_seen_at = ?,
                    docker_version = ?,
                    cpu_count = ?,
                    memory_bytes = ?,
                    updated_at = ?
                WHERE id = ?
                "#,
            )
            .bind(&now)
            .bind(&stats.docker_version)
            .bind(stats.cpu_count)
            .bind(stats.memory_bytes)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update build server health stats: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }
        Err(e) => {
            tracing::warn!("Health check failed for build server {}: {}", id, e);
            sqlx::query(
                "UPDATE build_servers SET status = 'offline', updated_at = ? WHERE id = ?",
            )
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update build server status to offline: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }
    }

    let mut updated =
        sqlx::query_as::<_, BuildServer>("SELECT * FROM build_servers WHERE id = ?")
            .bind(&id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Mask SSH key in response
    updated.ssh_private_key = updated.ssh_private_key.map(|_| "[encrypted]".to_string());

    Ok(Json(updated))
}

/// Gathered stats from a build server SSH health check
struct BuildServerHealthStats {
    docker_version: Option<String>,
    cpu_count: Option<i64>,
    memory_bytes: Option<i64>,
}

/// Run SSH commands to gather build server health statistics.
async fn run_build_server_health_check(
    host: &str,
    port: u16,
    username: &str,
    private_key: Option<&str>,
) -> anyhow::Result<BuildServerHealthStats> {
    use std::io::Write;
    use tokio::process::Command;

    // Write private key to a temp file if provided
    let key_file = if let Some(key_content) = private_key {
        let mut tmpfile = tempfile::Builder::new()
            .prefix("rivetr-build-ssh-key-")
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
        echo "DOCKER:$(docker version --format '{{.Server.Version}}' 2>/dev/null || echo 'not installed')";
        echo "CPU:$(nproc 2>/dev/null || echo 'N/A')";
        echo "MEM:$(free -b 2>/dev/null | awk 'NR==2{print $2}' || echo 'N/A')"
    "#;

    let mut cmd = Command::new("ssh");
    cmd.arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("ConnectTimeout=5")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-p")
        .arg(port.to_string());

    if let Some(ref key_file) = key_file {
        cmd.arg("-i").arg(key_file.path());
    }

    cmd.arg(format!("{}@{}", username, host))
        .arg(remote_cmd);

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("SSH command failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_build_server_health_output(&stdout)
}

/// Parse the output from the build server SSH health check commands.
fn parse_build_server_health_output(output: &str) -> anyhow::Result<BuildServerHealthStats> {
    let mut docker_version: Option<String> = None;
    let mut cpu_count: Option<i64> = None;
    let mut memory_bytes: Option<i64> = None;

    for line in output.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("DOCKER:") {
            let val = val.trim();
            if !val.is_empty() {
                docker_version = Some(val.to_string());
            }
        } else if let Some(val) = line.strip_prefix("CPU:") {
            let val = val.trim();
            if val != "N/A" {
                if let Ok(v) = val.parse::<i64>() {
                    cpu_count = Some(v);
                }
            }
        } else if let Some(val) = line.strip_prefix("MEM:") {
            let val = val.trim();
            if val != "N/A" {
                if let Ok(v) = val.parse::<i64>() {
                    memory_bytes = Some(v);
                }
            }
        }
    }

    Ok(BuildServerHealthStats {
        docker_version,
        cpu_count,
        memory_bytes,
    })
}
