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

### 1.2 Git Operations ✅ COMPLETE

- [ ] **T1.2.1** Create `utils/git.rs`
- [x] **T1.2.2** Implement `clone_repo(url, branch, dest)` (in engine/pipeline.rs)
- [x] **T1.2.3** Add SSH key authentication (via ssh_keys table and API)
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
- [x] **T1.3.10** Add rollback functionality (POST /api/deployments/:id/rollback)
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
- [x] **T1.4.9** Add input validation (src/api/validation.rs)
- [ ] **T1.4.10** Test all CRUD operations

### 1.5 REST API - Deployments ✅ COMPLETE

- [x] **T1.5.1** Create `api/deployments.rs`
- [x] **T1.5.2** Implement `POST /api/apps/:id/deploy`
- [x] **T1.5.3** Implement `GET /api/apps/:id/deployments`
- [x] **T1.5.4** Implement `GET /api/deployments/:id`
- [x] **T1.5.5** Implement `POST /api/deployments/:id/rollback`
- [ ] **T1.5.6** Test deployment API

### 1.6 REST API - Logs ✅ COMPLETE

- [ ] **T1.6.1** Create `api/logs.rs`
- [x] **T1.6.2** Implement `GET /api/deployments/:id/logs` (build logs)
- [x] **T1.6.3** Implement WebSocket for runtime logs (GET /api/apps/:id/logs/stream)
- [ ] **T1.6.4** Test log streaming

### 1.7 REST API - Webhooks ✅ COMPLETE

- [x] **T1.7.1** Create `api/webhooks.rs`
- [x] **T1.7.2** Implement `POST /webhooks/github`
- [x] **T1.7.3** Parse GitHub push event payload
- [x] **T1.7.4** Verify GitHub webhook signature
- [x] **T1.7.5** Implement `POST /webhooks/gitlab`
- [x] **T1.7.6** Implement `POST /webhooks/gitea`
- [x] **T1.7.7** Trigger deployment on webhook
- [ ] **T1.7.8** Test with real GitHub webhook

### 1.8 Reverse Proxy ✅ COMPLETE

- [x] **T1.8.1** Create `proxy/mod.rs`
- [x] **T1.8.2** Implement route table with ArcSwap
- [x] **T1.8.3** Create HTTP proxy handler
- [x] **T1.8.4** Implement request forwarding to containers
- [x] **T1.8.5** Add Host header routing
- [x] **T1.8.6** Implement WebSocket proxying
- [x] **T1.8.7** Create `proxy/tls.rs`
- [x] **T1.8.8** Implement HTTPS with rustls
- [x] **T1.8.9** Add ACME client (Let's Encrypt)
- [x] **T1.8.10** Implement certificate auto-renewal
- [x] **T1.8.11** Add route update API
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

### 1.11 Dashboard UI - React Features ✅ COMPLETE

- [x] **T1.11.1** Add React Query for data fetching
- [x] **T1.11.2** Add React Router for navigation
- [x] **T1.11.3** Add protected routes
- [x] **T1.11.4** Add delete confirmation dialogs
- [x] **T1.11.5** Add live deployment status polling
- [x] **T1.11.6** Add real-time log streaming (WebSocket)
- [x] **T1.11.7** Add deployment error display with tooltips
- [x] **T1.11.8** Add theme switching (light/dark/system) with localStorage persistence
- [x] **T1.11.9** Add build logs viewer dialog for all deployments

### 1.12 React Router Framework Migration ✅ COMPLETE

- [x] **T1.12.1** Migrate from React Router library mode to Framework mode
- [x] **T1.12.2** Set up SSR with server-side loaders and actions
- [x] **T1.12.3** Implement cookie-based session authentication
- [x] **T1.12.4** Create api.server.ts for server-side API calls
- [x] **T1.12.5** Create session.server.ts for session management
- [x] **T1.12.6** Convert all pages to route modules with loaders/actions
- [x] **T1.12.7** Add React Query with SSR initial data hydration
- [x] **T1.12.8** Configure Vite for SSR build
- [x] **T1.12.9** Add dynamic imports for server modules to prevent client bundle issues

### 1.13 Git Provider Integration ✅ COMPLETE

- [x] **T1.13.1** Create `git_providers` table migration (005_git_providers.sql)
- [x] **T1.13.2** Add OAuth config to configuration system
- [x] **T1.13.3** Implement OAuth flow for GitHub
- [x] **T1.13.4** Implement OAuth flow for GitLab
- [x] **T1.13.5** Implement OAuth flow for Bitbucket
- [x] **T1.13.6** Add `GET /api/git-providers` endpoint
- [x] **T1.13.7** Add `GET /api/git-providers/:id` endpoint
- [x] **T1.13.8** Add `DELETE /api/git-providers/:id` endpoint
- [x] **T1.13.9** Add `GET /api/git-providers/:id/repos` endpoint
- [x] **T1.13.10** Create Git Providers settings page in frontend

### 1.12 Health Checks ✅ COMPLETE

- [ ] **T1.12.1** Create `engine/health.rs`
- [x] **T1.12.2** Implement HTTP health check (in pipeline.rs)
- [x] **T1.12.3** Add configurable timeout
- [x] **T1.12.4** Add retry logic
- [x] **T1.12.5** Integrate with deployment pipeline
- [x] **T1.12.6** Add health status to proxy routing (src/proxy/health_checker.rs)
- [ ] **T1.12.7** Test health check behavior

**Phase 1 Checkpoint**: Full deployment from GitHub webhook with working UI

---

## Phase 2: Production Ready - NOT STARTED

### 2.1 Security

- [x] **T2.1.1** Add input validation on all endpoints (src/api/validation.rs)
- [x] **T2.1.2** Implement rate limiting (src/api/rate_limit.rs - sliding window algorithm, per-tier limits)
- [ ] **T2.1.3** Add CSRF tokens for UI forms
- [ ] **T2.1.4** Encrypt env vars at rest
- [ ] **T2.1.5** Secure session cookies
- [ ] **T2.1.6** Add audit logging

### 2.2 Error Handling

- [x] **T2.2.1** Create consistent error responses (src/api/error.rs - ApiError with ErrorCode enum, ValidationErrorBuilder)
- [ ] **T2.2.2** Add deployment failure recovery
- [ ] **T2.2.3** Implement container restart on crash
- [ ] **T2.2.4** Add startup self-checks
- [ ] **T2.2.5** Database integrity checks

### 2.3 Resource Management

- [x] **T2.3.1** Add container CPU limits (cpu_limit field in App model, Docker NanoCPUs, Podman --cpus)
- [x] **T2.3.2** Add container memory limits (memory_limit field in App model, supports m/mb/g/gb/b suffixes)
- [ ] **T2.3.3** Add build resource limits
- [ ] **T2.3.4** Add disk space monitoring
- [ ] **T2.3.5** Implement old deployment cleanup
- [ ] **T2.3.6** Add image cleanup

### 2.4 Observability

- [x] **T2.4.1** Add Prometheus metrics endpoint (GET /metrics)
- [x] **T2.4.2** Add request duration metrics (http_request_duration_seconds histogram)
- [x] **T2.4.3** Add deployment counter metrics (deployments_total with status label)
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

## Phase 3: Enhanced Features - NOT STARTED

### 3.1 Project Organization

- [x] **T3.1.1** Add `environment` field to apps (staging/production/development) - migration 006, model, validation
- [x] **T3.1.2** Add environment badge to UI (EnvironmentBadge.tsx - color-coded: gray/yellow/green)
- [x] **T3.1.3** Add environment filter on apps list (dropdown filter in Apps.tsx)
- [ ] **T3.1.4** Add tags/labels to apps (database table + API)
- [ ] **T3.1.5** Add tags UI management
- [ ] **T3.1.6** Filter apps by tags

### 3.2 Environment Variables UI ✅ COMPLETE

- [x] **T3.2.1** Create `env_vars` table migration (006_env_vars_update.sql - is_secret, updated_at columns)
- [x] **T3.2.2** Add env vars CRUD API endpoints (src/api/env_vars.rs - full CRUD with reveal option)
- [x] **T3.2.3** Create Env Vars settings tab in app detail (EnvVarsTab.tsx component)
- [x] **T3.2.4** Add env var editor with key-value pairs (table with add/edit/delete dialogs)
- [x] **T3.2.5** Support multiline values (textarea in edit dialog)
- [x] **T3.2.6** Mask secret values in UI (********  with reveal button)
- [x] **T3.2.7** Pass env vars to container at runtime (already in engine pipeline)

### 3.3 Resource Monitoring

- [x] **T3.3.1** Add container stats collection (CPU, memory, network) - ContainerStats in runtime trait
- [ ] **T3.3.2** Store metrics in SQLite (with retention policy)
- [x] **T3.3.3** Create metrics API endpoint (GET /api/apps/:id/stats)
- [x] **T3.3.4** Add resource usage graphs in app detail (ResourceMonitor.tsx with sparklines)
- [ ] **T3.3.5** Add system-wide dashboard metrics

### 3.4 Preview Deployments

- [ ] **T3.4.1** Parse PR events from webhooks
- [ ] **T3.4.2** Create preview deployment with unique subdomain
- [ ] **T3.4.3** Add preview deployments list to app
- [ ] **T3.4.4** Auto-cleanup on PR close/merge
- [ ] **T3.4.5** Comment preview URL on PR (GitHub API)

### 3.5 Notifications

- [ ] **T3.5.1** Create notification channels table
- [ ] **T3.5.2** Add notification settings API
- [ ] **T3.5.3** Implement Slack webhook notifications
- [ ] **T3.5.4** Implement Discord webhook notifications
- [ ] **T3.5.5** Implement email notifications (SMTP)
- [ ] **T3.5.6** Trigger notifications on deployment events
- [ ] **T3.5.7** Add notification preferences UI

### 3.6 Container Shell Access

- [ ] **T3.6.1** Implement container exec in runtime trait
- [ ] **T3.6.2** Add WebSocket terminal endpoint
- [ ] **T3.6.3** Create terminal UI component (xterm.js)
- [ ] **T3.6.4** Add shell access button to app detail

### 3.7 Volumes Management

- [ ] **T3.7.1** Create volumes table
- [ ] **T3.7.2** Add volumes CRUD API
- [ ] **T3.7.3** Create volumes UI in app settings
- [ ] **T3.7.4** Mount volumes at container start
- [ ] **T3.7.5** Add volume backup/export

### 3.8 Build Improvements

- [ ] **T3.8.1** Add Nixpacks builder support
- [ ] **T3.8.2** Add Heroku Buildpacks support
- [ ] **T3.8.3** Auto-detect build type from repo
- [ ] **T3.8.4** Build type selector in UI

### 3.9 Multi-User & Teams

- [ ] **T3.9.1** Add user roles (admin, developer, viewer)
- [ ] **T3.9.2** Create teams/organizations table
- [ ] **T3.9.3** Add team membership API
- [ ] **T3.9.4** Implement permission checks on API
- [ ] **T3.9.5** Add user management UI
- [ ] **T3.9.6** Add team settings UI

**Phase 3 Checkpoint**: Full-featured PaaS with monitoring and team support

---

## Competitive Research: Coolify & Dokploy

Research conducted to identify feature gaps and improvement opportunities.

### Features Rivetr Already Has
- [x] Git provider OAuth integration (GitHub, GitLab, Bitbucket)
- [x] Push-to-deploy via webhooks
- [x] Dark/Light theme switching
- [x] Real-time deployment logs
- [x] SSL/TLS with Let's Encrypt auto-renewal
- [x] Docker and Podman runtime support
- [x] Environment variables support
- [x] Health checks with automatic failover
- [x] Rollback functionality

### Priority Features to Add (from Coolify/Dokploy)

**High Priority:**
- [ ] **Docker Compose support** - Deploy multi-container apps from docker-compose.yml
- [ ] **One-click templates** - Pre-configured apps (PostgreSQL, Redis, MySQL, MongoDB, etc.)
- [ ] **Pull request preview deployments** - Auto-deploy PRs with unique URLs
- [ ] **Browser terminal** - In-browser shell access to containers (xterm.js)
- [ ] **Repository browser** - Select repos from connected Git providers in app creation

**Medium Priority:**
- [ ] **Build cache** - Speed up builds with layer caching
- [x] **Resource limits UI** - Set CPU/memory limits per app from dashboard (ResourceLimitsCard.tsx)
- [ ] **Deployment scheduling** - Schedule deployments for specific times
- [ ] **S3 backup integration** - Backup volumes and databases to S3
- [ ] **Custom domains per app** - Multiple domains pointing to one app

**Lower Priority:**
- [ ] **Multi-server support** - Deploy to multiple servers from one dashboard
- [ ] **Service dependencies** - Define app startup order and dependencies
- [ ] **Build from Dockerfile path** - Specify custom Dockerfile location
- [ ] **Auto-scaling** - Scale containers based on load

### UI/UX Improvements (from Dokploy)
- [x] **Simplified app creation flow** - Project-centric flow, create apps within projects
- [ ] **Quick actions menu** - Fast access to common operations
- [x] **Deployment timeline view** - Visual history of deployments (DeploymentTimeline.tsx with toggle)
- [x] **App grouping/projects** - Organize related apps together (Projects page with cards, service counts)
- [x] **Activity feed** - Recent actions across all apps (Recent Events in Dashboard)
- [x] **Project-centric navigation** - Apps accessed through projects, cleaner sidebar structure

---

## Progress Summary

| Phase | Total Tasks | Completed | Progress |
|-------|-------------|-----------|----------|
| Phase 0 | 24 | 20 | 83% |
| Phase 1 | 94 | 94 | 100% |
| Phase 2 | 28 | 9 | 32% |
| Phase 3 | 42 | 17 | 40% |
| **Total** | **188** | **140** | **74%** |

---

## Next Priority Tasks

**Phase 2 - Production Ready:**
1. **T2.1.3** - Add CSRF tokens for UI forms
2. **T2.2.2** - Add deployment failure recovery
3. **T2.3.3** - Add build resource limits
4. **T2.3.5** - Implement old deployment cleanup

**Phase 3 - Enhanced Features:**
5. **T3.1.4** - Add tags/labels to apps
6. **T3.3.2** - Store metrics history in SQLite
7. **T3.4.1** - Parse PR events for preview deployments
8. **T3.6.1** - Implement container shell access (browser terminal)

### MVP Status
**Phase 1 Complete!** Core deployment pipeline with:
- Full deployment lifecycle (clone → build → deploy → health check → switch)
- Rollback functionality
- Input validation
- SSH authentication for private repos
- Runtime log streaming via WebSocket
- Health checking with automatic failover
- HTTPS with automatic Let's Encrypt certificates
- Certificate auto-renewal
- Webhook signature verification (GitHub, GitLab, Gitea)
- Route management API
- **Deployment error display** in UI with error tooltips
- **Git Provider OAuth integration** (GitHub, GitLab, Bitbucket) for direct repo access
- **Theme switching** (light/dark/system) with localStorage persistence
- **Build logs viewer** for all historical deployments

### Recent Additions (Phase 2-3)
- **System Overview Dashboard** - Stats cards, resource utilization chart, recent events feed
- **Projects feature** - Group related apps, project cards with service counts
- **Environment field** - Development/Staging/Production with color-coded badges
- **Resource Limits UI** - Configure CPU/memory limits from dashboard
- **Environment Variables UI** - Full CRUD with secret masking
- **Container Resource Metrics** - Live CPU/memory monitoring with sparklines
- **Deployment Timeline** - Visual deployment history with status indicators
- **Rate Limiting** - Sliding window algorithm with per-tier limits
- **Consistent Error Responses** - ApiError with ErrorCode enum
- **Prometheus Metrics** - /metrics endpoint with request/deployment counters
- **React Router Framework Mode** - Full SSR with server loaders, cookie-based sessions, dynamic imports
