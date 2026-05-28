# Changelog

All notable changes to Rivetr are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.23] - 2026-05-27

### Added
- **Host-protection resource defaults**: every container now inherits configurable safety defaults (memory limit, PID limit, OOM score bias) when it sets none, so a runaway container can no longer exhaust host RAM. Per-resource limits still override; set `default_memory_limit = ""` to opt out.
- **Container log rotation + critical-disk auto-reclaim**: containers now get log rotation defaults, and the disk monitor automatically prunes images and build cache when usage crosses the critical threshold, closing the disk host-hang vector.
- **Database integrity check on startup**: self-check runs a SQLite `PRAGMA quick_check` and surfaces corruption loudly in the startup report. Non-critical; the server still starts.
- **Tooling & reference docs**: added `rustfmt.toml`, a `[lints]` table in `Cargo.toml`, and configuration/API reference docs covering every `rivetr.toml` field and REST endpoint.

### Tests
- Expanded unit coverage for build-type auto-detection and API input validation (+33 tests covering precedence, framework detection, and injection/path-traversal blocking). Suite now 246 passing.

## [0.10.22] - 2026-05-27

### Fixed
- **Panic isolation**: switched the release profile from `panic = "abort"` to `panic = "unwind"` so a panic in one of the ~79 Tokio tasks unwinds only that task instead of killing the whole process. A new `guarded()` helper wraps all 16 background loops in `catch_unwind`, and the 8 reverse-proxy hot-path `unwrap()` calls now fall back to a header-less response on untrusted-input errors.

### Changed
- Release builds now unwind on panic (slightly larger binary, negligible runtime cost) for process-level resilience.

[0.10.23]: https://github.com/KwaminaWhyte/rivetr/compare/v0.10.22...v0.10.23
[0.10.22]: https://github.com/KwaminaWhyte/rivetr/compare/v0.10.21...v0.10.22

## [0.10.21] - 2026-05-02

### Added
- **Database-to-app linking with auto env injection**: a new "Linked Databases" section lets users connect a managed database to an app with one click, auto-injecting connection env vars (`DATABASE_URL`, `HOST`, `PORT`, etc., or `REDIS_URL` / `MONGODB_URL`) on the next deploy. An optional prefix avoids key collisions; user-defined env vars always win.
- **Inline credentials display on the project DB list**: each database card now has a "Show credentials" toggle that expands the connection string, username, and password inline with copy buttons.

### Fixed
- **MySQL 8 connection broken via published connection string**: managed MySQL containers now start with `--skip-ssl` so clients can connect without TLS errors, reverting the prior query-string workaround.
- **Audit `ip_address` was always null**: the `ClientIp` extractor is now wired into all 30+ audit-emitting handlers, recording socket IP (or `X-Forwarded-For` for proxied requests).
- **Disk-stats path inconsistency**: backend now canonicalizes `data_dir` so the Dashboard and Monitoring pages report the same number.

### UX
- **Sidebar user menu**: clearer spacing, hover/focus states, and a rotating chevron make the user pill obviously interactive.
- **Deploy menu commit/tag and ZIP-file modals**: both menu items now open their dialogs (fixed a Radix dropdown-to-dialog focus race).
- **Template page category anchors**: category sections have anchors and "View all N" links that smooth-scroll and expand inline.
- **Resource Limits live-apply**: saving limits now applies them immediately to a running container via `docker update`, with helper text that adapts to running/stopped state.

### Removed
- The prior `?ssl-mode=DISABLED` MySQL connection-string workaround, replaced by the `--skip-ssl` server-side fix.

[0.10.21]: https://github.com/KwaminaWhyte/rivetr/compare/v0.10.20...v0.10.21

## [0.10.20] - 2026-05-02

### Added
- **Coolify-style deploy log side panel**: a dockable panel opens automatically on Deploy/Start/Restart (apps, services, databases) and streams image-pull and container-start activity in real time, via new per-resource WebSocket/REST routes.
- **MariaDB as a managed database type**: MariaDB joins the catalogue (versions 11 default, 10.11, 10.6, 10.5) with the correct `mariadb://` scheme, data mount, and `mariadb-dump` backups.
- **`scripts/deploy-vm.sh`**: local Parallels Ubuntu ARM64 VM deploy mirror for faster iteration on Apple Silicon.

### Fixed
- **Container monitor service health check broken since v0.10.18**: the health-check query omitted columns added by a later migration, silently disabling crash detection for all Compose services. Now selects the full column set.
- **`/api/apps/:id/insights` returned 503 when AI not configured**: now returns 404, eliminating misleading console 503s and log spam.
- **Docker network "endpoint already exists" warnings on restart**: the connect helper now treats the `403 + "already exists"` pair as an idempotent no-op.
- **MariaDB/MySQL post-start user-provisioning warning silenced** when the entrypoint already created the user.
- **Audit log now records the full set of mutating actions** (token, app, service, database, deployment, env-var, and domain operations) that were previously missing.
- **`/api/audit?limit=N` alias accepted**, matching the Coolify/Dokploy convention.
- **POST/DELETE endpoints accept empty body / no `Content-Type`**: delete-app, rollback, and cancel no longer return 415 without a body.
- **Cancel deployment in non-cancellable state returns 409** (was 404), reflecting that the deployment exists but isn't cancellable.
- **Rollback `commit_message` references the correct deployment id** (the path-param target).
- **Old running deployment marked `replaced` not `failed` after rollback**, since no build error occurred.
- **Stable internal hostname surfaced in App model**: `GET /api/apps/:id` now exposes a derived `internal_hostname` so the Network tab connection example stays correct across restarts.
- **Templates list endpoint compressed + slim**: responses now gzip-compress and return a slim field set (was ~500 KB plain, now ~86 KB gzipped); full bodies live at `GET /api/templates/:id`.
- **Database SQL backup downloads use `application/sql` Content-Type** so browsers can display rather than save-as.
- **Compose service auto-domain fallback**: services with no domain now auto-assign `<service-name>.local` so they're reachable via the proxy.

### Frontend bug fixes (VM sweep)
- Setup form placeholder and validator now agree on a 12-character minimum.
- App General tab hides the "Dockerfile path" field unless build type is `dockerfile`.
- Network tab shows the actual network name `"rivetr"` (was `"rivetr-network"`).
- Templates page first paint capped to 6 cards per category with a "View all N" link (was ~1300 cards).
- `/monitoring` chart wrapper now has an explicit min-height, ending the recharts `width(-1)` warning.
- Setup auto-logs-in when the API returns a session token, falling back to `/login` otherwise.
- Deployment detail label shows "Deployed at" instead of "Finished" while still running.
- Project page no longer double-renders empty-state CTA buttons.
- Added form `autocomplete` attributes so password managers and iOS strong-password generation work.
- Login submit button is disabled when email or password is empty.
- Project page hides the Healthy/Issues/Building tabs when a project has zero apps.
- Deployment-log render strips ANSI escape sequences.

### Infrastructure
- Added `scripts/deploy-vm.sh` for fast Parallels-VM iteration on Apple Silicon.
- Fixed 7 pre-existing clippy errors; `cargo clippy` with `-D warnings` now exits 0.

[0.10.20]: https://github.com/KwaminaWhyte/rivetr/compare/v0.10.19...v0.10.20

## [v0.10.19] - 2026-03-25

### Fixed
- **Zero-downtime HTTPS during instance restarts**: proxy listeners are now pre-bound once in `main.rs` and shared, so the HTTPS server inherits its systemd socket instead of failing with "Address already in use." Connections now queue at the kernel during restarts instead of getting refused.

## [v0.10.18] - 2026-03-25

### Added
- **Service resource monitor**: the service detail page now shows live CPU, memory, and network stats when running.
- **Service public port exposure**: services can expose a container port on the host via a new Network tab, with port-conflict checks and auto-restart on change.
- **Full backup includes service data**: backups now bundle Compose files and database dumps for running service containers (Postgres, MySQL/MariaDB, Redis-family, MongoDB), with non-fatal dump failures.

### Fixed
- **Migrations 104 and 105 not auto-applied**: both are now registered with idempotent guards so they apply on next startup.

## [v0.10.17] - 2026-03-25

### Changed
- **AI provider config moved to dashboard**: provider, API key, model, and max-token cap are now set under Settings, stored in the database, and hot-applied with no restart. The `[ai]` config section remains a startup fallback; the raw key is never returned by the API.

### Added
- **Settings AI Provider card**: configure provider (Claude, OpenAI, Gemini, Moonshot), masked API key, model override, and max-tokens, with a "Configured" badge.

## [v0.10.16] - 2026-03-25

### Added
- **AI-powered features (Phase 1)**: multi-provider AI client across the platform, all requiring an `[ai]` API key in `rivetr.toml`.
  - **Deployment error diagnosis**: sends recent logs to the AI and returns a structured diagnosis with actionable suggestions.
  - **Deployment insights**: success rate, average build time, and 30-day trend with an optional AI summary.
  - **Cost optimisation suggestions**: AI-generated resource tuning recommendations on the Monitoring tab.
  - **Dockerfile Optimizer**: returns an optimised Dockerfile with a list of improvements.
  - **Security & Compliance Advisor**: five rule-based checks plus an optional AI summary, surfaced in a new Security tab; a global scan aggregates across all apps.

## [v0.10.15] - 2026-03-25

### Fixed
- **ACME cert saved under wrong directory on renewal**: `save_certificate` now takes an explicit `instance_domain` so renewed certs land in the correct directory and survive restarts.
- **Uptime checker skipped apps with no custom domain**: the poller now falls back to `auto_subdomain` and checks `*.rivetr.site` subdomains over HTTPS.
- **Monitoring page showed "100% availability" with 0 checks**: now renders `"-"` when there are zero checks.
- **Uptime summary stats ignored period selector**: the period dropdown now threads through to the backend and re-fetches.
- **Westel Dockerfiles**: removed unsupported BuildKit cache mounts, upgraded PHP 8.3 to 8.4, and preserved layer-ordering optimisations.

---

## [v0.10.13] - 2026-03-22

### Fixed
- **App API boolean types**: app endpoints now return `AppResponse`, mapping SQLite integer columns to JSON booleans and fixing frontend deserialization errors.
- **Frontend error handling for non-JSON responses**: the API helper now catches `SyntaxError` and surfaces the raw text body instead of a confusing JSON-parse message.
- **Docker image prune filter**: replaced the invalid `system prune --filter dangling=true` with `docker image prune -f`.
- **Full system backup failure**: `ManagedDatabase` queries now use `SELECT *` so newer schema fields no longer cause column-not-found errors.
- **Firewall security check false positive**: the check no longer marks inactive firewalls as active.

### Added
- **Webhook events pagination**: `GET /api/webhook-events` now accepts `page`/`per_page` and returns a paginated envelope; the UI persists state in URL params.
- **Disk pressure auto-cleanup**: the cleanup engine checks disk usage and, above 85% full, keeps only the latest deployment per app and prunes images and build cache.
- **Docker build cache pruning**: every cleanup cycle now runs `docker builder prune -f` to reclaim cache that previously grew unbounded.
- **Reduced default deployment retention**: `max_deployments_per_app` default lowered from 10 to 3.

---

## [v0.10.12] - 2026-03-18

### Added
- **Proxy access logs**: every proxied request is now logged to a `proxy_logs` table (host, method, path, status, latency, bytes, client IP, user agent), with a Settings page offering filters and auto-refresh.
- **User Preferences panel**: a Settings page with localStorage preferences for theme, date/time format, default log lines, deployment notifications, and compact mode.
- **11 new service templates (Sprint 26)**: MediaWiki, SuperTokens, Netbird, AFFiNE, HeyForm, OpnForm, GitHub Actions Runner, Bluesky PDS, PeerTube, Roundcube, and docker-mailserver. Rivetr now has 335+ one-click templates.

---

## [v0.10.11] - 2026-03-18

### Added
- **Custom container labels**: apps support arbitrary Docker labels via a key-value editor, applied at launch alongside built-in Rivetr labels.
- **Proxy-level www redirect**: `www_redirect_mode` now issues 301 redirects at the proxy layer before any upstream connection.
- **5 new service templates (Sprint 25)**: Joomla, Drupal, Grafana standalone, Etebase, Obsidian Remote. Rivetr now has 315+ one-click templates.

### Fixed
- **Missing migrations 092-099**: the runner was missing these migrations, causing 500s on app/server endpoints; now registered.
- **Migration 101 column check**: the guard now checks for the `destination_id` column, not just table existence, fixing a 500 on fresh deployments.

---

## [v0.10.10] - 2026-03-18

### Added
- **Docker Destinations**: named Docker networks that apps can be assigned to, mirroring Coolify's destinations.
- **CA Certificate management**: upload trusted CA certificates (PEM) for custom TLS trust chains.
- **Inline Dockerfile**: deploy from a pasted Dockerfile with no git repo, skipping the clone entirely.
- **Fetch server details**: SSHes into a server to collect OS, Docker version, disk, CPU, and RAM, shown via "Refresh details."
- **WWW redirect mode**: per-domain dropdown (No redirect / Serve both / to www / to non-www) issuing 301s for the redirect variants.
- **Instance timezone**: global IANA timezone for scheduled tasks and log timestamps.
- **11 new service templates (Sprint 24)**: Vaultwarden, LiteLLM, MindsDB, Matrix Synapse, Rocket.Chat, NodeBB, Zipline, Joplin Server, Siyuan Notes, Hatchet, EasyAppointments. Rivetr now has ~113 one-click templates.

---

## [v0.10.9] - 2026-03-18

### Fixed
- **Cost analysis pricing**: estimates were 300-600x too low. Fixed with a per-server `hourly_rate` model (default $0.036/hr matching DigitalOcean), DO-aligned fallback rates ($10/vCPU + $1/GB per month) for unassigned apps, and billing against allocated rather than used memory.

### Added
- **Hourly rate field on servers**: the server edit dialog now includes an "Hourly Cost (USD/hr)" input with DigitalOcean size hints.

---

### Added
- **Git clone options**: per-app toggles for submodules (`--recurse-submodules`), LFS (`git lfs pull`), and shallow clone (`--depth 1`, default on).
- **Build cache control**: a `disable_build_cache` toggle passes `--no-cache` to Dockerfile/Nixpacks builds for a clean rebuild.
- **SOURCE_COMMIT build arg**: an `include_source_commit` toggle injects the current git SHA as a Docker build argument.
- **Custom container name**: overrides the auto-generated `rivetr-<app-name>` name.
- **Static site flag**: marks an app as a static site, toggled in Build Settings.
- **Generate domain**: auto-assigns a random subdomain from the server's base domain (falls back to `sslip.io`).
- **Links dropdown**: app nav now lists all configured domains as quick-open external links.
- **Environment Clone**: duplicate a project environment's apps, databases, and services into a fresh environment (containers stopped, domains cleared, deployment state reset).
- **8 new service templates (Sprint 23)**: Flowise, Langflow, Open WebUI, AnythingLLM, Pocket ID, Activepieces, Trigger.dev, SigNoz. Rivetr now has ~102 one-click templates.
- **Server timezone setting**: per-server IANA timezone (default `UTC`), in the Edit Server dialog.
- **Strip URL prefix**: a per-app `strip_prefix` removes a URL prefix before proxying, in both HTTP and WebSocket forwarding.
- **Resend notification channel**: fixed validation for the `resend`, `mattermost`, `lark`, and `gotify` channel types, which were previously blocked.
- **Theme toggle in top nav**: the Light/Dark/System toggle is now in the header bar as well as the sidebar.
- **Version + feedback in sidebar**: the footer now shows the current version (linked to GitHub releases) and a Feedback link.
- **7 new service templates (Sprint 22)**: Minecraft Java, Palworld, Terraria, Satisfactory, Argilla, Mage AI, Glitchtip. Rivetr now has ~94 one-click templates.
- **13 new service templates (Sprint 21)**: Beszel Agent, ClassicPress, CloudBeaver, Diun, Homebox, Karakeep, Linkding, PairDrop, Readeck, Ryot, Shlink, Slash, Wakapi. Rivetr now has ~87 one-click templates.

---

## [v0.10.8] - 2026-03-17

### Fixed
- **Server status stuck offline**: the SSH health check now falls back to `sshpass` password auth when a stored key fails to load, instead of leaving the server permanently offline.
- **Server action buttons cluttered**: replaced the flat row of buttons with Terminal and Files as primaries and the rest grouped in a `...` dropdown.

---

## [v0.10.7] - 2026-03-16

### Added
- **Service resource monitoring**: built-in template services now have a Stats tab with live CPU, RAM, and network, aggregated across all containers in the Compose project.
- **Deployment Cleanup Settings UI**: configure `max_deployments_per_app` and image pruning from Settings; values persist in the database and fall back to config defaults.
- **Restart History in Deployments Tab**: app restarts now create deployment records with logs and a distinct "restart" trigger badge.
- **TUI (`rivetr tui`)**: a terminal UI with tabbed Apps/Deployments/Servers/Logs views, keyboard navigation, and live log polling (enable with `--features tui`).
- **Fine-grained RBAC**: per-resource permission overrides let admins grant or deny individual members access to specific apps, projects, databases, or services.
- **Deployment Queue Cancellation**: any queued or running deployment can be cancelled via the detail view, aborting the current pipeline stage.
- **Community Template Submissions**: users can submit Compose templates for admin review; approved submissions are promoted to the live registry.
- **Remote Filesystem Browser**: browse, read, write, and delete files on any connected server over SSH, with a full file browser and inline editor.

### Fixed
- **GitHub App webhooks silently dropped by HTTPS redirect**: `/webhooks/` paths are now excluded from the HTTP-to-HTTPS redirect so push events aren't dropped after TLS is issued.
- **GitHub App webhook URL used wrong scheme after HTTPS setup**: the registered URL now defaults to HTTPS, and a new sync endpoint updates the GitHub App's webhook URL.
- **App settings never showed webhook setup instructions**: the General tab now shows an "Auto-Deploy Setup" card with the ready-to-paste webhook URL and steps for the detected provider.
- **Duplicate webhook deployments on push**: fixed four overlapping bugs (non-atomic delivery-ID guard, unfiltered branch-deletion events, missing per-app idempotency, and a too-narrow active-status check) so a single push deploys once.
- **CPU chart compared against full host CPU instead of allocated limit**: the Resource Monitor now normalises against the container's CPU limit so 100% means the allocated quota.
- **API token auth in middleware**: `rvt_`-prefixed tokens are now validated in `auth_middleware`, fixing 401s on endpoints that don't use the user extractor.
- **TUI ping using auth-required endpoint**: switched to the public `/health` endpoint so connectivity and auth failures are reported separately.
- **TUI `/api/deployments` 404**: replaced the nonexistent global query with merged per-app requests.
- **Stale `container_id` causing stuck 'starting' state**: when a database's container is gone, Rivetr now clears the id, resets status, and provisions a fresh container.
- **Reconciliation queries failing after new migrations**: recovery queries now use `SELECT *` to stay resilient to schema additions.
- **5-field cron expressions rejected by scheduler**: 5-field Unix cron strings are now normalized by prepending a seconds field.
- **React setState-during-render in LogRetentionSection**: moved the sync state update into a `useEffect`.
- **MySQL/MariaDB user not created on reused data directory**: Rivetr now idempotently provisions the app user via the Unix socket after every container start.
- **New app subdomains missing SSL coverage**: certs can now be hot-swapped without a restart, and the renewal manager reissues immediately when new app subdomains appear.
- **Orphaned restart containers causing CPU/RAM drain**: stale `rivetr-*-restart-*` containers are now removed on startup, and the stop/remove task logs failures.
- **No swap warning**: Rivetr now warns at startup when the host has no swap, and the installer creates a 2-4 GB swapfile with `swappiness=10`.

### Planned
- SAML 2.0 support
- Remote build execution (SSH-based)
- Overlay networking and rolling updates for Docker Swarm

---

## [0.10.6] - 2026-03-14

### Added
- **Container Resource Limits UI**: configure CPU and memory limits per app, with an "Apply Now (Live)" button that enforces them on the running container via `docker update`, no redeploy needed.
- **DragonFlyDB**: new Redis-compatible managed database type (port 6379, `redis://`, RDB backups).
- **KeyDB**: new multi-threaded Redis-compatible managed database type (port 6379, `redis://`).
- **ClickHouse**: new columnar analytics managed database (port 8123, `clickhouse://`) with access management and post-start init.
- **Docker Compose Raw Mode**: a per-service toggle that deploys the compose file exactly as written, skipping Rivetr network injection, name namespacing, and labels.
- **Ansible Playbook**: `ansible/rivetr.yml` for idempotent server provisioning on Ubuntu/Debian (Docker, binary, systemd, UFW).
- **Service Templates Master Registry**: a reference doc listing all unique templates by category to prevent duplicates.

### Fixed
- **Duplicate Service Templates**: removed 55 duplicate entries, reducing unique templates from 328 to 273.
- **Env Vars in Storage Settings**: removed the duplicate Environment Variables panel; env vars are managed only from the dedicated tab.

---

## [0.10.5] - 2026-03-13

### Added
- **27 New Service Templates**: added across Dashboards, AI, Analytics, Backup, Communication, Dev Tools, Media, Storage, and Security categories. Total now ~119.
- **Port Conflict Validation**: ports already in use are now rejected at template deployment (debounced live check), custom service creation, and database external-port update (409 Conflict), backed by a `check-port` endpoint.
- **Auto-Subdomain for Template Services**: template-deployed services now get a `{name}.{base_domain}` subdomain with a proxy route on startup.

### Fixed
- **Service Stop Status**: template services now show "Stopped" instead of "Failed" after stop (fixed a compose-dir lookup that ignored the temp dir).

---

## [0.10.4] - 2026-03-12

### Added
- **API Tokens**: create named tokens (prefixed `rvt_`, stored as SHA-256 hashes, optional expiry) for programmatic access via `/api/tokens`. Existing admin-token scripts are unaffected.

### Fixed
- **Version Reporting**: `Cargo.toml` version now matches the release so Auto Updates reports the correct running version.
- **Breadcrumbs**: added missing entries for Cost Analysis and the Previews/Jobs/Log Drains/Monitoring app sub-tabs.
- **Notification Channels 403**: the team channels endpoint now grants access to the admin API token.
- **Team Breadcrumb**: the team detail page now shows the team name instead of "Page."
- **Build Server Docker Column**: no longer shows "vnot installed"; the `v` prefix is added only for version strings.
- **SSH Password Auth for Servers & Build Servers**: servers can now be registered with a password (no key required), using `sshpass` transparently.

---

## [0.10.3] - 2026-03-12

### Added
- **Recharts Dashboard**: replaced custom SVG charts with Recharts across the dashboard and monitoring pages.
- **Service Domain Routing**: Compose services now support a configurable domain with full proxy integration (registered on start, removed on stop/delete, restored on startup).
- **Service Restart Button**: services now have a Restart button alongside Stop when running.

### Fixed
- **Proxy Route Restore on Startup**: running apps now have routes (including Basic Auth and www variants) fully restored after a restart, preventing post-update 404s.
- **Audit Log User Display**: the audit log now shows the user's email instead of their UUID.
- **Audit Log Resource Type Formatting**: multi-word types like `ssh_key` now display as "SSH Key."
- **SSH Key Delete Audit Log**: deleting a key now records its name, not the UUID.
- **DB Backup Download 401**: backup downloads now fall back to the stored auth token.
- **Database Data Directory Uniqueness**: data directories now include the first 8 chars of the DB UUID to prevent path collisions.
- **Deployment Detail Status Badge**: now shows "Running" for active deployments (plus "Replaced" and "Stopped" labels).
- **Service Logs Duplication**: the log viewer no longer shows REST and SSE history entries simultaneously.
- **Service Network Tab Open Link**: now uses the configured domain URL when the port matches.
- **Dashboard Stats Console Errors**: the app stats endpoint now returns zeroed stats (200) for apps with no running deployment.
- **Recent Events Replaced Status**: superseded deployments now show a descriptive message instead of "unknown."
- **Deployment Log Streaming**: Docker and Nixpacks build output is now streamed in real time.
- **Logout Auth Header**: logout now passes the Authorization header and clears the token.
- **Team-Scoped Queries Backward Compat**: team-scoped queries now include `OR team_id IS NULL` so legacy resources stay visible.
- **Freeze Windows API Path**: the frontend now calls the correct freeze-windows endpoint.
- **Migration 067/068 Registration**: the container-slug and service domain/port migrations are now registered.
- **Admin API Token Permissions**: the admin token now has full access to all teams and can delete apps without a password.
- **Stuck Deployments Cleanup**: deployments stuck in `running`/`pending` are cleaned up on startup.
- **TypeScript Fixes**: corrected a `DeploymentLog.id` type mismatch, a missing icon prop, and a log sort comparison.

---

## [0.10.2] - 2026-03-11

### Added
- **Deployment Detail Page**: shows real-time build logs over WebSocket plus metadata and error details, navigated to automatically after a deploy.
- **GitLab Source in New App**: a GitLab repo picker is now available alongside GitHub in the new-app form.

### Fixed
- **Git Clone Authentication**: HTTPS clones for private GitHub/GitLab/Bitbucket repos now inject the provider's token so git doesn't prompt for credentials.
- **App Name Validation**: names are auto-sanitized in the frontend and validation is relaxed; the global uniqueness constraint was removed (apps are identified by UUID).
- **Bitbucket Auth**: migrated from deprecated App Passwords to Atlassian API Tokens.
- **HTTP-to-HTTPS Redirect**: port 80 no longer redirects to HTTPS until a TLS certificate is confirmed.
- **Sidebar URL Restructure**: infrastructure and access items now live at clean top-level URLs.
- **Missing Project Routes**: the project environments and env-vars routes are now registered.

### Changed
- **Deployment Diff & Build Logs Modals**: widened for readability.
- **"View Full Logs" Button**: now navigates to the deployment detail page instead of a small modal.
- **Deploy Button**: now takes the user to the deployment detail page to watch live logs.
- **Database Migration 066**: removed the `apps.name` UNIQUE constraint (existing data preserved).

---

## [0.10.1] - 2026-03-10

### Fixed
- **PackConfig Default**: `trust_builder` now correctly defaults to `true`, fixing `test_empty_config`.
- **Frontend TypeScript**: fixed option-object args and removed a reference to a nonexistent app field.
- **servers.tsx**: replaced an inline `require("react")` with proper top-level imports.
- **Cargo.toml**: bumped version to `0.10.1` and fixed the repository URL.
- **install.sh**: fixed the Railpack download URL (uses `-musl`, includes the version tag).
- **Database migrations**: fixed a startup crash in the comment-aware SQL parser and registered missing migrations (051, 052, 054, 055, 056, 065).

---

## [0.10.0] - 2026-03-10

### Added

#### Container Registry Push
- **Registry Push on Deploy**: apps can push built images to any Docker registry after a successful build (URL, username, encrypted password, per-app toggle), configured in a new app settings section.

#### Rollback Retention Policies
- **Configurable Retention**: keep 1-50 previous deployments per app (default 10); older deployments and logs are trimmed after each successful deploy. Configurable in deployment settings.

#### Community Template Suggestions
- **Suggest a Template**: users can submit template suggestions (name, image, category, description, URL); admins can list and approve them into the service templates.

#### Auto-scaling Foundation
- **Autoscaling Rules**: define per-app CPU/memory threshold rules with min/max replica bounds and cooldowns, enforced by a 60-second background scaler, managed from an app settings card.

#### Enhanced Prometheus Metrics
- New metrics for deployment count and duration by app, live app/database gauges, and webhook ingestion by provider.

#### Webhook Audit Log
- **Webhook Events**: all incoming webhook events are logged (provider, type, repo, branch, apps triggered, status) and viewable on a Settings page with filters and auto-refresh.

#### MCP Server
- **Model Context Protocol**: a `/mcp` endpoint exposes Rivetr tools (`list_apps`, `deploy_app`, `get_app_status`, `get_deployment_logs`) for AI assistant integration.

---

## [0.9.0] - 2026-03-10

### Added

#### Multi-Server Enhancements
- **Server App Assignment**: assign apps to specific registered servers, with a `RemoteContext` foundation for the remote build pipeline.
- **Remote Server Terminal**: an xterm.js WebSocket SSH terminal embedded in the server settings page.
- **Server App Management API**: endpoints to list, assign, and remove server apps.

#### Docker Swarm Integration
- **Swarm Management**: initialize/leave a Swarm and store join tokens.
- **Node Management**: list, sync, and drain/activate nodes.
- **Service Scaling**: create, scale, remove, and inspect Swarm services.
- **Swarm Dashboard**: a settings page with status, nodes, and services tables.

#### Build Servers
- **Dedicated Build Server Registration**: separate build servers with encrypted SSH keys, concurrency limits, and health checks, plus full CRUD and a dashboard.

#### Deployment Preview Diff
- **Diff View**: a per-deployment "View Diff" button showing the commit range, count, messages, and changed files in a modal.

#### Instance Backup to S3
- **Upload to S3**: a per-backup button to upload an instance backup to S3.

---

## [0.8.0] - 2026-03-10

### Added

#### DockerHub Webhook
- **DockerHub Integration**: apps with a matching `docker_image` auto-deploy when the image is pushed to DockerHub, with callback acknowledgement.

#### Scheduled Backups
- **Backup Schedules**: cron-based scheduling for instance and S3 backups with configurable retention and an enable/disable toggle, run by a 60s background scheduler.

#### 2FA Enforcement Per Team
- **Team-level 2FA Requirement**: owners can require all members to have 2FA, blocking non-TOTP users from team resources, via an owner-only Security tab.

#### Template Search & Filtering
- **Template Search**: a `search` param filters templates by name/description, with a result count and scrollable category pills.

#### Service Dependency Graph
- **Dependency Visualization**: projects show a graph of apps, databases, and services with labeled nodes and edges, backed by dependency endpoints and a `service_dependencies` table.

#### Zero-Downtime Indicator
- **Deployment Phase Banner**: the Deployments tab shows a real-time phase indicator (Stable, Deploying, Health Checking, Switching Traffic), and app status now returns `deployment_phase`, `active_deployment_id`, and `uptime_seconds`.

#### Multi-Server Support
- **SSH Server Registration**: register remote servers with encrypted credentials; health checks gather CPU/memory/disk/OS/Docker stats, shown on a management page with per-server "Check Now."

#### SSO/OIDC
- **OpenID Connect**: a full OIDC auth flow with provider management and quick-fill presets for Auth0, Keycloak, Google, Azure AD, and Okta; the callback creates or links a user account.

#### Container Replicas
- **Replica Scaling**: set 1-10 replicas per app; the pipeline starts N containers and the proxy round-robins across them via a lock-free `RoundRobinBackend`.

### Refactored
- Split all Rust files over 1000 lines into organized submodules, and split the frontend API types into 7 domain files and several large pages into components (all imports unchanged).

---

## [0.5.0] - 2026-03-10

### Added

#### Deployment Enhancements
- **Approval Workflow**: apps can require approval before deploys; pending deployments await admin sign-off with an approve/reject UI and reason field, surfaced via a badge and timeline status.
- **Scheduled Deployments**: deploy at a specific time via `scheduled_at`, picked up by a 60s scheduler.
- **Deployment Freeze Windows**: define windows (days + UTC start/end) when deploys are blocked (returns 409).

#### Bulk Operations & App Management
- **Bulk Actions**: multi-select apps to Start/Stop/Restart/Deploy at once.
- **App Cloning**: deep-copy an app's config, env vars, and domains as `{name}-copy`.
- **Config Snapshots**: save and restore named point-in-time snapshots of app config and env vars.
- **Project Export/Import**: download a full project as JSON and re-import to recreate it.
- **Maintenance Mode**: toggle per-app maintenance mode with a custom message and header badge.

#### Shared Environment Variables
- **Inheritance Chain**: team, project, environment, and app variables resolve in that order (highest wins), with a "Resolved" tab showing effective variables and source badges, managed from team and project settings.

---

## [0.4.0] - 2026-03-10

### Added

#### S3 Backup Integration
- **S3 Storage Configs**: support for AWS S3, MinIO, Cloudflare R2, and any S3-compatible endpoint with encrypted credentials.
- **Backup to S3 / Restore from S3**: upload instance, database, and volume backups to S3 and restore any of them with one click, managed from an S3 settings UI.

#### Advanced Monitoring
- **Full-Text Log Search**: search deployment logs by query, date range, and level.
- **Uptime Tracking**: 60s health checks with availability percentage, response time, and 24h/7d/30d history.
- **Log Retention Policies**: per-app retention (days + max size) with daily background cleanup.
- **Scheduled Container Restarts**: cron-based per-app restarts with an enable/disable toggle.
- **Monitoring Tab**: a per-app tab combining log search, uptime, retention, and scheduled restarts.

#### Log Draining
- **Provider Support**: Axiom, New Relic, Datadog, Logtail (Better Stack), and a generic Custom HTTP target.
- **Batched Forwarding**: logs buffered and flushed every 5 seconds or 100 lines, with error tracking and a per-app management tab with test button.

### Changed
- **Watch path filtering** added to the GitHub push handler (was missing; only Gitea/GitLab had it).

---

## [0.3.0] - 2026-03-09

### Added

#### Preview Deployments
- **PR Preview Environments**: automatic preview deploys for pull/merge requests across GitHub, GitLab, and Gitea, with unique subdomains, auto-posted GitHub PR comments, automatic cleanup on close/merge, and lower default resource limits.

#### Watch Paths
- **Selective Deployment**: configure per-app glob patterns so pushes only deploy when matched files change, applied across all four Git providers, with a settings card.

#### Bitbucket Webhooks
- **Bitbucket Support**: push and PR webhook handling with HMAC-SHA256 verification, a `bitbucket_secret` config option, and a Git Providers tab with webhook URL, copy, and connection test.

#### Notification Channels (4 New)
- **Telegram, Microsoft Teams, Pushover, Ntfy**: added rich-formatted integrations, bringing Rivetr to 8 notification channels.

#### Instance Backup & Restore
- **Full Instance Backup**: bundle the database (after WAL checkpoint), config, and SSL certs to a tar.gz, with create/list/download/delete/restore endpoints, `rivetr backup`/`restore` CLI commands, and a Settings page.

#### OAuth Login
- **GitHub & Google OAuth**: full authorization flows with user creation/linking, account linking in settings, an admin provider config page, and conditional login buttons.

#### Project Environments
- **Environments**: dev/staging/production environments per project (auto-created) with environment-scoped variables, predefined system variables (`RIVETR_ENV`, `RIVETR_APP_NAME`, `RIVETR_URL`), a switcher, and a management page.

#### Two-Factor Authentication
- **TOTP 2FA**: compatible with standard authenticator apps, with QR-code setup, 10 hashed recovery codes, AES-256-GCM-encrypted secrets, a 2FA-aware login flow, and a security settings page.

#### Service Templates Expansion (26 to 74)
- Added ~48 templates across AI/ML, Analytics, Automation, CMS, Communication, Development, Documentation, File/Media, Monitoring, Security, Search, and Project Management, with new category enum variants for gallery organization.

#### Scheduled Jobs
- **Cron Scheduler**: a 60s background evaluator runs container jobs via Docker/Podman, with CRUD endpoints, a `scheduled_job_runs` history table, and a per-app management tab.

#### Deploy by Commit/Tag
- **Commit/Tag Deploy**: deploy a specific commit SHA or tag (`commit_sha`/`git_tag`); the pipeline checks out the ref, the API lists commits/tags, and a deploy modal offers a selector with timeline badges.

### Changed
- Updated the notification channel CHECK constraint to include all 8 providers.
- Login response now includes `requires_2fa`.
- Project creation auto-creates production, staging, and development environments.
- The deployment pipeline merges environment-scoped and system variables and supports commit/tag checkout.
- The template gallery now shows 74 templates across 12+ categories.

---

## [0.2.16] - 2026-02-13

### Fixed
- **Auto-Update Download**: fixed the "Downloaded update undefined" message by adding a `version` field to the response.
- **Auto-Update Apply**: fixed the "Failed to create backup" error by falling back to a temp dir when the install dir is read-only, with symlink resolution and proper permissions.
- **Delete GitHub App**: fixed a 405 by adding and registering the DELETE handler that also removes installations.
- **Git Providers FK Constraint**: fixed a 500 when adding GitLab/Bitbucket providers by using the authenticated user's ID instead of a hardcoded one.

### Added
- **Audit Logging**: extended audit logging to Auth (login/setup), SSH Keys, GitHub App deletes, and Git Provider add/delete, with new action and resource constants.

---

## [0.2.15] - 2026-02-13

### Fixed
- **Install Script**: changed the default version to `latest` so it auto-fetches a real release instead of failing on a never-released tag, and added a frontend build step before `cargo build` to fix the `RustEmbed` compilation error.
- **GitHub App Callback**: corrected the post-install redirect to the existing git-providers page.

---

## [0.2.14] - 2026-02-05

### Fixed
- **Container Monitor**: added the missing `team_id` column to database queries, ending the recurring "no column found" warning.
- **Notification Channels**: migration 038 adds `webhook` to the channel-type CHECK constraint, handling foreign keys correctly.

---

## [0.2.13] - 2026-02-05

### Fixed
- **Teams API Panic**: fixed a string-slicing panic when creating a Personal team for user IDs shorter than 8 characters, using safe character iteration for slug generation.

---

## [0.2.12] - 2025-02-05

### Fixed
- **Dashboard Stats Chart**: fixed an auth-token key mismatch so the stats history API receives the token on the dashboard and monitoring pages.

---

## [0.2.11] - 2025-02-05

### Added
- **Auto-Update System**: background update checking (default 6h) with version-info, check, download, and apply endpoints, configured via `[auto_update]` in `rivetr.toml`, with an optional auto-apply mode.

### Changed
- Added comprehensive testing documentation.

---

## [0.2.10] - 2025-02-05

### Fixed
- **PORT Environment Variable**: Rivetr now auto-injects `PORT` (set to the container port) into deployments, rollbacks, and previews when not already set, so Heroku-style apps work out of the box.

---

## [0.2.9] - 2025-02-05

### Changed
- Version bump for release pipeline.

---

## [0.2.8] - 2025-02-04

### Fixed
- **Container Monitor**: added the missing `team_id` column to the services query, fixing a SQL error when restarting crashed services.

---

## [0.2.7] - 2025-02-04

### Added
- **Personal Team Auto-Creation**: new users get a "Personal" team on first login, and existing teamless users get one when needed.

---

## [0.2.6] - 2025-02-04

### Fixed
- **Frontend API URL**: uses `window.location.hostname` instead of hardcoded `localhost`, so the dashboard works via IP or custom domain.

---

## [0.2.5] - 2025-02-04

### Fixed
- **Systemd Service**: multiple Docker/Podman compatibility fixes (read-only home, added ReadWritePaths, created the rivetr home directory).
- **Install Script**: auto-detect `external_url` from the server IP.
- **Cost API**: fixed a SQL type mismatch in cost calculations.

---

## [0.2.4] - 2025-01-10

### Added
- **Team Collaboration (Multi-tenant)**: full multi-tenant teams with resource scoping via `team_id`, a sidebar team switcher, email invitations (7-day expiry) with HTML/text templates, role hierarchy (owner > admin > developer > viewer) and member management, app sharing, team-scoped stats, audit logging with a paginated UI, a `migrate-teams` CLI command, and a Personal workspace fallback.
- **Resource Alerts & Cost Estimation**: per-app CPU/memory/disk/network metrics, customizable alert thresholds with email notifications and event history, configurable cost rates, daily cost snapshots, and a team-cost aggregation API.
- **Embedded Frontend Assets**: frontend files are embedded via `rust-embed` for single-binary deployment, with compressed cached assets, SPA fallback, and MIME detection.
- **CLI Tool**: `status`, `apps list`/`show`, `deploy`, `logs --follow`, and `config check`, with `--api-url`/`--token` options and env-var equivalents.
- **Metrics Storage with Retention**: `stats_hourly` (30-day) and `stats_daily` (365-day) aggregates from an hourly background task, configurable retention, and a stats-summary endpoint.

### Fixed
- **Team Switcher**: fixed switching back to the Personal workspace by persisting an explicit marker in localStorage.
- **Install Script**: corrected the binary download URL and architecture detection, added `CAP_NET_BIND_SERVICE` for ports 80/443, added build-dependency installation, and added dynamic version fetching for `latest`.

---

## [0.2.3] - 2025-01-10

### Fixed
- Updated the macOS x86_64 build runner from retired `macos-13` to `macos-15-intel`.

---

## [0.2.2] - 2025-01-10

### Fixed
- Resolved OpenSSL cross-compilation issues by adding `git2`'s `vendored-openssl` feature.
- Changed macOS builds to native runners instead of cross-compilation for reliability.

---

## [0.2.1] - 2025-01-10

### Fixed
- Switched `reqwest` from `native-tls` to `rustls-tls` for cross-platform builds.

---

## [0.2.0] - 2025-01-09

### Added
- **Railpack Builder**: Railway's Nixpacks successor with BuildKit integration.
- **Cloud Native Buildpacks**: Heroku and Paketo buildpack support via the `pack` CLI.
- **Auto-rollback**: automatic rollback on health-check failure.
- **Paginated Deployments API**: `GET /api/apps/:id/deployments` with `page`/`per_page`.

### Changed
- Updated Claude Code agents and skills documentation.

### Fixed
- JWT generation test for malformed PEM structures.
- Redeployment for ZIP-uploaded apps using existing images.

---

## [0.1.0] - 2025-01-08

### Added

#### Core Deployment Engine
- Git deployments from GitHub, GitLab, and Gitea with webhook signature verification.
- Multiple build types: Dockerfile, Nixpacks, and static sites.
- Docker and Podman runtime support with auto-detection.
- Zero-downtime deployments with health checks.
- Real-time build and runtime log streaming via WebSocket/SSE.
- Rollback to any previous deployment.

#### Platform Services
- **Managed Databases**: one-click PostgreSQL, MySQL, MongoDB, and Redis.
- **Docker Compose**: deploy multi-container apps from a compose file.
- **26 Service Templates**: Portainer, Grafana, Uptime Kuma, Gitea, n8n, MinIO, Traefik, and more.
- **Database Backups**: scheduled hourly/daily/weekly backups.

#### Security
- HTTPS with automatic Let's Encrypt certificates and renewal.
- Team management with RBAC (owner/admin/developer/viewer).
- Rate limiting with a sliding-window algorithm.
- Input validation and command-injection protection.
- Security-headers middleware.
- AES-256-GCM encryption for environment variables at rest.
- Constant-time comparison for timing-attack prevention.

#### Dashboard
- A React + TypeScript dashboard with SSR (React Router v7).
- Real-time deployment status and resource monitoring.
- Browser-based container terminal (xterm.js).
- Theme switching (light/dark/system).
- A build-logs viewer for all historical deployments.

#### Operations
- ZIP-upload deployment with build-type auto-detection.
- GitHub App integration for repository access.
- Container crash recovery with exponential backoff.
- Startup self-checks (database, runtime, directories, disk).
- A Prometheus metrics endpoint (`/metrics`).
- Disk-space monitoring with alerts.

#### Configuration
- Per-app and build resource limits.
- Custom Dockerfile path and build targets.
- Pre/post deployment commands.
- Multiple domains per app with auto-SSL.
- HTTP Basic Auth protection.
- Container labels for Traefik/Caddy integration.
- Volume management with backup/export.

### Infrastructure
- GitHub Actions CI/CD with multi-platform releases.
- SQLite with WAL mode.
- An embedded reverse proxy with ArcSwap for lock-free route updates.

---

## Version History Summary

| Version | Date | Highlights |
|---------|------|------------|
| 0.10.20 | 2026-05-02 | Coolify-style deploy log side panel, MariaDB managed DB type, container monitor service-health regression fix, AI insights 503 to 404 |
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

No breaking changes. The 0.2.x releases focus on build system improvements and new builder support.

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

[Unreleased]: https://github.com/KwaminaWhyte/rivetr/compare/v0.10.20...HEAD
[0.10.20]: https://github.com/KwaminaWhyte/rivetr/compare/v0.10.19...v0.10.20
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
