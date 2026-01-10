---
name: deployment-pipeline
description: Debug and understand the Rivetr deployment pipeline. Use when troubleshooting deployment failures, understanding pipeline stages, or working on engine code.
allowed-tools: Read, Grep, Glob, Bash
---

# Rivetr Deployment Pipeline

## Pipeline Overview

The deployment pipeline runs in `src/engine/pipeline.rs` and follows these stages:

```
Pending → Cloning → Building → Starting → HealthCheck → Running
                                              ↓
                                    [On failure: Rollback]
```

## Pipeline Stages

### 1. Pending
- Deployment created in database with `status = "pending"`
- Job queued via Tokio MPSC channel

### 2. Cloning
- Git repository cloned to temporary directory
- Supports HTTPS and SSH authentication
- Branch specified in app config

```rust
// Key function: clone_repo() in pipeline.rs
git2::Repository::clone(&git_url, &dest_path)?;
```

### 3. Building
- Dockerfile detected (or buildpack used)
- Container image built via runtime trait
- Build logs streamed to database

```rust
// Key function: run_build() in pipeline.rs
runtime.build(&BuildContext { ... }).await?;
```

#### Build Types
- **dockerfile**: Standard Dockerfile build
- **nixpacks**: Auto-generate Dockerfile from source (src/engine/nixpacks.rs)
- **railpack**: Railway's Nixpacks successor (src/engine/railpack.rs)
- **static**: Static site with NGINX (src/engine/static_builder.rs)
- **pack**: Heroku/Paketo buildpacks (src/engine/pack_builder.rs)
- **compose**: Docker Compose (src/engine/compose.rs)

### 4. Starting
- Container started on ephemeral port
- Environment variables injected
- Resource limits applied (CPU, memory)
- Pre-deploy commands executed

### 5. Health Check
- HTTP request to healthcheck endpoint
- Configurable timeout and retries
- On success: proceed to Running
- On failure: trigger rollback (if enabled)

```rust
// Key function: check_health() in pipeline.rs
reqwest::get(&health_url).await?.status().is_success()
```

### 6. Running
- Proxy route updated atomically (ArcSwap)
- Old container stopped and removed
- Post-deploy commands executed
- Status set to "running"

## Key Files

| File | Purpose |
|------|---------|
| `src/engine/pipeline.rs` | Main pipeline orchestration |
| `src/engine/mod.rs` | Engine module exports |
| `src/engine/nixpacks.rs` | Nixpacks builder |
| `src/engine/railpack.rs` | Railpack builder |
| `src/engine/static_builder.rs` | Static site builder |
| `src/engine/pack_builder.rs` | Heroku/Paketo CNB builder |
| `src/engine/build_detect.rs` | Auto-detect build type |
| `src/engine/cleanup.rs` | Old deployment cleanup |
| `src/engine/container_monitor.rs` | Container crash recovery |
| `src/engine/database_config.rs` | Database container configs |

## Debugging Commands

```bash
# Check deployment status
sqlite3 data/rivetr.db "SELECT id, app_id, status, error_message FROM deployments ORDER BY created_at DESC LIMIT 5"

# View deployment logs
sqlite3 data/rivetr.db "SELECT timestamp, message FROM deployment_logs WHERE deployment_id = 'DEPLOYMENT_ID' ORDER BY timestamp"

# Check container status
docker ps -a | grep rivetr

# View container logs
docker logs CONTAINER_ID --tail 100

# Check build context
ls -la /tmp/rivetr-build-*

# Manual health check
curl http://localhost:PORT/health
```

## Common Failure Scenarios

### Clone Failures
- **SSH key issues**: Check `ssh_keys` table, verify key format
- **Branch not found**: Verify branch exists in remote
- **Network issues**: Check DNS, firewall, git host availability

### Build Failures
- **Dockerfile not found**: Check `dockerfile_path` setting
- **Base image unavailable**: Check Docker Hub access
- **Build timeout**: Check `build_cpu_limit`, `build_memory_limit`
- **OOM during build**: Increase memory limit in config

### Start Failures
- **Port conflict**: Another container using the port
- **Image not found**: Build failed silently
- **Resource exhaustion**: Check system resources

### Health Check Failures
- **Endpoint not found**: Verify `healthcheck` path
- **App needs startup time**: Increase `healthcheck_timeout`
- **Wrong port**: Check `port` configuration
- **App crashes on start**: Check container logs

## Auto-Rollback

When enabled (`auto_rollback = true`), health check failure triggers:

1. Find last successful deployment for the app
2. Start container from that deployment's image
3. Update proxy route to old container
4. Mark current deployment as "failed"
5. Notify via configured channels

```rust
// Key function: trigger_auto_rollback() in pipeline.rs
```

## Environment Variables

Env vars are:
1. Encrypted at rest (AES-256-GCM) in `env_vars` table
2. Decrypted at container start
3. Passed via Docker/Podman `-e` flags

## Resource Limits

Applied to containers:
- `cpu_limit`: Docker NanoCPUs, Podman `--cpus`
- `memory_limit`: Supports m/mb/g/gb suffixes

Applied to builds:
- `runtime.build_cpu_limit` in config
- `runtime.build_memory_limit` in config

## Pre/Post Deploy Commands

Executed inside the container:
- `pre_deploy_commands`: After container starts, before health check
- `post_deploy_commands`: After health check passes

```rust
runtime.exec(&container_id, &["sh", "-c", &command]).await?;
```
