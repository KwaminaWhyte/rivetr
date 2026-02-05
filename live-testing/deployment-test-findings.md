# Rivetr Deployment Test - 2026-02-05

## Server Details
- IP: 167.71.46.193
- OS: Ubuntu 24.04.3 LTS
- Kernel: 6.8.0-90-generic (needs reboot for 6.8.0-94-generic)
- RAM: 1.9GB total, ~1.4GB available
- Disk: 48GB total, 42GB available

## Pre-Installation State
- Docker: Not installed
- Nginx: Running on ports 80, 443 (stopped and disabled)
- Previous Rivetr: None found

## Installation Process

### Cleanup Performed
1. Stopped and disabled nginx (`systemctl stop nginx && systemctl disable nginx`)
2. Removed old Rivetr artifacts (`/opt/rivetr`, `/var/lib/rivetr`, service file)
3. Deleted old rivetr user

### Installation Results
- Docker: v29.2.1 installed successfully
- Nixpacks: v1.41.0 installed successfully
- Railpack: FAILED (optional, GitHub binary not available for architecture)
- Pack CLI: v0.36.1 installed successfully
- Git: Already installed v2.43.0
- Rivetr: v0.2.9 installed successfully

### Service Status
- Status: Active (running)
- PID: 1286199
- Memory: 7.3MB (excellent!)
- API Port: 8080
- Proxy Port: 80

## Access Information
- Dashboard URL: http://167.71.46.193:8080
- Admin Token: eefaa7eea776342bfbd3ebf4992f4dc2c6afa4c7509c98ce826370dec6723333
- Config File: /opt/rivetr/rivetr.toml
- Data Directory: /var/lib/rivetr

## Issues Found

### 1. Railpack Installation Failed
- **Severity**: Minor (optional component)
- **Impact**: Railpack builds not available, but Nixpacks and Dockerfile builds work
- **Root Cause**: Binary not available on GitHub releases for linux-x86_64
- **Resolution**: Consider adding Railpack binary to releases or improving fallback

### 2. Kernel Update Pending
- **Severity**: Low
- **Impact**: Server needs reboot to load kernel 6.8.0-94-generic
- **Resolution**: User should schedule reboot at convenient time

### 3. PORT Environment Variable Not Set (BUG)
- **Severity**: High
- **Impact**: Apps that use PORT env var (like Heroku apps) listen on wrong port
- **Root Cause**: Rivetr doesn't set PORT env var when starting containers
- **Observed Behavior**:
  - Container port mapping: 32768->3000
  - App listening on: 5006 (Heroku node app default)
  - Result: Connection refused when accessing app
- **Resolution**: Rivetr should automatically set `PORT={configured_port}` env var when starting containers
- **Workaround**: None currently available (no env vars UI in dashboard)

### 4. WebSocket Build Logs Not Working
- **Severity**: Medium
- **Impact**: Build logs show "No logs yet" in UI during deployment
- **Root Cause**: WebSocket connection to `/api/apps/{id}/deployments/{id}/logs/stream` fails
- **Console Error**: `WebSocket connection to 'ws://167.71.46.193:8080/...' failed`
- **Note**: Server logs show build progress correctly via journalctl
- **Resolution**: Debug WebSocket proxy/handler for build log streaming

### 5. Stats History API Error
- **Severity**: Low
- **Impact**: Dashboard resource charts may not display
- **Console Error**: `Failed to load resource: /api/system/stats/history?hours=24`
- **Resolution**: Investigate stats history endpoint

### 6. Missing Environment Variables UI
- **Severity**: Medium
- **Impact**: Cannot set custom env vars for apps via dashboard
- **Note**: No "Environment Variables" section found in Settings tabs
- **Resolution**: Add environment variables management to app settings

## Testing Checklist
- [x] Web dashboard loads
- [x] User registration works (with strong password requirement)
- [x] Login works
- [x] Create team (auto-created "Personal" team)
- [x] Create project
- [x] Create app
- [x] Deploy app (Nixpacks build successful, 3m 9s)
- [ ] App accessible externally (FAILED - PORT env var issue)
- [ ] Build logs streaming (FAILED - WebSocket issue)
- [x] Container running (verified via docker ps)
- [x] App works inside container (verified via docker exec curl)

## Deployment Test Details

### App Deployed
- Name: hello-world-test
- Git URL: https://github.com/heroku/node-js-getting-started.git
- Build Type: Nixpacks (auto-detected Node.js)
- Build Time: 3 minutes 9 seconds
- Container Status: Running
- Memory Usage: 30.9 MB of 512 MB limit

### Container Info
- Container ID: c87d38614427
- Container Name: rivetr-hello-world-test
- Image: rivetr-hello-world-test:484ea04c-cc4b-4da6-9db8-e4666b90e5af
- Port Mapping: 0.0.0.0:32768->3000/tcp
- Internal URL: http://rivetr-hello-world-test-484ea04c:3000

## Fixes Applied

### PORT Environment Variable Fix (Applied)
**Files Modified:**
- `src/engine/pipeline.rs` (2 locations: main deploy & rollback)
- `src/engine/preview.rs` (1 location: preview deploys)

**Change:** Automatically set `PORT` environment variable to the configured container port if not already set by the user. This follows the standard PaaS pattern used by Heroku, Railway, and others.

**Code Added:**
```rust
// Automatically set PORT environment variable if not already set
// This is a common pattern in PaaS systems (Heroku, Railway, etc.)
if !env_vars.iter().any(|(k, _)| k == "PORT") {
    env_vars.push(("PORT".to_string(), app.port.to_string()));
}
```

## Remaining Issues (To Fix)
1. **WebSocket logs streaming** - Build logs not connecting
2. **Add env vars UI** - No way to set custom env vars in dashboard
3. **Stats history API** - Returns errors on dashboard

## Pre-Existing Test Failures (Unrelated to deployment)
- `engine::pack_builder::tests::test_empty_config`
- `engine::preview::tests::test_generate_preview_domain`

## Testing Summary

### v0.2.9 (Initial Test)
- **Installation**: PASS
- **Dashboard**: PASS
- **User Auth**: PASS
- **App Creation**: PASS
- **Deployment**: PASS (Nixpacks build completes)
- **Container Running**: PASS
- **App Accessibility**: FAILED (PORT env var not set)

### v0.2.10 (With Fix)
- **Update**: PASS (binary updated successfully)
- **Redeploy**: PASS (new container started)
- **PORT env var**: PASS (verified `PORT=3000` in container)
- **App Listening**: PASS (confirmed listening on port 3000)
- **App Accessibility**: **PASS** (http://167.71.46.193:32769 loads correctly)

## Verification Commands
```bash
# Verify PORT is set
docker exec rivetr-hello-world-test env | grep PORT
# Output: PORT=3000

# Verify app is listening on correct port
docker logs rivetr-hello-world-test 2>&1 | tail -5
# Output: Listening on 3000
```
