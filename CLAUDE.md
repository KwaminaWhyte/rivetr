# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rivetr is a single-binary PaaS (Platform as a Service) built in Rust. It deploys applications from Git webhooks with minimal resource usage (~30MB RAM idle vs Coolify's ~800MB). The project uses an embedded SQLite database, supports both Docker and Podman runtimes, and includes an embedded reverse proxy.

## Build Commands

```bash
# Development with auto-reload (recommended)
# Install once: cargo install cargo-watch
cargo watch -x "run -- --config rivetr.toml"

# Development build and run (manual)
cargo run -- --config rivetr.toml

# Release build (local macOS only, NOT suitable for server deploy)
cargo build --release

# Check compilation without building
cargo check

# Run tests
cargo test

# Run a single test
cargo test test_name

# Linting
cargo fmt --check
cargo clippy
```

## Deploying to the Dev Server

**ALWAYS use the deploy script**: never scp a locally-built macOS binary to the Linux server (Exec format error).

```bash
# Full deploy (frontend + backend): cross-compiles for Linux x86_64 via cargo-zigbuild
./scripts/deploy-dev.sh

# Backend only (skip frontend rebuild)
./scripts/deploy-dev.sh --backend-only

# Frontend only (rare, frontend is embedded in binary, so this flag is mostly a no-op)
./scripts/deploy-dev.sh --frontend-only
```

Requires `cargo-zigbuild` installed: `cargo install cargo-zigbuild` and Zig on PATH.
Server: `root@187.124.50.183`, binary installed at `/opt/rivetr/rivetr`.

## Verify

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings   # same flags as CI
cargo test
cd frontend && npm run typecheck && npm run build
```

## Architecture

See `docs/architecture/architecture.md` (components, source layout, tech stack). Conventions that matter when editing:

- Container operations go through the `ContainerRuntime` trait (`src/runtime/mod.rs`); implement new operations for both Docker and Podman.
- Migrations are numbered files in `migrations/`, registered at the end of `run_migrations` in `src/db/mod.rs`, and must be idempotent.
- Compose services deploy through `src/engine/service_deploy/`; see `docs/reference/service-deployments.md`.
- Frontend URL structure follows `docs/planning/ui-information-architecture.md`; when moving a page add a redirect in `frontend/app/lib/redirects.ts` and update `lib/nav.ts` / `lib/resource-nav.ts` (breadcrumbs and sidebars read from them).
- Shared page blocks in `frontend/app/components/page/`; per-browser UI state via `usePersistedState` in `lib/persisted-state.ts`; theme engine in `lib/theme/`.

## Sub-Agents

Custom sub-agents are available in `.claude/agents/` for specialized tasks:

| Agent | Purpose | When to Use |
|-------|---------|-------------|
| `code-reviewer` | Rust code review specialist | After writing/modifying Rust code for quality, safety, and patterns |
| `frontend-reviewer` | React/TypeScript review | After writing/modifying frontend code (SSR, accessibility, best practices) |
| `debugger` | Debug errors and failures | When encountering compilation errors, panics, test failures, or deployment issues |
| `test-runner` | Run and fix tests | After code changes to verify tests pass (backend and frontend) |
| `security-reviewer` | Security audit specialist | After writing code handling auth, user input, secrets, or external data |

### Invoking Agents

Agents are automatically invoked based on context, or explicitly:
```
Use the code-reviewer agent to review my changes
Have the debugger agent investigate this error
Ask the test-runner agent to fix failing tests
Run security-reviewer to audit authentication changes
```

## Skills

Skills in `.claude/skills/` provide domain-specific knowledge and workflows:

| Skill | Type | Purpose |
|-------|------|---------|
| `api-testing` | Reference | curl commands for testing all REST API endpoints |
| `docker-testing` | Reference | Container runtime testing, Docker/Podman debugging, build types |
| `database-operations` | Reference | SQLite/SQLx patterns, queries, migrations, models |
| `deployment-pipeline` | Reference | Pipeline stages, build types, rollbacks, debugging |
| `frontend-patterns` | Reference | React Router v8, React Query, shadcn/ui patterns for the dashboard |
| `/prd` | Workflow | Generate Product Requirements Documents for new features |

Reference skills are auto-loaded by Claude when relevant. Workflow skills (prefixed with `/`) are invoked manually.

## Configuration

`rivetr.toml` is the only config file (server, auth, runtime, proxy). Reference: `docs/reference/configuration.md`. Per-app settings live in the database.
