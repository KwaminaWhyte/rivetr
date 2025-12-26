# Rivetr

A fast, lightweight deployment engine built in Rust.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

## What is Rivetr?

Rivetr is a **single-binary PaaS** that lets you deploy applications from Git with minimal resource usage. Think Coolify, but using 30MB RAM instead of 800MB.

### Features

- **Single Binary** - No external databases, no Redis, no separate proxy
- **Git Webhooks** - Deploy on push from GitHub, GitLab, or Gitea
- **Embedded Proxy** - Automatic HTTPS with Let's Encrypt
- **Hybrid Runtime** - Works with Docker or Podman
- **Web Dashboard** - Simple UI for managing deployments
- **Real-time Logs** - Stream build and runtime logs

### Resource Comparison

| System | RAM (Idle) | Dependencies |
|--------|------------|--------------|
| Coolify | 400-800MB | PostgreSQL, Redis, Traefik |
| **Rivetr** | **30-80MB** | Docker or Podman |

## Quick Start

```bash
# Download
curl -sSL https://github.com/yourusername/rivetr/releases/latest/download/rivetr-linux-amd64 -o rivetr
chmod +x rivetr

# Run
./rivetr --config rivetr.toml
```

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

- Linux (x86_64 or aarch64)
- Docker Engine 24+ OR Podman 4+
- Ports 80, 443, 8080 available

## Building from Source

```bash
# Clone
git clone https://github.com/yourusername/rivetr.git
cd rivetr

# Build
cargo build --release

# Binary at target/release/rivetr
```

## Project Structure

```
rivetr/
├── src/
│   ├── main.rs          # Entry point
│   ├── api/             # REST API routes
│   ├── config/          # Configuration
│   ├── db/              # SQLite database
│   ├── engine/          # Deployment pipeline
│   ├── proxy/           # Reverse proxy
│   ├── runtime/         # Container abstraction
│   └── ui/              # Dashboard templates
├── templates/           # Askama HTML templates
├── migrations/          # Database migrations
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

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Inspired by [Coolify](https://coolify.io/), built for developers who want simplicity and efficiency.
