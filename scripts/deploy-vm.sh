#!/usr/bin/env bash
# Fast LOCAL VM deploy: cross-compile aarch64-linux, push binary to Parallels Ubuntu VM
# Usage: ./scripts/deploy-vm.sh [--frontend-only | --backend-only]
#
# VM: parallels@10.211.55.5 (Ubuntu 24.04 ARM64, Parallels Desktop)
# Binary lands at /opt/rivetr/rivetr (managed by systemd unit `rivetr`)
set -e

VM="parallels@10.211.55.5"
REMOTE_BIN="/opt/rivetr/rivetr"
TARGET="aarch64-unknown-linux-gnu"

FRONTEND_ONLY=false
BACKEND_ONLY=false
for arg in "$@"; do
  case $arg in
    --frontend-only) FRONTEND_ONLY=true ;;
    --backend-only) BACKEND_ONLY=true ;;
  esac
done

if [ "$BACKEND_ONLY" = false ]; then
  echo "→ Building frontend..."
  cd "$(dirname "$0")/../frontend"
  npm run build --silent
  cd ..
  echo "  ✓ Frontend built"
fi

if [ "$FRONTEND_ONLY" = false ]; then
  echo "→ Cross-compiling Rust binary for Linux ARM64 (zigbuild)..."
  cargo zigbuild --release --target "$TARGET" --features tui 2>&1 | grep -E "Compiling rivetr|Finished|error" || true
  echo "  ✓ Binary compiled: $(du -sh "target/$TARGET/release/rivetr" | cut -f1)"
fi

echo "→ Deploying to $VM..."

if [ "$FRONTEND_ONLY" = false ]; then
  echo "  → Uploading binary..."
  scp "target/$TARGET/release/rivetr" "$VM:/tmp/rivetr.new"
  ssh "$VM" "sudo mv /tmp/rivetr.new $REMOTE_BIN && sudo chmod +x $REMOTE_BIN && sudo systemctl restart rivetr && sleep 2 && systemctl is-active rivetr"
else
  ssh "$VM" "sudo systemctl restart rivetr && sleep 2 && systemctl is-active rivetr"
fi

echo "✓ Deployed to VM!"
echo "  Dashboard: http://10.211.55.5:8080"
