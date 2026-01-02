#!/bin/bash
# Rivetr Setup Script for Linux/macOS
# This script sets up everything needed to run Rivetr

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}========================================"
echo "  Rivetr Setup Script"
echo -e "========================================${NC}"
echo ""

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
echo -e "${YELLOW}[1/5] Checking prerequisites...${NC}"

# Check for Rust
if command_exists cargo; then
    RUST_VERSION=$(rustc --version)
    echo -e "  ${GREEN}✓ Rust is installed: $RUST_VERSION${NC}"
else
    echo -e "  ${RED}✗ Rust is not installed${NC}"
    echo -e "    Please install Rust from https://rustup.rs"
    echo -e "    Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check for Git
if command_exists git; then
    GIT_VERSION=$(git --version)
    echo -e "  ${GREEN}✓ Git is installed: $GIT_VERSION${NC}"
else
    echo -e "  ${RED}✗ Git is not installed${NC}"
    echo -e "    Please install Git:"
    echo -e "    - Ubuntu/Debian: sudo apt install git"
    echo -e "    - macOS: brew install git"
    exit 1
fi

# Check for Docker or Podman
HAS_DOCKER=false
HAS_PODMAN=false

if command_exists docker; then
    HAS_DOCKER=true
    echo -e "  ${GREEN}✓ Docker is installed${NC}"
    # Check if Docker daemon is running
    if docker info >/dev/null 2>&1; then
        echo -e "  ${GREEN}✓ Docker daemon is running${NC}"
    else
        echo -e "  ${YELLOW}⚠ Docker is installed but not running${NC}"
        echo -e "    Please start Docker daemon: sudo systemctl start docker"
    fi
elif command_exists podman; then
    HAS_PODMAN=true
    echo -e "  ${GREEN}✓ Podman is installed${NC}"
else
    echo -e "  ${YELLOW}⚠ No container runtime found (Docker or Podman)${NC}"
    echo -e "    Rivetr will start but deployments won't work"
    echo -e "    Install Docker: https://docs.docker.com/get-docker/"
    echo -e "    Or Podman: https://podman.io/getting-started/installation"
fi

echo ""

# Check and install build tools
echo -e "${YELLOW}[2/7] Checking build tools...${NC}"

# Check for Nixpacks
if command_exists nixpacks; then
    echo -e "  ${GREEN}✓ Nixpacks is installed: $(nixpacks --version 2>/dev/null)${NC}"
else
    echo -e "  ${YELLOW}⚠ Nixpacks not found - attempting installation...${NC}"
    if curl -sSL https://nixpacks.com/install.sh | bash 2>/dev/null; then
        # Add to PATH if installed in home directory
        if [ -f "$HOME/.nixpacks/bin/nixpacks" ]; then
            export PATH="$HOME/.nixpacks/bin:$PATH"
        fi
        if command_exists nixpacks; then
            echo -e "  ${GREEN}✓ Nixpacks installed successfully${NC}"
        else
            echo -e "  ${YELLOW}⚠ Nixpacks installed but may need PATH update. Add to your shell profile:${NC}"
            echo -e "      export PATH=\"\$HOME/.nixpacks/bin:\$PATH\""
        fi
    else
        echo -e "  ${YELLOW}⚠ Could not install Nixpacks automatically${NC}"
        echo -e "      Nixpacks builds will not work until installed"
    fi
fi

# Check for Railpack
if command_exists railpack; then
    echo -e "  ${GREEN}✓ Railpack is installed: $(railpack --version 2>/dev/null)${NC}"
else
    echo -e "  ${YELLOW}⚠ Railpack not found (optional - Railway's next-gen builder)${NC}"
    echo -e "      Install with: cargo install railpack"
fi

# Check for Pack CLI (Cloud Native Buildpacks)
if command_exists pack; then
    echo -e "  ${GREEN}✓ Pack CLI is installed: $(pack version 2>/dev/null)${NC}"
else
    echo -e "  ${YELLOW}⚠ Pack CLI not found (optional - for CNB/Heroku buildpacks)${NC}"
    echo -e "      Install from: https://buildpacks.io/docs/tools/pack/"
fi

echo ""

# Create data directory
echo -e "${YELLOW}[3/7] Creating data directory...${NC}"
DATA_DIR="$PROJECT_ROOT/data"
if [ ! -d "$DATA_DIR" ]; then
    mkdir -p "$DATA_DIR"
    echo -e "  ${GREEN}✓ Created data directory: $DATA_DIR${NC}"
else
    echo -e "  ${GREEN}✓ Data directory already exists${NC}"
fi

echo ""

# Create config file if not exists
echo -e "${YELLOW}[4/7] Setting up configuration...${NC}"
CONFIG_FILE="$PROJECT_ROOT/rivetr.toml"
EXAMPLE_CONFIG="$PROJECT_ROOT/rivetr.example.toml"

if [ ! -f "$CONFIG_FILE" ]; then
    if [ -f "$EXAMPLE_CONFIG" ]; then
        cp "$EXAMPLE_CONFIG" "$CONFIG_FILE"
        echo -e "  ${GREEN}✓ Created rivetr.toml from example config${NC}"
        echo -e "  ${YELLOW}⚠ Please edit rivetr.toml to customize settings${NC}"
    else
        echo -e "  ${RED}✗ Example config not found${NC}"
        exit 1
    fi
else
    echo -e "  ${GREEN}✓ Configuration file already exists${NC}"
fi

echo ""

# Build the backend
echo -e "${YELLOW}[5/7] Building Rivetr backend...${NC}"
cd "$PROJECT_ROOT"

echo -e "  ${CYAN}Building in release mode (this may take a few minutes)...${NC}"
if cargo build --release; then
    echo -e "  ${GREEN}✓ Build successful${NC}"
else
    echo -e "  ${RED}✗ Build failed${NC}"
    exit 1
fi

echo ""

# Build the frontend
echo -e "${YELLOW}[6/7] Building frontend...${NC}"
FRONTEND_DIR="$PROJECT_ROOT/frontend"

if [ -d "$FRONTEND_DIR" ]; then
    cd "$FRONTEND_DIR"

    # Check for Node.js
    if command_exists node; then
        echo -e "  ${GREEN}✓ Node.js is installed: $(node --version)${NC}"

        # Install dependencies if needed
        if [ ! -d "node_modules" ]; then
            echo -e "  ${CYAN}Installing frontend dependencies...${NC}"
            if command_exists npm; then
                npm install
            elif command_exists pnpm; then
                pnpm install
            fi
        fi

        # Build frontend
        echo -e "  ${CYAN}Building frontend assets...${NC}"
        if npm run build 2>/dev/null || pnpm run build 2>/dev/null; then
            echo -e "  ${GREEN}✓ Frontend built successfully${NC}"
        else
            echo -e "  ${YELLOW}⚠ Frontend build failed - web UI may not work${NC}"
        fi
    else
        echo -e "  ${YELLOW}⚠ Node.js not found - skipping frontend build${NC}"
        echo -e "      Install Node.js from: https://nodejs.org/"
    fi

    cd "$PROJECT_ROOT"
else
    echo -e "  ${YELLOW}⚠ Frontend directory not found - skipping${NC}"
fi

echo ""

# Print success message
echo -e "${YELLOW}[7/7] Setup complete!${NC}"
echo ""
echo -e "${CYAN}========================================"
echo "  Rivetr is ready to use!"
echo -e "========================================${NC}"
echo ""
echo -e "To start Rivetr:"
echo -e "  ${YELLOW}./target/release/rivetr --config rivetr.toml${NC}"
echo ""
echo -e "Or for development:"
echo -e "  ${YELLOW}cargo run -- --config rivetr.example.toml${NC}"
echo ""
echo -e "Then open http://localhost:8080 in your browser"
echo -e "You'll be prompted to create your admin account on first visit."
echo ""

# Ask if user wants to start now
read -p "Would you like to start Rivetr now? (y/n) " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo -e "${CYAN}Starting Rivetr...${NC}"
    "$PROJECT_ROOT/target/release/rivetr" --config "$PROJECT_ROOT/rivetr.toml"
fi
