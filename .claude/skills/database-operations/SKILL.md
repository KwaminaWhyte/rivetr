---
name: database-operations
description: SQLite database patterns and SQLx usage in Rivetr. Use when writing database queries, debugging schema issues, or understanding data models.
allowed-tools: Read, Grep, Glob, Bash
---

# Rivetr Database Operations

## Overview

Rivetr uses SQLite with SQLx for all persistent storage:
- Single file: `data/rivetr.db`
- WAL mode for concurrent access
- Compile-time query verification (when enabled)

## Connection Setup

```rust
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

// From src/db/mod.rs
pub async fn init_db(data_dir: &Path) -> Result<SqlitePool> {
    let db_path = data_dir.join("rivetr.db");
    let url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;

    // Enable WAL mode
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;

    Ok(pool)
}
```

## Query Patterns

### Fetch One (Required)
```rust
let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
    .bind(&id)
    .fetch_one(&pool)
    .await?;  // Errors if not found
```

### Fetch Optional
```rust
let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
    .bind(&id)
    .fetch_optional(&pool)
    .await?;  // Returns Option<App>
```

### Fetch All
```rust
let apps = sqlx::query_as::<_, App>("SELECT * FROM apps ORDER BY created_at DESC")
    .fetch_all(&pool)
    .await?;  // Returns Vec<App>
```

### Insert
```rust
sqlx::query(
    "INSERT INTO apps (id, name, git_url, branch, port, created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?)"
)
    .bind(&app.id)
    .bind(&app.name)
    .bind(&app.git_url)
    .bind(&app.branch)
    .bind(app.port)
    .bind(&app.created_at)
    .bind(&app.updated_at)
    .execute(&pool)
    .await?;
```

### Update
```rust
sqlx::query("UPDATE apps SET name = ?, updated_at = ? WHERE id = ?")
    .bind(&name)
    .bind(Utc::now())
    .bind(&id)
    .execute(&pool)
    .await?;
```

### Delete
```rust
sqlx::query("DELETE FROM apps WHERE id = ?")
    .bind(&id)
    .execute(&pool)
    .await?;
```

## Model Pattern

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct App {
    pub id: String,
    pub name: String,
    pub git_url: String,
    pub branch: String,
    pub port: i64,
    pub domain: Option<String>,
    pub healthcheck: Option<String>,
    pub cpu_limit: Option<String>,
    pub memory_limit: Option<String>,
    pub environment: Option<String>,
    pub project_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl App {
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM apps WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }
}
```

## Migrations

Migrations live in `migrations/` directory:

```
migrations/
├── 001_initial.sql
├── 002_add_webhooks.sql
├── 003_add_env_vars.sql
...
```

### Run Migrations
```rust
// At startup in main.rs
sqlx::migrate!("./migrations")
    .run(&pool)
    .await?;
```

### Check Migration Status
```bash
sqlite3 data/rivetr.db "SELECT * FROM _sqlx_migrations"
```

## Core Tables

### apps
```sql
CREATE TABLE apps (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    git_url TEXT NOT NULL,
    branch TEXT NOT NULL DEFAULT 'main',
    port INTEGER NOT NULL DEFAULT 3000,
    domain TEXT,
    healthcheck TEXT,
    cpu_limit TEXT,
    memory_limit TEXT,
    environment TEXT DEFAULT 'development',
    project_id TEXT REFERENCES projects(id),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### deployments
```sql
CREATE TABLE deployments (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL REFERENCES apps(id),
    commit_sha TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    container_id TEXT,
    image_tag TEXT,
    error_message TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at DATETIME
);
```

### env_vars
```sql
CREATE TABLE env_vars (
    id TEXT PRIMARY KEY,
    app_id TEXT NOT NULL REFERENCES apps(id),
    key TEXT NOT NULL,
    value TEXT NOT NULL,      -- Encrypted with AES-256-GCM
    is_secret BOOLEAN DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(app_id, key)
);
```

### deployment_logs
```sql
CREATE TABLE deployment_logs (
    id TEXT PRIMARY KEY,
    deployment_id TEXT NOT NULL REFERENCES deployments(id),
    level TEXT NOT NULL DEFAULT 'info',
    message TEXT NOT NULL,
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

## Debugging Commands

```bash
# Open SQLite CLI
sqlite3 data/rivetr.db

# List all tables
.tables

# Show table schema
.schema apps

# Check WAL mode
PRAGMA journal_mode;

# Check foreign keys
PRAGMA foreign_keys;

# Recent deployments
SELECT id, app_id, status, created_at FROM deployments ORDER BY created_at DESC LIMIT 10;

# Failed deployments with errors
SELECT id, app_id, error_message, created_at FROM deployments WHERE status = 'failed' ORDER BY created_at DESC;

# Apps with their deployment counts
SELECT a.name, COUNT(d.id) as deploys FROM apps a LEFT JOIN deployments d ON a.id = d.app_id GROUP BY a.id;
```

## Common Issues

### "database is locked"
- Usually means WAL mode not enabled
- Check: `PRAGMA journal_mode;` should return "wal"
- Fix: Restart server, ensure single writer

### Schema Mismatch
- Migration didn't run or failed partway
- Check: `SELECT * FROM _sqlx_migrations`
- Fix: Delete `rivetr.db` for fresh start (dev only)

### Foreign Key Constraint Failed
- Trying to delete parent with children
- Use `ON DELETE CASCADE` or delete children first

### Type Mismatch in FromRow
- SQLite stores integers as i64
- Ensure Rust struct uses `i64` not `i32`
