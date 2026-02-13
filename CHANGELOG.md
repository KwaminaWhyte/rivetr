# Changelog

All notable changes to Rivetr are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Preview deployments for pull requests
- S3 backup integration

---

## [0.2.16] - 2026-02-13

### Fixed
- **Auto-Update Download** - Fixed "Downloaded update undefined" message:
  - Added `version` field to download response so the frontend displays the actual version
- **Auto-Update Apply** - Fixed "Failed to create backup of current binary" error:
  - Binary backup now falls back to temp directory when install dir is read-only (systemd `ProtectSystem=strict`)
  - Added symlink resolution and proper permission handling after update
- **Delete GitHub App** - Fixed 405 Method Not Allowed on DELETE `/api/github-apps/:id`:
  - Added `delete_app` handler that removes the app and its installations
  - Registered DELETE route in the API router
- **Git Providers FK Constraint** - Fixed 500 error when adding GitLab/Bitbucket providers:
  - Replaced hardcoded `user_id = "admin"` with actual authenticated user ID
  - OAuth callback now queries the first admin user from the database

### Added
- **Audit Logging** - Extended audit logging to previously unlogged API modules:
  - Auth: login and initial setup events
  - SSH Keys: create, update, and delete operations
  - GitHub Apps: delete operations
  - Git Providers: add and delete operations
  - Added new action constants and resource types for all new audit events

---

## [0.2.15] - 2026-02-13

### Fixed
- **Install Script** - Fixed binary download failure causing slow build-from-source fallback:
  - Default version was hardcoded to `v0.2.13` which was never released (no binary existed)
  - Changed default to `latest` which auto-fetches the actual latest release from GitHub API
- **Install Script** - Fixed build-from-source `RustEmbed` compilation error:
  - Added frontend build step (Node.js install + `npm run build`) before `cargo build`
  - Falls back to creating minimal placeholder directory if Node.js is unavailable
  - Resolves `#[derive(RustEmbed)] folder 'static/dist/client' does not exist` error
- **GitHub App Callback** - Fixed 404 after GitHub App installation:
  - Backend redirected to `/settings/github-apps` which doesn't exist in the frontend
  - Corrected redirect to `/settings/git-providers` where the GitHub Apps tab lives

---

## [0.2.14] - 2026-02-05

### Fixed
- **Container Monitor** - Fixed missing `team_id` column in database queries
  - `check_databases` and `reconcile_databases` queries now include `team_id` field
  - Eliminates recurring "no column found for name: team_id" warning every 30 seconds
- **Notification Channels** - Added 'webhook' to CHECK constraint in `notification_channels` table
  - Migration 038 recreates table with updated constraint allowing webhook channel type
  - Handles foreign key constraints properly with PRAGMA foreign_keys=OFF

---

## [0.2.13] - 2026-02-05

### Fixed
- **Teams API Panic** - Fixed string slicing panic when creating Personal team
  - User IDs shorter than 8 characters (e.g., "system") no longer cause panic
  - Uses safe character iteration instead of byte slicing for slug generation

---

## [0.2.12] - 2025-02-05

### Fixed
- **Dashboard Stats Chart** - Fixed authentication token key mismatch in resource chart component
  - Stats history API now correctly receives auth token on dashboard and monitoring pages

---

## [0.2.11] - 2025-02-05

### Added
- **Auto-Update System** - Automatic version checking and update management:
  - Background update checker with configurable interval (default: 6 hours)
  - API endpoints for version info, update check, download, and apply
  - `GET /api/system/version` - Current version and update status
  - `POST /api/system/update/check` - Trigger immediate update check
  - `POST /api/system/update/download` - Download update binary
  - `POST /api/system/update/apply` - Apply downloaded update
  - Configuration via `[auto_update]` section in rivetr.toml
  - Optional auto-apply mode for fully automated updates

### Changed
- Comprehensive testing documentation in `live-testing/` directory

---

## [0.2.10] - 2025-02-05

### Fixed
- **PORT Environment Variable** - Automatically inject `PORT` env var into containers:
  - Apps that expect PORT (like Heroku apps) now work correctly out of the box
  - PORT is set to the configured container port if not already set by user
  - Applied to main deployments, rollbacks, and preview deployments

---

## [0.2.9] - 2025-02-05

### Changed
- Version bump for release pipeline

---

## [0.2.8] - 2025-02-04

### Fixed
- **Container Monitor** - Added missing `team_id` column to services SELECT query
  - Fixed SQL error when container monitor tried to restart crashed services

---

## [0.2.7] - 2025-02-04

### Added
- **Personal Team Auto-Creation** - Automatically create "Personal" team for users without teams
  - New users get a Personal team created on first login
  - Existing users without teams get one created when needed

---

## [0.2.6] - 2025-02-04

### Fixed
- **Frontend API URL** - Use `window.location.hostname` instead of hardcoded `localhost`
  - Dashboard now works correctly when accessed via IP address or custom domain

---

## [0.2.5] - 2025-02-04

### Fixed
- **Systemd Service** - Multiple fixes for Docker/Podman compatibility:
  - Changed `ProtectHome=read-only` instead of `true` for Docker Compose access
  - Added `/home/rivetr` to systemd `ReadWritePaths` for Docker config
  - Create rivetr user home directory during installation

- **Install Script** - Auto-detect external_url from server IP
- **Cost API** - Fixed SQL type mismatch in cost calculations

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
| 0.2.16 | 2026-02-13 | Auto-update fixes, delete GitHub App, git provider FK fix, audit logging |
| 0.2.15 | 2026-02-13 | Install script download fix, GitHub App callback fix |
| 0.2.14 | 2026-02-05 | Container monitor and notification webhook fixes |
| 0.2.13 | 2025-02-05 | Teams API panic fix for short user IDs |
| 0.2.12 | 2025-02-05 | Dashboard stats chart auth fix |
| 0.2.11 | 2025-02-05 | Auto-update system with API endpoints |
| 0.2.10 | 2025-02-05 | Auto-inject PORT env var for containers |
| 0.2.9 | 2025-02-05 | Release pipeline update |
| 0.2.8 | 2025-02-04 | Container monitor team_id fix |
| 0.2.7 | 2025-02-04 | Personal team auto-creation |
| 0.2.6 | 2025-02-04 | Frontend dynamic hostname |
| 0.2.5 | 2025-02-04 | Systemd and install script fixes |
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

[Unreleased]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.16...HEAD
[0.2.16]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.15...v0.2.16
[0.2.15]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.14...v0.2.15
[0.2.14]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.13...v0.2.14
[0.2.13]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.12...v0.2.13
[0.2.12]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.11...v0.2.12
[0.2.11]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.10...v0.2.11
[0.2.10]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.9...v0.2.10
[0.2.9]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.8...v0.2.9
[0.2.8]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.7...v0.2.8
[0.2.7]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/KwaminaWhyte/rivetr/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/KwaminaWhyte/rivetr/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/KwaminaWhyte/rivetr/releases/tag/v0.1.0
