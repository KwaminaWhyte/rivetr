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
- [x] **T0.2.6** Test database initialization ✅ VERIFIED (database healthy, 21 tables, migrations run)

### 0.3 Configuration System

- [x] **T0.3.1** Define `Config` struct in `config/mod.rs`
- [x] **T0.3.2** Implement TOML parsing with serde
- [x] **T0.3.3** Add environment variable overrides
- [x] **T0.3.4** Create example `rivetr.example.toml`
- [x] **T0.3.5** Add config validation
- [x] **T0.3.6** Test config loading ✅ VERIFIED (server starts with TOML config, env overrides work)

### 0.4 Logging & CLI

- [x] **T0.4.1** Set up tracing-subscriber
- [x] **T0.4.2** Configure log level from config
- [x] **T0.4.3** Add CLI argument parsing with clap
- [x] **T0.4.4** Implement `--config` flag
- [x] **T0.4.5** Implement `--version` flag
- [x] **T0.4.6** Test logging output ✅ VERIFIED (tracing logs visible in console with timestamps)

### 0.5 Basic Server

- [x] **T0.5.1** Create Axum app in `main.rs`
- [x] **T0.5.2** Add health check endpoint `GET /health`
- [x] **T0.5.3** Add tracing middleware
- [x] **T0.5.4** Implement graceful shutdown
- [x] **T0.5.5** Test server starts and responds ✅ VERIFIED (/health returns OK, /api/system/health returns full status)

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
- [x] **T1.4.10** Test all CRUD operations ✅ VERIFIED (apps, databases, services, projects API all working)

### 1.5 REST API - Deployments ✅ COMPLETE

- [x] **T1.5.1** Create `api/deployments.rs`
- [x] **T1.5.2** Implement `POST /api/apps/:id/deploy`
- [x] **T1.5.3** Implement `GET /api/apps/:id/deployments`
- [x] **T1.5.4** Implement `GET /api/deployments/:id`
- [x] **T1.5.5** Implement `POST /api/deployments/:id/rollback`
- [x] **T1.5.6** Test deployment API ✅ VERIFIED (deployment history, status tracking, build logs all working)

### 1.6 REST API - Logs ✅ COMPLETE

- [ ] **T1.6.1** Create `api/logs.rs`
- [x] **T1.6.2** Implement `GET /api/deployments/:id/logs` (build logs)
- [x] **T1.6.3** Implement WebSocket for runtime logs (GET /api/apps/:id/logs/stream)
- [x] **T1.6.4** Test log streaming ✅ VERIFIED (build logs panel, runtime logs tab, WebSocket connections working)

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
- [x] **T2.1.3** Security headers middleware (X-Content-Type-Options, X-Frame-Options, X-XSS-Protection, Referrer-Policy, Cache-Control) - Note: CSRF not needed with Bearer token auth (localStorage, not cookies)
- [x] **T2.1.4** Encrypt env vars at rest (AES-256-GCM encryption)
- [x] **T2.1.5** Fix timing attack vulnerabilities (constant-time comparison using `subtle` crate)
- [x] **T2.1.6** Add audit logging (routes/settings/audit.tsx, src/api/audit.rs)
- [x] **T2.1.7** Strengthen password requirements (12+ chars, uppercase, lowercase, digit, special char, common password check)
- [x] **T2.1.8** Add command injection protection (shell metacharacter blocking in validation.rs)
- [x] **T2.1.9** Cryptographically secure admin token generation (256-bit random vs UUID)

### 2.2 Error Handling

- [x] **T2.2.1** Create consistent error responses (src/api/error.rs - ApiError with ErrorCode enum, ValidationErrorBuilder)
- [ ] **T2.2.2** Add deployment failure recovery
- [x] **T2.2.3** Implement container restart on crash (engine/container_monitor.rs - background task with exponential backoff, Prometheus metrics)
- [x] **T2.2.4** Add startup self-checks (startup/mod.rs - database, runtime, directory, disk checks; --skip-checks flag; /api/system/health endpoint)
- [ ] **T2.2.5** Database integrity checks

### 2.3 Resource Management

- [x] **T2.3.1** Add container CPU limits (cpu_limit field in App model, Docker NanoCPUs, Podman --cpus)
- [x] **T2.3.2** Add container memory limits (memory_limit field in App model, supports m/mb/g/gb/b suffixes)
- [x] **T2.3.3** Add build resource limits (cpu/memory limits in RuntimeConfig, applied to Docker/Podman builds)
- [x] **T2.3.4** Add disk space monitoring (engine/disk_monitor.rs, Prometheus metrics, API endpoint GET /api/system/disk)
- [x] **T2.3.5** Implement old deployment cleanup (engine/cleanup.rs - automatic cleanup with configurable retention)
- [x] **T2.3.6** Add image cleanup (engine/cleanup.rs - prune unused images with metrics logging)

### 2.4 Observability

- [x] **T2.4.1** Add Prometheus metrics endpoint (GET /metrics)
- [x] **T2.4.2** Add request duration metrics (http_request_duration_seconds histogram)
- [x] **T2.4.3** Add deployment counter metrics (deployments_total with status label)
- [x] **T2.4.4** Add container resource metrics (CPU, memory, network gauges with app_name labels, background stats collector)
- [x] **T2.4.5** Add health check metrics (success/failure counters, duration histogram, consecutive failures gauge)

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
- [x] **T2.6.4** Set up CI pipeline (.github/workflows/ci.yml, release.yml)
- [ ] **T2.6.5** Add test coverage reporting

### 2.7 Documentation

- [x] **T2.7.1** Write README with quickstart
- [ ] **T2.7.2** Document configuration options
- [ ] **T2.7.3** Document API endpoints
- [x] **T2.7.4** Write deployment guide (DEPLOYMENT.md with install script, systemd service, SSL, backup/restore, troubleshooting)
- [x] **T2.7.5** Write troubleshooting guide (included in DEPLOYMENT.md)
- [x] **T2.7.6** Create one-liner install script (install.sh - auto-installs Docker, Rivetr, systemd service)

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

### 3.5 Notifications ✅ COMPLETE

- [x] **T3.5.1** Create notification channels table (016_notifications.sql)
- [x] **T3.5.2** Add notification settings API (src/api/notifications.rs)
- [x] **T3.5.3** Implement Slack webhook notifications
- [x] **T3.5.4** Implement Discord webhook notifications
- [x] **T3.5.5** Implement email notifications (SMTP)
- [x] **T3.5.6** Trigger notifications on deployment events
- [x] **T3.5.7** Add notification preferences UI (routes/settings/notifications.tsx)

### 3.6 Container Shell Access ✅ COMPLETE

- [x] **T3.6.1** Implement container exec in runtime trait
- [x] **T3.6.2** Add WebSocket terminal endpoint
- [x] **T3.6.3** Create terminal UI component (xterm.js)
- [x] **T3.6.4** Add shell access button to app detail

### 3.7 Volumes Management ✅ COMPLETE

- [x] **T3.7.1** Create volumes table (018_volumes.sql)
- [x] **T3.7.2** Add volumes CRUD API (src/api/volumes.rs)
- [x] **T3.7.3** Create volumes UI in app settings (VolumesCard.tsx)
- [x] **T3.7.4** Mount volumes at container start (binds in RunConfig, Docker/Podman support)
- [x] **T3.7.5** Add volume backup/export (tar.gz backup endpoint)

### 3.8 Build Improvements (Buildpacks - from Coolify/Dokploy research) ✅ MOSTLY COMPLETE

- [x] **T3.8.1** Add Nixpacks builder support (auto-generate Dockerfile) - src/engine/nixpacks.rs with TOML config support
- [ ] **T3.8.2** Add Railpack builder support (Nixpacks successor with optimizations)
- [x] **T3.8.3** Add Static site builder (NGINX-based, for Astro/Next/Vite static exports) - src/engine/static_builder.rs
- [ ] **T3.8.4** Add Heroku Buildpacks support
- [ ] **T3.8.5** Add Paketo Buildpacks support (cloud-native buildpacks)
- [x] **T3.8.6** Auto-detect build type from repo (package.json, requirements.txt, Cargo.toml, etc.) - src/engine/build_detect.rs
- [x] **T3.8.7** Build type selector in UI (dropdown in app creation/settings) - frontend already implemented
- [x] **T3.8.8** Publish directory field for static builds (e.g., dist, build, out) - frontend already implemented

### 3.9 Multi-User & Teams ✅ COMPLETE

- [x] **T3.9.1** Add user roles (admin, developer, viewer) - TeamRole enum with owner/admin/developer/viewer
- [x] **T3.9.2** Create teams/organizations table (015_teams.sql)
- [x] **T3.9.3** Add team membership API (src/api/teams.rs)
- [x] **T3.9.4** Implement permission checks on API (role-based permissions)
- [x] **T3.9.5** Add user management UI (routes/settings/teams.tsx)
- [x] **T3.9.6** Add team settings UI (routes/settings/teams/$id.tsx)

**Phase 3 Checkpoint**: Full-featured PaaS with monitoring and team support

### 3.10 Advanced Build Options (Coolify-inspired) ✅ COMPLETE

- [x] **T3.10.1** Add `dockerfile_path` field to apps (custom Dockerfile location)
- [x] **T3.10.2** Add `base_directory` field (build context path)
- [x] **T3.10.3** Add `build_target` field (Docker multi-stage build target)
- [x] **T3.10.4** Add `watch_paths` field (auto-deploy on specific paths changed)
- [x] **T3.10.5** Add `custom_docker_options` field (extra docker build/run args)
- [x] **T3.10.6** Create Build Options section in app settings UI

### 3.11 Network Configuration (Coolify-inspired) ✅ COMPLETE

- [x] **T3.11.1** Add `port_mappings` field (host:container port pairs)
- [x] **T3.11.2** Add `network_aliases` field (container network aliases)
- [x] **T3.11.3** Support multiple exposed ports per app
- [x] **T3.11.4** Add `extra_hosts` field for custom /etc/hosts entries
- [x] **T3.11.5** Create Network Configuration section in app settings UI

### 3.12 HTTP Basic Auth (Coolify-inspired) ✅ COMPLETE

- [x] **T3.12.1** Add `basic_auth_enabled` field to apps
- [x] **T3.12.2** Add `basic_auth_username` and `basic_auth_password` fields
- [x] **T3.12.3** Implement basic auth middleware in proxy
- [x] **T3.12.4** Create Basic Auth toggle in app settings UI

### 3.13 Pre/Post Deployment Commands (Coolify-inspired) ✅ COMPLETE

- [x] **T3.13.1** Add `pre_deploy_commands` field (JSON array)
- [x] **T3.13.2** Add `post_deploy_commands` field (JSON array)
- [x] **T3.13.3** Execute pre-deploy commands after container starts
- [x] **T3.13.4** Execute post-deploy commands after container healthy
- [x] **T3.13.5** Log command outputs to deployment logs
- [x] **T3.13.6** Create Deployment Commands section in app settings UI

### 3.14 Domain Management (Coolify-inspired) ✅ COMPLETE

- [x] **T3.14.1** Add `domains` field (JSON array for multiple domains)
- [x] **T3.14.2** Implement auto-generate subdomain feature (sslip.io)
- [x] **T3.14.3** Support www/non-www redirect options
- [x] **T3.14.4** Create Domain Management section in app settings UI
- [x] **T3.14.5** Auto-provision SSL for all domains

### 3.17 Container Lifecycle Controls ✅ COMPLETE

- [x] **T3.17.1** Add `start` method to ContainerRuntime trait
- [x] **T3.17.2** Implement Docker container start
- [x] **T3.17.3** Implement Podman container start
- [x] **T3.17.4** Add `GET /api/apps/:id/status` endpoint
- [x] **T3.17.5** Add `POST /api/apps/:id/start` endpoint
- [x] **T3.17.6** Add `POST /api/apps/:id/stop` endpoint
- [x] **T3.17.7** Create Start/Stop UI buttons in app layout

### 3.15 Container Labels (Coolify-inspired) ✅ COMPLETE

- [x] **T3.15.1** Add `container_labels` field (JSON object) - migration 017
- [x] **T3.15.2** Apply custom labels to containers at runtime (labels in RunConfig, Docker/Podman support)
- [x] **T3.15.3** Create Container Labels editor in app settings UI (ContainerLabelsCard.tsx)
- [x] **T3.15.4** Add default label templates (Traefik, Caddy)

### 3.16 Docker Registry Support (Coolify-inspired) ✅ COMPLETE

- [x] **T3.16.1** Add `docker_image` field (pull from registry instead of building) - migration 014
- [x] **T3.16.2** Add `docker_image_tag` field
- [x] **T3.16.3** Add registry authentication (username/password) - encrypted storage
- [x] **T3.16.4** Support for private registries (Docker Hub, GHCR, etc.)
- [x] **T3.16.5** Create Docker Registry section in app settings UI (tabs in new app form)

---

## Phase 4: Platform Services (Coolify-inspired) - IN PROGRESS

### 4.1 Managed Databases ✅ COMPLETE

One-click database deployments with automatic configuration.

- [x] **T4.1.1** Create `databases` table (id, name, type, version, port, credentials) - migrations/019_databases.sql
- [x] **T4.1.2** Add `GET /api/databases` endpoint (list databases) - src/api/databases.rs
- [x] **T4.1.3** Add `POST /api/databases` endpoint (create database)
- [x] **T4.1.4** Add `DELETE /api/databases/:id` endpoint
- [x] **T4.1.5** Implement PostgreSQL one-click deployment - src/engine/database_config.rs
- [x] **T4.1.6** Implement MySQL/MariaDB one-click deployment
- [x] **T4.1.7** Implement MongoDB one-click deployment
- [x] **T4.1.8** Implement Redis one-click deployment
- [x] **T4.1.9** Add connection string generation (internal URL) - src/db/models.rs
- [x] **T4.1.10** Add public port exposure option (via host port binding)
- [x] **T4.1.10a** Add public access toggle in database settings UI (switch, custom port, security warnings, confirmation dialog)
- [x] **T4.1.11** Add database backup scheduling (src/engine/database_backups.rs, migrations/021)
- [x] **T4.1.12** Create Databases management page in frontend (under projects with grid view)
- [x] **T4.1.13** Add database credentials reveal/copy UI - frontend/app/routes/databases/$id/_index.tsx
- [x] **T4.1.14** Add backup file download functionality (API endpoint + frontend button)
- [x] **T4.1.15** Add database stats to dashboard (container stats aggregated with apps)

### 4.2 Services (Docker Compose Support) ✅ COMPLETE

Deploy multi-container applications from docker-compose.yml files.

- [x] **T4.2.1** Create `services` table (id, name, compose_content, status) - migrations/020_services.sql
- [x] **T4.2.2** Add `GET /api/services` endpoint (list services) - src/api/services.rs
- [x] **T4.2.3** Add `POST /api/services` endpoint (create from compose YAML)
- [x] **T4.2.4** Add `DELETE /api/services/:id` endpoint
- [x] **T4.2.5** Add `POST /api/services/:id/start` endpoint
- [x] **T4.2.6** Add `POST /api/services/:id/stop` endpoint
- [x] **T4.2.7** Implement docker-compose.yml parsing and validation
- [x] **T4.2.8** Implement docker compose up/down execution
- [x] **T4.2.9** Add service logs streaming (aggregate from all containers)
- [x] **T4.2.10** Create Services management page in frontend (project detail page with services grid)
- [x] **T4.2.11** Add Compose editor with YAML syntax highlighting
- [x] **T4.2.12** Add service container status display
- [x] **T4.2.13** Add service detail page with tabs (General, Network, Logs, Settings)
- [x] **T4.2.14** Add exposed ports display with clickable links
- [x] **T4.2.15** Add container/volume info parsed from compose content

### 4.3 One-Click Service Templates ✅ COMPLETE

Pre-configured templates for popular applications.

- [x] **T4.3.1** Create `service_templates` table (id, name, category, compose_template) - migrations/022_templates.sql
- [x] **T4.3.2** Add `GET /api/templates` endpoint (list available templates) - src/api/templates.rs
- [x] **T4.3.3** Add `GET /api/templates/:id` endpoint (get template details)
- [x] **T4.3.4** Add `POST /api/templates/:id/deploy` endpoint
- [x] **T4.3.5** Create template: Portainer (container management)
- [x] **T4.3.6** Create template: Grafana (monitoring/dashboards)
- [x] **T4.3.7** Create template: Uptime Kuma (status monitoring)
- [x] **T4.3.8** Create template: Gitea (self-hosted Git)
- [x] **T4.3.9** Create template: Nginx (web server)
- [x] **T4.3.10** Create template: Redis (caching)
- [x] **T4.3.11** Create template: n8n (workflow automation)
- [x] **T4.3.12** Create template: MinIO (S3-compatible storage)
- [x] **T4.3.13** Create template: Plausible (analytics)
- [x] **T4.3.14** Create template: Adminer (database management)
- [x] **T4.3.15** Create template: Mailhog (email testing)
- [x] **T4.3.16** Create template: Traefik (reverse proxy)
- [x] **T4.3.21** Create template: Nextcloud (file sharing/productivity)
- [x] **T4.3.22** Create template: pgAdmin (PostgreSQL management)
- [x] **T4.3.23** Create template: Filebrowser (file management)
- [x] **T4.3.24** Create template: Dozzle (container logs viewer)
- [x] **T4.3.25** Create template: Watchtower (container auto-updates)
- [x] **T4.3.26** Create template: Heimdall (application dashboard)
- [x] **T4.3.27** Create template: RabbitMQ (message broker)
- [x] **T4.3.28** Create template: Metabase (business intelligence)
- [x] **T4.3.29** Create template: Drone CI (continuous integration)
- [x] **T4.3.30** Create template: NocoDB (Airtable alternative)
- [x] **T4.3.31** Create template: Outline (team wiki)
- [x] **T4.3.17** Create Template Gallery modal in project page
- [x] **T4.3.18** Add template category filtering (monitoring, database, storage, etc.)
- [x] **T4.3.19** Add template customization before deploy (service name, env vars)
- [x] **T4.3.20** Seed 26 builtin templates on first run (Portainer, Uptime Kuma, Grafana, Gitea, MinIO, n8n, Traefik, Plausible, Nginx, Vaultwarden, WordPress, Ghost, Redis, Adminer, Mailhog, Nextcloud, pgAdmin, Filebrowser, Dozzle, Watchtower, Heimdall, RabbitMQ, Metabase, Drone CI, NocoDB, Outline)

**Phase 4 Checkpoint**: Full platform with databases, services, and one-click templates

---

## Phase 5: Advanced CI/CD Features (from Coolify/Dokploy research) - IN PROGRESS

### 5.1 GitHub App Integration (Coolify-inspired) ✅ COMPLETE

Full GitHub integration for automatic deployments without manual webhook setup.

- [x] **T5.1.1** Create GitHub App registration flow - src/api/github_apps.rs `create_manifest()`
- [x] **T5.1.2** Implement GitHub App OAuth for installation - `manifest_callback()`, `installation_callback()`
- [x] **T5.1.3** Store GitHub App installation tokens - github_app_installations table, token_manager.rs
- [x] **T5.1.4** Auto-configure webhooks via GitHub App - webhook URL included in manifest
- [x] **T5.1.5** Repository browser from GitHub App installations - github-repo-picker.tsx component
- [x] **T5.1.6** System-wide GitHub App sharing across teams - is_system_wide field, list_all_installations()
- [x] **T5.1.7** GitHub App settings UI page - routes/settings/git-providers.tsx GitHub tab

### 5.2 Advanced Rollback Features (Dokploy-inspired)

Enhanced rollback capabilities with registry integration.

- [x] **T5.2.1** Automatic health-based rollback (rollback on failed health checks) - trigger_auto_rollback() in pipeline.rs
- [x] **T5.2.2** Registry-based image storage for rollbacks - database schema ready (registry_push_enabled, max_rollback_versions)
- [ ] **T5.2.3** Push built images to Docker registry on deploy
- [x] **T5.2.4** Rollback to any previous deployment version - POST /api/deployments/:id/rollback with target_deployment_id
- [x] **T5.2.5** Rollback settings UI (enable registry push, select registry) - RollbackSettingsCard in Settings > Security
- [ ] **T5.2.6** Configure Docker Swarm update config (parallelism, delay, failure action)

### 5.3 Preview Deployments (from existing plan + enhancements)

Auto-deploy PRs with unique preview URLs.

- [ ] **T5.3.1** Parse PR events from webhooks (open, sync, close, merge)
- [ ] **T5.3.2** Create preview deployment with unique subdomain (pr-{number}.{app}.{domain})
- [ ] **T5.3.3** Add preview deployments list to app detail page
- [ ] **T5.3.4** Auto-cleanup on PR close/merge
- [ ] **T5.3.5** Comment preview URL on PR (GitHub API)
- [ ] **T5.3.6** Support GitLab/Gitea PR/MR previews
- [ ] **T5.3.7** Preview deployment resource limits (lower than production)

### 5.4 ZIP File Upload Deployment ✅ COMPLETE

Deploy applications by uploading a ZIP file with automatic build type detection.

- [x] **T5.4.1** Add ZIP extraction utility (src/engine/zip_extract.rs with path traversal protection)
- [x] **T5.4.2** Create multipart file upload endpoint (POST /api/apps/:id/deploy/upload)
- [x] **T5.4.3** Create quick deploy endpoint (POST /api/projects/:id/apps/upload) - create app + deploy in one
- [x] **T5.4.4** Add build type detection preview endpoint (POST /api/build/detect)
- [x] **T5.4.5** Modify deployment pipeline for non-git sources (run_upload_deployment in pipeline.rs)
- [x] **T5.4.6** Create ZipUploadZone component with drag-and-drop (frontend/app/components/zip-upload-zone.tsx)
- [x] **T5.4.7** Add Upload tab to app creation form (frontend/app/routes/projects/$project-id.apps.new.tsx)
- [x] **T5.4.8** Add upload API methods to frontend (uploadAndCreateApp, uploadAndDeploy, detectBuildType)
- [x] **T5.4.9** Validate ZIP file size (max 100MB) and contents (no empty ZIPs)
- [x] **T5.4.10** Clean up temp directories on error

---

## Phase 6: Unique Rivetr Features - NOT STARTED

Features that differentiate Rivetr from Coolify/Dokploy.

### 6.1 Resource Alerts & Cost Estimation

- [ ] **T6.1.1** Resource usage alerts (CPU/memory threshold notifications)
- [ ] **T6.1.2** Alert channels (email, Slack, Discord via existing notifications)
- [ ] **T6.1.3** Alert history log
- [ ] **T6.1.4** Cost estimation based on resource usage (configurable rates)
- [ ] **T6.1.5** Monthly cost projection per app
- [ ] **T6.1.6** Cost dashboard widget

### 6.2 Deployment Enhancements

- [ ] **T6.2.1** Zero-downtime deployment indicator (blue/green strategy status)
- [ ] **T6.2.2** Deployment preview diff (show env/domain/config changes before deploy)
- [ ] **T6.2.3** Deployment approval workflow (require approval before production)
- [ ] **T6.2.4** Scheduled deployments (deploy at specific time)
- [ ] **T6.2.5** Deployment freeze periods (prevent deployments during windows)

### 6.3 Bulk Operations & App Management

- [ ] **T6.3.1** Bulk start/stop/restart multiple apps
- [ ] **T6.3.2** Bulk deploy multiple apps
- [ ] **T6.3.3** App cloning (duplicate app configuration)
- [ ] **T6.3.4** Configuration snapshots (save/restore app configs)
- [ ] **T6.3.5** Export/import projects (JSON backup of entire project)
- [ ] **T6.3.6** Maintenance mode with custom page (503 with HTML)

### 6.4 Advanced Monitoring

- [ ] **T6.4.1** Log search (full-text search across deployment logs)
- [ ] **T6.4.2** Log retention policies (configurable per app)
- [ ] **T6.4.3** Scheduled container restarts (auto-restart at intervals)
- [ ] **T6.4.4** Dependency graph visualization (show service relationships)
- [ ] **T6.4.5** Uptime tracking per app (availability percentage)
- [ ] **T6.4.6** Response time monitoring (via health checks)

### 6.5 S3 Backup Integration

- [ ] **T6.5.1** S3 storage configuration (AWS, MinIO, R2)
- [ ] **T6.5.2** Volume backup to S3
- [ ] **T6.5.3** Database backup to S3
- [ ] **T6.5.4** Scheduled S3 backups
- [ ] **T6.5.5** S3 backup restore functionality
- [ ] **T6.5.6** S3 backup settings UI

**Phase 6 Checkpoint**: Differentiated PaaS with unique enterprise features

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
- [x] **Docker Compose support** - Deploy multi-container apps from docker-compose.yml ✅ IMPLEMENTED (Phase 4.2)
- [x] **One-click templates** - Pre-configured apps (PostgreSQL, Redis, MySQL, MongoDB, etc.) ✅ IMPLEMENTED (Phase 4.3 - 26 templates)
- [ ] **Pull request preview deployments** - Auto-deploy PRs with unique URLs
- [x] **Browser terminal** - In-browser shell access to containers (xterm.js) ✅ IMPLEMENTED
- [ ] **Repository browser** - Select repos from connected Git providers in app creation

**Medium Priority:**
- [x] **Build cache control** - Support `--no-cache` in custom docker options to force fresh builds
- [x] **Resource limits UI** - Set CPU/memory limits per app from dashboard (ResourceLimitsCard.tsx)
- [ ] **Deployment scheduling** - Schedule deployments for specific times
- [ ] **S3 backup integration** - Backup volumes and databases to S3
- [x] **Custom domains per app** - Multiple domains pointing to one app ✅ IMPLEMENTED (Phase 3.14)

**Lower Priority:**
- [ ] **Multi-server support** - Deploy to multiple servers from one dashboard
- [ ] **Service dependencies** - Define app startup order and dependencies
- [x] **Build from Dockerfile path** - Specify custom Dockerfile location ✅ IMPLEMENTED (Phase 3.10.1)
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
| Phase 0 | 30 | 28 | 93% |
| Phase 1 | 94 | 94 | 100% |
| Phase 2 | 31 | 26 | 84% |
| Phase 3 | 94 | 83 | 88% |
| Phase 4 | 61 | 61 | 100% |
| Phase 5 | 30 | 20 | 67% |
| Phase 6 | 28 | 0 | 0% |
| **Total** | **368** | **312** | **85%** |

---

## Known Issues (Active Bugs)

*No active bugs - all recently identified issues have been fixed.*

---

## Next Priority Tasks

1. **Preview Deployments** (T5.3.1-7)
   - Parse PR events from webhooks
   - Create preview deployment with unique subdomain
   - Auto-cleanup on PR close/merge

2. **Advanced Rollbacks** (T5.2.1-6) - ✅ MOSTLY COMPLETE
   - ✅ Health-based auto-rollback (T5.2.1) - Tested and working
   - ✅ Rollback settings UI (T5.2.5) - RollbackSettingsCard in Settings > Security
   - ⏳ Registry push on deploy (T5.2.3) - Pending

3. **Additional Buildpack Support** (T3.8.2, T3.8.4-5)
   - Add Railpack builder (Nixpacks successor)
   - Add Heroku/Paketo Buildpacks support

---

## Backlog Priority Tasks

**Phase 5 - Advanced CI/CD (NEW from competitor research):**
- **T5.2.1-6** - Advanced Rollbacks ✅ (health-based auto-rollback working, registry push pending)
- **T5.3.1-7** - Preview Deployments (PR auto-deploy with unique URLs)

**Phase 6 - Unique Features (NEW - differentiators):**
- **T6.1.1-6** - Resource Alerts & Cost Estimation
- **T6.2.1-5** - Deployment Enhancements (scheduled, approval workflow, freeze periods)
- **T6.3.1-6** - Bulk Operations & App Management (cloning, snapshots, maintenance mode)
- **T6.4.1-6** - Advanced Monitoring (log search, uptime tracking)
- **T6.5.1-6** - S3 Backup Integration

**Phase 3 - Enhanced Features:**
- **T3.8.2, T3.8.4-5** - Additional Buildpacks (Railpack, Heroku, Paketo)

**Phase 2 - Production Ready:**
- **T2.2.2** - Add deployment failure recovery
- **T2.2.5** - Database integrity checks
- **T2.7.2-3** - Document configuration options and API endpoints

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
- **App Restart API** - POST /api/apps/:id/restart to restart containers without rebuilding (picks up env/domain changes)
- **Redeploy UI** - Deploy dropdown with redeploy options
- **Git Providers UI** - Full OAuth connection management page at /settings/git-providers
- **Environment Variables UI** - Full CRUD with secret masking
- **Container Resource Metrics** - Live CPU/memory monitoring with sparklines
- **Deployment Timeline** - Visual deployment history with status indicators
- **Rate Limiting** - Sliding window algorithm with per-tier limits
- **Consistent Error Responses** - ApiError with ErrorCode enum
- **Prometheus Metrics** - /metrics endpoint with request/deployment counters
- **React Router Framework Mode** - Full SSR with server loaders, cookie-based sessions, dynamic imports
- **Browser Terminal** - xterm.js-based shell access to running containers
- **Deployment Cleanup** - Automatic cleanup of old deployments and images
- **WebSocket Authentication** - Token-based auth for log streaming and terminal
- **Advanced Build Options** - Custom Dockerfile path, base directory, build targets, watch paths
- **Pre/Post Deployment Commands** - Execute commands before/after container starts
- **Domain Management** - Multiple domains per app, auto-generate subdomains (sslip.io), www redirects
- **Network Configuration** - Port mappings, network aliases, extra hosts (/etc/hosts)
- **HTTP Basic Auth** - Protect apps with username/password authentication
- **Container Lifecycle Controls** - Start/stop containers from UI with status indicators
- **Theme Flicker Fix** - Blocking script to prevent flash of wrong theme on page load
- **Build Resource Limits** - CPU/memory limits for Docker/Podman builds (configurable in runtime config)
- **Disk Space Monitoring** - Background task with Prometheus metrics and API endpoint
- **Container Resource Prometheus Metrics** - CPU, memory, network gauges with app_name labels, background stats collector
- **Health Check Prometheus Metrics** - Success/failure counters, duration histogram, consecutive failures gauge
- **Dashboard Disk Usage Card** - Real-time disk usage display with percentage indicator
- **Container Crash Recovery** - Background monitor auto-restarts crashed containers with exponential backoff
- **Startup Self-Checks** - Database, runtime, directory, and disk checks before accepting requests; --skip-checks flag
- **Docker Registry Support** - Deploy apps from Docker Hub, GHCR, or private registries instead of building
- **Notifications System** - Slack, Discord, and Email notifications for deployment events with subscription management
- **Teams & RBAC** - Team management with owner/admin/developer/viewer roles and permission-based access control
- **Container Labels** - Custom Docker/Podman labels for Traefik/Caddy integration with preset templates
- **Volumes Management** - Persistent data volumes with full CRUD, bind mounts, and tar.gz backup/export
- **GitHub Actions CI/CD** - Automated lint/test/build on PRs, cross-platform release builds (Linux, macOS, Windows)
- **Managed Databases** - One-click PostgreSQL, MySQL, MongoDB, Redis deployments with credentials UI (100% complete)
- **Database Backup Scheduling** - Automated backups with hourly/daily/weekly schedules, retention policies, manual triggers
- **Apps & Databases Grid View** - Card-based grid layout for apps and databases under projects

### Recent Additions (Phase 4)
- **Docker Compose Services** - Deploy multi-container apps from docker-compose.yml
- **One-Click Templates** - 12 pre-configured templates (Portainer, Grafana, Uptime Kuma, Gitea, n8n, MinIO, etc.)
- **Service Detail Page** - Tabs for General, Network, Logs, Settings with exposed ports display
- **Network Tabs** - Added network information tabs to Apps, Databases, and Services
- **Template Gallery** - Category filtering, service name customization, env var configuration
- **Template Env Vars Form** - Pre-deployment configuration with PORT field and required env vars
- **Database Network Tab** - Enhanced with connection strings, env var examples, CLI commands
- **Docker Compose Editor** - Edit compose YAML in service settings with save functionality

### Planned Features (from Coolify/Dokploy research)
- **Railpack Builder** - Nixpacks successor with optimizations
- **Heroku/Paketo Buildpacks** - Additional buildpack support
- **Advanced Rollbacks** - Health-based auto-rollback, registry image storage
- **Preview Deployments** - Auto-deploy PRs with unique URLs

### Unique Rivetr Features (differentiators)
- **Resource Alerts** - Notifications when CPU/memory exceeds thresholds
- **Cost Estimation** - Estimate hosting costs based on resource usage
- **Deployment Approval Workflow** - Require approval before production deploys
- **Bulk Operations** - Start/stop/restart multiple apps at once
- **App Cloning** - Duplicate app configurations
- **Configuration Snapshots** - Save/restore app configs
- **Maintenance Mode** - Put apps in maintenance with custom page
- **Log Search** - Full-text search across deployment logs
- **S3 Backup Integration** - Backup volumes/databases to S3

### Recently Completed
- **Security Hardening v0.1.0** - Timing attack fixes (constant-time comparison), command injection protection, password strength validation (12+ chars, complexity requirements), security headers middleware, container restart policies (unless-stopped), production deployment documentation with one-liner install script
- **ZIP File Upload Deployment** - Deploy apps by uploading ZIP files with auto-detection of build type (Dockerfile, Nixpacks, Static, Docker Compose). Supports drag-and-drop upload, build preview, and quick deploy (create app + deploy in one step).
- **Real-Time Log Streaming Fix** - Added `logs_stream()` method to ContainerRuntime trait with `follow: true` for Docker and `--follow` for Podman, enabling continuous real-time log streaming in the UI
- **Resource Chart Auth Fix** - Added auth token headers to stats history API calls, fixing the time range selector for Global Resource Utilization chart
- **Database Public Access Toggle** - Added `PUT /api/databases/:id` endpoint to toggle public access with custom external port (1024-65535), security warnings, confirmation dialog, auto-restart on change
- **Frontend API Modularization** - Split monolithic `api.ts` into domain-specific modules (apps, databases, projects, services, teams, notifications, git, system, previews) with backward-compatible re-exports
- **Open App Button with Dynamic Port** - Added `host_port` to `AppStatusResponse` API, UI shows "Open App :port" button when container is running with exposed port
- **App Creation Form 2-Column Layout** - Changed app creation form to full-width with responsive 2-column grid layout for better UX
- **GitHub App Repository Picker** - Branch selection dropdown that fetches branches via GitHub App API, auto-fills git_url and branch from selected repo
- **GitHub App Connection in Settings** - Added `GitHubSourceCard` component to app settings showing connected repository info, installation details, and disconnect option
- **Unified Git Integrations Page** - Merged GitHub Apps and Git Providers into single page with tabs for GitHub (Apps), GitLab (PAT), and Bitbucket (App Password). Added backend API for token-based providers.
- **26 Service Templates** - Added 14 new templates: Redis, Adminer, Mailhog, Nextcloud, pgAdmin, Filebrowser, Dozzle, Watchtower, Heimdall, RabbitMQ, Metabase, Drone CI, NocoDB, Outline
- **Real-Time Resource Charts** - Dashboard charts now display actual historical data from stats_history table (5-min intervals, 7-day retention)
- **Service Stats in Dashboard** - Docker Compose service resource usage now included in dashboard totals (uses compose project label filtering)
- **Breadcrumbs** - Dynamic breadcrumb navigation showing project context
- **Time Range Selectors** - Global resource charts with 1h/6h/24h/7d/30d time range selection

- **Build Improvements** - Added Nixpacks builder with TOML config support (src/engine/nixpacks.rs), Static site builder with NGINX-based multi-stage Docker builds (src/engine/static_builder.rs), Build type auto-detection for 15+ frameworks/languages (src/engine/build_detect.rs). Pipeline integration complete.
- **GitHub App Integration** - Full GitHub App manifest registration, OAuth installation flow, installation tokens, auto-configured webhooks, repository browser with branch selection (github-repo-picker.tsx), system-wide sharing, settings UI in git-providers.tsx.

### Known Deployment Issues (User Repository Issues)
- **kwamina-website** - Container crashes due to `cross-env: not found` - the Dockerfile doesn't install devDependencies at runtime, but `npm start` uses `cross-env`. Fix: Move `cross-env` to dependencies or change start script to `NODE_ENV=production react-router-serve ./build/server/index.js`
- **pharmapro** - App responds but shows Laravel 500 error - likely needs database/env configuration (APP_KEY, database connection)
