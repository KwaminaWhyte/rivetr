# Code Organization & File Splitting Guide

> Strategy for splitting large files (1000+ lines) into clean, organized submodules.

## Principle

Any file exceeding **1000 lines** is split into a **subdirectory module**. The parent directory
(`mod.rs`) only contains: module declarations, re-exports, and the public API surface. Logic
lives in focused submodules by domain concern.

### Rust Pattern

```
src/api/apps.rs          →   src/api/apps/
                               ├── mod.rs       (route registration, re-exports)
                               ├── crud.rs      (list, get, create, update, delete)
                               ├── control.rs   (start, stop, restart)
                               ├── sharing.rs   (share/unshare app between teams)
                               └── upload.rs    (ZIP upload deploy flow)
```

In Rust, `mod apps;` in `src/api/mod.rs` transparently resolves to either
`src/api/apps.rs` **or** `src/api/apps/mod.rs` — no change to the caller required.

### Frontend Pattern

```
frontend/app/types/api.ts   →   frontend/app/types/
                                   ├── index.ts         (re-exports everything)
                                   ├── apps.ts
                                   ├── deployments.ts
                                   ├── databases.ts
                                   ├── teams.ts
                                   └── notifications.ts
```

---

## Backend Splits

### ✅ Done

| Original File | Split Into | Status |
|---|---|---|
| `src/db/seeders.rs` (2663 lines) | `src/db/seeders/` (10 files by category) | ✅ Complete |
| `src/api/webhooks.rs` (1670 lines) | `src/api/webhooks/` (mod + github/gitlab/gitea/bitbucket) | ✅ Complete |
| `src/api/apps.rs` (1990 lines) | `src/api/apps/` (mod + crud/control/sharing/upload/logs) | ✅ Complete |
| `src/api/teams.rs` (1682 lines) | `src/api/teams/` (mod + crud/members/invitations/audit) | ✅ Complete |
| `src/runtime/docker.rs` (1016 lines) | `src/runtime/docker/` (mod + build/container/logs) | ✅ Complete |
| `src/api/git_providers.rs` (1263 lines) | `src/api/git_providers/` (mod + github/gitlab/bitbucket) | ✅ Complete |
| `src/engine/pipeline.rs` (1906 lines) | `src/engine/pipeline/` (mod + clone/build/start/rollback) | ✅ Complete |
| `src/engine/container_monitor.rs` (1183 lines) | `src/engine/container_monitor/` (mod + health/stats/recovery) | ✅ Complete |
| `src/api/services.rs` (1038 lines) | `src/api/services/` (mod + crud/compose/control) | ✅ Complete |
| `src/api/system.rs` (1028 lines) | `src/api/system/` (mod + health/backup/updates) | ✅ Complete |
| `src/notifications/alert_notifications.rs` (1225 lines) | `src/notifications/alert_notifications/` (mod + email/slack/discord/channels) | ✅ Complete |
| `src/api/deployments.rs` (1557 lines) | `src/api/deployments/` (mod + handlers/rollback/approval/freeze/shared) | ✅ Complete |
| `src/api/validation.rs` (1073 lines) | `src/api/validation/` (mod + apps/databases/services) | ✅ Complete |
| `src/cli/mod.rs` (1275 lines) | `src/cli/` (mod + deploy/backup/database/server) | ✅ Complete |

### Queued (after Sprint 4 agents finish)

| File | Lines | Split Plan | Blocks |
|---|---|---|---|
| `src/api/apps.rs` | 1990 | `crud.rs`, `control.rs`, `sharing.rs`, `upload.rs` | None |
| `src/api/teams.rs` | 1682 | `crud.rs`, `members.rs`, `invitations.rs`, `audit.rs` | None |
| `src/engine/build_detect.rs` | 1004 | `detectors.rs`, `languages.rs`, `config.rs` | None |

---

## Frontend Splits

### ✅ Done

| Original File | Split Into | Status |
|---|---|---|
| `frontend/app/types/api.ts` (1812 lines) | `frontend/app/types/` (api.ts barrel + apps/deployments/databases/services/teams/notifications/system) | ✅ Complete |

### Queued (after Sprint 4 agents finish)

| File | Lines | Split Plan |
|---|---|---|
| `frontend/app/routes/projects/$id.tsx` | 1952 | Extract tab panels as separate components in `$id/` folder |
| `frontend/app/routes/settings/teams/$id.tsx` | 1311 | Extract `MembersTab`, `InvitationsTab`, `AuditTab` components |
| `frontend/app/routes/settings/notifications.tsx` | 1058 | Extract per-channel config cards as separate components |
| `frontend/app/components/team-notification-channels-card.tsx` | 1058 | Split per-provider cards into `notifications/` subfolder |
| `frontend/app/routes/settings/git-providers.tsx` | 1000 | Extract `GithubTab`, `GitlabTab`, `GiteaTab`, `BitbucketTab` |

---

## Detailed Split Plans

### `src/db/seeders/` ✅

```
mod.rs          — seed_service_templates() entry point, orchestrates sub-seeder calls
infrastructure.rs — Portainer, Gitea, n8n, Grafana, Prometheus, Uptime Kuma, etc. (original 26)
ai_ml.rs        — Ollama, Open WebUI, LiteLLM, Langflow, Flowise, ChromaDB
analytics.rs    — Plausible, Umami, PostHog, Matomo
automation.rs   — Activepieces, Windmill, Trigger.dev
cms.rs          — WordPress, Ghost, Strapi, Directus, Payload CMS
communication.rs— Rocket.Chat, Mattermost, Matrix/Synapse
devtools.rs     — Code Server, Supabase, Appwrite, Pocketbase, Hoppscotch, Forgejo
documentation.rs— BookStack, Wiki.js, Docmost, Outline
media.rs        — Immich, Jellyfin, Navidrome, Seafile
monitoring.rs   — SigNoz, Beszel, Checkmate
security.rs     — Authentik, Keycloak, Vaultwarden, Infisical
search.rs       — Meilisearch, Typesense
project_mgmt.rs — Plane, Vikunja, Leantime, Cal.com
other.rs        — Paperless-ngx, Trilium, Linkwarden, Tandoor, Stirling-PDF
```

### `src/api/webhooks/` ✅

```
mod.rs          — route registration, shared types, signature verification utils
github.rs       — github_webhook handler, handle_github_push, handle_github_pull_request
gitlab.rs       — gitlab_webhook handler, handle_gitlab_push, handle_gitlab_merge_request
gitea.rs        — gitea_webhook handler, handle_gitea_push, handle_gitea_pull_request
bitbucket.rs    — bitbucket_webhook handler, handle_bitbucket_push, handle_bitbucket_pr
```

### `src/api/apps/`

```
mod.rs          — route registration, re-exports
crud.rs         — list_apps, get_app, create_app, update_app, delete_app, validation
control.rs      — start_app, stop_app, restart_app, get_app_status
sharing.rs      — list_app_shares, create_app_share, delete_app_share, list_apps_with_sharing
upload.rs       — upload_create_app (ZIP upload flow)
logs.rs         — stream_app_logs (SSE log streaming)
```

### `src/engine/pipeline/`

```
mod.rs          — run_deployment, run_rollback, public types (DeploymentStage, etc.)
clone.rs        — clone_repository, clone_with_ssh_key, git_checkout helpers
build.rs        — execute_deployment_commands (Dockerfile/Nixpacks/Railpack/etc.)
start.rs        — container start, health check, proxy route switch
rollback.rs     — run_rollback, trigger_auto_rollback, AutoRollbackTriggered
```

### `src/api/teams/`

```
mod.rs          — route registration, re-exports, shared helpers (validate_team_role, etc.)
crud.rs         — list_teams, get_team, create_team, update_team, delete_team
members.rs      — list_members, invite_member, update_member_role, remove_member
invitations.rs  — list_invitations, create_invitation, delete_invitation, resend_invitation,
                   validate_invitation, accept_invitation
audit.rs        — list_audit_logs, log_team_audit
```

### `src/runtime/docker/`

```
mod.rs          — DockerRuntime struct, ContainerRuntime trait impl (delegates to submodules)
build.rs        — build() implementation
container.rs    — run(), stop(), remove(), exec(), inspect()
logs.rs         — logs() streaming implementation
```

### `frontend/app/types/`

```
index.ts        — re-exports: export * from './apps'; export * from './deployments'; etc.
apps.ts         — App, CreateAppRequest, UpdateAppRequest, AppStatus, AppShare
deployments.ts  — Deployment, DeploymentLog, TriggerDeployRequest, RollbackRequest
databases.ts    — Database, CreateDatabaseRequest, DatabaseBackup
services.ts     — Service, ServiceTemplate, CreateServiceRequest
teams.ts        — Team, TeamMember, TeamInvitation, AuditLog, TeamRole
notifications.ts— NotificationChannel, CreateChannelRequest, NotificationEvent
system.ts       — SystemHealth, BackupFile, S3Config, OAuthProvider
costs.ts        — CostSummary, CostSnapshot, ResourceAlert
```

---

## Rules for New Files

1. **Backend**: Any new handler file > 500 lines should start as a subdirectory
2. **Frontend**: Any route file > 600 lines should extract tab/section panels into components
3. **No circular imports**: submodules import from `crate::`, never from `super::super::`
4. **Re-export everything public**: `mod.rs` must re-export all public types/functions used externally

---

## How to Execute a Split (Rust)

```bash
# 1. Create the subdirectory
mkdir -p src/api/teams

# 2. Create mod.rs with public API + route registration
# 3. Create submodule files with extracted code
# 4. Rename original: mv src/api/teams.rs src/api/teams_old.rs (for reference)
# 5. Verify: cargo check
# 6. Delete old file
# 7. cargo test
```
