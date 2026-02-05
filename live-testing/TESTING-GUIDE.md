# Rivetr Comprehensive Testing Guide

This document provides actionable testing tasks for every module/feature of the Rivetr deployment platform.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation Testing](#installation-testing)
3. [Authentication & Authorization](#authentication--authorization)
4. [Team Management](#team-management)
5. [Project Management](#project-management)
6. [App Deployment](#app-deployment)
7. [Environment Variables](#environment-variables)
8. [Managed Databases](#managed-databases)
9. [Docker Compose Services](#docker-compose-services)
10. [Volumes & Persistence](#volumes--persistence)
11. [Alerting & Notifications](#alerting--notifications)
12. [Preview Deployments](#preview-deployments)
13. [System Monitoring](#system-monitoring)
14. [Proxy & Routing](#proxy--routing)
15. [Security Features](#security-features)
16. [WebSocket Features](#websocket-features)
17. [API Endpoints](#api-endpoints)

---

## Prerequisites

### Test Server Requirements
- Fresh Ubuntu 22.04/24.04 or Debian 12 server
- Minimum 2GB RAM (1GB for minimal testing)
- 20GB disk space
- Root/sudo access
- Ports 80, 443, 8080 accessible

### Test Accounts
- GitHub account with test repos
- Email account for notifications (optional)

### Test Repositories
- `https://github.com/heroku/node-js-getting-started.git` (Node.js, Nixpacks)
- `https://github.com/KwaminaWhyte/adamus-forms` (Docker)
- Any repo with `Dockerfile` in root

---

## Installation Testing

### Task 1.1: Fresh Installation
```bash
# Clean state verification
systemctl stop rivetr 2>/dev/null
rm -rf /opt/rivetr /var/lib/rivetr /etc/systemd/system/rivetr.service
userdel rivetr 2>/dev/null

# Run installation
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

**Verify:**
- [ ] Docker installed and running
- [ ] Nixpacks installed (`nixpacks --version`)
- [ ] Pack CLI installed (`pack version`)
- [ ] Rivetr binary at `/opt/rivetr/rivetr`
- [ ] Config at `/opt/rivetr/rivetr.toml`
- [ ] Service running (`systemctl status rivetr`)
- [ ] Dashboard accessible at `http://SERVER_IP:8080`

**Issues Found:**
- (Document any issues here)

### Task 1.2: Verify Binary Download (Not Source Build)
```bash
# Check install log or re-run with verbose
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh > /tmp/install.sh
bash -x /tmp/install.sh 2>&1 | tee /tmp/install-verbose.log

# Look for "Downloaded binary from GitHub releases" message
grep -i "download" /tmp/install-verbose.log
```

**Verify:**
- [ ] Binary downloaded from GitHub releases (not built from source)
- [ ] Source build only used as fallback

### Task 1.3: Upgrade/Re-installation
```bash
# Simulate upgrade
export RIVETR_VERSION=latest
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

**Verify:**
- [ ] Existing config preserved
- [ ] Existing data preserved
- [ ] Service restarts with new binary

---

## Authentication & Authorization

### Task 2.1: Initial Setup
1. Navigate to `http://SERVER_IP:8080`
2. Create first admin account

**Verify:**
- [ ] Registration form validates password strength
- [ ] Account created successfully
- [ ] Redirected to dashboard after setup

**Issues Found:**
- (Document any issues here)

### Task 2.2: Login/Logout
1. Logout from dashboard
2. Login with created credentials
3. Test "Remember me" functionality

**Verify:**
- [ ] Login works with correct credentials
- [ ] Login fails with wrong credentials
- [ ] Session persists on page refresh
- [ ] Logout clears session

### Task 2.3: API Token Authentication
```bash
# Get token from /opt/rivetr/rivetr.toml
TOKEN=$(grep admin_token /opt/rivetr/rivetr.toml | cut -d'"' -f2)

# Test API auth
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/apps
```

**Verify:**
- [ ] Token auth works for API endpoints
- [ ] Invalid token returns 401

---

## Team Management

### Task 3.1: Auto-Created Personal Team
**Verify:**
- [ ] "Personal" team auto-created for new users
- [ ] User is owner of Personal team

### Task 3.2: Team CRUD
1. Create new team "Test Team"
2. Update team name
3. View team details

**Verify:**
- [ ] Team created successfully
- [ ] Team name updated
- [ ] Team shows in team switcher

### Task 3.3: Team Members
1. Invite member by email
2. Verify invitation email sent (if SMTP configured)
3. Accept invitation (in new browser/incognito)
4. Change member role
5. Remove member

**Verify:**
- [ ] Invitation created
- [ ] Member can accept invite
- [ ] Role changes applied
- [ ] Member removal works

### Task 3.4: Team Audit Logs
1. Perform various actions on team
2. Check audit logs at Teams > Settings > Audit

**Verify:**
- [ ] Actions logged with timestamp
- [ ] Actor (user) recorded
- [ ] Resource type and action recorded

---

## Project Management

### Task 4.1: Project CRUD
1. Create new project "Test Project"
2. Update project description
3. Delete project (empty)

**Verify:**
- [ ] Project created
- [ ] Project updated
- [ ] Project deleted

### Task 4.2: Project Organization
1. Create apps in different projects
2. Move app between projects

**Verify:**
- [ ] Apps organized by project
- [ ] App can be reassigned to different project

---

## App Deployment

### Task 5.1: Create App from Git URL
1. Apps > New App
2. Enter: `https://github.com/heroku/node-js-getting-started.git`
3. Set name, port (3000), build type (auto/nixpacks)
4. Deploy

**Verify:**
- [ ] App created
- [ ] Build starts automatically
- [ ] Build logs stream (check WebSocket)
- [ ] Container running after build
- [ ] App accessible via assigned port

### Task 5.2: Create App via GitHub App
1. Connect GitHub account
2. Select repository from list
3. Configure and deploy

**Verify:**
- [ ] GitHub repos listed
- [ ] Can select branch
- [ ] Webhook configured automatically

### Task 5.3: Build Types
Test each build type:

**5.3.1 Nixpacks (Auto-detect)**
- Use Node.js repo
- [ ] Build completes
- [ ] Correct runtime detected

**5.3.2 Dockerfile**
- Use repo with Dockerfile
- [ ] Dockerfile detected
- [ ] Build completes

**5.3.3 Railpack** (if available)
- [ ] Build completes (or graceful fallback)

**5.3.4 Cloud Native Buildpacks**
- Configure app to use `buildpack` type
- [ ] Build completes with pack CLI

**5.3.5 Static Site**
- Configure as static build
- [ ] Build output served correctly

### Task 5.4: Deployment Operations
1. View deployment history
2. Trigger manual redeploy
3. Rollback to previous version
4. View deployment logs

**Verify:**
- [ ] Deployment history shows all deploys
- [ ] Manual redeploy works
- [ ] Rollback switches to previous container
- [ ] Logs show build output

### Task 5.5: App Controls
1. Stop running app
2. Start stopped app
3. Restart running app

**Verify:**
- [ ] Stop: Container stops, status updated
- [ ] Start: Container starts, status updated
- [ ] Restart: Quick stop/start cycle

### Task 5.6: Resource Limits
1. Edit app settings
2. Set CPU limit (e.g., "1")
3. Set Memory limit (e.g., "512m")
4. Redeploy and verify

**Verify:**
- [ ] Limits applied to container
- [ ] `docker inspect` shows correct limits

### Task 5.7: Health Checks
1. Configure health check path (e.g., `/health`)
2. Set health check interval
3. Deploy and monitor

**Verify:**
- [ ] Health check running
- [ ] Status shows healthy/unhealthy correctly
- [ ] Auto-rollback triggers on failure (if enabled)

---

## Environment Variables

### Task 6.1: Environment Variables UI
1. Go to App > Settings > Environment
2. Add variable: `TEST_VAR=hello`
3. Add secret: `SECRET_KEY=abc123` (mark as secret)
4. Redeploy app

**Verify:**
- [ ] Variables UI accessible
- [ ] Variables saved
- [ ] Secrets hidden in UI
- [ ] Variables available in container (`docker exec <container> env`)

### Task 6.2: PORT Variable Auto-Injection
1. Create app without setting PORT
2. Deploy
3. Check container environment

**Verify:**
- [ ] PORT automatically set to configured app port
- [ ] App listens on correct port

---

## Managed Databases

### Task 7.1: Create Database
1. Databases > New Database
2. Select type: PostgreSQL
3. Configure name, version, credentials
4. Create

**Verify:**
- [ ] Database container created
- [ ] Database running
- [ ] Connection string generated

### Task 7.2: Database Operations
1. Stop database
2. Start database
3. View logs
4. Check stats

**Verify:**
- [ ] Stop/start works
- [ ] Logs accessible
- [ ] Stats show CPU/memory

### Task 7.3: Database Backups
1. Create manual backup
2. Schedule automatic backup
3. Download backup
4. Delete old backup

**Verify:**
- [ ] Manual backup created
- [ ] Schedule saved
- [ ] Backup downloadable
- [ ] Delete removes file

### Task 7.4: Database Types
Test creating each supported type:
- [ ] PostgreSQL
- [ ] MySQL
- [ ] MariaDB
- [ ] MongoDB
- [ ] Redis
- [ ] Valkey

---

## Docker Compose Services

### Task 8.1: Service Templates
1. Services > Templates
2. Browse available templates
3. Deploy a template (e.g., Redis, Nginx)

**Verify:**
- [ ] Templates listed with categories
- [ ] Template deploys successfully
- [ ] Service running

### Task 8.2: Custom Docker Compose
1. Services > New Service
2. Enter custom docker-compose.yml
3. Deploy

**Verify:**
- [ ] Compose file validated
- [ ] Multi-container service starts
- [ ] All containers accessible

### Task 8.3: Service Operations
1. Stop service
2. Start service
3. View logs
4. Delete service

**Verify:**
- [ ] All operations complete successfully
- [ ] Resources cleaned up on delete

---

## Volumes & Persistence

### Task 9.1: Create Volume
1. App > Settings > Volumes
2. Add volume with mount path
3. Redeploy app

**Verify:**
- [ ] Volume created
- [ ] Data persists across restarts

### Task 9.2: Volume Backup
1. Create volume backup
2. Download backup

**Verify:**
- [ ] Backup file generated
- [ ] Backup downloadable

---

## Alerting & Notifications

### Task 10.1: Create Alert
1. App > Settings > Alerts
2. Add CPU usage alert (> 80%)
3. Add memory alert (> 90%)

**Verify:**
- [ ] Alerts created
- [ ] Alert conditions evaluated

### Task 10.2: Notification Channels
1. Settings > Notifications
2. Add Slack webhook channel
3. Add email channel (if SMTP configured)
4. Test notification

**Verify:**
- [ ] Channel created
- [ ] Test notification received
- [ ] Alert triggers notification

### Task 10.3: Alert Events
1. View alert event history
2. Check triggered/resolved events

**Verify:**
- [ ] Events logged
- [ ] Timestamps accurate

---

## Preview Deployments

### Task 11.1: PR Preview (GitHub App)
1. Open PR on connected repo
2. Check for preview deployment
3. Access preview URL

**Verify:**
- [ ] Preview auto-created on PR
- [ ] Preview accessible
- [ ] Preview deleted on PR close/merge

### Task 11.2: Manual Preview
1. Create preview for specific branch
2. Test preview URL
3. Delete preview

**Verify:**
- [ ] Preview created
- [ ] Isolated from main deployment
- [ ] Cleanup works

---

## System Monitoring

### Task 12.1: Dashboard Stats
1. Check main dashboard
2. Verify running apps count
3. Verify memory/CPU charts

**Verify:**
- [ ] Stats accurate
- [ ] Charts render (stats history endpoint)
- [ ] Recent events list

### Task 12.2: System Health
```bash
curl http://localhost:8080/api/system/health
```

**Verify:**
- [ ] Health endpoint returns status
- [ ] All checks pass (database, runtime, disk)

### Task 12.3: Disk Monitoring
```bash
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/system/disk
```

**Verify:**
- [ ] Disk stats returned
- [ ] Values reasonable

### Task 12.4: Metrics Endpoint
```bash
curl http://localhost:8080/metrics
```

**Verify:**
- [ ] Prometheus metrics exposed
- [ ] Request counts, durations visible

---

## Proxy & Routing

### Task 13.1: Custom Domain
1. App > Settings > Domains
2. Add custom domain
3. Configure DNS

**Verify:**
- [ ] Domain added to routes
- [ ] App accessible via domain

### Task 13.2: SSL/TLS
1. Enable HTTPS for custom domain
2. Verify certificate provisioned (Let's Encrypt)

**Verify:**
- [ ] HTTPS works
- [ ] Certificate valid
- [ ] HTTP redirects to HTTPS

### Task 13.3: Routes API
```bash
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/routes
```

**Verify:**
- [ ] All routes listed
- [ ] Health status accurate

---

## Security Features

### Task 14.1: Basic Auth
1. App > Settings > Security
2. Enable HTTP Basic Auth
3. Set username/password
4. Test access

**Verify:**
- [ ] Auth prompt appears
- [ ] Correct credentials grant access
- [ ] Wrong credentials denied

### Task 14.2: Rate Limiting
```bash
# Rapid requests to trigger rate limit
for i in {1..100}; do curl -s http://localhost:8080/api/auth/login > /dev/null; done
```

**Verify:**
- [ ] 429 status returned after limit reached
- [ ] Limits configurable in rivetr.toml

### Task 14.3: SSH Keys
1. Settings > SSH Keys
2. Add SSH public key
3. Test git clone with SSH

**Verify:**
- [ ] Key stored
- [ ] Private repos accessible (if configured)

---

## WebSocket Features

### Task 15.1: Build Log Streaming
1. Deploy an app
2. Open browser DevTools > Network > WS
3. Watch for WebSocket connection

**Verify:**
- [ ] WebSocket connects to `/api/deployments/:id/logs/stream`
- [ ] Logs stream in real-time
- [ ] Connection closes after build

### Task 15.2: Container Terminal
1. App > Actions > Terminal
2. Execute commands

**Verify:**
- [ ] Terminal opens
- [ ] Commands execute in container
- [ ] Output displayed

---

## API Endpoints

### Full API Test Suite
Run these curl commands to verify all major endpoints:

```bash
TOKEN=$(grep admin_token /opt/rivetr/rivetr.toml | cut -d'"' -f2)
BASE="http://localhost:8080/api"

# Health (no auth)
curl http://localhost:8080/health

# System
curl -H "Authorization: Bearer $TOKEN" $BASE/system/stats
curl -H "Authorization: Bearer $TOKEN" $BASE/system/disk
curl -H "Authorization: Bearer $TOKEN" $BASE/system/health
curl -H "Authorization: Bearer $TOKEN" "$BASE/system/stats/history?hours=24"

# Apps
curl -H "Authorization: Bearer $TOKEN" $BASE/apps
curl -H "Authorization: Bearer $TOKEN" $BASE/apps/{id}
curl -H "Authorization: Bearer $TOKEN" $BASE/apps/{id}/deployments
curl -H "Authorization: Bearer $TOKEN" $BASE/apps/{id}/env-vars

# Teams
curl -H "Authorization: Bearer $TOKEN" $BASE/teams
curl -H "Authorization: Bearer $TOKEN" $BASE/teams/{id}/members

# Databases
curl -H "Authorization: Bearer $TOKEN" $BASE/databases

# Services
curl -H "Authorization: Bearer $TOKEN" $BASE/services
curl -H "Authorization: Bearer $TOKEN" $BASE/templates

# Routes
curl -H "Authorization: Bearer $TOKEN" $BASE/routes
curl -H "Authorization: Bearer $TOKEN" $BASE/routes/health

# Audit
curl -H "Authorization: Bearer $TOKEN" $BASE/audit
```

---

## Issues Log

### Critical Issues
| ID | Description | Status | Fix Version |
|----|-------------|--------|-------------|
| | No critical issues found | N/A | |

### High Priority Issues
| ID | Description | Status | Fix Version |
|----|-------------|--------|-------------|
| H1 | Stats history chart 401 Unauthorized - wrong localStorage key | Fixed | v0.2.12 |

### Medium Priority Issues
| ID | Description | Status | Fix Version |
|----|-------------|--------|-------------|
| | | | |

### Low Priority Issues
| ID | Description | Status | Fix Version |
|----|-------------|--------|-------------|
| L1 | Console warning: Pattern attribute regex error | Open | |

---

## Testing Checklist Summary

**Last tested: 2026-02-05 | Version: v0.2.12 | Server: 167.71.46.193**

### Core Functionality
- [x] Installation complete and service running
- [x] User authentication working
- [x] Team/project management working
- [x] App deployment (Nixpacks tested)
- [x] Environment variables working (PORT auto-injection verified)
- [x] Managed databases working (PostgreSQL 16 tested)
- [x] Docker Compose services working (Uptime Kuma template deployed)

### Advanced Features
- [ ] Volumes and persistence
- [ ] Alerting and notifications
- [ ] Preview deployments
- [x] System monitoring (all health checks passing)
- [ ] Proxy and routing
- [ ] Security features

### Integrations
- [ ] GitHub App integration
- [ ] Webhook deployments
- [ ] SSL/TLS certificates

### WebSocket/Real-time
- [x] Build log streaming
- [x] Container terminal (commands execute successfully)
- [x] Stats history charts (fixed in v0.2.12)
- [x] Runtime log streaming (SSE working)

---

## Appendix: Common Troubleshooting

### Service won't start
```bash
journalctl -u rivetr -f  # View logs
systemctl status rivetr  # Check status
```

### Container issues
```bash
docker ps -a                    # List all containers
docker logs <container_id>      # View container logs
docker inspect <container_id>   # Detailed info
```

### Database issues
```bash
sqlite3 /var/lib/rivetr/rivetr.db "SELECT * FROM apps;"
```

### Network issues
```bash
ss -tlnp | grep -E "(80|443|8080)"  # Check ports
curl -v localhost:8080              # Test API
```
