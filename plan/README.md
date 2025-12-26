# Rivetr - Project Plan

> A fast, lightweight deployment engine built in Rust

## Vision

Rivetr is a **single-binary PaaS** that provides Coolify-like power with 10-30% of the resource usage. It targets engineers tired of bloat, VPS users with limited RAM, indie hackers, and self-hosters.

## Core Philosophy

**Build a "deployment runtime", not a "platform".**

### What We Are
- Stateless controller
- Minimal runtime executor
- Opinionated but override-friendly
- Config-first, UI-second

### What We Avoid
- No heavy daemons
- No unnecessary UI state
- No Docker Desktop dependency
- No complex permission systems

## Target Resource Usage

| System | RAM Idle | CPU Idle |
|--------|----------|----------|
| Coolify | 400-800MB | High |
| **Rivetr** | **30-80MB** | **Near zero** |

## MVP Scope

- **Single-node only** (installed directly on the target server)
- **Hybrid container runtime** (Docker via Bollard + Podman/containerd support)
- **Dockerfile-first** builds (no magic buildpacks)
- **Single admin user** (simple token/basic auth)
- **Webhooks only** (no git polling)

## Plan Documents

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | System design and component breakdown |
| [TECH_STACK.md](./TECH_STACK.md) | Technology choices and crate selection |
| [PHASES.md](./PHASES.md) | Development phases with milestones |
| [TASKS.md](./TASKS.md) | Actionable task tracking with checkboxes |

## Quick Start (Future)

```bash
# Install
curl -sSL https://rivetr.dev/install.sh | sh

# Run
./rivetr --config rivetr.toml
```

## License

TBD
