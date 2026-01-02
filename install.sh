#!/bin/bash
# Rivetr Production Install Script
# Usage: curl -fsSL https://get.rivetr.io | sudo bash
#
# This script:
# - Installs Docker if not present
# - Downloads and installs Rivetr binary
# - Creates systemd service for auto-restart
# - Configures container restart policies
#
# Tested on: Ubuntu 22.04, Debian 12, Fedora 39

set -e

# =============================================================================
# Configuration
# =============================================================================
RIVETR_VERSION="${RIVETR_VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-/opt/rivetr}"
DATA_DIR="${DATA_DIR:-/var/lib/rivetr}"
CONFIG_FILE="$INSTALL_DIR/rivetr.toml"
BINARY_URL="https://github.com/KwaminaWhyte/rivetr/releases/download/${RIVETR_VERSION}/rivetr-linux-amd64"
SERVICE_USER="rivetr"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# =============================================================================
# Helper Functions
# =============================================================================
info() { echo -e "${CYAN}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

command_exists() { command -v "$1" >/dev/null 2>&1; }

check_root() {
    if [ "$EUID" -ne 0 ]; then
        error "This script must be run as root. Use: sudo bash install.sh"
    fi
}

detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        OS_VERSION=$VERSION_ID
    elif command_exists lsb_release; then
        OS=$(lsb_release -si | tr '[:upper:]' '[:lower:]')
        OS_VERSION=$(lsb_release -sr)
    else
        error "Unable to detect OS. Please install manually."
    fi

    case "$OS" in
        ubuntu|debian|fedora|centos|rhel|rocky|almalinux)
            success "Detected OS: $OS $OS_VERSION"
            ;;
        *)
            warn "Untested OS: $OS. Proceeding anyway..."
            ;;
    esac
}

detect_arch() {
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64|amd64)
            ARCH="amd64"
            ;;
        aarch64|arm64)
            ARCH="arm64"
            ;;
        *)
            error "Unsupported architecture: $ARCH"
            ;;
    esac
    success "Detected architecture: $ARCH"
}

# =============================================================================
# Docker Installation
# =============================================================================
install_docker() {
    if command_exists docker; then
        success "Docker is already installed"
        return
    fi

    info "Installing Docker..."

    case "$OS" in
        ubuntu|debian)
            # Install prerequisites
            apt-get update -qq
            apt-get install -y -qq ca-certificates curl gnupg

            # Add Docker's official GPG key
            install -m 0755 -d /etc/apt/keyrings
            curl -fsSL "https://download.docker.com/linux/$OS/gpg" | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
            chmod a+r /etc/apt/keyrings/docker.gpg

            # Add the repository
            echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/$OS $(. /etc/os-release && echo "$VERSION_CODENAME") stable" > /etc/apt/sources.list.d/docker.list

            # Install Docker
            apt-get update -qq
            apt-get install -y -qq docker-ce docker-ce-cli containerd.io docker-compose-plugin
            ;;
        fedora)
            dnf install -y dnf-plugins-core
            dnf config-manager --add-repo https://download.docker.com/linux/fedora/docker-ce.repo
            dnf install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
            ;;
        centos|rhel|rocky|almalinux)
            yum install -y yum-utils
            yum-config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo
            yum install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
            ;;
        *)
            error "Cannot auto-install Docker on $OS. Please install manually and re-run."
            ;;
    esac

    # Start and enable Docker
    systemctl start docker
    systemctl enable docker

    success "Docker installed and started"
}

# =============================================================================
# Build Tools Installation (Nixpacks, Railpack, Pack CLI)
# =============================================================================
install_git() {
    if command_exists git; then
        success "Git is already installed"
        return
    fi

    info "Installing Git..."
    case "$OS" in
        ubuntu|debian)
            apt-get install -y -qq git
            ;;
        fedora)
            dnf install -y git
            ;;
        centos|rhel|rocky|almalinux)
            yum install -y git
            ;;
    esac
    success "Git installed"
}

install_nixpacks() {
    if command_exists nixpacks; then
        success "Nixpacks is already installed: $(nixpacks --version)"
        return
    fi

    info "Installing Nixpacks..."

    # Try the official installer first
    if curl -sSL https://nixpacks.com/install.sh | bash 2>/dev/null; then
        # Move to /usr/local/bin if installed elsewhere
        if [ -f "$HOME/.nixpacks/bin/nixpacks" ]; then
            mv "$HOME/.nixpacks/bin/nixpacks" /usr/local/bin/
            chmod +x /usr/local/bin/nixpacks
        fi
        if command_exists nixpacks; then
            success "Nixpacks installed: $(nixpacks --version 2>/dev/null || echo 'installed')"
            return
        fi
    fi

    # Fallback: Download binary directly from GitHub
    info "Trying direct binary download..."
    local NIXPACKS_VERSION="1.29.1"
    local NIXPACKS_URL="https://github.com/railwayapp/nixpacks/releases/download/v${NIXPACKS_VERSION}/nixpacks-v${NIXPACKS_VERSION}-${ARCH}-unknown-linux-musl.tar.gz"

    local TEMP_DIR=$(mktemp -d)
    if curl -fsSL -o "$TEMP_DIR/nixpacks.tar.gz" "$NIXPACKS_URL" 2>/dev/null; then
        tar -xzf "$TEMP_DIR/nixpacks.tar.gz" -C "$TEMP_DIR"
        if [ -f "$TEMP_DIR/nixpacks" ]; then
            mv "$TEMP_DIR/nixpacks" /usr/local/bin/
            chmod +x /usr/local/bin/nixpacks
            success "Nixpacks installed from GitHub releases"
        fi
    fi
    rm -rf "$TEMP_DIR"

    if ! command_exists nixpacks; then
        error "Failed to install Nixpacks. Please check your internet connection."
    fi
}

install_railpack() {
    if command_exists railpack; then
        success "Railpack is already installed: $(railpack --version)"
        return
    fi

    info "Installing Railpack..."

    # Try downloading binary directly from GitHub releases
    local RAILPACK_ARCH="$ARCH"
    if [ "$ARCH" = "amd64" ]; then
        RAILPACK_ARCH="x86_64"
    elif [ "$ARCH" = "arm64" ]; then
        RAILPACK_ARCH="aarch64"
    fi

    # Try latest release binary
    local RAILPACK_URL="https://github.com/railwayapp/railpack/releases/latest/download/railpack-${RAILPACK_ARCH}-unknown-linux-gnu.tar.gz"
    local TEMP_DIR=$(mktemp -d)

    if curl -fsSL -o "$TEMP_DIR/railpack.tar.gz" "$RAILPACK_URL" 2>/dev/null; then
        if tar -xzf "$TEMP_DIR/railpack.tar.gz" -C "$TEMP_DIR" 2>/dev/null; then
            if [ -f "$TEMP_DIR/railpack" ]; then
                mv "$TEMP_DIR/railpack" /usr/local/bin/
                chmod +x /usr/local/bin/railpack
                rm -rf "$TEMP_DIR"
                success "Railpack installed from GitHub releases"
                return
            fi
        fi
    fi
    rm -rf "$TEMP_DIR"

    # Try alternative URL format (plain binary)
    RAILPACK_URL="https://github.com/railwayapp/railpack/releases/latest/download/railpack-linux-$ARCH"
    if curl -fsSL -o /usr/local/bin/railpack "$RAILPACK_URL" 2>/dev/null; then
        chmod +x /usr/local/bin/railpack
        if /usr/local/bin/railpack --version >/dev/null 2>&1; then
            success "Railpack installed from GitHub releases"
            return
        else
            rm -f /usr/local/bin/railpack
        fi
    fi

    # Try mise if available
    if command_exists mise; then
        if mise install ubi:railwayapp/railpack@latest 2>/dev/null; then
            success "Railpack installed via mise"
            return
        fi
    fi

    # Try cargo install as last resort
    if command_exists cargo; then
        info "Attempting to build Railpack from source (this may take a while)..."
        if cargo install railpack 2>/dev/null; then
            # Move to /usr/local/bin
            if [ -f "$HOME/.cargo/bin/railpack" ]; then
                mv "$HOME/.cargo/bin/railpack" /usr/local/bin/
            fi
            success "Railpack installed via cargo"
            return
        fi
    fi

    warn "Railpack installation failed. Railpack builds will not be available."
    warn "This is optional - Nixpacks and Dockerfile builds will still work."
}

install_pack_cli() {
    if command_exists pack; then
        success "Pack CLI is already installed: $(pack version)"
        return
    fi

    info "Installing Pack CLI (Cloud Native Buildpacks)..."

    local TEMP_DIR=$(mktemp -d)
    local INSTALLED=false

    # Try version 0.36.1 first (latest as of 2025)
    for PACK_VERSION in "0.36.1" "0.35.1" "0.34.2"; do
        local PACK_URL="https://github.com/buildpacks/pack/releases/download/v${PACK_VERSION}/pack-v${PACK_VERSION}-linux"

        if [ "$ARCH" = "amd64" ]; then
            PACK_URL="${PACK_URL}.tgz"
        else
            PACK_URL="${PACK_URL}-arm64.tgz"
        fi

        if curl -fsSL -o "$TEMP_DIR/pack.tgz" "$PACK_URL" 2>/dev/null; then
            if tar -xzf "$TEMP_DIR/pack.tgz" -C "$TEMP_DIR" 2>/dev/null; then
                if [ -f "$TEMP_DIR/pack" ]; then
                    mv "$TEMP_DIR/pack" /usr/local/bin/
                    chmod +x /usr/local/bin/pack
                    INSTALLED=true
                    success "Pack CLI v${PACK_VERSION} installed"
                    break
                fi
            fi
        fi
    done

    rm -rf "$TEMP_DIR"

    if [ "$INSTALLED" = false ]; then
        # Try brew as fallback on systems with Homebrew
        if command_exists brew; then
            if brew install buildpacks/tap/pack 2>/dev/null; then
                success "Pack CLI installed via Homebrew"
                return
            fi
        fi

        warn "Pack CLI installation failed. CNB/Buildpacks builds will not be available."
        warn "This is optional - Nixpacks, Railpack, and Dockerfile builds will still work."
    fi
}

install_build_tools() {
    info "Installing build tools..."

    install_git
    install_nixpacks
    install_railpack
    install_pack_cli

    echo ""
    info "Build tools installation summary:"
    command_exists git && success "  Git: $(git --version)" || warn "  Git: Not installed"
    command_exists nixpacks && success "  Nixpacks: $(nixpacks --version 2>/dev/null)" || warn "  Nixpacks: Not installed"
    command_exists railpack && success "  Railpack: $(railpack --version 2>/dev/null)" || warn "  Railpack: Not installed"
    command_exists pack && success "  Pack CLI: $(pack version 2>/dev/null)" || warn "  Pack CLI: Not installed"
}

# =============================================================================
# Rivetr Installation
# =============================================================================
create_user() {
    if id "$SERVICE_USER" &>/dev/null; then
        success "User $SERVICE_USER already exists"
    else
        info "Creating service user: $SERVICE_USER"
        useradd --system --no-create-home --shell /bin/false "$SERVICE_USER"
        usermod -aG docker "$SERVICE_USER"
        success "Created user $SERVICE_USER with docker access"
    fi
}

create_directories() {
    info "Creating directories..."

    mkdir -p "$INSTALL_DIR"
    mkdir -p "$DATA_DIR"
    mkdir -p "$DATA_DIR/backups"
    mkdir -p "$DATA_DIR/volumes"
    mkdir -p "$DATA_DIR/certs"

    # Set ownership
    chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"

    success "Created directories"
}

download_binary() {
    info "Downloading Rivetr binary..."

    # For now, we'll build from source if no binary available
    # In production, this would download from releases
    local BINARY_PATH="$INSTALL_DIR/rivetr"

    # Check if we have a local binary (for testing)
    if [ -f "./target/release/rivetr" ]; then
        cp "./target/release/rivetr" "$BINARY_PATH"
        success "Copied local binary"
    elif [ -n "$RIVETR_BINARY_PATH" ] && [ -f "$RIVETR_BINARY_PATH" ]; then
        cp "$RIVETR_BINARY_PATH" "$BINARY_PATH"
        success "Copied binary from $RIVETR_BINARY_PATH"
    else
        # Try to download from releases
        if curl -fsSL -o "$BINARY_PATH" "$BINARY_URL" 2>/dev/null; then
            success "Downloaded binary from GitHub releases"
        else
            warn "Cannot download binary. Building from source..."
            build_from_source
            return
        fi
    fi

    chmod +x "$BINARY_PATH"
    chown "$SERVICE_USER:$SERVICE_USER" "$BINARY_PATH"
}

build_from_source() {
    info "Building Rivetr from source..."

    # Install Rust if needed
    if ! command_exists cargo; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi

    # Clone and build
    local BUILD_DIR="/tmp/rivetr-build"
    rm -rf "$BUILD_DIR"
    git clone --depth 1 https://github.com/KwaminaWhyte/rivetr.git "$BUILD_DIR"
    cd "$BUILD_DIR"
    cargo build --release

    cp target/release/rivetr "$INSTALL_DIR/rivetr"
    chmod +x "$INSTALL_DIR/rivetr"
    chown "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR/rivetr"

    # Cleanup
    rm -rf "$BUILD_DIR"

    success "Built from source"
}

create_config() {
    if [ -f "$CONFIG_FILE" ]; then
        warn "Config file exists. Keeping existing configuration."
        return
    fi

    info "Creating configuration..."

    # Generate a secure admin token
    ADMIN_TOKEN=$(openssl rand -hex 32)

    cat > "$CONFIG_FILE" << EOF
# Rivetr Configuration
# Generated by install script on $(date)

[server]
host = "0.0.0.0"
api_port = 8080
proxy_port = 80
data_dir = "$DATA_DIR"

[auth]
# Admin API token - keep this secret!
# You can also log in through the web UI
admin_token = "$ADMIN_TOKEN"
session_lifetime_hours = 168  # 7 days

[logging]
level = "info"

[runtime]
# Container runtime: "docker" or "podman"
runtime = "docker"
# Build resource limits
build_cpu_limit = "2"
build_memory_limit = "2g"

[proxy]
# Health check settings
health_check_interval = 30
health_check_timeout = 10

[cleanup]
# Auto-cleanup old deployments
enabled = true
max_deployments_per_app = 10
cleanup_interval_hours = 24

[rate_limit]
# API rate limiting (requests per minute)
api_requests_per_window = 1000
webhook_requests_per_window = 200
auth_requests_per_window = 20
window_seconds = 60

[disk_monitor]
# Disk space monitoring
enabled = true
warning_threshold_percent = 80
critical_threshold_percent = 90
check_interval_seconds = 300

[container_monitor]
# Auto-restart crashed containers
enabled = true
check_interval_seconds = 60
max_restart_attempts = 3
restart_window_seconds = 300

[database_backup]
# Automatic database backups
enabled = true
check_interval_seconds = 3600
retention_days = 30
EOF

    chmod 600 "$CONFIG_FILE"
    chown "$SERVICE_USER:$SERVICE_USER" "$CONFIG_FILE"

    success "Created configuration"
    echo ""
    info "Admin token saved to: $CONFIG_FILE"
    info "Token: $ADMIN_TOKEN"
    echo ""
}

# =============================================================================
# Systemd Service
# =============================================================================
create_systemd_service() {
    info "Creating systemd service..."

    cat > /etc/systemd/system/rivetr.service << EOF
[Unit]
Description=Rivetr Deployment Engine
Documentation=https://github.com/KwaminaWhyte/rivetr
After=network-online.target docker.service
Wants=network-online.target
Requires=docker.service

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/rivetr --config $CONFIG_FILE
Restart=always
RestartSec=10

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$DATA_DIR
PrivateTmp=true

# Environment
Environment=RUST_LOG=info

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=rivetr

[Install]
WantedBy=multi-user.target
EOF

    # Reload systemd
    systemctl daemon-reload

    # Enable and start service
    systemctl enable rivetr

    success "Created systemd service"
}

# =============================================================================
# Docker Configuration
# =============================================================================
configure_docker_restart() {
    info "Configuring Docker restart policies..."

    # Create daemon.json if it doesn't exist
    DOCKER_CONFIG="/etc/docker/daemon.json"

    if [ ! -f "$DOCKER_CONFIG" ]; then
        cat > "$DOCKER_CONFIG" << 'EOF'
{
    "live-restore": true,
    "log-driver": "json-file",
    "log-opts": {
        "max-size": "10m",
        "max-file": "3"
    }
}
EOF
        systemctl restart docker
        success "Configured Docker daemon"
    else
        success "Docker daemon already configured"
    fi
}

# =============================================================================
# Firewall Configuration
# =============================================================================
configure_firewall() {
    info "Configuring firewall..."

    if command_exists ufw; then
        ufw allow 80/tcp comment 'Rivetr HTTP'
        ufw allow 443/tcp comment 'Rivetr HTTPS'
        ufw allow 8080/tcp comment 'Rivetr API'
        success "Configured UFW firewall"
    elif command_exists firewall-cmd; then
        firewall-cmd --permanent --add-port=80/tcp
        firewall-cmd --permanent --add-port=443/tcp
        firewall-cmd --permanent --add-port=8080/tcp
        firewall-cmd --reload
        success "Configured firewalld"
    else
        warn "No firewall detected. Please manually open ports 80, 443, and 8080."
    fi
}

# =============================================================================
# Main Installation
# =============================================================================
print_banner() {
    echo ""
    echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}                                                               ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}    ${CYAN}██████╗ ██╗██╗   ██╗███████╗████████╗██████╗${NC}              ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}    ${CYAN}██╔══██╗██║██║   ██║██╔════╝╚══██╔══╝██╔══██╗${NC}             ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}    ${CYAN}██████╔╝██║██║   ██║█████╗     ██║   ██████╔╝${NC}             ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}    ${CYAN}██╔══██╗██║╚██╗ ██╔╝██╔══╝     ██║   ██╔══██╗${NC}             ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}    ${CYAN}██║  ██║██║ ╚████╔╝ ███████╗   ██║   ██║  ██║${NC}             ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}    ${CYAN}╚═╝  ╚═╝╚═╝  ╚═══╝  ╚══════╝   ╚═╝   ╚═╝  ╚═╝${NC}             ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}                                                               ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}    ${GREEN}Fast, Lightweight Deployment Engine${NC}                       ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}                                                               ${BLUE}║${NC}"
    echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

print_summary() {
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  Installation Complete!${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "  ${CYAN}Rivetr is now installed and running!${NC}"
    echo ""
    echo -e "  ${YELLOW}Web Dashboard:${NC}"
    echo -e "    http://$(hostname -I | awk '{print $1}'):8080"
    echo -e "    http://localhost:8080"
    echo ""
    echo -e "  ${YELLOW}Service Management:${NC}"
    echo -e "    Start:   ${CYAN}sudo systemctl start rivetr${NC}"
    echo -e "    Stop:    ${CYAN}sudo systemctl stop rivetr${NC}"
    echo -e "    Status:  ${CYAN}sudo systemctl status rivetr${NC}"
    echo -e "    Logs:    ${CYAN}sudo journalctl -u rivetr -f${NC}"
    echo ""
    echo -e "  ${YELLOW}Configuration:${NC}"
    echo -e "    Config:  $CONFIG_FILE"
    echo -e "    Data:    $DATA_DIR"
    echo ""
    echo -e "  ${YELLOW}Admin Token (for API access):${NC}"
    echo -e "    ${CYAN}$ADMIN_TOKEN${NC}"
    echo ""
    echo -e "  ${YELLOW}Next Steps:${NC}"
    echo -e "    1. Visit the web dashboard to create your admin account"
    echo -e "    2. Add your first application"
    echo -e "    3. Configure your domain and SSL certificates"
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
}

main() {
    print_banner

    info "Starting Rivetr installation..."
    echo ""

    # Pre-flight checks
    check_root
    detect_os
    detect_arch

    # Installation steps
    echo ""
    info "Step 1/8: Installing Docker..."
    install_docker

    echo ""
    info "Step 2/8: Installing build tools (Nixpacks, Railpack, Pack CLI)..."
    install_build_tools

    echo ""
    info "Step 3/8: Creating service user..."
    create_user

    echo ""
    info "Step 4/8: Creating directories..."
    create_directories

    echo ""
    info "Step 5/8: Downloading/building Rivetr..."
    download_binary

    echo ""
    info "Step 6/8: Creating configuration..."
    create_config

    echo ""
    info "Step 7/8: Setting up systemd service..."
    create_systemd_service
    configure_docker_restart

    echo ""
    info "Step 8/8: Configuring firewall..."
    configure_firewall

    # Start the service
    echo ""
    info "Starting Rivetr..."
    systemctl start rivetr

    # Wait a moment for startup
    sleep 3

    # Check if running
    if systemctl is-active --quiet rivetr; then
        success "Rivetr is running!"
    else
        warn "Rivetr may not have started correctly. Check logs with: journalctl -u rivetr"
    fi

    print_summary
}

# Run installation
main "$@"
