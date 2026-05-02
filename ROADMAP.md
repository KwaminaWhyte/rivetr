# Rivetr Roadmap

> A fast, lightweight deployment engine built in Rust

This document outlines the planned development roadmap for Rivetr. For detailed task tracking, see [docs/TASKS.md](./docs/TASKS.md).

## Current Status

**Overall Progress: 599/599 tasks complete**

| Phase | Status | Progress |
|-------|--------|----------|
| Phase 0: Foundation | Complete | 93% |
| Phase 1: Core Engine (MVP) | Complete | 100% |
| Phase 2: Production Ready | Complete | 100% |
| Phase 3: Enhanced Features | Complete | 100% |
| Phase 4: Platform Services | Complete | 100% |
| Phase 5: Advanced CI/CD | Complete | 100% |
| Phase 6: Unique Features | Complete | 100% |
| Phase 7: Competitive Parity | Complete | 100% |
| Phase 8: Enterprise & Scale | Complete | 100% |

---

## Released Features (v0.2.x)

### Core Deployment Engine
- Git deployments from GitHub, GitLab, Gitea with webhook signature verification
- Multiple build types: Dockerfile, Nixpacks, Railpack, Heroku/Paketo buildpacks, static sites
- Docker and Podman runtime support with auto-detection
- Zero-downtime deployments with health checks and automatic rollback
- Real-time build and runtime log streaming via WebSocket

### Platform Services
- One-click managed databases (PostgreSQL, MySQL, MariaDB, MongoDB, Redis, DragonFlyDB, KeyDB, ClickHouse)
- Docker Compose multi-container deployments with raw mode, preview, and magic variables
- 285 pre-configured service templates (Grafana, Portainer, Uptime Kuma, Gitea, n8n, Memos, Beszel, AnythingLLM, Pi-hole, Nextcloud, Plex, PocketBase, Appwrite, Directus, Authentik, MinIO, and many more — see [docs/SERVICE-TEMPLATES.md](./docs/SERVICE-TEMPLATES.md))
- Port conflict validation across services and databases (real-time frontend checks + server-side enforcement)
- Auto-subdomain assignment for template-deployed services
- Automated database backup scheduling with retention policies

### Team Collaboration
Full multi-tenant team support with resource isolation.

- **Team Switching** - Switch between teams with persistent context
- **Resource Scoping** - Apps, projects, databases, and services scoped to teams
- **Team Invitations** - Email-based invitation system with 7-day expiry
- **Audit Logging** - Complete activity tracking for team operations
- **App Sharing** - Share apps between teams with view permissions
- **Member Management** - Role changes and member removal with role hierarchy
- **Team-scoped Stats** - Dashboard statistics filtered by current team
- **Migration CLI** - `rivetr db migrate-teams` to migrate legacy resources

### Security & Operations
- HTTPS with automatic Let's Encrypt certificates and auto-renewal
- Team management with RBAC (owner/admin/developer/viewer roles)
- Rate limiting, input validation, and security headers
- Container crash recovery with exponential backoff
- Prometheus metrics endpoint for monitoring

### Developer Experience
- Modern React + TypeScript dashboard with SSR
- ZIP file upload deployment with build type auto-detection
- Browser-based terminal access to containers (xterm.js)
- GitHub App integration for seamless repository access
- Environment variables with encrypted secret storage
- **Single binary deployment** - Frontend embedded in binary, no external files needed
- **One-liner production install** - `curl | bash` installs Docker, Rivetr, and systemd service
- **Ansible playbook** - `ansible/rivetr.yml` for automated server provisioning (Ubuntu/Debian)
- **Container resource limits** - Per-app CPU/memory limits with live apply via `docker update` (no redeploy)

---

## In Progress (v0.3.x)

### Preview Deployments
Automatic PR preview environments with unique URLs.

- [x] Parse PR events from webhooks (open, sync, close, merge) ✅
- [x] Create preview deployment with unique subdomain (`pr-{number}.{app}.{domain}`) ✅
- [x] Auto-cleanup on PR close/merge ✅
- [x] Post preview URL as comment on PR (GitHub API) ✅
- [x] Support GitLab/Gitea MR previews ✅

### Advanced Rollbacks
Enhanced rollback with registry integration.

- [x] Automatic health-based rollback
- [x] Rollback settings UI
- [x] Push built images to Docker registry on deploy ✅
- [x] Configure rollback retention policies ✅

---

## Planned (v0.4.x - Unique Features)

### Resource Alerts & Cost Estimation ✅ COMPLETE
- [x] CPU/memory threshold alerts
- [x] Alert channels (email, Slack, Discord, webhooks)
- [x] Cost estimation based on resource usage
- [x] Daily cost snapshots per app
- [x] Team costs API for aggregate reporting

### Deployment Enhancements
- [ ] Deployment preview diff (show changes before deploy)
- [x] Approval workflow for production deployments ✅
- [x] Scheduled deployments (deploy at specific time) ✅
- [x] Deployment freeze periods ✅
- [x] Zero-downtime indicator (blue/green status) ✅

### Bulk Operations & App Management ✅ COMPLETE
- [x] Bulk start/stop/restart multiple apps ✅
- [x] Bulk deploy multiple apps ✅
- [x] App cloning (duplicate configuration) ✅
- [x] Configuration snapshots (save/restore) ✅
- [x] Export/import projects (JSON backup) ✅
- [x] Maintenance mode with custom page ✅

### Advanced Monitoring ✅ COMPLETE
- [x] Full-text log search ✅
- [x] Configurable log retention policies ✅
- [x] Scheduled container restarts ✅
- [x] Service dependency graph visualization ✅
- [x] Uptime tracking per app ✅
- [x] Response time monitoring ✅

### S3 Backup Integration ✅ COMPLETE
- [x] S3 storage configuration (AWS, MinIO, R2, custom endpoints) ✅
- [x] Volume backup to S3 ✅
- [x] Database backup to S3 ✅
- [x] Scheduled S3 backups ✅
- [x] One-click restore from S3 ✅

---

## Planned (v0.5.x - Competitive Parity)

Features that both Coolify and Dokploy have. Required to compete.

### OAuth Login ✅ COMPLETE
- [x] GitHub OAuth login
- [x] Google OAuth login
- [x] OAuth provider configuration in settings UI
- [x] Account linking (connect OAuth to existing account)

### Project Environments ✅ COMPLETE
- [x] Environment model (dev/staging/production per project)
- [x] Environment-level environment variables
- [x] Environment switching in UI
- [x] Environment-scoped deployments
- [x] Predefined variables per environment (RIVETR_ENV, RIVETR_URL, etc.)

### Watch Paths ✅ COMPLETE
- [x] Watch path patterns per app (e.g., `src/*`, `package.json`)
- [x] Filter webhook deploys by changed files
- [x] Watch path configuration in app settings UI

### Bitbucket & DockerHub Webhooks ✅ COMPLETE
- [x] Bitbucket webhook signature verification ✅
- [x] Bitbucket push/PR event parsing ✅
- [x] DockerHub webhook (deploy on image push) ✅

### Additional Notification Channels
- [x] Telegram notifications (bot API) ✅
- [x] Microsoft Teams notifications (incoming webhooks) ✅
- [x] Pushover notifications ✅
- [x] Ntfy notifications ✅
- [x] Notification channel UI for new providers ✅

### Service Templates Expansion ✅ COMPLETE
- [x] Expand from 26 to 74 templates (AI/ML, Analytics, Automation, CMS, Communication, Dev Tools, Documentation, Media, Monitoring, Security, Search, PM) ✅
- [x] Template categories (Ai, Analytics, Automation, Cms, Communication, Development, Documentation, Media, Monitoring, Security, Search, ProjectManagement) ✅
- [x] Template search and filtering ✅
- [x] Community template submissions ✅ (suggestion flow + pending approval; admin review UI pending)

### Instance Backup & Restore ✅ COMPLETE
- [x] Full instance backup (SQLite DB + config + SSL certs) ✅
- [x] Scheduled instance backups ✅
- [x] One-click instance restore ✅
- [x] Instance backup to S3 ✅

### Scheduled Jobs ✅ COMPLETE
- [x] Cron-based job scheduling per app ✅
- [x] Execute commands inside containers on schedule ✅
- [x] Background cron scheduler with 60-second polling ✅
- [x] Job execution history and logs ✅
- [x] Job management UI ✅

### Container Replicas ✅ COMPLETE
- [x] Configurable replica count per app ✅
- [x] Load balancing across replicas (round-robin) ✅
- [x] Replica health monitoring ✅
- [x] Scale up/down from UI ✅

### Deploy by Commit/Tag ✅ COMPLETE
- [x] Deploy specific Git commit by SHA ✅
- [x] Deploy specific Git tag ✅
- [x] Commit/tag selector in deploy UI ✅
- [x] API endpoints for commit/tag deploy ✅

---

## Planned (v0.6.x - Enterprise & Scale)

Features required for enterprise adoption and high availability.

### Multi-Server Support ✅ COMPLETE
- [x] Remote server registration via SSH ✅
- [x] Server health monitoring from dashboard ✅
- [x] Server-level resource monitoring (CPU/memory/disk) ✅
- [x] Deploy apps to specific servers ✅
- [x] Remote server terminal access ✅
- [x] File system browser for remote servers ✅

### SSO / SAML / OIDC ✅ PARTIAL
- [x] OpenID Connect (OIDC) provider integration ✅
- [x] Auth0, Keycloak, Azure AD, Okta compatibility ✅
- [x] SSO configuration UI ✅
- [x] Per-team SSO provider settings ✅
- [ ] SAML 2.0 support

### Two-Factor Authentication (2FA) ✅ COMPLETE
- [x] TOTP-based 2FA (Google Authenticator, Authy) ✅
- [x] 2FA setup flow with QR code ✅
- [x] Recovery codes ✅
- [x] 2FA enforcement per team ✅

### Log Draining ✅ COMPLETE
- [x] Drain logs to Axiom ✅
- [x] Drain logs to New Relic ✅
- [x] Drain logs to Datadog ✅
- [x] Drain logs to Logtail ✅
- [x] Per-app log drain configuration ✅
- [x] Log drain settings UI ✅

### Docker Swarm Integration ✅ COMPLETE
- [x] Swarm mode initialization ✅
- [x] Worker/manager node management ✅
- [x] Service scaling across nodes ✅
- [ ] Overlay networking (future)
- [ ] Rolling updates with Swarm (future)

### Build Servers ✅ COMPLETE
- [x] Dedicated build server registration (separate from deploy) ✅
- [x] Build server health monitoring ✅
- [x] RemoteContext SSH foundation for remote builds ✅
- [ ] Full remote build execution (future)

### Shared Environment Variables ✅ COMPLETE
- [x] Team-level shared variables ✅
- [x] Project-level shared variables ✅
- [x] Environment-level shared variables ✅
- [x] Variable inheritance hierarchy (team > project > env > app) ✅
- [x] Shared variable management UI ✅

---

## Recent Bug Fixes (Unreleased)

- **v0.10.20 — Coolify-style deploy log side panel** ✅ — A dockable side panel auto-opens on Deploy/Start/Restart and live-streams image-pull + container-start logs for apps, services, and managed databases (new `StartLogRegistry` + WS/REST routes; `DeployPanelProvider` mounted at the dashboard root)
- **v0.10.20 — MariaDB managed database type** ✅ — Frontend now wires MariaDB through the `mariadb://...` MySQL scheme, `/var/lib/mysql` data path, and `mariadb-dump` backups; supports versions 11 (default), 10.11, 10.6, 10.5; backend covered by `test_mariadb_config` and `test_generate_env_vars_mariadb`
- **v0.10.20 — Container monitor service health check broken since v0.10.18 (HIGH)** ✅ — `check_services` SELECT was missing migration-105 columns (`public_access`, `external_port`, `expose_container_port`); compose service crash detection was silently disabled. SELECT now lists the full column set
- **v0.10.20 — `/api/apps/:id/insights` 503 noise** ✅ — Endpoint now returns 404 when no AI provider is configured (was 503), eliminating misleading browser console errors and `tower_http` warn spam
- **Stale `container_id` on destroyed database container** ✅ — Engine now detects when a database's recorded container no longer exists, clears the stale ID, resets status to `stopped`, and provisions a fresh container instead of getting stuck in `starting`
- **Reconciliation queries break on new migrations** ✅ — `reconcile_databases` / `reconcile_services` now use `SELECT *` to avoid `no column found` errors when new columns are added
- **5-field cron expressions for scheduled restarts** ✅ — Standard Unix cron strings (5 fields) now normalized to 6-field format required by the cron crate
- **Deployment cleanup settings configurable from UI** ✅ — `max_deployments_per_app` and `prune_images` settable from Settings page without editing `rivetr.toml`
- **MySQL/MariaDB user provisioning on reused data directories** ✅ — Rivetr now idempotently creates the app user via Unix socket after every container start, preventing SQLSTATE[1130] when a bind-mount data directory was pre-initialized
- **New subdomain SSL coverage** ✅ — Hot-reloadable TLS cert (`TlsReloadHandle`) + renewal manager DB queries for new subdomains; cert reissued immediately when new apps are deployed, no restart needed
- **Orphaned restart containers** ✅ — Startup reconciliation removes all `rivetr-*-restart-*` containers not associated with an active deployment
- **Swap not configured warning** ✅ — Startup warns if no swap is present; `install.sh` now automatically creates a 2–4 GB swapfile during installation

## Future Considerations

- **Auto-scaling** ✅ Foundation implemented (rules + background scaler)
- **Service Dependencies** ✅ Implemented (dependency graph + API)
- **MCP Server** ✅ Foundation implemented (`/mcp` endpoint with 4 tools)
- **Plugin System** - Extensible architecture for custom builders/runtimes
- **Kubernetes Support** - K8s as alternative orchestrator
- **Terraform Provider** - Infrastructure-as-code integration
- **SAML 2.0** - Enterprise SSO via SAML assertions
- **Remote Build Execution** - Full SSH-based remote build pipeline
- **File System Browser** ✅ Implemented (browse/read/write/delete files on remote servers over SSH)
- **Community Templates** ✅ Implemented (submit, review, approve/reject, auto-promote to registry)

---

## Rivetr Advantages (vs Coolify & Dokploy)

| Advantage | Detail |
|-----------|--------|
| ~30MB RAM idle | vs Coolify (~800MB) and Dokploy (~250MB) |
| Single binary | No PostgreSQL, Redis, or Docker Compose stack for the PaaS itself |
| Podman support | Neither Coolify nor Dokploy supports Podman |
| Railpack builder | Railway's next-gen builder |
| Rust performance | Fastest proxy, lowest latency |
| Embedded SQLite | No external database dependency |
| Built-in cost estimation | Neither competitor offers this |
| AES-256-GCM encryption | Env vars encrypted at rest |
| Native Prometheus /metrics | Built-in metrics endpoint |

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

### Priority Areas for Contributors

1. **Preview Deployments** - High impact, well-defined scope
2. **Service Templates** - Easy to add, high visibility
3. **OAuth Login** - Standard auth expectation
4. **Notification Channels** - Low effort, high value
5. **Documentation** - Always appreciated

---

## Version History

See [CHANGELOG.md](./CHANGELOG.md) for detailed release notes.
