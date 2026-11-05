# Rivetr Deployment Guide

Production install, upgrading, multi-server setup, backup, and troubleshooting.

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
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo RIVETR_VERSION=vX.Y.Z bash
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

## Configuration

The installer writes `/opt/rivetr/rivetr.toml`. Every section and key is documented in [docs/reference/configuration.md](docs/reference/configuration.md); [`rivetr.example.toml`](rivetr.example.toml) is a copyable starter. Restart the service after editing it. Instance settings changed from the dashboard are stored in the database and override the matching keys in the file.

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
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo RIVETR_VERSION=vX.Y.Z bash
```

## Multi-Server Setup

Rivetr deploys to remote servers over SSH. Adding a server, non-root users with passwordless sudo, and Tailscale are covered in [docs/reference/remote-servers.md](docs/reference/remote-servers.md).

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
