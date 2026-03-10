# Test Status Tracker

Track which feature areas have been fully tested. Update after each testing session.

| # | Feature Area | Status | Date | Tester | Version | Notes |
|---|-------------|--------|------|--------|---------|-------|
| 1 | Installation & Startup | [ ] | | | | |
| 2 | Authentication | [ ] | | | | |
| 3 | Team Management | [ ] | | | | |
| 4 | Project Management | [ ] | | | | |
| 5 | App Deployment — Git | [ ] | | | | |
| 6 | App Deployment — Other Sources | [ ] | | | | |
| 7 | Webhooks | [ ] | | | | |
| 8 | App Settings & Control | [ ] | | | | |
| 9 | Deployment Management | [ ] | | | | |
| 10 | Container Replicas & Auto-scaling | [ ] | | | | |
| 11 | Container Terminal & Logs | [ ] | | | | |
| 12 | Managed Databases | [ ] | | | | |
| 13 | Docker Compose Services | [ ] | | | | |
| 14 | Service Templates | [ ] | | | | |
| 15 | Bulk Operations | [ ] | | | | |
| 16 | S3 & Backups | [ ] | | | | |
| 17 | Notifications & Alerts | [ ] | | | | |
| 18 | Multi-Server | [ ] | | | | |
| 19 | Docker Swarm | [ ] | | | | |
| 20 | Build Servers | [ ] | | | | |
| 21 | Scheduled Jobs | [ ] | | | | |
| 22 | System | [ ] | | | | |
| 23 | Security | [ ] | | | | |

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

---

## Known Regressions to Re-Test

After any significant change, re-run these tests that have previously failed:

- [ ] PORT env var auto-injection (was broken in v0.2.9)
- [ ] WebSocket build log streaming
- [ ] Stats history chart (`/api/system/stats/history`)
- [ ] Notification channel webhook type support
- [ ] GitHub App callback redirect URL

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
