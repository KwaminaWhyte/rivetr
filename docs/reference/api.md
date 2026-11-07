# REST API Reference

Rivetr exposes a REST API (Axum) defined in `src/api/mod.rs` (`create_router`). This is a complete endpoint catalog grouped by resource. Request/response schemas are intentionally omitted, see the handler modules in `src/api/` for payload details.

## Authentication

Most endpoints under `/api` require a **Bearer token** in the `Authorization` header:

```
Authorization: Bearer <admin_token>
```

The token is `auth.admin_token` from `rivetr.toml`, or a token created via `POST /api/tokens`. Auth-flow, webhook, SSO, and a few public endpoints do **not** require a token (called out below). WebSocket endpoints authenticate via a query parameter rather than the header.

The admin token from the config has full access but, like API tokens, is subject to the API access policy (Settings > Advanced: on/off and IP allowlist). Dashboard sessions are not.

### API token scopes

Tokens created with `POST /api/tokens` (prefix `rvt_`) carry one or more scopes. Every route is classified once, by method and path, in the table in `src/api/scopes.rs` (a unit test fails when a route under `/api` has no entry; at runtime an unmapped route is refused to tokens).

| Scope | Allows |
|-------|--------|
| `read` | GET requests that do not reveal secrets. |
| `deploy` | Deploy, start, stop, restart, roll back, cancel, approve or reject a deployment, run a job, bulk and tag deploys. |
| `write` | Create and change resources (POST/PUT/PATCH), container terminals. |
| `delete` | DELETE requests. |
| `secrets` | Reveal secret values: `?reveal=true`, resolved and linked env vars, generated service vars, container file reads, database backup downloads, project and DB exports. |
| `admin` | Instance and team administration (`/settings`, `/system`, `/sso`, `/audit`, `/swarm`, `/tokens`, team members, invitations and roles, 2FA and passkeys), host access (server files and terminal, reboot, patching, Docker install and cleanup). Implies every other scope. |

Scopes do not imply each other (except `admin`): a CI token usually needs `read,deploy`. A token never grants more than its owner's account and team role allow; the RBAC check runs after the scope check. Insufficient scope returns `403` with `{"error":{"code":"insufficient_scope", ...}}`.

Restrictions (optional, combined with scopes):
- **Team** (`team_id`): routes that address a resource by id must target that team's resources; `/api/teams/:id/...` must be that team; collection reads get `team_id` forced into the query string; other non-resource writes need a JSON body with that `team_id`.
- **Resources** (`resources`, list of `{"type": "project|app|database|service", "id"}`): only routes that address a listed resource (or, for a project, its apps, databases, services and environments; for an app, its deployments, volumes and previews) by id. Collection routes are refused.
- Restricted tokens cannot call `admin` routes and cannot use the MCP server.

Lifecycle: optional expiry (`expires_at`), pause/resume (a paused token gets `401`), roll (new secret shown once; the previous secret keeps working for `grace_minutes`, default 60, max 10080, 0 revokes it at once). Last use time and client IP are recorded. Tokens created before scopes existed have every scope; `POST /api/tokens` without `scopes` also creates a full-scope token, for compatibility.

MCP (`POST /mcp`): read-only tools need `read`, `deploy_app` and `restart_app` need `deploy`. `tools/list` only lists the tools the token may call.

## Rate-limiting tiers

- **Auth tier** (strict, default 20/min): login/register/2FA/OAuth flows.
- **API tier** (default 100/min): all protected `/api` endpoints and auth-info reads.
- **Webhook tier** (default 500/min): `/webhooks/*`.

---

## Unauthenticated / top-level

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/health` | Liveness check (returns `OK`). |
| GET | `/metrics` | Prometheus metrics. |
| POST | `/mcp` | MCP (Model Context Protocol) server handler. |
| GET | `/api/white-label` | White-label config (needed by login page pre-auth). |
| GET | `/api/sdk` | Download the generated TypeScript SDK. |

## Auth flow (auth tier, public)

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/auth/login` | Log in. |
| POST | `/api/auth/logout` | Log out. |
| POST | `/api/auth/setup` | First-run admin setup. |
| POST | `/api/auth/register-with-invitation` | Register via a team invitation. |
| GET | `/api/auth/oauth/:provider/authorize` | Start Git provider OAuth connection. |
| GET | `/api/auth/oauth/:provider/callback` | Git provider OAuth callback. |
| GET | `/api/auth/github-apps/callback` | GitHub App manifest creation callback. |
| GET | `/api/auth/github-apps/installation/callback` | GitHub App installation callback. |
| GET | `/api/auth/oauth-login/:provider/authorize` | Start social-login OAuth. |
| GET | `/api/auth/oauth-login/:provider/callback` | Social-login OAuth callback. |
| POST | `/api/auth/2fa/validate` | Validate 2FA during login (temp session token). |

## Auth info (API tier, public)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/auth/validate` | Validate current session/token. |
| GET | `/api/auth/me` | Current user info. |
| GET | `/api/auth/setup-status` | Whether first-run setup is complete. |
| GET | `/api/auth/oauth/providers` | List enabled OAuth login providers. |
| GET | `/api/auth/invitations/:token` | Validate a team invitation token. |

## SSO / OIDC (public, browser redirects)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/auth/sso/:provider_id/login` | Initiate SSO login. |
| GET | `/auth/sso/:provider_id/callback` | Handle SSO callback. |

## Webhooks (webhook tier, public)

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/webhooks/github` | GitHub push/PR webhook. |
| POST | `/webhooks/gitlab` | GitLab webhook. |
| POST | `/webhooks/gitea` | Gitea webhook. |
| POST | `/webhooks/bitbucket` | Bitbucket webhook. |
| POST | `/webhooks/dockerhub` | Docker Hub image-push webhook. |

---

# Protected API (Bearer token required)

## Apps

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/apps` | List apps. |
| POST | `/api/apps` | Create an app. |
| GET | `/api/apps/:id` | Get an app. |
| PUT | `/api/apps/:id` | Update an app. |
| DELETE | `/api/apps/:id` | Delete an app. |
| GET | `/api/apps/:id/status` | App container status. |
| POST | `/api/apps/:id/start` | Start the app. |
| POST | `/api/apps/:id/stop` | Stop the app. |
| POST | `/api/apps/:id/restart` | Restart the app. |
| POST | `/api/apps/:id/apply-limits` | Apply resource limits. |
| POST | `/api/apps/:id/generate-domain` | Generate an auto domain. |
| GET | `/api/apps/:id/activity` | App activity feed. |
| GET | `/api/apps/:id/logs/stream` | Stream app logs (SSE). |
| GET | `/api/apps/:id/github-actions-workflow` | Suggested GitHub Actions workflow. |
| POST | `/api/projects/:id/apps/upload` | Create app via uploaded archive (project-scoped). |

### App sharing

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/apps/with-sharing` | List apps including shares. |
| GET | `/api/apps/:id/shares` | List shares for an app. |
| POST | `/api/apps/:id/shares` | Share app with a team. |
| DELETE | `/api/apps/:id/shares/:team_id` | Remove a share. |

### App database links

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/apps/:app_id/links` | List DB→app env-injection links. |
| POST | `/api/apps/:app_id/links` | Create a link. |
| DELETE | `/api/apps/:app_id/links/:link_id` | Delete a link. |
| GET | `/api/apps/:app_id/linked-env-vars` | Preview env vars from links. |

### App clone / snapshots / maintenance / replicas / autoscaling

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/apps/:id/clone` | Clone an app. |
| POST | `/api/apps/:id/snapshots` | Create a snapshot. |
| GET | `/api/apps/:id/snapshots` | List snapshots. |
| POST | `/api/apps/:id/snapshots/:sid/restore` | Restore a snapshot. |
| DELETE | `/api/apps/:id/snapshots/:sid` | Delete a snapshot. |
| PUT | `/api/apps/:id/maintenance` | Toggle maintenance mode. |
| GET | `/api/apps/:id/replicas` | List replicas. |
| PUT | `/api/apps/:id/replicas/count` | Set replica count. |
| POST | `/api/apps/:id/replicas/:index/restart` | Restart a replica. |
| GET | `/api/apps/:id/autoscaling` | List autoscaling rules. |
| POST | `/api/apps/:id/autoscaling` | Create an autoscaling rule. |
| PUT | `/api/apps/:id/autoscaling/:rule_id` | Update a rule. |
| DELETE | `/api/apps/:id/autoscaling/:rule_id` | Delete a rule. |

## Deployments

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/apps/:id/deploy` | Trigger a deploy. |
| POST | `/api/apps/:id/deploy/upload` | Deploy from an uploaded archive. |
| GET | `/api/apps/:id/deployments` | List deployments for an app. |
| GET | `/api/apps/:id/stats` | App resource stats, including `storage_bytes` (writable layer plus volumes, refreshed once a minute; omitted when the runtime cannot measure it). |
| GET | `/api/apps/:id/commits` | List repo commits. |
| GET | `/api/apps/:id/tags` | List repo tags. |
| GET | `/api/deployments/:id` | Get a deployment. |
| GET | `/api/deployments/:id/logs` | Deployment logs. |
| GET | `/api/deployments/:id/diff` | Deployment diff. |
| POST | `/api/deployments/:id/rollback` | Roll back to a deployment. |
| POST | `/api/deployments/:id/approve` | Approve a pending deployment. |
| POST | `/api/deployments/:id/reject` | Reject a pending deployment. |
| GET | `/api/apps/:id/deployments/pending` | List pending deployments. |
| POST | `/api/apps/:app_id/deployments/:id/cancel` | Cancel a running deployment. |
| GET | `/api/apps/:id/freeze-windows` | List deploy freeze windows. |
| POST | `/api/apps/:id/freeze-windows` | Create a freeze window. |
| DELETE | `/api/apps/:id/freeze-windows/:window_id` | Delete a freeze window. |
| POST | `/api/build/detect` | Detect build type from an upload. |

## Environment variables

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/apps/:id/env-vars` | List app env vars. |
| POST | `/api/apps/:id/env-vars` | Create an env var. |
| GET | `/api/apps/:id/env-vars/resolved` | Resolved env vars (incl. shared/linked). |
| GET | `/api/apps/:id/env-vars/:key` | Get an env var. |
| PUT | `/api/apps/:id/env-vars/:key` | Update an env var. |
| DELETE | `/api/apps/:id/env-vars/:key` | Delete an env var. |

## Patches, alerts, costs, basic-auth, redirects, volumes (app-scoped)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/apps/:id/patches` | List file-injection patches. |
| POST | `/api/apps/:id/patches` | Create a patch. |
| PUT | `/api/apps/:id/patches/:patch_id` | Update a patch. |
| DELETE | `/api/apps/:id/patches/:patch_id` | Delete a patch. |
| GET | `/api/apps/:id/alerts` | List alert configs. |
| POST | `/api/apps/:id/alerts` | Create an alert. |
| GET | `/api/apps/:id/alerts/:alert_id` | Get an alert. |
| PUT | `/api/apps/:id/alerts/:alert_id` | Update an alert. |
| DELETE | `/api/apps/:id/alerts/:alert_id` | Delete an alert. |
| GET | `/api/apps/:id/alert-events` | List alert events. |
| GET | `/api/apps/:id/costs` | App cost breakdown. |
| GET | `/api/apps/:id/basic-auth` | Get HTTP basic-auth config. |
| PUT | `/api/apps/:id/basic-auth` | Set basic-auth config. |
| DELETE | `/api/apps/:id/basic-auth` | Remove basic-auth config. |
| GET | `/api/apps/:id/redirects` | List redirect rules. |
| POST | `/api/apps/:id/redirects` | Create a redirect rule. |
| PUT | `/api/apps/:id/redirects/:rid` | Update a redirect rule. |
| DELETE | `/api/apps/:id/redirects/:rid` | Delete a redirect rule. |
| GET | `/api/apps/:id/volumes` | List app volumes. |
| POST | `/api/apps/:id/volumes` | Create a volume. |
| GET | `/api/volumes/:id` | Get a volume. |
| PUT | `/api/volumes/:id` | Update a volume. |
| DELETE | `/api/volumes/:id` | Delete a volume. |
| POST | `/api/volumes/:id/backup` | Download a volume as a tar.gz archive. |
| GET | `/api/volumes/:id/backups` | Volume backup history, local and S3 (`?scan=true` lists buckets). |
| POST | `/api/volumes/:id/backups` | Start a volume backup (local or S3). |
| DELETE | `/api/volumes/:id/backups/:backup_id` | Delete a volume backup (`?source=local\|s3`). |
| POST | `/api/volumes/:id/backups/:backup_id/restore` | Restore a volume backup (local or S3). |
| GET | `/api/volumes/:id/backups/restores` | Recent volume restore jobs. |
| GET/PUT/DELETE | `/api/volumes/:id/backup-schedule` | Per-volume backup schedule. |

## Logs, monitoring, scheduled jobs (app-scoped)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/apps/:id/log-drains` | List log drains. |
| POST | `/api/apps/:id/log-drains` | Create a log drain. |
| PUT | `/api/apps/:id/log-drains/:drain_id` | Update a log drain. |
| DELETE | `/api/apps/:id/log-drains/:drain_id` | Delete a log drain. |
| POST | `/api/apps/:id/log-drains/:drain_id/test` | Test a log drain. |
| GET | `/api/apps/:id/logs/search` | Search app logs. |
| GET | `/api/apps/:id/log-retention` | Get log retention policy. |
| PUT | `/api/apps/:id/log-retention` | Update log retention policy. |
| GET | `/api/apps/:id/uptime` | Current uptime status. |
| GET | `/api/apps/:id/uptime/history` | Uptime history. |
| POST | `/api/apps/:id/scheduled-restarts` | Create a scheduled restart. |
| GET | `/api/apps/:id/scheduled-restarts/runs` | Recent runs of this app's scheduled restarts (status, times, error, restart deployment). |
| GET | `/api/apps/:id/scheduled-restarts` | List scheduled restarts. |
| PUT | `/api/apps/:id/scheduled-restarts/:restart_id` | Update a scheduled restart. |
| DELETE | `/api/apps/:id/scheduled-restarts/:restart_id` | Delete a scheduled restart. |
| GET | `/api/apps/:id/jobs` | List scheduled jobs. |
| POST | `/api/apps/:id/jobs` | Create a job. |
| GET | `/api/apps/:id/jobs/:job_id` | Get a job. |
| PUT | `/api/apps/:id/jobs/:job_id` | Update a job. |
| DELETE | `/api/apps/:id/jobs/:job_id` | Delete a job. |
| POST | `/api/apps/:id/jobs/:job_id/run` | Trigger a job run. |
| GET | `/api/apps/:id/jobs/:job_id/runs` | List job runs. |

## Databases (managed)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/databases` | List databases. |
| POST | `/api/databases` | Create a database. |
| GET | `/api/databases/:id` | Get a database. |
| PUT | `/api/databases/:id` | Update a database. |
| DELETE | `/api/databases/:id` | Delete a database. |
| POST | `/api/databases/:id/start` | Start a database. |
| POST | `/api/databases/:id/stop` | Stop a database. |
| GET | `/api/databases/:id/logs` | Database logs. |
| GET | `/api/databases/:id/start-events` | Start-event snapshot. |
| GET | `/api/databases/:id/stats` | Database resource stats. |
| POST | `/api/databases/:id/import` | Import a dump. |
| GET | `/api/databases/:id/extensions` | List extensions (PostgreSQL). |
| POST | `/api/databases/:id/extensions` | Install an extension. |
| GET | `/api/databases/:id/backups` | List backups. |
| POST | `/api/databases/:id/backups` | Create a backup. |
| GET | `/api/databases/:id/backups/:backup_id` | Get a backup. |
| DELETE | `/api/databases/:id/backups/:backup_id` | Delete a backup. |
| GET | `/api/databases/:id/backups/:backup_id/download` | Download a backup. |
| GET | `/api/databases/:id/backups/schedule` | Get backup schedule. |
| POST | `/api/databases/:id/backups/schedule` | Upsert backup schedule. |
| DELETE | `/api/databases/:id/backups/schedule` | Delete backup schedule. |
| GET | `/api/databases/:id/backups/catalog` | Local and S3 backups (`?scan=true` lists buckets). |
| POST | `/api/databases/:id/backups/:backup_id/restore` | Restore a backup (local or S3). |
| POST | `/api/databases/:id/backups/:backup_id/upload-s3` | Copy a local backup to S3. |
| GET | `/api/databases/:id/backups/restores` | Recent database restore jobs. |

## Services (Docker Compose)

A service's `domain` and `port` route that hostname to `127.0.0.1:<port>` on the host, so the compose file must publish that port on loopback (for example `"127.0.0.1:32990:3000"`). The domain is added to the instance TLS certificate automatically, the same way app domains are.

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/services` | List services. |
| POST | `/api/services` | Create a service. |
| GET | `/api/services/check-port` | Check port availability. |
| GET | `/api/services/:id` | Get a service. |
| PUT | `/api/services/:id` | Update a service. |
| DELETE | `/api/services/:id` | Delete a service. |
| POST | `/api/services/:id/start` | Start a service. |
| POST | `/api/services/:id/stop` | Stop a service. |
| POST | `/api/services/:id/restart` | Restart a service. |
| POST | `/api/services/:id/deploy` | Deploy: pull or build images, run hooks, `docker compose up -d`. See [service-deployments.md](service-deployments.md). |
| GET | `/api/services/:id/deployments` | Service deployment history. |
| GET | `/api/services/:id/deployments/:deployment_id` | One service deployment. |
| GET | `/api/services/:id/deployments/:deployment_id/logs` | Service deployment logs. |
| POST | `/api/services/:id/deployments/:deployment_id/rollback` | Roll back to a deployment's image digests. |
| GET/PUT | `/api/services/:id/deploy-config` | Build source, images, registry, hooks. |
| GET | `/api/services/:id/containers` | Containers with state, health and usage. |
| POST | `/api/services/:id/containers/:container/restart` | Restart one container. |
| GET | `/api/services/:id/containers/:container/logs` | One container's logs. |
| GET/POST | `/api/services/:id/env-vars` | Service env vars. |
| PUT/DELETE | `/api/services/:id/env-vars/:key` | Update or delete a service env var. |
| POST | `/api/services/:id/env-import` | Import a `.env` file. |
| GET | `/api/services/:id/start-events` | Start-event snapshot. |
| GET | `/api/services/:id/stats` | Service stats summed across its containers, including `storage_bytes` (shared volumes counted once). |
| GET | `/api/services/:id/logs` | Service logs. |
| GET | `/api/services/:id/logs/stream` | Stream service logs. |
| GET | `/api/services/:id/preview-compose` | Preview generated compose file. |
| GET | `/api/services/:id/generated-vars` | Auto-generated env vars. |
| POST | `/api/services/:id/import-db` | Import DB into a service. |
| GET | `/api/services/:id/export-db` | Export service DB. |

### Service templates

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/templates` | List service templates. |
| GET | `/api/templates/categories` | List template categories. |
| POST | `/api/templates/suggest` | Suggest a template. |
| GET | `/api/templates/suggestions` | List template suggestions. |
| PUT | `/api/templates/suggestions/:id/approve` | Approve a suggestion. |
| GET | `/api/templates/:id` | Get a template. |
| POST | `/api/templates/:id/deploy` | Deploy a template. |

## Projects & environments

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/projects` | List projects. |
| POST | `/api/projects` | Create a project. |
| GET | `/api/projects/:id` | Get a project. |
| PUT | `/api/projects/:id` | Update a project. |
| DELETE | `/api/projects/:id` | Delete a project. |
| GET | `/api/projects/:id/costs` | Project costs. |
| GET | `/api/projects/:id/dependency-graph` | App dependency graph. |
| POST | `/api/apps/:id/dependencies` | Add an app dependency. |
| DELETE | `/api/apps/:id/dependencies/:dep_id` | Remove a dependency. |
| PUT | `/api/apps/:id/project` | Assign app to a project. |
| GET | `/api/projects/:id/env-vars` | List project shared env vars. |
| POST | `/api/projects/:id/env-vars` | Create a project env var. |
| PUT | `/api/projects/:id/env-vars/:var_id` | Update a project env var. |
| DELETE | `/api/projects/:id/env-vars/:var_id` | Delete a project env var. |
| GET | `/api/projects/:id/environments` | List environments. |
| POST | `/api/projects/:id/environments` | Create an environment. |
| PUT | `/api/environments/:id` | Update an environment. |
| DELETE | `/api/environments/:id` | Delete an environment. |
| POST | `/api/projects/:project_id/environments/:env_id/clone` | Clone an environment. |
| GET | `/api/environments/:id/env-vars` | List environment env vars. |
| POST | `/api/environments/:id/env-vars` | Create an environment env var. |
| PUT | `/api/environments/:env_id/env-vars/:id` | Update an environment env var. |
| DELETE | `/api/environments/:env_id/env-vars/:id` | Delete an environment env var. |
| GET | `/api/projects/:id/export` | Export a project. |
| POST | `/api/projects/:id/import` | Import a project. |

## Teams

### Team roles and abilities

Team access is decided from fine-grained abilities. A member's `role` is a preset key or `custom:<role id>`; `GET /api/teams/:id` returns the caller's resolved `user_abilities`. Checks are centralized in `src/api/authz.rs` (`require`, `require_team_ability`, `authorize_action`); the resource middleware maps each request to an action with the same route table as API token scopes (`src/api/scopes.rs`) and then to an ability (e.g. `POST /api/apps/:id/deploy` needs `apps.deploy`).

Abilities: `apps.{read,deploy,write,delete}`, `services.{read,deploy,write,delete}`, `databases.{read,deploy,write,delete}`, `projects.{read,write,delete}`, `env.read_secrets`, `servers.{read,manage}`, `integrations.manage`, `team.manage`, `team.audit`, `team.delete`, `settings.manage`, `tokens.manage`.

| Preset | Abilities |
|--------|-----------|
| `owner` | All. |
| `admin` | All except `team.delete` (delete the team, 2FA requirement). |
| `developer` | All resource abilities including delete, `env.read_secrets`, `servers.manage`, `integrations.manage`, `tokens.manage`; no team administration. |
| `deployer` | Read everything, `*.deploy`, `tokens.manage`. |
| `viewer` | Read only. |

**Team 2FA requirement** (`PUT /api/teams/:id/2fa-enforcement`, owners only): while it is on, a member without TOTP and without a passkey gets `403` with `{"error":{"code":"two_factor_required", ...}}` on every resource of that team (apps, databases, services, projects, servers and the rest) and on team management routes. Reading the team itself (`GET /api/teams/:id`, members list) still works, and `GET /api/teams` and `GET /api/teams/:id` return `two_factor_blocked: true` so the dashboard shows a banner linking to `/account/security`. API tokens act as their owner, so a token of a blocked member is refused for that team's resources too. Instance admins and the admin token from the config are not team-scoped and are not affected.

The pre-1.1 values `owner`, `admin`, `developer` and `viewer` are these presets with the same access. Rules: only owners manage owners or hand out the owner role; nobody can create, edit or assign a role with abilities their own role lacks. Per-member resource overrides (`/members/:user_id/permissions`) apply on top of the role: `deny` removes all access to that resource (or, on a project, its apps, databases, services and environments; on an app, its deployments, volumes and previews); `allow` grants read, deploy and edit on it even when the role does not (never delete or secrets). Overrides on the resource, its app and its project are combined and a deny anywhere wins over an allow. Members with `team.manage` set overrides, but not on themselves and not on members they could not manage. Instance admins and the admin token bypass team checks.

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/teams` | List teams. |
| POST | `/api/teams` | Create a team. |
| GET | `/api/teams/:id` | Get a team. |
| PUT | `/api/teams/:id` | Update a team. |
| DELETE | `/api/teams/:id` | Delete a team. |
| PUT | `/api/teams/:id/2fa-enforcement` | Toggle 2FA enforcement. |
| GET | `/api/teams/:id/members` | List members. |
| POST | `/api/teams/:id/members` | Invite a member. |
| PUT | `/api/teams/:id/members/:user_id` | Update a member's role. |
| DELETE | `/api/teams/:id/members/:user_id` | Remove a member. |
| GET | `/api/teams/:id/members/:user_id/permissions` | List member resource permissions. |
| PUT | `/api/teams/:id/members/:user_id/permissions` | Set member permissions. |
| DELETE | `/api/teams/:id/members/:user_id/permissions/:perm_id` | Delete a member permission. |
| GET | `/api/teams/:id/roles` | Role presets, the team's custom roles and the ability catalogue. |
| POST | `/api/teams/:id/roles` | Create a custom role: `{"name", "description", "abilities": ["apps.read", ...]}`. |
| PUT | `/api/teams/:id/roles/:role_id` | Update a custom role. |
| DELETE | `/api/teams/:id/roles/:role_id` | Delete a custom role (409 while members or pending invitations hold it). |
| GET | `/api/teams/:id/invitations` | List invitations. |
| POST | `/api/teams/:id/invitations` | Create an invitation. |
| DELETE | `/api/teams/:id/invitations/:inv_id` | Delete an invitation. |
| POST | `/api/teams/:id/invitations/:inv_id/resend` | Resend an invitation. |
| POST | `/api/invitations/:token/accept` | Accept an invitation. |
| GET | `/api/teams/:id/env-vars` | List team shared env vars. |
| POST | `/api/teams/:id/env-vars` | Create a team env var. |
| PUT | `/api/teams/:id/env-vars/:var_id` | Update a team env var. |
| DELETE | `/api/teams/:id/env-vars/:var_id` | Delete a team env var. |
| GET | `/api/teams/:id/audit-logs` | Team audit logs. |
| GET | `/api/teams/:id/costs` | Team costs. |
| GET | `/api/teams/:id/notification-channels` | List team notification channels. |
| POST | `/api/teams/:id/notification-channels` | Create a team channel. |
| GET | `/api/teams/:id/notification-channels/:channel_id` | Get a team channel. |
| PUT | `/api/teams/:id/notification-channels/:channel_id` | Update a team channel. |
| DELETE | `/api/teams/:id/notification-channels/:channel_id` | Delete a team channel. |
| POST | `/api/teams/:id/notification-channels/:channel_id/test` | Test a team channel. |

## Notification channels (global)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/notification-channels` | List channels. |
| POST | `/api/notification-channels` | Create a channel. |
| GET | `/api/notification-channels/:id` | Get a channel. |
| PUT | `/api/notification-channels/:id` | Update a channel. |
| DELETE | `/api/notification-channels/:id` | Delete a channel. |
| POST | `/api/notification-channels/:id/test` | Test a channel. |
| GET | `/api/notification-channels/:id/subscriptions` | List subscriptions. |
| POST | `/api/notification-channels/:id/subscriptions` | Create a subscription. |
| DELETE | `/api/notification-subscriptions/:id` | Delete a subscription. |

## Routes (proxy management)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/routes` | List proxy routes. |
| POST | `/api/routes` | Add a route. |
| GET | `/api/routes/domains` | List routed domains. |
| GET | `/api/routes/health` | Routes health overview. |
| GET | `/api/routes/:domain` | Get a route. |
| DELETE | `/api/routes/:domain` | Remove a route. |
| PUT | `/api/routes/:domain/health` | Update route health. |

## TLS certificates and DNS providers

`:resource` is `apps` or `services`; the domain must belong to that resource.

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/:resource/:id/certificates` | Certificate status per domain (mode, status, issuer, expiry, last error). |
| PUT | `/api/:resource/:id/certificates/:domain` | Set mode: `{"mode":"auto"}` or `{"mode":"dns","dns_provider_id":"..."}`. |
| POST | `/api/:resource/:id/certificates/:domain/custom` | Upload `{"certificate":"PEM chain","private_key":"PEM key"}`. |
| DELETE | `/api/:resource/:id/certificates/:domain/custom` | Remove the upload; automatic issuance resumes. |
| GET | `/api/dns-providers` | List DNS providers (no secrets). |
| POST | `/api/dns-providers` | Add a provider (admin): `{"name","provider":"cloudflare\|route53\|hetzner\|digitalocean","credentials":{...}}`. |
| PUT | `/api/dns-providers/:id` | Rename or replace credentials (admin). |
| DELETE | `/api/dns-providers/:id` | Delete an unused provider (admin). |
| POST | `/api/dns-providers/:id/verify` | Test the stored credentials (admin). |
| GET | `/api/certificates/wildcard` | Wildcard instance domain setting and status (admin). |
| PUT | `/api/certificates/wildcard` | Set `{"domain":"*.apps.example.com","dns_provider_id":"..."}`; empty domain turns it off (admin). |

## SSH keys

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/ssh-keys` | List SSH keys. |
| POST | `/api/ssh-keys` | Create an SSH key. |
| GET | `/api/ssh-keys/:id` | Get an SSH key. |
| PUT | `/api/ssh-keys/:id` | Update an SSH key. |
| DELETE | `/api/ssh-keys/:id` | Delete an SSH key. |
| GET | `/api/apps/:id/ssh-keys` | App-associated SSH keys. |

## Remote servers & filesystem

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/servers` | List servers. |
| POST | `/api/servers` | Create a server. |
| GET | `/api/servers/:id` | Get a server. |
| PUT | `/api/servers/:id` | Update a server. |
| DELETE | `/api/servers/:id` | Delete a server. |
| POST | `/api/servers/:id/check` | Check server health. |
| POST | `/api/servers/:id/install-docker` | Install Docker on a server. |
| POST | `/api/servers/:id/fetch-details` | Fetch server details. |
| GET | `/api/servers/:id/patches` | Check available OS patches. |
| GET | `/api/servers/:id/security-check` | Run a security check. |
| GET | `/api/servers/:id/files` | Browse files. |
| DELETE | `/api/servers/:id/files` | Delete a file. |
| GET | `/api/servers/:id/files/content` | Read file content. |
| PUT | `/api/servers/:id/files/content` | Write file content. |
| GET | `/api/servers/:id/apps` | List apps assigned to a server. |
| POST | `/api/servers/:id/apps/:app_id` | Assign an app to a server. |
| DELETE | `/api/servers/:id/apps/:app_id` | Unassign an app. |

## Build servers

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/build-servers` | List build servers. |
| POST | `/api/build-servers` | Create a build server. |
| GET | `/api/build-servers/:id` | Get a build server. |
| PUT | `/api/build-servers/:id` | Update a build server. |
| DELETE | `/api/build-servers/:id` | Delete a build server. |
| POST | `/api/build-servers/:id/check` | Check build-server health. |

## Cloudflare tunnels

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/tunnels` | List tunnels. |
| POST | `/api/tunnels` | Create a tunnel. |
| DELETE | `/api/tunnels/:id` | Delete a tunnel. |
| POST | `/api/tunnels/:id/start` | Start a tunnel. |
| POST | `/api/tunnels/:id/stop` | Stop a tunnel. |
| GET | `/api/tunnels/:id/routes` | List tunnel routes. |
| POST | `/api/tunnels/:id/routes` | Create a tunnel route. |
| DELETE | `/api/tunnels/:id/routes/:route_id` | Delete a tunnel route. |

## Git providers

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/git-providers` | List providers. |
| POST | `/api/git-providers` | Add a token (PAT) provider. |
| GET | `/api/git-providers/:id` | Get a provider. |
| DELETE | `/api/git-providers/:id` | Delete a provider. |
| GET | `/api/git-providers/:id/repos` | List repos for a provider. |

## GitHub Apps

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/github-apps` | List GitHub Apps. |
| POST | `/api/github-apps` | Create via manifest. |
| GET | `/api/github-apps/installations` | List all installations. |
| GET | `/api/github-apps/installations/:installation_id/repos` | Repos for an installation. |
| GET | `/api/github-apps/installations/:installation_id/repos/:owner/:repo/branches` | Repo branches. |
| GET | `/api/github-apps/:id` | Get an app. |
| DELETE | `/api/github-apps/:id` | Delete an app. |
| GET | `/api/github-apps/:id/install` | Get install URL. |
| GET | `/api/github-apps/:id/installations` | List installations for an app. |
| GET | `/api/github-apps/:id/installations/:iid/repos` | Repos for an installation. |
| POST | `/api/github-apps/:id/sync-webhook` | Sync webhook URL. |

## API tokens

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/tokens` | List your API tokens (scopes, restrictions, status, last use time and IP). |
| POST | `/api/tokens` | Create a token: `name`, `scopes`, optional `expires_at`, `team_id`, `resources`. The secret is returned once. |
| PATCH | `/api/tokens/:id` | Change name, scopes, expiry (`""` clears), team (`""` clears) or resources (`[]` clears). |
| POST | `/api/tokens/:id/pause` | Pause: the token is rejected until resumed. |
| POST | `/api/tokens/:id/resume` | Resume a paused token. |
| POST | `/api/tokens/:id/roll` | New secret (returned once); body `{"grace_minutes": 60}` keeps the old one valid that long. |
| DELETE | `/api/tokens/:id` | Delete an API token. |

All `/api/tokens` routes need the `admin` scope when called with an API token.

## External secret managers

Team-scoped connections to Vault/OpenBao, Infisical, Doppler and AWS Secrets Manager. Credentials are encrypted at rest and never returned. See [secret-managers.md](secret-managers.md).

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/secret-connections` | List connections (no secrets). |
| POST | `/api/secret-connections` | Create: `name`, `provider`, `config`, `credentials`, optional `team_id`. |
| GET | `/api/secret-connections/:id` | One connection. |
| PUT | `/api/secret-connections/:id` | Update; blank credential fields keep the stored value. |
| DELETE | `/api/secret-connections/:id` | Delete. |
| POST | `/api/secret-connections/:id/test` | Test the stored credentials. |
| POST | `/api/secret-connections/test` | Test unsaved settings. |
| GET | `/api/secret-connections/:id/browse?path=` | List folders and key names (never values). |
| GET/POST | `/api/:resource/:id/secret-imports` | List or add a whole-path import (`app`, `service`, `project`). |
| DELETE | `/api/:resource/:id/secret-imports/:import_id` | Remove an import. |
| GET/PUT | `/api/:resource/:id/secret-policy` | Failure policy: `fail` (default) or `last_known`. |

## Multi-server placement

One app on several servers behind the proxy. See [remote-servers.md](remote-servers.md#one-app-on-several-servers).

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/apps/:id/placement` | Placement, rollout settings and current healthy upstreams. |
| PUT | `/api/apps/:id/placement` | Replace the placement: `{"placements":[{"server_id":"localhost","replicas":1}, ...],"settings":{...}}`. An empty list returns the app to single-host deploys. |
| GET | `/api/apps/placement-counts` | Number of servers per app (for the dashboard badge). |
| GET | `/api/deployments/:id/targets` | Per-server status, upstream and log for a deployment. |

## CA certificates

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/ca-certificates` | List CA certs. |
| POST | `/api/ca-certificates` | Add a CA cert. |
| DELETE | `/api/ca-certificates/:id` | Delete a CA cert. |

## Destinations (Docker networks)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/destinations` | List destinations. |
| POST | `/api/destinations` | Create a destination. |
| GET | `/api/destinations/:id` | Get a destination. |
| DELETE | `/api/destinations/:id` | Delete a destination. |

## Two-factor authentication

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/auth/2fa/setup` | Begin 2FA setup. |
| POST | `/api/auth/2fa/verify` | Verify and enable 2FA. |
| POST | `/api/auth/2fa/disable` | Disable 2FA. |
| GET | `/api/auth/2fa/status` | 2FA status. |

## Settings & instance config

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/settings/alert-defaults` | Get default alert config. |
| PUT | `/api/settings/alert-defaults` | Update default alert config. |
| GET | `/api/settings/alert-stats` | Alert statistics. |
| GET | `/api/settings/cost-rates` | Get cost rates. |
| PUT | `/api/settings/cost-rates` | Update cost rates. |
| GET | `/api/settings/instance` | Get instance settings (domain, name). |
| PUT | `/api/settings/instance` | Update instance settings. |
| GET | `/api/settings/oauth-providers` | List OAuth providers (admin). |
| POST | `/api/settings/oauth-providers` | Create an OAuth provider. |
| DELETE | `/api/settings/oauth-providers/:id` | Delete an OAuth provider. |
| GET | `/api/settings/oauth-connections` | List user OAuth connections. |
| DELETE | `/api/settings/oauth-connections/:id` | Remove a user connection. |
| PUT | `/api/white-label` | Update white-label config (admin). |

## SSO providers (admin)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/sso/providers` | List SSO/OIDC providers. |
| POST | `/api/sso/providers` | Create a provider. |
| GET | `/api/sso/providers/:id` | Get a provider. |
| PUT | `/api/sso/providers/:id` | Update a provider. |
| DELETE | `/api/sso/providers/:id` | Delete a provider. |

## System, stats, updates, backups

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/system/stats` | Current system stats. |
| GET | `/api/system/stats/history` | Stats history. |
| GET | `/api/system/stats/summary` | Stats summary. |
| GET | `/api/system/disk` | Disk stats. |
| GET | `/api/system/health` | Detailed health. |
| GET | `/api/system/costs` | Dashboard cost overview. |
| GET | `/api/events/recent` | Recent system events. |
| GET | `/api/system/version` | Version info. |
| POST | `/api/system/update/check` | Check for updates. |
| POST | `/api/system/update/download` | Download an update. |
| POST | `/api/system/update/apply` | Apply an update. |
| POST | `/api/system/backup` | Create a backup. |
| POST | `/api/system/backup/full` | Create a full backup. |
| GET | `/api/system/backups` | List backups. |
| DELETE | `/api/system/backups/:name` | Delete a backup. |
| GET | `/api/system/backups/:name/download` | Download a backup. |
| POST | `/api/system/backups/:name/upload-to-s3` | Upload a backup to S3. |
| POST | `/api/system/restore` | Restore from a backup. |
| GET | `/api/backups/schedules` | List backup schedules. |
| POST | `/api/backups/schedules` | Create a backup schedule. |
| DELETE | `/api/backups/schedules/:id` | Delete a schedule. |
| PUT | `/api/backups/schedules/:id/toggle` | Toggle a schedule. |
| POST | `/api/backups/schedules/:id/run` | Run a schedule now. |
| POST | `/api/system/log-cleanup` | Trigger log cleanup. |
| POST | `/api/system/docker-cleanup` | Prune dangling images. |

## S3 storage

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/s3/configs` | Create an S3 config. |
| GET | `/api/s3/configs` | List S3 configs. |
| PUT | `/api/s3/configs/:id` | Update an S3 config. |
| DELETE | `/api/s3/configs/:id` | Delete an S3 config. |
| POST | `/api/s3/configs/:id/test` | Test an S3 config. |
| POST | `/api/s3/backup` | Trigger an S3 backup (instance, database or volume). |
| GET | `/api/s3/backups` | List S3 backups. |
| POST | `/api/s3/backups/:id/restore` | Restore an S3 backup (database and volume run as a background job). |
| DELETE | `/api/s3/backups/:id` | Delete an S3 backup. |

## Audit, webhook events, proxy logs

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/audit` | List audit logs. |
| GET | `/api/audit/actions` | List audit action types. |
| GET | `/api/audit/resource-types` | List audit resource types. |
| GET | `/api/webhook-events` | List webhook audit events. |
| GET | `/api/proxy/logs` | List proxy access logs. |

## Docker Swarm

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/swarm/init` | Initialize Swarm. |
| GET | `/api/swarm/status` | Swarm status. |
| POST | `/api/swarm/leave` | Leave the Swarm. |
| GET | `/api/swarm/nodes` | List nodes. |
| POST | `/api/swarm/sync-nodes` | Sync nodes. |
| PUT | `/api/swarm/nodes/:id/availability` | Update node availability. |
| GET | `/api/swarm/services` | List Swarm services. |
| POST | `/api/swarm/services` | Create a Swarm service. |
| DELETE | `/api/swarm/services/:id` | Delete a Swarm service. |
| POST | `/api/swarm/services/:id/scale` | Scale a Swarm service. |
| GET | `/api/swarm/services/:id/logs` | Swarm service logs. |

`POST /api/swarm/init` and `POST /api/swarm/leave` reset Docker's networking layer, which clears the container name records behind embedded DNS. Both endpoints reconnect every container on the `rivetr` and `rivetr-app-*` networks before returning, so applications keep resolving databases and services by container name.

## Bulk operations

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/bulk/start` | Start many resources. |
| POST | `/api/bulk/stop` | Stop many resources. |
| POST | `/api/bulk/restart` | Restart many resources. |
| POST | `/api/bulk/deploy` | Deploy many apps. |

## Preview deployments (PR previews)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/apps/:id/previews` | List an app's previews. |
| GET | `/api/previews` | List all previews. |
| GET | `/api/previews/status/:status` | List previews by status. |
| GET | `/api/previews/:id` | Get a preview. |
| DELETE | `/api/previews/:id` | Delete a preview. |
| POST | `/api/previews/:id/redeploy` | Redeploy a preview. |

## AI features

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/apps/:app_id/deployments/:deployment_id/diagnose` | Diagnose a failed deployment. |
| GET | `/api/apps/:app_id/insights` | Deployment insights. |
| GET | `/api/apps/:app_id/cost-suggestions` | Cost-optimization suggestions. |
| POST | `/api/apps/:app_id/suggest-dockerfile` | Suggest a Dockerfile. |
| GET | `/api/apps/:app_id/security-scan` | Scan an app for security issues. |
| GET | `/api/security/scan` | Scan all apps for security issues. |

---

## WebSocket endpoints

Authenticate via query parameter (not the `Authorization` header). All under `/api`.

| Path | Purpose |
|------|---------|
| `/api/deployments/:id/logs/stream` | Stream deployment logs. |
| `/api/apps/:id/terminal` | App container terminal. |
| `/api/servers/:id/terminal` | Remote server terminal. |
| `/api/services/:id/start-stream` | Stream service start logs. |
| `/api/databases/:id/start-stream` | Stream database start logs. |
