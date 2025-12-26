# Rivetr - Development Phases

## Overview

```
Phase 0: Foundation        ████████████████████  ✅ Complete (83%)
Phase 1: Core Engine       ██████████████████░░  🔄 In Progress (89%)
Phase 2: Production Ready  ░░░░░░░░░░░░░░░░░░░░  ⏳ Not Started
Phase 3: Advanced          ░░░░░░░░░░░░░░░░░░░░  ⏳ Future
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

## Phase 1: Core Engine (MVP) - IN PROGRESS

> **Goal**: Deploy apps from git webhooks with working proxy

### 1.1 Container Runtime Abstraction ✅ COMPLETE
- [x] Define `ContainerRuntime` trait
- [x] Implement `DockerRuntime` with Bollard
  - [x] Build images
  - [x] Run containers
  - [x] Stop/remove containers
  - [x] Stream logs
  - [ ] Get container stats
- [x] Implement `PodmanRuntime` CLI wrapper
- [x] Add runtime auto-detection

### 1.2 Git Operations (Partial)
- [x] Implement repository cloning (git2)
- [ ] Handle SSH authentication
- [ ] Handle HTTPS authentication
- [x] Clone to temporary directories
- [ ] Cleanup after build

### 1.3 Deployment Pipeline ✅ COMPLETE
- [x] Create deployment state machine
- [x] Implement clone step
- [x] Implement build step (Dockerfile)
- [x] Implement container start step
- [x] Implement health check step
- [x] Implement traffic switch step
- [ ] Add rollback capability
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
  - [ ] `POST /api/deployments/:id/rollback` - Rollback
- [x] **Logs API**
  - [x] `GET /api/deployments/:id/logs` - Get build logs
  - [ ] `GET /api/apps/:id/logs` - Stream runtime logs (WebSocket)
- [x] **Webhooks API**
  - [x] `POST /webhooks/github` - GitHub webhook
  - [x] `POST /webhooks/gitlab` - GitLab webhook
  - [x] `POST /webhooks/gitea` - Gitea webhook

### 1.5 Embedded Reverse Proxy - IN PROGRESS
- [x] Implement HTTP proxy with Hyper
- [x] Create route table with ArcSwap
- [x] Add dynamic route updates
- [x] Implement request forwarding
- [ ] Add WebSocket proxying
- [ ] Implement HTTPS with rustls
- [ ] Add ACME/Let's Encrypt integration
- [ ] Auto-renew certificates
- [ ] Handle multiple domains per app

### 1.6 Dashboard UI ✅ COMPLETE (React + shadcn/ui)
- [x] Set up Vite + React + TypeScript
- [x] Initialize shadcn/ui components
- [x] Configure Tailwind CSS v4
- [x] **Pages**
  - [x] Login page with token auth
  - [x] Dashboard home (stats cards)
  - [x] App detail page (deployments table)
  - [x] Apps list page
  - [x] Create app form
  - [x] Settings page (placeholder)
- [x] **React Features**
  - [x] React Query for data fetching
  - [x] React Router for navigation
  - [x] Protected routes (AuthProvider)
  - [x] Delete confirmation dialogs
  - [ ] Live deployment status polling
  - [ ] Real-time log streaming (WebSocket)
- [x] Static file serving (tower-http)

### 1.7 Health Checks (Partial)
- [x] HTTP health check implementation
- [x] Configurable timeout and retries
- [ ] Health status in proxy routing
- [ ] Unhealthy container handling

### Milestone: Deploy an app via GitHub webhook, access via domain

---

## Phase 2: Production Ready

> **Goal**: Stable, secure, and user-friendly

### 2.1 Security Hardening
- [ ] Input validation on all endpoints
- [ ] Rate limiting
- [ ] CSRF protection for UI
- [ ] Encrypt secrets at rest (env vars)
- [ ] Secure cookie handling
- [ ] Audit logging

### 2.2 Error Handling & Recovery
- [ ] Graceful error responses
- [ ] Deployment failure recovery
- [ ] Container crash detection and restart
- [ ] Database corruption recovery
- [ ] Startup health checks

### 2.3 Resource Management
- [ ] Container CPU limits
- [ ] Container memory limits
- [ ] Build resource limits
- [ ] Disk space monitoring
- [ ] Old deployment cleanup

### 2.4 Observability
- [ ] Prometheus metrics endpoint
- [ ] Request duration metrics
- [ ] Deployment metrics
- [ ] Container resource metrics
- [ ] Health check metrics

### 2.5 CLI Improvements
- [ ] `rivetr status` - Show status
- [ ] `rivetr apps list` - List apps
- [ ] `rivetr deploy <app>` - Manual deploy
- [ ] `rivetr logs <app>` - Tail logs
- [ ] `rivetr config check` - Validate config

### 2.6 Documentation
- [ ] README with quickstart
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

## Phase 3: Advanced Features (Future)

> **Goal**: Extended capabilities for power users

### 3.1 Deployment Strategies
- [ ] Blue-green deployments
- [ ] Canary deployments
- [ ] Manual promotion gates

### 3.2 Native Process Support
- [ ] Run apps without containers (systemd)
- [ ] Binary deployment support
- [ ] Hybrid container/native mode

### 3.3 Database Management
- [ ] PostgreSQL provisioning
- [ ] MySQL provisioning
- [ ] Automatic backups
- [ ] Connection string injection

### 3.4 Multi-Node (Agent Mode)
- [ ] Agent binary mode
- [ ] Controller-agent communication
- [ ] Agent discovery
- [ ] Load balancing across nodes

### 3.5 Advanced Proxy
- [ ] Migrate to Pingora
- [ ] Load balancing algorithms
- [ ] Request caching
- [ ] Custom headers/middleware

### 3.6 Plugin System
- [ ] Plugin API definition
- [ ] Lua scripting support
- [ ] Build hooks
- [ ] Deploy hooks
- [ ] Custom health checks

### 3.7 Backups & Restore
- [ ] Scheduled config backups
- [ ] App data backups
- [ ] Point-in-time restore
- [ ] Export/import apps

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
