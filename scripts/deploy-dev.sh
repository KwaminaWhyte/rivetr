#!/usr/bin/env bash
# Fast development deploy: cross-compile locally, push binary + frontend to server
# Usage: ./scripts/deploy-dev.sh [--frontend-only | --backend-only]
set -e

SERVER="root@46.101.187.233"
REMOTE_BIN="/opt/rivetr/rivetr"
TARGET="x86_64-unknown-linux-gnu"

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
  echo "→ Cross-compiling Rust binary for Linux x86_64..."
  cargo zigbuild --release --target "$TARGET" --features tui 2>&1 | grep -E "Compiling rivetr|Finished|error"
  echo "  ✓ Binary compiled: $(du -sh target/$TARGET/release/rivetr | cut -f1)"
fi

echo "→ Deploying to $SERVER..."

if [ "$FRONTEND_ONLY" = false ]; then
  # Upload the new binary to a temporary path while the proxy is still running.
  # This avoids ~30 seconds of downtime caused by uploading over a stopped service.
  echo "  → Uploading binary (proxy remains live during transfer)..."
  scp "target/$TARGET/release/rivetr" "$SERVER:/opt/rivetr/rivetr.new"
fi

# Ensure the systemd socket unit is installed (idempotent — safe to run every deploy).
# The socket unit keeps ports 80 and 443 open in the kernel during service restarts,
# so connections queue instead of getting ECONNREFUSED.
echo "  → Installing systemd socket unit (zero-downtime socket activation)..."
scp "deploy/rivetr-proxy.socket" "$SERVER:/etc/systemd/system/rivetr-proxy.socket"
ssh "$SERVER" "systemctl daemon-reload && systemctl enable rivetr-proxy.socket && systemctl start rivetr-proxy.socket 2>/dev/null || true"

# Frontend is embedded in the binary (rust_embed) - but if we want to swap just frontend,
# we'd need a separate assets dir. For now, always sync via binary.

if [ "$FRONTEND_ONLY" = false ]; then
  # Atomic swap + restart: stop → mv (instant) → start.
  # Total downtime is now ~2s (service restart) rather than ~30s (binary transfer time).
  # With the socket unit active, kernel-queued connections survive even the 2s gap.
  ssh "$SERVER" "mv /opt/rivetr/rivetr.new /opt/rivetr/rivetr && systemctl restart rivetr && sleep 2 && systemctl is-active rivetr"
else
  ssh "$SERVER" "systemctl restart rivetr && sleep 2 && systemctl is-active rivetr"
fi
echo "✓ Deployed and running!"
