# Rivetr - Task Tracker

> Use checkboxes to track progress: `- [ ]` → `- [x]`

---

## Phase 0: Foundation ✅ COMPLETE

### 0.1 Project Setup

- [x] **T0.1.1** Create Cargo.toml with workspace configuration
- [x] **T0.1.2** Add all dependencies to Cargo.toml (see TECH_STACK.md)
- [x] **T0.1.3** Create directory structure
- [x] **T0.1.4** Create `.gitignore`
- [ ] **T0.1.5** Create `rustfmt.toml` with project conventions
- [ ] **T0.1.6** Create `clippy.toml` or configure in Cargo.toml
- [x] **T0.1.7** Verify `cargo build` succeeds

### 0.2 Database Foundation

- [x] **T0.2.1** Create `db/mod.rs` with connection pool setup
- [x] **T0.2.2** Configure SQLite with WAL mode
- [x] **T0.2.3** Create migration: `001_initial.sql`
- [x] **T0.2.4** Implement `db::init()` function
- [x] **T0.2.5** Add migration runner at startup
- [ ] **T0.2.6** Test database initialization

### 0.3 Configuration System

- [x] **T0.3.1** Define `Config` struct in `config/mod.rs`
- [x] **T0.3.2** Implement TOML parsing with serde
- [x] **T0.3.3** Add environment variable overrides
- [x] **T0.3.4** Create example `rivetr.example.toml`
- [x] **T0.3.5** Add config validation
- [ ] **T0.3.6** Test config loading

### 0.4 Logging & CLI

- [x] **T0.4.1** Set up tracing-subscriber
- [x] **T0.4.2** Configure log level from config
- [x] **T0.4.3** Add CLI argument parsing with clap
- [x] **T0.4.4** Implement `--config` flag
- [x] **T0.4.5** Implement `--version` flag
- [ ] **T0.4.6** Test logging output

### 0.5 Basic Server

- [x] **T0.5.1** Create Axum app in `main.rs`
- [x] **T0.5.2** Add health check endpoint `GET /health`
- [x] **T0.5.3** Add tracing middleware
- [x] **T0.5.4** Implement graceful shutdown
- [ ] **T0.5.5** Test server starts and responds

**Phase 0 Checkpoint**: ✅ Server runs, connects to SQLite, responds to `/health`

---

## Phase 1: Core Engine (MVP) - IN PROGRESS

### 1.1 Container Runtime ✅ COMPLETE

- [x] **T1.1.1** Define `ContainerRuntime` trait in `runtime/mod.rs`
- [x] **T1.1.2** Implement `DockerRuntime` in `runtime/docker.rs`
- [x] **T1.1.3** Add Docker build with stream output
- [x] **T1.1.4** Add Docker run with port mapping
- [x] **T1.1.5** Add Docker stop/remove
- [x] **T1.1.6** Add Docker log streaming
- [x] **T1.1.7** Implement `PodmanRuntime` in `runtime/podman.rs`
- [x] **T1.1.8** Add Podman CLI build wrapper
- [x] **T1.1.9** Add Podman run/stop/logs wrappers
- [x] **T1.1.10** Implement runtime auto-detection
- [ ] **T1.1.11** Test both runtimes with sample container

### 1.2 Git Operations

- [ ] **T1.2.1** Create `utils/git.rs`
- [x] **T1.2.2** Implement `clone_repo(url, branch, dest)` (in engine/pipeline.rs)
- [ ] **T1.2.3** Add SSH key authentication
- [ ] **T1.2.4** Add HTTPS token authentication
- [ ] **T1.2.5** Add clone progress callback
- [ ] **T1.2.6** Implement cleanup function
- [ ] **T1.2.7** Test cloning public repo
- [ ] **T1.2.8** Test cloning private repo

### 1.3 Deployment Pipeline ✅ COMPLETE

- [x] **T1.3.1** Define `DeploymentStatus` enum
- [x] **T1.3.2** Create `engine/pipeline.rs`
- [x] **T1.3.3** Implement state machine for deployment
- [x] **T1.3.4** Implement `CloneStep`
- [x] **T1.3.5** Implement `BuildStep` (with build log streaming)
- [x] **T1.3.6** Implement `StartStep` (container on private port)
- [x] **T1.3.7** Implement `HealthCheckStep`
- [x] **T1.3.8** Implement `SwitchStep` (update proxy route)
- [x] **T1.3.9** Implement `CleanupStep` (remove old container)
- [ ] **T1.3.10** Add rollback functionality
- [x] **T1.3.11** Store deployment logs to database
- [ ] **T1.3.12** Test full deployment pipeline

### 1.4 REST API - Apps ✅ COMPLETE

- [x] **T1.4.1** Create `api/mod.rs` with router setup
- [x] **T1.4.2** Implement auth middleware (token validation)
- [x] **T1.4.3** Create `api/apps.rs`
- [x] **T1.4.4** Implement `POST /api/apps`
- [x] **T1.4.5** Implement `GET /api/apps`
- [x] **T1.4.6** Implement `GET /api/apps/:id`
- [x] **T1.4.7** Implement `PUT /api/apps/:id`
- [x] **T1.4.8** Implement `DELETE /api/apps/:id`
- [ ] **T1.4.9** Add input validation
- [ ] **T1.4.10** Test all CRUD operations

### 1.5 REST API - Deployments ✅ COMPLETE

- [x] **T1.5.1** Create `api/deployments.rs`
- [x] **T1.5.2** Implement `POST /api/apps/:id/deploy`
- [x] **T1.5.3** Implement `GET /api/apps/:id/deployments`
- [x] **T1.5.4** Implement `GET /api/deployments/:id`
- [ ] **T1.5.5** Implement `POST /api/deployments/:id/rollback`
- [ ] **T1.5.6** Test deployment API

### 1.6 REST API - Logs

- [ ] **T1.6.1** Create `api/logs.rs`
- [x] **T1.6.2** Implement `GET /api/deployments/:id/logs` (build logs)
- [ ] **T1.6.3** Implement WebSocket for runtime logs
- [ ] **T1.6.4** Test log streaming

### 1.7 REST API - Webhooks ✅ COMPLETE

- [x] **T1.7.1** Create `api/webhooks.rs`
- [x] **T1.7.2** Implement `POST /webhooks/github`
- [x] **T1.7.3** Parse GitHub push event payload
- [ ] **T1.7.4** Verify GitHub webhook signature
- [x] **T1.7.5** Implement `POST /webhooks/gitlab`
- [x] **T1.7.6** Implement `POST /webhooks/gitea`
- [x] **T1.7.7** Trigger deployment on webhook
- [ ] **T1.7.8** Test with real GitHub webhook

### 1.8 Reverse Proxy - IN PROGRESS

- [x] **T1.8.1** Create `proxy/mod.rs`
- [x] **T1.8.2** Implement route table with ArcSwap
- [x] **T1.8.3** Create HTTP proxy handler
- [x] **T1.8.4** Implement request forwarding to containers
- [x] **T1.8.5** Add Host header routing
- [x] **T1.8.6** Implement WebSocket proxying
- [ ] **T1.8.7** Create `proxy/tls.rs`
- [ ] **T1.8.8** Implement HTTPS with rustls
- [ ] **T1.8.9** Add ACME client (Let's Encrypt)
- [ ] **T1.8.10** Implement certificate auto-renewal
- [ ] **T1.8.11** Add route update API
- [ ] **T1.8.12** Test proxy with multiple domains

### 1.9 Dashboard UI - Setup ✅ COMPLETE (React + shadcn/ui)

- [x] **T1.9.1** Set up Vite + React + TypeScript
- [x] **T1.9.2** Install Tailwind CSS v4
- [x] **T1.9.3** Initialize shadcn/ui components
- [x] **T1.9.4** Configure path aliases (@/)
- [x] **T1.9.5** Set up Vite proxy for API
- [x] **T1.9.6** Add static file serving (tower-http)

### 1.10 Dashboard UI - Pages ✅ COMPLETE

- [x] **T1.10.1** Create login page with token auth
- [x] **T1.10.2** Implement AuthProvider context
- [x] **T1.10.3** Create Dashboard page (stats cards)
- [x] **T1.10.4** Create Apps list page (table)
- [x] **T1.10.5** Create App detail page (deployments)
- [x] **T1.10.6** Create "New App" form
- [x] **T1.10.7** Create Settings page (placeholder)
- [x] **T1.10.8** Create sidebar layout

### 1.11 Dashboard UI - React Features - IN PROGRESS

- [x] **T1.11.1** Add React Query for data fetching
- [x] **T1.11.2** Add React Router for navigation
- [x] **T1.11.3** Add protected routes
- [x] **T1.11.4** Add delete confirmation dialogs
- [x] **T1.11.5** Add live deployment status polling
- [x] **T1.11.6** Add real-time log streaming (WebSocket)

### 1.12 Health Checks

- [ ] **T1.12.1** Create `engine/health.rs`
- [x] **T1.12.2** Implement HTTP health check (in pipeline.rs)
- [x] **T1.12.3** Add configurable timeout
- [x] **T1.12.4** Add retry logic
- [x] **T1.12.5** Integrate with deployment pipeline
- [ ] **T1.12.6** Add health status to proxy routing
- [ ] **T1.12.7** Test health check behavior

**Phase 1 Checkpoint**: Full deployment from GitHub webhook with working UI

---

## Phase 2: Production Ready - NOT STARTED

### 2.1 Security

- [ ] **T2.1.1** Add input validation on all endpoints
- [ ] **T2.1.2** Implement rate limiting
- [ ] **T2.1.3** Add CSRF tokens for UI forms
- [ ] **T2.1.4** Encrypt env vars at rest
- [ ] **T2.1.5** Secure session cookies
- [ ] **T2.1.6** Add audit logging

### 2.2 Error Handling

- [ ] **T2.2.1** Create consistent error responses
- [ ] **T2.2.2** Add deployment failure recovery
- [ ] **T2.2.3** Implement container restart on crash
- [ ] **T2.2.4** Add startup self-checks
- [ ] **T2.2.5** Database integrity checks

### 2.3 Resource Management

- [ ] **T2.3.1** Add container CPU limits
- [ ] **T2.3.2** Add container memory limits
- [ ] **T2.3.3** Add build resource limits
- [ ] **T2.3.4** Add disk space monitoring
- [ ] **T2.3.5** Implement old deployment cleanup
- [ ] **T2.3.6** Add image cleanup

### 2.4 Observability

- [ ] **T2.4.1** Add Prometheus metrics endpoint
- [ ] **T2.4.2** Add request duration metrics
- [ ] **T2.4.3** Add deployment counter metrics
- [ ] **T2.4.4** Add container resource metrics
- [ ] **T2.4.5** Add health check metrics

### 2.5 CLI

- [ ] **T2.5.1** Implement `rivetr status`
- [ ] **T2.5.2** Implement `rivetr apps list`
- [ ] **T2.5.3** Implement `rivetr deploy <app>`
- [ ] **T2.5.4** Implement `rivetr logs <app>`
- [ ] **T2.5.5** Implement `rivetr config check`

### 2.6 Testing

- [ ] **T2.6.1** Add unit tests for core modules
- [ ] **T2.6.2** Add integration tests for API
- [ ] **T2.6.3** Add E2E deployment tests
- [ ] **T2.6.4** Set up CI pipeline
- [ ] **T2.6.5** Add test coverage reporting

### 2.7 Documentation

- [x] **T2.7.1** Write README with quickstart
- [ ] **T2.7.2** Document configuration options
- [ ] **T2.7.3** Document API endpoints
- [ ] **T2.7.4** Write deployment guide
- [ ] **T2.7.5** Write troubleshooting guide

**Phase 2 Checkpoint**: Production-ready release

---

## Progress Summary

| Phase | Total Tasks | Completed | Progress |
|-------|-------------|-----------|----------|
| Phase 0 | 24 | 20 | 83% |
| Phase 1 | 72 | 64 | 89% |
| Phase 2 | 28 | 1 | 4% |
| **Total** | **124** | **85** | **69%** |

---

## Next Priority Tasks

1. ~~**T1.11.5** - Add live deployment status polling~~ ✅
2. ~~**T1.11.6** - Add real-time log streaming (WebSocket)~~ ✅
3. ~~**T1.8.6** - Implement WebSocket proxying~~ ✅
4. **T1.12.6** - Add health status to proxy routing
5. **T1.3.10** - Add rollback functionality
6. **T1.4.9** - Add input validation

### MVP Status
Core deployment pipeline complete. Remaining items are polish and production-ready features.
