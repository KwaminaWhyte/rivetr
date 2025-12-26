# Rivetr - Development Phases

## Overview

```
Phase 0: Foundation        ████████████████████  ✅ Complete (83%)
Phase 1: Core Engine       ████████████████████  ✅ Complete (100%)
Phase 2: Production Ready  ██████░░░░░░░░░░░░░░  🔄 In Progress (32%)
Phase 3: Advanced          ████████░░░░░░░░░░░░  🔄 In Progress (40%)
```

---

## Phase 0: Foundation ✅ COMPLETE

> **Goal**: Bootable project with core infrastructure

### 0.1 Project Setup
- [x] Initialize Cargo workspace
- [x] Configure Cargo.toml with all dependencies
- [x] Set up project directory structure
- [ ] Configure rustfmt and clippy
- [x] Create .gitignore

### 0.2 Database Foundation
- [x] Set up SQLx with SQLite
- [x] Create initial migrations (apps, deployments, logs tables)
- [x] Implement database connection pool
- [x] Add WAL mode configuration

### 0.3 Configuration System
- [x] Define config schema (rivetr.toml)
- [x] Implement TOML config parsing
- [x] Add environment variable overrides
- [x] Create default configuration

### 0.4 Logging & Tracing
- [x] Set up tracing subscriber
- [x] Configure log levels
- [x] Add request tracing middleware

### Milestone: ✅ `cargo run` starts server, connects to SQLite

---

## Phase 1: Core Engine (MVP) ✅ COMPLETE

> **Goal**: Deploy apps from git webhooks with working proxy

### 1.1 Container Runtime Abstraction ✅ COMPLETE
- [x] Define `ContainerRuntime` trait
- [x] Implement `DockerRuntime` with Bollard
  - [x] Build images
  - [x] Run containers
  - [x] Stop/remove containers
  - [x] Stream logs
  - [x] Get container stats
- [x] Implement `PodmanRuntime` CLI wrapper
- [x] Add runtime auto-detection

### 1.2 Git Operations ✅ COMPLETE
- [x] Implement repository cloning (git2)
- [x] Handle SSH authentication
- [x] Clone to temporary directories

### 1.3 Deployment Pipeline ✅ COMPLETE
- [x] Create deployment state machine
- [x] Implement clone step
- [x] Implement build step (Dockerfile)
- [x] Implement container start step
- [x] Implement health check step
- [x] Implement traffic switch step
- [x] Add rollback capability
- [x] Store deployment logs

### 1.4 REST API ✅ COMPLETE
- [x] Set up Axum router
- [x] Implement authentication middleware
- [x] **Apps API**
  - [x] `POST /api/apps` - Create app
  - [x] `GET /api/apps` - List apps
  - [x] `GET /api/apps/:id` - Get app details
  - [x] `PUT /api/apps/:id` - Update app
  - [x] `DELETE /api/apps/:id` - Delete app
- [x] **Deployments API**
  - [x] `POST /api/apps/:id/deploy` - Trigger deploy
  - [x] `GET /api/apps/:id/deployments` - List deployments
  - [x] `GET /api/deployments/:id` - Get deployment status
  - [x] `POST /api/deployments/:id/rollback` - Rollback
- [x] **Logs API**
  - [x] `GET /api/deployments/:id/logs` - Get build logs
  - [x] `GET /api/apps/:id/logs/stream` - Stream runtime logs (WebSocket)
- [x] **Webhooks API**
  - [x] `POST /webhooks/github` - GitHub webhook
  - [x] `POST /webhooks/gitlab` - GitLab webhook
  - [x] `POST /webhooks/gitea` - Gitea webhook

### 1.5 Embedded Reverse Proxy ✅ COMPLETE
- [x] Implement HTTP proxy with Hyper
- [x] Create route table with ArcSwap
- [x] Add dynamic route updates
- [x] Implement request forwarding
- [x] Add WebSocket proxying
- [x] Implement HTTPS with rustls
- [x] Add ACME/Let's Encrypt integration
- [x] Auto-renew certificates
- [x] Route management API

### 1.6 Dashboard UI ✅ COMPLETE (React + shadcn/ui)
- [x] Set up Vite + React + TypeScript
- [x] Initialize shadcn/ui components
- [x] Configure Tailwind CSS v4
- [x] **Pages**
  - [x] Login page with token auth
  - [x] Dashboard home (stats cards, recent events)
  - [x] App detail page (deployments, env vars, settings)
  - [x] Projects list and detail pages
  - [x] Create app form (project-centric flow)
  - [x] Settings pages (general, git providers, SSH keys)
- [x] **React Features**
  - [x] React Query for data fetching
  - [x] React Router for navigation
  - [x] Protected routes (AuthProvider)
  - [x] Delete confirmation dialogs
  - [x] Live deployment status polling
  - [x] Real-time log streaming (WebSocket)
  - [x] Theme switching (light/dark/system)
- [x] Static file serving (tower-http)

### 1.7 Health Checks ✅ COMPLETE
- [x] HTTP health check implementation
- [x] Configurable timeout and retries
- [x] Health status in proxy routing
- [x] Health checker background task

### 1.8 Git Provider Integration ✅ COMPLETE
- [x] OAuth flow for GitHub, GitLab, Bitbucket
- [x] Git providers CRUD API
- [x] Repository listing from connected providers
- [x] Git Providers settings page in UI

### Milestone: ✅ Full deployment from GitHub webhook with working UI

---

## Phase 2: Production Ready - IN PROGRESS

> **Goal**: Stable, secure, and user-friendly

### 2.1 Security Hardening (Partial)
- [x] Input validation on all endpoints (src/api/validation.rs)
- [x] Rate limiting (sliding window algorithm, per-tier limits)
- [ ] CSRF protection for UI
- [ ] Encrypt secrets at rest (env vars)
- [ ] Secure cookie handling
- [ ] Audit logging

### 2.2 Error Handling & Recovery (Partial)
- [x] Graceful error responses (ApiError with ErrorCode enum)
- [ ] Deployment failure recovery
- [ ] Container crash detection and restart
- [ ] Database corruption recovery
- [ ] Startup health checks

### 2.3 Resource Management (Partial)
- [x] Container CPU limits (cpu_limit field, Docker NanoCPUs, Podman --cpus)
- [x] Container memory limits (memory_limit field with m/mb/g/gb/b suffixes)
- [ ] Build resource limits
- [ ] Disk space monitoring
- [ ] Old deployment cleanup

### 2.4 Observability (Partial)
- [x] Prometheus metrics endpoint (GET /metrics)
- [x] Request duration metrics (http_request_duration_seconds histogram)
- [x] Deployment metrics (deployments_total with status label)
- [ ] Container resource metrics
- [ ] Health check metrics

### 2.5 CLI Improvements
- [ ] `rivetr status` - Show status
- [ ] `rivetr apps list` - List apps
- [ ] `rivetr deploy <app>` - Manual deploy
- [ ] `rivetr logs <app>` - Tail logs
- [ ] `rivetr config check` - Validate config

### 2.6 Documentation (Partial)
- [x] README with quickstart
- [ ] Configuration reference
- [ ] API documentation
- [ ] Deployment guide
- [ ] Troubleshooting guide

### 2.7 Testing
- [ ] Unit tests for core components
- [ ] Integration tests for API
- [ ] E2E tests for deployment flow
- [ ] CI/CD pipeline

### Milestone: Production-ready single binary

---

## Phase 3: Advanced Features - IN PROGRESS

> **Goal**: Extended capabilities for power users

### 3.1 Project Organization (Partial)
- [x] Environment field for apps (development/staging/production)
- [x] Environment badges with color coding
- [x] Environment filter on apps list
- [x] Projects feature - group related apps
- [x] Project-centric navigation flow
- [ ] Tags/labels for apps

### 3.2 Environment Variables UI ✅ COMPLETE
- [x] Env vars CRUD API with reveal option
- [x] Env vars settings tab in app detail
- [x] Key-value editor with add/edit/delete dialogs
- [x] Multiline value support
- [x] Secret value masking with reveal button
- [x] Pass env vars to container at runtime

### 3.3 Resource Monitoring (Partial)
- [x] Container stats collection (CPU, memory, network)
- [x] Container stats API endpoint
- [x] Resource usage graphs with sparklines
- [ ] Store metrics history in SQLite
- [ ] System-wide dashboard metrics

### 3.4 Deployment Strategies
- [ ] Blue-green deployments
- [ ] Canary deployments
- [ ] Manual promotion gates

### 3.5 Preview Deployments
- [ ] Parse PR events from webhooks
- [ ] Create preview deployment with unique subdomain
- [ ] Auto-cleanup on PR close/merge
- [ ] Comment preview URL on PR

### 3.6 Notifications
- [ ] Notification channels (Slack, Discord, Email)
- [ ] Notification settings API
- [ ] Trigger notifications on deployment events
- [ ] Notification preferences UI

### 3.7 Container Shell Access
- [ ] Container exec in runtime trait
- [ ] WebSocket terminal endpoint
- [ ] Terminal UI component (xterm.js)

### 3.8 Volumes Management
- [ ] Volumes CRUD API
- [ ] Volumes UI in app settings
- [ ] Mount volumes at container start
- [ ] Volume backup/export

### 3.9 Multi-User & Teams
- [ ] User roles (admin, developer, viewer)
- [ ] Teams/organizations
- [ ] Permission checks on API
- [ ] User management UI

---

## Phase Dependencies

```
Phase 0 ──► Phase 1.1 ──► Phase 1.3
              │              │
              ▼              ▼
         Phase 1.2      Phase 1.5
              │              │
              └──────┬───────┘
                     ▼
               Phase 1.4
                     │
                     ▼
               Phase 1.6
                     │
                     ▼
               Phase 1.7
                     │
                     ▼
               Phase 2.x
```

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Bollard API changes | Pin version, write adapter layer |
| ACME rate limits | Use staging in dev, cache certs |
| Large repo clones | Shallow clone, timeout limits |
| Container resource exhaustion | Build-time limits, quotas |
| SQLite write contention | WAL mode, connection pooling |
