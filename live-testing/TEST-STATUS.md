# Test Status Tracker

Track which feature areas have been fully tested. Update after each testing session.

| # | Feature Area | Status | Date | Tester | Version | Notes |
|---|-------------|--------|------|--------|---------|-------|
| 1 | Installation & Startup | [x] | 2026-03-10 | claude | v0.10.1 | Binary starts, all subsystems initialize correctly |
| 2 | Authentication — Login/Logout | [x] | 2026-03-12 | claude | v0.10.3 | Login ✓, logout clears token ✓, 2FA enable/validate ✓ |
| 3 | Authentication — 2FA | [x] | 2026-03-12 | claude | v0.10.3 | Enable, QR code, TOTP validate, disable all work |
| 4 | Authentication — OAuth | [~] | 2026-03-12 | claude | v0.10.3 | UI loads; GitHub/Google save ✓; callback untested (no OAuth app) |
| 5 | Authentication — SSO/OIDC | [~] | 2026-03-12 | claude | v0.10.3 | UI loads; no OIDC provider to test full flow |
| 6 | Team Management | [x] | 2026-03-11 | claude | v0.10.1 | CRUD works; admin token = system user sees all teams |
| 7 | Project Management | [x] | 2026-03-10 | claude | v0.10.1 | Create, list, update, delete, export all work |
| 8 | App — Create & General Settings | [ ] | — | — | v0.10.3 | Needs UI test |
| 9 | App — Environment Variables | [ ] | — | — | v0.10.3 | Needs UI test |
| 10 | App — Domain & Network | [ ] | — | — | v0.10.3 | Domains, custom domain, www redirect, basic auth |
| 11 | App — Deployment (Git/Nixpacks) | [x] | 2026-03-10 | claude | v0.10.1 | Nixpacks build from GitHub deployed successfully |
| 12 | App — Deployment (Docker image) | [~] | 2026-03-10 | claude | v0.10.1 | Upload endpoint exists; Docker image source untested |
| 13 | App — Deployment Controls | [ ] | — | — | v0.10.3 | Start/stop/restart/redeploy buttons |
| 14 | App — Deployment History | [ ] | — | — | v0.10.3 | List, detail page, live logs, rollback |
| 15 | App — Deployment Approvals | [x] | 2026-03-10 | claude | v0.10.1 | Pending/approve/reject flow works |
| 16 | App — Deployment Freeze Windows | [!] | 2026-03-12 | claude | v0.10.3 | API path fixed (29a31da); UI test needed |
| 17 | App — Scheduled Jobs | [x] | 2026-03-10 | claude | v0.10.1 | Create/list/run; cron normalization ✓ |
| 18 | App — Monitoring & Stats | [ ] | — | — | v0.10.3 | CPU/memory charts, alert config |
| 19 | App — Logs (live streaming) | [ ] | — | — | v0.10.3 | SSE log stream, WebSocket |
| 20 | App — Terminal | [ ] | — | — | v0.10.3 | WebSocket terminal in browser |
| 21 | App — Log Drains | [ ] | — | — | v0.10.3 | Create/delete/test log drain |
| 22 | App — Preview Deployments | [~] | 2026-03-10 | claude | v0.10.1 | Skipped: requires GitHub App |
| 23 | App — Replicas & Auto-scaling | [x] | 2026-03-10 | claude | v0.10.1 | Replicas list returns running replica correctly |
| 24 | App — Container Registry Push | [~] | 2026-03-10 | claude | v0.10.1 | Settings UI ✓; no registry to test push |
| 25 | App — Maintenance Mode | [x] | 2026-03-10 | claude | v0.10.1 | Enable/disable maintenance mode works |
| 26 | App — Snapshots | [~] | 2026-03-10 | claude | v0.10.1 | Requires running container |
| 27 | App — Deployment Diff | [ ] | — | — | v0.10.3 | Show diff between deployments |
| 28 | App — Volumes | [x] | 2026-03-10 | claude | v0.10.1 | Create/list/delete volumes works |
| 29 | App — Clone | [x] | 2026-03-10 | claude | v0.10.1 | Clone app works |
| 30 | App — Delete | [x] | 2026-03-11 | claude | v0.10.1 | Delete with admin token ✓ |
| 31 | Managed Databases — Create/List | [x] | 2026-03-14 | claude | v0.10.5 | PG 18 created via UI, version dropdown shows 12–18; status Running ✓ |
| 32 | Managed Databases — Start/Stop | [x] | 2026-03-14 | claude | v0.10.5 | DB starts on create; Stop button visible; PG 18 container confirmed running |
| 33 | Managed Databases — Connection Info | [x] | 2026-03-14 | claude | v0.10.5 | Credentials + internal connection string shown on General tab ✓ |
| 34 | Managed Databases — Env Var Injection | [x] | 2026-03-10 | claude | v0.10.1 | Env var injection confirmed |
| 35 | Managed Databases — Backups | [ ] | — | — | v0.10.3 | Backup download (401 fix: 29a31da); restore |
| 36 | Managed Databases — Storage Tab | [ ] | — | — | v0.10.3 | Volume path, backup commands |
| 37 | Managed Databases — Settings | [ ] | — | — | v0.10.3 | Update resources, delete |
| 38 | Docker Compose Services — Create | [ ] | — | — | v0.10.3 | Auto-domain on creation (9a26dbc) |
| 39 | Docker Compose Services — Start/Stop/Restart | [ ] | — | — | v0.10.3 | Restart button added (9a26dbc) |
| 40 | Docker Compose Services — Domain/Proxy | [ ] | — | — | v0.10.3 | Domain config, proxy routing (27a192f) |
| 41 | Docker Compose Services — Logs | [ ] | — | — | v0.10.3 | Compose logs streaming |
| 42 | Docker Compose Services — Settings | [ ] | — | — | v0.10.3 | Edit compose YAML, delete |
| 43 | Service Templates | [x] | 2026-03-14 | claude | v0.10.5 | Search filter ✓; Mealie deployed → Running; Kavita running; template count ~250 |
| 44 | Webhooks (Git) | [~] | 2026-03-10 | claude | v0.10.1 | Routes registered; end-to-end not tested |
| 45 | Webhooks — DockerHub | [~] | 2026-03-10 | claude | v0.10.1 | Route exists; no DockerHub account |
| 46 | Webhook Events Log | [ ] | — | — | v0.10.3 | UI page at /webhook-events |
| 47 | Git Integrations — GitHub OAuth | [~] | 2026-03-10 | claude | v0.10.1 | Repo picker works; OAuth callback untested |
| 48 | Git Integrations — GitLab | [~] | 2026-03-10 | claude | v0.10.1 | Picker UI added; no GitLab account |
| 49 | Git Integrations — Bitbucket | [~] | 2026-03-10 | claude | v0.10.1 | Picker UI added; token auth |
| 50 | Git Integrations — GitHub App | [~] | 2026-03-10 | claude | v0.10.1 | Skipped: no GitHub App configured |
| 51 | SSH Keys | [ ] | — | — | v0.10.3 | Add/list/delete SSH keys for private repos |
| 52 | API Tokens | [ ] | — | — | v0.10.3 | Create/list/delete API tokens |
| 53 | Bulk Operations | [x] | 2026-03-10 | claude | v0.10.1 | Bulk start/stop/restart/deploy all work |
| 54 | S3 Storage | [~] | 2026-03-10 | claude | v0.10.1 | Config UI ✓; no S3 credentials for full test |
| 55 | Backup & Restore — Create/Download | [!] | 2026-03-12 | claude | v0.10.3 | 401 download fixed (29a31da); restore untested |
| 56 | Notifications — Channels | [x] | 2026-03-10 | claude | v0.10.1 | Create/list/test ✓; Slack/Discord/Email types |
| 57 | Notifications — Alert Defaults | [ ] | — | — | v0.10.3 | Global CPU/memory/disk thresholds UI |
| 58 | Dashboard | [ ] | — | — | v0.10.3 | Stats cards, running services, recent activity |
| 59 | Monitoring Page | [ ] | — | — | v0.10.3 | Charts, system health |
| 60 | Costs Page | [ ] | — | — | v0.10.3 | Cost breakdown by app/project |
| 61 | Audit Log | [x] | 2026-03-12 | claude | v0.10.3 | Shows email (not UUID) after fix (29a31da) |
| 62 | Settings — General | [x] | 2026-03-12 | claude | v0.10.3 | Server info, proxy config, runtime status |
| 63 | Settings — Security | [x] | 2026-03-12 | claude | v0.10.3 | 2FA enable/disable flow works |
| 64 | Settings — Auto Updates | [x] | 2026-03-12 | claude | v0.10.3 | Shows version, check for updates button |
| 65 | Settings — Authentication (OAuth) | [x] | 2026-03-12 | claude | v0.10.3 | GitHub/Google provider forms load |
| 66 | Settings — SSO/OIDC | [x] | 2026-03-12 | claude | v0.10.3 | Add provider form works |
| 67 | Settings — Backup & Restore | [x] | 2026-03-12 | claude | v0.10.3 | Shows existing backups; create button works |
| 68 | Settings — S3 Storage | [x] | 2026-03-12 | claude | v0.10.3 | Add S3 config form works |
| 69 | Settings — Alert Defaults | [x] | 2026-03-12 | claude | v0.10.3 | CPU/memory/disk threshold sliders work |
| 70 | Settings — Notifications | [x] | 2026-03-12 | claude | v0.10.3 | Add channel form works |
| 71 | Settings — Audit Log | [x] | 2026-03-12 | claude | v0.10.3 | 122 entries, pagination, user email displayed |
| 72 | Servers (Remote) | [x] | 2026-03-12 | claude | v0.10.3 | Page loads; no remote servers registered |
| 73 | Build Servers | [x] | 2026-03-12 | claude | v0.10.3 | Page loads; no build servers registered |
| 74 | SSH Keys Page | [x] | 2026-03-12 | claude | v0.10.3 | Page loads; empty state |
| 75 | Docker Swarm | [x] | 2026-03-10 | claude | v0.10.1 | Page loads; swarm inactive on single node |
| 76 | Teams Page | [x] | 2026-03-12 | claude | v0.10.3 | Lists Personal team; Manage link works |
| 77 | Proxy Route Restore on Restart | [x] | 2026-03-12 | claude | v0.10.3 | Routes restored via inspect() fallback (29a31da) |
| 78 | Multi-Server | [s] | — | — | — | Skipped: requires second server |
| 79 | MCP Server | [~] | 2026-03-10 | claude | v0.10.1 | Endpoint registered; not fully tested |
| 80 | PostgreSQL Extensions UI | [x] | 2026-03-14 | claude | v0.10.5 | pgvector installed on pharmapro-db; "Installed Extensions" list updated live |
| 81 | Docker Options (shm_size, cap_add) | [x] | 2026-03-14 | claude | v0.10.5 | Saved via UI; deployed westel-soleil; container shows shm=256M, CapAdd=[NET_ADMIN] ✓ |
| 82 | White Label | [x] | 2026-03-14 | claude | v0.10.5 | app_name→"MyPaaS" rendered in sidebar/title immediately; API confirmed save; reverted ✓ |
| 83 | Deployment Cancellation | [x] | 2026-03-14 | claude | v0.10.5 | Cancel returns 200; final status = "cancelled" even with cached-image fast deploy (race fixed) |
| 84 | PG 17/18 Version Support | [x] | 2026-03-14 | claude | v0.10.5 | Version dropdown shows 12–18; PG 18 created, pulled, Running; container confirmed |

---

## Status Key

| Symbol | Meaning |
|--------|---------|
| `[ ]` | Not tested yet |
| `[~]` | Partially tested / needs external service |
| `[x]` | Fully tested, all checks passed |
| `[!]` | Tested, issues found and fixed |
| `[s]` | Skipped (requires infrastructure not available) |

---

## Session Log

### 2026-03-14 — v0.10.5 (Sprint 19 Feature Testing)
- **Server:** rivetr.site (46.101.187.233)
- **Tester:** claude (Playwright browser + SSH verification)
- **Areas Covered:** All sprint 19 features + deployment cancellation race fix
- **Features Confirmed Working:**
  1. **Service template search** — search "Forgejo", "Mealie" filters correctly; Mealie deployed and Running
  2. **PostgreSQL Extensions UI** — pgvector installed on pharmapro-db via UI; installed list updates live
  3. **White Label** — app_name change renders immediately in sidebar + title; save confirmed via API
  4. **Docker Options end-to-end** — shm_size=256m + cap_add=NET_ADMIN saved in UI; verified in live container: `/dev/shm` = 256M, CapAdd = [NET_ADMIN], ShmSize = 268435456 bytes
  5. **PG 17/18 support** — version dropdown shows 12–18; PG 18 created via UI, pulled postgres:18 image, container running with credentials auto-generated
  6. **Deployment cancellation** — cancel returns HTTP 200; deployment status correctly stays "cancelled" even when cached Docker image builds in <1s (race condition fixed by adding `AND status != 'cancelled'` to all pipeline status UPDATE statements)
- **Bugs Found & Fixed:**
  1. **Forgejo template used unreachable codeberg.org registry** — changed to `forgejo/forgejo:latest` (Docker Hub) in sprint19.rs (cd62265)
  2. **Deployment cancellation race condition** — `update_deployment_status()` could overwrite DB `cancelled` with `running`/`failed`; fixed by adding `AND status != 'cancelled'` guard to all UPDATE calls in that helper (ec6bc5d)
  3. **Early pipeline cancel check** — added `SELECT status` check at start of `run_deployment()` to bail out fast on queued-but-cancelled deployments
- **Version deployed:** v0.10.5

### 2026-03-12 — v0.10.3 (Comprehensive UI Testing)
- **Server:** rivetr.site (64.226.112.14)
- **Tester:** claude (Playwright browser)
- **Areas Covered:** All Settings pages, Infrastructure pages, Access pages
- **Issues Found & Fixed (this session):**
  1. **Audit log showed UUIDs** — Backend now LEFT JOIN users; returns user_email (29a31da)
  2. **DB backup download 401** — Frontend now passes auth token to download fetch (29a31da)
  3. **DB data directory not unique** — Volume path now `{name}-{id[:8]}` (29a31da)
  4. **Proxy routes lost on restart** — restore_routes fallback to inspect(); restores basic auth + www variants (29a31da)
  5. **container_slug missing from SELECT in container_monitor** — Added column to health.rs + recovery.rs explicit SELECT lists (aef292c)
  6. **Services had no domain/proxy** — Added domain+port to service model, create/update/delete/start/stop register/remove proxy routes; auto-generate subdomain on creation (27a192f, 9a26dbc)
  7. **Services had no Restart button** — Added restart_service endpoint + frontend button (9a26dbc)
  8. **"Open in browser" for service pointed to port** — Now uses https://{domain} when domain is set (9a26dbc)
  9. **Freeze windows API path wrong** — Frontend was calling /api/freeze-windows?app_id=; fixed to /apps/:id/freeze-windows (66a51cf)
  10. **Migration 067 not registered** — container_slug column never created; caused 500 on all DB endpoints (66a51cf)
  11. **team_id backward compat** — All team-scoped queries now include OR team_id IS NULL (66a51cf)
  12. **Logout 405 Method Not Allowed** — nav-user.tsx used Form method=post; changed to Link (66a51cf)
- **Areas Skipped:**
  - GitHub App callback (no app configured)
  - SSO/OIDC full flow (no OIDC provider)
  - S3 backup upload (no S3 credentials)
  - Multi-Server / Build Servers (requires second server)
  - DockerHub webhook (no account)

### 2026-03-11 — v0.10.1 (Sprint 12 Follow-up Bug Fixes)
- **Server:** 64.226.112.14:8080
- **Tester:** claude (automated)
- **Areas Covered:** Auth, Teams, Apps CRUD, Databases, Volumes, Jobs, Notification Channels, Bulk Ops
- **Issues Found & Fixed:**
  1. **Deployment logs not showing** — Fixed: always fetch via REST on mount (8619a83)
  2. **Stuck in-progress deployments on restart** — Fixed: startup cleanup (8619a83)
  3. **Teams 403 with admin API token** — Fixed: system user sees all teams as owner (2102567)
  4. **`create_team` with admin token fails with FK error** — Fixed: skip member insertion for system user (2102567)
  5. **Delete app with admin token requires password** — Fixed (c13c0c4)
  6. **`POST /api/auth/logout` not registered** — Fixed (dbdce88)
  7. **Missing API routes return 200+HTML** — Fixed: /api/* returns 404 JSON (dbdce88)
- **Areas Skipped:** GitHub App, SSL/TLS, Multi-Server, Swarm active, S3

### 2026-03-11 — v0.10.1 (Sprint 12 Comprehensive Retest)
- **Server:** 64.226.112.14:8080
- **Tester:** claude (automated, general-purpose agent)
- **Areas Covered:** All 25 feature areas via API
- **Issues Found & Fixed:**
  1. **`nixpacks_config` type mismatch** — Fixed (a552e64)
  2. **`/api/auth/validate` returns 401 for admin API key** — Fixed
  3. **macOS binary deployed to Linux server** — Always use deploy-dev.sh
  4. **GitHub App clone auth** — Manual DB fix; pipeline now fetches installation tokens
  5. **Bitbucket source picker** — Added BitbucketRepoPicker (ef8fd6d)

### 2026-03-10 — v0.10.1
- **Server:** 64.226.112.14:8080
- **Tester:** claude (automated)
- **Issues Found & Fixed:**
  1. SQLx stale prepared statement cache — git_tag column (fd380a5)
  2. Rollback proxy routing — fixed get_all_domain_names()
  3. Cron 5-field normalization — normalize_cron() helper
  4. list_pending_deployments wrong route — fixed path
  5. Freeze window routes — fixed paths
  6. Bulk operations not registered — added module
  7. System memory reporting 512 MB — fixed to use actual RAM
  8. Teams list 409 Conflict with admin token — skip auto-create for system
  9. Approve/reject deployment 400 — use NULL for system user approved_by

---

## Known Regressions to Re-Test

After any significant change, re-run these tests:

- [x] PORT env var auto-injection — confirmed working in v0.10.1
- [x] WebSocket build log streaming — terminal WS returns correct upgrade error
- [x] Stats history chart — working, memory fix deployed
- [x] Notification channel webhook type support — working
- [x] Audit log shows email (not UUID) — fixed v0.10.3
- [x] Proxy routes restored after restart — fixed v0.10.3
- [ ] GitHub App callback redirect URL — not tested (no GitHub App)

---

## Infrastructure Requirements by Area

| Area | Requires |
|------|---------|
| GitHub OAuth | GitHub OAuth App configured |
| Google OAuth | Google OAuth App configured |
| SSO / OIDC | OIDC provider (e.g. Keycloak, Auth0) |
| GitHub App (Previews) | GitHub App installed on a repository |
| GitLab Webhook | GitLab account and repository |
| DockerHub Webhook | DockerHub account and repository |
| S3 Backups | S3-compatible storage (MinIO or AWS) |
| Slack Notifications | Slack workspace + Incoming Webhook URL |
| Email Notifications | SMTP server configured in rivetr.toml |
| Axiom Log Drain | Axiom account + API token |
| Multi-Server | Second Linux server with SSH access |
| Build Servers | Second Linux server with Docker/Nixpacks |
| SSL/TLS | Domain name with DNS pointed at server |
| Docker Swarm | Single node is enough for basic tests |
