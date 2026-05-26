# Rivetr - Architecture

## High-Level Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         RIVETR BINARY                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   Axum API   │  │  React UI    │  │  Embedded Proxy      │  │
│  │   (REST)     │  │  (Dashboard) │  │  (Hyper + rustls)    │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘  │
│         │                 │                      │              │
│         └────────────┬────┴──────────────────────┘              │
│                      │                                          │
│              ┌───────▼───────┐                                  │
│              │  Core Engine  │                                  │
│              │  (Tokio)      │                                  │
│              └───────┬───────┘                                  │
│                      │                                          │
│    ┌─────────────────┼─────────────────┐                       │
│    │                 │                 │                        │
│    ▼                 ▼                 ▼                        │
│ ┌──────┐      ┌───────────┐     ┌───────────┐                  │
│ │SQLite│      │  Bollard  │     │  Podman   │                  │
│ │(State)│     │  (Docker) │     │  (CLI)    │                  │
│ └──────┘      └───────────┘     └───────────┘                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │      Container Runtime        │
              │   (Docker / Podman / Both)    │
              └───────────────────────────────┘
```

## Component Breakdown

### 1. Web Layer

#### REST API (Axum)
- Handles webhook endpoints (GitHub, GitLab, Gitea)
- Provides management API for apps, deployments, logs
- Authentication via API tokens
- WebSocket support for real-time log streaming

#### Dashboard UI (React + shadcn/ui)
- Vite + React + TypeScript frontend
- shadcn/ui components with Tailwind CSS v4
- React Query for data fetching and caching
- React Router for SPA navigation
- Real-time updates via WebSocket
- Static files served by tower-http

### 2. Core Engine

#### Deployment Pipeline
```
Webhook Received
      │
      ▼
┌─────────────┐
│  Validate   │ → Check config, auth
└──────┬──────┘
       │
       ▼
┌─────────────┐
│    Clone    │ → git2 crate (not CLI)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│    Build    │ → Docker/Podman build
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Deploy    │ → Start container on private port
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Health    │ → HTTP health check
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Switch    │ → Atomic proxy route update
└─────────────┘
```

#### State Management
- **SQLite** in WAL mode for all persistent state
- Single file: `rivetr.db`
- No external database required
- Migrations handled at startup

#### Task Queue
- **Tokio MPSC channels** for job processing
- No Redis required
- In-memory with persistence checkpoints

### 3. Container Runtime Abstraction

```rust
trait ContainerRuntime {
    async fn build(&self, context: &BuildContext) -> Result<ImageId>;
    async fn run(&self, image: ImageId, config: &RunConfig) -> Result<ContainerId>;
    async fn stop(&self, container: ContainerId) -> Result<()>;
    async fn logs(&self, container: ContainerId) -> impl Stream<Item = LogLine>;
    async fn stats(&self, container: ContainerId) -> Result<Stats>;
}
```

- **DockerRuntime**: Uses Bollard crate (socket API)
- **PodmanRuntime**: Uses CLI wrapper (no daemon)
- Runtime auto-detected or configured

### 4. Embedded Reverse Proxy

#### Routing Table (ArcSwap)
```rust
struct RouteTable {
    routes: HashMap<Domain, Backend>,
}

struct Backend {
    container_id: String,
    internal_port: u16,
    health_status: HealthStatus,
}
```

#### Features
- Dynamic route updates (no restarts) via ArcSwap
- Automatic HTTPS via ACME (Let's Encrypt) with auto-renewal
- Health-aware routing with background health checker
- WebSocket proxying support
- Route management API (GET/POST/DELETE /api/routes)

### 5. Data Models

#### App
```rust
struct App {
    id: Uuid,
    name: String,
    git_url: String,
    branch: String,
    dockerfile: String,
    domain: Option<String>,
    port: u16,
    healthcheck: Option<String>,
    cpu_limit: Option<String>,      // e.g., "1", "0.5", "2"
    memory_limit: Option<String>,   // e.g., "512m", "1g"
    environment: AppEnvironment,    // development, staging, production
    project_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

#### EnvVar
```rust
struct EnvVar {
    id: Uuid,
    app_id: Uuid,
    key: String,
    value: String,          // Stored encrypted at rest
    is_secret: bool,        // UI masking indicator
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

#### Deployment
```rust
struct Deployment {
    id: Uuid,
    app_id: Uuid,
    commit_sha: String,
    status: DeploymentStatus,
    container_id: Option<String>,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
}

enum DeploymentStatus {
    Pending,
    Cloning,
    Building,
    Deploying,
    Running,
    Failed(String),
    Stopped,
}
```

## Directory Structure

```
rivetr/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI parsing
│   ├── lib.rs               # Library root
│   ├── config/              # Configuration parsing
│   │   ├── mod.rs
│   │   └── schema.rs
│   ├── api/                 # Axum routes
│   │   ├── mod.rs
│   │   ├── webhooks.rs      # Git webhook handlers
│   │   ├── apps.rs          # App CRUD
│   │   ├── deployments.rs   # Deployment management
│   │   └── auth.rs          # Authentication
│   ├── ui/                  # Reserved for email templates
│   │   └── mod.rs
│   ├── engine/              # Core deployment logic
│   │   ├── mod.rs
│   │   ├── pipeline.rs      # Deployment pipeline
│   │   ├── builder.rs       # Build orchestration
│   │   └── health.rs        # Health checking
│   ├── runtime/             # Container abstraction
│   │   ├── mod.rs
│   │   ├── docker.rs        # Bollard implementation
│   │   └── podman.rs        # Podman CLI wrapper
│   ├── proxy/               # Reverse proxy
│   │   ├── mod.rs
│   │   ├── router.rs        # Route management
│   │   └── tls.rs           # ACME/TLS handling
│   ├── db/                  # Database layer
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   └── migrations/
│   └── utils/               # Shared utilities
│       ├── mod.rs
│       └── git.rs           # Git operations
├── frontend/                # React + Vite + TypeScript dashboard
│   ├── src/
│   │   ├── components/      # React components (shadcn/ui)
│   │   ├── pages/           # Page components
│   │   ├── lib/             # API client, utilities
│   │   └── types/           # TypeScript types
│   └── vite.config.ts
├── static/dist/             # Built frontend assets (served by tower-http)
├── migrations/              # SQLx migrations
└── tests/
    ├── integration/
    └── e2e/
```

## Configuration File

### `rivetr.toml` (Global)

```toml
[server]
host = "0.0.0.0"
api_port = 8080
proxy_port = 80
proxy_https_port = 443
data_dir = "/var/lib/rivetr"

[auth]
admin_token = "your-secret-token"

[runtime]
# "docker", "podman", or "auto"
type = "auto"
docker_socket = "/var/run/docker.sock"

[proxy]
# ACME email for Let's Encrypt
acme_email = "admin@example.com"
acme_staging = false

[logging]
level = "info"
```

### `deploy.toml` (Per-App, in repo root)

```toml
app = "my-api"
port = 3000

[build]
dockerfile = "./Dockerfile"
# Optional build args
args = { NODE_ENV = "production" }

[deploy]
strategy = "rolling"  # or "blue-green"
healthcheck = "/health"
healthcheck_timeout = 30

[resources]
memory = "256mb"
cpu = "0.5"

[env]
# Non-secret env vars (secrets via UI/API)
LOG_LEVEL = "info"
```

## Security Considerations

1. **Container Isolation**: All apps run in containers with resource limits
2. **Network Isolation**: Apps only exposed through proxy
3. **Secret Management**: Env vars encrypted at rest in SQLite
4. **Auth**: Token-based authentication for API
5. **TLS**: Automatic HTTPS for all domains
6. **Build Isolation**: Builds run with CPU/memory limits

## Future Extensibility

### Phase 2+ Considerations
- **Multi-node**: Agent mode with controller-agent communication
- **Plugins**: Lua or WASM-based plugin system
- **Databases**: Managed database provisioning
- **Backups**: Automated backup scheduling
