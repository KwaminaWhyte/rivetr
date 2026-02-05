# Rivetr Live Testing Runbook

This document guides live testing sessions for the Rivetr deployment platform. Follow these steps systematically to ensure comprehensive testing.

---

## IMPORTANT REMINDERS

### After Making Code Changes
1. **Update CHANGELOG.md** - Every fix/feature must be documented in `/CHANGELOG.md`
2. **Commit with version tag** - Use semantic versioning (e.g., `v0.2.12`)
3. **Direct upload for fast testing** - Build locally and SCP to server instead of waiting for GitHub Actions

### Fast Testing Workflow (Skip GitHub Actions)

**IMPORTANT: Do NOT build locally on macOS!**
The server runs Linux x86_64. Building on macOS produces incompatible binaries.
Always push changes to server and build there.

```bash
# Push code changes to GitHub (or directly to server)
git push origin main

# SSH to server and build there
ssh root@167.71.46.193 << 'EOF'
cd /tmp && rm -rf rivetr
git clone --depth 1 https://github.com/KwaminaWhyte/rivetr.git
cd rivetr
cargo build --release
systemctl stop rivetr
cp target/release/rivetr /opt/rivetr/rivetr
chmod +x /opt/rivetr/rivetr
chown rivetr:rivetr /opt/rivetr/rivetr
setcap 'cap_net_bind_service=+ep' /opt/rivetr/rivetr
systemctl start rivetr
EOF
```

Alternative: Use cross-compilation with cargo-zigbuild (if installed):
```bash
# On macOS with cargo-zigbuild and zig installed
cargo zigbuild --release --target x86_64-unknown-linux-gnu
scp target/x86_64-unknown-linux-gnu/release/rivetr root@167.71.46.193:/opt/rivetr/rivetr.new
ssh root@167.71.46.193 "systemctl stop rivetr && mv /opt/rivetr/rivetr.new /opt/rivetr/rivetr && chmod +x /opt/rivetr/rivetr && chown rivetr:rivetr /opt/rivetr/rivetr && setcap 'cap_net_bind_service=+ep' /opt/rivetr/rivetr && systemctl start rivetr"
```

---

## Quick Reference

### Server Credentials
```
IP: 167.71.46.193
Password: famous10@365#Rich
SSH: ssh root@167.71.46.193
```

### Rivetr Access
```
Dashboard: http://167.71.46.193:8080
Config: /opt/rivetr/rivetr.toml
Logs: journalctl -u rivetr -f
```

### Test Repositories
| Repo | Build Type | Notes |
|------|------------|-------|
| `https://github.com/heroku/node-js-getting-started.git` | Nixpacks | Node.js, PORT env var |
| `https://github.com/KwaminaWhyte/adamus-forms` | Docker | Has Dockerfile |
| `https://github.com/KwaminaWhyte/pizzazone-chain/tree/dev/docker` | Docker | Multi-service |
| `https://github.com/KwaminaWhyte/bizcore-enterprise` | Docker | May need DB |

---

## Phase 1: Server Preparation

### Step 1.1: Connect and Assess
```bash
ssh root@167.71.46.193

# Check current state
systemctl status rivetr
docker ps -a
ls -la /opt/rivetr /var/lib/rivetr

# Check system resources
free -h
df -h
```

### Step 1.2: Clean Installation (If Needed)
Only run if starting fresh or previous installation is corrupted:
```bash
# Stop and remove existing installation
systemctl stop rivetr 2>/dev/null
rm -rf /opt/rivetr /var/lib/rivetr
rm -f /etc/systemd/system/rivetr.service
systemctl daemon-reload
userdel -r rivetr 2>/dev/null

# Stop conflicting services
systemctl stop nginx 2>/dev/null && systemctl disable nginx 2>/dev/null

# Clean Docker (optional - removes all containers/images)
# docker system prune -af
```

### Step 1.3: Install Rivetr
```bash
# Install latest release
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash

# Or specific version
export RIVETR_VERSION=v0.2.10
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

### Step 1.4: Verify Installation
```bash
# Service status
systemctl status rivetr

# Binary version (if available)
/opt/rivetr/rivetr --version 2>/dev/null || echo "No version flag"

# Build tools
nixpacks --version
pack version
railpack --version 2>/dev/null || echo "Railpack not installed (optional)"

# Network
curl -s http://localhost:8080/health
ss -tlnp | grep -E "(80|8080)"
```

Document installation results in findings file.

---

## Phase 2: Initial Setup & Authentication

### Step 2.1: Access Dashboard
Open in browser: `http://167.71.46.193:8080`

**Test using Playwright MCP:**
```javascript
// Navigate to dashboard
await page.goto('http://167.71.46.193:8080');

// Should see login/setup page
await page.waitForSelector('[data-testid="auth-form"]', { timeout: 10000 });
```

### Step 2.2: Create Admin Account
If first setup:
1. Fill registration form
2. Use strong password (min 12 chars, mixed case, numbers, symbols)
3. Submit and verify redirect to dashboard

**Record credentials in secure location (not this file for real deployments).**

### Step 2.3: Test Login/Logout
1. Logout
2. Login with created credentials
3. Verify session persists on refresh
4. Test API token from config file:
```bash
TOKEN=$(grep admin_token /opt/rivetr/rivetr.toml | cut -d'"' -f2)
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/apps
```

---

## Phase 3: App Deployment Testing

### Step 3.1: Create and Deploy Test App
1. Navigate to Apps > New App
2. Configure:
   - Name: `hello-world-test`
   - Git URL: `https://github.com/heroku/node-js-getting-started.git`
   - Branch: `main`
   - Port: `3000`
   - Build Type: `Nixpacks (Auto-detect)`
3. Click Create & Deploy

### Step 3.2: Monitor Build
1. Watch build logs in UI
2. If logs not streaming, check console for WebSocket errors
3. Monitor via SSH:
```bash
journalctl -u rivetr -f | grep -i "build\|deploy"
docker logs -f rivetr-hello-world-test 2>/dev/null
```

### Step 3.3: Verify Deployment
```bash
# Get container info
docker ps | grep rivetr-hello-world-test

# Check port mapping (e.g., 32768->3000)
PORT=$(docker port rivetr-hello-world-test 3000 | cut -d: -f2)
echo "App accessible at: http://167.71.46.193:$PORT"

# Verify PORT env var
docker exec rivetr-hello-world-test env | grep PORT

# Test app responds
curl -s http://localhost:$PORT | head -20
```

### Step 3.4: Test App Controls
1. Stop app (UI or API)
2. Verify container stopped
3. Start app
4. Verify container running
5. Restart app

---

## Phase 4: Environment Variables Testing

### Step 4.1: Add Environment Variable (API)
```bash
TOKEN=$(grep admin_token /opt/rivetr/rivetr.toml | cut -d'"' -f2)
APP_ID="<get-from-dashboard>"

curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"key": "TEST_VAR", "value": "hello_world", "is_secret": false}' \
  "http://localhost:8080/api/apps/$APP_ID/env-vars"
```

### Step 4.2: Verify Variable Propagation
1. Redeploy app
2. Check container:
```bash
docker exec rivetr-hello-world-test env | grep TEST_VAR
```

---

## Phase 5: Database Testing

### Step 5.1: Create Database
1. Navigate to Databases > New Database
2. Configure:
   - Type: PostgreSQL
   - Name: `test-postgres`
   - Version: 16
3. Create and wait for container

### Step 5.2: Verify Database
```bash
# Check container
docker ps | grep test-postgres

# Test connection (get credentials from UI)
docker exec -it <container_id> psql -U <username> -d <database> -c "SELECT 1;"
```

### Step 5.3: Test Database Operations
- Stop database
- Start database
- View logs
- Create backup

---

## Phase 6: Advanced Features Testing

### Step 6.1: Docker Compose Service
1. Services > New Service
2. Use template or custom compose
3. Deploy and verify all containers

### Step 6.2: Volumes
1. Add volume to app
2. Write data inside container
3. Restart app
4. Verify data persists

### Step 6.3: Alerts
1. Create CPU alert (> 80%)
2. Create memory alert (> 90%)
3. Configure notification channel

### Step 6.4: System Stats
```bash
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/system/stats
curl -H "Authorization: Bearer $TOKEN" "http://localhost:8080/api/system/stats/history?hours=24"
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/system/disk
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/system/health
```

---

## Phase 7: Issue Documentation

Document all issues found in `deployment-test-findings.md`:

```markdown
### Issue: [Title]
- **Severity**: Critical/High/Medium/Low
- **Category**: Installation/Deployment/UI/API/etc.
- **Description**: What happened
- **Expected**: What should happen
- **Steps to Reproduce**:
  1. Step one
  2. Step two
- **Workaround**: (if any)
- **Fix**: (after resolution)
```

---

## Phase 8: Bug Fixing Workflow

### Step 8.1: Identify Root Cause
```bash
# Check logs
journalctl -u rivetr -n 100

# Check server-side errors
grep -i "error\|panic\|fail" /var/log/syslog | tail -50

# Check container logs
docker logs rivetr-<app-name> 2>&1 | tail -50
```

### Step 8.2: Fix Locally
1. Make code changes on local machine
2. Run tests: `cargo test`
3. Check formatting: `cargo fmt --check && cargo clippy`
4. Build release binary: `cargo build --release`

### Step 8.3: Direct Upload to Server (FAST - No GitHub Wait)
**Use this method to test fixes immediately without waiting for GitHub Actions build.**

```bash
# On LOCAL machine - build release binary
cd /Users/admin/Desktop/solo/rivetr
cargo build --release

# Upload directly to server
scp target/release/rivetr root@167.71.46.193:/opt/rivetr/rivetr.new

# On SERVER - swap binary and restart
ssh root@167.71.46.193 << 'EOF'
systemctl stop rivetr
cp /opt/rivetr/rivetr /opt/rivetr/rivetr.bak  # Backup
mv /opt/rivetr/rivetr.new /opt/rivetr/rivetr
chmod +x /opt/rivetr/rivetr
chown rivetr:rivetr /opt/rivetr/rivetr
setcap 'cap_net_bind_service=+ep' /opt/rivetr/rivetr
systemctl start rivetr
systemctl status rivetr
EOF

# Verify fix works
<run test that previously failed>
```

### Step 8.4: Commit, Tag, and Update Changelog
Once the fix is verified working:

```bash
# Update CHANGELOG.md with the fix details
# Follow Keep a Changelog format (https://keepachangelog.com)

# Commit with descriptive message
git add -A
git commit -m "fix: Description of fix"

# Create version tag (increment from current version)
git tag -a v0.2.XX -m "Release v0.2.XX: Fix description"

# Push code and tags
git push origin main --tags
```

**IMPORTANT: Always update `/CHANGELOG.md` when making changes!**
- Add entry under appropriate version section
- Include date, category (Added/Fixed/Changed), and description
- Update Version History Summary table
- Update link references at bottom of file

### Step 8.5: (Optional) Wait for GitHub Build
If you need the official release binary:
1. Check GitHub Actions for build completion (~5-10 minutes)
2. Verify binary artifact created at releases page

### Step 8.6: Update Server from GitHub Release
```bash
# On server - download official release
systemctl stop rivetr

export RIVETR_VERSION=v0.2.XX
curl -fsSL -o /opt/rivetr/rivetr \
  "https://github.com/KwaminaWhyte/rivetr/releases/download/$RIVETR_VERSION/rivetr-$RIVETR_VERSION-linux-x86_64"
chmod +x /opt/rivetr/rivetr
chown rivetr:rivetr /opt/rivetr/rivetr
setcap 'cap_net_bind_service=+ep' /opt/rivetr/rivetr

systemctl start rivetr
```

---

## Phase 9: Final Checklist

Before concluding testing session:

### Core Features
- [ ] Fresh installation works
- [ ] User registration/login works
- [ ] App deployment succeeds
- [ ] Container accessible externally
- [ ] Environment variables injected
- [ ] App controls (start/stop/restart) work

### Database Features
- [ ] Database creation works
- [ ] Database accessible
- [ ] Backups work

### UI Features
- [ ] Dashboard loads
- [ ] Stats display correctly
- [ ] Build logs stream (WebSocket)
- [ ] No console errors

### API Features
- [ ] Token auth works
- [ ] All major endpoints respond
- [ ] Rate limiting functional

### Documentation Updated
- [ ] Issues logged in findings file
- [ ] Install script version updated (if changed)
- [ ] TESTING-GUIDE.md reflects any new features

---

## Appendix: Quick Commands Reference

### SSH Access
```bash
ssh root@167.71.46.193
```

### Service Management
```bash
systemctl start|stop|restart|status rivetr
journalctl -u rivetr -f
```

### Docker Commands
```bash
docker ps -a
docker logs <container>
docker exec -it <container> /bin/sh
docker system prune -af  # Clean all
```

### API Testing
```bash
TOKEN=$(grep admin_token /opt/rivetr/rivetr.toml | cut -d'"' -f2)
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/<endpoint>
```

### Database Access
```bash
sqlite3 /var/lib/rivetr/rivetr.db
.tables
SELECT * FROM apps;
.exit
```
