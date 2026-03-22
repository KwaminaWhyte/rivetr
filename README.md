# Rivetr

Deploy applications from Git with a single binary — ~30MB RAM idle, no external dependencies.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Build](https://img.shields.io/github/actions/workflow/status/KwaminaWhyte/rivetr/ci.yml?branch=main)](https://github.com/KwaminaWhyte/rivetr/actions)

## What is Rivetr?

Rivetr is a self-hosted PaaS (Platform as a Service) that deploys applications from Git with minimal resource usage. It ships as a single binary with an embedded SQLite database, an embedded reverse proxy with automatic HTTPS, and a full-featured React dashboard — no Redis, no PostgreSQL, no Traefik required.

It supports Docker and Podman runtimes, handles webhooks from GitHub, GitLab, Gitea, and Bitbucket, and provides zero-downtime deployments with health checks and automatic rollback.

## Why Rivetr?

| | Rivetr | Coolify | Dokploy |
|---|---|---|---|
| **RAM idle** | ~30–80 MB | 400–800 MB | ~300 MB |
| **External dependencies** | None | PostgreSQL, Redis, Traefik | PostgreSQL |
| **Single binary** | Yes | No | No |
| **Container runtimes** | Docker + Podman | Docker | Docker |
| **Git providers** | GitHub, GitLab, Gitea, Bitbucket | GitHub, GitLab, Gitea, Bitbucket | GitHub, GitLab, Bitbucket |
| **Preview deployments** | Yes | Yes | No |
| **MCP server** | Yes | No | No |
| **Terminal UI (TUI)** | Yes | No | No |
| **Remote filesystem browser** | Yes | No | No |
| **License** | MIT | Apache 2.0 | MIT |

## Features

### Core Deployment

- Git deployments from GitHub, GitLab, Gitea, and Bitbucket with webhook signature verification
- Build types: Dockerfile, Nixpacks, Railpack, Heroku/Paketo buildpacks, static sites
- Docker and Podman runtime with automatic detection
- Zero-downtime deployments: clone → build → start → health check → atomic proxy switch
- Automatic rollback on health check failure
- Preview deployments for pull requests (unique subdomain per PR, auto-cleaned on close/merge)
- Deploy by specific commit SHA or git tag
- Container replicas with round-robin load balancing
- ZIP file upload as an alternative to Git
- Deployment queue cancellation: cancel any queued or running deployment mid-flight

### Platform Services

- Managed databases: PostgreSQL, MySQL, MongoDB, Redis, DragonFlyDB, KeyDB, ClickHouse (one-click provisioning)
- Docker Compose multi-container deployments with optional raw mode (skip Rivetr network injection)
- 273 pre-configured service templates across categories: AI/ML, Analytics, Automation, CMS, Communication, Dev Tools, Documentation, File/Media, Monitoring, Security, Search, Project Management, and more
- Scheduled jobs: cron-based command execution inside running containers
- Community template submissions: users can submit custom templates for admin review and promotion

### Team Collaboration

- Multi-tenant with full resource isolation between teams
- RBAC: owner / admin / developer / viewer roles with fine-grained per-resource permission overrides
- Team invitations via email with 7-day expiry
- Audit logging for all team operations
- App sharing between teams
- Per-team 2FA enforcement

### Security and Authentication

- Automatic HTTPS with Let's Encrypt and auto-renewal
- OAuth login: GitHub, Google
- SSO/OIDC: Auth0, Keycloak, Azure AD, Okta
- TOTP-based 2FA with recovery codes
- AES-256-GCM encryption for secrets and SSH keys at rest
- Named API tokens: create scoped tokens for CI/CD and scripts (prefixed `rvt_`, stored as SHA-256 hashes)
- Rate limiting on API, webhook, and auth endpoints
- Strict input validation and security headers

### CI/CD

- Deployment approval workflow: require sign-off before a deploy goes live
- Deployment freeze windows: block deployments during maintenance periods
- Scheduled deployments: trigger a deploy at a specific date and time
- DockerHub webhook: auto-deploy when a new image is pushed
- Watch paths: only trigger a deploy when specific file paths change in a push
- Container resource limits: set CPU and memory limits per app, apply live via `docker update` (no redeploy)

### Operations and Monitoring

- Real-time log streaming via WebSocket and SSE
- Full-text log search with configurable per-app retention policies
- Uptime tracking and response time monitoring via health check latency
- Container crash recovery with exponential backoff and configurable restart limits
- Scheduled container restarts (cron-based)
- Prometheus `/metrics` endpoint for external monitoring
- Webhook audit log
- Recharts-powered dashboard for CPU, memory, and deployment history

### Developer Experience

- Modern React + TypeScript dashboard using shadcn/ui components
- Browser-based terminal for running containers and remote servers (SSH)
- Remote filesystem browser: browse, read, write, and delete files on any registered server over SSH
- Terminal UI (`rivetr tui`): keyboard-driven dashboard for managing apps, deployments, and servers from any terminal
- Environment variables with AES-256-GCM encrypted storage
- Shared env vars with inheritance: team → project → environment → app
- Service dependency graph
- Config snapshots: save and restore full app configuration
- Export and import projects as JSON
- MCP server for AI assistant integration (Claude, Copilot, etc.)
- Named API tokens for programmatic access from scripts and CI systems

### Backup and Storage

- S3-compatible backup destinations: AWS S3, MinIO, Cloudflare R2, or any custom endpoint
- Volume and database backup to S3
- Scheduled backups with retention policies
- Full instance backup: SQLite database + config + SSL certificates as a single archive

### Multi-Server and Scale

- Multi-server support: register and deploy to remote servers via SSH
- Remote server browser-based terminal and filesystem browser
- Docker Swarm: init swarm, manage nodes, scale services
- Build servers: offload builds to dedicated remote nodes
- Container registry push after build
- Log draining: Axiom, New Relic, Datadog, Logtail, or any HTTP endpoint
- Ansible playbook (`ansible/rivetr.yml`) for automated server provisioning on Ubuntu/Debian

## Terminal UI (TUI)

Rivetr ships with an optional keyboard-driven terminal dashboard built with [ratatui](https://ratatui.rs/). It connects to any running Rivetr instance — local or remote — without opening a browser.

### Using the TUI from your local machine

**Prerequisites:** A running Rivetr instance (e.g. `https://rivetr.site`) and an API token from **Settings → API Tokens**.

**Option 1 — Download the pre-built binary (macOS/Linux):**

```bash
# Download the TUI-enabled binary for macOS (Apple Silicon)
curl -Lo rivetr https://github.com/KwaminaWhyte/rivetr/releases/latest/download/rivetr-macos-arm64
chmod +x rivetr

# Connect to your Rivetr instance
./rivetr tui --url https://rivetr.site --token rvt_your_token_here
```

**Option 2 — Build from source:**

```bash
git clone https://github.com/KwaminaWhyte/rivetr.git
cd rivetr
cargo build --release --features tui
./target/release/rivetr tui --url https://rivetr.site --token rvt_your_token_here
```

**Using environment variables (avoids typing the token every time):**

```bash
export RIVETR_URL=https://rivetr.site
export RIVETR_TOKEN=rvt_your_token_here
./rivetr tui
```

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `Tab` / `1–4` | Switch between Apps / Deployments / Servers / Logs tabs |
| `↑` / `↓` | Navigate list |
| `d` | Deploy selected app |
| `s` | Stop selected app |
| `r` | Restart selected app |
| `?` | Toggle help overlay |
| `q` / `Ctrl-C` | Quit |

Data refreshes automatically every 5 seconds. The status bar shows the connected instance URL and live connection state.

---

## Quick Start

### One-Line Install (Production)

```bash
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

The install script:
1. Installs Docker if not already present (Ubuntu, Debian, Fedora, CentOS, RHEL)
2. Downloads and installs the Rivetr binary from GitHub Releases
3. Installs build tools: Git, Nixpacks, Railpack, Pack CLI
4. Creates a `rivetr` system user with Docker access
5. Writes a config file at `/opt/rivetr/rivetr.toml` with a generated admin token
6. Registers a systemd service with auto-restart
7. Opens firewall ports 80, 443, and 8080

After installation:

```
Dashboard:      http://your-server-ip:8080
Config:         /opt/rivetr/rivetr.toml
Data:           /var/lib/rivetr
Logs:           sudo journalctl -u rivetr -f
```

Create your admin account on the first visit to the dashboard.

**Install a specific version:**

```bash
RIVETR_VERSION=v0.2.6 curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

### Service Management

```bash
sudo systemctl status rivetr
sudo systemctl restart rivetr
sudo journalctl -u rivetr -f
```

### Development Setup

**Prerequisites:** Rust 1.75+, Node.js 20+, Docker or Podman

```bash
git clone https://github.com/KwaminaWhyte/rivetr.git
cd rivetr

# Run the backend (auto-reloads on file changes)
cargo install cargo-watch   # once
cargo watch -x "run -- --config rivetr.example.toml"

# Run the frontend dev server (separate terminal)
cd frontend
npm install
npm run dev
```

The backend listens on `http://localhost:8080`. The frontend dev server proxies API calls to it.

**Manual build:**

```bash
cargo build --release
./target/release/rivetr --config rivetr.example.toml
```

## Configuration

### `rivetr.toml` — Server Configuration

```toml
[server]
host = "0.0.0.0"
api_port = 8080
proxy_port = 80
proxy_https_port = 443
data_dir = "/var/lib/rivetr"
# Required for GitHub App callbacks when behind a tunnel (ngrok, etc.)
# external_url = "https://your-tunnel.example.com"

[auth]
admin_token = "change-me-in-production"
# AES-256-GCM key for encrypting env vars at rest (optional, recommended)
# encryption_key = "your-32-char-minimum-key-here"

[runtime]
runtime_type = "auto"   # "docker", "podman", or "auto"
build_cpu_limit = "2"
build_memory_limit = "2g"

[proxy]
acme_enabled = true
acme_email = "admin@example.com"
acme_staging = false    # set true for testing (avoids Let's Encrypt rate limits)
acme_cache_dir = "./data/acme"
# base_domain = "apps.example.com"  # enables *.apps.example.com subdomains
sslip_enabled = true    # auto-generates sslip.io domains per app

[logging]
level = "info"   # trace | debug | info | warn | error

[webhooks]
# github_secret = "your-hmac-secret"
# gitlab_token  = "your-token"
# gitea_secret  = "your-hmac-secret"

[rate_limit]
enabled = true
api_requests_per_window = 100
webhook_requests_per_window = 500
auth_requests_per_window = 20
window_seconds = 60

[cleanup]
enabled = true
max_deployments_per_app = 10
cleanup_interval_seconds = 3600
prune_images = true

[container_monitor]
enabled = true
check_interval_secs = 30
max_restart_attempts = 5
initial_backoff_secs = 5
max_backoff_secs = 300
stable_duration_secs = 120

[email]
# enabled = true
# smtp_host = "smtp.example.com"
# smtp_port = 587
# smtp_username = "apikey"
# smtp_password = "your-smtp-password"
# from_address = "noreply@example.com"
```

See [`rivetr.example.toml`](rivetr.example.toml) for the full reference with all options documented.

### `deploy.toml` — Per-App Configuration

Place this file in your repository root:

```toml
app = "my-api"
port = 3000

[build]
dockerfile = "./Dockerfile"

[deploy]
healthcheck = "/health"
healthcheck_timeout = 30

[resources]
memory = "256mb"
cpu = "0.5"
```

## CLI Reference

Rivetr ships a full CLI that can control a running instance from any machine:

```bash
# Check instance health and version
rivetr status --api-url https://rivetr.site --token rvt_…

# List apps
rivetr apps list

# Trigger a deployment
rivetr deploy my-app

# Stream live logs
rivetr logs my-app --follow

# Launch the terminal dashboard
rivetr tui --url https://rivetr.site --token rvt_…

# Backup and restore
rivetr backup --output ./my-backup.tar.gz
rivetr restore ./my-backup.tar.gz
```

Environment variables `RIVETR_API_URL` and `RIVETR_TOKEN` are accepted for all subcommands.

The `tui` subcommand requires a build with `--features tui`. Pre-built binaries from GitHub Releases include it by default.

## Project Structure

```
rivetr/
├── src/
│   ├── main.rs              # Entry point and CLI
│   ├── lib.rs               # AppState and shared state
│   ├── api/                 # Axum REST API routes
│   │   ├── apps/            # App CRUD, control, sharing, ZIP upload
│   │   ├── deployments/     # Deployment handlers, rollback, approval, freeze
│   │   ├── webhooks/        # GitHub, GitLab, Gitea, Bitbucket handlers
│   │   ├── teams/           # Team CRUD, members, invitations, audit
│   │   ├── git_providers/   # OAuth connections to git providers
│   │   ├── services/        # Docker Compose and service templates
│   │   ├── system/          # Health, backup, auto-update
│   │   └── validation/      # Request validation layer
│   ├── engine/              # Deployment pipeline
│   │   ├── pipeline/        # Clone → build → start → rollback
│   │   ├── container_monitor/ # Crash detection and recovery
│   │   └── scheduler.rs     # Cron-based job scheduler
│   ├── runtime/             # Container runtime abstraction
│   │   ├── docker/          # Bollard-based Docker implementation
│   │   └── podman.rs        # Podman CLI wrapper
│   ├── proxy/               # Embedded reverse proxy
│   │   ├── router.rs        # ArcSwap-based route table
│   │   └── tls.rs           # ACME/Let's Encrypt
│   ├── db/                  # SQLite database layer
│   │   ├── models.rs        # All data models
│   │   └── seeders/         # 74 service template definitions
│   ├── backup/              # S3-compatible backup
│   ├── logging/             # Log draining (Axiom, New Relic, Datadog, HTTP)
│   ├── monitoring/          # Uptime, metrics, Prometheus exporter
│   ├── notifications/       # Alert channels (Slack, Discord, email, Telegram, etc.)
│   ├── mcp/                 # MCP server for AI assistant integration
│   ├── tui/                 # Terminal UI (ratatui + crossterm, --features tui)
│   ├── crypto/              # AES-256-GCM encryption utilities
│   ├── cli/                 # CLI subcommands (backup, restore, server, tui)
│   ├── config/              # Configuration parsing (TOML)
│   ├── startup/             # Self-checks and initialization
│   └── utils/               # Git operations and shared helpers
├── frontend/                # React + TypeScript dashboard
│   └── app/
│       ├── routes/          # Page-level route components
│       ├── components/      # Reusable UI components (shadcn/ui)
│       ├── types/           # TypeScript type definitions
│       └── lib/             # API client and utilities
├── migrations/              # SQLite schema migrations
├── static/dist/             # Built frontend assets (served by Rivetr)
├── docs/                    # Architecture and reference documentation
├── ansible/                 # Ansible playbook for automated provisioning
├── scripts/ralph/           # Ralph autonomous agent loop
├── .claude/                 # Claude Code agents and skills
├── live-testing/            # Manual testing guides
├── rivetr.example.toml      # Annotated configuration reference
└── install.sh               # Production install script
```

## Requirements

- **OS**: Linux (x86_64 or aarch64) for production; macOS and Windows for development
- **Runtime**: Docker Engine 24+ or Podman 4+
- **Ports**: 80 (HTTP proxy), 443 (HTTPS proxy), 8080 (API and dashboard)
- **Build from source**: Rust 1.75+, Node.js 20+

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and the PR checklist.

## Documentation

| Document | Description |
|---|---|
| [CONTRIBUTING.md](CONTRIBUTING.md) | Development setup, code style, PR process |
| [DEPLOYMENT.md](DEPLOYMENT.md) | Production install, configuration, backup, troubleshooting |
| [CHANGELOG.md](CHANGELOG.md) | Version history and release notes |
| [ROADMAP.md](ROADMAP.md) | Planned features and future direction |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | System design and component breakdown |
| [docs/TECH_STACK.md](docs/TECH_STACK.md) | Technology choices and crate selection |
| [docs/SERVICE-TEMPLATES.md](docs/SERVICE-TEMPLATES.md) | Catalogue of all 273 service templates by category |
| [docs/COMPETITIVE-GAP-ANALYSIS.md](docs/COMPETITIVE-GAP-ANALYSIS.md) | Feature comparison with Coolify and Dokploy |
| [docs/REFACTORING.md](docs/REFACTORING.md) | Code organization and module splitting guide |
| [docs/RALPH_GUIDE.md](docs/RALPH_GUIDE.md) | Ralph autonomous agent loop for feature development |
| [ansible/rivetr.yml](ansible/rivetr.yml) | Ansible playbook for automated server provisioning |
| [live-testing/TESTING-GUIDE.md](live-testing/TESTING-GUIDE.md) | Manual testing procedures |

## License

MIT — see [LICENSE](LICENSE) for details.


----
spin up subagents to work on the remaining items. whiles at it , spin up another agent to test already exsiting features to make sure there are no issues, way to improve
things, performance improvements, ui improvements,  etc...
coolify has this thing where when deploying a service/database , it opens a side pannel to load logs of the image pulling and starting
