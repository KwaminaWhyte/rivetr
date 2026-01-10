# Changelog

All notable changes to Rivetr are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Preview deployments for pull requests
- S3 backup integration

---

## [0.2.4] - 2025-01-10

### Added
- **Team Collaboration (Multi-tenant)** - Full multi-tenant team support with resource isolation:
  - **Team Switching** - Sidebar team switcher with persistent context across sessions
  - **Resource Scoping** - Apps, projects, databases, and services scoped to teams via `team_id` columns
  - **Team Invitations** - Email-based invitation system with secure tokens and 7-day expiry
  - **Invitation Emails** - Professional HTML/text email templates via configurable SMTP
  - **Invitation Accept Flow** - Complete invite acceptance with login redirect support
  - **Audit Logging** - 23 action types tracking all team and resource operations
  - **Audit Log UI** - Paginated activity log with action, resource type, and date filters
  - **App Sharing** - Share apps between teams with view-only permissions
  - **Member Management** - Role changes with hierarchy (owner > admin > developer > viewer)
  - **Member Removal** - Remove team members with proper role-based access control
  - **Team-scoped Stats** - Dashboard statistics filtered by current team context
  - **Migration CLI** - `rivetr db migrate-teams` command to migrate legacy resources to teams
  - **Personal Workspace** - "Personal (default)" option for resources without team context

- **Resource Alerts & Cost Estimation** - Monitoring and cost tracking:
  - **Resource Metrics Collection** - Per-app CPU, memory, disk, and network usage tracking
  - **Alert Configurations** - Customizable thresholds per app with email notifications
  - **Alert Events** - Historical record of threshold breaches with severity levels
  - **Cost Rates** - Configurable pricing for CPU, memory, disk, and network resources
  - **Cost Snapshots** - Daily cost calculations per app for billing and reporting
  - **Team Costs API** - Aggregate cost reporting by team

- **Embedded Frontend Assets** - Frontend static files are now embedded in the binary using `rust-embed`:
  - Single binary deployment - no external static files needed
  - Compressed assets with proper cache headers
  - SPA fallback for client-side routing
  - MIME type detection for all asset types

- **CLI Tool** - Full command-line interface:
  - `rivetr status` - Show server health, version, uptime, and resource usage
  - `rivetr apps list` - List all applications in a formatted table
  - `rivetr apps show <app>` - Show details for a specific app
  - `rivetr deploy <app>` - Trigger deployment (by name or ID)
  - `rivetr logs <app> [--follow]` - Stream application logs via SSE
  - `rivetr config check` - Validate configuration file
  - Global options: `--api-url`, `--token` (or `RIVETR_API_URL`, `RIVETR_TOKEN` env vars)

- **Metrics Storage with Retention** - SQLite-based metrics aggregation:
  - `stats_hourly` table for hourly aggregates (30-day retention)
  - `stats_daily` table for daily aggregates (365-day retention)
  - Background aggregation task running hourly
  - Configurable retention policies via `[stats_retention]` config section
  - New `GET /api/system/stats/summary` endpoint for system-wide metrics

### Fixed
- **Team Switcher** - Fixed switching to Personal workspace after creating other teams:
  - Personal workspace selection now persists correctly using a marker value in localStorage
  - Distinguished between "no preference yet" and "explicitly chose personal workspace"

- **Install Script** - Fixed production installation script (`install.sh`):
  - Corrected binary download URL format to match GitHub releases (`rivetr-v{VERSION}-linux-{ARCH}`)
  - Fixed architecture detection (`x86_64` and `aarch64` instead of `amd64`/`arm64`)
  - Added `AmbientCapabilities=CAP_NET_BIND_SERVICE` to systemd service for port 80/443 binding
  - Added automatic build dependency installation for source compilation fallback
  - Added dynamic version fetching from GitHub API when `RIVETR_VERSION=latest`

---

## [0.2.3] - 2025-01-10

### Fixed
- Updated macOS x86_64 build runner from retired `macos-13` to `macos-15-intel`

---

## [0.2.2] - 2025-01-10

### Fixed
- Resolved OpenSSL cross-compilation issues by adding `vendored-openssl` feature to `git2`
- Changed macOS builds to use native runners instead of cross-compilation for reliability

---

## [0.2.1] - 2025-01-10

### Fixed
- Switched `reqwest` from `native-tls` to `rustls-tls` for cross-platform builds

---

## [0.2.0] - 2025-01-09

### Added
- **Railpack Builder** - Railway's Nixpacks successor with BuildKit integration
- **Cloud Native Buildpacks** - Heroku and Paketo buildpack support via `pack` CLI
- **Auto-rollback** - Automatic rollback on health check failure
- **Paginated Deployments API** - `GET /api/apps/:id/deployments?page=1&per_page=20`

### Changed
- Updated Claude Code agents and skills documentation

### Fixed
- JWT generation test for malformed PEM structures
- Redeployment for ZIP-uploaded apps using existing images

---

## [0.1.0] - 2025-01-08

### Added

#### Core Deployment Engine
- Git deployments from GitHub, GitLab, Gitea with webhook signature verification
- Multiple build types: Dockerfile, Nixpacks, static sites
- Docker and Podman runtime support with auto-detection
- Zero-downtime deployments with health checks
- Real-time build and runtime log streaming via WebSocket/SSE
- Rollback to any previous deployment

#### Platform Services
- **Managed Databases** - One-click PostgreSQL, MySQL, MongoDB, Redis deployments
- **Docker Compose** - Deploy multi-container apps from docker-compose.yml
- **26 Service Templates** - Portainer, Grafana, Uptime Kuma, Gitea, n8n, MinIO, Traefik, and more
- **Database Backups** - Scheduled backups with hourly/daily/weekly options

#### Security
- HTTPS with automatic Let's Encrypt certificates and auto-renewal
- Team management with RBAC (owner/admin/developer/viewer roles)
- Rate limiting with sliding window algorithm
- Input validation and command injection protection
- Security headers middleware (X-Content-Type-Options, X-Frame-Options, etc.)
- AES-256-GCM encryption for environment variables at rest
- Constant-time comparison for timing attack prevention

#### Dashboard
- Modern React + TypeScript dashboard with SSR (React Router v7)
- Real-time deployment status and resource monitoring
- Browser-based terminal access to containers (xterm.js)
- Theme switching (light/dark/system)
- Build logs viewer for all historical deployments

#### Operations
- ZIP file upload deployment with build type auto-detection
- GitHub App integration for seamless repository access
- Container crash recovery with exponential backoff
- Startup self-checks (database, runtime, directories, disk space)
- Prometheus metrics endpoint (`/metrics`)
- Disk space monitoring with alerts

#### Configuration
- Per-app resource limits (CPU/memory)
- Build resource limits
- Custom Dockerfile path and build targets
- Pre/post deployment commands
- Multiple domains per app with auto-SSL
- HTTP Basic Auth protection
- Container labels for Traefik/Caddy integration
- Volume management with backup/export

### Infrastructure
- GitHub Actions CI/CD with multi-platform releases (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows)
- SQLite database with WAL mode
- Embedded reverse proxy with ArcSwap for lock-free route updates

---

## Version History Summary

| Version | Date | Highlights |
|---------|------|------------|
| 0.2.4 | 2025-01-10 | Team collaboration, resource alerts, cost estimation |
| 0.2.3 | 2025-01-10 | macOS runner update |
| 0.2.2 | 2025-01-10 | OpenSSL vendoring fix |
| 0.2.1 | 2025-01-10 | rustls-tls migration |
| 0.2.0 | 2025-01-09 | Railpack, CNB buildpacks, auto-rollback |
| 0.1.0 | 2025-01-08 | Initial release with full PaaS features |

---

## Migration Notes

### Upgrading to 0.2.x

No breaking changes. The 0.2.x releases are focused on build system improvements and new builder support.

### From Source

```bash
git pull origin main
cargo build --release
# Restart the service
```

### Using Install Script

```bash
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

---

[Unreleased]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.4...HEAD
[0.2.4]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/KwaminaWhyte/rivetr/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/KwaminaWhyte/rivetr/releases/tag/v0.1.0
