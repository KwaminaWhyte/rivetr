---
name: rust-review
description: Rust idioms and patterns used in the Rivetr codebase. Use when writing new Rust code, reviewing patterns, or understanding existing implementations.
allowed-tools: Read, Grep, Glob
---

# Rust Patterns for Rivetr

## Error Handling

Use `anyhow::Result` for application errors and `thiserror` for library errors:

```rust
use anyhow::{Context, Result};

async fn deploy_app(app: &App) -> Result<()> {
    clone_repo(&app.git_url)
        .await
        .context("Failed to clone repository")?;
    Ok(())
}
```

### Custom Errors with thiserror

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Unauthorized")]
    Unauthorized,
}
```

## Async Patterns

All I/O operations are async using Tokio:

```rust
use tokio::sync::mpsc;

// Channel for job queue
let (tx, rx) = mpsc::channel::<DeploymentJob>(100);

// Spawn background task
tokio::spawn(async move {
    while let Some(job) = rx.recv().await {
        process_job(job).await;
    }
});
```

### Blocking Code in Async Context

```rust
// DON'T: blocking in async
let result = std::fs::read_to_string(path)?;

// DO: use spawn_blocking
let result = tokio::task::spawn_blocking(move || {
    std::fs::read_to_string(path)
}).await??;

// OR: use async versions
let result = tokio::fs::read_to_string(path).await?;
```

### Timeout Patterns

```rust
use tokio::time::{timeout, Duration};

// With timeout
let result = timeout(Duration::from_secs(30), async_operation())
    .await
    .context("Operation timed out")??;
```

## Trait Objects for Runtime Polymorphism

Container runtime uses trait objects:

```rust
pub async fn detect_runtime(config: &RuntimeConfig) -> Result<Arc<dyn ContainerRuntime>> {
    match config.runtime_type {
        RuntimeType::Docker => Ok(Arc::new(DockerRuntime::new()?)),
        RuntimeType::Podman => Ok(Arc::new(PodmanRuntime::new())),
        RuntimeType::Auto => { /* auto-detect */ }
    }
}
```

### The ContainerRuntime Trait

```rust
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    async fn build(&self, ctx: &BuildContext) -> Result<String>;
    async fn run(&self, config: &RunConfig) -> Result<String>;
    async fn stop(&self, container_id: &str) -> Result<()>;
    async fn remove(&self, container_id: &str) -> Result<()>;
    async fn logs(&self, container_id: &str) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>>;
    async fn logs_stream(&self, container_id: &str) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>>;
    async fn inspect(&self, container_id: &str) -> Result<ContainerInfo>;
    async fn stats(&self, container_id: &str) -> Result<ContainerStats>;
    async fn is_available(&self) -> bool;
}
```

## State Management

Use `Arc` for shared state, `ArcSwap` for atomic updates:

```rust
use arc_swap::ArcSwap;

struct RouteTable {
    routes: Arc<ArcSwap<HashMap<String, Backend>>>,
}

impl RouteTable {
    fn update(&self, domain: String, backend: Backend) {
        self.routes.rcu(|current| {
            let mut new = (**current).clone();
            new.insert(domain, backend);
            new
        });
    }

    fn get(&self, domain: &str) -> Option<Backend> {
        self.routes.load().get(domain).cloned()
    }
}
```

### AppState Pattern

```rust
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
    pub runtime: Arc<dyn ContainerRuntime>,
    pub deploy_tx: mpsc::Sender<DeploymentJob>,
    pub route_table: Arc<RouteTable>,
}

// Clone is cheap (all Arc)
impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            db: self.db.clone(),
            runtime: Arc::clone(&self.runtime),
            deploy_tx: self.deploy_tx.clone(),
            route_table: Arc::clone(&self.route_table),
        }
    }
}
```

## Axum Patterns

### Extractor Order

Order matters - State before Path before Json:

```rust
pub async fn get_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<App>, StatusCode> {
    // ...
}

pub async fn create_app(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAppRequest>,
) -> Result<(StatusCode, Json<App>), ApiError> {
    // ...
}
```

### Custom Extractors

```rust
pub struct AuthUser(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract and validate token
        let token = parts.headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or(ApiError::Unauthorized)?;

        // Validate token and return user
        Ok(AuthUser(validate_token(token).await?))
    }
}
```

### Response Patterns

```rust
// Return with status code
async fn create_resource() -> (StatusCode, Json<Resource>) {
    (StatusCode::CREATED, Json(resource))
}

// Return with headers
async fn download() -> ([(HeaderName, HeaderValue); 2], Vec<u8>) {
    ([
        (header::CONTENT_TYPE, "application/octet-stream".parse().unwrap()),
        (header::CONTENT_DISPOSITION, "attachment; filename=\"file.zip\"".parse().unwrap()),
    ], data)
}
```

## Database Queries

Use sqlx with parameterized queries:

```rust
// Fetch optional
let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("App not found".into()))?;

// Fetch all
let apps = sqlx::query_as::<_, App>("SELECT * FROM apps ORDER BY created_at DESC")
    .fetch_all(&state.db)
    .await?;

// Insert
sqlx::query("INSERT INTO apps (id, name, git_url) VALUES (?, ?, ?)")
    .bind(&id)
    .bind(&name)
    .bind(&git_url)
    .execute(&state.db)
    .await?;

// Transaction
let mut tx = state.db.begin().await?;
sqlx::query("INSERT INTO apps ...").execute(&mut *tx).await?;
sqlx::query("INSERT INTO env_vars ...").execute(&mut *tx).await?;
tx.commit().await?;
```

## Streaming Patterns

### Log Streaming

```rust
use futures::Stream;
use tokio_stream::StreamExt;

pub async fn stream_logs(
    container_id: &str,
) -> Result<impl Stream<Item = LogLine>> {
    let stream = runtime.logs_stream(container_id).await?;
    Ok(stream.map(|line| {
        // Transform log line
        line
    }))
}
```

### WebSocket Streaming

```rust
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Handle message
            }
            _ => break,
        }
    }
}
```

## Security Patterns

### Constant-Time Comparison

```rust
use subtle::ConstantTimeEq;

fn verify_token(provided: &str, expected: &str) -> bool {
    provided.as_bytes().ct_eq(expected.as_bytes()).into()
}
```

### Input Validation

```rust
pub fn validate_app_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::new("name", "Name is required"));
    }
    if name.len() > 63 {
        return Err(ValidationError::new("name", "Name too long (max 63 chars)"));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(ValidationError::new("name", "Name can only contain alphanumeric characters and hyphens"));
    }
    Ok(())
}
```
