#!/usr/bin/env bash
set -euo pipefail

# ============================================================
#  Setup iniziale: builda Caddy con caddy-security
# ============================================================

BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
CADDY_BIN="$BASE_DIR/caddy"

# Verifica che xcaddy sia installato
if ! command -v xcaddy &>/dev/null; then
    echo "[*] Installazione xcaddy..."
    go install github.com/caddyserver/xcaddy/cmd/xcaddy@latest
fi

export PATH="$PATH:$(go env GOPATH)/bin"

echo "[*] Building Caddy con caddy-security..."
xcaddy build --with github.com/greenpau/caddy-security --output "$CADDY_BIN"

echo "[OK] Caddy binary: $CADDY_BIN"
echo ""
echo "Prossimi passi:"
echo "  1. cp .env.example .env  (e configura)"
echo "  2. sudo bash start.sh"
