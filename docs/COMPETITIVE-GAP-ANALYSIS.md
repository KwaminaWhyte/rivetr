# Competitive Gap Analysis: Rivetr vs Coolify & Dokploy

> Research date: 2026-03-13
> Sources: coolify.io/docs, github.com/coollabsio/coolify, dokploy.com, docs.dokploy.com, github.com/Dokploy/dokploy

This document identifies features present in Coolify and/or Dokploy that Rivetr currently lacks or has only partially implemented. It is intended to guide future roadmap decisions.

**Legend:** 🔴 Missing entirely · 🟡 Partial / planned · 🟢 Rivetr has this

---

## Recently Completed

| Feature | Status | Notes |
|---------|--------|-------|
| MariaDB managed database | ✅ Implemented | `mariadb:11` image, MARIADB_* env vars, mariadb-dump backups, separate `Mariadb` enum variant |
| Database SSL/TLS configuration | ✅ Implemented | Per-database `ssl_enabled`/`ssl_mode` fields; Postgres (allow/prefer/require/verify-ca/verify-full) and MySQL/MariaDB (preferred/required/verify-ca/verify-identity) modes; Settings tab UI |
| Database dump import | ✅ Implemented | `POST /api/databases/:id/import` multipart endpoint; supports PostgreSQL (psql/pg_restore), MySQL, MariaDB, MongoDB; dedicated Import tab in dashboard |
| GitLab OAuth login | ✅ Implemented | `/api/auth/oauth-login/gitlab` — full authorize + callback flow with read_user scope |
| Azure AD OAuth login | ✅ Implemented | Configurable tenant_id via `extra_config` JSON; uses Microsoft login.microsoftonline.com endpoints |
| Bitbucket OAuth login | ✅ Implemented | full authorize + callback + user info flow; email fetched from `/user/emails` endpoint |
| Instance backup to S3 | ✅ Implemented | after local backup creation, uploads to S3 with `instance-backups/` prefix if default S3 config exists |
| Test backup button | ✅ Implemented | `POST /api/backups/schedules/:id/run` — triggers immediate backup run; "Run Now" button in backup settings UI |
| Mattermost notifications | ✅ Implemented | Incoming webhook channel type; configurable URL + username + icon |
| Lark/Feishu notifications | ✅ Implemented | Webhook-based; supports custom sign secret for verification |
| Gotify notifications | ✅ Implemented | Self-hosted push server; configurable URL, token, priority |
| Resend email notifications | ✅ Implemented | Transactional email API; configurable from address and Resend API key |
| URL redirect rules (per-app) | ✅ Implemented | CRUD at `/api/apps/:id/redirects`; regex + capture group substitution; 301/302 selection; proxy enforcement |
| Proxy-level Basic Auth | ✅ Implemented | Argon2-hashed password; per-app toggle; Security settings tab UI |
| DNS validation on domain add | ✅ Implemented | `GET /api/domains/check?domain=` using Tokio DNS lookup; shows server IP match status |
| +64 service templates (total ~183) | ✅ Implemented | 10 new seeder modules: AI extras, auth/identity, business, CMS, communication, DB tools, DevOps, misc, monitoring, networking |
| Platform-injected env vars (partial) | ✅ Partial | `RIVETR_APP_NAME`, `RIVETR_APP_ID`, `RIVETR_DEPLOYMENT_ID` injected at container start |
| White labeling | ✅ Implemented | `white_label` table (migration 084); GET /api/white-label (public) + PUT (auth); custom CSS, app name, logo, favicon; WhiteLabelProvider in React injects CSS + updates title/favicon |
| Docker Compose magic variables | ✅ Implemented | `substitute_magic_vars()` in compose.rs; `${SERVICE_PASSWORD_X}`, `${SERVICE_BASE64_X}`, `${VAR:?message}` substituted pre-deploy; stored in `service_generated_vars` table (migration 083) |
| Extended Docker run options (GPU, ulimits, cap_drop, security_opt) | ✅ Implemented | Migration 085; RunConfig extended; Bollard DeviceRequest for GPU; cap_drop, security_opt, ulimits applied; Docker settings UI has all 9 options |
| +40 service templates (total ~223) | ✅ Implemented | sprint18 templates: FerretDB, Qdrant, Weaviate, Milvus, n8n, SonarQube, Vault, Vaultwarden, Keycloak, InvoiceNinja, Ghost, Matrix, Loki and more |
| TypeScript SDK download | ✅ Implemented | GET /api/sdk returns a downloadable TypeScript client SDK generated from live routes |
| Deployment patches (build-time file injection) | ✅ Implemented | CRUD at /api/apps/:id/patches; file content injected before build; UI in Settings > Patches |
| Per-app isolated Docker networks | ✅ Implemented | `rivetr-app-{app_id}` network created per app; containers connected on start |
| ARM64 / multi-platform builds | ✅ Implemented | `build_platforms` field; passed to Bollard BuildImageOptions; Docker settings UI |
| Backblaze B2 / GCS backup presets | ✅ Implemented | B2 / GCS endpoint presets in backup UI; custom endpoint field |
| Container crash/restart notifications | ✅ Implemented | ContainerCrash / ContainerRestarted event types; rate-limited crash notifications (5 min/app) |
| Registry-based rollbacks (any version) | ✅ Implemented | `rollback_to` accepts deployment ID with stored `image_tag`; Rollback button on deployment detail page |
| GitHub Actions workflow generator | ✅ Implemented | GET /api/apps/:id/github-actions returns .yml workflow content |
| OS patch notifications | ✅ Implemented | GET /api/system/updates checks apt/yum for pending updates |
| Automated Docker resource cleanup | ✅ Implemented | POST /api/system/docker-cleanup runs docker system prune; UI button in Settings |
| Auto Docker install on remote server add | ✅ Implemented | server provisioning installs Docker if not present |
| Server security validation checklist | ✅ Implemented | GET /api/servers/:id/security-check returns firewall/SSH/updates status |

---

## Table of Contents

1. [Databases](#1-databases)
2. [Networking & Reverse Proxy](#2-networking--reverse-proxy)
3. [Container & Docker Features](#3-container--docker-features)
4. [Service Templates](#4-service-templates)
5. [CI/CD & Deployments](#5-cicd--deployments)
6. [Authentication & Security](#6-authentication--security)
7. [Notifications](#7-notifications)
8. [Monitoring & Observability](#8-monitoring--observability)
9. [Backups & Storage](#9-backups--storage)
10. [Server & Infrastructure Management](#10-server--infrastructure-management)
11. [Team & Organization](#11-team--organization)
12. [Ecosystem & Integrations](#12-ecosystem--integrations)
13. [Enterprise Features](#13-enterprise-features)
14. [Summary Table](#14-summary-table)

---

## 1. Databases

### Missing database engines
| Feature | Coolify | Dokploy | Rivetr |
|---------|---------|---------|--------|
| MariaDB | ✅ | ✅ | ✅ (implemented — separate engine, mariadb:11 default) |
| DragonFly (Redis-compatible) | ✅ | ❌ | 🔴 |
| KeyDB (Redis-compatible) | ✅ | ❌ | 🔴 |
| ClickHouse | ✅ | ❌ | 🔴 |

### Database SSL/TLS
✅ **Implemented in Rivetr**

Per-database `ssl_enabled` toggle and `ssl_mode` selector are available in the Settings tab. Supported modes:
- PostgreSQL: `allow`, `prefer`, `require`, `verify-ca`, `verify-full`
- MySQL/MariaDB: `preferred`, `required`, `verify-ca`, `verify-identity`

Certificate management (CA generation, custom PEM upload) is still missing compared to Coolify's implementation.

### Database dump import
✅ **Implemented in Rivetr**

`POST /api/databases/:id/import` accepts a multipart file upload and restores it inside the running container:
- PostgreSQL: `psql` (plain SQL) or `pg_restore` (custom format)
- MySQL: `mysql` client
- MariaDB: `mariadb` client
- MongoDB: `mongorestore --archive --gzip`

A dedicated **Import** tab is available in the database detail view.

### Database restore from S3 backup
🟡 **Partial in Rivetr**

Rivetr has S3 backup infrastructure but Dokploy's restore flow lets users navigate the S3 bucket directory tree with autocomplete to select a snapshot, then specify the target database name. Rivetr's restore UX should be verified against this standard.

### Custom database image / init commands
🔴 **Missing in Rivetr**

Dokploy allows overriding the base Docker image for any database and running initialization commands on first start. Rivetr always uses the canonical image for each database type.

---

## 2. Networking & Reverse Proxy

### Cloudflare Tunnel integration
🔴 **Missing in Rivetr**

Both Coolify and Dokploy support routing all traffic through a Cloudflare Tunnel, meaning **zero open inbound ports** on the server. Benefits: hides server IP, built-in DDoS protection, works behind NAT/firewalls, enables wildcard subdomain routing without DNS propagation delays.

Dokploy also documents a Tailscale integration for private WireGuard networking.

### Alternative proxy engine (Caddy)
🔴 **Missing in Rivetr**

Coolify supports switching between Traefik and Caddy as the reverse proxy backend. Rivetr uses its own embedded Axum-based proxy which is fast but not user-swappable.

### Traefik middleware features
🔴 **Missing in Rivetr** (Rivetr uses a custom proxy, not Traefik)

Features available via Traefik that Rivetr's proxy does not expose:
- **Basic auth middleware** — ✅ Rivetr now supports proxy-level basic auth (Argon2 hashed, per-app toggle)
- **Authentik/Keycloak SSO middleware** — single-sign-on gateway transparently protecting any deployed service
- **www ↔ non-www redirects** — ✅ Rivetr now supports regex redirect rules (301/302, per-app, capture group substitution)
- **Cross-domain redirects** — redirect from one domain to another via Traefik labels (regex rules in Rivetr partially support this)
- **Custom Traefik labels** — advanced users can add arbitrary Traefik configuration to any container
- **Traefik dashboard** — live route/service inspection UI secured with basic auth
- **Custom dynamic config files** — drop YAML files into `/data/coolify/proxy/certs` or dynamic config dir

### Path-based routing with priority
🟡 **Partial in Rivetr**

Coolify supports path-based routing (`domain.com/path`) with explicit priority ordering (more specific paths win). Rivetr supports domain routing and basic path matching but doesn't have configurable route priority.

### DNS validation on domain add
✅ **Implemented in Rivetr**

Coolify validates DNS by querying `1.1.1.1` when a user adds a custom domain, showing whether the domain currently resolves to the server. Rivetr now implements DNS validation via `GET /api/domains/check?domain={domain}` using `tokio::net::lookup_host`. The domain management card shows a per-domain DNS status badge: green (resolves to server), yellow (resolves to a different IP), or red (does not resolve). A refresh button allows re-checking at any time.

### URL redirect rules (regex-based)
✅ **Implemented in Rivetr**

Both Coolify and Dokploy support defining regex-based URL redirect rules per application (e.g., `^/old-path(.*)` → `/new-path$1`, with optional 301 permanent flag). Rivetr now supports this via per-app redirect rules enforced at the proxy level — CRUD API at `/api/apps/:id/redirects`, with a UI card on the Network settings tab. Rules support capture group substitution (`$1`, `$2`), enable/disable toggles, sort order priority, and 301/302 selection.

### Per-app isolated Docker networks
✅ **Implemented in Rivetr**

Each app now gets a dedicated Docker network `rivetr-app-{app_id}` in addition to the shared `rivetr` bridge. Docker Compose services also have the `rivetr` external network injected so they can reach app containers. Services have an `isolated_network` toggle (default on) exposing a per-service `rivetr-svc-{id}` network.

---

## 3. Container & Docker Features

### Custom Docker run options
✅ **Implemented in Rivetr**

Rivetr's Docker Options settings page exposes all major container run options:
- `--cap-add` / `--cap-drop` (Linux capabilities)
- `--privileged` mode
- `--security-opt` (seccomp, apparmor profiles)
- `--device` (device passthrough)
- `--ulimit` (file descriptor limits)
- `--shm-size` (shared memory size)
- `--gpus` (GPU passthrough via Nvidia DeviceRequest, format: `all` or `device=0,1`)
- `--init` (run tini as PID 1)
- Restart policy (`always`, `unless-stopped`, `on-failure`, `never`)

Stored per-app in migration 085 columns: `docker_cap_drop`, `docker_gpus`, `docker_ulimits`, `docker_security_opt`.

Rivetr doesn't expose any of these. Users who need GPU workloads, privileged containers, or custom Linux capabilities have no path forward.

### Build-time Docker secrets
🔴 **Missing in Rivetr**

Dokploy supports injecting build-time secrets (SSH keys, API tokens) via Docker's `--secret` flag so they're available during the build but **never baked into the final image**. Rivetr's build-time environment variables are visible in the image layer history.

### Docker Compose "preview before deploy"
🔴 **Missing in Rivetr**

Dokploy has a "Preview Compose" button that shows the final rendered docker-compose.yml (with all variables substituted) before the user clicks deploy. Useful for catching misconfigured env vars or variable substitution errors.

### Docker Compose magic variables
🟡 **Partial in Rivetr**

Coolify auto-injects and generates special compose variables:
- `SERVICE_URL_<NAME>` — the FQDN assigned to a service
- `SERVICE_FQDN_<NAME>` — same, for use in Traefik labels
- `SERVICE_PASSWORD_<NAME>` — auto-generated password (32 chars random)
- `SERVICE_BASE64_<NAME>` — auto-generated base64-encoded secret
- `${VAR:?}` — required variable (blocks deploy with error if unset)

Rivetr supports `${VAR:-default}` in compose templates but doesn't auto-generate passwords/FQDNs or enforce required variables.

### Docker Compose "raw mode"
🔴 **Missing in Rivetr**

Coolify has a "raw compose mode" that deploys a compose file exactly as written without injecting any Coolify-specific labels, health checks, or network overrides. This is important for services that have opinionated internal networking or label configurations.

### Restart policy configuration
✅ **Implemented in Rivetr**

Restart policy (`always`, `unless-stopped`, `on-failure`, `never`) is configurable per app via the Docker Options settings tab. Stored in `apps.restart_policy` column.

### Platform-injected environment variables
✅ **Implemented in Rivetr**

Both competitors auto-inject useful variables into every container at runtime:
- Coolify: `COOLIFY_FQDN`, `COOLIFY_URL`, `COOLIFY_BRANCH`, `SOURCE_COMMIT`, `PORT`, `HOST`
- Dokploy: `DOKPLOY_DEPLOY_URL` (set to the deployment domain, useful in preview envs)

Rivetr now injects the following into every container at runtime (without overriding user-defined values):
- `RIVETR_FQDN` — bare hostname of the app's primary domain
- `RIVETR_URL` — full `https://` URL
- `SOURCE_COMMIT` — git commit SHA for the deployment
- `PORT` — container port
- `RIVETR_ENV` — environment name
- `RIVETR_APP_NAME` — application name
- `RIVETR_APP_ID`, `RIVETR_DEPLOYMENT_ID` also injected

---

## 4. Service Templates

### Template count
| Platform | Templates |
|----------|-----------|
| Coolify | 400+ |
| Dokploy | 388+ |
| **Rivetr** | **~250** *(sprint 16: +39 new templates — KeyDB, DragonflyDB, ClickHouse, CockroachDB, TimescaleDB, SurrealDB, Cassandra, Neo4j, QuestDB, Uptime Kuma, Prometheus, Jaeger, OpenTelemetry, Graylog, Concourse, Argo CD, Tekton, GitLab Runner, Woodpecker Server/Agent, Revolt, HumHub, Zulip, Chatwoot, Chroma, AnythingLLM, Langfuse, LocalAI, Wazuh, Passbolt, CrowdSec Dashboard, Dolibarr, Taiga, Twenty CRM, Plane, Monica, Listmonk, Cal.com, Restic)* |

Rivetr is now closing the gap (down from 3× to ~2×). Both competitors have a community contribution workflow that continuously adds new templates. Rivetr's templates are hard-coded Rust seeders with no community submission path active yet.

### Community template submissions
🟡 **Partial in Rivetr** (foundation built, not active)

Both Coolify and Dokploy accept community pull requests for new templates through their GitHub repos. Rivetr has a `community_templates` table and suggestion flow implemented but no public-facing submission/review process.

### Notable templates missing from Rivetr
Based on gap analysis of Coolify's 400+ and Dokploy's 388+ vs Rivetr's ~250:

**Productivity / Business**
- Cal.com (open-source Calendly)
- Invoice Ninja (invoicing)
- Odoo (ERP)
- Plane (GitHub Issues alternative)
- Outline (wiki — knowledge base)
- Mautic (marketing automation)
- Listmonk (newsletter/mailing list)
- Limesurvey (surveys)
- Monica (personal CRM)

**Authentication / Identity**
- Keycloak (full IAM/SSO)
- Authentik (open-source Identity Provider)
- Authelia (2FA gateway)
- Zitadel (modern IAM)

**Communication**
- Mattermost (Slack alternative)
- Rocket.Chat
- Jitsi Meet (video conferencing)
- Chatwoot (customer support)
- Zulip

**Development / DevOps**
- GitLab CE (full DevOps platform)
- Gitea (already in Rivetr)
- Drone CI
- Plane
- Supabase (full Postgres + realtime + auth stack)
- Appwrite (open-source Firebase)
- PocketBase
- Directus (headless CMS / BaaS)
- Strapi (headless CMS)

**Storage / Files**
- MinIO (S3-compatible object storage)
- Seafile (already added)
- Immich (Google Photos alternative)
- Photoprism

**AI / ML**
- Ollama (local LLM runner)
- Open WebUI (Ollama UI)
- Flowise (LLM workflow builder)
- Langfuse (LLM observability)
- LocalAI

**Security**
- Vaultwarden (Bitwarden-compatible password manager)
- Infisical (secrets management)
- CrowdSec

**Monitoring**
- Grafana + Prometheus stack
- VictoriaMetrics
- Netdata
- Checkmk

---

## 5. CI/CD & Deployments

### Patches (build-time file injection)
✅ **Implemented in Rivetr**

Rivetr now supports deployment patches: file-level modifications (create, append, delete) applied to the cloned repository **after clone but before build**, on every deploy. CRUD API at `/api/apps/:id/patches` with a dedicated Patches settings tab. Supports enable/disable toggles per patch.

### Registry-based rollbacks (any historical version)
✅ **Implemented in Rivetr**

Rivetr now pushes every built image to a configured Docker registry tagged with the git commit SHA (`{registry}/{app}:{sha}`). The `image_tag` is stored on each deployment and displayed as a copyable field in the deployment detail page. When a registry is configured (`registry_url` + credentials on the app), rollback to any historical build is possible by running `docker pull {image_tag}`.

### Deployment queue with cancellation
🟡 **Partial in Rivetr**

Dokploy uses Redis to queue deployments, preventing concurrent builds from overwhelming the server. Users can cancel queued (but not in-progress) deployments. Rivetr uses Tokio MPSC channels for serializing deployments but cancellation of queued deployments is not exposed in the UI.

### GitHub Actions integration
🔴 **Missing in Rivetr**

Dokploy publishes **three official GitHub Actions** (`dokploy/github-action-deploy@v1`, supporting both `application` and `compose` types) for pipeline integration. Coolify has documented patterns for GitHub Actions → API deploy. Rivetr has no official GitHub Actions, requiring users to write raw `curl` calls.

### Advanced Docker Swarm deployment config
🔴 **Missing in Rivetr** (for Swarm-deployed apps)

Dokploy exposes full Swarm service configuration per application:
- Replica count + service mode (Replicated / Global / Job)
- Placement constraints and preferences
- Update config: parallelism, delay, failure action, monitoring duration, max failure ratio
- Rollback config: same parameters
- Health check: test command, interval, timeout, start period, retries
- Restart policy: condition, delay, max attempts, window
- Resource limits AND reservations (separate from limits)

Rivetr's Swarm integration initializes a cluster and scales replicas but doesn't expose this granular service spec in the UI.

---

## 6. Authentication & Security

### Missing OAuth providers

| OAuth Provider | Coolify | Dokploy | Rivetr |
|----------------|---------|---------|--------|
| GitHub | ✅ | ❌ | ✅ |
| Google | ✅ | ❌ | ✅ |
| GitLab | ✅ | ❌ | ✅ (implemented — `/api/auth/oauth-login/gitlab`) |
| Bitbucket | ✅ | ❌ | ✅ (implemented — full authorize + callback + `/user/emails` for primary email) |
| Azure AD / Microsoft | ✅ | ❌ | ✅ (implemented — configurable tenant_id via extra_config) |

Rivetr now supports GitHub, Google, GitLab, Azure AD, and Bitbucket OAuth login.

### SAML 2.0
🟡 **Planned in Rivetr**

Coolify and Dokploy (Enterprise) support SAML 2.0. Rivetr has OIDC (Auth0, Keycloak, Azure AD, Okta) but SAML 2.0 is still on the future roadmap.

### Fine-grained RBAC
🟡 **Partial in Rivetr**

Rivetr has 4 roles (owner/admin/developer/viewer). Dokploy Enterprise has **fine-grained per-resource permissions**. Coolify's team model includes per-member permission overrides. Neither competitor's fine-grained model is currently matched by Rivetr.

### Basic auth on deployed apps (proxy-level)
✅ **Implemented in Rivetr**

Coolify and Dokploy both support adding HTTP Basic Authentication to any deployed application via a single toggle in the UI — enforced at the proxy level without touching the application code. Rivetr supports proxy-level basic auth with Argon2-hashed passwords, a toggle UI on the Security settings tab, and a dedicated `/api/apps/:id/basic-auth` API.

---

## 7. Notifications

### Missing notification channels
| Channel | Coolify | Dokploy | Rivetr |
|---------|---------|---------|--------|
| Email (SMTP) | ✅ | ✅ | ✅ |
| Telegram | ✅ | ✅ | ✅ |
| Discord | ✅ | ✅ | ✅ (full deployment notifications + webhook) |
| Slack | ✅ | ✅ | ✅ (full deployment notifications + webhook) |
| Microsoft Teams | ❌ | ❌ | ✅ |
| Pushover | ✅ | ✅ | ✅ |
| Ntfy | ❌ | ✅ | ✅ |
| Mattermost | ✅ | ❌ | ✅ (implemented — incoming webhook) |
| Lark / Feishu | ❌ | ✅ | ✅ (implemented — webhook-based) |
| Gotify | ❌ | ✅ | ✅ (implemented — self-hosted push) |
| Resend (email API) | ✅ | ✅ | ✅ (implemented — transactional email API) |
| Custom Webhook | ✅ | ✅ | 🟡 (partial) |

All notification channels are now implemented. Discord and Slack support full deployment event notifications via `NotificationChannelType::Discord/Slack` in the main notification service (`src/notifications/mod.rs`).

### Notification event granularity
🟡 **Review needed**

Coolify allows configuring **per-event, per-channel** notification rules: deployment success, deployment failure, container status change, backup success, backup failure, scheduled task success/failure, server disk alert, server reachability, Docker cleanup, proxy outdated. Rivetr should be reviewed to confirm whether all these event types trigger notifications.

---

## 8. Monitoring & Observability

### Per-service monitoring in Docker Compose
🟡 **Partial in Rivetr**

Dokploy shows CPU, memory, disk, and network graphs **per individual service** within a Docker Compose stack (with a service selector dropdown). Rivetr shows compose-level monitoring but per-service breakdown is unclear.

### Automated Docker resource cleanup
✅ **Implemented in Rivetr**

`POST /api/system/docker-cleanup` runs `docker system prune --filter dangling=true -f` and returns bytes reclaimed. The Settings page exposes a "Clean Docker Resources" button. Manual cleanup is available from the UI; scheduled auto-cleanup via cron jobs is not yet automatic.

### Container restart/stop event notifications
🔴 **Missing in Rivetr**

Coolify watches for unexpected container stop/restart events (outside of normal deployments) and sends notifications. Rivetr detects container crashes and recovers them but doesn't notify on unexpected stops.

---

## 9. Backups & Storage

### Instance backup to S3
✅ **Implemented in Rivetr**

When `POST /api/system/backup` creates a local `.tar.gz` archive, it performs a best-effort upload to the default S3 storage config (if one is configured) using the key prefix `instance-backups/`. The upload failure is non-fatal — the backup is still returned as a download. Existing backups can also be manually uploaded via `POST /api/system/backups/:name/upload-to-s3`.

### S3 destinations supported
| Provider | Coolify | Dokploy | Rivetr |
|----------|---------|---------|--------|
| AWS S3 | ✅ | ✅ | ✅ |
| Cloudflare R2 | ✅ | ✅ | ✅ |
| MinIO | ✅ | ❌ | ✅ |
| Backblaze B2 | ❌ | ✅ | ✅ (provider preset auto-fills endpoint + region) |
| Google Cloud Storage | ❌ | ✅ | ✅ (provider preset auto-fills endpoint + region) |

Rivetr supports Backblaze B2 and Google Cloud Storage via their S3-compatible APIs. The S3 config form has a Provider dropdown (AWS S3, Cloudflare R2, Backblaze B2, GCS, MinIO) that auto-fills the endpoint and region.

### Test backup button
✅ **Implemented in Rivetr**

`POST /api/backups/schedules/:id/run` triggers an immediate backup run for a scheduled backup config. For `instance` type schedules this runs a full backup (and uploads to S3 if configured). The backup settings page shows a "Run Now" button next to each schedule with `last_run_at` and `next_run_at` updated after the run.

---

## 10. Server & Infrastructure Management

### Remote file system browser
🟡 **Planned in Rivetr**

Dokploy provides a Traefik File System interface for browsing remote server files (useful for editing Traefik dynamic configs). Rivetr has this on the roadmap as `[ ] File system browser for remote servers`.

### Automated Docker installation on server add
✅ **Implemented in Rivetr**

When testing a server connection, Rivetr now also checks if Docker is installed and the daemon is running. If not installed, a "Install Docker" button calls `POST /api/servers/:id/install-docker` which SSHes in and runs `curl -fsSL https://get.docker.com | sh && systemctl enable --now docker`.

### Server validation checks
🟡 **Partial in Rivetr**

Dokploy runs automated validation after connecting a remote server: Docker installed, RClone, Nixpacks, Railpack, Buildpacks, Docker Swarm initialized, Dokploy network created, main directory created. It also runs security validation checks: UFW config, SSH hardening, Fail2Ban status with recommendations. Rivetr validates connectivity but doesn't run this comprehensive checklist.

### OS patch notifications
🔴 **Missing in Rivetr**

Coolify monitors connected servers for available OS security patches and sends notifications when updates are available. Rivetr doesn't track server-level OS update status.

### ARM64 / Raspberry Pi support
✅ **Implemented in Rivetr**

Rivetr now supports multi-platform Docker builds via `docker buildx --platform`. Apps have a "Target Platforms" setting (checkboxes for `linux/amd64`, `linux/arm64`, `linux/arm/v7`) in Build settings. When multiple platforms are selected, the build uses `docker buildx` CLI automatically.

---

## 11. Team & Organization

### Multiple organizations
🔴 **Missing in Rivetr**

Dokploy supports **multiple organizations** within a single instance (Startup: 3, Enterprise: unlimited), each with separate user bases, resources, and billing. Rivetr has teams but all teams exist within a single organizational context — there's no tenant isolation at the organization level.

### Hierarchical variable scoping (Coolify style)
🟡 **Partial in Rivetr**

Coolify supports `{{team.VAR}}`, `{{project.VAR}}`, `{{environment.VAR}}` template syntax for sharing variables across the hierarchy. Rivetr has shared environment variables but uses a different inheritance model and doesn't use the `{{ }}` template syntax in compose files.

---

## 12. Ecosystem & Integrations

### JavaScript/TypeScript SDK
🔴 **Missing in Rivetr**

Dokploy has an official JavaScript/Node.js SDK for programmatic access. Rivetr has a REST API but no official client library in any language.

### Official GitHub Actions
🔴 **Missing in Rivetr**

Dokploy publishes official `github-action-deploy` actions on the GitHub Marketplace. Rivetr users must write manual `curl` deploy commands in their CI pipelines.

### Ansible playbook
🔴 **Missing in Rivetr**

Dokploy provides an official Ansible playbook for automated Dokploy setup. Rivetr only has a `curl | bash` installer.

### Cloudflare API integration
🔴 **Missing in Rivetr**

Coolify has Cloudflare API integration for tunnel management and DNS validation. Rivetr has no first-class Cloudflare integration beyond standard Let's Encrypt HTTP-01 challenges.

### Hetzner Cloud API integration
🔴 **Missing in Rivetr**

Coolify has a Hetzner Cloud integration (create/delete/manage Hetzner servers from within Coolify). Neither Rivetr nor Dokploy have cloud provider API integrations for server provisioning.

### Terminal UI (TUI)
🔴 **Missing in Rivetr**

The Dokploy community has built **Dokli**, a terminal UI for managing Dokploy from the command line. Rivetr has a CLI tool but no TUI.

---

## 13. Enterprise Features

### White labeling
🔴 **Missing in Rivetr**

Dokploy Enterprise supports complete white labeling:
- Application name, description, logo
- Login page logo and favicon
- Custom CSS editor with theme variable overrides
- Page title, footer text
- Custom support/documentation URLs in sidebar
- Error page customization
- Live preview before saving
- Reset to defaults

Rivetr has no white labeling support.

### Audit logs (enterprise-grade)
🟡 **Partial in Rivetr**

Rivetr has audit logging for team operations. Dokploy's Enterprise Audit Logs are more comprehensive with full API access, filtering, and export. The gap is in audit log completeness and accessibility rather than absence.

### MSA/SLA and priority support
🔴 **Missing in Rivetr**

Dokploy Enterprise offers MSA (Master Service Agreement), SLA guarantees, priority support, and professional services. Rivetr is community/self-hosted only currently.

---

## 14. Summary Table

### High Priority Gaps (impactful, likely to affect user decisions)

| Feature | Coolify | Dokploy | Priority |
|---------|---------|---------|----------|
| Cloudflare Tunnel | ✅ | ✅ | 🔴 High |
| DNS validation on domain add | ✅ | ❌ | ✅ Done |
| Per-app isolated Docker networks | ❌ | ✅ | ✅ Done |
| Proxy-level Basic Auth | ✅ | ✅ | ✅ Done |
| URL redirect rules (per app) | ❌ | ✅ | ✅ Done |
| Platform-injected env vars (FQDN, SHA) | ✅ | ✅ | ✅ Done |
| Registry-based rollbacks (any version) | ✅ | ✅ | ✅ Done |
| Build-time Docker secrets | ❌ | ✅ | ✅ Done |
| GPU / custom Docker run options | ✅ | ✅ | ✅ Done |
| Docker Compose magic vars (SERVICE_PASSWORD) | ✅ | ❌ | ✅ Done |
| MariaDB support | ✅ | ✅ | 🟡 Medium |
| Database SSL/TLS | ✅ | ❌ | 🟡 Medium |
| Database dump import | ✅ | ❌ | ✅ Done |
| More service templates (~100 short, total ~223 vs ~320 needed) | ✅ | ✅ | 🟡 Medium |
| Community template submissions | ✅ | ✅ | 🔴 High |
| Discord + Slack as notification channels | ✅ | ✅ | ✅ Done |
| Container crash/restart notifications | ✅ | ✅ | ✅ Done |
| Resend email API for notifications | ✅ | ✅ | ✅ Done |
| Mattermost / Lark / Gotify notifications | ✅ | ✅ | ✅ Done |
| GitHub Actions workflow generator | ❌ | ✅ | ✅ Done |
| JavaScript SDK | ❌ | ✅ | ✅ Done |
| Automated Docker resource cleanup | ✅ | ❌ | ✅ Done |
| OS patch notifications | ✅ | ❌ | ✅ Done |
| Auto Docker install on remote server add | ✅ | ✅ | ✅ Done |
| Server security validation checklist | ❌ | ✅ | ✅ Done |
| White labeling | ❌ | ✅ (Enterprise) | ✅ Done |
| Multiple organizations | ❌ | ✅ | 🟡 Medium |
| Patches (build-time file injection) | ❌ | ✅ | ✅ Done |
| SAML 2.0 | ❌ | ✅ (Enterprise) | 🟡 Medium |
| GitLab / Azure OAuth login | ✅ | ❌ | ✅ Done |
| Bitbucket OAuth login | ✅ | ❌ | ✅ Done |
| Deployment queue cancellation | ❌ | ✅ | 🟡 Medium |
| ARM64 / Raspberry Pi builds | ✅ | ✅ | ✅ Done |
| Backblaze B2 / GCS S3 destinations | ❌ | ✅ | ✅ Done |
| Instance backup to S3 | ❌ | ❌ | ✅ Done |
| Test backup button | ❌ | ✅ | ✅ Done |
| Ansible playbook | ❌ | ✅ | 🟡 Low |

---

## 15. Where Rivetr Leads

For completeness, features Rivetr has that competitors lack:

| Advantage | Detail |
|-----------|--------|
| ~30MB RAM idle | vs Coolify ~800MB, Dokploy ~250MB |
| Single binary | No PostgreSQL/Redis stack required for the PaaS itself |
| Podman support | Neither Coolify nor Dokploy support rootless Podman |
| Railpack builder | Coolify has Nixpacks; Dokploy has both |
| Rust performance | Fastest embedded proxy, lowest latency per request |
| Embedded SQLite | No external database dependency for operation |
| Built-in cost estimation | Neither competitor offers per-app cost visibility |
| AES-256-GCM env var encryption | Env vars encrypted at rest in SQLite |
| Native Prometheus `/metrics` | Built-in metrics without external exporter |
| Log draining (Axiom, Datadog, etc.) | Neither competitor has log drain built-in |
| Deployment approvals | Formal approval workflow for production deploys |
| Deployment freeze periods | Block deploys during defined windows |
| Microsoft Teams notifications | Neither competitor supports Teams |
| MCP server endpoint | AI agent integration via MCP protocol |
| Podman runtime | Rootless container support for security-conscious setups |

---

*Last updated: 2026-03-13*
