---
name: rust-patterns
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
}
```

## Axum Extractors

Order matters - State before Path before Json:

```rust
pub async fn get_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<App>, StatusCode> {
    // ...
}
```

## Database Queries

Use sqlx with compile-time checked queries:

```rust
let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;
```
