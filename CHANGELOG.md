# Changelog

All notable changes to Rivetr are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **TUI (`rivetr tui`)**: terminal UI for managing Rivetr from the command line; tabbed Apps/Deployments/Servers/Logs views; keyboard navigation (d=deploy, s=stop, r=restart, ?=help); live log polling every 5s; connects to any instance via --url/--token; built with ratatui + crossterm (enable with `--features tui`)
- **Fine-grained RBAC** — Per-resource permission overrides allow team admins to grant or deny individual members access to specific apps, projects, databases, or services. Overrides are stored in the new `team_resource_permissions` table (migration 089). Managed via GET/PUT `/api/teams/:id/members/:user_id/permissions` and DELETE `/api/teams/:id/members/:user_id/permissions/:perm_id`. Admin UI available as a dialog in team member settings.
- **Deployment Queue Cancellation** — Any queued or running deployment can now be cancelled via the Cancel button in the deployment detail view. The backend records `cancelled_at` (migration 090) and signals the engine's per-deployment `CancellationToken` to abort the current pipeline stage.
- **Community Template Submissions** — Users can submit custom Docker Compose templates for admin review from a new Submit dialog on the Templates page. Submissions are stored in `community_template_submissions` (migration 091) with `pending`/`approved`/`rejected` status. Admins review from an All Submissions page; approved submissions are automatically promoted to the live service template registry. Users can track their own submissions from My Submissions.
- **Remote Filesystem Browser** — Browse, read, write, and delete files on any connected remote server over SSH. API: `GET /api/servers/:id/files` (directory listing), `GET /api/servers/:id/files/content` (read), `PUT /api/servers/:id/files/content` (write), `DELETE /api/servers/:id/files` (delete). Frontend: full file browser at `/servers/:id/files` with breadcrumb navigation and inline text editor. Accessible via the new Files button on the Servers settings page.

### Planned
- SAML 2.0 support
- Remote build execution (SSH-based, RemoteContext foundation in place)
- Overlay networking for Docker Swarm
- Rolling updates with Swarm

---

## [0.10.6] - 2026-03-14

### Added
- **Container Resource Limits UI** — CPU and memory limits are now configurable per app from Settings → Docker Options. A new "Apply Now (Live)" button enforces limits on the running container immediately via `docker update` (cgroup changes, no redeploy needed). Useful for throttling runaway containers without downtime.
- **DragonFlyDB** — New managed database type. Redis-compatible, port 6379, `redis://` connection string, RDB backup format.
- **KeyDB** — New managed database type. Redis-compatible with multi-threading, port 6379, `redis://` connection string.
- **ClickHouse** — New managed database type. Analytics-focused columnar store on port 8123, `clickhouse://` connection string, `CLICKHOUSE_DEFAULT_ACCESS_MANAGEMENT=1` enabled by default, post-start database init.
- **Docker Compose Raw Mode** — New per-service toggle that deploys the compose file exactly as written, skipping all Rivetr network injection, container name namespacing, and label additions. Essential for services with opinionated internal networking.
- **Ansible Playbook** — `ansible/rivetr.yml` provides an idempotent playbook for automated server provisioning on Ubuntu 22.04/24.04 and Debian 12. Installs Docker, downloads the Rivetr binary, configures systemd service, and sets up UFW firewall rules.
- **Service Templates Master Registry** — `docs/SERVICE-TEMPLATES.md` lists all 273 unique templates by category with IDs and source files. Referenced before adding new templates to prevent duplicates.

### Fixed
- **Duplicate Service Templates** — Removed 55 duplicate template entries (same app, different IDs across sprint files). Total unique templates reduced from 328 → 273 after full deduplication. All future templates must be checked against `docs/SERVICE-TEMPLATES.md` first.
- **Env Vars in Storage Settings** — Removed the duplicate Environment Variables panel from Settings → Storage. Env vars are only managed from the dedicated Env Vars tab.

---

## [0.10.5] - 2026-03-13

### Added
- **27 New Service Templates** — Added templates across 7 categories: Administration/Dashboards (Homepage, Homarr, Dashy, Organizr), AI Services (AnythingLLM, LibreChat, Langflow, LiteLLM, LibreTranslate), Analytics (GoatCounter, OpenPanel), Backup (Duplicati), Communication (Matrix Synapse), Development Tools (NocoDB, Budibase, Dozzle, Portainer CE, Jenkins, Appsmith), Media Servers (Plex, Emby, qBittorrent, Sonarr, Radarr), Storage (Nextcloud, Seafile), Security (Pi-hole). Total templates now ~119.
- **Port Conflict Validation** — Users are now prevented from using ports already in use across the entire platform. Validation occurs in three places:
  - Service template deployment: real-time debounced check (400ms) with red-border indicator and disabled deploy button
  - Custom service creation: server-side 409 Conflict response
  - Database `external_port` update: server-side 409 Conflict response with descriptive error toast
  - New `GET /api/services/check-port?port=N` endpoint returning `{ available, conflict }` for frontend checks
- **Auto-Subdomain for Template Services** — Services deployed from templates now automatically get a subdomain (`{name}.{base_domain}`) with a proxy route registered on startup, making them accessible via domain instead of raw `host:port` URLs.

### Fixed
- **Service Stop Status** — Services deployed from templates were showing "Failed" after stop instead of "Stopped". Root cause: `stop_service` was using `get_compose_dir` (data dir only) instead of `get_service_compose_dir` (falls back to temp dir used by template deployments).

---

## [0.10.4] - 2026-03-12

### Added
- **API Tokens** — Users can now create named API tokens for programmatic access. Tokens are shown once on creation (prefixed `rvt_`), stored as SHA-256 hashes, and support optional expiry dates. The `/api/tokens` CRUD endpoints are available to all authenticated users. Existing scripts using the admin config token are unaffected.

### Fixed
- **Version Reporting** — `Cargo.toml` version now correctly reflects the deployed release so that the Auto Updates page reports the accurate running version.
- **Breadcrumbs** — Several routes were falling through to the "Page" fallback. Added entries for `/costs` (Cost Analysis) and dynamic patterns for app sub-tabs: Previews, Jobs, Log Drains, and Monitoring.
- **Notification Channels 403** — Team notification channels endpoint now correctly grants access when authenticating with the admin API token (system user bypass was missing from `require_team_role` in notifications.rs).
- **Team Breadcrumb** — The team detail page breadcrumb now shows the team name (e.g. "Teams > Personal") instead of the generic "Page" fallback.
- **Build Server Docker Column** — Docker version column no longer shows "vnot installed" — the `v` prefix is only added when the version string starts with a digit.
- **SSH Password Auth for Servers & Build Servers** — Servers and build servers can now be registered using a password (no SSH key required). `sshpass` is used transparently for health checks and terminal connections when no key is provided.

---

## [0.10.3] - 2026-03-12

### Added
- **Recharts Dashboard** — Replaced custom SVG charts with Recharts across the dashboard and monitoring pages for improved interactivity and maintainability.
- **Service Domain Routing** — Docker Compose services now support a configurable domain (auto-populated as `{name}.{base_domain}`) with full proxy integration: routes registered on start, removed on stop/delete, and restored on server startup.
- **Service Restart Button** — Services now have a Restart button (alongside Stop) when running.

### Fixed
- **Proxy Route Restore on Startup** — Running apps now have their proxy routes fully restored after a Rivetr restart (binary update). The startup `restore_routes` function now falls back to `inspect()` when `list_containers` doesn't return a port, and also restores Basic Auth config and `www.` redirect variants. This prevents 404s for all apps after a server update.
- **Audit Log User Display** — The audit log now shows the user's email address instead of their UUID. Backend does a `LEFT JOIN users` and returns `user_email` in the response.
- **Audit Log Resource Type Formatting** — Multi-word resource types like `ssh_key` now display as "SSH Key" instead of "Ssh_key".
- **SSH Key Delete Audit Log** — Deleting an SSH key now records the key name (not the UUID) in the audit log resource_name field.
- **DB Backup Download 401** — Database backup downloads now correctly include the Authorization header by falling back to the stored auth token when none is explicitly passed.
- **Database Data Directory Uniqueness** — The data directory for managed databases now includes the first 8 characters of the database UUID (e.g., `pharmapro-db-a1b2c3d4`) to prevent path collisions when databases with the same name are created across time.
- **Deployment Detail Status Badge** — The status badge on the deployment detail page now correctly shows "Running" for active deployments (was falling back to "Pending" because "running" was missing from STATUS_CONFIG). Also added "Replaced" and "Stopped" labels.
- **Service Logs Duplication** — Service log viewer no longer shows duplicate entries (was showing REST-fetched initial logs plus SSE-streamed history replay simultaneously).
- **Service Network Tab Open Link** — The "Open" button in the Service Network tab now uses the configured domain URL (when port matches service.port) instead of always using hostname:port.
- **Dashboard Stats Console Errors** — The `/api/apps/:id/stats` endpoint now returns zeroed stats (HTTP 200) for apps without a running deployment instead of 404, eliminating spurious browser console errors.
- **Recent Events: Replaced Status** — Deployments with status "replaced" (superseded by newer deploy) now show a descriptive event message instead of "unknown".
- **Deployment Log Streaming** — Build output from Docker and Nixpacks is now streamed in real time to deployment logs.
- **Logout Auth Header** — Logout now correctly passes the Authorization header and clears the stored token.
- **Team-Scoped Queries Backward Compat** — All team-scoped queries (`apps`, `databases`, `services`, `system stats`) now include `OR team_id IS NULL` so legacy resources without a team assignment remain visible.
- **Freeze Windows API Path** — Frontend now calls the correct endpoint `/api/apps/:id/freeze-windows` (was incorrectly using `/api/freeze-windows?app_id=...`).
- **Migration 067/068 Registration** — The `databases.container_slug` and `services.domain/port` migrations are registered in `run_migrations()` so columns are created on first run.
- **Admin API Token Permissions** — System/admin API token now has full access to all teams and can delete apps without requiring a password.
- **Stuck Deployments Cleanup** — Deployments stuck in `running` or `pending` state are now cleaned up on server startup.
- **TypeScript Fixes** — Fixed `DeploymentLog.id` type mismatch (`number` → `string`), `GitLabIcon` missing `style` prop, log sort using string timestamp comparison.

---

## [0.10.2] - 2026-03-11

### Added
- **Deployment Detail Page** — `/apps/:id/deployments/:deploymentId` shows real-time build logs via WebSocket, deployment metadata (commit, timing, status), and error details. Navigating to this page after triggering a deploy is now automatic.
- **GitLab Source in New App** — GitLab repo picker is now available in the new app creation form alongside GitHub.

### Fixed
- **Git Clone Authentication** — HTTPS clones for private GitHub, GitLab, and Bitbucket repos now inject the linked provider's OAuth/PAT token into the URL so git does not prompt for credentials (`fatal: could not read Username`). GitHub uses `x-access-token`, GitLab uses `oauth2:`, Bitbucket uses `x-token-auth:`. The `git_provider_id` is now stored on creation and looked up at deploy time.
- **App Name Validation** — Names are now auto-sanitized in the frontend (lowercase, spaces → dashes, invalid chars stripped) so the format error is never shown to users. Backend validation is also relaxed (single-char names allowed, trailing dashes allowed). Global uniqueness constraint removed — apps with the same name are now allowed (apps are identified by UUID).
- **Bitbucket Auth** — Updated from deprecated App Passwords to Atlassian API Tokens (ATATT prefix). Label and help link updated in the git providers settings.
- **HTTP→HTTPS Redirect** — Port 80 no longer redirects to HTTPS when no TLS certificate is available. The redirect is only activated after the certificate is confirmed.
- **Sidebar URL Restructure** — Infrastructure and access items (servers, teams, git-providers, ssh-keys, tokens, webhooks) are now at clean top-level URLs instead of `/settings/*`.
- **Missing Project Routes** — `/projects/:id/environments` and `/projects/:id/env-vars` are now registered and render correctly.

### Changed
- **Deployment Diff & Build Logs Modals** — Width increased to `max-w-5xl`/`max-w-4xl` (was `max-w-4xl`/`max-w-2xl`).
- **"View Full Logs" Button** — Now navigates to the deployment detail page instead of opening a small modal.
- **Deploy Button** — After triggering a deployment, the user is automatically taken to the deployment detail page to watch live logs.
- **Database Migration 066** — `apps.name` UNIQUE constraint removed. Existing data is preserved; all indexes are recreated.

---

## [0.10.1] - 2026-03-10

### Fixed
- **PackConfig Default** — `trust_builder` now correctly defaults to `true` in Rust's `Default` impl (was `false` due to `#[derive(Default)]`), fixing the `test_empty_config` test
- **Frontend TypeScript** — Fixed `running-services-card.tsx` passing raw `string` instead of `{ teamId }` option objects to `api.getApps/getDatabases/getServices`; removed reference to non-existent `app.current_deployment` field
- **servers.tsx** — Replaced `require("react")` inside component body with proper top-level imports (`useEffect`, `useRef`)
- **Cargo.toml** — Bumped version to `0.10.1` and fixed repository URL to `KwaminaWhyte/rivetr`
- **install.sh** — Fixed Railpack download URL (uses `-musl` not `-gnu`, includes version tag from GitHub API)
- **Database migrations** — Fixed startup crash: `execute_sql` split-on-semicolon parser now handles migration comments correctly; removed semicolon from comment in `061_registry_push.sql`
- **Database migrations** — Added missing migrations 051 (shared env vars), 052 (multi-server), 054 (container replicas), 055 (scheduled backups), 056 (2FA enforcement), 065 (webhook audit) to the migration runner

---

## [0.10.0] - 2026-03-10

### Added

#### Container Registry Push
- **Registry Push on Deploy** — Apps can push built images to any Docker registry after a successful build (registry URL, username, encrypted password, toggle per app)
- **Settings UI** — Container Registry section in app settings

#### Rollback Retention Policies
- **Configurable Retention** — Keep 1–50 previous deployments per app (default: 10); older deployments and their logs are automatically trimmed after each successful deploy
- **Settings UI** — Rollback retention count input in deployment settings

#### Community Template Suggestions
- **Suggest a Template** — Users can submit template suggestions (name, Docker image, category, description, website URL); stored in `template_suggestions` table
- **Admin Approval** — Admins can list pending suggestions and approve them (seeds into service_templates)

#### Auto-scaling Foundation
- **Autoscaling Rules** — Define CPU/memory threshold-based scaling rules per app with min/max replica bounds and cooldown periods
- **Background Scaler** — 60-second cycle reads resource metrics, scales replicas up/down within configured bounds
- **Settings UI** — Auto-scaling card in app settings with create/edit/delete dialog

#### Enhanced Prometheus Metrics
- `rivetr_deployments_total{app,status}` — deployment count by app and outcome
- `rivetr_deployment_duration_seconds{app}` — deployment duration tracking
- `rivetr_active_apps_total` / `rivetr_active_databases_total` — live resource gauges
- `rivetr_webhooks_received_total{provider}` — webhook ingestion by provider

#### Webhook Audit Log
- **Webhook Events Table** — All incoming webhook events logged with provider, event type, repository, branch, apps triggered, and status
- **Settings Page** — Webhook Events viewer with provider/status filters and 30-second auto-refresh

#### MCP Server
- **Model Context Protocol** — `/mcp` endpoint exposing Rivetr tools for AI assistant integration: `list_apps`, `deploy_app`, `get_app_status`, `get_deployment_logs`

---

## [0.9.0] - 2026-03-10

### Added

#### Multi-Server Enhancements
- **Server App Assignment** — Assign apps to specific registered servers; engine logs remote deployment intent with `RemoteContext` foundation for full remote build pipeline
- **Remote Server Terminal** — WebSocket SSH terminal embedded in server settings page; uses xterm.js, same UX as container terminal
- **Server App Management API** — `GET/POST/DELETE /api/servers/:id/apps` endpoints

#### Docker Swarm Integration
- **Swarm Management** — Initialize/leave Docker Swarm; stores manager/worker join tokens
- **Node Management** — List, sync from Docker, drain/activate nodes via `docker node update`
- **Service Scaling** — Create, scale, remove, and inspect logs for Swarm services via `docker service` CLI
- **Swarm Dashboard** — Full settings page with status card, nodes table, services table

#### Build Servers
- **Dedicated Build Server Registration** — Separate build servers with encrypted SSH keys, concurrent build limits, health checks
- **Build Server API** — Full CRUD + health check (`docker version`, CPU, memory stats)
- **Build Server Dashboard** — Settings page with active/concurrent build counts

#### Deployment Preview Diff
- **Diff View** — "View Diff" button per deployment shows commit SHA range, commit count, commit messages, and files changed
- **Diff Dialog** — Modal with scrollable commit messages and file list

#### Instance Backup to S3
- **Upload to S3** — "Upload to S3" button per backup in the backup list; calls `POST /api/system/backups/:name/upload-to-s3`

---

## [0.8.0] - 2026-03-10

### Added

#### DockerHub Webhook
- **DockerHub Integration** — Deploy apps automatically when a Docker image is pushed to DockerHub; apps with matching `docker_image` field are triggered; supports `callback_url` acknowledgement

#### Scheduled Backups
- **Backup Schedules** — Cron-based scheduling for instance and S3 backups; configurable retention days; background scheduler runs every 60s
- **Backup Schedule API** — CRUD endpoints for managing backup schedules with enable/disable toggle

#### 2FA Enforcement Per Team
- **Team-level 2FA Requirement** — Owners can mandate that all team members have 2FA enabled; users without TOTP are blocked from team resources
- **Security Tab** — New owner-only Security tab in team settings with 2FA enforcement toggle and warning banner

#### Template Search & Filtering
- **Template Search** — Backend `search` query param filters templates by name/description; frontend shows result count and scrollable category pills for all 12+ categories

#### Service Dependency Graph
- **Dependency Visualization** — Projects show a dependency graph of apps, databases, and services with colored node labels and edge arrows
- **Dependency API** — `GET /api/projects/:id/dependency-graph`, `POST /api/apps/:id/dependencies`, `DELETE /api/apps/:id/dependencies/:dep_id`
- **service_dependencies table** — Track inter-service dependencies with referential integrity

#### Zero-Downtime Indicator
- **Deployment Phase Banner** — Deployments tab shows real-time phase indicator: Stable (green), Deploying (blue pulsing), Health Checking (yellow), Switching Traffic (orange spinning)
- **Extended App Status** — `GET /api/apps/:id/status` now returns `deployment_phase`, `active_deployment_id`, and `uptime_seconds`

#### Multi-Server Support
- **SSH Server Registration** — Register remote servers with SSH credentials (encrypted); health check gathers CPU/memory/disk/OS/Docker stats
- **Servers Management Page** — Settings page with server status indicators and "Check Now" per server

#### SSO/OIDC
- **OpenID Connect** — Full OIDC auth flow with provider management; supports Auth0, Keycloak, Google, Azure AD, Okta with quick-fill presets
- **SSO Auth Flow** — `/auth/sso/:id/login` initiates OIDC redirect; `/auth/sso/:id/callback` exchanges code, creates or links user account

#### Container Replicas
- **Replica Scaling** — Set replica count 1–10 per app; pipeline starts N containers on deploy; proxy does round-robin across all backends
- **Round-Robin Load Balancer** — Proxy layer updated with `RoundRobinBackend` and atomic counter for lock-free selection

### Refactored
- All Rust files >1000 lines split into organized subdirectory modules (pipeline, container_monitor, docker, git_providers, deployments, validation, services, system, alert_notifications, cli)
- Frontend `types/api.ts` split into 7 domain files (apps, deployments, databases, services, teams, notifications, system) — all imports unchanged via barrel re-export
- Frontend `projects/$id.tsx` (2103→275 lines) — extracted apps/databases/services tabs into components
- Frontend `teams/$id.tsx` (1327→641 lines) — extracted members/invitations/audit tabs into components
- Frontend notifications split into shared `channel-config-fields.tsx` component

---

## [0.5.0] - 2026-03-10

### Added

#### Deployment Enhancements
- **Approval Workflow** — Apps can require approval before deploys execute; pending deployments await admin/owner sign-off with approve/reject UI and reason field
- **Scheduled Deployments** — Deploy at a specific time by passing `scheduled_at` in the deploy request; background scheduler picks up due jobs every 60s
- **Deployment Freeze Windows** — Define time windows (days of week + UTC start/end) when deploys are blocked; returns 409 during frozen periods
- **Pending Approvals Badge** — Deployments tab shows red badge count when approvals are waiting
- **Approval Status** — Deployment timeline shows Awaiting/Approved/Rejected badges with rejection reason

#### Bulk Operations & App Management
- **Bulk Actions** — Multi-select apps in project view, then Start / Stop / Restart / Deploy all at once
- **App Cloning** — Deep-copy any app (config, env vars, domains) with one click; gets name `{name}-copy`
- **Config Snapshots** — Save named point-in-time snapshots of app config and env vars; restore any snapshot later
- **Project Export/Import** — Download a full project as JSON (all apps, env vars, domains); re-import to recreate
- **Maintenance Mode** — Toggle per-app maintenance mode with custom message; shows badge in header

#### Shared Environment Variables
- **Team Shared Vars** — Set key/value variables at the team level inherited by all team apps
- **Project Shared Vars** — Set variables at the project level, overriding team vars
- **Inheritance Chain** — Resolution order: team → project → environment → app (highest wins)
- **Resolved Variables View** — New "Resolved" tab in env vars UI shows effective variables with source badges (team/project/environment/app)
- **Shared Var Management** — Team settings and project settings each have a Shared Variables page

#### Code Organization
- `src/api/apps.rs` (1990 lines) → `src/api/apps/` (mod + crud/control/sharing/upload/logs)
- `src/api/teams.rs` (1682 lines) → `src/api/teams/` (mod + crud/members/invitations/audit)

---

## [0.4.0] - 2026-03-10

### Added

#### S3 Backup Integration
- **S3 Storage Configs** - Support for AWS S3, MinIO, Cloudflare R2, and any S3-compatible endpoint with encrypted credential storage
- **Backup to S3** - Upload instance, database, and volume backups to S3 buckets in background
- **Restore from S3** - Browse and restore any backup stored in S3 with one click
- **S3 Settings UI** - Configure storage configs, test connections, manage backups, trigger restores

#### Advanced Monitoring
- **Full-Text Log Search** - Search deployment logs by query, date range, and log level
- **Uptime Tracking** - Background health checks every 60s with availability percentage, response time, and 24h/7d/30d history
- **Log Retention Policies** - Per-app configurable retention (days + max size), with daily background cleanup
- **Scheduled Container Restarts** - Cron-based automatic restarts per app with enable/disable toggle
- **Monitoring Tab** - New tab on each app with log search, uptime stats, retention config, and scheduled restart management

#### Log Draining
- **Axiom** - HTTPS ingest with dataset and API token
- **New Relic** - Log API with US/EU region support
- **Datadog** - Log intake with configurable site (datadoghq.com, EU, etc.)
- **Logtail** (Better Stack) - Source token based ingestion
- **Custom HTTP** - Generic POST to any endpoint with optional auth header
- **Batched Forwarding** - Logs buffered and flushed every 5 seconds or 100 lines, with error tracking
- **Log Drains Tab** - Per-app management UI with provider config forms, enable/disable, and test button

#### Code Organization (File Splitting)
- `src/db/seeders.rs` → `src/db/seeders/` (10 files by template category)
- `src/api/webhooks.rs` → `src/api/webhooks/` (mod.rs + github.rs, gitlab.rs, gitea.rs, bitbucket.rs)

### Changed
- **Watch path filtering** added to GitHub push handler (was missing, only Gitea/GitLab had it)

---

## [0.3.0] - 2026-03-09

### Added

#### Preview Deployments
- **PR Preview Environments** - Automatic preview deployments for pull requests with unique subdomains (`pr-{number}.{app}.{domain}`)
- **GitLab Merge Request Support** - Full MR event handling (open, update, close, merge) triggers preview deploy/cleanup
- **Gitea Pull Request Support** - Full PR event handling with preview deploy/cleanup
- **GitHub PR Comments** - Auto-post/update preview URL as comment on GitHub PRs via GitHub App API
- **Preview Cleanup** - Automatic container and proxy route removal on PR close/merge across all 4 Git providers
- **Preview Resource Limits** - Default lower limits (256MB memory, 0.5 CPU) for preview containers

#### Watch Paths
- **Selective Deployment** - Configure glob patterns per app (e.g., `src/*`, `Dockerfile`) to only deploy when matched files change
- **Webhook Filtering** - GitHub, GitLab, Gitea, and Bitbucket push webhooks now skip deployment when no watched files are modified
- **Watch Paths UI** - Settings card with add/remove pattern chips and glob documentation

#### Bitbucket Webhooks
- **Bitbucket Push Events** - `repo:push` webhook handler with HMAC-SHA256 signature verification
- **Bitbucket PR Events** - `pullrequest:created/updated/fulfilled/rejected` handling with preview deployment support
- **Bitbucket Config** - `bitbucket_secret` webhook configuration option
- **Git Providers UI** - Bitbucket tab with webhook URL display, copy button, and connection test

#### Notification Channels (4 New)
- **Telegram** - Bot API integration with HTML formatting and forum/topic support
- **Microsoft Teams** - Incoming webhook with Adaptive Card v1.4 rich formatting
- **Pushover** - Multi-device push notifications with configurable priority (-2 to 2)
- **Ntfy** - Self-hosted push notification support with configurable server URL, priority, and tags
- Rivetr now supports **8 notification channels** (Slack, Discord, Email, Webhook, Telegram, MS Teams, Pushover, Ntfy)

#### Instance Backup & Restore
- **Full Instance Backup** - SQLite WAL checkpoint + database, config, and SSL certificates bundled to tar.gz
- **Backup API** - 5 endpoints: create, list, download, delete, restore (POST /api/system/backup)
- **CLI Commands** - `rivetr backup [--output path]` and `rivetr restore <file>`
- **Backup Settings Page** - Create & download backups, upload restore with confirmation dialog, backup list management

#### OAuth Login
- **GitHub OAuth** - Full authorization flow with callback, user creation/linking
- **Google OAuth** - Full authorization flow with callback, user creation/linking
- **Account Linking** - Connect OAuth identities to existing accounts in settings
- **OAuth Admin Config** - Provider management page (client ID/secret, enable/disable per provider)
- **Login Page** - OAuth provider buttons shown conditionally based on enabled providers

#### Project Environments
- **Environment Model** - dev/staging/production environments per project with auto-creation on project create
- **Environment-Scoped Variables** - Separate env vars per environment, merged into deployment pipeline
- **Predefined System Variables** - `RIVETR_ENV`, `RIVETR_APP_NAME`, `RIVETR_URL` injected automatically
- **Environment Switching** - Dropdown selector in project UI to filter apps by environment
- **Environments Management Page** - Full CRUD with embedded env var editor per environment

#### Two-Factor Authentication
- **TOTP 2FA** - Compatible with Google Authenticator, Authy, and other TOTP apps
- **QR Code Setup** - Guided setup flow with QR code display and verification step
- **Recovery Codes** - 10 one-time recovery codes (SHA-256 hashed, consumed on use)
- **Encrypted Secrets** - TOTP secrets encrypted at rest with AES-256-GCM
- **Login Flow** - Modified to support 2FA: temporary 5-minute session, then TOTP validation
- **Security Settings** - New settings page for 2FA enable/disable/recovery code management

#### Service Templates Expansion (26 → 74)
- **AI/ML** - Ollama, Open WebUI, LiteLLM, Langflow, Flowise, ChromaDB
- **Analytics** - Plausible, Umami, PostHog, Matomo
- **Automation** - Activepieces, Windmill, Trigger.dev
- **CMS** - WordPress, Ghost, Strapi, Directus, Payload CMS
- **Communication** - Rocket.Chat, Mattermost, Matrix/Synapse
- **Development** - Code Server, Supabase, Appwrite, Pocketbase, Hoppscotch, Forgejo
- **Documentation** - BookStack, Wiki.js, Docmost, Outline
- **File/Media** - Immich, Jellyfin, Navidrome, Seafile
- **Monitoring** - SigNoz, Beszel, Checkmate
- **Security** - Authentik, Keycloak, Vaultwarden, Infisical
- **Search** - Meilisearch, Typesense
- **Project Management** - Plane, Vikunja, Leantime, Cal.com
- **Other** - Paperless-ngx, Trilium, Linkwarden, Tandoor, Stirling-PDF
- **Template Categories** - New category enum variants (Ai, Automation, Cms, Communication) for gallery organization

#### Scheduled Jobs
- **Cron Scheduler** - Background cron evaluator with 60-second polling and container exec via Docker/Podman
- **Job Management API** - 7 CRUD endpoints (GET/POST/PUT/DELETE /api/apps/:id/jobs, run, history)
- **Execution History** - `scheduled_job_runs` table tracking status, output, duration per execution
- **Jobs UI** - Full management tab per app: create, edit, enable/disable, cron expression input, run history viewer

#### Deploy by Commit/Tag
- **Commit/Tag Deploy** - Deploy specific Git commit SHA or tag via API (`commit_sha`/`git_tag` in deploy request)
- **Git Checkout** - Pipeline clones full history and checks out specific ref during build
- **Commits/Tags API** - List commits and tags from GitHub API (GET /api/apps/:id/commits, /api/apps/:id/tags)
- **Deploy Modal** - Commit/tag selector dropdown with SHA preview and tag badges
- **Deployment History** - Tag badge displayed in deployment timeline for tagged deploys

### Changed
- **Notification CHECK Constraint** - Migrations 039 and 041 update the channel_type constraint to include all 8 providers
- **Login Response** - Now includes `requires_2fa` field when 2FA is enabled
- **Project Creation** - Auto-creates production, staging, and development environments
- **Deployment Pipeline** - Merges environment-scoped variables and injects system predefined variables; supports commit/tag checkout
- **Template Gallery** - Now shows 74 templates across 12+ categories (up from 26)
- **Service Template Model** - Added new category variants: Ai, Automation, Cms, Communication

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
