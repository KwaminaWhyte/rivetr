# Rivetr Deployment Guide

This guide covers deploying Rivetr in production. Rivetr is a lightweight, single-binary PaaS that manages application deployments, databases, and Docker Compose services.

## Quick Install (Recommended)

Run this single command on your server:

```bash
curl -fsSL https://raw.githubusercontent.com/KwaminaWhyte/rivetr/main/install.sh | sudo bash
```

This script will:

- Install Docker if not present
- Download and install Rivetr
- Create a systemd service for automatic restarts
- Configure firewall rules
- Generate a secure admin token

After installation, visit `http://your-server-ip:8080` to create your admin account.

## Manual Installation

### Prerequisites

- Linux server (Ubuntu 22.04+, Debian 12+, or Fedora 39+ recommended)
- Docker or Podman installed and running
- At least 1GB RAM (2GB+ recommended)
- Ports 80, 443, and 8080 available

### Step 1: Install Dependencies

**Ubuntu/Debian:**

```bash
sudo apt update
sudo apt install -y docker.io git
sudo systemctl enable docker
sudo systemctl start docker
```

**Fedora/RHEL:**

```bash
sudo dnf install -y docker git
sudo systemctl enable docker
sudo systemctl start docker
```

### Step 2: Download Rivetr

Download the latest release:

```bash
# Create installation directory
sudo mkdir -p /opt/rivetr
cd /opt/rivetr

# Download the binary (replace VERSION with actual version)
sudo curl -L -o rivetr https://github.com/KwaminaWhyte/rivetr/releases/download/VERSION/rivetr-linux-amd64
sudo chmod +x rivetr
```

Or build from source:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone https://github.com/KwaminaWhyte/rivetr.git
cd rivetr
cargo build --release

# Install
sudo cp target/release/rivetr /opt/rivetr/
```

### Step 3: Create Configuration

```bash
sudo mkdir -p /var/lib/rivetr

# Create config file
sudo tee /opt/rivetr/rivetr.toml > /dev/null << 'EOF'
[server]
host = "0.0.0.0"
api_port = 8080
proxy_port = 80
data_dir = "/var/lib/rivetr"

[auth]
# Generate a secure token: openssl rand -hex 32
admin_token = "YOUR_SECURE_TOKEN_HERE"
session_lifetime_hours = 168

[logging]
level = "info"

[runtime]
runtime = "docker"
build_cpu_limit = "2"
build_memory_limit = "2g"

[cleanup]
enabled = true
max_deployments_per_app = 10
cleanup_interval_hours = 24

[container_monitor]
enabled = true
check_interval_seconds = 60
max_restart_attempts = 3
EOF

# Generate admin token
ADMIN_TOKEN=$(openssl rand -hex 32)
sudo sed -i "s/YOUR_SECURE_TOKEN_HERE/$ADMIN_TOKEN/" /opt/rivetr/rivetr.toml
echo "Admin token: $ADMIN_TOKEN"
```

### Step 4: Create Systemd Service

```bash
sudo tee /etc/systemd/system/rivetr.service > /dev/null << 'EOF'
[Unit]
Description=Rivetr Deployment Engine
After=network-online.target docker.service
Requires=docker.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/rivetr
ExecStart=/opt/rivetr/rivetr --config /opt/rivetr/rivetr.toml
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable rivetr
sudo systemctl start rivetr
```

### Step 5: Configure Firewall

```bash
# UFW (Ubuntu)
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 8080/tcp

# firewalld (Fedora/RHEL)
sudo firewall-cmd --permanent --add-port=80/tcp
sudo firewall-cmd --permanent --add-port=443/tcp
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --reload
```

## Auto-Restart Behavior

Rivetr ensures all your services restart automatically:

### Rivetr Service

The systemd service has `Restart=always`, so Rivetr restarts on crash or reboot.

### Deployed Applications

All containers deployed by Rivetr are created with `--restart=unless-stopped`, meaning:

- Containers restart automatically if they crash
- Containers restart after server reboot
- Only manually stopped containers stay stopped

### Managed Databases

Database containers (PostgreSQL, MySQL, Redis, MongoDB) also use `--restart=unless-stopped`.

### Docker Compose Services

Services from Docker Compose files inherit their restart policy from the compose file. If not specified, Rivetr adds `restart: unless-stopped`.

## Service Management

```bash
# Check status
sudo systemctl status rivetr

# View logs
sudo journalctl -u rivetr -f

# Restart service
sudo systemctl restart rivetr

# Stop service
sudo systemctl stop rivetr
```

## Configuration Reference

### Server Settings

| Setting             | Default   | Description            |
| ------------------- | --------- | ---------------------- |
| `server.host`       | `0.0.0.0` | Bind address           |
| `server.api_port`   | `8080`    | API/dashboard port     |
| `server.proxy_port` | `80`      | Reverse proxy port     |
| `server.data_dir`   | `./data`  | Data storage directory |

### Auth Settings

| Setting                       | Default     | Description               |
| ----------------------------- | ----------- | ------------------------- |
| `auth.admin_token`            | (generated) | API authentication token  |
| `auth.session_lifetime_hours` | `168`       | Session duration (7 days) |

### Runtime Settings

| Setting                      | Default  | Description                       |
| ---------------------------- | -------- | --------------------------------- |
| `runtime.runtime`            | `docker` | Container runtime (docker/podman) |
| `runtime.build_cpu_limit`    | `2`      | CPU limit for builds              |
| `runtime.build_memory_limit` | `2g`     | Memory limit for builds           |

### Container Monitor

| Setting                                    | Default | Description               |
| ------------------------------------------ | ------- | ------------------------- |
| `container_monitor.enabled`                | `true`  | Enable crash monitoring   |
| `container_monitor.check_interval_seconds` | `60`    | Check frequency           |
| `container_monitor.max_restart_attempts`   | `3`     | Restarts before giving up |

## SSL/TLS with Let's Encrypt

Rivetr includes automatic SSL certificate management. To enable:

1. Point your domain's DNS to your server
2. Add a route in the dashboard with your domain
3. Rivetr will automatically obtain and renew certificates

For manual certificate configuration:

```toml
[proxy.tls]
cert_path = "/path/to/cert.pem"
key_path = "/path/to/key.pem"
```

## Reverse Proxy Setup (Nginx/Traefik)

If running behind another reverse proxy:

**Nginx:**

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## Backup and Recovery

### Data Directory

All Rivetr data is stored in the data directory (default: `/var/lib/rivetr`):

- `rivetr.db` - SQLite database (apps, deployments, users)
- `backups/` - Database backups
- `volumes/` - Persistent volume data
- `certs/` - SSL certificates

### Backup

```bash
# Stop Rivetr (optional, for consistency)
sudo systemctl stop rivetr

# Create backup
sudo tar -czf rivetr-backup-$(date +%Y%m%d).tar.gz /var/lib/rivetr

# Restart
sudo systemctl start rivetr
```

### Restore

```bash
sudo systemctl stop rivetr
sudo tar -xzf rivetr-backup-YYYYMMDD.tar.gz -C /
sudo systemctl start rivetr
```

## Upgrading

```bash
# Stop service
sudo systemctl stop rivetr

# Backup data
sudo tar -czf rivetr-backup-$(date +%Y%m%d).tar.gz /var/lib/rivetr

# Download new version
curl -L -o /opt/rivetr/rivetr.new https://github.com/KwaminaWhyte/rivetr/releases/download/NEW_VERSION/rivetr-linux-amd64
chmod +x /opt/rivetr/rivetr.new
mv /opt/rivetr/rivetr /opt/rivetr/rivetr.old
mv /opt/rivetr/rivetr.new /opt/rivetr/rivetr

# Start service
sudo systemctl start rivetr

# Verify
sudo systemctl status rivetr
```

## Troubleshooting

### Service won't start

```bash
# Check logs
sudo journalctl -u rivetr -n 100

# Check Docker
sudo systemctl status docker
sudo docker info

# Test manually
sudo /opt/rivetr/rivetr --config /opt/rivetr/rivetr.toml
```

### Containers not restarting

```bash
# Check container restart policy
docker inspect <container-id> | grep -A 5 RestartPolicy

# Manually set restart policy
docker update --restart unless-stopped <container-id>
```

### Port conflicts

```bash
# Check what's using the port
sudo ss -tlnp | grep :8080
sudo ss -tlnp | grep :80

# Change ports in config if needed
sudo nano /opt/rivetr/rivetr.toml
sudo systemctl restart rivetr
```

### Disk space issues

```bash
# Check disk usage
df -h

# Clean Docker
docker system prune -a

# Check Rivetr data
du -sh /var/lib/rivetr/*
```

## Uninstall

```bash
# Stop and disable service
sudo systemctl stop rivetr
sudo systemctl disable rivetr

# Remove files
sudo rm /etc/systemd/system/rivetr.service
sudo rm -rf /opt/rivetr

# Optionally remove data
sudo rm -rf /var/lib/rivetr

# Reload systemd
sudo systemctl daemon-reload
```

## Support

- GitHub Issues: https://github.com/KwaminaWhyte/rivetr/issues
- Documentation: https://docs.rivetr.io
