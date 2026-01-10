# Rivetr Roadmap

> A fast, lightweight deployment engine built in Rust

This document outlines the planned development roadmap for Rivetr. For detailed task tracking, see [plan/TASKS.md](./plan/TASKS.md).

## Current Status

**Overall Progress: 89% Complete (372/418 tasks)**

| Phase | Status | Progress |
|-------|--------|----------|
| Phase 0: Foundation | Complete | 93% |
| Phase 1: Core Engine (MVP) | Complete | 100% |
| Phase 2: Production Ready | Complete | 100% |
| Phase 3: Enhanced Features | Complete | 96% |
| Phase 4: Platform Services | Complete | 100% |
| Phase 5: Advanced CI/CD | In Progress | 67% |
| Phase 6: Unique Features | Planned | 0% |

---

## Released Features (v0.2.x)

### Core Deployment Engine
- Git deployments from GitHub, GitLab, Gitea with webhook signature verification
- Multiple build types: Dockerfile, Nixpacks, Railpack, Heroku/Paketo buildpacks, static sites
- Docker and Podman runtime support with auto-detection
- Zero-downtime deployments with health checks and automatic rollback
- Real-time build and runtime log streaming via WebSocket

### Platform Services
- One-click managed databases (PostgreSQL, MySQL, MongoDB, Redis)
- Docker Compose multi-container deployments
- 26 pre-configured service templates (Grafana, Portainer, Uptime Kuma, Gitea, n8n, etc.)
- Automated database backup scheduling with retention policies

### Team Collaboration ✅ NEW
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

---

## In Progress (v0.3.x)

### Preview Deployments
Automatic PR preview environments with unique URLs.

- [ ] Parse PR events from webhooks (open, sync, close, merge)
- [ ] Create preview deployment with unique subdomain (`pr-{number}.{app}.{domain}`)
- [ ] Auto-cleanup on PR close/merge
- [ ] Post preview URL as comment on PR (GitHub API)
- [ ] Support GitLab/Gitea MR previews

### Advanced Rollbacks
Enhanced rollback with registry integration.

- [x] Automatic health-based rollback
- [x] Rollback settings UI
- [ ] Push built images to Docker registry on deploy
- [ ] Configure rollback retention policies
- [ ] Docker Swarm update configuration

### CLI Tool ✅ COMPLETE
Command-line interface for common operations.

- [x] `rivetr status` - Show server status
- [x] `rivetr apps list` - List all applications
- [x] `rivetr deploy <app>` - Trigger deployment
- [x] `rivetr logs <app>` - Stream application logs
- [x] `rivetr config check` - Validate configuration

---

## Planned (v0.4.x+)

### Resource Alerts & Cost Estimation
Proactive monitoring and cost visibility.

- [ ] CPU/memory threshold alerts
- [ ] Alert channels (email, Slack, Discord)
- [ ] Cost estimation based on resource usage
- [ ] Monthly cost projections per app
- [ ] Cost dashboard widget

### Deployment Enhancements
Advanced deployment workflows.

- [ ] Deployment preview diff (show changes before deploy)
- [ ] Approval workflow for production deployments
- [ ] Scheduled deployments (deploy at specific time)
- [ ] Deployment freeze periods
- [ ] Zero-downtime indicator (blue/green status)

### Bulk Operations & App Management
Efficiency features for managing multiple apps.

- [ ] Bulk start/stop/restart multiple apps
- [ ] Bulk deploy multiple apps
- [ ] App cloning (duplicate configuration)
- [ ] Configuration snapshots (save/restore)
- [ ] Export/import projects (JSON backup)
- [ ] Maintenance mode with custom page

### Advanced Monitoring
Enhanced observability features.

- [ ] Full-text log search
- [ ] Configurable log retention policies
- [ ] Scheduled container restarts
- [ ] Service dependency graph visualization
- [ ] Uptime tracking per app
- [ ] Response time monitoring

### S3 Backup Integration
Cloud backup for volumes and databases.

- [ ] S3 storage configuration (AWS, MinIO, R2)
- [ ] Volume backup to S3
- [ ] Database backup to S3
- [ ] Scheduled S3 backups
- [ ] One-click restore from S3

---

## Future Considerations

These features are under consideration but not yet planned:

- **Multi-server Support** - Deploy to multiple servers from one dashboard
- **Auto-scaling** - Scale containers based on load
- **Service Dependencies** - Define app startup order and dependencies
- **Custom Domains API** - Programmatic domain management
- **Plugin System** - Extensible architecture for custom builders/runtimes

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

### Priority Areas for Contributors

1. **Preview Deployments** - High impact, well-defined scope
2. **CLI Tool** - Self-contained, good for newcomers
3. **Documentation** - Always appreciated
4. **Testing** - Integration tests for core features

---

## Version History

See [CHANGELOG.md](./CHANGELOG.md) for detailed release notes.
