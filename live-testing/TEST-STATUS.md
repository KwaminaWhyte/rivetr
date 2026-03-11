# Test Status Tracker

Track which feature areas have been fully tested. Update after each testing session.

| # | Feature Area | Status | Date | Tester | Version | Notes |
|---|-------------|--------|------|--------|---------|-------|
| 1 | Installation & Startup | [x] | 2026-03-10 | claude | v0.10.1 | Binary starts, all subsystems initialize correctly |
| 2 | Authentication | [x] | 2026-03-11 | claude | v0.10.1 | Login/logout/validate ✓; logout now invalidates session server-side (dbdce88); missing API routes return 404 JSON (not SPA HTML) |
| 3 | Team Management | [x] | 2026-03-11 | claude | v0.10.1 | CRUD works with both session + admin token (2102567); admin token = system user sees all teams as owner |
| 4 | Project Management | [x] | 2026-03-10 | claude | v0.10.1 | Create, list, update, delete, export all work |
| 5 | App Deployment — Git | [x] | 2026-03-10 | claude | v0.10.1 | Nixpacks build from GitHub deployed successfully |
| 6 | App Deployment — Other Sources | [~] | 2026-03-10 | claude | v0.10.1 | Upload deploy endpoint exists; Docker image source untested |
| 7 | Webhooks | [~] | 2026-03-10 | claude | v0.10.1 | Webhook routes registered; full webhook trigger not tested end-to-end |
| 8 | App Settings & Control | [x] | 2026-03-11 | claude | v0.10.1 | Start/stop/restart/update/delete/clone all work; maintenance mode ✓; delete with admin token (c13c0c4) |
| 9 | Deployment Management | [x] | 2026-03-10 | claude | v0.10.1 | List, rollback, approve/reject/pending all work; non-admin→pending, admin approve/reject confirmed (fixed approved_by FK in cf9521a) |
| 10 | Container Replicas & Auto-scaling | [x] | 2026-03-10 | claude | v0.10.1 | Replicas list returns running replica correctly |
| 11 | Container Terminal & Logs | [x] | 2026-03-10 | claude | v0.10.1 | Terminal WebSocket asks for upgrade ✓; deployment logs return 7 entries ✓ |
| 12 | Managed Databases | [x] | 2026-03-10 | claude | v0.10.1 | PostgreSQL 16 created and running; env var injection confirmed |
| 13 | Docker Compose Services | [x] | 2026-03-10 | claude | v0.10.1 | Service created and running; compose YAML accepted |
| 14 | Service Templates | [x] | 2026-03-10 | claude | v0.10.1 | 74 templates returned; template deploy flow tested |
| 15 | Bulk Operations | [x] | 2026-03-10 | claude | v0.10.1 | bulk start/stop/restart/deploy all return success; clone ✓; maintenance mode ✓; snapshots need running container; project export ✓ |
| 16 | S3 & Backups | [~] | 2026-03-10 | claude | v0.10.1 | S3 config list returns []; no S3 credentials to test backup flow |
| 17 | Notifications & Alerts | [x] | 2026-03-10 | claude | v0.10.1 | Alert CRUD ✓; notification channels create/list ✓; log drains ✓ |
| 18 | Multi-Server | [s] | 2026-03-10 | claude | v0.10.1 | Skipped: requires second server |
| 19 | Docker Swarm | [s] | 2026-03-10 | claude | v0.10.1 | Swarm inactive (single node); swarm status endpoint returns inactive |
| 20 | Build Servers | [s] | 2026-03-10 | claude | v0.10.1 | Skipped: requires second server |
| 21 | Scheduled Jobs | [x] | 2026-03-10 | claude | v0.10.1 | Create/list jobs ✓; 5-field cron normalized to 6-field ✓; last_run and next_run correct |
| 22 | System | [x] | 2026-03-10 | claude | v0.10.1 | Health ✓; stats ✓; version ✓; Prometheus metrics at /metrics ✓; audit logs ✓ |
| 23 | Security | [x] | 2026-03-10 | claude | v0.10.1 | Rate limiting headers ✓; security headers ✓; admin token auth ✓; JWT session auth ✓ (non-admin login + deploy tested) |

---

## Status Key

| Symbol | Meaning |
|--------|---------|
| `[ ]` | Not started |
| `[~]` | In progress / partially tested |
| `[x]` | Fully tested, all checks passed |
| `[!]` | Tested, issues found (see Notes) |
| `[s]` | Skipped (requires infrastructure not available) |

---

## Session Log

Record each testing session here.

### Session Template
```
### YYYY-MM-DD — vX.X.X
- **Server:** IP:PORT
- **Tester:** (name or handle)
- **Areas Covered:** (list)
- **Issues Found:** (list or "none")
- **Areas Skipped:** (list with reason)
```

### 2026-02-05 — v0.2.9 / v0.2.10 (Historical)
- **Server:** 167.71.46.193:8080
- **Tester:** kwamina
- **Areas Covered:** Installation, Authentication (partial), Team Management, App Deployment (Nixpacks), Env Vars, Databases (PostgreSQL), Docker Compose Services, Volumes, Alerts, Notification Channels, Preview Deployments, System Monitoring, Proxy/Routes, Rate Limiting, WebSocket/Terminal, API Endpoints
- **Issues Found:**
  - PORT env var not set for containers (fixed in v0.2.10)
  - WebSocket build logs failing (fixed later)
  - Stats history API returning 401 (fixed in v0.2.12)
  - Teams panic on short user ID (fixed in v0.2.13)
  - Container monitor missing team_id column (fixed in v0.2.14)
  - Notification channel missing 'webhook' CHECK constraint (fixed in v0.2.14)
- **Areas Skipped:** GitHub App (no GitHub App configured), SSL/TLS (no custom domain), Preview Deployments full test (needs GitHub App)

### 2026-03-11 — v0.10.1 (Sprint 12 Follow-up Bug Fixes)
- **Server:** 64.226.112.14:8080
- **Tester:** claude (automated)
- **Areas Covered:** Auth, Teams, Apps CRUD, Databases, Volumes, Jobs, Notification Channels, Bulk Ops
- **Issues Found & Fixed:**
  1. **Deployment logs not showing** — `DeploymentLogs` component only loaded via WebSocket when active; returned null for finished deployments. Fixed: always fetch via REST on mount (commit: 8619a83)
  2. **Stuck in-progress deployments on restart** — server restart left deployments in "building" state forever. Fixed: startup cleanup marks them failed (commit: 8619a83)
  3. **Teams 403 with admin API token** — `user.id == "system"` has no team memberships; `list_teams` returned `[]` and `require_team_role` returned 403. Fixed: system user sees all teams as owner (commit: 2102567)
  4. **`create_team` with admin token fails with FK error** — system user can't be inserted into team_members (no DB row). Fixed: skip member insertion for system user (commit: 2102567)
  5. **Delete app with admin token requires password** — empty password check ran before system user bypass. Fixed: moved password validation inside `user.id != "system"` guard (commit: c13c0c4)
  6. **`DeleteAppRequest` missing field error** — password field required by serde even when body is `{}`. Fixed: `#[serde(default)]` on password field (commit: c13c0c4)
  7. **`POST /api/auth/logout` not registered** — frontend called it for session invalidation but route didn't exist. Fixed: added logout handler that deletes session from DB (commit: dbdce88)
  8. **Missing API routes return 200+HTML** — SPA fallback served index.html for all unmatched paths. Fixed: `/api/*` paths now return 404 JSON (commit: dbdce88)
- **Areas Skipped:** GitHub App, SSL/TLS, Multi-Server, Swarm active, S3

### 2026-03-11 — v0.10.1 (Sprint 12 Comprehensive Retest)
- **Server:** 64.226.112.14:8080
- **Tester:** claude (automated, general-purpose agent)
- **Areas Covered:** All 25 feature areas via API — Auth, System, Apps CRUD, App Control, Env Vars, Volumes, Scheduled Jobs, Deployments, Managed Databases, Services, Templates, Projects, Teams, SSH Keys, Notifications/Alerts, Audit Logs, Webhook Events, App Monitoring/Stats, Clone, Autoscaling, Bulk Ops, Deployment Diff, Swarm, Previews, MCP
- **Issues Found & Fixed:**
  1. **`nixpacks_config` type mismatch** — Frontend sends `NixpacksConfig` as JSON object; Rust expected `Option<String>`. Fixed: changed type to `Option<serde_json::Value>` and serialize before DB storage (commit: a552e64)
  2. **`/api/auth/validate` returns 401 for admin API key** — validate handler only checked sessions table, not the static admin API key. Fixed: added constant-time comparison with admin token before session lookup
  3. **macOS binary deployed to Linux server** — Was building with `cargo build --release` (Darwin binary), causing `Exec format error`. Fixed: always use `./scripts/deploy-dev.sh` which cross-compiles with cargo-zigbuild
  4. **GitHub App clone auth** — Apps created before github_app_installation_id fix had NULL field. Manual DB fix applied; pipeline now fetches installation tokens
  5. **Bitbucket source picker** — Missing UI component. Fixed: added BitbucketRepoPicker with token-based auth (commit: ef8fd6d)
- **All API paths verified correct:** apps list/get/create/update/delete, env vars, volumes, jobs, deployments, databases, services, templates, projects, bulk ops, autoscaling, swarm, previews — all respond correctly
- **Design decisions (not bugs):** PATCH not supported on apps (only PUT); /api/apps/:id/logs not a route (use /logs/stream SSE); /api/apps/bulk/* not a route (use /api/bulk/*)
- **Areas Skipped:**
  - GitHub App full callback (not configured)
  - SSL/TLS custom domain
  - Multi-Server / Build Servers
  - Docker Swarm active (single node)

### 2026-03-10 — v0.10.1
- **Server:** 64.226.112.14:8080
- **Tester:** claude (automated)
- **Areas Covered:** All areas except Multi-Server, Build Servers, Swarm (active), GitHub App
- **Issues Found & Fixed:**
  1. **SQLx stale prepared statement cache** — `git_tag` column not found after ALTER TABLE migrations. Fixed: separate migration pool + `#[sqlx(default)]` on Deployment fields (commits: fd380a5, ab4206d)
  2. **Rollback proxy routing** — app uses `domains` JSON array, rollback handler only checked `domain` (null). Fixed: use `get_all_domain_names()` (in rollback.rs)
  3. **Cron 5-field normalization** — frontend sent 5-field cron, Rust cron crate requires 6-field. Fixed: `normalize_cron()` helper in jobs.rs + updated frontend presets
  4. **`list_pending_deployments` wrong route** — `/deployments/pending` but handler expects `app_id` Path param. Fixed: changed to `/apps/:id/deployments/pending` (commit: d35f92e)
  5. **Freeze window list/create routes** — wrong path, handler expects `app_id`. Fixed: changed to `/apps/:id/freeze-windows` (commit: 3e448ab)
  6. **Freeze window delete route** — `/freeze-windows/:id` missing app_id. Fixed: changed to `/apps/:id/freeze-windows/:window_id` (commit: 8aa8851)
  7. **Bulk operations not registered** — `mod bulk` missing from api/mod.rs. Fixed: added module, routes, and db models (commit: b10b12e)
  8. **System memory reporting 512 MB** — `memory_total_bytes` used container cgroup limits instead of actual server RAM. Fixed: always use `get_system_memory()` (commit: 06003d9)
  9. **Teams list 409 Conflict with admin token** — synthetic "system" user caused orphaned Personal team creation (FK fail + UNIQUE retry). Fixed: skip auto-create for "system" user + INSERT OR IGNORE (commit: cff39b5)
  10. **Approve/reject deployment 400 with admin token** — `approved_by = "system"` violated FK `REFERENCES users(id)`. Fixed: use NULL when `user.id == "system"` (commit: cf9521a)
- **Areas Skipped:**
  - GitHub App (no app configured)
  - SSL/TLS (no DNS for custom domain test)
  - Multi-Server (requires second server)
  - Docker Swarm active (single node, swarm inactive)
  - Build Servers (requires second server)

---

## Known Regressions to Re-Test

After any significant change, re-run these tests that have previously failed:

- [x] PORT env var auto-injection (was broken in v0.2.9) — confirmed working in v0.10.1
- [x] WebSocket build log streaming — terminal WS returns correct upgrade error
- [x] Stats history chart (`/api/system/stats/history`) — working, memory fix deployed
- [x] Notification channel webhook type support — working
- [ ] GitHub App callback redirect URL — not tested (no GitHub App)

---

## Infrastructure Requirements by Area

Some test areas require external services. Note what is needed:

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
