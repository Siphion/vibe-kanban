#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
CADDY_BIN="$BASE_DIR/caddy"

# --- Caddy ---
if pgrep -f "$CADDY_BIN" > /dev/null 2>&1; then
    "$CADDY_BIN" stop 2>/dev/null || sudo pkill -f "$CADDY_BIN"
    echo "[OK] Caddy stopped"
else
    echo "[--] Caddy not running"
fi

# --- Vibe Kanban ---
if pgrep -f "vibe-kanban" > /dev/null 2>&1; then
    pkill -f "vibe-kanban"
    echo "[OK] Vibe Kanban stopped"
else
    echo "[--] Vibe Kanban not running"
fi

# --- code-server ---
if pgrep -f "code-server" > /dev/null 2>&1; then
    pkill -f "code-server"
    echo "[OK] code-server stopped"
else
    echo "[--] code-server not running"
fi
