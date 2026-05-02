# Implementation Plan: Competitive Feature Parity (Archive)

> **Note**: This document is archived. All sprints described here are complete as of March 2026.
> All features implemented in these sprints are now documented in [README.md](../../README.md).
> This file is kept for historical reference only.

---

> Execution plan for Phases 5-8 using parallel sub-agents for maximum throughput.

## Execution Strategy

Each sprint uses **multiple sub-agents running simultaneously** on independent feature streams. Features are grouped by:
1. **No shared code dependencies** - agents won't conflict on the same files
2. **Backend vs Frontend separation** - where possible, split backend and frontend work
3. **Complexity ordering** - quick wins first to build momentum

### Agent Types Used

| Agent | Role |
|-------|------|
| `general-purpose` | Feature implementation (backend + frontend) |
| `code-reviewer` | Post-implementation Rust code review |
| `frontend-reviewer` | Post-implementation React/TypeScript review |
| `test-runner` | Run tests after each sprint |
| `security-reviewer` | Audit auth-related features (OAuth, 2FA, SSO) |
| `debugger` | Fix compilation errors or test failures |

---

## Sprint 1: Quick Wins + Preview Deployments

**Goal**: Close easy gaps + deliver the #1 missing feature

### Parallel Streams (5 agents)

#### Agent 1: Preview Deployments (T5.3.1-7)
**Files**: `src/api/webhooks.rs`, `src/engine/pipeline.rs`, `src/engine/preview.rs` (new), `src/proxy/`, `frontend/app/routes/apps/`
- Parse PR open/sync/close/merge events from GitHub, GitLab, Gitea webhooks
- Create `preview_deployments` table (id, app_id, pr_number, subdomain, deployment_id, status)
- Generate unique subdomain: `pr-{number}.{app_name}.{domain}`
- Register preview subdomain in proxy routing table
- Build and deploy preview container (reuse existing pipeline)
- Auto-delete container and proxy route on PR close/merge
- POST preview URL as comment on GitHub PR via API
- Frontend: Add preview deployments tab to app detail page

#### Agent 2: Watch Paths (T7.3.1-6)
**Files**: `src/api/webhooks.rs` (webhook filtering logic), `src/db/models.rs`, `migrations/`, `frontend/app/components/`
- Add `watch_paths` JSON column to apps table
- Parse `commits[].added/modified/removed` from webhook push payload
- Implement glob matching against watch path patterns
- Skip deployment if no matched files (log reason, return 200)
- Add WatchPathsCard to app settings UI

#### Agent 3: Bitbucket Webhooks (T7.4.1-7)
**Files**: `src/api/webhooks_bitbucket.rs` (new), `src/api/mod.rs`, `frontend/app/routes/settings/`
- Implement Bitbucket webhook signature verification
- Parse `repo:push` and `pullrequest:*` event payloads
- Register route: POST /api/webhooks/bitbucket
- Add Bitbucket tab to Git Providers settings page

#### Agent 4: Notification Channels - Telegram + MS Teams (T7.5.1-2)
**Files**: `src/notifications/telegram.rs` (new), `src/notifications/teams.rs` (new), `src/notifications/mod.rs`, `frontend/app/components/notifications/`
- Telegram: Bot API integration (sendMessage to chat_id with optional topic)
- MS Teams: Incoming webhook POST with adaptive card payload
- Add to notification_channels CHECK constraint migration
- Channel configuration forms in notification settings UI

#### Agent 5: Instance Backup (T7.7.1-4)
**Files**: `src/api/system.rs`, `src/db/backup.rs` (new), CLI
- Backup: copy SQLite DB (with WAL checkpoint) + rivetr.toml + SSL certs → tar.gz
- API endpoint: POST /api/system/backup (returns download)
- Scheduled backup via background task (configurable interval)
- CLI: `rivetr backup` and `rivetr restore <file>`

### Post-Sprint
- Run `test-runner` agent
- Run `code-reviewer` on all new Rust code
- Run `frontend-reviewer` on all new React code

---

## Sprint 2: Auth + Environments

**Goal**: OAuth login and project environments - high-impact UX features

### Parallel Streams (4 agents)

#### Agent 1: OAuth Login (T7.1.1-7)
**Files**: `src/api/auth.rs`, `src/api/oauth.rs` (new), `src/db/models.rs`, `migrations/`, `frontend/app/routes/login.tsx`
- Create `oauth_providers` table and `user_oauth_connections` table
- GitHub OAuth: authorize URL → callback → exchange code → upsert user
- Google OAuth: same flow with Google endpoints
- Account linking for existing users
- OAuth provider configuration in Settings > Authentication
- Login page OAuth buttons (conditionally shown based on enabled providers)

#### Agent 2: Project Environments (T7.2.1-9)
**Files**: `src/api/environments.rs` (new), `src/db/models.rs`, `migrations/`, `frontend/app/routes/projects/`
- Create `environments` table with project_id FK
- Create `env_variables_environment` table for env-scoped vars
- Add `environment_id` FK to `apps` table
- CRUD API: /api/projects/:id/environments
- Auto-create default environments (production, staging, development) on project create
- Environment tabs/switcher in project UI
- Predefined variables: RIVETR_ENV, RIVETR_URL

#### Agent 3: Two-Factor Auth (T8.3.1-6)
**Files**: `src/api/auth.rs`, `src/api/two_factor.rs` (new), `migrations/`, `frontend/app/routes/settings/security.tsx`
- TOTP secret generation (encrypted storage)
- QR code generation for authenticator app setup
- 2FA verification middleware on login
- Recovery codes (10 codes, hashed storage)
- 2FA settings page: enable, disable, view recovery codes
- Team-level 2FA enforcement setting

#### Agent 4: Notification Channels - Pushover + Ntfy (T7.5.3-4)
**Files**: `src/notifications/pushover.rs` (new), `src/notifications/ntfy.rs` (new), `frontend/app/components/notifications/`
- Pushover: REST API with user key + app token
- Ntfy: POST to topic URL with priority and tags
- Configuration forms in notification UI

### Post-Sprint
- Run `security-reviewer` on OAuth + 2FA code
- Run `test-runner` agent
- Run `code-reviewer` + `frontend-reviewer`

---

## Sprint 3: Templates + Jobs + Deploy Options

**Goal**: Close the template gap, add scheduled jobs and flexible deploys

### Parallel Streams (4 agents)

#### Agent 1: Service Templates Batch 1 (T7.6.1-7) - ~30 templates
**Files**: `src/templates/` (template definitions), `frontend/app/components/templates/`
- AI/ML: Ollama, Open WebUI, LiteLLM, Langflow, Flowise, ChromaDB
- Analytics: Plausible, Umami, PostHog, Matomo
- Automation: Activepieces, Windmill, Trigger.dev
- CMS: WordPress, Ghost, Strapi, Directus, Payload CMS
- Communication: Rocket.Chat, Mattermost, Matrix/Synapse
- Dev Tools: Code Server, Supabase, Appwrite, Pocketbase, Hoppscotch, Forgejo
- Template category system + search in gallery

#### Agent 2: Service Templates Batch 2 (T7.6.8-15) - ~25 templates
**Files**: `src/templates/` (template definitions)
- Documentation: BookStack, Wiki.js, Docmost
- File/Media: Immich, Jellyfin, Navidrome, Seafile
- Monitoring: SigNoz, Beszel, Checkmate
- Security: Authentik, Keycloak, Vaultwarden, Infisical
- Search: Meilisearch, Typesense
- PM: Plane, Vikunja, Leantime, Cal.com
- Other: Paperless-ngx, Trilium, Linkwarden, Tandoor, Stirling-PDF

#### Agent 3: Scheduled Jobs (T7.8.1-7)
**Files**: `src/api/jobs.rs` (new), `src/engine/scheduler.rs` (new), `src/db/models.rs`, `migrations/`, `frontend/app/routes/apps/$app-id.jobs.tsx` (new)
- `scheduled_jobs` and `scheduled_job_runs` tables
- Background cron evaluator (check jobs every minute, execute due jobs)
- docker exec / podman exec to run commands in containers
- CRUD API: /api/apps/:id/jobs
- Job management UI: create, edit, enable/disable, history, output viewer

#### Agent 4: Deploy by Commit/Tag (T7.10.1-6)
**Files**: `src/api/deployments.rs`, `src/engine/pipeline.rs`, `src/utils/git.rs`, `frontend/app/components/deploy/`
- Add optional `commit_sha` and `git_tag` to deploy request body
- Checkout specific ref during git clone step
- Commits/tags list API (fetch from GitHub/GitLab API using stored tokens)
- Commit/tag selector dropdown in deploy modal

### Post-Sprint
- Run `test-runner` agent
- Run `code-reviewer` + `frontend-reviewer`

---

## Sprint 4: S3 + Monitoring + Log Draining

**Goal**: Production operations features

### Parallel Streams (3 agents)

#### Agent 1: S3 Backup Integration (T6.5.1-6)
**Files**: `src/backup/s3.rs` (new), `src/api/backups.rs`, `migrations/`, `frontend/app/routes/settings/backups.tsx`
- S3 client using `aws-sdk-s3` or `rusoto` crate
- S3 storage configuration table (id, name, endpoint, bucket, access_key, secret_key, region)
- Upload volume backups to S3
- Upload database dumps to S3
- Scheduled S3 backups via cron (reuse scheduler from Sprint 3)
- S3 restore: list backups from bucket, download and apply
- S3 settings UI: configure destination, test connection, manage backups

#### Agent 2: Advanced Monitoring (T6.4.1-6)
**Files**: `src/api/monitoring.rs` (new), `src/db/models.rs`, `migrations/`, `frontend/app/routes/monitoring/`
- Full-text log search across deployment_logs table
- Log retention policies table (per-app configurable)
- Scheduled container restarts (cron-based, reuse scheduler)
- Uptime tracking: periodic health check pings, store availability %
- Response time monitoring via health check latency
- Monitoring dashboard UI with search, uptime charts

#### Agent 3: Log Draining (T8.4.1-7)
**Files**: `src/logging/drain.rs` (new), `src/api/log_drains.rs` (new), `migrations/`, `frontend/app/routes/apps/$app-id.settings.tsx`
- Log drain config table (app_id, provider, config JSON, enabled)
- Axiom provider (HTTPS ingest API)
- New Relic provider (Log API)
- Generic HTTP POST provider (any endpoint)
- Intercept container log stream → fan out to configured drains
- Per-app log drain settings UI

### Post-Sprint
- Run `security-reviewer` on S3 credential handling
- Run `test-runner` agent

---

## Sprint 5: Deployment Enhancements + Bulk Ops

**Goal**: Power-user workflow features

### Parallel Streams (3 agents)

#### Agent 1: Deployment Enhancements (T6.2.1-5)
**Files**: `src/api/deployments.rs`, `src/engine/pipeline.rs`, `migrations/`, `frontend/app/components/deploy/`
- Deployment diff preview: compare current vs new env vars, domains, config
- Approval workflow: pending state, approve/reject API, notification to approvers
- Scheduled deployments: store deploy request with scheduled_at, cron evaluator triggers
- Deployment freeze: `deployment_freeze_windows` table, block deploys during windows
- Blue/green status indicator in UI

#### Agent 2: Bulk Operations (T6.3.1-6)
**Files**: `src/api/bulk.rs` (new), `frontend/app/components/bulk/`
- Bulk API: POST /api/bulk/action (action: start|stop|restart|deploy, app_ids: [...])
- App cloning: POST /api/apps/:id/clone (deep copy config, env vars, domains)
- Config snapshots: save app config as JSON, restore from snapshot
- Export/import projects: download/upload JSON with all apps, envs, domains
- Maintenance mode: toggle per app, proxy returns 503 with custom HTML
- Bulk action UI: multi-select apps, action dropdown

#### Agent 3: Shared Environment Variables (T8.7.1-5)
**Files**: `src/api/env_vars.rs`, `src/db/models.rs`, `migrations/`, `frontend/app/routes/`
- Team-level and project-level shared variable tables
- Resolution order: team → project → environment → app (later overrides earlier)
- Effective variables API: GET /api/apps/:id/env-vars/resolved
- Override indicators in env vars UI (badge showing "from team", "from project", etc.)
- Shared variable management pages in team and project settings

### Post-Sprint
- Run `test-runner` agent
- Run `code-reviewer` + `frontend-reviewer`

---

## Sprint 6: Enterprise (Complex, Partially Sequential)

**Goal**: Multi-server, SSO, Swarm - enterprise-grade features

### Parallel Streams (3 agents)

#### Agent 1: Multi-Server Support (T8.1.1-10)
**Files**: `src/api/servers.rs` (new), `src/runtime/remote.rs` (new), `src/db/models.rs`, `migrations/`, `frontend/app/routes/servers/`
- `servers` table with SSH connection details
- SSH key management (encrypted storage)
- Test SSH connection on registration
- Remote Docker command execution via SSH tunnel (using `openssh` crate or similar)
- Deploy to remote server: clone locally, build, push image, pull on remote
- Server health monitoring (periodic SSH + Docker ping)
- Server resource metrics collection via SSH
- Remote container log streaming
- Server management UI: list, add, status, terminal, file browser

#### Agent 2: SSO/SAML/OIDC (T8.2.1-8)
**Files**: `src/api/sso.rs` (new), `src/auth/oidc.rs` (new), `src/auth/saml.rs` (new), `migrations/`, `frontend/app/routes/settings/`
- OIDC: discovery document parsing, authorization flow, token validation
- SAML: assertion parsing, signature verification
- SSO provider config table (team_id, provider_type, metadata_url, client_id, etc.)
- User auto-provisioning from SSO claims (email, name, groups)
- SSO configuration UI in team settings
- Per-team SSO enforcement toggle

#### Agent 3: Container Replicas (T7.9.1-7) + Docker Swarm Prep (T8.5.1-3)
**Files**: `src/runtime/replicas.rs` (new), `src/proxy/`, `src/api/apps.rs`, `frontend/`
- Replica deployment: start N containers with indexed names
- Proxy load balancing config across replicas (round-robin)
- Scale API: PUT /api/apps/:id/scale
- Health monitoring per replica instance
- Graceful scale-down with connection draining
- Swarm init/join token management (prep for full Swarm)

### Post-Sprint
- Run `security-reviewer` on SSH key handling, SSO/SAML
- Run `test-runner` agent
- Run `code-reviewer` + `frontend-reviewer`

---

## Sprint 7: Remaining Enterprise + Polish

### Parallel Streams (3 agents)

#### Agent 1: Docker Swarm Full (T8.5.4-8)
- Deploy as Swarm services
- Service scaling across nodes
- Overlay networking
- Rolling update configuration
- Node health monitoring

#### Agent 2: Build Servers (T8.6.1-7)
- Build server registration
- Remote build execution
- Image push/pull workflow
- Build queue management

#### Agent 3: Remaining Tasks
- Deploy registry push (T5.2.3)
- Quick actions menu
- Deployment failure recovery (T2.2.2)
- Database integrity checks (T2.2.5)
- API documentation (T2.7.2-3)

---

## Dependency Map

```
Sprint 1 (no deps)
    ├── Preview Deployments
    ├── Watch Paths
    ├── Bitbucket Webhooks
    ├── Telegram + MS Teams notifications
    └── Instance Backup

Sprint 2 (no deps on Sprint 1)
    ├── OAuth Login
    ├── Project Environments
    ├── 2FA
    └── Pushover + Ntfy notifications

Sprint 3 (Scheduled Jobs scheduler reused in Sprint 4)
    ├── Templates Batch 1
    ├── Templates Batch 2
    ├── Scheduled Jobs ←── Sprint 4 reuses scheduler
    └── Deploy by Commit/Tag

Sprint 4 (S3 reuses scheduler from Sprint 3)
    ├── S3 Backups (depends on scheduler)
    ├── Advanced Monitoring
    └── Log Draining

Sprint 5 (no hard deps)
    ├── Deployment Enhancements (reuses scheduler)
    ├── Bulk Operations
    └── Shared Env Vars (depends on environments from Sprint 2)

Sprint 6 (multi-server needed before build servers)
    ├── Multi-Server Support ←── Sprint 7 build servers depend on this
    ├── SSO/SAML/OIDC
    └── Container Replicas + Swarm Prep

Sprint 7 (depends on Sprint 6)
    ├── Docker Swarm Full (depends on Swarm Prep)
    ├── Build Servers (depends on Multi-Server)
    └── Remaining cleanup tasks
```

## Estimated Scope

| Sprint | New Tasks | Agent Streams | Key Deliverable |
|--------|-----------|---------------|-----------------|
| 1 | ~33 | 5 parallel | Preview deploys + quick wins |
| 2 | ~29 | 4 parallel | OAuth + environments + 2FA |
| 3 | ~83 | 4 parallel | 55+ templates + jobs + deploy options |
| 4 | ~19 | 3 parallel | S3 backups + monitoring + log drain |
| 5 | ~16 | 3 parallel | Deployment workflow + bulk ops |
| 6 | ~28 | 3 parallel | Multi-server + SSO + replicas |
| 7 | ~20 | 3 parallel | Swarm + build servers + polish |
