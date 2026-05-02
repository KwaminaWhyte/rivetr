# Rivetr - Technology Stack

## Core Principles

1. **Stability over novelty**: Use battle-tested crates
2. **Minimal dependencies**: Fewer deps = smaller binary, fewer CVEs
3. **Async-first**: Everything runs on Tokio
4. **Zero runtime overhead**: No GC, no interpreters

## Crate Selection

### Web Framework

| Crate | Version | Purpose |
|-------|---------|---------|
| **axum** | 0.7+ | HTTP server, routing, middleware |
| tokio | 1.x | Async runtime |
| tower | 0.4+ | Middleware/service abstractions |
| tower-http | 0.5+ | CORS, compression, tracing |

**Why Axum?**
- Built by Tokio team
- Excellent ergonomics with extractors
- Native WebSocket support
- Battle-tested in production

### Database

| Crate | Version | Purpose |
|-------|---------|---------|
| **sqlx** | 0.7+ | Async database driver |
| sqlite | (bundled) | Embedded database |

**Why SQLx + SQLite?**
- Compile-time SQL verification
- WAL mode for concurrent reads
- Zero external dependencies
- Single-file state (`rivetr.db`)

**Configuration:**
```rust
// Enable WAL mode for performance
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
```

### Container Runtime

| Crate | Version | Purpose |
|-------|---------|---------|
| **bollard** | 0.15+ | Docker Engine API client |
| tokio-process | (in tokio) | Podman CLI wrapper |

**Why Bollard?**
- Async Docker API access via socket
- Stream build output in real-time
- Granular error handling
- No shell command parsing

**Podman Fallback:**
```rust
// When Docker unavailable, use Podman CLI
Command::new("podman")
    .args(["build", "-t", &tag, "."])
    .spawn()
```

### Git Operations

| Crate | Version | Purpose |
|-------|---------|---------|
| **git2** | 0.18+ | libgit2 bindings |

**Why git2?**
- Much faster than CLI git
- In-process cloning
- Progress callbacks for UI
- SSH key handling built-in

### Reverse Proxy

| Crate | Version | Purpose |
|-------|---------|---------|
| **hyper** | 1.x | HTTP client/server |
| **hyper-util** | 0.1+ | Utilities for hyper |
| **rustls** | 0.22+ | TLS implementation |
| **rcgen** | 0.12+ | Certificate generation |
| **instant-acme** | 0.4+ | ACME/Let's Encrypt client |

**Alternative: Pingora**
- More features (load balancing, caching)
- Higher complexity
- Consider for Phase 2

**Why start with Hyper?**
- Already a dependency of Axum
- Full control over proxy behavior
- Simpler initial implementation

### State Management

| Crate | Version | Purpose |
|-------|---------|---------|
| **arc-swap** | 1.x | Atomic route table updates |
| **dashmap** | 5.x | Concurrent HashMap |
| **parking_lot** | 0.12+ | Faster mutexes |

### Templating (UI)

| Crate | Version | Purpose |
|-------|---------|---------|
| **askama** | 0.12+ | Type-safe HTML templates |
| **askama_axum** | 0.4+ | Axum integration |

**Why Askama?**
- Compile-time template checking
- Microsecond render times
- No runtime template parsing
- Rust type integration

### Frontend

| Technology | Purpose |
|------------|---------|
| **HTMX** | Dynamic UI without JS framework |
| **Alpine.js** | Minimal reactivity (optional) |
| **Tailwind CSS** | Styling (or simple CSS) |

**Why HTMX?**
- No build step required
- Server-driven UI updates
- Works perfectly with Askama
- ~14KB gzipped

### Serialization

| Crate | Version | Purpose |
|-------|---------|---------|
| **serde** | 1.x | Serialization framework |
| **serde_json** | 1.x | JSON handling |
| **toml** | 0.8+ | Config file parsing |

### Logging & Observability

| Crate | Version | Purpose |
|-------|---------|---------|
| **tracing** | 0.1+ | Structured logging |
| **tracing-subscriber** | 0.3+ | Log output formatting |

### CLI

| Crate | Version | Purpose |
|-------|---------|---------|
| **clap** | 4.x | Command-line parsing |

### Utilities

| Crate | Version | Purpose |
|-------|---------|---------|
| **uuid** | 1.x | UUID generation |
| **chrono** | 0.4+ | Date/time handling |
| **thiserror** | 1.x | Error definitions |
| **anyhow** | 1.x | Error handling |
| **reqwest** | 0.11+ | HTTP client (health checks) |
| **futures** | 0.3+ | Async utilities |
| **bytes** | 1.x | Byte buffer handling |

## Cargo.toml Structure

```toml
[package]
name = "rivetr"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[dependencies]
# Web
axum = { version = "0.7", features = ["ws", "macros"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "compression-gzip", "trace"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "migrate"] }

# Docker
bollard = "0.15"

# Git
git2 = "0.18"

# Proxy
hyper = { version = "1", features = ["full"] }
hyper-util = { version = "0.1", features = ["full"] }
rustls = "0.22"
instant-acme = "0.4"

# State
arc-swap = "1"
dashmap = "5"
parking_lot = "0.12"

# Templates
askama = "0.12"
askama_axum = "0.4"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# CLI
clap = { version = "4", features = ["derive"] }

# Utils
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
anyhow = "1"
reqwest = { version = "0.11", features = ["json"] }
futures = "0.3"
bytes = "1"

[dev-dependencies]
tokio-test = "0.4"
```

## Build Optimization

### Release Profile
```toml
[profile.release]
lto = "thin"
codegen-units = 1
panic = "abort"
strip = true
```

### Expected Binary Size
- Debug: ~50-80MB
- Release: ~15-25MB
- Release + strip: ~10-15MB

## Runtime Requirements

### Minimum
- Linux (x86_64 or aarch64)
- 512MB RAM
- Docker OR Podman installed
- Port 80/443 available

### Recommended
- 1GB+ RAM
- SSD storage
- Docker Engine 24+

## Security Crates (Future)

| Crate | Purpose |
|-------|---------|
| argon2 | Password hashing |
| jsonwebtoken | JWT tokens |
| secrecy | Secret value handling |
