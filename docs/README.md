# Rivetr - Project Plan

> A fast, lightweight deployment engine built in Rust

## Vision

Rivetr is a **single-binary PaaS** that provides Coolify-like power with 10-30% of the resource usage. It targets engineers tired of bloat, VPS users with limited RAM, indie hackers, and self-hosters.

## Core Philosophy

**Build a "deployment runtime", not a "platform".**

### What We Are
- Single-binary, zero-dependency deployment engine
- Minimal runtime executor (~30MB RAM idle)
- Opinionated but override-friendly
- Full-featured dashboard with SSR

### What We Avoid
- No heavy daemons or external databases
- No Docker Desktop dependency
- No complex permission systems
- No bloated microservices architecture

## Target Resource Usage

| System | RAM Idle | CPU Idle |
|--------|----------|----------|
| Coolify | 400-800MB | High |
| **Rivetr** | **30-80MB** | **Near zero** |

## Current Capabilities (86% Complete)

### Core Features (Complete)
- **Git Deployments**: GitHub, GitLab, Gitea webhooks with signature verification
- **Multiple Build Types**: Dockerfile, Nixpacks, Railpack, Heroku/Paketo buildpacks, static sites
- **Container Runtime**: Docker and Podman support with auto-detection
- **HTTPS/TLS**: Automatic Let's Encrypt certificates with auto-renewal
- **Health Checks**: Configurable health endpoints with automatic rollback on failure
- **Real-time Logs**: WebSocket-based log streaming for builds and runtime

### Platform Services (Complete)
- **Managed Databases**: One-click PostgreSQL, MySQL, MongoDB, Redis
- **Docker Compose**: Deploy multi-container apps from compose files
- **26 Service Templates**: Grafana, Portainer, Uptime Kuma, Gitea, n8n, and more
- **Projects**: Organize apps, databases, and services together

### Dashboard Features
- **React + TypeScript**: Modern SSR dashboard with React Router v7
- **Real-time Updates**: Live deployment status, resource monitoring
- **Team Management**: RBAC with owner/admin/developer/viewer roles
- **Git Provider OAuth**: GitHub, GitLab, Bitbucket integration

## Plan Documents

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | System design and component breakdown |
| [TECH_STACK.md](./TECH_STACK.md) | Technology choices and crate selection |
| [TASKS.md](./TASKS.md) | Detailed task tracking (315/368 complete) |

## Quick Start

```bash
# One-liner install (Linux)
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash

# Or build from source
git clone https://github.com/KwaminaWhyte/rivetr.git
cd rivetr
cargo build --release
./target/release/rivetr --config rivetr.toml
```

## License

MIT License - see [LICENSE](../LICENSE)
