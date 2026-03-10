# Rivetr Deployment Guide

Production deployment, configuration reference, upgrading, multi-server setup, backup, and troubleshooting.

## Quick Install

Run this on a fresh Linux server:

```bash
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

The script handles everything:

1. Detects your Linux distribution (Ubuntu, Debian, Fedora, CentOS, RHEL)
2. Installs Docker if not present
3. Installs build tools: Git, Nixpacks, Railpack, Pack CLI (Heroku/Paketo buildpacks)
4. Creates a `rivetr` system user with Docker socket access
5. Downloads the Rivetr binary from GitHub Releases (or builds from source as fallback)
6. Writes a config at `/opt/rivetr/rivetr.toml` with a randomly generated admin token
7. Registers a systemd service with `Restart=always`
8. Opens firewall ports 80, 443, and 8080 (UFW or firewalld)

After installation:

```
Dashboard:      http://your-server-ip:8080
Config file:    /opt/rivetr/rivetr.toml
Data directory: /var/lib/rivetr
Service logs:   sudo journalctl -u rivetr -f
```

Visit the dashboard to create your admin account on first use.

**Install a specific version:**

```bash
RIVETR_VERSION=v0.2.6 curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

The install script preserves your existing configuration and data when run again.

## Manual Installation

### Prerequisites

- Linux (Ubuntu 22.04+, Debian 12+, Fedora 39+, or compatible)
- Docker Engine 24+ or Podman 4+
- Ports 80, 443, and 8080 available
- At least 512 MB RAM (1 GB+ recommended)

### Step 1: Install Docker

**Ubuntu/Debian:**

```bash
sudo apt update && sudo apt install -y docker.io git
sudo systemctl enable --now docker
```

**Fedora/RHEL:**

```bash
sudo dnf install -y docker git
sudo systemctl enable --now docker
```

### Step 2: Download Rivetr

```bash
sudo mkdir -p /opt/rivetr
sudo curl -L -o /opt/rivetr/rivetr \
  https://github.com/KwaminaWhyte/rivetr/releases/latest/download/rivetr-linux-amd64
sudo chmod +x /opt/rivetr/rivetr
```

Or build from source:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
git clone https://github.com/KwaminaWhyte/rivetr.git && cd rivetr
cargo build --release
sudo cp target/release/rivetr /opt/rivetr/
```

### Step 3: Create Configuration

```bash
sudo mkdir -p /var/lib/rivetr
ADMIN_TOKEN=$(openssl rand -hex 32)

sudo tee /opt/rivetr/rivetr.toml > /dev/null << EOF
[server]
host = "0.0.0.0"
api_port = 8080
proxy_port = 80
proxy_https_port = 443
data_dir = "/var/lib/rivetr"

[auth]
admin_token = "$ADMIN_TOKEN"

[runtime]
runtime_type = "auto"
build_cpu_limit = "2"
build_memory_limit = "2g"

[proxy]
acme_enabled = true
acme_email = "admin@example.com"
acme_staging = false
sslip_enabled = true

[logging]
level = "info"

[container_monitor]
enabled = true
check_interval_secs = 30
max_restart_attempts = 5
EOF

echo "Admin token: $ADMIN_TOKEN"
```

### Step 4: Create Systemd Service

```bash
sudo tee /etc/systemd/system/rivetr.service > /dev/null << 'EOF'
[Unit]
Description=Rivetr PaaS
After=network-online.target docker.service
Requires=docker.service

[Service]
Type=simple
User=rivetr
WorkingDirectory=/opt/rivetr
ExecStart=/opt/rivetr/rivetr --config /opt/rivetr/rivetr.toml
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now rivetr
```

### Step 5: Open Firewall

```bash
# UFW (Ubuntu/Debian)
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 8080/tcp

# firewalld (Fedora/RHEL)
sudo firewall-cmd --permanent --add-port={80,443,8080}/tcp
sudo firewall-cmd --reload
```

## Service Management

```bash
sudo systemctl status rivetr
sudo systemctl start rivetr
sudo systemctl stop rivetr
sudo systemctl restart rivetr

# Live logs
sudo journalctl -u rivetr -f

# Last 100 lines
sudo journalctl -u rivetr -n 100 --no-pager
```

## Configuration Reference

The full annotated reference is [`rivetr.example.toml`](rivetr.example.toml). Below is a summary of every section.

### `[server]`

| Key | Default | Description |
|---|---|---|
| `host` | `0.0.0.0` | Bind address |
| `api_port` | `8080` | API and dashboard port |
| `proxy_port` | `80` | HTTP proxy port |
| `proxy_https_port` | `443` | HTTPS proxy port |
| `data_dir` | `./data` | SQLite database, certs, volumes |
| `external_url` | — | Public URL for OAuth callbacks (required when behind a tunnel) |

### `[auth]`

| Key | Default | Description |
|---|---|---|
| `admin_token` | (required) | API token for programmatic access |
| `encryption_key` | — | AES-256-GCM key for encrypting env vars at rest (recommended) |

### `[runtime]`

| Key | Default | Description |
|---|---|---|
| `runtime_type` | `auto` | `docker`, `podman`, or `auto` |
| `docker_socket` | `/var/run/docker.sock` | Docker socket path |
| `build_cpu_limit` | `2` | CPU limit for builds |
| `build_memory_limit` | `2g` | Memory limit for builds |

### `[proxy]`

| Key | Default | Description |
|---|---|---|
| `acme_enabled` | `false` | Enable Let's Encrypt |
| `acme_email` | — | Email for ACME account (required if enabled) |
| `acme_staging` | `true` | Use Let's Encrypt staging (set false for production) |
| `acme_cache_dir` | `./data/acme` | Directory for certs and account key |
| `base_domain` | — | Base domain for `app.base_domain` auto-subdomains |
| `auto_subdomain_enabled` | `false` | Auto-generate subdomains for new apps |
| `sslip_enabled` | `true` | Generate `hash.ip.sslip.io` domains per app |
| `server_ip` | — | Public IP for sslip.io domain generation |
| `health_check_interval` | `30` | Proxy health check interval (seconds) |
| `health_check_timeout` | `5` | Health check timeout (seconds) |

### `[logging]`

| Key | Default | Description |
|---|---|---|
| `level` | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error` |

### `[webhooks]`

| Key | Description |
|---|---|
| `github_secret` | HMAC-SHA256 secret for `X-Hub-Signature-256` verification |
| `gitlab_token` | Token matched against `X-Gitlab-Token` |
| `gitea_secret` | HMAC-SHA256 secret for `X-Gitea-Signature` verification |

### `[rate_limit]`

| Key | Default | Description |
|---|---|---|
| `enabled` | `true` | Enable rate limiting |
| `api_requests_per_window` | `100` | General API limit |
| `webhook_requests_per_window` | `500` | Webhook endpoint limit |
| `auth_requests_per_window` | `20` | Auth endpoint limit (brute-force protection) |
| `window_seconds` | `60` | Rolling window duration |

### `[cleanup]`

| Key | Default | Description |
|---|---|---|
| `enabled` | `true` | Automatically prune old deployments |
| `max_deployments_per_app` | `10` | Deployments retained per app |
| `cleanup_interval_seconds` | `3600` | How often cleanup runs |
| `prune_images` | `true` | Prune dangling Docker images after cleanup |

### `[disk_monitor]`

| Key | Default | Description |
|---|---|---|
| `enabled` | `true` | Monitor data directory disk usage |
| `check_interval_seconds` | `300` | Check frequency |
| `warning_threshold` | `80` | Warn at this disk usage % |
| `critical_threshold` | `90` | Log critical at this disk usage % |

### `[container_monitor]`

| Key | Default | Description |
|---|---|---|
| `enabled` | `true` | Monitor and auto-restart crashed containers |
| `check_interval_secs` | `30` | Check frequency |
| `max_restart_attempts` | `5` | Restarts before marking deployment failed |
| `initial_backoff_secs` | `5` | Initial delay after crash |
| `max_backoff_secs` | `300` | Maximum backoff delay |
| `stable_duration_secs` | `120` | Runtime required to reset restart counter |

### `[email]`

| Key | Default | Description |
|---|---|---|
| `enabled` | `false` | Enable SMTP email |
| `smtp_host` | — | SMTP server hostname |
| `smtp_port` | `587` | SMTP port |
| `smtp_username` | — | SMTP auth username |
| `smtp_password` | — | SMTP auth password |
| `smtp_tls` | `true` | Use TLS for SMTP |
| `from_address` | — | Sender email address |
| `from_name` | `Rivetr` | Sender display name |

## Upgrading

### Auto-Update (via API)

Rivetr can update itself:

```bash
# Trigger auto-update via API
curl -X POST http://localhost:8080/api/system/update \
  -H "Authorization: Bearer YOUR_ADMIN_TOKEN"
```

The service restarts automatically after the binary is replaced.

### Manual Upgrade

```bash
# Back up data first
sudo tar -czf rivetr-backup-$(date +%Y%m%d).tar.gz /var/lib/rivetr

# Download new binary
sudo curl -L -o /opt/rivetr/rivetr.new \
  https://github.com/KwaminaWhyte/rivetr/releases/download/NEW_VERSION/rivetr-linux-amd64
sudo chmod +x /opt/rivetr/rivetr.new

# Swap binaries
sudo systemctl stop rivetr
sudo mv /opt/rivetr/rivetr /opt/rivetr/rivetr.old
sudo mv /opt/rivetr/rivetr.new /opt/rivetr/rivetr

# Start and verify
sudo systemctl start rivetr
sudo systemctl status rivetr
```

Roll back by swapping `rivetr.old` back to `rivetr` and restarting.

### Re-run Install Script

The install script is idempotent and preserves existing config:

```bash
RIVETR_VERSION=v0.2.6 curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

## Multi-Server Setup

Rivetr can manage deployments on remote servers via SSH.

### Register a Remote Server

1. Open the dashboard, go to **Settings → Servers**.
2. Click **Add Server** and provide:
   - Server name and hostname/IP
   - SSH port and username
   - SSH private key (Rivetr encrypts and stores this with AES-256-GCM)
3. Rivetr tests the connection and verifies Docker is available on the remote.

### Using a Remote Server

Once registered, you can assign apps to deploy to any registered server. The deployment workflow is:

1. Clone and build on the build server (local or dedicated remote)
2. Push the image to the configured container registry
3. Pull the image on the target server and start the container
4. Update the proxy route on the target server

### Browser-Based Terminal

From the dashboard, open **Servers → [server name] → Terminal** for a browser-based SSH session to any registered remote server.

## Backup and Restore

### Automated S3 Backups

Configure S3-compatible backup in the dashboard under **Settings → Backups**:

- Supports AWS S3, MinIO, Cloudflare R2, and any S3-compatible endpoint
- Choose what to back up: SQLite database, volumes, SSL certificates
- Set a schedule (cron expression) and retention count

### Manual Backup

```bash
# Full instance backup (database + config + certs)
sudo tar -czf rivetr-backup-$(date +%Y%m%d-%H%M%S).tar.gz \
  /var/lib/rivetr \
  /opt/rivetr/rivetr.toml
```

Or trigger via API:

```bash
curl -X POST http://localhost:8080/api/system/backup \
  -H "Authorization: Bearer YOUR_ADMIN_TOKEN" \
  --output rivetr-backup.tar.gz
```

### Restore

```bash
sudo systemctl stop rivetr
sudo tar -xzf rivetr-backup-YYYYMMDD.tar.gz -C /
sudo systemctl start rivetr
```

Database migrations run automatically at startup, so restoring an older backup and starting a newer binary is safe.

## Troubleshooting

### Service won't start

```bash
# Check recent logs
sudo journalctl -u rivetr -n 100 --no-pager

# Run manually to see errors immediately
sudo /opt/rivetr/rivetr --config /opt/rivetr/rivetr.toml

# Verify Docker is running
sudo systemctl status docker
sudo docker info
```

Common causes:
- **Docker not running**: `sudo systemctl start docker`
- **Config syntax error**: Check `rivetr.toml` for missing quotes or invalid values
- **Port in use**: See "Port conflicts" below

### Port 80 permission denied

```bash
# Allow the binary to bind privileged ports without running as root
sudo setcap 'cap_net_bind_service=+ep' /opt/rivetr/rivetr
```

The install script does this automatically. Re-apply after upgrading the binary.

### Port conflicts

```bash
sudo ss -tlnp | grep -E ':(80|443|8080)'
```

Change ports in `/opt/rivetr/rivetr.toml` if something else is already bound:

```toml
[server]
api_port = 8081
proxy_port = 8000
```

### Containers not restarting after server reboot

All containers deployed by Rivetr are created with `--restart=unless-stopped`. Verify:

```bash
docker inspect <container-id> | grep -A 3 RestartPolicy

# Fix containers missing the policy
docker update --restart unless-stopped <container-id>
```

### Disk space

```bash
df -h
du -sh /var/lib/rivetr/*

# Clean old Docker layers
docker system prune -a

# Rivetr also does this on its cleanup schedule (see [cleanup] config)
```

### Reinstalling from scratch

```bash
sudo systemctl stop rivetr
sudo systemctl disable rivetr
sudo rm /etc/systemd/system/rivetr.service
sudo rm -rf /opt/rivetr /var/lib/rivetr
sudo userdel rivetr
sudo systemctl daemon-reload

# Fresh install
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

## Uninstall

```bash
sudo systemctl stop rivetr
sudo systemctl disable rivetr
sudo rm /etc/systemd/system/rivetr.service
sudo rm -rf /opt/rivetr

# Optionally remove data (this deletes all apps, deployments, and databases)
sudo rm -rf /var/lib/rivetr

sudo systemctl daemon-reload
```

## Support

- GitHub Issues: https://github.com/KwaminaWhyte/rivetr/issues
- Security vulnerabilities: https://github.com/KwaminaWhyte/rivetr/security/advisories/new
