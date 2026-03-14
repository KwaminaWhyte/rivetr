# Rivetr Ansible Playbook

Automates the full installation of [Rivetr](https://github.com/KwaminaWhyte/rivetr) on a fresh Ubuntu 22.04/24.04 or Debian 12 server.

**What it does:**

- Installs Docker CE (required for running deployed apps)
- Creates `/opt/rivetr/` and `/var/lib/rivetr/` directory structure
- Downloads the Rivetr binary from GitHub releases
- Writes a `rivetr.toml` config from your variables
- Installs and enables a systemd service (`rivetr.service`)
- Configures UFW firewall (SSH, 80, 443, 8080)
- Optionally installs Nginx as a TLS front-proxy

## Requirements

On your **control machine** (laptop / CI):

```bash
pip install ansible
# The community.general collection provides the ufw module:
ansible-galaxy collection install community.general
```

Your server must be reachable via SSH as `root` (or a user with `sudo`).

## Quick Start

### 1. Create your inventory

```bash
cp inventory.example inventory
# Edit inventory and replace "your-server-ip" with your server's IP/hostname
```

### 2. Set your variables

```bash
cp group_vars/rivetr.yml.example group_vars/rivetr.yml
# Edit group_vars/rivetr.yml — at minimum change:
#   rivetr_domain, rivetr_admin_email, rivetr_admin_password, rivetr_jwt_secret
```

Generate a secure JWT secret:

```bash
openssl rand -hex 32
```

### 3. Run the playbook

```bash
ansible-playbook -i inventory rivetr.yml
```

The playbook is **idempotent** — safe to re-run after upgrades or config changes.

## Targeting specific tasks

Use Ansible tags to run only part of the playbook:

```bash
# Install/update only the Rivetr binary and config
ansible-playbook -i inventory rivetr.yml --tags rivetr

# Apply firewall rules only
ansible-playbook -i inventory rivetr.yml --tags firewall

# Install Docker only
ansible-playbook -i inventory rivetr.yml --tags docker

# Skip the Nginx tasks
ansible-playbook -i inventory rivetr.yml --skip-tags nginx
```

Available tags: `setup`, `docker`, `rivetr`, `firewall`, `nginx`

## Updating Rivetr

Re-run the playbook. Because `rivetr_version: "latest"` sets `force: true` on
the download task, the binary is always refreshed:

```bash
ansible-playbook -i inventory rivetr.yml --tags rivetr
```

To pin a specific version, set `rivetr_version: "v0.10.4"` in your vars file.

## Optional: Nginx + HTTPS

Set `install_nginx: true` in `group_vars/rivetr.yml`, then run the playbook.
After the playbook finishes, obtain a certificate with certbot:

```bash
apt install certbot python3-certbot-nginx
certbot --nginx -d yourdomain.com
```

Then uncomment the HTTPS server block in `/etc/nginx/sites-available/rivetr`
and reload Nginx:

```bash
systemctl reload nginx
```

## Variable reference

| Variable | Default | Description |
|---|---|---|
| `rivetr_domain` | `yourdomain.com` | Primary domain for the dashboard |
| `rivetr_admin_email` | `admin@yourdomain.com` | Admin login email |
| `rivetr_admin_password` | `changeme123` | Admin login password |
| `rivetr_jwt_secret` | *(placeholder)* | JWT signing secret (min 32 chars) |
| `rivetr_version` | `latest` | Binary version or `latest` |
| `rivetr_data_dir` | `/var/lib/rivetr` | Persistent data directory |
| `rivetr_install_dir` | `/opt/rivetr` | Binary and config directory |
| `rivetr_port` | `8080` | Rivetr dashboard/API port |
| `rivetr_http_port` | `80` | Reverse proxy HTTP port |
| `rivetr_https_port` | `443` | Reverse proxy HTTPS port |
| `install_nginx` | `false` | Install Nginx as a front-proxy |

## Post-install checks

```bash
# On the server:
systemctl status rivetr          # should show "active (running)"
journalctl -u rivetr -f          # follow logs

# From your machine:
curl http://<server-ip>:8080/health
```

The dashboard is available at `http://<server-ip>:8080` (or your domain once DNS is configured).
