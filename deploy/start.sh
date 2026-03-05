#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$BASE_DIR")"
CADDY_BIN="$BASE_DIR/caddy"
LOGS_DIR="$BASE_DIR/logs"
REAL_USER="${SUDO_USER:-$USER}"

# Load nvm (needed for non-interactive shells, e.g. via SSH)
export NVM_DIR="/Users/$REAL_USER/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

# Load environment variables
if [ -f "$BASE_DIR/.env" ]; then
    set -a
    source "$BASE_DIR/.env"
    set +a
else
    echo "[!] ERROR: .env file not found. Copy .env.example and configure it."
    exit 1
fi

TS_HOSTNAME="${TS_HOSTNAME:?ERROR: TS_HOSTNAME not set in .env}"

# Verify custom Caddy binary exists
if [ ! -x "$CADDY_BIN" ]; then
    echo "[!] ERROR: Custom Caddy binary not found at $CADDY_BIN"
    echo "    Build it with: xcaddy build --with github.com/greenpau/caddy-security --output $CADDY_BIN"
    exit 1
fi

# Verify modded vibe-kanban exists
if [ ! -f "$PROJECT_DIR/npx-cli/bin/cli.js" ]; then
    echo "[!] ERROR: vibe-kanban cli not found at $PROJECT_DIR/npx-cli/bin/cli.js"
    exit 1
fi

# Verify code-server is installed
if sudo -u "$REAL_USER" bash -c 'command -v code-server' &>/dev/null; then
    :
else
    echo "[!] ERROR: code-server not found. Install it with: brew install code-server"
    exit 1
fi

# Verify TLS certificates exist
CERT_FILE="/usr/local/etc/letsencrypt/certificates/siphion.dev.crt"
KEY_FILE="/usr/local/etc/letsencrypt/certificates/siphion.dev.key"
if [ ! -f "$CERT_FILE" ] || [ ! -f "$KEY_FILE" ]; then
    echo "[!] ERROR: TLS certificates not found. Run: sudo bash ~/dev/mydns/certs/run-cert.sh"
    exit 1
fi

mkdir -p "$LOGS_DIR"
chown "$REAL_USER" "$LOGS_DIR"

# --- Vibe Kanban (modded) --- runs as real user
if pgrep -f "vibe-kanban" > /dev/null 2>&1; then
    echo "[OK] Vibe Kanban already running (PID $(pgrep -f 'vibe-kanban' | head -1))"
else
    echo "[*] Starting Vibe Kanban (modded) on 127.0.0.1:38100..."
    sudo -u "$REAL_USER" bash -c "
        export NVM_DIR='/Users/$REAL_USER/.nvm'
        [ -s \"\$NVM_DIR/nvm.sh\" ] && . \"\$NVM_DIR/nvm.sh\"
        HOST=127.0.0.1 PORT=38100 PREVIEW_PROXY_PORT=${PREVIEW_PROXY_PORT:-38101} VK_ALLOWED_ORIGINS='https://$TS_HOSTNAME' \
            nohup node '$PROJECT_DIR/npx-cli/bin/cli.js' > '$LOGS_DIR/vibe-kanban.log' 2>&1 &
    "
    sleep 2
    if pgrep -f "vibe-kanban" > /dev/null 2>&1; then
        echo "[OK] Vibe Kanban (modded) started (PID $(pgrep -f 'vibe-kanban' | head -1))"
    else
        echo "[!] ERROR: Vibe Kanban (modded) failed to start. Check $LOGS_DIR/vibe-kanban.log"
        exit 1
    fi
fi

# --- code-server (VS Code for the Web) --- runs as real user
if pgrep -f "code-server" > /dev/null 2>&1; then
    echo "[OK] code-server already running (PID $(pgrep -f 'code-server' | head -1))"
else
    echo "[*] Starting code-server on 127.0.0.1:38200..."
    sudo -u "$REAL_USER" bash -c "
        PORT= HOST= nohup code-server \
            --bind-addr 127.0.0.1:38200 \
            --auth none \
            --disable-telemetry \
            > '$LOGS_DIR/code-server.log' 2>&1 &
    "
    sleep 2
    if pgrep -f "code-server" > /dev/null 2>&1; then
        echo "[OK] code-server started (PID $(pgrep -f 'code-server' | head -1))"
    else
        echo "[!] ERROR: code-server failed to start. Check $LOGS_DIR/code-server.log"
        exit 1
    fi
fi

# --- Caddy (custom build with caddy-security, TLS + auth on :443) ---
if pgrep -f "$CADDY_BIN" > /dev/null 2>&1; then
    echo "[*] Caddy already running, reloading config..."
    "$CADDY_BIN" reload --config "$BASE_DIR/Caddyfile" --force 2>/dev/null
    echo "[OK] Caddy config reloaded"
else
    echo "[*] Starting Caddy on :443 (TLS with siphion.dev cert)..."
    HOME="/Users/$REAL_USER" "$CADDY_BIN" start --config "$BASE_DIR/Caddyfile"
    echo "[OK] Caddy started"
fi

echo ""
echo "=== Running (siphion.dev) ==="
echo "Vibe Kanban (mod) : http://127.0.0.1:38100 (local only)"
echo "code-server       : http://127.0.0.1:38200 (local only)"
echo "Caddy (TLS+auth)  : https://$TS_HOSTNAME     (tailnet)"
echo "code-server (web) : https://code.siphion.dev  (tailnet)"
echo "Logs              : $LOGS_DIR/"
