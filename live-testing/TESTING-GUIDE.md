# Rivetr Comprehensive Testing Guide

Covers v0.3 through v0.10. Each item is a checklist box so you can check off tests as you run them.

**How to use:** Set `TOKEN` and `SERVER` before running curl commands.
```bash
export SERVER="http://YOUR_IP:8080"
export TOKEN="your-api-token-here"
```

---

## Table of Contents

1. [Installation & Startup](#1-installation--startup)
2. [Authentication](#2-authentication)
3. [Team Management](#3-team-management)
4. [Project Management](#4-project-management)
5. [App Deployment — Git](#5-app-deployment--git)
6. [App Deployment — Other Sources](#6-app-deployment--other-sources)
7. [Webhooks](#7-webhooks)
8. [App Settings & Control](#8-app-settings--control)
9. [Deployment Management](#9-deployment-management)
10. [Container Replicas & Auto-scaling](#10-container-replicas--auto-scaling)
11. [Container Terminal & Logs](#11-container-terminal--logs)
12. [Managed Databases](#12-managed-databases)
13. [Docker Compose Services](#13-docker-compose-services)
14. [Service Templates](#14-service-templates)
15. [Bulk Operations](#15-bulk-operations)
16. [S3 & Backups](#16-s3--backups)
17. [Notifications & Alerts](#17-notifications--alerts)
18. [Multi-Server](#18-multi-server)
19. [Docker Swarm](#19-docker-swarm)
20. [Build Servers](#20-build-servers)
21. [Scheduled Jobs](#21-scheduled-jobs)
22. [System](#22-system)
23. [Security](#23-security)

---

## 1. Installation & Startup

### 1.1 Fresh Install via Curl Script
- [ ] **Clean state** — Remove any prior installation before testing
  ```bash
  systemctl stop rivetr 2>/dev/null
  rm -rf /opt/rivetr /var/lib/rivetr /etc/systemd/system/rivetr.service
  userdel rivetr 2>/dev/null
  systemctl daemon-reload
  # Stop conflicting services
  systemctl stop nginx 2>/dev/null && systemctl disable nginx 2>/dev/null
  ```
  **Expected:** No errors (missing dirs/users are fine)

- [ ] **Run installer**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
  ```
  **Expected:** Script runs to completion, reports success

- [ ] **Binary downloaded from GitHub Releases** (not built from source)
  ```bash
  curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh > /tmp/install.sh
  bash -x /tmp/install.sh 2>&1 | grep -i "download\|release"
  ```
  **Expected:** Log line containing "Downloaded binary from GitHub releases"

- [ ] **Build tools installed**
  ```bash
  nixpacks --version
  pack version
  docker --version
  ```
  **Expected:** All three print version strings

- [ ] **Rivetr binary present and executable**
  ```bash
  ls -la /opt/rivetr/rivetr
  /opt/rivetr/rivetr --version 2>/dev/null || echo "no --version flag"
  ```
  **Expected:** File exists, is executable

### 1.2 Config File Verification
- [ ] **Config file created at expected path**
  ```bash
  cat /opt/rivetr/rivetr.toml
  ```
  **Expected:** File contains `admin_token`, `[server]`, `[database]` sections

- [ ] **Data directory created**
  ```bash
  ls -la /var/lib/rivetr/
  ```
  **Expected:** Directory exists, contains `rivetr.db` after first start

### 1.3 Service Status
- [ ] **Systemd service running**
  ```bash
  systemctl status rivetr
  ```
  **Expected:** `Active: active (running)`

- [ ] **Service listens on port 8080**
  ```bash
  ss -tlnp | grep 8080
  ```
  **Expected:** Port 8080 bound

- [ ] **Health endpoint responds**
  ```bash
  curl http://localhost:8080/health
  ```
  **Expected:** `OK`

### 1.4 Dashboard Loads
- [ ] Open `http://SERVER_IP:8080` in a browser
  **Expected:** Login/setup page loads, no 502 or blank screen

### 1.5 First-Time Admin Account Creation
- [ ] Complete the first-time setup form (name, email, password)
  **Expected:** Redirected to main dashboard after submit

### 1.6 Upgrade / Re-installation
- [ ] **Re-run installer on existing install**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
  ```
  **Expected:** Config preserved, data preserved, service restarts with new binary

---

## 2. Authentication

### 2.1 Email/Password Login
- [ ] **Login with correct credentials**
  ```bash
  curl -X POST -H "Content-Type: application/json" \
    -d '{"email":"admin@example.com","password":"YourPassword"}' \
    $SERVER/api/auth/login
  ```
  **Expected:** `200 OK` with `{ "token": "..." }`

- [ ] **Login with wrong password returns 401**
  ```bash
  curl -s -o /dev/null -w "%{http_code}" -X POST \
    -H "Content-Type: application/json" \
    -d '{"email":"admin@example.com","password":"wrong"}' \
    $SERVER/api/auth/login
  ```
  **Expected:** `401`

- [ ] **Session persists on page refresh** — reload dashboard in browser
  **Expected:** Still logged in, no redirect to login page

- [ ] **Logout clears session** — click logout, refresh
  **Expected:** Redirected to login page

### 2.2 GitHub OAuth Login
- [ ] Navigate to login page, click "Sign in with GitHub"
  **Expected:** Redirected to GitHub, back to dashboard after authorizing
  **Notes:** Requires OAuth app configured in `rivetr.toml` or via Settings > OAuth Providers

### 2.3 Google OAuth Login
- [ ] Navigate to login page, click "Sign in with Google"
  **Expected:** Redirected to Google, back to dashboard after authorizing
  **Notes:** Requires Google OAuth app configured

### 2.4 SSO / OIDC Login
- [ ] Initiate SSO login via provider URL
  ```bash
  # Replace PROVIDER_ID with the ID from /api/sso/providers
  curl -v "$SERVER/auth/sso/PROVIDER_ID/login"
  ```
  **Expected:** Redirect to OIDC provider's authorization endpoint

- [ ] Complete OIDC callback and land on dashboard
  **Expected:** Logged in as SSO user

### 2.5 Two-Factor Authentication (TOTP)
- [ ] **2FA setup — get QR code**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/auth/2fa/setup
  ```
  **Expected:** `{ "secret": "...", "qr_code": "data:image/png;base64,..." }`

- [ ] **Scan QR code in authenticator app** (Google Authenticator, Authy, etc.)
  **Expected:** 6-digit codes generated

- [ ] **Verify TOTP to complete setup**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"code":"123456"}' \
    $SERVER/api/auth/2fa/verify
  ```
  **Expected:** `{ "recovery_codes": [...] }` — save these codes

- [ ] **2FA status shows enabled**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/auth/2fa/status
  ```
  **Expected:** `{ "enabled": true }`

- [ ] **Login flow requires TOTP after enabling** — log out, log back in, enter TOTP code
  **Expected:** Login blocked until correct TOTP entered

- [ ] **Recovery codes work** — use a recovery code instead of TOTP
  ```bash
  curl -X POST -H "Content-Type: application/json" \
    -d '{"temp_token":"<from_login>","code":"RECOVERY-CODE"}' \
    $SERVER/api/auth/2fa/validate
  ```
  **Expected:** `200 OK` with full session token

- [ ] **Disable 2FA**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"code":"123456"}' \
    $SERVER/api/auth/2fa/disable
  ```
  **Expected:** `200 OK`, 2FA disabled

### 2.6 API Token Authentication
- [ ] **Bearer token accepted**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps
  ```
  **Expected:** `200 OK` with app list

- [ ] **Missing token returns 401**
  ```bash
  curl -s -o /dev/null -w "%{http_code}" $SERVER/api/apps
  ```
  **Expected:** `401`

- [ ] **Invalid token returns 401**
  ```bash
  curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer invalidtoken" $SERVER/api/apps
  ```
  **Expected:** `401`

---

## 3. Team Management

### 3.1 Create Team
- [ ] **Personal team auto-created** on first login
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/teams
  ```
  **Expected:** Response includes a team with name "Personal", user is `owner`

- [ ] **Create a new team**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"Test Team","slug":"test-team"}' \
    $SERVER/api/teams
  ```
  **Expected:** `201 Created` with team object

- [ ] **Update team name**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"Renamed Team"}' \
    $SERVER/api/teams/TEAM_ID
  ```
  **Expected:** Updated team returned

- [ ] **Team appears in team switcher** — check dashboard UI
  **Expected:** Dropdown lists both Personal and new team

### 3.2 Invite Member via Email
- [ ] **Send invitation**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"email":"newuser@example.com","role":"developer"}' \
    $SERVER/api/teams/TEAM_ID/invitations
  ```
  **Expected:** `201 Created` with invitation token

- [ ] **Validate invitation token (public endpoint)**
  ```bash
  curl $SERVER/api/auth/invitations/INVITATION_TOKEN
  ```
  **Expected:** `200 OK` with team and inviter info

- [ ] **Accept invitation** (as the invited user)
  ```bash
  curl -X POST -H "Authorization: Bearer $OTHER_USER_TOKEN" \
    $SERVER/api/invitations/INVITATION_TOKEN/accept
  ```
  **Expected:** User added to team

- [ ] **Resend invitation**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/teams/TEAM_ID/invitations/INV_ID/resend
  ```
  **Expected:** `200 OK`

### 3.3 Role Management
- [ ] **Update member role**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"role":"admin"}' \
    $SERVER/api/teams/TEAM_ID/members/USER_ID
  ```
  **Expected:** Updated member returned

- [ ] **List members shows correct roles**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/teams/TEAM_ID/members
  ```
  **Expected:** Array with role field per member

### 3.4 Remove Member
- [ ] **Remove a member from team**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/teams/TEAM_ID/members/USER_ID
  ```
  **Expected:** `200 OK`, user no longer in member list

### 3.5 Audit Log Viewing
- [ ] **Team audit logs contain recent actions**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/teams/TEAM_ID/audit-logs
  ```
  **Expected:** Array of audit entries with `actor`, `action`, `resource_type`, `created_at`

### 3.6 2FA Enforcement (Owner Only)
- [ ] **Enable 2FA enforcement for team**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"enforce_2fa":true}' \
    $SERVER/api/teams/TEAM_ID/2fa-enforcement
  ```
  **Expected:** `200 OK`

- [ ] **Non-owner cannot toggle enforcement** — test with admin or developer token
  **Expected:** `403 Forbidden`

### 3.7 Team Switching
- [ ] Switch active team in dashboard UI
  **Expected:** Apps and projects shown update to reflect selected team

### 3.8 Delete Team
- [ ] **Delete a non-Personal team**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" $SERVER/api/teams/TEAM_ID
  ```
  **Expected:** `200 OK`, team no longer in list

---

## 4. Project Management

### 4.1 Create Project
- [ ] **Create project**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"Test Project","team_id":"TEAM_ID"}' \
    $SERVER/api/projects
  ```
  **Expected:** `201 Created` with project object

- [ ] **Update project**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"Renamed Project","description":"Updated desc"}' \
    $SERVER/api/projects/PROJECT_ID
  ```
  **Expected:** Updated project returned

### 4.2 Assign Environments
- [ ] **Create environment for project**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"production","slug":"production"}' \
    $SERVER/api/projects/PROJECT_ID/environments
  ```
  **Expected:** `201 Created`

- [ ] **Create staging and dev environments** — repeat above with `staging` and `development`
  **Expected:** Three environments listed

- [ ] **Add env vars to environment**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"key":"DATABASE_URL","value":"postgres://...","is_secret":true}' \
    $SERVER/api/environments/ENV_ID/env-vars
  ```
  **Expected:** `201 Created`

### 4.3 Project-Level Environment Variables
- [ ] **Create project env var (shared across apps)**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"key":"SHARED_VAR","value":"shared_value","is_secret":false}' \
    $SERVER/api/projects/PROJECT_ID/env-vars
  ```
  **Expected:** `201 Created`

- [ ] **List resolves app + project + team vars together**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/env-vars/resolved
  ```
  **Expected:** Merged list of all env var sources

### 4.4 Add Apps to Project
- [ ] **Assign app to project**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"project_id":"PROJECT_ID"}' \
    $SERVER/api/apps/APP_ID/project
  ```
  **Expected:** `200 OK`

### 4.5 Service Dependency Graph
- [ ] **Add dependency (app A depends on app B)**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"dependency_id":"APP_B_ID"}' \
    $SERVER/api/apps/APP_A_ID/dependencies
  ```
  **Expected:** `201 Created`

- [ ] **View dependency graph**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/projects/PROJECT_ID/dependency-graph
  ```
  **Expected:** JSON graph with nodes and edges

- [ ] **Remove dependency**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/apps/APP_A_ID/dependencies/DEP_ID
  ```
  **Expected:** `200 OK`

### 4.6 Delete Project
- [ ] **Delete project**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" $SERVER/api/projects/PROJECT_ID
  ```
  **Expected:** `200 OK`
  **Notes:** Deleting a project with apps may require removing apps first

---

## 5. App Deployment — Git

### 5.1 GitHub Repo with Dockerfile
- [ ] **Create and deploy app**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "docker-test",
      "git_url": "https://github.com/KwaminaWhyte/adamus-forms",
      "branch": "main",
      "port": 3000,
      "build_type": "dockerfile",
      "team_id": "TEAM_ID"
    }' \
    $SERVER/api/apps
  ```
  **Expected:** App created, deployment queued

- [ ] **Trigger deploy and confirm build runs**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/deploy
  ```
  **Expected:** Deployment object returned with `status: "queued"` or `"building"`

- [ ] **Container running after build**
  ```bash
  docker ps | grep docker-test
  ```
  **Expected:** Container listed as `Up`

### 5.2 GitHub Repo with Nixpacks Auto-detect
- [ ] **Create Nixpacks app**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "nixpacks-test",
      "git_url": "https://github.com/heroku/node-js-getting-started.git",
      "branch": "main",
      "port": 3000,
      "build_type": "nixpacks",
      "team_id": "TEAM_ID"
    }' \
    $SERVER/api/apps
  ```
  **Expected:** Build completes, app accessible on mapped port

- [ ] **PORT env var automatically set**
  ```bash
  docker exec rivetr-nixpacks-test env | grep PORT
  ```
  **Expected:** `PORT=3000`

### 5.3 GitLab Repo
- [ ] Create app with a GitLab git URL (public repo)
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "gitlab-test",
      "git_url": "https://gitlab.com/YOUR_ACCOUNT/YOUR_REPO.git",
      "branch": "main",
      "port": 8000,
      "build_type": "dockerfile"
    }' \
    $SERVER/api/apps
  ```
  **Expected:** Clones from GitLab, builds, and runs

### 5.4 Public Repo (No Auth Required)
- [ ] Create app from a fully public repo with no credentials configured
  **Expected:** Clone succeeds without authentication errors

---

## 6. App Deployment — Other Sources

### 6.1 ZIP Upload Deployment
- [ ] **Upload a ZIP and deploy**
  ```bash
  # Create a minimal Dockerfile app as ZIP
  mkdir /tmp/testapp && echo 'FROM nginx:alpine' > /tmp/testapp/Dockerfile
  cd /tmp && zip -r testapp.zip testapp/

  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -F "file=@/tmp/testapp.zip" \
    $SERVER/api/apps/APP_ID/deploy/upload
  ```
  **Expected:** Build type auto-detected, deployment starts

- [ ] **Build type detection endpoint**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -F "file=@/tmp/testapp.zip" \
    $SERVER/api/build/detect
  ```
  **Expected:** JSON with `build_type` field

### 6.2 Docker Image Deploy
- [ ] **Create app using existing Docker image**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "image-test",
      "image": "nginx:alpine",
      "port": 80,
      "build_type": "image"
    }' \
    $SERVER/api/apps
  ```
  **Expected:** Container starts from the specified image, no build phase

### 6.3 Deploy Specific Commit SHA
- [ ] **Trigger deploy at a specific SHA**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"commit_sha":"abc1234def5678"}' \
    $SERVER/api/apps/APP_ID/deploy
  ```
  **Expected:** Build clones the repo at that specific commit

- [ ] **List available commits**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/commits
  ```
  **Expected:** Array of commit objects with `sha` and `message`

### 6.4 Deploy Specific Git Tag
- [ ] **Trigger deploy at a specific tag**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"git_tag":"v1.0.0"}' \
    $SERVER/api/apps/APP_ID/deploy
  ```
  **Expected:** Build clones the repo at that tag

- [ ] **List available tags**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/tags
  ```
  **Expected:** Array of tag objects

---

## 7. Webhooks

### 7.1 GitHub Push Webhook
- [ ] **Webhook endpoint accepts GitHub push events**
  ```bash
  # Simulate a GitHub push event (replace WEBHOOK_SECRET with actual secret)
  curl -X POST \
    -H "X-GitHub-Event: push" \
    -H "X-Hub-Signature-256: sha256=HMAC_SIGNATURE" \
    -H "Content-Type: application/json" \
    -d '{"ref":"refs/heads/main","repository":{"clone_url":"https://github.com/OWNER/REPO.git"}}' \
    $SERVER/webhooks/github
  ```
  **Expected:** `200 OK`, deployment triggered for matching app

- [ ] **Push to connected GitHub repo triggers auto-deploy** — make a real push
  **Expected:** New deployment appears in app's deployment list within seconds

### 7.2 GitLab Push Webhook
- [ ] Configure GitLab webhook pointing to `$SERVER/webhooks/gitlab`
- [ ] Push to repository
  **Expected:** Deployment triggered

### 7.3 Gitea Push Webhook
- [ ] Configure Gitea webhook pointing to `$SERVER/webhooks/gitea`
- [ ] Push to repository
  **Expected:** Deployment triggered

### 7.4 DockerHub Webhook
- [ ] Configure DockerHub webhook pointing to `$SERVER/webhooks/dockerhub`
- [ ] Push a Docker image tag to DockerHub
  **Expected:** App deployment triggered for image-based app

### 7.5 Preview Deployment on PR Open
- [ ] **Open a Pull Request on a connected GitHub repo**
  **Expected:** Preview deployment created automatically, PR comment with preview URL added

- [ ] **View preview in dashboard**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/previews
  ```
  **Expected:** Preview entry with status and URL

### 7.6 Preview Cleanup on PR Close
- [ ] **Close or merge the PR**
  **Expected:** Preview deployment deleted, container removed

- [ ] **Verify preview gone**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/previews
  ```
  **Expected:** Closed PR's preview no longer in list

---

## 8. App Settings & Control

### 8.1 Start / Stop / Restart
- [ ] **Stop app**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/stop
  ```
  **Expected:** `200 OK`, `docker ps` no longer shows container

- [ ] **Start app**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/start
  ```
  **Expected:** `200 OK`, container running again

- [ ] **Restart app**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/restart
  ```
  **Expected:** `200 OK`, brief downtime then container up

### 8.2 Environment Variables (Plain + Secret)
- [ ] **Add plain env var**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"key":"APP_ENV","value":"production","is_secret":false}' \
    $SERVER/api/apps/APP_ID/env-vars
  ```
  **Expected:** `201 Created`

- [ ] **Add secret env var**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"key":"SECRET_KEY","value":"super_secret_value","is_secret":true}' \
    $SERVER/api/apps/APP_ID/env-vars
  ```
  **Expected:** `201 Created`, value masked in list response

- [ ] **Redeploy and verify vars in container**
  ```bash
  docker exec rivetr-APP_NAME env | grep APP_ENV
  ```
  **Expected:** `APP_ENV=production`

- [ ] **Update env var**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"value":"staging"}' \
    $SERVER/api/apps/APP_ID/env-vars/APP_ENV
  ```
  **Expected:** `200 OK`

- [ ] **Delete env var**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/apps/APP_ID/env-vars/APP_ENV
  ```
  **Expected:** `200 OK`

### 8.3 Shared Environment Variables
- [ ] **Create team-level shared env var**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"key":"TEAM_SHARED","value":"shared_val","is_secret":false}' \
    $SERVER/api/teams/TEAM_ID/env-vars
  ```
  **Expected:** `201 Created`

- [ ] **Create project-level shared env var** (see Section 4.3)

- [ ] **Resolved env vars merge all levels**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/env-vars/resolved
  ```
  **Expected:** App vars + project vars + team vars all present

### 8.4 Domain Management
- [ ] **Add custom domain**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"domain":"myapp.example.com"}' \
    $SERVER/api/routes
  ```
  **Expected:** `201 Created`, route registered

- [ ] **List routes**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/routes
  ```
  **Expected:** Domain appears with `app_id` association

- [ ] **Access app via custom domain** (after DNS configured)
  ```bash
  curl -H "Host: myapp.example.com" http://SERVER_IP/
  ```
  **Expected:** App response

### 8.5 Watch Paths
- [ ] **Configure watch paths on app** — set `watch_paths` to `["src/", "Dockerfile"]` in app settings
  **Expected:** Setting saved

- [ ] **Push change outside watch path** (e.g., only change `README.md`)
  **Expected:** Webhook received but no deployment triggered

- [ ] **Push change inside watch path** (e.g., edit a file in `src/`)
  **Expected:** Deployment triggered

### 8.6 Maintenance Mode
- [ ] **Enable maintenance mode** — toggle in app settings UI
  **Expected:** App URL returns maintenance page instead of app

- [ ] **Custom maintenance page** — if configured, verify custom message displayed

- [ ] **Disable maintenance mode**
  **Expected:** App serves normally again

### 8.7 App Cloning
- [ ] **Clone an app** — use "Clone App" option in dashboard UI
  **Expected:** New app created with same settings, git URL, env vars

### 8.8 Config Snapshot (Save / Restore)
- [ ] **Save a config snapshot** — use snapshot option in app settings
  **Expected:** Snapshot stored with timestamp

- [ ] **Modify app config** (change port or build type)
  **Expected:** Change visible in app settings

- [ ] **Restore to snapshot**
  **Expected:** Config reverts to saved values

### 8.9 HTTP Basic Auth
- [ ] **Enable basic auth for app**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"username":"user","password":"pass","enabled":true}' \
    $SERVER/api/apps/APP_ID/basic-auth
  ```
  **Expected:** `200 OK`

- [ ] **Access app in browser — auth prompt appears**
  **Expected:** Browser shows HTTP Basic Auth dialog

- [ ] **Correct credentials grant access**
  ```bash
  curl -u user:pass http://SERVER_IP:APP_PORT/
  ```
  **Expected:** App response

- [ ] **Disable basic auth**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/basic-auth
  ```
  **Expected:** `200 OK`, no auth prompt

---

## 9. Deployment Management

### 9.1 View Deployment History
- [ ] **List deployments with pagination**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" \
    "$SERVER/api/apps/APP_ID/deployments?page=1&per_page=10"
  ```
  **Expected:** `{ "items": [...], "total": N, "page": 1, "per_page": 10, "total_pages": M }`

- [ ] **Get single deployment**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/deployments/DEPLOYMENT_ID
  ```
  **Expected:** Full deployment object with status, logs reference

### 9.2 Rollback to Previous Deployment
- [ ] **Trigger rollback**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/deployments/DEPLOYMENT_ID/rollback
  ```
  **Expected:** Previous container image started, status switches to `running`

### 9.3 Approval Workflow
- [ ] **Set app to require deployment approval** — update app with `require_approval: true`
  **Expected:** Next deploy goes into `pending_approval` state

- [ ] **Trigger a new deploy**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/deploy
  ```
  **Expected:** Deployment created with `status: "pending_approval"`

- [ ] **List pending deployments**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/deployments/pending
  ```
  **Expected:** Pending deployment appears

- [ ] **Approve deployment**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/deployments/DEPLOYMENT_ID/approve
  ```
  **Expected:** Deployment proceeds to build

- [ ] **Reject deployment**
  ```bash
  # Trigger another deploy first, then reject it
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/deployments/DEPLOYMENT_ID/reject
  ```
  **Expected:** Deployment status set to `rejected`, no build occurs

### 9.4 Freeze Window
- [ ] **Create freeze window** (block deploys during a time range)
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "Holiday Freeze",
      "start_time": "2026-12-24T00:00:00Z",
      "end_time": "2026-12-26T23:59:59Z",
      "reason": "No deploys during holidays"
    }' \
    $SERVER/api/freeze-windows
  ```
  **Expected:** `201 Created`

- [ ] **Attempt deploy during active freeze window**
  **Expected:** Deploy blocked with an error indicating freeze window is active

- [ ] **Delete freeze window**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" $SERVER/api/freeze-windows/WINDOW_ID
  ```
  **Expected:** `200 OK`, deploys allowed again

### 9.5 Scheduled Deployment
- [ ] **Schedule a future deployment**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"scheduled_at":"2026-03-11T02:00:00Z"}' \
    $SERVER/api/apps/APP_ID/deploy
  ```
  **Expected:** Deployment created with `scheduled_at` timestamp and `status: "scheduled"`

- [ ] **Verify deployment runs at scheduled time**
  **Expected:** Shortly after the scheduled time, deployment status changes to `building`

### 9.6 Deployment Retention
- [ ] **Configure rollback retention count** — set `rollback_retention_count` in app settings (e.g. 5)
  **Expected:** Only last 5 completed deployments kept, older ones cleaned up

### 9.7 Zero-Downtime Deploy
- [ ] **Deploy while app is receiving traffic**
  **Expected:** During deployment, old container continues serving until new one is healthy, then swap occurs; no connection errors observed

### 9.8 Deployment Diff
- [ ] **View diff between two deployments**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/deployments/DEPLOYMENT_ID/diff
  ```
  **Expected:** Response contains changed files or commit range between current and previous deployment

---

## 10. Container Replicas & Auto-scaling

### 10.1 Set Replica Count
- [ ] **Scale to 3 replicas**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"count":3}' \
    $SERVER/api/apps/APP_ID/replicas/count
  ```
  **Expected:** `200 OK`

- [ ] **List replicas**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/replicas
  ```
  **Expected:** 3 replica entries each with `container_id` and `status`

### 10.2 Load Balancing
- [ ] **Make repeated requests and verify different replicas serve them**
  ```bash
  for i in $(seq 1 10); do
    curl -s http://SERVER_IP:APP_PORT/health
  done
  ```
  **Expected:** Requests distributed across replicas (verify via container logs)
  ```bash
  docker logs rivetr-APP_NAME-0 2>&1 | tail -5
  docker logs rivetr-APP_NAME-1 2>&1 | tail -5
  docker logs rivetr-APP_NAME-2 2>&1 | tail -5
  ```

### 10.3 Restart Specific Replica
- [ ] **Restart replica at index 1**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/apps/APP_ID/replicas/1/restart
  ```
  **Expected:** `200 OK`, that replica restarts, others unaffected

### 10.4 Auto-scaling Rule
- [ ] **Create CPU-based auto-scaling rule**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "metric": "cpu",
      "threshold": 70,
      "scale_up_by": 1,
      "scale_down_by": 1,
      "min_replicas": 1,
      "max_replicas": 5,
      "cooldown_seconds": 120
    }' \
    $SERVER/api/apps/APP_ID/autoscaling
  ```
  **Expected:** `201 Created` with rule object

- [ ] **List auto-scaling rules**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/autoscaling
  ```
  **Expected:** Rule listed

- [ ] **Update rule**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"threshold":80}' \
    $SERVER/api/apps/APP_ID/autoscaling/RULE_ID
  ```
  **Expected:** `200 OK`

- [ ] **Delete rule**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/apps/APP_ID/autoscaling/RULE_ID
  ```
  **Expected:** `200 OK`

---

## 11. Container Terminal & Logs

### 11.1 Container Terminal
- [ ] **Open terminal via dashboard** — navigate to App > Terminal
  **Expected:** WebSocket connects to `/api/apps/APP_ID/terminal`, shell prompt appears

- [ ] **Run commands in terminal**
  - `ls /app` — directory listing
  - `echo $PORT` — environment variable
  - `cat /etc/os-release` — OS info
  **Expected:** Each command outputs results in terminal

### 11.2 Live Log Streaming
- [ ] **View live logs in dashboard** — open App > Logs
  **Expected:** Logs stream in real-time via SSE at `/api/apps/APP_ID/logs/stream`

- [ ] **Build log streaming during deploy**
  **Expected:** Browser DevTools > Network shows WS connection to `/api/deployments/ID/logs/stream`, logs appear line-by-line during build

### 11.3 Log Search
- [ ] **Search logs by keyword**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" \
    "$SERVER/api/apps/APP_ID/logs/search?q=error&limit=50"
  ```
  **Expected:** Matching log lines returned

### 11.4 Log Retention Policy
- [ ] **Set log retention**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"max_lines":10000,"max_age_days":30}' \
    $SERVER/api/apps/APP_ID/log-retention
  ```
  **Expected:** `200 OK`

- [ ] **Get log retention settings**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/log-retention
  ```
  **Expected:** Returns the configured policy

---

## 12. Managed Databases

### 12.1 Create PostgreSQL Database
- [ ] **Create database**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "test-postgres",
      "db_type": "postgres",
      "version": "16",
      "username": "testuser",
      "password": "testpass123",
      "database": "testdb",
      "team_id": "TEAM_ID"
    }' \
    $SERVER/api/databases
  ```
  **Expected:** `201 Created`, database container starts

### 12.2 Create MySQL Database
- [ ] **Create MySQL database**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "test-mysql",
      "db_type": "mysql",
      "version": "8.0",
      "username": "mysqluser",
      "password": "mysqlpass123",
      "database": "testdb",
      "team_id": "TEAM_ID"
    }' \
    $SERVER/api/databases
  ```
  **Expected:** `201 Created`, MySQL container starts

### 12.3 Create Redis Database
- [ ] **Create Redis**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "test-redis",
      "db_type": "redis",
      "version": "7",
      "team_id": "TEAM_ID"
    }' \
    $SERVER/api/databases
  ```
  **Expected:** `201 Created`, Redis container starts

### 12.4 View Connection Credentials
- [ ] **Get database details**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/databases/DB_ID
  ```
  **Expected:** Response includes `connection_string`, `host`, `port`, `username`

### 12.5 Connect to Database from Terminal
- [ ] **Connect via app terminal or SSH to server**
  ```bash
  # For PostgreSQL
  docker exec -it rivetr-test-postgres psql -U testuser -d testdb -c "SELECT 1;"
  # For Redis
  docker exec -it rivetr-test-redis redis-cli ping
  ```
  **Expected:** Connection successful, query or PONG returned

### 12.6 Database Backup (Manual)
- [ ] **Create manual backup**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/databases/DB_ID/backups
  ```
  **Expected:** `201 Created` with backup object

- [ ] **Download backup**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" \
    -o /tmp/backup.sql \
    $SERVER/api/databases/DB_ID/backups/BACKUP_ID/download
  ```
  **Expected:** File downloaded, non-empty

### 12.7 Backup Schedule
- [ ] **Create backup schedule**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"cron_expression":"0 2 * * *","retention_count":7}' \
    $SERVER/api/databases/DB_ID/backups/schedule
  ```
  **Expected:** `201 Created`

- [ ] **Get backup schedule**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/databases/DB_ID/backups/schedule
  ```
  **Expected:** Schedule object returned

### 12.8 Stop / Start Database
- [ ] **Stop database**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/databases/DB_ID/stop
  ```
  **Expected:** `200 OK`, container stopped

- [ ] **Start database**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/databases/DB_ID/start
  ```
  **Expected:** `200 OK`, container running

---

## 13. Docker Compose Services

### 13.1 Deploy from Template
- [ ] **List available templates**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/templates
  ```
  **Expected:** Array of template objects (should be 74 templates)

- [ ] **Deploy Grafana template**
  ```bash
  # Get the template ID for Grafana
  TEMPLATE_ID=$(curl -s -H "Authorization: Bearer $TOKEN" $SERVER/api/templates \
    | python3 -c "import sys,json; t=json.load(sys.stdin); print([x['id'] for x in t if 'grafana' in x['name'].lower()][0])")

  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"my-grafana","team_id":"TEAM_ID"}' \
    $SERVER/api/templates/$TEMPLATE_ID/deploy
  ```
  **Expected:** Compose service created, containers starting

### 13.2 Create Custom Compose Service
- [ ] **Create service from custom compose YAML**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "custom-service",
      "compose_content": "services:\n  web:\n    image: nginx:alpine\n    ports:\n      - \"8081:80\"",
      "team_id": "TEAM_ID"
    }' \
    $SERVER/api/services
  ```
  **Expected:** `201 Created`, service defined

### 13.3 Start / Stop Compose Service
- [ ] **Start service**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/services/SERVICE_ID/start
  ```
  **Expected:** `200 OK`, all containers in compose up

- [ ] **Stop service**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/services/SERVICE_ID/stop
  ```
  **Expected:** `200 OK`, containers stopped

### 13.4 View Compose Logs
- [ ] **Get logs**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/services/SERVICE_ID/logs
  ```
  **Expected:** Log lines from service containers

- [ ] **Stream service logs**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" \
    -N $SERVER/api/services/SERVICE_ID/logs/stream
  ```
  **Expected:** SSE stream of live log events

### 13.5 Delete Compose Service
- [ ] **Delete service**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" $SERVER/api/services/SERVICE_ID
  ```
  **Expected:** `200 OK`, containers and volumes cleaned up

---

## 14. Service Templates

### 14.1 Browse Templates
- [ ] **Browse all 74 templates**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/templates | python3 -c \
    "import sys,json; t=json.load(sys.stdin); print(f'Total: {len(t)}')"
  ```
  **Expected:** Count should be 74

- [ ] **List categories**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/templates/categories
  ```
  **Expected:** Array of category strings

### 14.2 Search Templates
- [ ] Search in dashboard UI using the search box
  **Expected:** Filtered results update as you type

### 14.3 Filter by Category
- [ ] Select a category filter in dashboard UI
  **Expected:** Only templates in that category shown

### 14.4 Deploy a Template (Plausible Analytics)
- [ ] Find Plausible template ID and deploy it
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"my-plausible","team_id":"TEAM_ID"}' \
    $SERVER/api/templates/PLAUSIBLE_TEMPLATE_ID/deploy
  ```
  **Expected:** `201 Created`, service containers starting

### 14.5 Suggest a Template
- [ ] **Submit a template suggestion**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "My Custom App",
      "description": "A template suggestion for testing",
      "category": "databases",
      "docker_compose": "services:\n  app:\n    image: myapp:latest"
    }' \
    $SERVER/api/templates/suggest
  ```
  **Expected:** `201 Created`

- [ ] **List suggestions (admin)**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/templates/suggestions
  ```
  **Expected:** Suggestion listed with `status: "pending"`

---

## 15. Bulk Operations

### 15.1 Bulk Start
- [ ] **Select multiple apps and start them all**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"app_ids":["APP_ID_1","APP_ID_2"],"action":"start"}' \
    $SERVER/api/apps/bulk
  ```
  **Expected:** All specified apps start

### 15.2 Bulk Stop
- [ ] **Bulk stop**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"app_ids":["APP_ID_1","APP_ID_2"],"action":"stop"}' \
    $SERVER/api/apps/bulk
  ```
  **Expected:** All specified apps stop

### 15.3 Bulk Restart
- [ ] **Bulk restart**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"app_ids":["APP_ID_1","APP_ID_2"],"action":"restart"}' \
    $SERVER/api/apps/bulk
  ```
  **Expected:** All specified apps restart

### 15.4 Bulk Deploy
- [ ] **Bulk deploy**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"app_ids":["APP_ID_1","APP_ID_2"],"action":"deploy"}' \
    $SERVER/api/apps/bulk
  ```
  **Expected:** Deployments queued for all specified apps

---

## 16. S3 & Backups

### 16.1 Configure S3 Storage
- [ ] **Add S3 config (MinIO or AWS)**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "my-s3",
      "endpoint": "https://play.min.io",
      "access_key": "YOUR_ACCESS_KEY",
      "secret_key": "YOUR_SECRET_KEY",
      "bucket": "rivetr-backups",
      "region": "us-east-1"
    }' \
    $SERVER/api/s3/configs
  ```
  **Expected:** `201 Created`

- [ ] **Test S3 config connectivity**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/s3/configs/CONFIG_ID/test
  ```
  **Expected:** `200 OK` with success message

### 16.2 Database Backup to S3
- [ ] **Trigger S3 backup for a database backup**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"s3_config_id":"S3_CONFIG_ID","source":"database","source_id":"DB_ID"}' \
    $SERVER/api/s3/backup
  ```
  **Expected:** `202 Accepted`, backup upload starts

### 16.3 Volume Backup to S3
- [ ] **Backup volume**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/volumes/VOLUME_ID/backup
  ```
  **Expected:** `200 OK` with backup file details

### 16.4 Create Instance Backup
- [ ] **Create full instance backup**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/system/backup
  ```
  **Expected:** Backup file created, response includes file name

- [ ] **List instance backups**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/system/backups
  ```
  **Expected:** Array of backup entries with name and size

### 16.5 Upload Instance Backup to S3
- [ ] **Upload backup to S3**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"s3_config_id":"S3_CONFIG_ID"}' \
    $SERVER/api/system/backups/BACKUP_NAME/upload-to-s3
  ```
  **Expected:** `200 OK`

### 16.6 Create Backup Schedule
- [ ] **Schedule recurring backups**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "Nightly Backup",
      "cron_expression": "0 3 * * *",
      "s3_config_id": "S3_CONFIG_ID",
      "retention_count": 14
    }' \
    $SERVER/api/backups/schedules
  ```
  **Expected:** `201 Created`

- [ ] **List backup schedules**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/backups/schedules
  ```
  **Expected:** Schedule listed

- [ ] **Toggle schedule on/off**
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" $SERVER/api/backups/schedules/SCHEDULE_ID/toggle
  ```
  **Expected:** `200 OK`, `enabled` flag toggled

### 16.7 Restore from Backup
- [ ] **Restore instance from backup**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"backup_name":"BACKUP_NAME"}' \
    $SERVER/api/system/restore
  ```
  **Expected:** `200 OK`
  **Notes:** Exercise caution on a live server; test on a staging instance

---

## 17. Notifications & Alerts

### 17.1 Configure Slack Notification Channel
- [ ] **Create Slack channel**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "Slack Alerts",
      "type": "slack",
      "config": {"webhook_url": "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"}
    }' \
    $SERVER/api/notification-channels
  ```
  **Expected:** `201 Created`

- [ ] **Test Slack channel**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/notification-channels/CHANNEL_ID/test
  ```
  **Expected:** `200 OK`, test message appears in Slack

### 17.2 Configure Email Notification
- [ ] **Create email channel** (requires SMTP configured)
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "Email Alerts",
      "type": "email",
      "config": {"to": "alerts@example.com"}
    }' \
    $SERVER/api/notification-channels
  ```
  **Expected:** `201 Created`

### 17.3 Create Alert Rule (CPU Threshold)
- [ ] **Create CPU alert**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "metric": "cpu",
      "threshold": 80,
      "comparison": "gt",
      "duration_seconds": 60,
      "notification_channel_id": "CHANNEL_ID"
    }' \
    $SERVER/api/apps/APP_ID/alerts
  ```
  **Expected:** `201 Created`

- [ ] **Create memory alert**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "metric": "memory",
      "threshold": 90,
      "comparison": "gt",
      "duration_seconds": 120,
      "notification_channel_id": "CHANNEL_ID"
    }' \
    $SERVER/api/apps/APP_ID/alerts
  ```
  **Expected:** `201 Created`

### 17.4 Trigger Alert
- [ ] **Stress the container to trigger CPU alert**
  ```bash
  # Run stress inside container
  docker exec rivetr-APP_NAME sh -c "apt-get install -y stress 2>/dev/null; stress --cpu 4 --timeout 90s &"
  ```
  **Expected:** Alert fires within configured duration, notification received on channel

- [ ] **Verify alert event logged**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/alert-events
  ```
  **Expected:** Alert event entry with timestamp and metric values

### 17.5 Log Drain (Axiom)
- [ ] **Create Axiom log drain**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "Axiom Drain",
      "type": "axiom",
      "config": {
        "dataset": "rivetr-logs",
        "api_token": "YOUR_AXIOM_TOKEN"
      }
    }' \
    $SERVER/api/apps/APP_ID/log-drains
  ```
  **Expected:** `201 Created`

- [ ] **Test log drain**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/apps/APP_ID/log-drains/DRAIN_ID/test
  ```
  **Expected:** `200 OK`, test event appears in Axiom dataset

- [ ] **Generate app logs and verify forwarding**
  **Expected:** Logs appear in Axiom within a few seconds

---

## 18. Multi-Server

### 18.1 Register Remote Server
- [ ] **Add remote server**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "Remote Server 1",
      "host": "REMOTE_IP",
      "port": 22,
      "username": "root",
      "private_key": "-----BEGIN OPENSSH PRIVATE KEY-----\n..."
    }' \
    $SERVER/api/servers
  ```
  **Expected:** `201 Created`

### 18.2 Health Check Server
- [ ] **Check server health (CPU/memory/disk)**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/servers/SERVER_ID/check
  ```
  **Expected:** `200 OK` with CPU %, memory %, disk % stats

### 18.3 Remote Server Terminal
- [ ] **Open SSH terminal in browser** — navigate to Servers > SERVER_NAME > Terminal
  **Expected:** WebSocket connects to `/api/servers/SERVER_ID/terminal`, remote shell prompt appears

### 18.4 Assign App to Server
- [ ] **Assign app to remote server**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/servers/SERVER_ID/apps/APP_ID
  ```
  **Expected:** `200 OK`

- [ ] **Verify assignment**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/servers/SERVER_ID/apps
  ```
  **Expected:** App listed

- [ ] **Unassign app from server**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/servers/SERVER_ID/apps/APP_ID
  ```
  **Expected:** `200 OK`

---

## 19. Docker Swarm

### 19.1 Initialize Swarm
- [ ] **Initialize Docker Swarm on the server**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"advertise_addr":"SERVER_IP"}' \
    $SERVER/api/swarm/init
  ```
  **Expected:** `200 OK` with swarm join token

### 19.2 Check Swarm Status
- [ ] **Get swarm status**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/swarm/status
  ```
  **Expected:** `{ "active": true, "node_id": "...", "role": "manager" }`

- [ ] **List swarm nodes**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/swarm/nodes
  ```
  **Expected:** Array with at least one manager node

### 19.3 Create Swarm Service
- [ ] **Create a service in the swarm**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "swarm-nginx",
      "image": "nginx:alpine",
      "replicas": 1,
      "port": 8082
    }' \
    $SERVER/api/swarm/services
  ```
  **Expected:** `201 Created`

### 19.4 Scale Swarm Service
- [ ] **Scale to 3 replicas**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"replicas":3}' \
    $SERVER/api/swarm/services/SERVICE_ID/scale
  ```
  **Expected:** `200 OK`

### 19.5 Remove Swarm Service
- [ ] **Delete service**
  ```bash
  curl -X DELETE -H "Authorization: Bearer $TOKEN" $SERVER/api/swarm/services/SERVICE_ID
  ```
  **Expected:** `200 OK`

### 19.6 Leave Swarm
- [ ] **Leave swarm (and disable Swarm mode)**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"force":true}' \
    $SERVER/api/swarm/leave
  ```
  **Expected:** `200 OK`

---

## 20. Build Servers

### 20.1 Register Build Server
- [ ] **Add a build server**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "Build Server 1",
      "host": "BUILD_SERVER_IP",
      "port": 22,
      "username": "root",
      "private_key": "-----BEGIN OPENSSH PRIVATE KEY-----\n...",
      "max_concurrent_builds": 4
    }' \
    $SERVER/api/build-servers
  ```
  **Expected:** `201 Created`

### 20.2 Health Check Build Server
- [ ] **Check build server health**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/build-servers/BUILD_SERVER_ID/check
  ```
  **Expected:** `200 OK` with status and capacity info

### 20.3 View Active Builds
- [ ] **List build servers and their current load**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/build-servers
  ```
  **Expected:** Build server listed, shows concurrent build count

---

## 21. Scheduled Jobs

### 21.1 Create Cron Job
- [ ] **Create a scheduled job**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "hourly-hello",
      "command": "echo hello from cron",
      "schedule": "0 * * * *",
      "enabled": true
    }' \
    $SERVER/api/apps/APP_ID/jobs
  ```
  **Expected:** `201 Created` with job object

- [ ] **Manually trigger job run**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/apps/APP_ID/jobs/JOB_ID/run
  ```
  **Expected:** `200 OK`, job run queued immediately

### 21.2 View Job Execution History
- [ ] **List job runs**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" \
    $SERVER/api/apps/APP_ID/jobs/JOB_ID/runs
  ```
  **Expected:** Array of run records with `status`, `started_at`, `finished_at`, `output`

### 21.3 Verify Job Runs on Schedule
- [ ] Set schedule to `* * * * *` (every minute), wait 2 minutes
  ```bash
  curl -X PUT -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"schedule":"* * * * *"}' \
    $SERVER/api/apps/APP_ID/jobs/JOB_ID
  # Wait ~2 minutes, then:
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/apps/APP_ID/jobs/JOB_ID/runs
  ```
  **Expected:** At least one successful run recorded after the minute mark

---

## 22. System

### 22.1 System Stats
- [ ] **Current stats**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/system/stats
  ```
  **Expected:** JSON with `cpu_percent`, `memory_used`, `memory_total`, `uptime`

- [ ] **Stats history (last 24 hours)**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" "$SERVER/api/system/stats/history?hours=24"
  ```
  **Expected:** Array of timestamped stat snapshots

- [ ] **Disk stats**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/system/disk
  ```
  **Expected:** JSON with `total`, `used`, `available` in bytes

### 22.2 System Health
- [ ] **Detailed health check**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/system/health
  ```
  **Expected:** All checks pass: `database`, `runtime`, `disk`

### 22.3 Prometheus Metrics
- [ ] **Metrics endpoint**
  ```bash
  curl $SERVER/metrics
  ```
  **Expected:** Prometheus format text with `rivetr_` prefixed metrics, HTTP request counts

### 22.4 Webhook Events Log
- [ ] **View webhook events**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/webhook-events
  ```
  **Expected:** Array of recent webhook events with source, payload hash, and timestamp

### 22.5 Auto-Update Settings
- [ ] **Check current version**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/system/version
  ```
  **Expected:** JSON with `current_version`, `latest_version`, `update_available`

- [ ] **Check for updates**
  ```bash
  curl -X POST -H "Authorization: Bearer $TOKEN" $SERVER/api/system/update/check
  ```
  **Expected:** `200 OK` with version comparison result

### 22.6 System Backup and Restore
- [ ] Create instance backup (see Section 16.4)
- [ ] Download backup
  ```bash
  curl -H "Authorization: Bearer $TOKEN" \
    -o /tmp/rivetr-backup.tar.gz \
    $SERVER/api/system/backups/BACKUP_NAME/download
  ```
  **Expected:** Archive downloaded, non-empty
- [ ] Restore from backup (see Section 16.7)

### 22.7 Audit Log (Global)
- [ ] **List all audit log entries**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/audit
  ```
  **Expected:** Array of audit events across all resources

- [ ] **List available action types**
  ```bash
  curl -H "Authorization: Bearer $TOKEN" $SERVER/api/audit/actions
  ```
  **Expected:** Array of action type strings

---

## 23. Security

### 23.1 Rate Limiting
- [ ] **Auth endpoint rate limit (20 req/min)**
  ```bash
  for i in $(seq 1 25); do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
      -H "Content-Type: application/json" \
      -d '{"email":"x","password":"y"}' \
      $SERVER/api/auth/login)
    echo "Request $i: $STATUS"
  done
  ```
  **Expected:** Requests 1–20 return `401`, request 21+ return `429 Too Many Requests`

- [ ] **API endpoint rate limit**
  ```bash
  for i in $(seq 1 200); do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
      -H "Authorization: Bearer $TOKEN" $SERVER/api/apps)
    [ "$STATUS" = "429" ] && echo "429 hit at request $i" && break
  done
  ```
  **Expected:** `429` returned when API rate limit exceeded

### 23.2 Invalid Token Returns 401
- [ ] **Verify any malformed or expired token is rejected**
  ```bash
  curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer thisisnotavalidtoken" \
    $SERVER/api/apps
  ```
  **Expected:** `401`

### 23.3 Role-Based Access Control
- [ ] **Viewer role cannot trigger deploy**
  ```bash
  # Use token of a viewer-role user
  curl -s -o /dev/null -w "%{http_code}" -X POST \
    -H "Authorization: Bearer $VIEWER_TOKEN" \
    $SERVER/api/apps/APP_ID/deploy
  ```
  **Expected:** `403 Forbidden`

- [ ] **Developer role cannot manage team members**
  ```bash
  curl -s -o /dev/null -w "%{http_code}" -X POST \
    -H "Authorization: Bearer $DEVELOPER_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"email":"someone@example.com","role":"developer"}' \
    $SERVER/api/teams/TEAM_ID/invitations
  ```
  **Expected:** `403 Forbidden`

### 23.4 2FA Blocks Login Without TOTP
- [ ] Enable 2FA on an account (see Section 2.5)
- [ ] Log out, then attempt login with correct email/password but no TOTP
  ```bash
  RESULT=$(curl -s -X POST -H "Content-Type: application/json" \
    -d '{"email":"admin@example.com","password":"YourPassword"}' \
    $SERVER/api/auth/login)
  echo $RESULT
  ```
  **Expected:** Response contains `"requires_2fa": true` and a temporary token, NOT a full session token

- [ ] **Completing 2FA unlocks the session**
  ```bash
  curl -X POST -H "Content-Type: application/json" \
    -d '{"temp_token":"TEMP_TOKEN_FROM_LOGIN","code":"123456"}' \
    $SERVER/api/auth/2fa/validate
  ```
  **Expected:** `200 OK` with full session token

### 23.5 Security Headers
- [ ] **Verify security headers on responses**
  ```bash
  curl -I $SERVER/health
  ```
  **Expected:** Response includes:
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY`
  - `X-XSS-Protection: 1; mode=block`
  - `Referrer-Policy: strict-origin-when-cross-origin`

---

## Appendix: Quick Troubleshooting

### Service won't start
```bash
journalctl -u rivetr -f
systemctl status rivetr
```

### Container issues
```bash
docker ps -a
docker logs rivetr-APP_NAME 2>&1 | tail -50
docker inspect rivetr-APP_NAME
```

### Database issues
```bash
sqlite3 /var/lib/rivetr/rivetr.db ".tables"
sqlite3 /var/lib/rivetr/rivetr.db "SELECT name, status FROM apps;"
```

### Network issues
```bash
ss -tlnp | grep -E "(80|443|8080)"
curl -v localhost:8080/health
```

### Reset token from config
```bash
TOKEN=$(grep admin_token /opt/rivetr/rivetr.toml | cut -d'"' -f2)
echo $TOKEN
```

### Known fixed issues (from v0.2.x testing)
| Issue | Fixed In |
|-------|----------|
| `byte index 8 is out of bounds` panic in teams.rs when user.id short | v0.2.13 |
| Stats history chart 401 — wrong localStorage key | v0.2.12 |
| Container monitor: `no column found for name: team_id` | v0.2.14 |
| Notification channels: CHECK constraint missing 'webhook' type | v0.2.14 |
| Auto-update page missing route registration | v0.2.13 |
| Migration 038 needs `PRAGMA foreign_keys=OFF` for table recreation | v0.2.14 |

### Known fixed issues (v0.10.20 — Parallels VM sweep)
| Issue | Fixed In |
|-------|----------|
| `/api/apps/:id/insights` returned 503 polluting tower_http logs | v0.10.20 (B3) |
| Container monitor `check_services` SELECT missing migration-105 columns — service crash detection silently broken | v0.10.20 (B4) |
| MySQL/MariaDB user provisioning warning even though entrypoint succeeded | v0.10.20 (B5) |
| Audit log missing event types (token, deployment.cancel, env_var.*, app.update, etc.) | v0.10.20 (B7) |
| `?limit=N` query alias on `/api/audit` ignored | v0.10.20 (B9) |
| POST/DELETE returning 415 without `Content-Type: application/json` even when no body needed | v0.10.20 (B10) |
| Cancel deployment in non-cancellable state returned 404 instead of 409 | v0.10.20 (B11) |
| Templates list endpoint 500KB no-gzip with no-store cache | v0.10.20 (B17) — gzip + cache-control + slim list |
| Database SQL backup downloaded as `application/octet-stream` | v0.10.20 (B26) — now `application/sql` |
| Compose service has null domain when `instance_domain` unset | v0.10.20 (B27) — falls back to `<name>.local` |
| App `internal_hostname` derived field for stable network alias | v0.10.20 (B14/B15) |
| Docker network "endpoint already exists" 403 warning every restart | v0.10.20 (B28) |

### Open in v0.10.20 (move to v0.10.21 backlog)
- B6 — MySQL 8 default SSL breaks published connection string
- B8 — Audit `ip_address` extractor staged but unwired into handlers
- B12/B13 — Rollback flow needs live multi-deploy validation (code merged)
- B20 — Disk usage path inconsistency (Dashboard vs Monitoring page)
- B25 — DB-to-app env-var auto-injection UI
- 8 frontend fixes need browser-driven validation pass

### Validation reports
- `live-testing/VM-SWEEP-2026-05-02.md` — original 27-bug sweep + post-fix status table
- `live-testing/VM-VALIDATION-2026-05-02.md` — MariaDB + side panel browser session
- `live-testing/VM-VALIDATION-2026-05-02-backend.md` — curl-based API validation (13/15 PASS)
- `live-testing/VM-VALIDATION-2026-05-02-frontend.md` — static-bundle frontend validation (4/12 confirmed)
