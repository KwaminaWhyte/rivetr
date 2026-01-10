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

### Production Install (Recommended)

The one-liner install script sets up everything you need on a fresh Linux server:

```bash
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

**What the install script does:**

1. **Installs Docker** - If not already present (supports Ubuntu, Debian, Fedora, CentOS, RHEL)
2. **Installs Build Tools** - Git, Nixpacks, Railpack (optional), Pack CLI for buildpacks
3. **Creates Service User** - `rivetr` user with Docker access
4. **Downloads Binary** - From GitHub releases (or builds from source as fallback)
5. **Creates Configuration** - `/opt/rivetr/rivetr.toml` with secure admin token
6. **Sets Up Systemd Service** - Auto-start on boot, auto-restart on crash
7. **Configures Firewall** - Opens ports 80, 443, and 8080

**After installation:**

```
Web Dashboard:  http://your-server-ip:8080
Config File:    /opt/rivetr/rivetr.toml
Data Directory: /var/lib/rivetr
Service Logs:   sudo journalctl -u rivetr -f
```

**Environment Variables:**

| Variable | Default | Description |
|----------|---------|-------------|
| `RIVETR_VERSION` | `v0.2.3` | Version to install (or `latest`) |
| `INSTALL_DIR` | `/opt/rivetr` | Binary installation directory |
| `DATA_DIR` | `/var/lib/rivetr` | Data storage directory |

Example with custom version:
```bash
RIVETR_VERSION=latest curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

### Service Management

```bash
# Check status
sudo systemctl status rivetr

# Start/stop/restart
sudo systemctl start rivetr
sudo systemctl stop rivetr
sudo systemctl restart rivetr

# View logs
sudo journalctl -u rivetr -f

# View last 100 lines
sudo journalctl -u rivetr -n 100
```

### Development Setup

**Linux/macOS:**

```bash
git clone https://github.com/KwaminaWhyte/rivetr.git
cd rivetr
chmod +x scripts/setup.sh
./scripts/setup.sh
```

**Windows (PowerShell):**

```powershell
git clone https://github.com/KwaminaWhyte/rivetr.git
cd rivetr
.\scripts\setup.ps1
```

**Manual Build:**

```bash
git clone https://github.com/KwaminaWhyte/rivetr.git
cd rivetr
cargo build --release
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

## Troubleshooting

### Port 80 Permission Denied

If you see `Proxy server error error=Permission denied (os error 13)` in the logs, the service doesn't have permission to bind to port 80. The install script handles this automatically, but if you're running manually:

```bash
# Option 1: Use setcap (recommended)
sudo setcap 'cap_net_bind_service=+ep' /opt/rivetr/rivetr

# Option 2: Run as root (not recommended)
sudo ./rivetr --config rivetr.toml
```

### Service Won't Start

Check the logs for details:
```bash
sudo journalctl -u rivetr -n 50 --no-pager
```

Common issues:
- **Missing Docker**: Ensure Docker is installed and running (`systemctl status docker`)
- **Port in use**: Check if ports 80 or 8080 are already in use (`ss -tlnp | grep -E ':(80|8080)'`)
- **Config error**: Validate your config file (`/opt/rivetr/rivetr --config /opt/rivetr/rivetr.toml config check`)

### Reinstalling

To completely remove and reinstall:
```bash
sudo systemctl stop rivetr
sudo rm -rf /opt/rivetr /var/lib/rivetr /etc/systemd/system/rivetr.service
sudo userdel rivetr
sudo systemctl daemon-reload

# Then run the install script again
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

### Updating

To update to a newer version:
```bash
sudo systemctl stop rivetr
RIVETR_VERSION=v0.2.4 curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

The install script preserves your existing configuration.

## Documentation

- [ROADMAP.md](ROADMAP.md) - Development roadmap and planned features
- [CHANGELOG.md](CHANGELOG.md) - Version history and release notes
- [plan/TASKS.md](plan/TASKS.md) - Detailed task tracking

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Inspired by [Coolify](https://coolify.io/), built for developers who want simplicity and efficiency.
