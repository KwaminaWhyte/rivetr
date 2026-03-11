#!/usr/bin/env bash
# Fast development deploy: cross-compile locally, push binary + frontend to server
# Usage: ./scripts/deploy-dev.sh [--frontend-only | --backend-only]
set -e

SERVER="root@64.226.112.14"
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
  cargo zigbuild --release --target "$TARGET" 2>&1 | grep -E "Compiling rivetr|Finished|error"
  echo "  ✓ Binary compiled: $(du -sh target/$TARGET/release/rivetr | cut -f1)"
fi

echo "→ Deploying to $SERVER..."
ssh "$SERVER" "systemctl stop rivetr"

if [ "$FRONTEND_ONLY" = false ]; then
  scp "target/$TARGET/release/rivetr" "$SERVER:$REMOTE_BIN"
fi

# Frontend is embedded in the binary (rust_embed) - but if we want to swap just frontend,
# we'd need a separate assets dir. For now, always sync via binary.

ssh "$SERVER" "systemctl start rivetr && sleep 2 && systemctl is-active rivetr"
echo "✓ Deployed and running!"
