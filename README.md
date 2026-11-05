<p align="center">
  <img src="frontend/public/brand/rivetr-mark.png" alt="Rivetr logo" width="88" height="88" />
</p>

# Rivetr

Deploy applications from Git with a single binary, ~30MB RAM idle, no external dependencies.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](https://www.rust-lang.org/)
[![Build](https://img.shields.io/github/actions/workflow/status/KwaminaWhyte/rivetr/ci.yml?branch=main)](https://github.com/KwaminaWhyte/rivetr/actions)
[![XERJ Corpus Hub contributor](https://hub.xerj.org/contributors/KwaminaWhyte/badge.svg)](https://hub.xerj.org/contributors/KwaminaWhyte)

![Rivetr projects page](docs/images/projects.png)

![Rivetr app overview with live resource usage](docs/images/app-overview.png)

## What is Rivetr?

Rivetr is a self-hosted PaaS (Platform as a Service) that deploys applications from Git with minimal resource usage. It ships as a single binary with an embedded SQLite database, an embedded reverse proxy with automatic HTTPS, and a full-featured React dashboard, no Redis, no PostgreSQL, no Traefik required.

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
- Service deployments: Deploy button with recorded history and live logs, build images from git or a server path and push to a registry, pre/post-deploy hooks (for example migrations), rollback by image digest, per-container logs/restart/terminal, encrypted env vars written to a managed `.env` (see [docs/reference/service-deployments.md](docs/reference/service-deployments.md))
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

Rivetr ships with an optional keyboard-driven terminal dashboard built with [ratatui](https://ratatui.rs/). It connects to any running Rivetr instance, local or remote, without opening a browser.

### Using the TUI from your local machine

**Prerequisites:** A running Rivetr instance (e.g. `https://rivetr.site`) and an API token from **Settings → API Tokens**.

**Option 1: Download the pre-built binary (macOS/Linux):**

```bash
# Download the TUI-enabled binary for macOS (Apple Silicon)
curl -Lo rivetr https://github.com/KwaminaWhyte/rivetr/releases/latest/download/rivetr-macos-arm64
chmod +x rivetr

# Connect to your Rivetr instance
./rivetr tui --url https://rivetr.site --token rvt_your_token_here
```

**Option 2: Build from source:**

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

It installs Docker if needed, the Rivetr binary, build tools (Nixpacks, Railpack, Pack), a systemd service and a config with a generated admin token. Then open `http://your-server-ip:8080` and create your admin account.

Manual install, upgrades, backups and troubleshooting: [DEPLOYMENT.md](DEPLOYMENT.md).

### Development Setup

See [CONTRIBUTING.md](CONTRIBUTING.md#development-setup). Prerequisites: Rust 1.95+, Node.js 20+, Docker or Podman.

## Configuration

Server settings live in `rivetr.toml`. A minimal production file:

```toml
[server]
api_port = 8080
data_dir = "/var/lib/rivetr"

[auth]
admin_token = "change-me-in-production"

[proxy]
acme_email = "admin@example.com"
# base_domain = "apps.example.com"  # enables *.apps.example.com subdomains
```

Every key is documented in [docs/reference/configuration.md](docs/reference/configuration.md); [`rivetr.example.toml`](rivetr.example.toml) is a copyable starter. Per-app settings (build type, health check, resources, domains, env vars) are managed from the dashboard or API.

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

See [docs/architecture/architecture.md](docs/architecture/architecture.md#source-layout) for the source layout and how the components fit together.

## Requirements

- **OS**: Linux (x86_64 or aarch64) for production; macOS and Windows for development
- **Runtime**: Docker Engine 24+ or Podman 4+
- **Ports**: 80 (HTTP proxy), 443 (HTTPS proxy), 8080 (API and dashboard)
- **Build from source**: Rust 1.95+, Node.js 20+

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and the PR checklist.

## Documentation

| Document | Description |
|---|---|
| [CONTRIBUTING.md](CONTRIBUTING.md) | Development setup, code style, PR process |
| [DEPLOYMENT.md](DEPLOYMENT.md) | Production install, upgrades, backup, troubleshooting |
| [CHANGELOG.md](CHANGELOG.md) | Version history and release notes |
| [ROADMAP.md](ROADMAP.md) | Open work and where it is tracked |
| [docs/architecture/architecture.md](docs/architecture/architecture.md) | System design, source layout and tech stack |
| [docs/reference/configuration.md](docs/reference/configuration.md) | Every `rivetr.toml` key |
| [docs/reference/remote-servers.md](docs/reference/remote-servers.md) | Remote servers: adding, non-root sudo, Tailscale |
| [docs/reference/service-templates.md](docs/reference/service-templates.md) | Catalogue of all 273 service templates by category |
| [docs/reference/service-deployments.md](docs/reference/service-deployments.md) | Deploying, building, hooks, rollback and env for Docker Compose services |
| [docs/research/competitive-gap-analysis.md](docs/research/competitive-gap-analysis.md) | Open gaps against Coolify, Dokploy and others (Nov 2026) |
| [ansible/rivetr.yml](ansible/rivetr.yml) | Ansible playbook for automated server provisioning |
| [live-testing/TESTING-GUIDE.md](live-testing/TESTING-GUIDE.md) | Manual testing procedures |

## License

MIT, see [LICENSE](LICENSE) for details.
