# Test Server Credentials

> This file stores credentials created during live testing. Update as you go.
> Do NOT commit this file with real credentials filled in.

---

## Server Access

- **Host:** (fill in)
- **IP:** (fill in)
- **OS:** Ubuntu 22.04 / 24.04 (circle one)
- **SSH:** `ssh root@IP`
- **SSH Key:** (path to private key, or "password auth")
- **Root Password:** (fill in, or leave blank if key-only)

---

## Rivetr Admin

- **Dashboard:** `http://IP:8080`
- **Email:** (fill in)
- **Password:** (fill in)
- **API Token:** (from `/opt/rivetr/rivetr.toml` — `admin_token` field)

To read the token from the server:
```bash
TOKEN=$(grep admin_token /opt/rivetr/rivetr.toml | cut -d'"' -f2)
echo $TOKEN
```

---

## Test Users

| Role | Email | Password | Notes |
|------|-------|----------|-------|
| Owner | | | Primary admin created at setup |
| Admin | | | |
| Developer | | | |
| Viewer | | | |

---

## OAuth Apps (for testing)

### GitHub OAuth
- **Client ID:** (fill in)
- **Client Secret:** (fill in)
- **Callback URL:** `http://IP:8080/api/auth/oauth/github/callback`

### Google OAuth
- **Client ID:** (fill in)
- **Client Secret:** (fill in)
- **Callback URL:** `http://IP:8080/api/auth/oauth/google/callback`

### SSO / OIDC Provider
- **Provider Name:** (fill in)
- **Issuer URL:** (fill in)
- **Client ID:** (fill in)
- **Client Secret:** (fill in)

---

## Git Provider Connections

| Provider | Account | Auth Type | PAT / OAuth Token |
|----------|---------|-----------|-------------------|
| GitHub | | OAuth | |
| GitLab | | PAT | |
| Gitea | | PAT | |
| Bitbucket | | PAT | |

---

## GitHub App (for preview deployments)

- **App ID:** (fill in)
- **App Name:** (fill in)
- **Installation ID:** (fill in — from GitHub App settings)
- **Private Key Path:** (e.g. `/opt/rivetr/github-app.pem`)
- **Webhook Secret:** (fill in)

---

## Test Databases (created during testing)

| App Name | Type | Version | Host | Port | Database | Username | Password |
|----------|------|---------|------|------|----------|----------|---------|
| test-postgres | PostgreSQL | 16 | localhost | (auto) | testdb | testuser | |
| test-mysql | MySQL | 8.0 | localhost | (auto) | testdb | mysqluser | |
| test-redis | Redis | 7 | localhost | (auto) | — | — | |

---

## S3 Configuration (for backup testing)

- **Provider:** AWS S3 / MinIO (circle one)
- **Endpoint:** (fill in — for MinIO, e.g. `https://play.min.io`)
- **Access Key:** (fill in)
- **Secret Key:** (fill in)
- **Bucket:** (fill in)
- **Region:** (fill in — e.g. `us-east-1`)

---

## Notification Channels

| Name | Type | Config / Webhook URL | Tested |
|------|------|---------------------|--------|
| | Slack | | [ ] |
| | Email | | [ ] |
| | Webhook | | [ ] |

---

## Test Apps Deployed

| App Name | Build Type | Git URL / Image | Port | Status | Notes |
|----------|------------|----------------|------|--------|-------|
| nixpacks-test | nixpacks | github.com/heroku/node-js-getting-started | 3000 | | |
| docker-test | dockerfile | github.com/KwaminaWhyte/adamus-forms | 3000 | | |
| image-test | image | nginx:alpine | 80 | | |

---

## Test Services (Compose / Templates)

| Service Name | Template Used | Status | URL |
|-------------|--------------|--------|-----|
| | Grafana | | |
| | Plausible Analytics | | |
| custom-service | (custom YAML) | | |

---

## Teams Created

| Team Name | Slug | ID | Notes |
|-----------|------|-----|-------|
| Personal | personal | | Auto-created |
| Test Team | test-team | | Created during testing |

---

## Projects Created

| Project Name | Team | ID | Notes |
|-------------|------|----|-------|
| Test Project | Test Team | | |

---

## Remote Servers (Multi-Server Testing)

| Server Name | IP | SSH User | Status |
|------------|-----|---------|--------|
| | | root | |

---

## Build Servers

| Server Name | IP | Max Concurrent Builds | Status |
|------------|-----|-----------------------|--------|
| | | 4 | |

---

## Freeze Windows

| Name | Start | End | Active |
|------|-------|-----|--------|
| Holiday Freeze | 2026-12-24T00:00:00Z | 2026-12-26T23:59:59Z | [ ] |

---

## IDs Reference (fill in during testing)

```bash
# Paste IDs here as you create resources during testing
TEAM_ID=""
PROJECT_ID=""
APP_ID=""
DB_ID=""
SERVICE_ID=""
CHANNEL_ID=""
S3_CONFIG_ID=""
SERVER_ID=""
BUILD_SERVER_ID=""
```
