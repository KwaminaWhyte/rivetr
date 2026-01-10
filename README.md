# Rivetr

A fast, lightweight deployment engine built in Rust.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

## What is Rivetr?

Rivetr is a **single-binary PaaS** that lets you deploy applications from Git with minimal resource usage. Think Coolify, but using 30MB RAM instead of 800MB.

### Features

- **Single Binary** - No external databases, no Redis, no separate proxy
- **Git Webhooks** - Deploy on push from GitHub, GitLab, or Gitea
- **Multiple Build Types** - Dockerfile, Nixpacks, Railpack, Heroku/Paketo buildpacks, static sites
- **Embedded Proxy** - Automatic HTTPS with Let's Encrypt
- **Hybrid Runtime** - Works with Docker or Podman
- **Managed Databases** - One-click PostgreSQL, MySQL, MongoDB, Redis
- **Docker Compose** - Deploy multi-container apps from compose files
- **26 Service Templates** - Grafana, Portainer, Uptime Kuma, Gitea, n8n, and more
- **Web Dashboard** - Modern React SSR dashboard with real-time updates
- **Real-time Logs** - Stream build and runtime logs via WebSocket
- **Team Management** - RBAC with owner/admin/developer/viewer roles

### Resource Comparison

| System     | RAM (Idle)  | Dependencies               |
| ---------- | ----------- | -------------------------- |
| Coolify    | 400-800MB   | PostgreSQL, Redis, Traefik |
| **Rivetr** | **30-80MB** | Docker or Podman           |

## Quick Start

### Using Setup Script

**Linux/macOS:**

```bash
# One-liner install (Linux)
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash

# Or build from source
git clone https://github.com/KwaminaWhyte/rivetr.git
cd rivetr
cargo build --release
./target/release/rivetr --config rivetr.toml


git clone https://github.com/Kwaminawhyte/rivetr.git
cd rivetr
chmod +x scripts/setup.sh
./scripts/setup.sh
```

**Windows (PowerShell):**

```powershell
git clone https://github.com/Kwaminawhyte/rivetr.git
cd rivetr
.\scripts\setup.ps1
```

The setup script will:

1. Check prerequisites (Rust, Git, Docker/Podman)
2. Create necessary directories
3. Set up configuration
4. Build the project
5. Optionally start the server

### Manual Setup

```bash
# Clone
git clone https://github.com/Kwaminawhyte/rivetr.git
cd rivetr

# Build
cargo build --release

# Run
./target/release/rivetr --config rivetr.toml
```

On first visit to `http://localhost:8080`, you'll be prompted to create your admin account.

## Configuration

Create `rivetr.toml`:

```toml
[server]
host = "0.0.0.0"
api_port = 8080
proxy_port = 80

[auth]
admin_token = "your-secret-token"

[runtime]
type = "auto"  # "docker", "podman", or "auto"
```

## Usage

### Deploy an App

1. Open the dashboard at `http://your-server:8080`
2. Add your Git repository URL
3. Configure your domain
4. Push to deploy

### Per-App Configuration

Add `deploy.toml` to your repository root:

```toml
app = "my-api"
port = 3000

[build]
dockerfile = "./Dockerfile"

[deploy]
healthcheck = "/health"

[resources]
memory = "256mb"
```

## Requirements

- **OS**: Windows 10+, Linux (x86_64 or aarch64), macOS
- **Runtime**: Docker Engine 24+ OR Podman 4+
- **Build**: Rust 1.75+, Git
- **Ports**: 80 (proxy), 8080 (API/dashboard)

## Project Structure

```
rivetr/
├── src/
│   ├── main.rs          # Entry point
│   ├── api/             # REST API routes (Axum)
│   ├── config/          # Configuration (TOML)
│   ├── db/              # SQLite database + models
│   ├── engine/          # Deployment pipeline + builders
│   ├── proxy/           # Reverse proxy + TLS/ACME
│   ├── runtime/         # Container abstraction (Docker/Podman)
│   └── startup/         # Self-checks and initialization
├── frontend/            # React Router v7 + Vite + shadcn/ui
├── .claude/
│   ├── agents/          # Claude Code sub-agents
│   └── skills/          # Claude Code skills
├── scripts/             # Setup scripts (setup.sh, setup.ps1)
├── migrations/          # SQLite migrations
└── plan/                # Development roadmap
```

## Contributing

Contributions are welcome! Please read the development plan in `plan/` to understand the architecture.

```bash
# Run in development
cargo run -- --config rivetr.example.toml

# Run tests
cargo test

# Check formatting
cargo fmt --check
cargo clippy
```

## Documentation

- [ROADMAP.md](ROADMAP.md) - Development roadmap and planned features
- [CHANGELOG.md](CHANGELOG.md) - Version history and release notes
- [plan/TASKS.md](plan/TASKS.md) - Detailed task tracking

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Inspired by [Coolify](https://coolify.io/), built for developers who want simplicity and efficiency.
