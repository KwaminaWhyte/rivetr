//! Container file browser (beta) for apps, databases and services.
//!
//! Directory listings run `ls` inside the container (argv only, never a shell string
//! built from user input). File contents move through the runtime's tar archive API,
//! so binary files survive intact. Every path is normalised to an absolute path with
//! `.` and `..` resolved before it reaches the runtime, and transfers are size capped.

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::Arc;

use crate::db::{Deployment, User};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::authz;
use super::error::ApiError;

/// Largest file shown in the editor.
pub const MAX_VIEW_BYTES: u64 = 1024 * 1024;
/// Largest file that can be downloaded or uploaded.
pub const MAX_TRANSFER_BYTES: u64 = 50 * 1024 * 1024;
/// Request body limit for the upload route (file plus multipart overhead).
pub const UPLOAD_BODY_LIMIT: usize = MAX_TRANSFER_BYTES as usize + 1024 * 1024;
/// Tar headers and padding on top of the file bytes.
const TAR_OVERHEAD: u64 = 64 * 1024;
const MAX_LISTED_ENTRIES: usize = 5000;

// ---------------------------------------------------------------------------
// Path handling
// ---------------------------------------------------------------------------

/// Normalise a container path: must be absolute, no NUL or control characters, `.`
/// and `..` resolved lexically (never above `/`), duplicate slashes removed.
pub fn sanitize_container_path(path: &str) -> Result<String, ApiError> {
    let path = path.trim();
    if path.is_empty() {
        return Ok("/".to_string());
    }
    if path.len() > 4096 {
        return Err(ApiError::bad_request("Path is too long"));
    }
    if !path.starts_with('/') {
        return Err(ApiError::bad_request("Path must be absolute"));
    }
    if path.chars().any(|c| c == '\0' || c.is_control()) {
        return Err(ApiError::bad_request("Path contains invalid characters"));
    }
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    Ok(format!("/{}", parts.join("/")))
}

/// Validate a single file or folder name (no separators, not `.` or `..`).
pub fn sanitize_file_name(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return Err(ApiError::bad_request("Invalid file name"));
    }
    if name.len() > 255 {
        return Err(ApiError::bad_request("File name is too long"));
    }
    if name
        .chars()
        .any(|c| c == '/' || c == '\\' || c == '\0' || c.is_control())
    {
        return Err(ApiError::bad_request(
            "File name cannot contain slashes or control characters",
        ));
    }
    Ok(name.to_string())
}

/// Split a sanitised path into (parent directory, file name).
fn split_path(path: &str) -> Result<(String, String), ApiError> {
    if path == "/" {
        return Err(ApiError::bad_request("Path must point to a file"));
    }
    let idx = path.rfind('/').unwrap_or(0);
    let parent = if idx == 0 {
        "/".to_string()
    } else {
        path[..idx].to_string()
    };
    Ok((parent, path[idx + 1..].to_string()))
}

fn join_path(dir: &str, name: &str) -> String {
    if dir == "/" {
        format!("/{}", name)
    } else {
        format!("{}/{}", dir, name)
    }
}

// ---------------------------------------------------------------------------
// Listing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ContainerFileEntry {
    pub name: String,
    pub path: String,
    /// "dir" | "file" | "symlink" | "other"
    pub kind: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    /// Modification time as Unix seconds
    pub modified: Option<i64>,
    pub permissions: String,
    pub owner: String,
    pub group: String,
    pub link_target: Option<String>,
}

fn kind_of(perms: &str) -> &'static str {
    match perms.chars().next() {
        Some('d') => "dir",
        Some('-') => "file",
        Some('l') => "symlink",
        _ => "other",
    }
}

fn make_entry(
    dir: &str,
    perms: &str,
    owner: &str,
    group: &str,
    size: Option<u64>,
    modified: Option<i64>,
    raw_name: &str,
) -> Option<ContainerFileEntry> {
    let kind = kind_of(perms);
    let (name, link_target) = if kind == "symlink" {
        match raw_name.split_once(" -> ") {
            Some((n, t)) => (n.to_string(), Some(t.to_string())),
            None => (raw_name.to_string(), None),
        }
    } else {
        (raw_name.to_string(), None)
    };
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    Some(ContainerFileEntry {
        path: join_path(dir, &name),
        name,
        kind: kind.to_string(),
        is_dir: kind == "dir",
        size,
        modified,
        permissions: perms.to_string(),
        owner: owner.to_string(),
        group: group.to_string(),
        link_target,
    })
}

/// Parse `ls -la --time-style=+%s` output (GNU coreutils).
pub fn parse_ls_output(dir: &str, output: &str) -> Vec<ContainerFileEntry> {
    let mut out = Vec::new();
    for line in output.lines() {
        if line.starts_with("total ") || line.trim().is_empty() {
            continue;
        }
        // Walk whitespace-separated fields, remembering where the name starts.
        let mut fields: Vec<&str> = Vec::new();
        let mut rest = line;
        // perms, links, owner, group, size (two fields for devices), mtime
        let device = matches!(line.chars().next(), Some('c') | Some('b'));
        let wanted = if device { 7 } else { 6 };
        while fields.len() < wanted {
            rest = rest.trim_start();
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            if end == 0 {
                break;
            }
            fields.push(&rest[..end]);
            rest = &rest[end..];
        }
        if fields.len() < wanted {
            continue;
        }
        let name = rest.strip_prefix(' ').unwrap_or(rest);
        let size = if device { None } else { fields[4].parse().ok() };
        let mtime = fields[wanted - 1].parse().ok();
        if let Some(e) = make_entry(dir, fields[0], fields[2], fields[3], size, mtime, name) {
            out.push(e);
        }
    }
    out
}

/// Parse `stat -c '%A|%U|%G|%s|%Y|%n'` output (fallback for busybox images).
pub fn parse_stat_output(dir: &str, output: &str) -> Vec<ContainerFileEntry> {
    output
        .lines()
        .filter_map(|line| {
            let mut it = line.splitn(6, '|');
            let perms = it.next()?;
            let owner = it.next()?;
            let group = it.next()?;
            let size = it.next()?.parse().ok();
            let mtime = it.next()?.parse().ok();
            let name = it.next()?;
            if perms.len() < 10 {
                return None;
            }
            let size = if kind_of(perms) == "other" {
                None
            } else {
                size
            };
            make_entry(dir, perms, owner, group, size, mtime, name)
        })
        .collect()
}

fn sort_entries(entries: &mut Vec<ContainerFileEntry>) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries.truncate(MAX_LISTED_ENTRIES);
}

async fn list_dir(
    state: &AppState,
    container: &str,
    dir: &str,
) -> Result<Vec<ContainerFileEntry>, ApiError> {
    let gnu = state
        .runtime
        .run_command(
            container,
            vec![
                "ls".into(),
                "-la".into(),
                "--time-style=+%s".into(),
                "--".into(),
                dir.into(),
            ],
        )
        .await
        .map_err(|e| ApiError::internal(format!("Could not run ls in the container: {}", e)))?;
    if gnu.exit_code == 0 {
        let mut entries = parse_ls_output(dir, &gnu.stdout);
        sort_entries(&mut entries);
        return Ok(entries);
    }

    // busybox ls has no --time-style; use stat through sh with the path as $1 so it
    // is never parsed as shell code.
    let script =
        "cd -- \"$1\" || exit 3; stat -c '%A|%U|%G|%s|%Y|%n' -- .[!.]* ..?* * 2>/dev/null; exit 0";
    let res = state
        .runtime
        .run_command(
            container,
            vec![
                "sh".into(),
                "-c".into(),
                script.into(),
                "sh".into(),
                dir.into(),
            ],
        )
        .await
        .map_err(|e| ApiError::internal(format!("Could not list the folder: {}", e)))?;
    if res.exit_code != 0 {
        let detail = if gnu.stderr.trim().is_empty() {
            res.stderr.trim().to_string()
        } else {
            gnu.stderr.trim().to_string()
        };
        return Err(ApiError::bad_request(format!(
            "Could not list {}: {}",
            dir,
            if detail.is_empty() {
                "not a folder or not readable".to_string()
            } else {
                detail
            }
        )));
    }
    let mut entries = parse_stat_output(dir, &res.stdout);
    sort_entries(&mut entries);
    Ok(entries)
}

// ---------------------------------------------------------------------------
// Tar helpers
// ---------------------------------------------------------------------------

/// Pull the single regular file out of a tar archive produced for `path`.
fn extract_single_file(tar_bytes: &[u8], limit: u64) -> Result<Vec<u8>, ApiError> {
    let mut archive = tar::Archive::new(tar_bytes);
    let mut entries = archive
        .entries()
        .map_err(|e| ApiError::internal(format!("Invalid archive: {}", e)))?;
    let mut entry = entries
        .next()
        .ok_or_else(|| ApiError::not_found("File not found"))?
        .map_err(|e| ApiError::internal(format!("Invalid archive: {}", e)))?;
    let kind = entry.header().entry_type();
    if kind.is_dir() {
        return Err(ApiError::bad_request(
            "Folders cannot be opened or downloaded",
        ));
    }
    if !kind.is_file() {
        return Err(ApiError::bad_request("Only regular files can be opened"));
    }
    let size = entry.header().size().unwrap_or(0);
    if size > limit {
        return Err(ApiError::bad_request(format!(
            "File is {} bytes; the limit is {} bytes",
            size, limit
        )));
    }
    let mut buf = Vec::with_capacity(size as usize);
    entry
        .read_to_end(&mut buf)
        .map_err(|e| ApiError::internal(format!("Could not read archive: {}", e)))?;
    Ok(buf)
}

/// Build a tar archive holding one file (or an empty folder when `data` is `None`).
fn single_entry_tar(name: &str, data: Option<&[u8]>) -> Result<Vec<u8>, ApiError> {
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    let mtime = chrono::Utc::now().timestamp().max(0) as u64;
    header.set_mtime(mtime);
    match data {
        Some(bytes) => {
            header.set_entry_type(tar::EntryType::Regular);
            header.set_mode(0o644);
            header.set_size(bytes.len() as u64);
            header.set_cksum();
            builder
                .append_data(&mut header, name, bytes)
                .map_err(|e| ApiError::internal(format!("Could not build archive: {}", e)))?;
        }
        None => {
            header.set_entry_type(tar::EntryType::Directory);
            header.set_mode(0o755);
            header.set_size(0);
            header.set_cksum();
            builder
                .append_data(&mut header, format!("{}/", name), std::io::empty())
                .map_err(|e| ApiError::internal(format!("Could not build archive: {}", e)))?;
        }
    }
    builder
        .into_inner()
        .map_err(|e| ApiError::internal(format!("Could not build archive: {}", e)))
}

// ---------------------------------------------------------------------------
// Container resolution
// ---------------------------------------------------------------------------

/// Which resource a request targets.
#[derive(Debug, Clone, Copy)]
enum Kind {
    App,
    Database,
    Service,
}

impl Kind {
    fn resource_type(self) -> &'static str {
        match self {
            Kind::App => "app",
            Kind::Database => "database",
            Kind::Service => "service",
        }
    }
}

/// Resolve the running container for a resource, checking access first.
async fn resolve_container(
    state: &Arc<AppState>,
    user: &User,
    kind: Kind,
    id: &str,
    container: Option<&str>,
) -> Result<String, ApiError> {
    match kind {
        Kind::App => {
            authz::authorize_app(state, user, id).await?;
            let deployment: Option<Deployment> = sqlx::query_as(
                "SELECT * FROM deployments WHERE app_id = ? AND status = 'running' \
                 ORDER BY started_at DESC LIMIT 1",
            )
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
            deployment
                .and_then(|d| d.container_id)
                .ok_or_else(|| ApiError::bad_request("The app has no running container"))
        }
        Kind::Database => {
            let db = authz::authorize_database(state, user, id).await?;
            match db.container_id {
                Some(cid) if db.status == "running" => Ok(cid),
                _ => Err(ApiError::bad_request("The database is not running")),
            }
        }
        Kind::Service => {
            let service = authz::authorize_service(state, user, id).await?;
            let key = container
                .filter(|c| !c.is_empty())
                .ok_or_else(|| ApiError::bad_request("Choose a container"))?;
            super::services::resolve_running_container(state, &service, key).await
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers (shared)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct FilesQuery {
    pub path: Option<String>,
    /// Service container name or ID (services only)
    pub container: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub path: String,
    pub entries: Vec<ContainerFileEntry>,
}

#[derive(Debug, Serialize)]
pub struct FileContentResponse {
    pub path: String,
    pub content: String,
    pub size: u64,
    /// True when the file is not valid UTF-8 text (content is then empty)
    pub binary: bool,
}

#[derive(Debug, Deserialize)]
pub struct WriteFileBody {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct MkdirBody {
    pub path: String,
}

async fn do_list(
    state: Arc<AppState>,
    user: User,
    kind: Kind,
    id: String,
    q: FilesQuery,
) -> Result<Json<ListResponse>, ApiError> {
    let path = sanitize_container_path(q.path.as_deref().unwrap_or("/"))?;
    let container = resolve_container(&state, &user, kind, &id, q.container.as_deref()).await?;
    let entries = list_dir(&state, &container, &path).await?;
    Ok(Json(ListResponse { path, entries }))
}

async fn do_read(
    state: Arc<AppState>,
    user: User,
    kind: Kind,
    id: String,
    q: FilesQuery,
) -> Result<Json<FileContentResponse>, ApiError> {
    let path = sanitize_container_path(
        q.path
            .as_deref()
            .ok_or_else(|| ApiError::bad_request("path is required"))?,
    )?;
    split_path(&path)?;
    let container = resolve_container(&state, &user, kind, &id, q.container.as_deref()).await?;
    let tar = state
        .runtime
        .copy_from_container(&container, &path, MAX_VIEW_BYTES + TAR_OVERHEAD)
        .await
        .map_err(|e| ApiError::bad_request(format!("Could not read {}: {}", path, e)))?;
    let bytes = extract_single_file(&tar, MAX_VIEW_BYTES)?;
    let size = bytes.len() as u64;
    let (content, binary) = match String::from_utf8(bytes) {
        Ok(s) if !s.contains('\0') => (s, false),
        _ => (String::new(), true),
    };
    Ok(Json(FileContentResponse {
        path,
        content,
        size,
        binary,
    }))
}

async fn do_download(
    state: Arc<AppState>,
    user: User,
    kind: Kind,
    id: String,
    q: FilesQuery,
) -> Result<Response, ApiError> {
    let path = sanitize_container_path(
        q.path
            .as_deref()
            .ok_or_else(|| ApiError::bad_request("path is required"))?,
    )?;
    let (_, name) = split_path(&path)?;
    let container = resolve_container(&state, &user, kind, &id, q.container.as_deref()).await?;
    let tar = state
        .runtime
        .copy_from_container(&container, &path, MAX_TRANSFER_BYTES + TAR_OVERHEAD)
        .await
        .map_err(|e| ApiError::bad_request(format!("Could not read {}: {}", path, e)))?;
    let bytes = extract_single_file(&tar, MAX_TRANSFER_BYTES)?;
    let safe_name: String = name
        .chars()
        .map(|c| if c == '"' || c.is_control() { '_' } else { c })
        .collect();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", safe_name),
        )
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(Body::from(bytes))
        .map_err(|e| ApiError::internal(e.to_string()))
}

#[allow(clippy::too_many_arguments)]
async fn do_write(
    state: Arc<AppState>,
    user: User,
    client_ip: ClientIp,
    kind: Kind,
    id: String,
    container_key: Option<String>,
    path: &str,
    data: &[u8],
) -> Result<(), ApiError> {
    if data.len() as u64 > MAX_TRANSFER_BYTES {
        return Err(ApiError::bad_request(format!(
            "File is larger than the {} byte limit",
            MAX_TRANSFER_BYTES
        )));
    }
    let path = sanitize_container_path(path)?;
    let (parent, name) = split_path(&path)?;
    let name = sanitize_file_name(&name)?;
    let container = resolve_container(&state, &user, kind, &id, container_key.as_deref()).await?;
    let tar = single_entry_tar(&name, Some(data))?;
    state
        .runtime
        .copy_to_container(&container, &parent, tar)
        .await
        .map_err(|e| ApiError::bad_request(format!("Could not write {}: {}", path, e)))?;
    audit_log(
        &state,
        "container_file.write",
        kind.resource_type(),
        Some(&id),
        Some(&path),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "bytes": data.len() })),
    )
    .await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Route handlers. Apps and databases address the resource's single container;
// services take a `container` query parameter.
// ---------------------------------------------------------------------------

macro_rules! file_routes {
    ($kind:expr, $list:ident, $read:ident, $download:ident, $write:ident, $upload:ident, $mkdir:ident, $delete:ident) => {
        pub async fn $list(
            State(state): State<Arc<AppState>>,
            user: User,
            Path(id): Path<String>,
            Query(q): Query<FilesQuery>,
        ) -> Result<Json<ListResponse>, ApiError> {
            do_list(state, user, $kind, id, q).await
        }

        pub async fn $read(
            State(state): State<Arc<AppState>>,
            user: User,
            Path(id): Path<String>,
            Query(q): Query<FilesQuery>,
        ) -> Result<Json<FileContentResponse>, ApiError> {
            do_read(state, user, $kind, id, q).await
        }

        pub async fn $download(
            State(state): State<Arc<AppState>>,
            user: User,
            Path(id): Path<String>,
            Query(q): Query<FilesQuery>,
        ) -> Result<Response, ApiError> {
            do_download(state, user, $kind, id, q).await
        }

        pub async fn $write(
            State(state): State<Arc<AppState>>,
            user: User,
            client_ip: ClientIp,
            Path(id): Path<String>,
            Query(q): Query<FilesQuery>,
            Json(body): Json<WriteFileBody>,
        ) -> Result<StatusCode, ApiError> {
            do_write(
                state,
                user,
                client_ip,
                $kind,
                id,
                q.container,
                &body.path,
                body.content.as_bytes(),
            )
            .await?;
            Ok(StatusCode::NO_CONTENT)
        }

        pub async fn $upload(
            State(state): State<Arc<AppState>>,
            user: User,
            client_ip: ClientIp,
            Path(id): Path<String>,
            Query(q): Query<FilesQuery>,
            multipart: Multipart,
        ) -> Result<StatusCode, ApiError> {
            do_upload(state, user, client_ip, $kind, id, q, multipart).await
        }

        pub async fn $mkdir(
            State(state): State<Arc<AppState>>,
            user: User,
            client_ip: ClientIp,
            Path(id): Path<String>,
            Query(q): Query<FilesQuery>,
            Json(body): Json<MkdirBody>,
        ) -> Result<StatusCode, ApiError> {
            do_mkdir(state, user, client_ip, $kind, id, q, &body.path).await
        }

        pub async fn $delete(
            State(state): State<Arc<AppState>>,
            user: User,
            client_ip: ClientIp,
            Path(id): Path<String>,
            Query(q): Query<FilesQuery>,
        ) -> Result<StatusCode, ApiError> {
            do_delete(state, user, client_ip, $kind, id, q).await
        }
    };
}

async fn do_upload(
    state: Arc<AppState>,
    user: User,
    client_ip: ClientIp,
    kind: Kind,
    id: String,
    q: FilesQuery,
    mut multipart: Multipart,
) -> Result<StatusCode, ApiError> {
    let dir = sanitize_container_path(q.path.as_deref().unwrap_or("/"))?;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Invalid upload: {}", e)))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let name = sanitize_file_name(field.file_name().unwrap_or_default())?;
        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::bad_request(format!("Upload failed: {}", e)))?;
        let path = join_path(&dir, &name);
        do_write(
            state.clone(),
            user.clone(),
            client_ip.clone(),
            kind,
            id.clone(),
            q.container.clone(),
            &path,
            &data,
        )
        .await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    Err(ApiError::bad_request("No file in the upload"))
}

async fn do_mkdir(
    state: Arc<AppState>,
    user: User,
    client_ip: ClientIp,
    kind: Kind,
    id: String,
    q: FilesQuery,
    path: &str,
) -> Result<StatusCode, ApiError> {
    let path = sanitize_container_path(path)?;
    let (parent, name) = split_path(&path)?;
    let name = sanitize_file_name(&name)?;
    let container = resolve_container(&state, &user, kind, &id, q.container.as_deref()).await?;
    let tar = single_entry_tar(&name, None)?;
    state
        .runtime
        .copy_to_container(&container, &parent, tar)
        .await
        .map_err(|e| ApiError::bad_request(format!("Could not create {}: {}", path, e)))?;
    audit_log(
        &state,
        "container_file.mkdir",
        kind.resource_type(),
        Some(&id),
        Some(&path),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn do_delete(
    state: Arc<AppState>,
    user: User,
    client_ip: ClientIp,
    kind: Kind,
    id: String,
    q: FilesQuery,
) -> Result<StatusCode, ApiError> {
    let path = sanitize_container_path(
        q.path
            .as_deref()
            .ok_or_else(|| ApiError::bad_request("path is required"))?,
    )?;
    if path == "/" {
        return Err(ApiError::bad_request("The root folder cannot be deleted"));
    }
    let container = resolve_container(&state, &user, kind, &id, q.container.as_deref()).await?;
    let res = state
        .runtime
        .run_command(
            &container,
            vec!["rm".into(), "-rf".into(), "--".into(), path.clone()],
        )
        .await
        .map_err(|e| ApiError::internal(format!("Could not delete {}: {}", path, e)))?;
    if res.exit_code != 0 {
        return Err(ApiError::bad_request(format!(
            "Could not delete {}: {}",
            path,
            res.stderr.trim()
        )));
    }
    audit_log(
        &state,
        "container_file.delete",
        kind.resource_type(),
        Some(&id),
        Some(&path),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

file_routes!(
    Kind::App,
    app_list,
    app_read,
    app_download,
    app_write,
    app_upload,
    app_mkdir,
    app_delete
);
file_routes!(
    Kind::Database,
    database_list,
    database_read,
    database_download,
    database_write,
    database_upload,
    database_mkdir,
    database_delete
);
file_routes!(
    Kind::Service,
    service_list,
    service_read,
    service_download,
    service_write,
    service_upload,
    service_mkdir,
    service_delete
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_resolves_dots_and_never_escapes_root() {
        assert_eq!(sanitize_container_path("/").unwrap(), "/");
        assert_eq!(sanitize_container_path("").unwrap(), "/");
        assert_eq!(
            sanitize_container_path("/app//src/./x").unwrap(),
            "/app/src/x"
        );
        assert_eq!(sanitize_container_path("/app/../etc").unwrap(), "/etc");
        assert_eq!(
            sanitize_container_path("/../../../etc/passwd").unwrap(),
            "/etc/passwd"
        );
        assert_eq!(sanitize_container_path("/a/b/..").unwrap(), "/a");
        assert_eq!(
            sanitize_container_path("/dir with spaces/f").unwrap(),
            "/dir with spaces/f"
        );
    }

    #[test]
    fn sanitize_rejects_relative_and_control_characters() {
        assert!(sanitize_container_path("etc/passwd").is_err());
        assert!(sanitize_container_path("../etc").is_err());
        assert!(sanitize_container_path("/tmp/a\0b").is_err());
        assert!(sanitize_container_path("/tmp/a\nb").is_err());
        assert!(sanitize_container_path(&format!("/{}", "a".repeat(5000))).is_err());
    }

    #[test]
    fn file_names_cannot_contain_separators_or_dots() {
        assert_eq!(sanitize_file_name("notes.txt").unwrap(), "notes.txt");
        assert!(sanitize_file_name("../x").is_err());
        assert!(sanitize_file_name("a/b").is_err());
        assert!(sanitize_file_name("a\\b").is_err());
        assert!(sanitize_file_name("..").is_err());
        assert!(sanitize_file_name(".").is_err());
        assert!(sanitize_file_name("").is_err());
    }

    #[test]
    fn split_path_returns_parent_and_name() {
        assert_eq!(
            split_path("/etc/hosts").unwrap(),
            ("/etc".to_string(), "hosts".to_string())
        );
        assert_eq!(
            split_path("/hosts").unwrap(),
            ("/".to_string(), "hosts".to_string())
        );
        assert!(split_path("/").is_err());
    }

    #[test]
    fn parses_gnu_ls_output() {
        let out = "total 16\n\
drwxr-xr-x 1 root root 4096 1700000000 .\n\
drwxr-xr-x 1 root root 4096 1700000000 ..\n\
-rw-r--r-- 1 node staff  220 1700000100 my file.txt\n\
lrwxrwxrwx 1 root root    7 1700000200 bin -> usr/bin\n\
crw-rw-rw- 1 root root 1, 3 1700000300 null\n\
drwxr-xr-x 2 root root 4096 1700000400 src\n";
        let entries = parse_ls_output("/app", out);
        assert_eq!(entries.len(), 4);
        let f = &entries[0];
        assert_eq!(f.name, "my file.txt");
        assert_eq!(f.path, "/app/my file.txt");
        assert_eq!(f.size, Some(220));
        assert_eq!(f.modified, Some(1700000100));
        assert_eq!((f.owner.as_str(), f.group.as_str()), ("node", "staff"));
        let l = &entries[1];
        assert_eq!(l.kind, "symlink");
        assert_eq!(l.name, "bin");
        assert_eq!(l.link_target.as_deref(), Some("usr/bin"));
        let d = &entries[2];
        assert_eq!(d.kind, "other");
        assert_eq!(d.size, None);
        assert_eq!(d.modified, Some(1700000300));
        assert!(entries[3].is_dir);
    }

    #[test]
    fn parses_stat_output() {
        let out =
            "drwxr-xr-x|root|root|4096|1700000000|etc\n-rw-r--r--|app|app|12|1700000001|a|b.txt\n";
        let mut entries = parse_stat_output("/", out);
        sort_entries(&mut entries);
        assert_eq!(entries[0].name, "etc");
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "a|b.txt");
        assert_eq!(entries[1].path, "/a|b.txt");
        assert_eq!(entries[1].size, Some(12));
    }

    #[test]
    fn tar_round_trip_and_limits() {
        let tar = single_entry_tar("hello.txt", Some(b"hi there")).unwrap();
        assert_eq!(extract_single_file(&tar, 100).unwrap(), b"hi there");
        assert!(extract_single_file(&tar, 3).is_err());
        let dir = single_entry_tar("newdir", None).unwrap();
        assert!(extract_single_file(&dir, 100).is_err());
    }
}
