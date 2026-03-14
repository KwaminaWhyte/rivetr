//! Remote filesystem browser API endpoints.
//!
//! Allows browsing and editing files on connected remote servers via SSH.
//! Uses the same SSH infrastructure as the server health checks and terminal.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::crypto;
use crate::db::{Server, User};
use crate::engine::remote::RemoteContext;
use crate::AppState;

use super::error::ApiError;

const KEY_LENGTH: usize = 32;

fn get_encryption_key(state: &AppState) -> Option<[u8; KEY_LENGTH]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crypto::derive_key(secret))
}

/// A single file or directory entry
#[derive(Debug, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<String>,
    pub permissions: Option<String>,
}

/// Query params for directory listing
#[derive(Debug, Deserialize)]
pub struct BrowseQuery {
    pub path: Option<String>,
}

/// Query params for reading a file
#[derive(Debug, Deserialize)]
pub struct FileContentQuery {
    pub path: String,
}

/// Request body for writing a file
#[derive(Debug, Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
}

/// Query params for deleting a file
#[derive(Debug, Deserialize)]
pub struct DeleteFileQuery {
    pub path: String,
}

/// Helper: fetch server and build a RemoteContext, decrypting SSH credentials.
async fn get_remote_context(
    state: &AppState,
    server_id: &str,
) -> Result<(Server, RemoteContext, Option<String>), ApiError> {
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(server_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Server not found"))?;

    let enc_key = get_encryption_key(state);

    // Decrypt SSH private key
    let private_key_content = if let Some(ref encrypted_key) = server.ssh_private_key {
        match crypto::decrypt_if_encrypted(encrypted_key, enc_key.as_ref()) {
            Ok(key) => Some(key),
            Err(e) => {
                tracing::warn!("Failed to decrypt SSH key for server {}: {}", server_id, e);
                None
            }
        }
    } else {
        None
    };

    // Decrypt SSH password
    let password_content = if let Some(ref encrypted_pwd) = server.ssh_password {
        match crypto::decrypt_if_encrypted(encrypted_pwd, enc_key.as_ref()) {
            Ok(pwd) => Some(pwd),
            Err(e) => {
                tracing::warn!(
                    "Failed to decrypt SSH password for server {}: {}",
                    server_id,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    // Write key to a temp file if available (RemoteContext takes a path)
    let key_path = if let Some(ref key) = private_key_content {
        let tmp = format!("/tmp/rivetr-fs-key-{}", uuid::Uuid::new_v4());
        std::fs::write(&tmp, key).map_err(|e| {
            tracing::error!("Failed to write temp key: {}", e);
            ApiError::internal("Failed to prepare SSH key")
        })?;
        // chmod 600
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        Some(tmp)
    } else {
        None
    };

    let ctx = RemoteContext {
        host: server.host.clone(),
        port: server.port,
        username: server.username.clone(),
        key_path: key_path.clone(),
        ssh_password: password_content,
    };

    Ok((server, ctx, key_path))
}

/// Clean up a temp key file after use.
fn cleanup_key(key_path: Option<String>) {
    if let Some(path) = key_path {
        let _ = std::fs::remove_file(path);
    }
}

/// Parse `ls -la --full-time` output into FileEntry structs.
/// Expected format (GNU coreutils):
///   drwxr-xr-x 2 root root 4096 2024-01-15T12:34:56Z dirname
///   -rw-r--r-- 1 root root 1234 2024-01-15T12:34:56Z filename
fn parse_ls_output(output: &str, base_path: &str) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    for line in output.lines() {
        // Skip total line and empty lines
        if line.starts_with("total") || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(9, ' ').filter(|s| !s.is_empty()).collect();
        // We need at least 9 columns: perms, links, user, group, size, date, time(?), tz(?), name
        // With --full-time and --time-style=+%Y-%m-%dT%H:%M:%SZ we get one date column
        if parts.len() < 6 {
            continue;
        }

        let permissions = parts[0].to_string();
        let is_dir = permissions.starts_with('d');
        let size: Option<u64> = parts[4].parse().ok();

        // The date is in column index 5 (after perms, links, user, group, size)
        let modified = parts.get(5).map(|s| s.to_string());

        // Name is the last part (may contain spaces if we use splitn correctly)
        let name = parts.last().unwrap_or(&"").to_string();
        // Skip . and ..
        if name == "." || name == ".." {
            continue;
        }

        // Build the full path
        let full_path = if base_path.ends_with('/') {
            format!("{}{}", base_path, name)
        } else {
            format!("{}/{}", base_path, name)
        };

        entries.push(FileEntry {
            name,
            path: full_path,
            is_dir,
            size,
            modified,
            permissions: Some(permissions),
        });
    }
    entries
}

/// GET /api/servers/:id/files?path=/etc
/// List the contents of a remote directory.
pub async fn browse_files(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<BrowseQuery>,
    _user: User,
) -> Result<Json<Vec<FileEntry>>, ApiError> {
    let path = query.path.unwrap_or_else(|| "/".to_string());

    // Basic path safety — disallow null bytes
    if path.contains('\0') {
        return Err(ApiError::bad_request("Invalid path"));
    }

    let (_, ctx, key_path) = get_remote_context(&state, &server_id).await?;

    // Use ls with --time-style to get a consistent date format
    let cmd = format!(
        "ls -la --time-style=+%Y-%m-%dT%H:%M:%SZ \"{}\" 2>&1",
        path.replace('"', "\\\"")
    );

    let (stdout, stderr) = ctx.run_command(&cmd).await.map_err(|e| {
        tracing::error!("SSH command failed for server {}: {}", server_id, e);
        ApiError::internal("Failed to execute remote command")
    })?;
    cleanup_key(key_path);

    if !stderr.is_empty() && stdout.is_empty() {
        return Err(ApiError::bad_request(format!(
            "Remote error: {}",
            stderr.trim()
        )));
    }

    let entries = parse_ls_output(&stdout, &path);
    Ok(Json(entries))
}

/// GET /api/servers/:id/files/content?path=/etc/nginx.conf
/// Read the contents of a remote file.
pub async fn read_file(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<FileContentQuery>,
    _user: User,
) -> Result<Json<serde_json::Value>, ApiError> {
    if query.path.contains('\0') {
        return Err(ApiError::bad_request("Invalid path"));
    }

    let (_, ctx, key_path) = get_remote_context(&state, &server_id).await?;

    let cmd = format!(
        "cat \"{}\" 2>&1",
        query.path.replace('"', "\\\"")
    );

    let (stdout, stderr) = ctx.run_command(&cmd).await.map_err(|e| {
        tracing::error!("SSH command failed for server {}: {}", server_id, e);
        ApiError::internal("Failed to execute remote command")
    })?;
    cleanup_key(key_path);

    if stderr.contains("No such file") || stderr.contains("cannot open") {
        return Err(ApiError::not_found("File not found"));
    }

    Ok(Json(serde_json::json!({ "content": stdout })))
}

/// PUT /api/servers/:id/files/content
/// Write content to a remote file (creates or overwrites).
pub async fn write_file(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    _user: User,
    Json(body): Json<WriteFileRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if body.path.contains('\0') {
        return Err(ApiError::bad_request("Invalid path"));
    }

    let (_, ctx, key_path) = get_remote_context(&state, &server_id).await?;

    // Use printf + redirect; this handles multi-line content and special characters
    // better than echo. We base64-encode the content to avoid shell quoting issues.
    let encoded = base64_encode(&body.content);
    let cmd = format!(
        "echo '{}' | base64 -d > \"{}\" 2>&1; echo exit:$?",
        encoded,
        body.path.replace('"', "\\\"")
    );

    let (stdout, _stderr) = ctx.run_command(&cmd).await.map_err(|e| {
        tracing::error!("SSH write command failed for server {}: {}", server_id, e);
        ApiError::internal("Failed to execute remote command")
    })?;
    cleanup_key(key_path);

    if stdout.contains("exit:0") || stdout.trim().ends_with("exit:0") {
        Ok(Json(serde_json::json!({ "message": "File written successfully" })))
    } else {
        Err(ApiError::internal(format!(
            "Failed to write file: {}",
            stdout.trim()
        )))
    }
}

/// DELETE /api/servers/:id/files?path=/tmp/foo
/// Delete a remote file or empty directory.
pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(query): Query<DeleteFileQuery>,
    _user: User,
) -> Result<StatusCode, ApiError> {
    if query.path.contains('\0') {
        return Err(ApiError::bad_request("Invalid path"));
    }

    // Refuse to delete root or obviously dangerous paths
    let path = query.path.trim();
    if path == "/" || path == "/etc" || path == "/bin" || path == "/usr" || path == "/var" {
        return Err(ApiError::bad_request("Cannot delete system directories"));
    }

    let (_, ctx, key_path) = get_remote_context(&state, &server_id).await?;

    let cmd = format!(
        "rm -rf \"{}\" 2>&1; echo exit:$?",
        path.replace('"', "\\\"")
    );

    let (stdout, _stderr) = ctx.run_command(&cmd).await.map_err(|e| {
        tracing::error!("SSH delete command failed for server {}: {}", server_id, e);
        ApiError::internal("Failed to execute remote command")
    })?;
    cleanup_key(key_path);

    if stdout.contains("exit:0") {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::internal(format!(
            "Failed to delete file: {}",
            stdout.trim()
        )))
    }
}

/// Simple base64 encoding without external crate dependency.
/// Uses the standard alphabet.
fn base64_encode(input: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();

    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] as u32 } else { 0 };

        result.push(ALPHABET[((b0 >> 2) & 0x3F) as usize] as char);
        result.push(ALPHABET[(((b0 << 4) | (b1 >> 4)) & 0x3F) as usize] as char);
        if i + 1 < bytes.len() {
            result.push(ALPHABET[(((b1 << 2) | (b2 >> 6)) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if i + 2 < bytes.len() {
            result.push(ALPHABET[(b2 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        i += 3;
    }
    result
}
