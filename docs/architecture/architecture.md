# Rivetr Architecture

Rivetr is one Rust binary that holds the REST API, the dashboard (embedded with `rust-embed`), the deployment engine, the reverse proxy and an embedded SQLite database. It talks to Docker (socket API) or Podman (CLI) on the host and, over SSH, on remote servers.

## High-Level Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         RIVETR BINARY                           │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │   Axum API   │  │  React UI    │  │  Embedded Proxy      │   │
│  │ (REST, WS)   │  │ (embedded)   │  │  (Hyper + rustls)    │   │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘   │
│         └────────────┬────┴─────────────────────┘               │
│              ┌───────▼───────┐                                  │
│              │  Core Engine  │                                  │
│              │  (Tokio)      │                                  │
│              └───────┬───────┘                                  │
│    ┌─────────────────┼─────────────────┐                        │
│    ▼                 ▼                 ▼                        │
│ ┌──────┐      ┌───────────┐     ┌───────────┐                   │
│ │SQLite│      │  Bollard  │     │  Podman   │                   │
│ │(WAL) │      │  (Docker) │     │  (CLI)    │                   │
│ └──────┘      └───────────┘     └───────────┘                   │
└─────────────────────────────────────────────────────────────────┘
```

### Data flow

```
Webhook / dashboard / API -> API -> Engine (MPSC queue) -> Runtime (Docker/Podman) -> Proxy route update
                                        |
                                     SQLite (state)
```

## Components

### API layer (`src/api/`)

- Axum REST API for apps, databases, services, servers, teams and settings; WebSocket and SSE for logs and terminals.
- Git webhooks for GitHub, GitLab, Gitea, Bitbucket and DockerHub, with signature verification.
- Auth: session tokens, `rvt_` API tokens, the admin token from `rivetr.toml`, OAuth, OIDC SSO, TOTP 2FA and passkeys.
- `resource_authz_middleware` (`src/api/authz.rs`) enforces team ownership on every `/:id` resource route.
- The built dashboard is served from the binary with SPA fallback.

### Deployment engine (`src/engine/`)

- App pipeline (`engine/pipeline/`): clone (git CLI) -> build -> start on a private port -> health check -> atomic proxy switch, with health-based auto rollback.
- Jobs arrive on a Tokio MPSC channel (no Redis). A semaphore caps concurrent builds; the limit is adjustable at runtime from instance settings.
- Builders: Dockerfile, Nixpacks, Railpack, Cloud Native Buildpacks (Heroku/Paketo), static sites, Docker Compose, registry images. Builds can run on a remote build server over SSH.
- Compose services deploy through `engine/service_deploy/` (build, pre/post hooks, image digests, rollback), tracked in `service_deployments`. See [service-deployments.md](../reference/service-deployments.md).
- Background loops (container monitor, scheduler, backups, stats, cleanup, disk monitor, updater, server ops) are wrapped in `utils::supervise::guarded()` so a panic restarts the loop instead of the process.
- Deployment state lives in the `deployments` table.

### Container runtime (`src/runtime/`)

- The `ContainerRuntime` trait in `src/runtime/mod.rs` is the only way the engine touches containers. Implement it for a new runtime.
- `DockerRuntime` (`runtime/docker/`) uses the Bollard crate over the Docker socket.
- `PodmanRuntime` (`runtime/podman.rs`) wraps the Podman CLI.
- `detect_runtime()` picks one at startup from `[runtime]` config (`auto`, `docker`, `podman`).
- Every managed container joins the `rivetr` user-defined network so containers resolve each other by name.

### Reverse proxy (`src/proxy/`)

- Hyper-based HTTP/HTTPS proxy. The `RouteTable` (`proxy/mod.rs`) sits behind `ArcSwap` for lock-free route swaps without restarts.
- HTTPS listener speaks HTTP/2 and HTTP/1.1 (ALPN); the plain listener also accepts h2c. Backends get HTTP/1.1.
- Per-domain certificates through a rustls SNI resolver (`proxy/certs/store.rs`): custom uploads, then ACME per-domain certificates, then the legacy shared certificate. `proxy/certs/manager.rs` orders and renews them one domain at a time (HTTP-01, or DNS-01 through `proxy/certs/dns.rs` for wildcards) with per-domain backoff; the ACME client is `proxy/acme.rs`.
- WebSocket tunnelling, basic auth, health-aware routing, access logs and traffic analytics.

### Database (`src/db/`)

- SQLite in WAL mode; one file at `<data_dir>/rivetr.db`.
- Migrations are numbered SQL files in `migrations/`, registered at the end of `run_migrations` in `src/db/mod.rs`. They must be idempotent.
- Models in `src/db/models/`, service template seeders in `src/db/seeders/`.

### Shared state

- `AppState` in `src/lib.rs` holds the config, DB pool, deploy channel, runtime, route table, rate limiter, AI client and deploy semaphore.
- Concurrent maps use `dashmap`; locks use `parking_lot` (no mutex poisoning).
- Instance-level toggles live in the `instance_settings` table; a DB value overrides the matching `rivetr.toml` value.

### Frontend (`frontend/`)

- React 19, React Router 8 (framework mode with SSR loaders), TypeScript, Vite, Tailwind CSS 4, shadcn/ui, TanStack Query.
- `frontend/app/routes/` pages, `components/` UI, `lib/api/` API client (re-exported from `lib/api/index.ts`), `types/` shared types.
- Build output in `static/dist/`, embedded into the binary at compile time.

## Source layout

| Directory | Responsibility |
|---|---|
| `src/api/` | Axum route handlers, grouped by resource |
| `src/engine/` | Deployment pipeline, service deploys, monitors, scheduler, builders, server ops |
| `src/runtime/` | Container runtime abstraction (Docker, Podman) |
| `src/proxy/` | Embedded reverse proxy, ACME/TLS, access logs |
| `src/db/` | SQLite pool, migrations runner, models, template seeders |
| `src/settings/` | Instance settings (advanced, DNS, email, network, updates) |
| `src/backup/` | Instance backup and S3-compatible storage |
| `src/logging/` | Log draining to external providers |
| `src/monitoring/` | Uptime tracking, log cleanup |
| `src/notifications/` | Alert channels (Slack, Discord, email, Telegram, Teams, ntfy and more) |
| `src/mcp/` | MCP server for AI assistants |
| `src/github/` | GitHub App API client and installation tokens |
| `src/crypto/` | AES-256-GCM encryption and key file handling |
| `src/cli/` | CLI subcommands (deploy, backup, database, server, secrets) |
| `src/tui/` | Terminal UI (`--features tui`) |
| `src/config/` | `rivetr.toml` parsing |
| `src/startup/` | Startup self-checks |
| `src/ui/` | Email templates |
| `src/utils/` | SSH, privileged command helper, task supervision |
| `migrations/` | Numbered SQL migrations |
| `frontend/app/` | Dashboard source |

## Tech stack

Principles: battle-tested crates, few dependencies, async everywhere on Tokio, no runtime services outside the binary. `Cargo.toml` and `frontend/package.json` are the source of truth for versions.

| Area | Choice | Why |
|---|---|---|
| Web | axum, tower-http | Tokio-native, extractors, built-in WebSocket |
| Database | sqlx + SQLite (WAL) | No external database, single-file state |
| Containers | bollard (Docker), Podman CLI | Async Docker API with streamed build output; Podman needs no daemon |
| Proxy | hyper, rustls, own ACME client | Full control over proxying, already an axum dependency |
| State | arc-swap, dashmap, parking_lot | Lock-free route swaps, concurrent maps |
| Crypto and auth | aes-gcm, argon2, webauthn-rs | Secrets at rest, password hashing, passkeys |
| CLI and TUI | clap, ratatui | |
| Frontend assets | rust-embed | Dashboard ships inside the binary |

The release profile uses thin LTO, one codegen unit and `panic = "unwind"` so a panic kills only the task that raised it.

## Configuration

Server config is `rivetr.toml`; every key is documented in [configuration.md](../reference/configuration.md), with a copyable starter in `rivetr.example.toml`. Per-app settings (build type, health check, resources, domains) are stored in the database and edited from the dashboard or API.

## Security model

- Apps run in containers with default memory, PID and OOM limits; builds run with CPU and memory limits.
- Apps are reachable only through the proxy unless a port is published explicitly.
- Env vars, SSH keys and provider secrets are encrypted at rest with AES-256-GCM.
- Team-scoped authorization on every resource route; SSRF guard on outbound requests; SSH host keys pinned on first use.
