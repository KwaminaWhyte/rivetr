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

# Release build
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

## Architecture

### Core Components

The application runs as a single binary with these main subsystems:

1. **API Layer** (`src/api/`) - Axum-based REST API handling:
   - App CRUD operations
   - Deployment triggers
   - Git webhooks (GitHub, GitLab, Gitea)
   - Token-based auth middleware

2. **Deployment Engine** (`src/engine/`) - Orchestrates the deployment pipeline:
   - Clone → Build → Start → Health Check → Switch
   - Jobs processed via Tokio MPSC channels (no Redis)
   - Pipeline state tracked in `deployments` table

3. **Container Runtime** (`src/runtime/`) - Abstraction over container engines:
   - `DockerRuntime` uses Bollard crate (socket API)
   - `PodmanRuntime` uses CLI wrapper
   - Auto-detection via `detect_runtime()` function

4. **Database** (`src/db/`) - SQLite with WAL mode:
   - Single file: `data/rivetr.db`
   - Migrations in `migrations/001_initial.sql`
   - Models: `App`, `Deployment`, `DeploymentLog`, `EnvVar`

5. **Proxy** (`src/proxy/`) - HTTP reverse proxy using ArcSwap for lock-free route updates

6. **Frontend** (`frontend/`) - React + Vite + TypeScript + shadcn/ui dashboard:
   - Located in `frontend/` directory
   - Build output goes to `static/dist/`
   - Uses React Router for navigation
   - React Query for data fetching
   - shadcn/ui components with Tailwind CSS

7. **Email Templates** (`src/ui/`) - Reserved for email notifications (Phase 2)

### Data Flow

```
Webhook → API → Engine (MPSC) → Runtime (Docker/Podman) → Proxy Route Update
                    ↓
                 SQLite (state persistence)
```

### Key Traits

```rust
// Container runtime abstraction - implement for new runtimes
trait ContainerRuntime: Send + Sync {
    async fn build(&self, ctx: &BuildContext) -> Result<String>;
    async fn run(&self, config: &RunConfig) -> Result<String>;
    async fn stop(&self, container_id: &str) -> Result<()>;
    async fn remove(&self, container_id: &str) -> Result<()>;
    async fn logs(&self, container_id: &str) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>>;
    async fn inspect(&self, container_id: &str) -> Result<ContainerInfo>;
    async fn is_available(&self) -> bool;
}
```

### State Management

- `AppState` in `src/lib.rs` holds shared state (config, db pool, deploy channel)
- Route table uses `ArcSwap` for lock-free atomic updates
- Concurrent data structures from `dashmap` and `parking_lot`

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

Skills in `.claude/skills/` provide domain-specific knowledge:

| Skill | Purpose |
|-------|---------|
| `rust-review` | Rust idioms and patterns used in Rivetr (error handling, async, traits, Axum patterns) |
| `api-testing` | curl commands and patterns for testing all REST API endpoints (apps, deployments, databases, services) |
| `docker-testing` | Container runtime testing, Docker/Podman debugging, Nixpacks, Railpack, and buildpack support |
| `database-operations` | SQLite database patterns and SQLx usage (queries, migrations, models, debugging) |
| `deployment-pipeline` | Debug and understand the deployment pipeline (stages, build types, rollbacks, health checks) |
| `prd` | Generate Product Requirements Documents for new features |
| `ralph` | Convert PRDs to prd.json format for Ralph autonomous agent execution |

Skills are automatically activated when relevant to the current task.

## Ralph (Autonomous Agent Loop)

Ralph is an autonomous AI agent loop that runs Claude Code repeatedly until all PRD items are complete. Located in `scripts/ralph/`.

### Workflow

1. **Create PRD**: Use the `prd` skill to generate requirements
   ```
   Load the prd skill and create a PRD for [feature description]
   ```

2. **Convert to JSON**: Use the `ralph` skill to create prd.json
   ```
   Load the ralph skill and convert tasks/prd-[feature].md to prd.json
   ```

3. **Run Ralph**:
   - Linux/macOS: `./scripts/ralph/ralph.sh [max_iterations]`
   - Windows: `.\scripts\ralph\ralph.ps1 [max_iterations]`

### Key Concepts

- Each iteration spawns a fresh Claude Code instance
- Memory persists via git history, `progress.txt`, and `prd.json`
- Stories should be small (completable in one context window)
- All commits must pass: `cargo fmt --check && cargo clippy && cargo test`

See `scripts/ralph/README.md` for detailed documentation.

## Development Status

See `plan/TASKS.md` for detailed task tracking. Current status:
- **Phase 0 (Foundation)**: Complete (93%)
- **Phase 1 (MVP)**: Complete (100%)
- **Phase 2 (Production Ready)**: In progress (84%)
- **Phase 3 (Enhanced Features)**: In progress (91%)
- **Phase 4 (Platform Services)**: Complete (100%)
- **Phase 5 (Advanced CI/CD)**: In progress (67%)
- **Overall Progress**: 86% complete (315/368 tasks)

## Configuration

Two config files:
- `rivetr.toml` - Global server config (ports, auth, runtime)
- `deploy.toml` - Per-app config in repository root (dockerfile, healthcheck, resources)
